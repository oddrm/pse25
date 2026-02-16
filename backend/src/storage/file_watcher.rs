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

async fn sync_file_added_or_modified(
    storage_manager: &StorageManager,
    path: &Path,
) -> Result<(), StorageError> {
    let is_mcap = parsing::file_is_mcap(path).await;
    let is_custom_metadata = parsing::file_is_custom_metadata(path).await?;

    // If this is an MCAP, insert/update the entry in the DB
    if is_mcap {
        if let Err(e) = parsing::insert_entry_into_db(storage_manager, path).await {
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
                        if let Err(e) = parsing::insert_entry_into_db(storage_manager, &p).await {
                            error!("Failed to insert/update entry from metadata scan: {:?}", e);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

async fn sync_file_removed(storage_manager: &StorageManager, path: &Path) {
    let removed_path = path.to_string_lossy().to_string();
    // check if an entry exists for this path
    if let Ok(Some(entry)) = storage_manager
        .get_entry_by_path(removed_path.clone(), storage_manager.get_transaction_id())
        .await
    {
        let txid = storage_manager.get_transaction_id();
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
        if let Ok(conn2) = pool.get().await {
            if let Err(e) = conn2
                .interact(move |conn| {
                    diesel::delete(
                        crate::schema::entries::dsl::entries
                            .filter(crate::schema::entries::dsl::id.eq(entry_id)),
                    )
                    .execute(conn)
                })
                .await
            {
                error!("Failed to remove entry {} from DB: {:?}", entry_id, e);
            }
        }
    }
}

#[instrument]
async fn process_event(
    storage_manager: &StorageManager,
    event: &notify::Event,
) -> Result<(), StorageError> {
    let conn = storage_manager.db_connection_pool().get().await?;

    let path = event.paths[0].to_string_lossy().to_string();
    match &event.kind {
        notify::event::EventKind::Create(_) => {
            let is_mcap = parsing::file_is_mcap(&event.paths[0]).await;
            let is_custom_metadata = parsing::file_is_custom_metadata(&event.paths[0]).await?;
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
            let p = event.paths[0].clone();
            if let Err(e) = sync_file_added_or_modified(&sm, &p).await {
                error!("Failed to sync added file {:?}: {:?}", p, e);
            }
        }
        notify::event::EventKind::Access(_) => {}
        notify::event::EventKind::Modify(_) => {
            let is_mcap = parsing::file_is_mcap(&event.paths[0]).await;
            let is_custom_metadata = parsing::file_is_custom_metadata(&event.paths[0]).await?;
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
            let p = event.paths[0].clone();
            if let Err(e) = sync_file_added_or_modified(&sm, &p).await {
                error!("Failed to sync modified file {:?}: {:?}", p, e);
            }
        }

        notify::event::EventKind::Remove(_) => {
            // while this won't receive create events of directories, if a directory is moved, the remove
            // event still shows up these have to be ignored and can't be distinguished from file events
            // in general there are no rename/move events, just create/ deletes.
            if !Path::new(&path).is_dir() {
                // remove from files table
                conn.interact(move |conn| {
                    diesel::delete(files::table.filter(files::path.eq(path))).execute(conn)
                })
                .await??;

                let sm = storage_manager.clone();
                let p = event.paths[0].clone();
                sync_file_removed(&sm, &p).await;
            }
        }
        _ => {
            warn!("Unhandled event kind: {:?}", event.kind);
        }
    };
    Ok(())
}

#[instrument(skip(fs_event_rx))]
async fn process_events(storage_manager: StorageManager, fs_event_rx: Receiver<notify::Event>) {
    debug!("Starting to process file events.");
    // up to 10 simultaneous process event tasks
    ReceiverStream::new(fs_event_rx)
        .for_each_concurrent(10, |event| {
            let storage_manager_clone = storage_manager.clone();
            async move {
                process_event(&storage_manager_clone, &event)
                    .await
                    .unwrap_or_else(|e| {
                        error!("Error processing file event {:?}: {:?}", event, e);
                    });
            }
        })
        .await;
}

// this both scans the directory on startup and starts the continuous scanning process
// the notify debouncers can't be used because the the mini-debouncer deletes information
// about the eventKind and the full-debouncer compacts events on whole directories as one event,
// which is not desired as each file change has to be processed individually.
#[instrument]
pub async fn start_scanning(
    storage_manager: &StorageManager,
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
    tokio::task::spawn(async move {
        debug!("Starting filesystem scan watcher.");
        watcher
            .watch(&watch_dir, RecursiveMode::Recursive)
            .unwrap_or_else(|e| {
                error!("Error on starting filesystem scan {:?}", e);
            });
        debug!("Filesystem scan started.");
        process_events(storage_manager_clone, fs_event_rx).await;
    });
    debug!("Finished starting filesystem scan method.");
    Ok(())
}

// Does not run in extra thread
pub async fn scan_once(storage_manager: &StorageManager) -> Result<(), StorageError> {
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
        if let Err(e) = sync_file_added_or_modified(&sm, &p).await {
            error!("Failed to sync discovered MCAP {:?}: {:?}", p, e);
        }
    }
    // TODO think about when/which files to parse
    Ok(())
}
