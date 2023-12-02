use async_trait::async_trait;
use log::info;
use reqwest::{Client,Response};
use tokio::io::AsyncWriteExt;
use tokio::sync::Notify;
use tribool::Tribool::{True,False,Indeterminate};
use url::Url;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tempfile::{TempDir, NamedTempFile};
use scopeguard::defer;

use tokio::fs::{DirBuilder, File};
use futures::StreamExt;

use crate::error::ZyppError;
use crate::media::{MediaError, driver::MediaDriver, spec::{*}};

struct AttachedMedia {
    attach_dir: TempDir,
    mirrors: Vec<Url>,
    spec: MediaSpec,
    requests_running: HashSet<PathBuf>,
    requests_notify: Notify
}

pub struct MediaHttpDriver {
    next_attach_id: u32,
    attached_media: HashMap<u32, AttachedMedia>
}

impl From<reqwest::Error> for MediaError {
    fn from(value: reqwest::Error) -> Self {
        todo!()
    }
}

impl MediaHttpDriver {

    pub fn new() -> Self { 
        todo!();
    }

    async fn download_file<P: AsRef<Path>>( mirror: &Url, path_on_medium: &Path, target_path: P, target_file_name: &str ) -> Result<PathBuf, ZyppError> {

        let target_file_path = target_path.as_ref().join(target_file_name);
        let tmp_file = NamedTempFile::new_in( &target_file_path )?.into_temp_path();

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
            tmp_file.persist( target_path ).map_err(|e| e.error )?;
            
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

    async fn attach( &mut self, urls: Vec<Url>, spec: MediaSpec ) -> Result<u32, ZyppError> {

        let maybeMedium = self.attached_media
        .iter()
        .find(|x| {
            match x.1.spec.is_same_medium(&spec) {
                Indeterminate => urls.first() == x.1.mirrors.first(),
                val @ ( False | True ) => val.try_into().unwrap()
            }
        });

        match maybeMedium {
            Some(m) => {
                return Ok(*m.0);
            },
            None => {
                // we have no medium, make one, this normally should also download
                // the media file and check it if the spec has one. In that case we need to change
                // the workflow here to async...
                self.next_attach_id += 1;
                self.attached_media.insert( self.next_attach_id, AttachedMedia{ 
                    attach_dir: tempfile::Builder::new().prefix("zypp-http").tempdir()?,
                    mirrors: urls, 
                    spec: spec,
                    requests_running: Default::default(),
                    requests_notify: Notify::new() 
                });
                Ok( self.next_attach_id )
            }

        }
    }

    fn detach( &mut self, id: u32 ) -> Result<(), ZyppError> {

        self.attached_media.get(&id)
        .and_then(|x|{
            info!("Detaching media {}", id);
            Some(x)
        });
        
        self.attached_media.remove(&id)
            .map_or(Err(MediaError::InvalidHandle.into()), |x| Ok(()) )
    }

    async fn provide( &mut self, attachId: u32, path: PathBuf, _spec: FileSpec ) -> Result<PathBuf, ZyppError> {

        let medium = self.attached_media.get_mut(&attachId).ok_or( MediaError::InvalidHandle )?;
        let mut targetPath = medium.attach_dir.path().to_owned();

        if let Some(parentPath) = path.parent() {
            targetPath.push( parentPath );
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
        

        let target_file_name = path.file_name().and_then( |x| x.to_str() ).ok_or(MediaError::FileNotFound)?;
        // full filename
        let targetFilePath = targetPath.join( target_file_name );
        
        loop {

            // if the target file is already there -> use it
            if targetFilePath.try_exists()? {
                return Ok(targetFilePath);
            }
    
            // we need to check if a request is already running
            // if yes we wait for it and then check if there is a local
            // file again. 
            if medium.requests_running.contains(&path) {
                medium.requests_notify.notified().await;
            } else {
                // its not in the list, so we do the request
                medium.requests_running.insert(path.to_owned());

                // need to work around borrowing all of medium in the defer! 
                let req_run = &mut medium.requests_running;
                let req_not = &medium.requests_notify;
                defer! {
                    req_run.remove( &path );
                    req_not.notify_waiters();
                }

                let mut lastResult: Option<ZyppError> = None;

                for url in &medium.mirrors {

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

                return Err(MediaError::FileNotFound.into());
            }       
        }
    }
}

impl Drop for MediaHttpDriver {
    fn drop(&mut self) {
        todo!()
    }
}