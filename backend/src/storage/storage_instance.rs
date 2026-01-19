#![allow(unused)]

use std::{path::PathBuf, time::Duration};

use crate::{
    error::{Error, StorageError},
    storage::entry::Entry,
    storage::{
        metadata::Metadata,
        sequence::{Sequence, SequenceID},
    },
};
use diesel::prelude::*;
use dotenvy::Iter;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;
use tracing::{info, instrument};

pub type EntryID = u64;
pub type Map<K, V> = std::collections::HashMap<K, V>;
pub type TxID = u64;
pub type Tag = String;

#[derive()]
pub struct StorageInstance {
    db_connection: PgConnection,
    event_sender: Sender<Event>,
    event_receiver: Receiver<Event>,
}

pub enum Event {
    NewEntry(PathBuf),
    UpdateEntry(PathBuf),
    DeleteEntry(PathBuf),
    UpdateMetadata(PathBuf, Metadata),
    GetMetadata(PathBuf, oneshot::Sender<Option<Metadata>>),
    GetPath(EntryID, oneshot::Sender<Option<PathBuf>>),
    GetSequences(EntryID, oneshot::Sender<Map<SequenceID, Sequence>>),
}

impl StorageInstance {
    // does not use a path to the db anymore but instead a database url
    #[instrument]
    pub fn new(url: &String) -> Result<Self, StorageError> {
        let db_connection = PgConnection::establish(url)?;
        info!("DB connection established.");
        // eventually decide on how much buffer is enough
        let (event_sender, event_receiver) = mpsc::channel(200);
        Ok(StorageInstance {
            db_connection,
            event_receiver,
            event_sender,
        })
    }

    pub fn path(&self) -> &PathBuf {
        todo!()
    }

    pub fn close(self) -> Result<(), StorageError> {
        todo!()
    }

    pub fn get_db_url(&self, id: EntryID) -> Result<Option<String>, StorageError> {
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

    pub async fn process_event(&mut self, event: &Event) -> Result<(), StorageError> {
        todo!()
    }

    pub async fn process_events(&mut self) -> Result<(), StorageError> {
        todo!()
    }

    pub async fn scan_once(&mut self) -> Result<(), Error> {
        todo!()
    }

    // only starts sending fs events into queue, events still have to be processed somewhere else
    pub fn start_scanning(&mut self, interval: &Duration) -> Result<(), Error> {
        todo!()
    }

    pub fn stop_scanning(&mut self) -> Result<(), Error> {
        todo!()
    }

    pub fn get_event_transmitter(&self) -> Sender<Event> {
        self.event_sender.clone()
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
