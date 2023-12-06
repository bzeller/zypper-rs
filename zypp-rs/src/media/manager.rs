use log::warn;
use url::Url;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak, Mutex};

use futures::{FutureExt, stream::{FuturesUnordered, StreamExt}};
use tokio::sync::{ mpsc, oneshot};

use crate::error::ZyppError;
use crate::media::driver::MediaDriver;
use crate::media::spec::{FileSpec,MediaSpec};
use crate::media::drivers::http::MediaHttpDriver;

use super::MediaError;

#[derive(Debug)]
pub struct AttachedMedium {
    parent: Weak<Mutex<ManagerData>>,
    id: u32,
    driver_id: u32,
    base_url: Url
}

enum ToWorkerMsg {
    Attach {
        res_rx: oneshot::Sender<Result<u32, ZyppError>>,
        urls: Vec<Url>,
        spec: MediaSpec
    },
    Fetch {
        res_rx: oneshot::Sender<Result<PathBuf, ZyppError>>,
        attachId: u32,
        path: PathBuf,
        spec: FileSpec
    },
    Detach {
        attachId: u32
    },
    Shutdown
}

pub struct Worker {
    driver: Box<dyn MediaDriver +Send +Sync>
}

impl Worker {
    pub fn new ( driver: Box<dyn MediaDriver +Send +Sync> ) -> Self
    {
        Worker {
            driver: driver
        }
    }

    pub async fn run ( &self, mut rx: mpsc::UnboundedReceiver<ToWorkerMsg> ) {
        {
            let mut running_reqs = FuturesUnordered::default();
            loop {
                futures::select! {
                    m = rx.recv().fuse() => {
                        if let Some(msg) = m {
                            running_reqs.push( self.execute_request(msg) );
                        } else {
                            break;
                        }
                    },
                    _ = running_reqs.select_next_some() => {
                        continue;
                    }
                }
            }
        }
        rx.close();
    }

    async fn execute_request ( &self, request: ToWorkerMsg ) -> () {
        match request {
            ToWorkerMsg::Attach { res_rx, urls, spec } => {
                let res = self.driver.attach( urls, spec ).await;
                res_rx.send( res );
            },
            ToWorkerMsg::Fetch { res_rx, attachId, path, spec } => {
                let res = self.driver.provide( attachId, path, spec ).await;
                res_rx.send( res );
            },
            ToWorkerMsg::Detach { attachId } => {
                if let Err(e) = self.driver.detach( attachId ) {
                    warn!("Detached unknown id{}", attachId);
                }
            },
            ToWorkerMsg::Shutdown => return,
        }
        return;
    }


}


#[derive(Clone)]
pub struct WorkerHandle {
    tx: mpsc::UnboundedSender<ToWorkerMsg>,
    schemes: Vec<String>
}

impl WorkerHandle {
    pub fn new ( driver: Box<dyn MediaDriver +Send +Sync>) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let schemes = driver.schemes();
        let worker = Worker::new( driver );

        tokio::spawn( async move {
            worker.run( receiver ).await;
        });

        if sender.is_closed()
        {
            log::warn!("WTF");

        }

        Self {
            tx: sender,
            schemes: schemes
        }
    }

    fn can_handle_scheme( &self, scheme: &str ) -> bool {
        self.schemes.contains( &scheme.to_owned() )
    }
}

impl Drop for AttachedMedium {
    fn drop( &mut self) {
        match self.parent.upgrade() {
            Some(ref v) => {
                let res = v.lock();
                if res.is_ok()  {
                    res.unwrap().detach(self);
                }
            },
            None => {}
        }
    }
}

#[derive(Default)]
struct ManagerData {
    next_driver_id: u32,
    drivers: HashMap<u32, WorkerHandle>
}

impl ManagerData {
    pub fn detach( &self, medium: &AttachedMedium ) {

    }
}

pub struct Manager {
    data: Arc<Mutex<ManagerData>>
}

impl Manager {
    pub fn new() -> Self {
        let me = Self { data: Arc::new(Mutex::new(ManagerData{ ..Default::default() })) };
        me.add_driver( Box::new(MediaHttpDriver::new()) );
        return me;
    }


    pub async fn attach ( &self, urls: &Vec<Url>, spec: &MediaSpec ) -> Result<AttachedMedium, ZyppError> {

        let mut resRx = None;
        for url in urls {
            let mut mut_data = self.data.lock().unwrap();
            for (driver_id, driver) in &mut mut_data.drivers {
                if !driver.can_handle_scheme(url.scheme()) {
                    continue;
                }

                let (tx, rx) = oneshot::channel();
                resRx = Some((*driver_id, url.clone(), rx));
                driver.tx.send( ToWorkerMsg::Attach { res_rx: tx, urls: urls.clone(), spec: spec.clone() } ).map_err(|e| MediaError::WorkerBroken(e.to_string()))?;
                break;
            }
            if resRx.is_some() {
                break;
            }
        }

        if let Some((driver_id, url, rx)) = resRx {
            return rx.await.map_err(|e|MediaError::WorkerBroken(e.to_string()) )?
            .and_then( |id| {
                Ok(AttachedMedium{
                    driver_id: driver_id,
                    base_url: url,
                    id: id,
                    parent: Arc::downgrade(&self.data)
                })
            });
        }

        return Err(MediaError::NoDriverFound.into());
    }

    pub async fn fetch<P: AsRef<Path>> ( &self, medium: &AttachedMedium, path: P, fileSpec: &FileSpec) -> Result<PathBuf, ZyppError> {
        let mut resRx = None;
        {
            let mut_data = self.data.lock().unwrap();
            let worker = mut_data.drivers.get( &medium.driver_id ).ok_or(MediaError::InvalidHandle)?;
            let (tx, rx) = oneshot::channel();
            worker.tx.send( ToWorkerMsg::Fetch { res_rx: tx, attachId: medium.id, path: path.as_ref().to_owned(), spec: fileSpec.clone() } ).map_err(|e| MediaError::WorkerBroken(e.to_string()))?;
            resRx = Some(rx);
        }

        if let Some(rx) = resRx {
            return rx.await.map_err(|e|MediaError::WorkerBroken(e.to_string()) )?;
        }
        Err( MediaError::InvalidHandle.into() )
    }

    pub fn add_driver( &self, driver: Box<dyn MediaDriver + Send + Sync> ) {
        let mut mut_data = self.data.lock().unwrap();
        mut_data.next_driver_id+=1;

        let my_id = mut_data.next_driver_id;
        mut_data.drivers.insert( my_id, WorkerHandle::new(driver) );
    }
}
