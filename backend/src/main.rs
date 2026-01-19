use std::env;

use crate::storage::storage_instance::{Event, StorageInstance};
use rocket::routes;
use routes::queries::{
    add_sequence, add_tag, get_entries, get_metadata, get_sequences, remove_sequence, remove_tag,
    update_metadata, update_sequence,
};
use tokio::sync::mpsc::Sender;
use tracing::Subscriber;
use tracing_subscriber::fmt::{
    Layer,
    writer::{BoxMakeWriter, MakeWriterExt},
};

pub mod error;
pub mod plugin_manager;
pub mod routes;
pub mod storage;
pub struct AppState {
    pub event_transmitter: Sender<Event>,
}

#[rocket::main]
async fn main() {
    // TODO findings: without the non-blocking appender, logs actually get written. They use some special characters which neither vscode nor zed can displayy, but my terminal can.
    let log_to_file = env::var("LOG_TO_FILE").is_ok_and(|v| v == "true");
    let log_level = match env::var("LOG_LEVEL")
        .unwrap_or("debug".to_string())
        .as_str()
    {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "info" => tracing::Level::INFO,
        "debug" => tracing::Level::DEBUG,
        _ => tracing::Level::INFO,
    };
    let file_appender = tracing_appender::rolling::daily("/logs", "backend.log");
    // non blocking so writing to file runs in a separate thread
    // this has to be kept in the main function and not in an if clause because otherwise the guard gets dropped
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // this needs to be boxed because the subscribers have very specific types
    let log_subscriber: Box<dyn Subscriber + Send + Sync + 'static> = if log_to_file {
        // if logging to file, log both to file and stdout
        Box::new(
            tracing_subscriber::fmt()
                .with_writer(
                    BoxMakeWriter::new(non_blocking).and(BoxMakeWriter::new(std::io::stdout)),
                )
                .with_ansi(false)
                .with_max_level(log_level)
                .finish(),
        )
    } else {
        // otherwise log to stdout but in pretty format
        Box::new(
            tracing_subscriber::fmt()
                .with_writer(BoxMakeWriter::new(std::io::stdout))
                .pretty()
                .with_ansi(false)
                .with_max_level(log_level)
                .finish(),
        )
    };
    tracing::subscriber::set_global_default(log_subscriber).unwrap();
    tracing::info!("Logging initialized.");
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    #[allow(unused_mut)]
    let mut storage_instance = StorageInstance::new(&db_url).unwrap();

    // storage_instance
    // .start_scanning(&Duration::from_secs(2))
    // .unwrap();

    // storage_instance.process_events().await.unwrap();

    let event_transmitter = storage_instance.get_event_transmitter();

    // web server
    rocket::build()
        .mount(
            "/",
            routes![
                get_entries,
                get_sequences,
                get_metadata,
                update_metadata,
                add_sequence,
                remove_sequence,
                update_sequence,
                add_tag,
                remove_tag,
            ],
        )
        .manage(AppState { event_transmitter })
        .launch()
        .await
        .unwrap();
}
