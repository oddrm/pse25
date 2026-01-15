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
use tokio::sync::mpsc;
use tokio::sync::oneshot;

pub type EntryID = u64;
pub type Map<K, V> = std::collections::HashMap<K, V>;
pub type TxID = u64;
pub type Tag = String;

#[derive(Clone)]
pub struct StorageInstance {}

pub enum Event {
    NewEntry(PathBuf),
    UpdateEntry(PathBuf),
    DeleteEntry(PathBuf),
    UpdateMetadata(PathBuf, Metadata),
    GetMetadata(PathBuf, oneshot::Sender<Option<Metadata>>),
    GetPath(EntryID, oneshot::Sender<Option<PathBuf>>),
    GetSequences(EntryID, oneshot::Sender<Map<SequenceID, Sequence>>),
    GetIterClone(oneshot::Sender<Iter>),
}
// TODO think about importing yaml metadata files
// TODO parallel read access, use multiversion concurrency control, as much as possible sled inbuilt functionality
// TODO make reads and writes always transactional
// TODO rename to records?
// find name for non-file data
// TODO differentiate between file and non-file read/write
// TODO system scan for file integrity
impl StorageInstance {
    pub fn new(path: &PathBuf) -> Result<Self, StorageError> {
        todo!()
    }

    pub fn path(&self) -> &PathBuf {
        todo!()
    }

    pub fn close(self) -> Result<(), StorageError> {
        todo!()
    }

    pub fn get_db_path(&self, id: EntryID, txid: TxID) -> Result<Option<PathBuf>, StorageError> {
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

    pub fn get_event_transmitter(&self) -> mpsc::Sender<Event> {
        todo!()
    }

    pub fn iter(&self) -> Iter {
        todo!()
    }
}

#[derive(Clone)]
pub struct Iter {}

impl Iterator for Iter {
    type Item = EntryID;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
