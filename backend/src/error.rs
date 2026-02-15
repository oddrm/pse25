#![allow(unused)]

use deadpool_diesel::{postgres::PoolError, InteractError};
use diesel::ConnectionError;
use rocket::http::Status;
use rocket::response::{self, Responder};
use rocket::serde::json::Json;
use serde::Serialize;

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

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

impl Error {
    fn to_status_and_message(&self) -> (Status, String) {
        match self {
            Error::ParsingError(msg) => (Status::BadRequest, msg.clone()),

            Error::CustomError(msg) => {
                // Spezieller Fall: "PluginManager lock timeout" => nicht stapeln, sondern retry.
                if msg.to_lowercase().contains("lock timeout")
                    || msg.to_lowercase().contains("busy")
                {
                    (Status::ServiceUnavailable, msg.clone())
                } else {
                    (Status::BadRequest, msg.clone())
                }
            }

            Error::StorageError(se) => match se {
                StorageError::NotFound(msg) => (Status::NotFound, msg.clone()),
                StorageError::AlreadyExists(msg) => (Status::Conflict, msg.clone()),
                StorageError::DecodingError(msg) => (Status::BadRequest, msg.clone()),

                // DB/Pool: in der Praxis oft temporär => 503
                StorageError::ConnectionError(_) | StorageError::PoolError(_) => (
                    Status::ServiceUnavailable,
                    "Database temporarily unavailable".to_string(),
                ),

                StorageError::IoError(_) => (
                    Status::InternalServerError,
                    "I/O error".to_string(),
                ),

                StorageError::EventProcessingError(msg)
                | StorageError::CustomError(msg) => (Status::InternalServerError, msg.clone()),
            },

            Error::PollingError(_) => (
                Status::InternalServerError,
                "Watcher/polling error".to_string(),
            ),

            Error::IoError(_) => (
                Status::InternalServerError,
                "I/O error".to_string(),
            ),
        }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> response::Result<'o> {
        let (status, message) = self.to_status_and_message();

        // Einheitliche JSON-Fehlerform fürs Frontend
        response::status::Custom(status, Json(ErrorResponse { error: message })).respond_to(req)
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

impl From<InteractError> for StorageError {
    fn from(err: InteractError) -> Self {
        StorageError::CustomError(format!("Deadpool interact error: {:?}", err))
    }
}

impl From<diesel::result::Error> for StorageError {
    fn from(err: diesel::result::Error) -> Self {
        StorageError::CustomError(format!("Diesel error: {:?}", err))
    }
}
