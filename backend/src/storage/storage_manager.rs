#![allow(unused)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::Duration,
};

use crate::{
    routes,
    schema::{files, sensors::entry_id},
};
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
        _search_string: Option<String>,
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
        Ok(entries)
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
        entry_id_: EntryID,
        txid: TxID,
    ) -> Result<Map<SequenceID, Sequence>, StorageError> {
        let conn = self.db_connection_pool().get().await?;
        let sequences = conn
            .interact(move |conn| {
                schema::sequences::dsl::sequences
                    .filter(schema::sequences::dsl::entry_id.eq(entry_id_))
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
                    .load::<crate::storage::models::Topic>(conn)
            })
            .await??;
        let topics_map = topics.into_iter().map(|s| (s.id, s)).collect();
        debug!(
            "Queried topics for entry_id {}: {:?}",
            entry_id_, topics_map
        );
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
                let next_id: i64 = diesel::select(diesel::dsl::sql::<diesel::sql_types::BigInt>(
                    "COALESCE(MAX(id),0)+1",
                ))
                .get_result(conn)?;

                let mut to_insert = t.clone();
                to_insert.id = next_id;
                diesel::insert_into(schema::topics::dsl::topics)
                    .values(&to_insert)
                    .returning(schema::topics::dsl::id)
                    .get_result::<TopicID>(conn)
            })
            .await??;
        debug!(
            "Added topic for entry_id {} with new topic_id {}",
            topic.entry_id, topic_id
        );
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
                let next_id: i64 = diesel::select(diesel::dsl::sql::<diesel::sql_types::BigInt>(
                    "COALESCE(MAX(id),0)+1",
                ))
                .get_result(conn)?;

                let mut to_insert = s.clone();
                to_insert.id = next_id;
                diesel::insert_into(schema::sensors::dsl::sensors)
                    .values(&to_insert)
                    .returning(schema::sensors::dsl::id)
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
