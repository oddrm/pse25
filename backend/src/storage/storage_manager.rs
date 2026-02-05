#![allow(unused)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::Duration,
};

use crate::error::{Error, StorageError};
use crate::schema::{files, sequences};
use crate::storage::models::*;
use deadpool::Runtime;
use deadpool_diesel::postgres::{Manager, Pool};
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use dotenvy::Iter;
use notify::{
    RecursiveMode, Watcher,
    event::{CreateKind, EventAttributes},
};
use rocket::futures::{FutureExt, StreamExt};
use tokio::sync::oneshot;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    watch,
};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info, instrument};
use tracing_subscriber::field::debug;

pub type Map<K, V> = std::collections::HashMap<K, V>;
pub type TxID = u64;
pub type Tag = String;

// this can be cloned and still refer to the same db
#[derive(Clone)]
pub struct StorageManager {
    db_connection_pool: Pool,
    watch_dir: PathBuf,
}

impl StorageManager {
    #[instrument]
    pub fn new(url: &String) -> Result<Self, StorageError> {
        let manager = Manager::new(url, Runtime::Tokio1);
        let pool = Pool::builder(manager)
            .max_size(10)
            .build()
            .map_err(|e| StorageError::CustomError(e.to_string()))?;

        info!("initialized storage");
        Ok(StorageManager {
            db_connection_pool: pool,
            // this only refers to the directory inside the docker container
            watch_dir: PathBuf::from("/data"),
        })
    }

    pub fn watch_dir(&self) -> &PathBuf {
        &self.watch_dir
    }

    pub fn db_pool(&self) -> &Pool {
        &self.db_connection_pool
    }

    pub fn close(self) -> Result<(), StorageError> {
        todo!()
    }

    pub async fn get_metadata(
        &self,
        id: EntryID,
        txid: TxID,
    ) -> Result<Option<Metadata>, StorageError> {
        todo!()
    }

    pub async fn update_metadata(
        &self,
        id: EntryID,
        metadata: &Metadata,
        txid: TxID,
    ) -> Result<EntryID, StorageError> {
        todo!()
    }

    pub async fn get_entries(
        &self,
        search_string: Option<String>,
        sort_by: Option<String>,
        ascending: Option<bool>,
        page: Option<u32>,
        page_size: Option<u32>,
        txid: TxID,
    ) -> Result<Vec<(EntryID, Metadata)>, StorageError> {
        todo!()
    }

    pub async fn get_entry(&self, id: EntryID, txid: TxID) -> Result<Option<Entry>, StorageError> {
        todo!()
    }

    pub async fn get_entry_by_path(
        &self,
        path: &String,
        txid: TxID,
    ) -> Result<Option<Entry>, StorageError> {
        todo!()
    }

    pub async fn get_sequences(
        &self,
        id: EntryID,
        txid: TxID,
    ) -> Result<Map<SequenceID, Sequence>, StorageError> {
        todo!()
    }

    pub async fn add_sequence(
        &self,
        entry_id: EntryID,
        sequence: Sequence,
        txid: TxID,
    ) -> Result<SequenceID, StorageError> {
        todo!()
    }

    pub async fn update_sequence(
        &self,
        entry_id: EntryID,
        sequence_id: SequenceID,
        sequence: Sequence,
        txid: TxID,
    ) -> Result<(), StorageError> {
        let conn = self.db_connection_pool.get().await?;
        
        conn.interact(move |conn| {
            diesel::update(sequences::table)
                .filter(sequences::id.eq(sequence_id))
                .filter(sequences::entry_id.eq(entry_id))
                .set((
                    sequences::description.eq(&sequence.description),
                    sequences::start_timestamp.eq(sequence.start_timestamp),
                    sequences::end_timestamp.eq(sequence.end_timestamp),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| StorageError::CustomError(e.to_string()))?
        .map_err(|e| StorageError::CustomError(e.to_string()))?;
        
        Ok(())
    }

    pub async fn remove_sequence(
        &self,
        entry_id: EntryID,
        sequence_id: SequenceID,
        txid: TxID,
    ) -> Result<(), StorageError> {
        todo!()
    }

    pub async fn add_tag(&self, id: EntryID, tag: Tag, txid: TxID) -> Result<(), StorageError> {
        todo!()
    }

    pub async fn remove_tag(&self, id: EntryID, tag: Tag, txid: TxID) -> Result<(), StorageError> {
        todo!()
    }

    pub fn get_transaction_id(&self) -> TxID {
        todo!()
    }

    #[instrument]
    pub async fn process_event(&self, event: &notify::Event) -> Result<(), StorageError> {
        debug!("Processing file event: {:?}", event);
        let conn = self.db_connection_pool.get().await?;
        match &event.kind {
            notify::event::EventKind::Create(_) => {
                let now = chrono::Utc::now().timestamp();
                let naive_datetime = chrono::DateTime::from_timestamp(now, 0)
                    .unwrap()
                    .naive_utc();

                let path = event.paths[0].to_string_lossy().to_string();
                let x = conn
                    .interact(move |conn| {
                        diesel::insert_into(files::table)
                            .values(File {
                                created: naive_datetime,
                                last_checked: naive_datetime,
                                last_modified: naive_datetime,
                                path,
                                size: 0,
                            })
                            .returning(File::as_returning())
                            .get_result(conn)
                    })
                    .await
                    .map_err(|e| StorageError::CustomError(e.to_string()))?
                    .map_err(|e| StorageError::CustomError(e.to_string()))?;
            }
            _ => {}
        };
        Ok(())
    }

    // this is a static method so no method uses mutable access to storage_manager and it can then
    // be shared around the web server so an unnecessary event queue can be skipped
    // while this won't receive create events of directories, if a directory is moved, the remove
    // event still shows up these have to be ignored and can't be distinguished from file events
    // in general there are no rename/move events, just create/ deletes.
    // They have to be inferred through some other way
    #[instrument]
    async fn process_events(self, fs_event_rx: Receiver<notify::Event>) {
        debug!("Starting to process file events.");
        // up to 10 simultaneous process event tasks
        ReceiverStream::new(fs_event_rx)
            .for_each_concurrent(10, |event| {
                let self_clone = self.clone();
                async move {
                    self_clone.process_event(&event).await.unwrap_or_else(|e| {
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
    pub fn start_scanning(&self, interval: Duration) -> Result<(), Error> {
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

        let initial_scan_callback = move |event: Result<PathBuf, notify::Error>| {
            if !event.as_ref().unwrap().is_dir() {
                // debug!("initial scan event: {:?}", event);
                return;
            }
        };

        let mut watcher = notify::PollWatcher::with_initial_scan(
            fs_event_callback,
            notify::Config::default().with_poll_interval(interval),
            initial_scan_callback,
        )?;

        let watch_dir = self.watch_dir().clone();
        let self_clone = self.clone();
        tokio::task::spawn(async move {
            debug!("Starting filesystem scan watcher.");
            watcher
                .watch(&watch_dir, RecursiveMode::Recursive)
                .unwrap_or_else(|e| {
                    error!("Error on starting filesystem scan {:?}", e);
                });
            debug!("Filesystem scan started.");
            StorageManager::process_events(self_clone, fs_event_rx).await;
        });
        Ok(())
    }

    pub fn submit_file(
        &self,
        old_path: &PathBuf,
        new_path: &PathBuf,
        txid: TxID,
    ) -> Result<(), StorageError> {
        todo!()
    }

    pub fn end_transaction(&self, txid: TxID) -> Result<(), StorageError> {
        todo!()
    }
}

impl std::fmt::Debug for StorageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageInstance")
            .field("watch_dir", &self.watch_dir)
            .finish()
    }
}
