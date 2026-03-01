#![allow(unused)]

use crate::{
    routes,
    schema::{files, sensors::entry_id},
};
use itertools::Itertools;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};
// use crate::schema::metadata::dsl::{entry_id as metadata_entry_id, metadata};
use crate::storage::models::*;
use crate::{
    error::{Error, StorageError},
    schema,
};

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
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
pub type TopicID = i64;
fn contains_part(value: &str, part: &str) -> bool {
    value.to_lowercase().contains(part)
}
fn opt_contains(opt: &Option<String>, part: &str) -> bool {
    opt.as_deref()
        .map(|s| contains_part(s, part))
        .unwrap_or(false)
}

/// Tries to parse a search term as a date/timestamp. Supports:
/// - Date only YYYY-MM-DD (e.g. "2024-01-15")
/// - ISO 8601 datetime (e.g. "2024-01-15T10:30:00Z")
fn parse_search_date(part: &str) -> Option<DateTime<Utc>> {
    if let Ok(secs) = part.parse::<i64>() {
        if let Some(dt) = Utc.timestamp_opt(secs, 0).single() {
            return Some(dt);
        }
    }
    if let Ok(date) = NaiveDate::parse_from_str(part, "%Y-%m-%d") {
        if let Some(ndt) = date.and_hms_opt(0, 0, 0) {
            return Some(Utc.from_utc_datetime(&ndt));
        }
    }
    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(part, "%Y-%m-%dT%H:%M:%S") {
        return Some(Utc.from_utc_datetime(&ndt));
    }
    None
}

// Returns true if the entry has any timestamp on the same calendar day as date_time.
fn entry_matches_date(entry: &Entry, date_time: &DateTime<Utc>) -> bool {
    let search_date = date_time.date_naive();
    entry.created_at.date_naive() == search_date
        || entry.updated_at.date_naive() == search_date
        || entry
            .scenario_creation_time
            .as_ref()
            .map(|time| time.date_naive() == search_date)
            .unwrap_or(false)
}

/// Returns true if this entry matches the search: every word in `search_parts` must appear
/// in at least one of the entry's string fields, or (if the word is a valid date) match
/// one of the entry's date fields (created_at, updated_at, scenario_creation_time).
fn entry_matches_search(entry: &Entry, search_parts: &[String]) -> bool {
    for part in search_parts {
        if let Some(search_dt) = parse_search_date(part) {
            if !entry_matches_date(entry, &search_dt) {
                return false;
            }
            continue;
        }
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
            || entry.tags.iter().any(|t| contains_part(t, part));
        //  || entry.topics.iter().any(|t| contains_part(t, part));
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
        entry_id_: EntryID,
        entry_metadata: routes::database::MetadataWeb,
        txid: TxID,
    ) -> Result<(), StorageError> {
        let conn = self.db_connection_pool().get().await?;
        conn.interact(move |conn| {
            diesel::update(
                schema::entries::dsl::entries.filter(schema::entries::dsl::id.eq(entry_id_)),
            )
            .set((
                schema::entries::dsl::time_machine.eq(entry_metadata.time_machine),
                schema::entries::dsl::platform_name.eq(entry_metadata.platform_name.clone()),
                schema::entries::dsl::platform_image_link
                    .eq(entry_metadata.platform_image_link.clone()),
                schema::entries::dsl::scenario_name.eq(entry_metadata.scenario_name.clone()),
                schema::entries::dsl::scenario_creation_time
                    .eq(entry_metadata.scenario_creation_time),
                schema::entries::dsl::scenario_description
                    .eq(entry_metadata.scenario_description.clone()),
                schema::entries::dsl::sequence_duration.eq(entry_metadata.sequence_duration),
                schema::entries::dsl::sequence_distance.eq(entry_metadata.sequence_distance),
                schema::entries::dsl::sequence_lat_starting_point_deg
                    .eq(entry_metadata.sequence_lat_starting_point_deg),
                schema::entries::dsl::sequence_lon_starting_point_deg
                    .eq(entry_metadata.sequence_lon_starting_point_deg),
                schema::entries::dsl::weather_cloudiness
                    .eq(entry_metadata.weather_cloudiness.clone()),
                schema::entries::dsl::weather_precipitation
                    .eq(entry_metadata.weather_precipitation.clone()),
                schema::entries::dsl::weather_precipitation_deposits
                    .eq(entry_metadata.weather_precipitation_deposits.clone()),
                schema::entries::dsl::weather_wind_intensity
                    .eq(entry_metadata.weather_wind_intensity.clone()),
                schema::entries::dsl::weather_road_humidity
                    .eq(entry_metadata.weather_road_humidity.clone()),
                schema::entries::dsl::weather_fog.eq(entry_metadata.weather_fog),
                schema::entries::dsl::weather_snow.eq(entry_metadata.weather_snow),
            ))
            .execute(conn)
        })
        .await??;

        debug!("Updated entry {}", entry_id_);
        Ok(())
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
    ) -> Result<Vec<Entry>, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let entries = conn
            .interact(move |conn| {
                schema::entries::dsl::entries
                    .select(Entry::as_select())
                    .load::<Entry>(conn)
            })
            .await??;
        // debug!("Queried all entries, count: {}", entries.len());
        // 1. Optional search: filter by search string when provided
        let search_parts: Vec<String> = match search_string.as_deref() {
            None | Some("") => Vec::new(),
            Some(s) => s.split_whitespace().map(|p| p.to_lowercase()).collect(),
        };
        // debug!("Search parts: {:?}", search_parts);
        let filtered: Vec<Entry> = if search_parts.is_empty() {
            entries
        } else {
            entries
                .into_iter()
                .filter(|e| entry_matches_search(e, &search_parts))
                .collect()
        };
        // debug!("Filtered entries count after search: {}", filtered.len());
        // 2. Sort (always applied
        let mut sorted: Vec<Entry> = filtered
            .into_iter()
            .sorted_by(|a, b| match sort_by.as_deref() {
                Some("Name") => Ord::cmp(&a.name, &b.name),
                Some("Path") => Ord::cmp(&a.path, &b.path),
                Some("Size") => Ord::cmp(&a.size, &b.size),
                Some("Platform") => Ord::cmp(&a.platform_name, &b.platform_name),
                _ => Ord::cmp(&a.name, &b.name),
            })
            .collect();
        // debug!("Sorted entries: {:?}", sorted);
        // 3. Ascending / descending
        if ascending.is_some_and(|a| !a) {
            sorted.reverse();
        }
        debug!("Applied ascending/descending");
        // 4. Paging
        let paged: Vec<Entry> = match (page, page_size) {
            (Some(p), Some(ps)) if ps > 0 => {
                let start = (p as usize).saturating_mul(ps as usize);
                // debug!("Applying paging: start index {}, page size {}", start, ps);
                sorted.into_iter().skip(start).take(ps as usize).collect()
            }
            _ => sorted,
        };
        // debug!("Applied paging: page {:?}, page_size {:?}", page, page_size);
        // debug!(
        // "Final entries count after filtering, sorting, and paging: {}",
        // paged.len()
        // );
        Ok(paged)
    }

    #[instrument]
    pub async fn get_entry(
        &self,
        entry_id_: EntryID,
        txid: TxID,
    ) -> Result<Option<Entry>, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let entry = conn
            .interact(move |conn| {
                schema::entries::dsl::entries
                    .find(entry_id_)
                    .select(Entry::as_select())
                    .first::<Entry>(conn)
                    .optional()
            })
            .await??;
        debug!("Queried entry by id {}: {:?}", entry_id_, entry);
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
                    .select(Entry::as_select())
                    .first::<Entry>(conn)
                    .optional()
            })
            .await??;
        // debug!("Queried entry by path: {:?}", entry);
        Ok(entry)
    }

    #[instrument]
    pub async fn get_sequences(
        &self,
        entry_id_: EntryID,
        txid: TxID,
    ) -> Result<Map<SequenceID, Sequence>, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let sequences = conn
            .interact(move |conn| {
                schema::sequences::dsl::sequences
                    .filter(schema::sequences::dsl::entry_id.eq(entry_id_))
                    .select(Sequence::as_select())
                    .load::<Sequence>(conn)
            })
            .await??;
        let sequences_map = sequences.into_iter().map(|s| (s.id, s)).collect();
        debug!(
            "Queried sequences for entry_id {}: {:?}",
            entry_id_, sequences_map
        );
        Ok(sequences_map)
    }

    #[instrument]
    pub async fn get_sensors(
        &self,
        entry_id_: EntryID,
        txid: TxID,
    ) -> Result<Map<SensorID, Sensor>, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let sensors = conn
            .interact(move |conn| {
                schema::sensors::dsl::sensors
                    .filter(schema::sensors::dsl::entry_id.eq(entry_id_))
                    .select(Sensor::as_select())
                    .load::<Sensor>(conn)
            })
            .await??;
        let sensors_map = sensors.into_iter().map(|s| (s.id, s)).collect();
        debug!(
            "Queried sensors for entry_id {}: {:?}",
            entry_id_, sensors_map
        );
        Ok(sensors_map)
    }

    #[instrument]
    pub async fn get_all_sensors(&self, txid: TxID) -> Result<Map<SensorID, Sensor>, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let sensors = conn
            .interact(move |conn| {
                schema::sensors::dsl::sensors
                    .select(Sensor::as_select())
                    .load::<Sensor>(conn)
            })
            .await??;
        let sensors_map = sensors.into_iter().map(|s| (s.id, s)).collect();
        debug!("Queried all sensors: {:?}", sensors_map);
        Ok(sensors_map)
    }

    #[instrument]
    pub async fn get_topics(
        &self,
        entry_id_: EntryID,
        txid: TxID,
    ) -> Result<Map<TopicID, crate::storage::models::Topic>, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let topics = conn
            .interact(move |conn| {
                schema::topics::dsl::topics
                    .filter(schema::topics::dsl::entry_id.eq(entry_id_))
                    .select(crate::storage::models::Topic::as_select())
                    .load::<crate::storage::models::Topic>(conn)
            })
            .await??;
        let topics_map = topics.into_iter().map(|s| (s.id, s)).collect();
        debug!("Queried topics for entry_id {}", entry_id_);
        Ok(topics_map)
    }

    #[instrument]
    pub async fn add_topic(
        &self,
        topic: crate::storage::models::Topic,
        txid: TxID,
    ) -> Result<TopicID, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let t = topic.clone();
        let topic_id = conn
            .interact(move |conn| -> Result<TopicID, diesel::result::Error> {
                use crate::schema::topics::dsl as topics_dsl;
                diesel::insert_into(topics_dsl::topics)
                    .values((
                        topics_dsl::entry_id.eq(t.entry_id),
                        topics_dsl::topic_name.eq(t.topic_name),
                        topics_dsl::topic_type.eq(t.topic_type),
                        topics_dsl::message_count.eq(t.message_count),
                        topics_dsl::frequency.eq(t.frequency),
                        topics_dsl::created_at.eq(t.created_at),
                        topics_dsl::updated_at.eq(t.updated_at),
                    ))
                    .returning(topics_dsl::id)
                    .get_result::<TopicID>(conn)
            })
            .await??;
        // debug!(
        //     "Added topic for entry_id {} with new topic_id {}",
        //     topic.entry_id, topic_id
        // );
        Ok(topic_id)
    }

    #[instrument]
    pub async fn update_topic(
        &self,
        topic: crate::storage::models::Topic,
        txid: TxID,
    ) -> Result<(), StorageError> {
        let topic_id = topic.id;
        let conn = self.db_connection_pool().get().await?;
        conn.interact(move |conn| {
            diesel::update(schema::topics::dsl::topics.filter(schema::topics::dsl::id.eq(topic_id)))
                .set((
                    schema::topics::dsl::topic_name.eq(topic.topic_name),
                    schema::topics::dsl::topic_type.eq(topic.topic_type),
                    schema::topics::dsl::message_count.eq(topic.message_count),
                    schema::topics::dsl::frequency.eq(topic.frequency),
                    schema::topics::dsl::updated_at.eq(topic.updated_at),
                ))
                .execute(conn)
        })
        .await??;
        debug!("Updated topic {}", topic_id);
        Ok(())
    }

    #[instrument]
    pub async fn remove_topic(&self, topic_id: TopicID, txid: TxID) -> Result<(), StorageError> {
        let conn = self.db_connection_pool().get().await?;
        conn.interact(move |conn| {
            diesel::delete(schema::topics::dsl::topics.filter(schema::topics::dsl::id.eq(topic_id)))
                .execute(conn)
        })
        .await??;
        debug!("Removed topic with id {}", topic_id);
        Ok(())
    }

    #[instrument]
    pub async fn add_sensor(&self, sensor: Sensor, txid: TxID) -> Result<SensorID, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let s = sensor.clone();
        let sensor_id = conn
            .interact(move |conn| -> Result<SensorID, diesel::result::Error> {
                use crate::schema::sensors::dsl as sensors_dsl;
                diesel::insert_into(sensors_dsl::sensors)
                    .values((
                        sensors_dsl::entry_id.eq(s.entry_id),
                        sensors_dsl::sensor_name.eq(s.sensor_name),
                        sensors_dsl::manufacturer.eq(s.manufacturer),
                        sensors_dsl::sensor_type.eq(s.sensor_type),
                        sensors_dsl::ros_topics.eq(s.ros_topics),
                        sensors_dsl::custom_parameters.eq(s.custom_parameters),
                    ))
                    .returning(sensors_dsl::id)
                    .get_result::<SensorID>(conn)
            })
            .await??;
        debug!(
            "Added sensor for entry_id {} with new sensor_id {}",
            sensor.entry_id, sensor_id
        );
        Ok(sensor_id)
    }

    #[instrument]
    pub async fn add_entry(&self, entry: Entry, txid: TxID) -> Result<EntryID, StorageError> {
        let pool = self.db_connection_pool();
        let e = entry.clone();
        let entry_id_ = {
            let conn = pool.get().await?;
            conn.interact(move |conn| -> Result<EntryID, diesel::result::Error> {
                use crate::schema::entries::dsl as entries_dsl;
                diesel::insert_into(entries_dsl::entries)
                    .values((
                        entries_dsl::name.eq(e.name),
                        entries_dsl::path.eq(e.path),
                        entries_dsl::size.eq(e.size),
                        entries_dsl::created_at.eq(e.created_at),
                        entries_dsl::updated_at.eq(e.updated_at),
                        entries_dsl::time_machine.eq(e.time_machine),
                        entries_dsl::platform_name.eq(e.platform_name),
                        entries_dsl::platform_image_link.eq(e.platform_image_link),
                        entries_dsl::scenario_name.eq(e.scenario_name),
                        entries_dsl::scenario_creation_time.eq(e.scenario_creation_time),
                        entries_dsl::scenario_description.eq(e.scenario_description),
                        entries_dsl::sequence_duration.eq(e.sequence_duration),
                        entries_dsl::sequence_distance.eq(e.sequence_distance),
                        entries_dsl::sequence_lat_starting_point_deg
                            .eq(e.sequence_lat_starting_point_deg),
                        entries_dsl::sequence_lon_starting_point_deg
                            .eq(e.sequence_lon_starting_point_deg),
                        entries_dsl::weather_cloudiness.eq(e.weather_cloudiness),
                        entries_dsl::weather_precipitation.eq(e.weather_precipitation),
                        entries_dsl::weather_precipitation_deposits
                            .eq(e.weather_precipitation_deposits),
                        entries_dsl::weather_wind_intensity.eq(e.weather_wind_intensity),
                        entries_dsl::weather_road_humidity.eq(e.weather_road_humidity),
                        entries_dsl::weather_fog.eq(e.weather_fog),
                        entries_dsl::weather_snow.eq(e.weather_snow),
                        entries_dsl::tags.eq(e.tags),
                        entries_dsl::status.eq(e.status.clone()),
                    ))
                    .returning(entries_dsl::id)
                    .get_result::<EntryID>(conn)
            })
            .await??
        };
        debug!("Added entry with id {}", entry_id_);
        Ok(entry_id_)
    }

    #[instrument]
    pub async fn update_sensor(&self, sensor: Sensor, txid: TxID) -> Result<(), StorageError> {
        let sensor_id = sensor.id;
        let conn = self.db_connection_pool().get().await?;
        conn.interact(move |conn| {
            diesel::update(
                schema::sensors::dsl::sensors.filter(schema::sensors::dsl::id.eq(sensor_id)),
            )
            .set((
                schema::sensors::dsl::sensor_name.eq(sensor.sensor_name),
                schema::sensors::dsl::manufacturer.eq(sensor.manufacturer),
                schema::sensors::dsl::sensor_type.eq(sensor.sensor_type),
                schema::sensors::dsl::ros_topics.eq(sensor.ros_topics),
                schema::sensors::dsl::custom_parameters.eq(sensor.custom_parameters),
            ))
            .execute(conn)
        })
        .await??;
        debug!("Updated sensor {}", sensor_id);
        Ok(())
    }

    #[instrument]
    pub async fn remove_sensor(&self, sensor_id: SensorID, txid: TxID) -> Result<(), StorageError> {
        let conn = self.db_connection_pool().get().await?;
        conn.interact(move |conn| {
            diesel::delete(
                schema::sensors::dsl::sensors.filter(schema::sensors::dsl::id.eq(sensor_id)),
            )
            .execute(conn)
        })
        .await??;
        debug!("Removed sensor with id {}", sensor_id);
        Ok(())
    }

    #[instrument]
    pub async fn add_sequence(
        &self,
        entry_id_: EntryID,
        sequence: Sequence,
        txid: TxID,
    ) -> Result<SequenceID, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let s = sequence.clone();
        debug!("Adding sequence for entry_id {}: {:?}", entry_id_, s);
        let sequence_id = conn
            .interact(move |conn| {
                use crate::schema::sequences::dsl as sequences_dsl;
                diesel::insert_into(sequences_dsl::sequences)
                    .values((
                        sequences_dsl::entry_id.eq(s.entry_id),
                        sequences_dsl::description.eq(s.description),
                        sequences_dsl::start_timestamp.eq(s.start_timestamp),
                        sequences_dsl::end_timestamp.eq(s.end_timestamp),
                        sequences_dsl::created_at.eq(s.created_at),
                        sequences_dsl::updated_at.eq(s.updated_at),
                        sequences_dsl::tags.eq(s.tags),
                    ))
                    .returning(sequences_dsl::id)
                    .get_result::<SequenceID>(conn)
            })
            .await??;
        debug!(
            "Added sequence for entry_id {} with new sequence_id {}",
            entry_id_, sequence_id
        );
        Ok(sequence_id)
    }

    #[instrument]
    pub async fn update_sequence(
        &self,
        entry_id_: EntryID,
        sequence_id: SequenceID,
        sequence: Sequence,
        txid: TxID,
    ) -> Result<(), StorageError> {
        let conn = self.db_connection_pool().get().await?;
        conn.interact(move |conn| {
            diesel::update(
                schema::sequences::dsl::sequences
                    .filter(schema::sequences::dsl::id.eq(sequence_id))
                    .filter(schema::sequences::dsl::entry_id.eq(entry_id_)),
            )
            .set((
                schema::sequences::dsl::description.eq(sequence.description),
                schema::sequences::dsl::start_timestamp.eq(sequence.start_timestamp),
                schema::sequences::dsl::end_timestamp.eq(sequence.end_timestamp),
                schema::sequences::dsl::updated_at.eq(sequence.updated_at),
                schema::sequences::dsl::tags.eq(sequence.tags),
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
        entry_id_: EntryID,
        sequence_id: SequenceID,
        txid: TxID,
    ) -> Result<(), StorageError> {
        let conn = self.db_connection_pool().get().await?;
        conn.interact(move |conn| {
            diesel::delete(
                schema::sequences::dsl::sequences
                    .filter(schema::sequences::dsl::id.eq(sequence_id))
                    .filter(schema::sequences::dsl::entry_id.eq(entry_id_)),
            )
            .execute(conn)
        })
        .await??;
        debug!(
            "Removed sequence with id {} for entry_id {}",
            sequence_id, entry_id_
        );
        Ok(())
    }

    #[instrument]
    pub async fn add_tag(
        &self,
        entry_id_: EntryID,
        tag: Tag,
        txid: TxID,
    ) -> Result<(), StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let t = tag.clone();
        conn.interact(move |conn| {
            diesel::sql_query("UPDATE entries SET tags = array_append(tags, $1) WHERE id = $2 AND NOT ($1 = ANY(tags))")
                .bind::<diesel::sql_types::Text,_>(t)
                .bind::<diesel::sql_types::BigInt,_>(entry_id_)
                .execute(conn)
        }).await??;
        debug!("Added tag for entry_id {}", entry_id_);
        Ok(())
    }

    #[instrument]
    pub async fn remove_tag(
        &self,
        entry_id_: EntryID,
        tag: Tag,
        txid: TxID,
    ) -> Result<(), StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let t = tag.clone();
        conn.interact(move |conn| {
            diesel::sql_query("UPDATE entries SET tags = array_remove(tags, $1) WHERE id = $2")
                .bind::<diesel::sql_types::Text, _>(t)
                .bind::<diesel::sql_types::BigInt, _>(entry_id_)
                .execute(conn)
        })
        .await??;
        debug!("Removed tag");
        Ok(())
    }

    #[instrument]
    pub fn start_transaction(&self) -> TxID {
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
    pub async fn commit_transaction(&self, txid: TxID) -> Result<(), StorageError> {
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
