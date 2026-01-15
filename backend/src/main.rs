use std::time::Duration;

use crate::storage::storage_instance::{Event, StorageInstance};
use rocket::routes;
use routes::queries::{
    add_sequence, add_tag, get_entries, get_metadata, get_sequences, remove_sequence, remove_tag,
    update_metadata, update_sequence,
};
use std::path::PathBuf;
use tokio::sync::mpsc::Sender;
use tracing_subscriber::FmtSubscriber;

pub mod error;
pub mod plugin_manager;
pub mod routes;
pub mod storage;
pub struct AppState {
    pub event_transmitter: Sender<Event>,
}

/// Main entry point.
/// Launch web server, start db threads etc.
#[rocket::main]
async fn main() {
    // logging
    let log_subscriber = FmtSubscriber::new();
    tracing::subscriber::set_global_default(log_subscriber).unwrap();
    // tracing::info!("Logging initialized.");
    // db
    let mut storage_instance = StorageInstance::new(&PathBuf::from("storage_path")).unwrap();
    storage_instance
        .start_scanning(&Duration::from_secs(2))
        .unwrap();
    // TODO: process events
    storage_instance.process_events().await.unwrap();
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
