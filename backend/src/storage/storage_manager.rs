#![allow(unused)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::Duration,
};

use crate::{
    error::{Error, StorageError},
    storage::entry::Entry,
    storage::{
        metadata::Metadata,
        sequence::{Sequence, SequenceID},
    },
};
use deadpool::Runtime;
use deadpool_diesel::postgres::{Manager, Pool};
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use dotenvy::Iter;
use notify::{
    INotifyWatcher, RecursiveMode, Watcher,
    event::{CreateKind, EventAttributes},
};
use tokio::sync::oneshot;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    watch,
};
use tracing::{debug, error, info, instrument};
use tracing_subscriber::field::debug;

pub type EntryID = u64;
pub type Map<K, V> = std::collections::HashMap<K, V>;
pub type TxID = u64;
pub type Tag = String;

#[derive()]
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
        path: &str,
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
        todo!()
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

    pub async fn process_event(&self, event: &notify::Event) -> Result<(), StorageError> {
        todo!()
    }

    // this is a static method so no method uses mutable access to storage_manager and it can then be shared
    // around the web server so an unnecessary event queue can be skipped
    // while this won't receive create events of directories, if a directory is moved, the remove event still shows up
    // these have to be ignored and can't be distinguished from file events
    // in general there are no rename/move events, jsut create/deletes. They have to be inferred through some other way
    #[instrument]
    async fn process_events(mut fs_event_rx: Receiver<notify::Event>) {
        debug!("Starting to process file events.");
        while let Some(event) = fs_event_rx.recv().await {
            debug!("Received file event: {:?}", event);
            // TODO
        }
    }

    // this both scans the directory on startup and starts the continuous scanning process
    // the notify debouncers can't be used because the the mini-debouncer deletes information about the eventKind
    // and the full-debouncer compacts events on whole directories as one event, which is not desired as each file
    // change has to be processed individually.
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
                debug!("initial scan event: {:?}", event);
                return;
            }
        };

        let mut watcher = notify::PollWatcher::with_initial_scan(
            fs_event_callback,
            notify::Config::default().with_poll_interval(interval),
            initial_scan_callback,
        )?;

        let watch_dir = self.watch_dir().clone();
        tokio::task::spawn(async move {
            debug!("Starting filesystem scan watcher.");
            watcher
                .watch(&watch_dir, RecursiveMode::Recursive)
                .unwrap_or_else(|e| {
                    error!("Error on starting filesystem scan {:?}", e);
                });
            debug!("Filesystem scan started.");
            StorageManager::process_events(fs_event_rx).await;
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
