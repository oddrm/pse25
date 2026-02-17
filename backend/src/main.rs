// use std::os::unix::fs::MetadataExt; -> unnötig und Windows Problem
use std::path::{Path, PathBuf};
use std::{env, time::Duration};

use backend::AppState;
use backend::plugin_manager::manager::PluginManager;
use backend::routes::database::*;
use backend::routes::health_check::health;
use backend::routes::logs::*;
use backend::routes::plugins::*;
use backend::storage::file_watcher;
use backend::storage::storage_manager::StorageManager;
use tracing::instrument;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
#[macro_use]
extern crate rocket;

#[instrument]
#[rocket::main]
async fn main() {
    let stdout_level = match env::var("LOG_LEVEL")
        .unwrap_or("debug".to_string())
        .as_str()
    {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "info" => tracing::Level::INFO,
        "debug" => tracing::Level::DEBUG,
        _ => tracing::Level::INFO,
    };

    let file_appender = tracing_appender::rolling::hourly("/logs", "backend.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let noise_filter = filter_fn(|metadata| {
        if metadata.target().contains("rocket::server") || metadata.target().contains("hyper") {
            return *metadata.level() <= tracing::Level::WARN;
        }
        true
    });

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_filter(LevelFilter::DEBUG)
        .with_filter(noise_filter);

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .pretty()
        .with_filter(LevelFilter::from_level(stdout_level));

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .init();

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
                get_entry_by_path,
                get_entry,
                get_sensors,
                get_all_sensors,
                add_sensor,
                update_sensor,
                remove_sensor,
                get_sequences,
                get_topics,
                get_metadata,
                update_metadata,
                add_sequence,
                remove_sequence,
                update_sequence,
                add_tag,
                remove_tag,
                get_logs,
                start_transaction,
                commit_transaction,
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
