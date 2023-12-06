use async_trait::async_trait;
use log::info;
use reqwest::{Client,Response};
use tokio::io::AsyncWriteExt;
use tokio::sync::{Notify, AcquireError, watch};
use tribool::Tribool::{True,False,Indeterminate};
use url::Url;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicI64;
use std::sync::{Arc, Weak, Mutex, PoisonError};
use tempfile::{TempDir, NamedTempFile};
use scopeguard::{defer, guard};

use tokio::fs::{DirBuilder, File};
use futures::StreamExt;

use crate::error::ZyppError;
use crate::media::{MediaError, driver::MediaDriver, spec::{*}};

struct AttachedMedia {
    use_cnt: Arc<AtomicI64>,
    attach_dir: TempDir,
    mirrors: Vec<Url>,
    spec: MediaSpec,
    requests_running: HashSet<PathBuf>,
    requests_notify: Arc<watch::Sender<()>>
}

struct MediaHttpDriverShared {
    next_attach_id: Mutex<u32>,
    attached_media: Mutex<HashMap<u32, AttachedMedia>>
}

pub struct MediaHttpDriver {
    inner: Arc<MediaHttpDriverShared>
}

impl<T> From<PoisonError<T>> for ZyppError {
    fn from(value: PoisonError<T>) -> Self {
        return ZyppError::Internal { message: value.to_string() };
    }
}

impl MediaHttpDriver {

    pub fn new() -> Self {
        Self {
            inner: Arc::new( MediaHttpDriverShared { next_attach_id: Mutex::new(1), attached_media: Default::default() })
        }
    }

    async fn download_file<P: AsRef<Path>>( mirror: &Url, path_on_medium: &Path, target_path: P, target_file_name: &str ) -> Result<PathBuf, ZyppError> {

        let target_file_path = target_path.as_ref().join(target_file_name);
        let tmp_file = NamedTempFile::new_in( &target_path )?.into_temp_path();

        info!("Downloading into tmp path: {}", tmp_file.to_str().unwrap_or_default() );

        let req_url = mirror.join( path_on_medium.to_str().ok_or( MediaError::InvalidPath)? ).map_err( |_| MediaError::InvalidPath )?;

        // open the file and truncate it
        let mut file = File::create( &tmp_file ).await?;

        let res = Client::new().get(req_url.clone()).send().await.map_err( MediaError::from )?;
        if res.status().is_success() {
            let mut stream = res.bytes_stream();

            // here we should also track downloaded bytes
            while let Some(item) = stream.next().await {
                file.write( &item.map_err( MediaError::from )? ).await?;
            }

            // yay we got the file
            // make sure its synced to the disk
            file.sync_all().await?;

            //@todo what if the file cannot be persistet?
            tmp_file.persist( &target_file_path ).map_err(|e| e.error )?;

            return Ok(target_file_path.to_owned());
        }

        // @todo match Reqwest status to actual error... e.g Auth, NotFound etc
        Err(MediaError::FileNotFound.into())
    }
}

#[async_trait]
impl MediaDriver for MediaHttpDriver {
    fn schemes( &self ) -> Vec<String> {
        vec!["http".to_owned(), "https".to_owned()]
    }

    async fn attach( &self, urls: Vec<Url>, spec: MediaSpec ) -> Result<u32, ZyppError> {

        let mut meds= self.inner.attached_media.lock()?;

        let maybeMedium = meds
        .iter()
        .find(|x| {
            match x.1.spec.is_same_medium(&spec) {
                Indeterminate => urls.first() == x.1.mirrors.first(),
                val @ ( False | True ) => val.try_into().unwrap()
            }
        });

        match maybeMedium {
            Some(m) => {
                m.1.use_cnt.fetch_add(1, std::sync::atomic::Ordering::Acquire);
                return Ok(*m.0);
            },
            None => {
                // we have no medium, make one, this normally should also download
                // the media file and check it if the spec has one. In that case we need to change
                // the workflow here to async...
                let (notify,_) = watch::channel(());
                let mut nId = self.inner.next_attach_id.lock()?;
                *nId += 1;
                meds.insert( *nId, AttachedMedia{
                    use_cnt: Arc::new( AtomicI64::new(1)),
                    attach_dir: tempfile::Builder::new().prefix("zypp-http").tempdir()?,
                    mirrors: urls,
                    spec: spec,
                    requests_running: Default::default(),
                    requests_notify: Arc::new(notify)
                });
                Ok( *nId )
            }

        }
    }

    fn detach( &self, id: u32 ) -> Result<(), ZyppError> {

        let mut meds= self.inner.attached_media.lock()?;
        let media = meds.get(&id);
        match media {
            Some(m) => {
                let old = m.use_cnt.fetch_sub(1, std::sync::atomic::Ordering::Release );
                if old <= 1 {
                    return meds.remove(&id).map_or(Err(MediaError::InvalidHandle.into()), |x| Ok(()) );
                }
                return Ok(());
            }
            None => { Err(MediaError::InvalidHandle.into()) }
        }
    }

    async fn provide( &self, attachId: u32, path: PathBuf, _spec: FileSpec ) -> Result<PathBuf, ZyppError> {

        let lock;
        let mut targetPath;

        {
            let mut medium = self.inner.attached_media.lock()?;
            let handle = medium.get_mut(&attachId).ok_or( MediaError::InvalidHandle )?;

            // mark this handle used
            lock = handle.use_cnt.clone();
            lock.fetch_add(1, std::sync::atomic::Ordering::Acquire );

            targetPath = handle.attach_dir.path().to_owned();
        }

        // release the handle once we are done with it
        defer!({
            let prev_cnt = lock.fetch_sub( 1, std::sync::atomic::Ordering::Release );
            if prev_cnt <= 1 {
                let _ = self.detach( attachId );
            }
        });

        if let Some(parent_path) = path.parent() {
            match parent_path.strip_prefix("/") {
                Ok(p) => {
                    targetPath.push( p );
                },
                Err(_) =>{
                    targetPath.push( parent_path );
                }
            }
        }

        // if the target dir does not exist, create it
        if !targetPath.try_exists()? {

            DirBuilder::new()
                .recursive(true)
                .create( &targetPath )
                .await?;
        }

        // must be a directory
        if !targetPath.as_path().is_dir() {
            return Err( MediaError::FileExists.into() );
        }


        let target_file_name = path.file_name().and_then( |x| {
            x.to_str()
        } ).ok_or(MediaError::NotAFile)?;
        // full filename
        let target_file_path = targetPath.join( target_file_name );

        let mut clean_guard = guard(false, |v|{
            if v {
                let medium = self.inner.attached_media.lock();
                if medium.is_err() {
                    return;
                }

                let mut medium = medium.unwrap();
                let handle = medium.get_mut(&attachId);
                if handle.is_none() {
                    return;
                }
                let handle = handle.unwrap();
                handle.requests_running.remove( &path );
                _ = handle.requests_notify.send(());
            }
        });

        loop {

            // if the target file is already there -> use it
            if target_file_path.try_exists()? {
                return Ok(target_file_path);
            }

            // we need to check if a request is already running
            // if yes we wait for it and then check if there is a local
            // file again.
            let mut rx = None;
            let mut mirrors = None;
            {
                let mut medium = self.inner.attached_media.lock()?;
                let handle = medium.get_mut(&attachId).ok_or( MediaError::InvalidHandle )?;
                if handle.requests_running.contains(&path) {
                    rx = Some( handle.requests_notify.subscribe());
                } else {
                    // its not in the list, so we do the request
                    handle.requests_running.insert(path.to_owned());
                    mirrors = Some(handle.mirrors.clone());
                    (*clean_guard) = true;
                }
            }
            if rx.is_some() {
                let _ = rx.unwrap().changed().await;
                continue;
            } else if mirrors.is_some() {
                let mut lastResult: Option<ZyppError> = None;
                for url in &mirrors.unwrap() {

                    let res: Result<PathBuf, ZyppError> = MediaHttpDriver::download_file(url, &path, &targetPath, &target_file_name).await;
                    match res {
                        Ok( result ) => {
                            return Ok(result);
                        },
                        Err(error) =>  {
                            lastResult = Some(error);
                        }
                    }
                }

                match lastResult {
                    Some(e) => { return Err(e); },
                    None=>{break;}
                }

            } else {
                break;
            }
        }
        return Err(MediaError::FileNotFound.into());
    }
}

impl Drop for MediaHttpDriverShared {
    fn drop(&mut self) {

    }
}
