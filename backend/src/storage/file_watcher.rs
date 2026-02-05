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
            // TODO trigger file scan
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
            // TODO trigger file re-scan
        }

        notify::event::EventKind::Remove(_) => {
            // while this won't receive create events of directories, if a directory is moved, the remove
            // event still shows up these have to be ignored and can't be distinguished from file events
            // in general there are no rename/move events, just create/ deletes.
            if !Path::new(&path).is_dir() {
                conn.interact(move |conn| {
                    diesel::delete(files::table.filter(files::path.eq(path))).execute(conn)
                })
                .await??;
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
pub fn start_scanning(storage_manager: &StorageManager, interval: Duration) -> Result<(), Error> {
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
            let _ = fs_event_tx.blocking_send(event);
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
    // TODO think about when/which files to parse
    Ok(())
}
