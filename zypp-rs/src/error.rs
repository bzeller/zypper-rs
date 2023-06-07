use thiserror::Error;
use crate::repoinfo::Error as RepoInfoError;

/*
 */

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

    #[error("unknown error")]
    Unknown,
}

#[derive(Error, Debug)]
pub enum MediaError {
    #[error("The file was not found")]
    FileNotFound,
    #[error("The file did exist already")]
    FileExists,
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
