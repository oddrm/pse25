// use std::os::unix::fs::MetadataExt; -> unnötig und Windows Problem
use backend::AppState;
use backend::plugin_manager::manager::PluginManager;
use backend::plugin_manager::plugin::Trigger;
use backend::routes::database::*;
use backend::routes::health_check::health;
use backend::routes::logs::*;
use backend::routes::plugins::*;
use backend::storage::file_watcher;
use backend::storage::storage_manager::StorageManager;
use std::path::PathBuf;
use std::sync::Arc;
use std::{env, time::Duration};
use tracing::debug;
use tracing::instrument;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
#[macro_use]
extern crate rocket;

/// Einstiegspunkt der Anwendung.
///
/// Startet:
/// - Logging
/// - Datenbank/Storage
/// - Plugin-Registrierung
/// - File-Watcher
/// - Hintergrundtasks für Reaping und Schedule-Ausführung
/// - Rocket-Webserver
#[instrument]
#[rocket::main]
async fn main() {
    // Log-Level aus Umgebungsvariable lesen.
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

    // Datenbankverbindung vorbereiten.
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    #[allow(unused_mut)]
    let mut storage_manager = StorageManager::new(&db_url).unwrap();

    // Plugin-Manager initialisieren und Plugins aus dem Verzeichnis laden.
    let mut plugin_manager = PluginManager::new();
    plugin_manager
        .register_plugins(PathBuf::from("/plugins"))
        .unwrap();
    plugin_manager
        .load_config_and_apply("/plugins/config/plugins.yaml")
        .unwrap();

    // Gemeinsamer, asynchroner Zugriff auf den Plugin-Manager.
    let plugin_manager_arc = Arc::new(tokio::sync::Mutex::new(plugin_manager));

    // File-Watcher starten, damit Dateisystem-Events in Backend-Events
    // bzw. Storage-Aktionen übersetzt werden können.
    file_watcher::start_scanning(
        &storage_manager,
        plugin_manager_arc.clone(),
        Duration::from_secs(5),
    )
    .await
    .unwrap();

    // Hintergrundtask:
    // räumt abgeschlossene oder unresponsive Plugin-Instanzen auf.
    {
        let pm_clone = plugin_manager_arc.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                if let Ok(mut guard) =
                    tokio::time::timeout(Duration::from_secs(1), pm_clone.lock()).await
                {
                    guard.reap_dead_and_unresponsive().await;
                }
            }
        });
    }

    // Hintergrundtask:
    // überwacht zeitgesteuerte Plugins und löst sie aus,
    // sobald ihr nächster Schedule-Zeitpunkt erreicht ist.
    {
        use chrono::{DateTime, Utc};
        use std::collections::HashMap;

        let pm = plugin_manager_arc.clone();
        tokio::spawn(async move {
            // Speichert für jedes Plugin den nächsten geplanten Ausführungszeitpunkt.
            // Schlüssel ist der Plugin-Pfad als stabiler Identifikator.
            let mut next_run: HashMap<String, DateTime<Utc>> = HashMap::new();

            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;

                // Phase 1:
                // Unter Lock Snapshot aller aktuell relevanten Schedule-Plugins holen.
                let scheduled_snapshot: Vec<(String, DateTime<Utc>)> = {
                    let guard = pm.lock().await;

                    let now = Utc::now();
                    let mut out = Vec::new();

                    for p in guard.registered.iter() {
                        if !p.enabled() || !p.valid() {
                            continue;
                        }

                        let Trigger::OnSchedule(schedule) = p.trigger() else {
                            continue;
                        };

                        if let Some(next_dt) = schedule.upcoming(Utc).next() {
                            let key = p.path().to_string_lossy().into_owned();

                            // Bereits berechnete Werte möglichst beibehalten,
                            // außer sie liegen inzwischen in der Vergangenheit.
                            let effective_next = next_run.get(&key).cloned().unwrap_or(next_dt);
                            let effective_next = if effective_next < now {
                                next_dt
                            } else {
                                effective_next
                            };
                            out.push((key, effective_next));
                        }
                    }

                    out
                };

                // Plugins entfernen, die nach einem Rescan nicht mehr existieren.
                {
                    let keys_in_snapshot: std::collections::HashSet<_> =
                        scheduled_snapshot.iter().map(|(k, _)| k.clone()).collect();
                    next_run.retain(|k, _| keys_in_snapshot.contains(k));
                }

                // Phase 2:
                // Fällige Plugins starten, ohne dabei dauerhaft den Lock zu halten.
                let now = Utc::now();
                for (key, planned_next) in scheduled_snapshot {
                    next_run.entry(key.clone()).or_insert(planned_next);

                    let due = match next_run.get(&key) {
                        Some(t) => *t <= now,
                        None => false,
                    };
                    if !due {
                        continue;
                    }

                    // Vor dem tatsächlichen Start erneut prüfen,
                    // weil sich durch Rescans inzwischen etwas geändert haben kann.
                    let schedule_clone = {
                        let guard = pm.lock().await;

                        let Some((_idx, plugin)) = guard
                            .registered
                            .iter()
                            .enumerate()
                            .find(|(_i, p)| p.path().to_string_lossy() == key)
                        else {
                            continue;
                        };

                        if !plugin.enabled() || !plugin.valid() {
                            continue;
                        }

                        let Trigger::OnSchedule(schedule) = plugin.trigger() else {
                            continue;
                        };

                        schedule.clone()
                    };

                    // Statt direkt Instanzen zu bauen, wird ein Backend-Event gefeuert.
                    // Das hält die Ausführung konsistent mit anderen Trigger-Arten.
                    let event = backend::plugin_manager::plugin::BackendEvent::OnSchedule {
                        schedule: schedule_clone.clone(),
                        path: "/data".to_string(),
                    };
                    debug!("Firing schedule event for '{}'", key);

                    let fire_res = tokio::time::timeout(
                        Duration::from_secs(10),
                        PluginManager::fire_event_detached(pm.clone(), event),
                    )
                    .await;

                    match fire_res {
                        Ok(Ok(_instance_ids)) => {
                            // Erfolgreich gestartet.
                        }
                        Ok(Err(e)) => {
                            tracing::warn!("schedule fire_event failed for '{}': {:?}", key, e);
                        }
                        Err(_) => {
                            tracing::warn!("schedule fire_event timed out for '{}'", key);
                        }
                    }

                    // Nächsten Run-Zeitpunkt neu berechnen.
                    if let Some(next_dt) = schedule_clone.upcoming(Utc).next() {
                        next_run.insert(key.clone(), next_dt);
                    }
                }
            }
        });
    }

    // Rocket-Webserver starten und alle Routen registrieren.
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
                get_plugin_instances,
                get_registered_plugins,
                enable_plugin,
                disable_plugin,
            ],
        )
        .manage(AppState {
            storage_manager,
            plugin_manager: plugin_manager_arc,
        })
        .launch()
        .await
        .unwrap();
}
