use std::collections::HashSet;
use std::path::PathBuf;
use std::{path::Path, time::Duration};

use crate::schema::files;
use crate::storage::models::*;
use crate::{
    error::{Error, StorageError},
    storage::{parsing, storage_manager::StorageManager},
};
use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use rocket::futures::StreamExt;
use tokio::time;
use tracing::{debug, error, instrument};
use tracing_subscriber::field::debug;
use walkdir::WalkDir;

const MODIFIED_GRACE_PERIOD: Duration = Duration::from_secs(10);

async fn sync_file_added_or_modified(
    storage_manager: &StorageManager,
    path: &Path,
) -> Result<(), StorageError> {
    let is_mcap = parsing::file_is_mcap(path);
    let is_custom_metadata = parsing::file_is_custom_metadata(path).await?;
    debug!(
        "Syncing added/modified file {:?}, is_mcap: {}, is_custom_metadata: {}",
        path, is_mcap, is_custom_metadata
    );
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
                    if parsing::file_is_mcap(&p) {
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

// NOTE: we intentionally avoid using `notify` for continuous watching.
// Instead we periodically call `scan_once` on a background task at the
// provided `interval`. This keeps behavior similar to the previous
// poll-based watcher but centralizes logic in `scan_once`.

// this both scans the directory on startup and starts the continuous scanning process
// the notify debouncers can't be used because the the mini-debouncer deletes information
// about the eventKind and the full-debouncer compacts events on whole directories as one event,
// which is not desired as each file change has to be processed individually.
#[instrument]
pub async fn start_scanning(
    storage_manager: &StorageManager,
    interval: Duration,
) -> Result<(), Error> {
    debug!("starting filesystem scan method (periodic)");
    let storage_manager_clone = storage_manager.clone();
    tokio::task::spawn(async move {
        debug!("Starting periodic filesystem scanner.");
        // run one immediate scan on startup
        if let Err(e) = scan_once(&storage_manager_clone).await {
            error!("Initial scan failed: {:?}", e);
        }
        let mut ticker = time::interval(interval);
        loop {
            ticker.tick().await;
            if let Err(e) = scan_once(&storage_manager_clone).await {
                error!("Periodic scan failed: {:?}", e);
            }
        }
    });
    debug!("Finished starting filesystem scan method.");
    Ok(())
}

// Does not run in extra thread
pub async fn scan_once(storage_manager: &StorageManager) -> Result<(), StorageError> {
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

    let to_add: Vec<File> = {
        let stream =
            rocket::futures::stream::iter(dir_contents.difference(&db_contents).cloned().map(
                async move |p_clone| {
                    let path = PathBuf::from(&p_clone);
                    let is_mcap = parsing::file_is_mcap(&path);
                    let is_custom_metadata = parsing::file_is_custom_metadata(&path).await;
                    Ok(File {
                        path: p_clone.clone(),
                        is_mcap,
                        is_custom_metadata: is_custom_metadata?,
                    })
                },
            ));
        let results: Vec<Result<File, StorageError>> = stream.buffer_unordered(10).collect().await;
        results
            .into_iter()
            .collect::<Result<Vec<File>, StorageError>>()?
    };

    let to_remove: Vec<_> = db_contents.difference(&dir_contents).cloned().collect();

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

    // Re-check potentially modified MCAP files that exist both in DB and on disk.
    // Compare filesystem modification time to the entry's `updated_at` and
    // re-run sync if the file is newer than the stored entry.
    let intersection: Vec<String> = db_contents.intersection(&dir_contents).cloned().collect();

    for p in intersection.into_iter() {
        let pathbuf = PathBuf::from(&p);
        // only consider MCAP files
        if !parsing::file_is_mcap(&pathbuf) {
            continue;
        }

        match tokio::fs::metadata(&pathbuf).await {
            Ok(meta) => {
                let file_size = meta.len() as i64;
                let mtime_dt_opt: Option<DateTime<Utc>> = match meta.modified() {
                    Ok(mtime_sys) => Some(DateTime::<Utc>::from(mtime_sys)),
                    Err(e) => {
                        error!("Failed to get modified time for {:?}: {:?}", pathbuf, e);

                        None
                    }
                };

                // fetch entry by path
                if let Ok(Some(entry)) = storage_manager
                    .get_entry_by_path(p.clone(), storage_manager.start_transaction())
                    .await
                {
                    // Re-sync when file size changed (handles copy completion where mtime may be older),
                    // or when mtime is newer than DB updated_at.
                    let mtime_newer = mtime_dt_opt.map_or(false, |t| t > entry.updated_at);
                    if file_size != entry.size || mtime_newer {
                        let sm = storage_manager.clone();
                        if let Err(e) = sync_file_added_or_modified(&sm, &pathbuf).await {
                            error!("Failed to re-sync modified MCAP {:?}: {:?}", pathbuf, e);
                        }
                    }
                }
            }
            Err(e) => error!("Failed to stat file {:?}: {:?}", pathbuf, e),
        }
    }
    Ok(())
}
