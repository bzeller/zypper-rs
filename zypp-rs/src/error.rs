use std::io;

use thiserror::Error;
use crate::repoinfo::Error as RepoInfoError;
use crate::media::MediaError as MediaError;

#[derive(Error, Debug)]
pub enum ZyppError {
    #[error("Media Error - {source}")]
    Media {
        #[from]
        source: MediaError
    },

    #[error("Pool Error - {source}")]
    Pool {
        #[from]
        source: PoolError
    },

    #[error("Repo Error - {source}")]
    Repo {
        #[from]
        source: RepoInfoError
    },
    #[error("IO Error - {source}")]
    IoError{
        #[from]
        source: io::Error
    },

    #[error("unknown error")]
    Unknown,
}

#[derive(Error, Debug)]
#[error("Some message - {msg}")]
pub struct UnknownPackage {
    msg: String
}

#[derive(Error, Debug)]
pub enum PoolError {
    #[error(transparent)]
    UnknownPackage(#[from] UnknownPackage)
}

#[derive(Error, Debug)]
pub enum SubRepoError {
    #[error("Even more nested repo error thingy")]
    TotallyUselessError
}
