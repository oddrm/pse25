#![allow(unused)]

use diesel::ConnectionError;
use rocket::response::{self, Responder};

#[derive(Debug)]
pub enum Error {
    StorageError(StorageError),
    ParsingError(String),
    PollingError(notify::Error),
    CustomError(String),
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
    // gehe mal davon aus, dass ConnectionError neuer ist als Stand Entwurfsheft
    ConnectionError(ConnectionError),
    CustomError(String),
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::IoError(err)
    }
}

// nicht in Entwurfsheft
impl From<ConnectionError> for StorageError {
    fn from(err: ConnectionError) -> Self {
        StorageError::ConnectionError(err)
    }
}
