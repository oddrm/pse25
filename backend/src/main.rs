use std::{env, time::Duration};

use crate::storage::storage_manager::StorageManager;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use rocket::routes;
use routes::queries::{
    add_sequence, add_tag, get_entries, get_metadata, get_sequences, remove_sequence, remove_tag,
    update_metadata, update_sequence,
};
use tracing::Subscriber;
use tracing_subscriber::fmt::writer::{BoxMakeWriter, MakeWriterExt};

pub mod error;
pub mod plugin_manager;
pub mod routes;
pub mod schema;
pub mod storage;
pub struct AppState {
    pub storage_manager: StorageManager,
}

//ChatGPT Vorschlag, BITTE ÜBERPRÜFEN
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

async fn run_migrations(database_url: &str) {
    let database_url = database_url.to_string();
    tokio::task::spawn_blocking(move || {
        let mut conn = PgConnection::establish(&database_url)
            .expect("Failed to establish database connection for migrations");
        
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run database migrations");
    })
    .await
    .expect("Migration task panicked");
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
                // .with_ansi(false)
                .with_max_level(log_level)
                .finish(),
        )
    };
    tracing::subscriber::set_global_default(log_subscriber).unwrap();
    tracing::info!("Logging initialized.");
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    
    // Run database migrations
    tracing::info!("Running database migrations...");
    run_migrations(&db_url).await;
    tracing::info!("Database migrations completed.");
    
    #[allow(unused_mut)]
    let mut storage_manager = StorageManager::new(&db_url).unwrap();
    storage_manager
        .start_scanning(Duration::from_secs(1))
        .unwrap();

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
        .manage(AppState { storage_manager })
        .launch()
        .await
        .unwrap();
}
