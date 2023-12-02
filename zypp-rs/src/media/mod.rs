use thiserror::Error;

pub mod manager;
pub(crate) mod driver;
pub(crate) mod spec;
mod drivers;

#[derive(Error, Debug)]
pub enum MediaError {
    #[error("The file was not found")]
    FileNotFound,
    #[error("The file did exist already")]
    FileExists,
    #[error("Invalid media handle")]
    InvalidHandle,
    #[error("Failed to parse given URL")]
    InvalidUrl,
    #[error("Given Path was invalid")]
    InvalidPath,
    #[error("Could not find a valid driver for the given Mirros")]
    NoDriverFound,
    #[error("Communication with worker task broke")]
    WorkerBroken
}