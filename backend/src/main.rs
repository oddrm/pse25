// use std::os::unix::fs::MetadataExt; -> unnötig und Windows Problem
use std::path::{Path, PathBuf};
use std::{env, time::Duration};

use backend::AppState;
use backend::plugin_manager::manager::PluginManager;
use backend::routes::database::*;
use backend::routes::health_check::health;
use backend::routes::plugins::*;
use backend::storage::file_watcher;
use backend::storage::storage_manager::StorageManager;
use tracing::{Subscriber, instrument};
use tracing_subscriber::fmt::writer::{BoxMakeWriter, MakeWriterExt};
#[macro_use]
extern crate rocket;

#[instrument]
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
    // this needs to be boxed because the subscribers have very specific types
    let log_subscriber: Box<dyn Subscriber + Send + Sync + 'static> = if log_to_file {
        let file_appender = tracing_appender::rolling::daily("/logs", 
                                                             "backend.log");
        // non blocking so writing to file runs in a separate thread
        // this has to be kept in the main function and not in an if clause because otherwise the 
        // guard gets dropped
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
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
    file_watcher::start_scanning(&storage_manager, Duration::from_secs(1))
        .await
        .unwrap();
    let mut plugin_manager = PluginManager::new();

    // check if /plugins exists and list all files
    let plugin_dir = Path::new("/plugins");

    plugin_manager
        .register_plugins(PathBuf::from("/plugins"))
        .unwrap();
    plugin_manager
        .load_config_and_apply("/plugins/config/plugins.yaml")
        .unwrap();

    // TODO check all methods used
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
                register_plugins,
                register_plugin,
                start_plugin_instance,
                stop_plugin_instance,
                pause_plugin_instance,
                resume_plugin_instance,
                get_running_instances,
                get_registered_plugins,
                enable_plugin,
                disable_plugin,
            ],
        )
        .manage(AppState {
            storage_manager,
            plugin_manager: tokio::sync::Mutex::new(plugin_manager),
        })
        .launch()
        .await
        .unwrap();
}
