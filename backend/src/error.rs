#![allow(unused)]

use deadpool_diesel::postgres::PoolError;
use diesel::ConnectionError;
use rocket::response::{self, Responder};

#[derive(Debug)]
pub enum Error {
    StorageError(StorageError),
    ParsingError(String),
    PollingError(notify::Error),
    CustomError(String),
    IoError(std::io::Error),
}

impl From<StorageError> for Error {
    fn from(err: StorageError) -> Self {
        Error::StorageError(err)
    }
}

impl From<notify::Error> for Error {
    fn from(err: notify::Error) -> Self {
        Error::PollingError(err)
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> response::Result<'o> {
        todo!()
    }
}

#[derive(Debug)]
pub enum StorageError {
    IoError(std::io::Error),
    NotFound(String),
    AlreadyExists(String),
    DecodingError(String),
    ConnectionError(ConnectionError),
    PoolError(PoolError),
    EventProcessingError(String),
    CustomError(String),
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::IoError(err)
    }
}

impl From<ConnectionError> for StorageError {
    fn from(err: ConnectionError) -> Self {
        StorageError::ConnectionError(err)
    }
}

impl From<PoolError> for StorageError {
    fn from(err: PoolError) -> Self {
        StorageError::PoolError(err)
    }
}

impl From<PoolError> for StorageError {
    fn from(err: PoolError) -> Self {
        StorageError::PoolError(err)
    }
}
