use log::warn;
use tokio::sync::oneshot::Receiver;
use url::Url;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak, Mutex};

use futures::{FutureExt, stream::{FuturesUnordered, StreamExt}};
use tokio::task::JoinHandle;
use tokio::sync::{ mpsc, oneshot};

use crate::error::ZyppError;
use crate::media::driver::MediaDriver;
use crate::media::spec::{FileSpec,MediaSpec};
use crate::media::drivers::http::MediaHttpDriver;

use super::MediaError;

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
    task: JoinHandle<()>,
    tx: mpsc::UnboundedSender<ToWorkerMsg>,
    schemes: Vec<String>
}

async fn execute_request ( driver: &mut dyn MediaDriver, request: ToWorkerMsg ) -> (){
    match request {
        ToWorkerMsg::Attach { res_rx, urls, spec } => {
            let res = driver.attach( urls, spec ).await; 
            res_rx.send( res );
        },
        ToWorkerMsg::Fetch { res_rx, attachId, path, spec } => {
            let res = driver.provide( attachId, path, spec ).await;
            res_rx.send( res );
        },
        ToWorkerMsg::Detach { attachId } => {
            if let Err(e) = driver.detach( attachId ) {
                warn!("Detached unknown id{}", attachId);
            }
        },
        ToWorkerMsg::Shutdown => return,
    }
    return; 
}

impl Worker {
    pub fn new ( driver: Box<dyn MediaDriver> ) -> Self
    {
        let ( tx, rx) = mpsc::unbounded_channel();
        Worker {
            tx: tx,
            schemes: driver.schemes(),
            task: tokio::spawn( async move {
                let mut rx = rx;
                let mut running_reqs = FuturesUnordered::default();
                let mut driver = RefCell::new(driver);

                loop {
                    let rx_fut = rx.recv().fuse();
                    futures::pin_mut!(rx_fut);
                    futures::select! {
                        m = rx_fut => {
                            if let Some(msg) = m {
                                let bo = driver.borrow_mut();
                                running_reqs.push( execute_request( &bo, msg) );
                            } else {
                                break;
                            }
                        },
                        _ = running_reqs.select_next_some() => {
                            continue;
                        }
                    }
                }
                rx.close();
            } )
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
                //v.detach(self);
            },
            None => {}
        }
    }
}

#[derive(Default)]
struct ManagerData {
    next_driver_id: u32,
    drivers: HashMap<u32, Worker>
}

pub struct Manager {
    data: Arc<Mutex<ManagerData>>
}

impl Manager {
    pub fn new() -> Self { 
        let mut me = Self { data: Arc::new(Mutex::new(ManagerData{ ..Default::default() })) };
        me.add_driver( Box::new(MediaHttpDriver::new()) );
        return me;
    }


    pub async fn attach ( &mut self, urls: &Vec<Url>, spec: &MediaSpec ) -> Result<AttachedMedium, ZyppError> {
        
        let mut resRx = None;
        for url in urls {
            let mut mut_data = self.data.lock().unwrap();
            for (driver_id, driver) in &mut mut_data.drivers {
                if !driver.can_handle_scheme(url.scheme()) {
                    continue;
                }

                let (tx, rx) = oneshot::channel();
                resRx = Some((*driver_id, url.clone(), rx));
                driver.tx.send( ToWorkerMsg::Attach { res_rx: tx, urls: urls.clone(), spec: spec.clone() } ).map_err(|_| MediaError::WorkerBroken)?;
                break;
            }
            if resRx.is_some() {
                break;
            }
        }

        if let Some((driver_id, url, rx)) = resRx {
            return rx.await.map_err(|_|MediaError::WorkerBroken )?
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

    pub async fn fetch<P: AsRef<Path>> ( &mut self, medium: &AttachedMedium, path: P, fileSpec: &FileSpec) -> Result<PathBuf, ZyppError> {
        let mut resRx = None;
        {
            let mut_data = self.data.lock().unwrap();
            let worker = mut_data.drivers.get( &medium.driver_id ).ok_or(MediaError::InvalidHandle)?;
            let (tx, rx) = oneshot::channel();
            worker.tx.send( ToWorkerMsg::Fetch { res_rx: tx, attachId: medium.id, path: path.as_ref().to_owned(), spec: fileSpec.clone() } );
            resRx = Some(rx);
        }

        if let Some(rx) = resRx {
            return rx.await.map_err(|_|MediaError::WorkerBroken )?;
        }
        Err( MediaError::InvalidHandle.into() )
    }

    pub fn detach( &mut self, medium: &AttachedMedium ) {

    }

    pub fn add_driver( &mut self, driver: Box<dyn MediaDriver> ) {
        let mut mut_data = self.data.lock().unwrap();
        mut_data.next_driver_id+=1;

        let myId = mut_data.next_driver_id;
        mut_data.drivers.insert( myId, Worker::new(driver) );
    }
}