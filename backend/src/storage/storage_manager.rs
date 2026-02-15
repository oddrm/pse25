#![allow(unused)]

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
        Mutex,
    },
    thread,
    time::Duration,
};

use crate::schema::files;
// use crate::schema::metadata::dsl::{entry_id as metadata_entry_id, metadata};
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
use rayon::iter::{IntoParallelIterator, ParallelIterator};
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
fn contains_part(value: &str, part: &str) -> bool {
    value.to_lowercase().contains(part)
}
fn opt_contains(opt: &Option<String>, part: &str) -> bool {
    opt.as_deref()
        .map(|s| contains_part(s, part))
        .unwrap_or(false)
}
/// Returns true if this entry matches the search: every word in `search_parts` must appear
/// in at least one of the entry's string fields (name, path, tags, scenario_name, etc.).
fn entry_matches_search(entry: &Entry, search_parts: &[String]) -> bool {
    for part in search_parts {
        let matches = contains_part(&entry.name, part)
            || contains_part(&entry.path, part)
            || opt_contains(&entry.platform_name, part)
            || opt_contains(&entry.scenario_name, part)
            || opt_contains(&entry.scenario_description, part)
            || opt_contains(&entry.weather_cloudiness, part)
            || opt_contains(&entry.weather_precipitation, part)
            || opt_contains(&entry.weather_precipitation_deposits, part)
            || opt_contains(&entry.weather_wind_intensity, part)
            || opt_contains(&entry.weather_road_humidity, part)
            || entry.tags.iter().any(|t| contains_part(t, part))
            || entry.topics.iter().any(|t| contains_part(t, part));
        if !matches {
            return false;
        }
    }
    true
}


#[derive(Clone)]
pub struct StorageManager {
    db_connection_pool: Pool,
    watch_dir: PathBuf,
    tx_counter: Arc<AtomicU64>,
    /// Set of transaction IDs that have been started but not yet ended.
    active_transactions: Arc<Mutex<HashSet<TxID>>>,
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
            tx_counter: Arc::new(AtomicU64::new(0)),
            active_transactions: Arc::new(Mutex::new(HashSet::new())),
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
    ) -> Result<Option<Entry>, StorageError> {
        // storage manager works with Entry; return the Entry directly
        let entry = self.get_entry(id, txid).await?;
        Ok(entry)
    }

    #[instrument]
    pub async fn update_entry(
        &self,
        entry_id: EntryID,
        entry_obj: &Entry,
        txid: TxID,
    ) -> Result<(), StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let cloned = entry_obj.clone();
        conn.interact(move |conn| {
            diesel::update(
                schema::entries::dsl::entries.filter(schema::entries::dsl::id.eq(entry_id)),
            )
            .set(&cloned)
            .execute(conn)
        })
        .await??;
        debug!("Updated entry {}", entry_id);
        Ok(())
    }

    #[instrument]
    pub async fn get_entries(
        &self,
        search_string: Option<String>,
        _sort_by: Option<String>,
        _ascending: Option<bool>,
        _page: Option<u32>,
        _page_size: Option<u32>,
        txid: TxID,
    ) -> Result<Vec<Entry>, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let entries = conn
            .interact(move |conn| {
                schema::entries::dsl::entries
                    .select(Entry::as_select())
                    .load::<Entry>(conn)
            })
            .await??;
        debug!("Queried entries: {}", entries.len());
        // --- Search logic (only when search_string is provided) ---
        // 1. Split the search string by whitespace into "words", lowercased.
        //    Example: "  Rain  Highway  " → ["rain", "highway"]
        let search_parts: Vec<String> = match search_string.as_deref() {
            None | Some("") => return Ok(entries),  // no search → return all entries
            Some(s) => s
                .split_whitespace()
                .map(|p| p.to_lowercase())
                .collect(),
        };

        if search_parts.is_empty() {
            return Ok(entries);
        }

        // 2. Keep only entries where every search word appears in at least one field.
        //    We look in: name, path, platform_name, scenario_name, scenario_description,
        let filtered: Vec<Entry> = entries
            .into_par_iter()
            .filter(|e| entry_matches_search(e, &search_parts))
            .collect();

        if filtered.is_empty() {
            return Err(StorageError::NotFound(format!(
                "no entries match search '{}'",
                search_parts.join(" ")
            )));
        }

        Ok(filtered)
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
        let t = tag.clone();
        conn.interact(move |conn| {
            diesel::sql_query("UPDATE entries SET tags = array_append(tags, $1) WHERE id = $2 AND NOT ($1 = ANY(tags))")
                .bind::<diesel::sql_types::Text,_>(t)
                .bind::<diesel::sql_types::BigInt,_>(entry_id)
                .execute(conn)
        }).await??;
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
        let t = tag.clone();
        conn.interact(move |conn| {
            diesel::sql_query("UPDATE entries SET tags = array_remove(tags, $1) WHERE id = $2")
                .bind::<diesel::sql_types::Text, _>(t)
                .bind::<diesel::sql_types::BigInt, _>(entry_id)
                .execute(conn)
        })
        .await??;
        debug!("Removed tag");
        Ok(())
    }

    #[instrument]
    pub fn get_transaction_id(&self) -> TxID {
        let txid = self.tx_counter.fetch_add(1, Ordering::Relaxed);
        self.active_transactions
            .lock()
            .expect("active_transactions lock")
            .insert(txid);
        txid
    }

    #[instrument]
    pub fn submit_file(
        &self,
        old_path: &PathBuf,
        new_path: &PathBuf,
        txid: TxID,
    ) -> Result<(), StorageError> {

        // I do not understand this one?
        todo!()
    }

  
    #[instrument]
    pub fn end_transaction(&self, txid: TxID) -> Result<(), StorageError> {
        let removed = self
            .active_transactions
            .lock()
            .map_err(|e| StorageError::CustomError(e.to_string()))?
            .remove(&txid);
        if removed {
            debug!("Ended transaction {}", txid);
            Ok(())
        } else {
            Err(StorageError::NotFound(format!(
                "transaction {} not found or already ended",
                txid
            )))
        }
    }
}

impl std::fmt::Debug for StorageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageInstance")
            .field("watch_dir", &self.watch_dir)
            .finish()
    }
}
