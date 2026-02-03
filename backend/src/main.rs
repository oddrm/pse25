use std::{env, time::Duration};

use backend::AppState;
use backend::routes::health_check::health;
use backend::routes::queries::{
    add_sequence, add_tag, get_entries, get_metadata, get_sequences, remove_sequence, remove_tag,
    update_metadata, update_sequence,
};
use backend::storage::file_watcher;
use backend::storage::storage_manager::StorageManager;
use tracing::Subscriber;
use tracing_subscriber::fmt::writer::{BoxMakeWriter, MakeWriterExt};
#[macro_use]
extern crate rocket;

#[rocket::main]
async fn main() {
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
                // .pretty()
                .compact()
                .with_max_level(log_level)
                .finish(),
        )
    };
    tracing::subscriber::set_global_default(log_subscriber).unwrap();
    tracing::info!("Logging initialized.");
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    #[allow(unused_mut)]
    let mut storage_manager = StorageManager::new(&db_url).unwrap();
    file_watcher::scan_once(&storage_manager).await.unwrap();
    file_watcher::start_scanning(&storage_manager, Duration::from_secs(1)).unwrap();

    // web server
    rocket::build()
        .mount(
            "/",
            routes![
                health,
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
        .manage(AppState { storage_manager })
        .launch()
        .await
        .unwrap();
}
