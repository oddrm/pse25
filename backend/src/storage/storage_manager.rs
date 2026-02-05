#![allow(unused)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::Duration,
};

use crate::schema::files;
use crate::schema::metadata::dsl::{entry_id as metadata_entry_id, metadata};
use crate::storage::models::*;
use crate::{
    error::{Error, StorageError},
    schema,
};

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

// this can be cloned cheaply and still refer to the same db
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

    #[instrument]
    pub fn db_connection_pool(&self) -> &Pool {
        &self.db_connection_pool
    }
    #[instrument]
    pub async fn get_metadata(
        &self,
        id: EntryID,
        txid: TxID,
    ) -> Result<Option<Metadata>, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let result = conn
            .interact(move |conn| {
                metadata
                    .filter(metadata_entry_id.eq(id))
                    .first::<Metadata>(conn)
                    .optional()
            })
            .await??;
        debug!("Queried metadata for entry_id {}: {:?}", id, result);
        Ok(result)
    }
    #[instrument]
    pub async fn update_metadata(
        &self,
        entry_id: EntryID,
        metadata_obj: &Metadata,
        txid: TxID,
    ) -> Result<EntryID, StorageError> {
        todo!()
    }

    #[instrument]
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

    #[instrument]
    pub async fn get_entry(
        &self,
        entry_id: EntryID,
        txid: TxID,
    ) -> Result<Option<Entry>, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let entry = conn
            .interact(move |conn| {
                schema::entries::dsl::entries
                    .find(entry_id)
                    .select(Entry::as_select())
                    .first::<Entry>(conn)
                    .optional()
            })
            .await??;
        debug!("Queried entry by id {}: {:?}", entry_id, entry);
        Ok(entry)
    }

    #[instrument]
    pub async fn get_entry_by_path(
        &self,
        path: String,
        txid: TxID,
    ) -> Result<Option<Entry>, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let entry = conn
            .interact(move |conn| {
                schema::entries::dsl::entries
                    .filter(schema::entries::dsl::path.eq(path))
                    .first::<Entry>(conn)
                    .optional()
            })
            .await??;
        debug!("Queried entry by path: {:?}", entry);
        Ok(entry)
    }

    #[instrument]
    pub async fn get_sequences(
        &self,
        entry_id: EntryID,
        txid: TxID,
    ) -> Result<Map<SequenceID, Sequence>, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let sequences = conn
            .interact(move |conn| {
                schema::sequences::dsl::sequences
                    .filter(schema::sequences::dsl::entry_id.eq(entry_id))
                    .load::<Sequence>(conn)
            })
            .await??;
        let sequences_map = sequences.into_iter().map(|s| (s.id, s)).collect();
        debug!(
            "Queried sequences for entry_id {}: {:?}",
            entry_id, sequences_map
        );
        Ok(sequences_map)
    }

    #[instrument]
    pub async fn add_sequence(
        &self,
        entry_id: EntryID,
        sequence: Sequence,
        txid: TxID,
    ) -> Result<SequenceID, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let sequence_id = conn
            .interact(move |conn| {
                diesel::insert_into(schema::sequences::dsl::sequences)
                    .values(&sequence)
                    .returning(schema::sequences::dsl::id)
                    .get_result::<SequenceID>(conn)
            })
            .await??;
        debug!(
            "Added sequence for entry_id {} with new sequence_id {}",
            entry_id, sequence_id
        );
        Ok(sequence_id)
    }

    #[instrument]
    pub async fn update_sequence(
        &self,
        entry_id: EntryID,
        sequence_id: SequenceID,
        sequence: Sequence,
        txid: TxID,
    ) -> Result<(), StorageError> {
        let conn = self.db_connection_pool().get().await?;
        conn.interact(move |conn| {
            diesel::update(
                schema::sequences::dsl::sequences
                    .filter(schema::sequences::dsl::id.eq(sequence_id))
                    .filter(schema::sequences::dsl::entry_id.eq(entry_id)),
            )
            .set((
                schema::sequences::dsl::description.eq(sequence.description),
                schema::sequences::dsl::start_timestamp.eq(sequence.start_timestamp),
                schema::sequences::dsl::end_timestamp.eq(sequence.end_timestamp),
                schema::sequences::dsl::updated_at.eq(sequence.updated_at),
            ))
            .execute(conn)
        })
        .await??;
        debug!("Updated sequences");
        Ok(())
    }

    #[instrument]
    pub async fn remove_sequence(
        &self,
        entry_id: EntryID,
        sequence_id: SequenceID,
        txid: TxID,
    ) -> Result<(), StorageError> {
        let conn = self.db_connection_pool().get().await?;
        conn.interact(move |conn| {
            diesel::delete(
                schema::sequences::dsl::sequences
                    .filter(schema::sequences::dsl::id.eq(sequence_id))
                    .filter(schema::sequences::dsl::entry_id.eq(entry_id)),
            )
            .execute(conn)
        })
        .await??;
        debug!(
            "Removed sequence with id {} for entry_id {}",
            sequence_id, entry_id
        );
        Ok(())
    }

    #[instrument]
    pub async fn add_tag(
        &self,
        entry_id: EntryID,
        tag: Tag,
        txid: TxID,
    ) -> Result<(), StorageError> {
        let conn = self.db_connection_pool().get().await?;
        conn.interact(move |conn| {
            diesel::insert_into(schema::tags::dsl::tags)
                .values((
                    schema::tags::dsl::entry_id.eq(entry_id),
                    schema::tags::dsl::name.eq(tag),
                ))
                .execute(conn)
        })
        .await??;
        debug!("Added tag for entry_id {}", entry_id);
        Ok(())
    }

    #[instrument]
    pub async fn remove_tag(
        &self,
        entry_id: EntryID,
        tag: Tag,
        txid: TxID,
    ) -> Result<(), StorageError> {
        let conn = self.db_connection_pool().get().await?;
        conn.interact(move |conn| {
            diesel::delete(
                schema::tags::dsl::tags
                    .filter(schema::tags::dsl::entry_id.eq(entry_id))
                    .filter(schema::tags::dsl::name.eq(tag)),
            )
            .execute(conn)
        })
        .await??;
        debug!("Removed tag");
        Ok(())
    }

    #[instrument]
    pub fn get_transaction_id(&self) -> TxID {
        // TODO
        return 0;
    }

    #[instrument]
    pub fn submit_file(
        &self,
        old_path: &PathBuf,
        new_path: &PathBuf,
        txid: TxID,
    ) -> Result<(), StorageError> {
        todo!()
    }

    #[instrument]
    pub fn end_transaction(&self, txid: TxID) -> Result<(), StorageError> {
        // TODO
        Ok(())
    }
}

impl std::fmt::Debug for StorageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageInstance")
            .field("watch_dir", &self.watch_dir)
            .finish()
    }
}
