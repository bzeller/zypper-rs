use thiserror::Error;

#[derive(Error, Debug)]
pub enum ZyppError {
    #[error("unknown error")]
    Unknown,
}