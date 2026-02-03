#![allow(unused)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::Duration,
};

use crate::error::{Error, StorageError};
use crate::schema::files;
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
use tracing::{debug, error, info, instrument, warn};
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

    pub fn db_connection_pool(&self) -> &Pool {
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
