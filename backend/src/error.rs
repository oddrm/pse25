use deadpool_diesel::{InteractError, postgres::PoolError};
use diesel::ConnectionError;
use rocket::http::Status;
use rocket::response::{self, Responder};
use serde::Serialize;

/// Zentrale Fehlerart der Anwendung.
///
/// Diese Enum bündelt Fehler aus verschiedenen Teilsystemen,
/// damit API-Routen und interne Funktionen einheitlich mit
/// `Result<T, Error>` arbeiten können.
#[derive(Debug)]
pub enum Error {
    /// Fehler aus dem Storage-/Datenbank-Bereich.
    StorageError(StorageError),
    /// Fehler beim Parsen von Eingabedaten.
    ParsingError(String),
    /// Fehler aus File-Watcher/Polling.
    PollingError(notify::Error),
    /// Frei formulierbarer Anwendungsfehler.
    CustomError(String),
    /// Allgemeiner I/O-Fehler.
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

/// Einfaches JSON-Fehlerformat für HTTP-Antworten.
/// Aktuell im Code nicht aktiv verwendet, aber als Struktur vorbereitet.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

impl Error {
    /// Ordnet einen internen Fehler einem HTTP-Statuscode plus
    /// benutzerfreundlicher Nachricht zu.
    ///
    /// Diese Methode ist nützlich, wenn Fehler sauber nach außen
    /// übersetzt werden sollen.
    fn to_status_and_message(&self) -> (Status, String) {
        match self {
            Error::ParsingError(msg) => (Status::BadRequest, msg.clone()),

            Error::CustomError(msg) => {
                // Spezieller Fall:
                // "busy"/"lock timeout" ist oft ein temporäres Problem,
                // deshalb 503 statt 400.
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

                // Verbindungs-/Poolprobleme sind häufig temporär.
                StorageError::ConnectionError(_) | StorageError::PoolError(_) => (
                    Status::ServiceUnavailable,
                    "Database temporarily unavailable".to_string(),
                ),

                StorageError::IoError(_) => (Status::InternalServerError, "I/O error".to_string()),

                StorageError::EventProcessingError(msg) | StorageError::CustomError(msg) => {
                    (Status::InternalServerError, msg.clone())
                }
                StorageError::McapError(_) => {
                    (Status::InternalServerError, "Mcap error".to_string())
                }
            },

            Error::PollingError(_) => (
                Status::InternalServerError,
                "Watcher/polling error".to_string(),
            ),

            Error::IoError(_) => (Status::InternalServerError, "I/O error".to_string()),
        }
    }
}

/// Macht `Error` direkt als Rocket-Responder verwendbar.
///
/// Dadurch können Routen einfach `Result<T, Error>` zurückgeben,
/// und Rocket baut automatisch eine HTTP-Antwort daraus.
impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> response::Result<'o> {
        let (status, message) = match self {
            Error::StorageError(e) => (
                rocket::http::Status::InternalServerError,
                format!("Storage error: {:?}", e),
            ),
            Error::ParsingError(msg) => (
                rocket::http::Status::BadRequest,
                format!("Parsing error: {}", msg),
            ),
            Error::PollingError(e) => (
                rocket::http::Status::InternalServerError,
                format!("Polling error: {:?}", e),
            ),
            Error::CustomError(msg) => (
                rocket::http::Status::InternalServerError,
                format!("Error: {}", msg),
            ),
            Error::IoError(e) => (
                rocket::http::Status::InternalServerError,
                format!("IO error: {:?}", e),
            ),
        };
        response::Response::build_from(message.respond_to(req)?)
            .status(status)
            .ok()
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
    McapError(mcap::McapError),
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

impl From<mcap::McapError> for StorageError {
    fn from(err: mcap::McapError) -> Self {
        StorageError::McapError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::Status;

    #[test]
    fn parsing_errors_map_to_bad_request() {
        let err = Error::ParsingError("bad input".to_string());
        let (status, message) = err.to_status_and_message();

        assert_eq!(status, Status::BadRequest);
        assert_eq!(message, "bad input");
    }

    #[test]
    fn busy_custom_errors_map_to_service_unavailable() {
        let err = Error::CustomError("database busy".to_string());
        let (status, message) = err.to_status_and_message();

        assert_eq!(status, Status::ServiceUnavailable);
        assert_eq!(message, "database busy");
    }

    #[test]
    fn regular_custom_errors_map_to_bad_request() {
        let err = Error::CustomError("plain validation error".to_string());
        let (status, message) = err.to_status_and_message();

        assert_eq!(status, Status::BadRequest);
        assert_eq!(message, "plain validation error");
    }

    #[test]
    fn storage_error_variants_map_to_expected_statuses() {
        let not_found = Error::StorageError(StorageError::NotFound("missing".to_string()));
        let already_exists =
            Error::StorageError(StorageError::AlreadyExists("duplicate".to_string()));
        let decoding = Error::StorageError(StorageError::DecodingError("decode".to_string()));
        let io = Error::StorageError(StorageError::IoError(std::io::Error::other("disk")));
        let event = Error::StorageError(StorageError::EventProcessingError("event".to_string()));
        let custom = Error::StorageError(StorageError::CustomError("custom".to_string()));

        assert_eq!(not_found.to_status_and_message(), (Status::NotFound, "missing".to_string()));
        assert_eq!(
            already_exists.to_status_and_message(),
            (Status::Conflict, "duplicate".to_string())
        );
        assert_eq!(
            decoding.to_status_and_message(),
            (Status::BadRequest, "decode".to_string())
        );
        assert_eq!(
            io.to_status_and_message(),
            (Status::InternalServerError, "I/O error".to_string())
        );
        assert_eq!(
            event.to_status_and_message(),
            (Status::InternalServerError, "event".to_string())
        );
        assert_eq!(
            custom.to_status_and_message(),
            (Status::InternalServerError, "custom".to_string())
        );
    }

    #[test]
    fn connection_errors_map_to_service_unavailable() {
        let err = Error::StorageError(StorageError::ConnectionError(
            ConnectionError::BadConnection("db unavailable".to_string()),
        ));

        assert_eq!(
            err.to_status_and_message(),
            (
                Status::ServiceUnavailable,
                "Database temporarily unavailable".to_string()
            )
        );
    }

    #[test]
    fn polling_and_io_errors_map_to_internal_server_error() {
        let polling = Error::PollingError(notify::Error::generic("watch failed"));
        let io = Error::IoError(std::io::Error::other("io failed"));

        assert_eq!(
            polling.to_status_and_message(),
            (Status::InternalServerError, "Watcher/polling error".to_string())
        );
        assert_eq!(
            io.to_status_and_message(),
            (Status::InternalServerError, "I/O error".to_string())
        );
    }

    #[test]
    fn from_impls_wrap_source_errors() {
        let storage = StorageError::IoError(std::io::Error::other("io"));
        let err: Error = storage.into();
        assert!(matches!(err, Error::StorageError(StorageError::IoError(_))));

        let polling: Error = notify::Error::generic("watch failed").into();
        assert!(matches!(polling, Error::PollingError(_)));
    }
}
