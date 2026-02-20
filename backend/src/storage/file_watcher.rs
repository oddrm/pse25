use std::collections::HashSet;
use std::{path::Path, time::Duration};

use crate::schema::files;
use crate::storage::models::*;
use crate::{
    error::{Error, StorageError},
    storage::{parsing, storage_manager::StorageManager},
};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use notify::{RecursiveMode, Watcher};
use rocket::futures::StreamExt;
use rocket::futures::stream::FuturesUnordered;
use tokio::sync::mpsc::{self, Receiver};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, instrument, warn};
use walkdir::WalkDir;

// NEW: plugin manager type
use crate::plugin_manager::manager::PluginManager;
use crate::plugin_manager::plugin::BackendEvent;
use std::sync::Arc;
use tokio::sync::Mutex;

// Helper: fire plugin backend event without holding global lock across awaits
async fn fire_plugin_event(
    plugin_manager: Arc<Mutex<PluginManager>>,
    event: BackendEvent,
    data: Option<String>,
) {
    // Phase 1: prepare under lock + attach plugin_name for detached build
    let plans: Vec<(usize, String, std::path::PathBuf, u64)> = {
        let pm = plugin_manager.lock().await;
        let raw = match pm.prepare_fire_event(&event) {
            Ok(v) => v,
            Err(e) => {
                warn!("prepare_fire_event failed: {:?}", e);
                return;
            }
        };

        raw.into_iter()
            .map(|(plugin_index, plugin_path, instance_id)| {
                let plugin_name = pm
                    .registered
                    .get(plugin_index)
                    .map(|p| p.name().clone())
                    .unwrap_or_else(|| "unknown".to_string());
                (plugin_index, plugin_name, plugin_path, instance_id)
            })
            .collect()
    };

    // Phase 2: build without lock
    let mut built: Vec<(u64, crate::plugin_manager::manager::PluginHandle)> = Vec::new();
    for (plugin_index, plugin_name, plugin_path, instance_id) in plans {
        let handle_res = match &data {
            Some(d) => PluginManager::build_started_instance_detached_with_data(
                plugin_index,
                plugin_name,
                &plugin_path,
                instance_id,
                d.clone(),
            )
            .await,
            None => PluginManager::build_started_instance_detached(
                plugin_index,
                plugin_name,
                &plugin_path,
                instance_id,
            )
            .await,
        };

        match handle_res {
            Ok(handle) => built.push((instance_id, handle)),
            Err(e) => warn!("build_started_instance_detached failed: {:?}", e),
        }
    }

    // Phase 3: commit under lock
    let mut pm = plugin_manager.lock().await;
    for (instance_id, handle) in built {
        if let Err(e) = pm.commit_fired_event_instance(instance_id, handle) {
            warn!("commit_fired_event_instance failed: {:?}", e);
        }
    }
}

async fn sync_file_added_or_modified(
    storage_manager: &StorageManager,
    plugin_manager: Arc<Mutex<PluginManager>>,
    path: &Path,
) -> Result<(), StorageError> {
    let is_mcap = parsing::file_is_mcap(path).await;
    let is_custom_metadata = parsing::file_is_custom_metadata(path).await?;

    // If this is an MCAP, insert/update the entry in the DB
    if is_mcap {
        if let Err(e) = parsing::insert_entry_into_db(storage_manager, path, plugin_manager.clone()).await {
            error!("Failed to insert/update entry from scan: {:?}", e);
        }
    }

    // If custom metadata file, rescan directory for MCAPs
    if is_custom_metadata {
        if let Some(parent) = path.parent() {
            if let Ok(mut dir) = tokio::fs::read_dir(parent).await {
                while let Ok(Some(ent)) = dir.next_entry().await {
                    let p = ent.path();
                    if parsing::file_is_mcap(&p).await {
                        if let Err(e) = parsing::insert_entry_into_db(storage_manager, &p, plugin_manager.clone()).await {
                            error!("Failed to insert/update entry from metadata scan: {:?}", e);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

async fn sync_file_removed(
    storage_manager: &StorageManager,
    plugin_manager: Arc<Mutex<PluginManager>>, // NEW
    path: &Path,
) {
    let removed_path = path.to_string_lossy().to_string();

    // check if an entry exists for this path
    if let Ok(Some(entry)) = storage_manager
        .get_entry_by_path(removed_path.clone(), storage_manager.start_transaction())
        .await
    {
        let txid = storage_manager.start_transaction();

        // remove topics
        if let Ok(topics_map) = storage_manager.get_topics(entry.id, txid).await {
            for (tid, _t) in topics_map.into_iter() {
                if let Err(e) = storage_manager.remove_topic(tid, txid).await {
                    error!(
                        "Failed to remove topic {} for entry {}: {:?}",
                        tid, entry.id, e
                    );
                }
            }
        }
        // remove sensors
        if let Ok(sensors_map) = storage_manager.get_sensors(entry.id, txid).await {
            for (sid, _s) in sensors_map.into_iter() {
                if let Err(e) = storage_manager.remove_sensor(sid, txid).await {
                    error!(
                        "Failed to remove sensor {} for entry {}: {:?}",
                        sid, entry.id, e
                    );
                }
            }
        }
        // remove sequences
        if let Ok(seqs_map) = storage_manager.get_sequences(entry.id, txid).await {
            for (seqid, _s) in seqs_map.into_iter() {
                if let Err(e) = storage_manager.remove_sequence(entry.id, seqid, txid).await {
                    error!(
                        "Failed to remove sequence {} for entry {}: {:?}",
                        seqid, entry.id, e
                    );
                }
            }
        }

        // finally remove entry row
        let pool = storage_manager.db_connection_pool();
        let entry_id = entry.id;

        let mut deleted_ok = false;

        if let Ok(conn2) = pool.get().await {
            let del_res = conn2
                .interact(move |conn| {
                    diesel::delete(
                        crate::schema::entries::dsl::entries
                            .filter(crate::schema::entries::dsl::id.eq(entry_id)),
                    )
                    .execute(conn)
                })
                .await;

            match del_res {
                Ok(Ok(rows)) => {
                    deleted_ok = rows > 0;
                }
                Ok(Err(e)) => {
                    error!("Failed to remove entry {} from DB: {:?}", entry_id, e);
                }
                Err(e) => {
                    error!("Failed to remove entry {} from DB (interact): {:?}", entry_id, e);
                }
            }
        }

        // Trigger only if DB delete actually happened
        if deleted_ok {
            // Provide payload for plugins (so they don't see empty data on delete)
            let plugin_data = serde_json::json!({
                "metadata": {
                    "time_machine": entry.time_machine,
                    "platform_name": entry.platform_name,
                    "platform_image_link": entry.platform_image_link,
                    "scenario_name": entry.scenario_name,
                    "scenario_creation_time": entry.scenario_creation_time.map(|dt| dt.to_rfc3339()),
                    "scenario_description": entry.scenario_description,
                    "sequence_duration": entry.sequence_duration,
                    "sequence_distance": entry.sequence_distance,
                    "sequence_lat_starting_point_deg": entry.sequence_lat_starting_point_deg,
                    "sequence_lon_starting_point_deg": entry.sequence_lon_starting_point_deg,
                    "weather_cloudiness": entry.weather_cloudiness,
                    "weather_precipitation": entry.weather_precipitation,
                    "weather_precipitation_deposits": entry.weather_precipitation_deposits,
                    "weather_wind_intensity": entry.weather_wind_intensity,
                    "weather_road_humidity": entry.weather_road_humidity,
                    "weather_fog": entry.weather_fog,
                    "weather_snow": entry.weather_snow,
                },
                "mcap_path": removed_path.clone(),
                "event": "deleted",
            })
            .to_string();

            fire_plugin_event(
                plugin_manager.clone(),
                BackendEvent::EntryDeleted {
                    path: removed_path.clone(),
                },
                Some(plugin_data),
            )
            .await;
        }
    }
}

#[instrument]
async fn process_event(
    storage_manager: &StorageManager,
    plugin_manager: Arc<Mutex<PluginManager>>,
    event: &notify::Event,
) -> Result<(), StorageError> {
    let conn = storage_manager.db_connection_pool().get().await?;

    let p0 = match event.paths.get(0) {
        Some(p) => p,
        None => return Ok(()),
    };

    // Ignore temporary files created by seed/import plugins (atomic copy).
    if let Some(name) = p0.file_name().and_then(|s| s.to_str()) {
        if name.ends_with(".partial") {
            return Ok(());
        }
    }

    let path = p0.to_string_lossy().to_string();

    match &event.kind {
        notify::event::EventKind::Create(_) => {
            let is_mcap = parsing::file_is_mcap(p0).await;
            let is_custom_metadata = parsing::file_is_custom_metadata(p0).await?;
            conn.interact(move |conn| {
                diesel::insert_into(files::table)
                    .values(File {
                        path,
                        is_mcap,
                        is_custom_metadata,
                    })
                    .execute(conn)
            })
            .await??;
            let sm = storage_manager.clone();
            let pm = plugin_manager.clone();
            let p = p0.clone();
            if let Err(e) = sync_file_added_or_modified(&sm, pm, &p).await {
                error!("Failed to sync added file {:?}: {:?}", p, e);
            }
        }
        notify::event::EventKind::Access(_) => {}
        notify::event::EventKind::Modify(_) => {
            let is_mcap = parsing::file_is_mcap(p0).await;
            let is_custom_metadata = parsing::file_is_custom_metadata(p0).await?;
            conn.interact(move |conn| {
                diesel::update(files::table.filter(files::path.eq(path)))
                    .set((
                        files::is_mcap.eq(is_mcap),
                        files::is_custom_metadata.eq(is_custom_metadata),
                    ))
                    .execute(conn)
            })
            .await??;
            let sm = storage_manager.clone();
            let pm = plugin_manager.clone();
            let p = p0.clone();
            if let Err(e) = sync_file_added_or_modified(&sm, pm, &p).await {
                error!("Failed to sync modified file {:?}: {:?}", p, e);
            }
        }
        notify::event::EventKind::Remove(_) => {
            if !Path::new(&path).is_dir() {
                conn.interact(move |conn| {
                    diesel::delete(files::table.filter(files::path.eq(path))).execute(conn)
                })
                .await??;

                let sm = storage_manager.clone();
                let pm = plugin_manager.clone();
                let p = p0.clone();
                sync_file_removed(&sm, pm, &p).await;
            }
        }
        _ => {
            warn!("Unhandled event kind: {:?}", event.kind);
        }
    };
    Ok(())
}

#[instrument(skip(fs_event_rx))]
async fn process_events(
    storage_manager: StorageManager,
    plugin_manager: Arc<Mutex<PluginManager>>,
    fs_event_rx: Receiver<notify::Event>,
) {
    debug!("Starting to process file events.");
    ReceiverStream::new(fs_event_rx)
        .for_each_concurrent(10, |event| {
            let storage_manager_clone = storage_manager.clone();
            let plugin_manager_clone = plugin_manager.clone();
            async move {
                process_event(&storage_manager_clone, plugin_manager_clone, &event)
                    .await
                    .unwrap_or_else(|e| {
                        error!("Error processing file event {:?}: {:?}", event, e);
                    });
            }
        })
        .await;
}

#[instrument]
pub async fn start_scanning(
    storage_manager: &StorageManager,
    plugin_manager: Arc<Mutex<PluginManager>>, // NEW
    interval: Duration,
) -> Result<(), Error> {
    debug!("starting filesystem scan method");
    let (fs_event_tx, fs_event_rx) = mpsc::channel(200);

    let fs_event_callback = move |event: Result<notify::Event, notify::Error>| match event {
        Err(e) => {
            error!("notify error during initial scan: {:?}", e);
            return;
        }
        Ok(event) => {
            if event.paths.iter().any(|p| p.is_dir()) {
                return;
            }
            if let Err(e) = fs_event_tx.blocking_send(event) {
                error!("Failed to enqueue fs event: {:?}", e);
            }
        }
    };

    let mut watcher = notify::PollWatcher::new(
        fs_event_callback,
        notify::Config::default().with_poll_interval(interval),
    )?;

    let watch_dir = storage_manager.watch_dir().clone();
    let storage_manager_clone = storage_manager.clone();
    let plugin_manager_clone = plugin_manager.clone();
    tokio::task::spawn(async move {
        debug!("Starting filesystem scan watcher.");
        watcher
            .watch(&watch_dir, RecursiveMode::Recursive)
            .unwrap_or_else(|e| {
                error!("Error on starting filesystem scan {:?}", e);
            });
        debug!("Filesystem scan started.");
        process_events(storage_manager_clone, plugin_manager_clone, fs_event_rx).await;
    });
    debug!("Finished starting filesystem scan method.");
    Ok(())
}

// Does not run in extra thread
pub async fn scan_once(
    storage_manager: &StorageManager,
    plugin_manager: Arc<Mutex<PluginManager>>, // NEW
) -> Result<(), StorageError> {
    debug!("starting one-time filesystem scan");
    let conn = storage_manager.db_connection_pool().get().await?;
    let db_contents: HashSet<_> = conn
        .interact(move |conn| files::table.select(File::as_select()).load::<File>(conn))
        .await??
        .into_iter()
        .map(|f| f.path)
        .collect();
    let dir_contents = WalkDir::new(storage_manager.watch_dir())
        .into_iter()
        .filter_map(|res| match res {
            Ok(entry) => entry
                .path()
                .is_file()
                .then_some(Ok(entry.path().to_string_lossy().to_string())),
            Err(e) => {
                error!("Error reading directory entry: {:?}", e);
                Some(Err(StorageError::IoError(e.into())))
            }
        })
        .collect::<Result<HashSet<String>, StorageError>>()?;

    let to_add: Vec<File> = dir_contents
        .difference(&db_contents)
        .map(|p_string| {
            let path = Path::new(p_string);
            let is_mcap = parsing::file_is_mcap(path);
            let is_custom_metadata = parsing::file_is_custom_metadata(path);
            async move {
                Ok(File {
                    path: p_string.clone(),
                    is_mcap: is_mcap.await,
                    is_custom_metadata: is_custom_metadata.await?,
                })
            }
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<Result<File, StorageError>>>()
        .await
        .into_iter()
        .collect::<Result<Vec<File>, StorageError>>()?;

    let to_remove: Vec<_> = db_contents.difference(&dir_contents).cloned().collect();

    debug!(
        "Files to add: {:?}, files to remove: {:?}",
        to_add, to_remove
    );

    // prepare list of MCAP paths to sync after DB insert
    let mcap_paths: Vec<std::path::PathBuf> = to_add
        .iter()
        .filter(|f| f.is_mcap)
        .map(|f| std::path::PathBuf::from(&f.path))
        .collect();

    conn.interact(move |conn| {
        for file in to_add {
            diesel::insert_into(files::table)
                .values(file)
                .execute(conn)?;
        }
        for path in to_remove {
            diesel::delete(files::table.filter(files::path.eq(path))).execute(conn)?;
        }
        Ok::<(), diesel::result::Error>(())
    })
    .await??;

    // sync newly added MCAP files into entries/topics/etc.
    for p in mcap_paths.into_iter() {
        let sm = storage_manager.clone();
        let pm = plugin_manager.clone();
        if let Err(e) = sync_file_added_or_modified(&sm, pm, &p).await {
            error!("Failed to sync discovered MCAP {:?}: {:?}", p, e);
        }
    }
    // TODO think about when/which files to parse
    Ok(())
}
