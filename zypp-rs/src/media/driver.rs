use async_trait::async_trait;
use std::error::Error;
use std::path::{Path, PathBuf};
use url::Url;

use crate::error::ZyppError;
use crate::media::spec::{MediaSpec, FileSpec};

#[async_trait]
pub trait MediaDriver : Send {
    fn schemes( &self ) -> Vec<String>;

    async fn attach( &mut self, urls: Vec<Url>, spec: MediaSpec ) -> Result<u32, ZyppError>;
    async fn provide( &mut self, attachId: u32, path: PathBuf, spec: FileSpec ) -> Result<PathBuf, ZyppError>;

    fn detach( &mut self, id: u32 ) -> Result<(), ZyppError>;
}