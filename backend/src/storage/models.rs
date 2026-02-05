#![allow(unused)]
use crate::error::Error;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rocket::serde::{Deserialize, Serialize};

pub type EntryID = i64;
pub type SequenceID = i64;
pub type MetadataID = i64;
pub type TagID = i64;
pub type TopicID = i64;
pub type Timestamp = i64;

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct File {
    pub path: String,
    pub is_mcap: bool,
    pub is_custom_metadata: bool,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::entries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Entry {
    pub id: EntryID,
    pub name: String,
    pub path: String,
    pub platform: String,
    pub size: i64,
    pub start_time_ns: Option<i64>,
    pub duration_ns: Option<i64>,
    pub total_message_count: Option<i64>,
    pub storage_identifier: Option<String>,
    pub compression_format: Option<String>,
    pub compression_mode: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::sequences)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Sequence {
    pub id: SequenceID,
    pub entry_id: i64,
    pub description: String,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::metadata)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Metadata {
    pub entry_id: EntryID,
    pub metadata_json: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Tag {
    pub id: TagID,
    pub entry_id: i64,
    pub name: String,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::topics)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Topic {
    pub id: TopicID,
    pub entry_id: i64,
    pub topic_name: String,
    pub message_count: i64,
    pub type_: Option<String>,
    pub type_description_hash: Option<String>,
    pub serialization_format: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::metadata_info)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(crate = "rocket::serde")]
pub struct MetadataInfo {
    pub id: i64,
    pub metadata_id: i64,
    pub data_spec_version: Option<String>,
    pub dataset_license: Option<String>,
    pub meta_data_spec_version: Option<String>,
    pub sequence_version: Option<String>,
    pub software_info: Option<String>,
    pub software_version: Option<String>,
    pub time_human: Option<String>,
    pub time_machine: Option<f64>,
    pub created_at: DateTime<Utc>,
}


#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::metadata_labeling)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(crate = "rocket::serde")]
pub struct MetadataLabeling {
    pub metadata_id: i64,
    pub labeling_key: String,
    pub creation_time_human: Option<String>,
    pub freetext: Option<String>,
    pub policy_version: Option<String>,
    pub provider: Option<String>,
    pub sensors_json: Option<serde_json::Value>,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::metadata_scenario)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(crate = "rocket::serde")]
pub struct MetadataScenario {
    pub id: i64,
    pub metadata_id: i64,
    pub environment_dynamics: Option<String>,
    pub environment_tags: Option<serde_json::Value>,
    pub name: Option<String>,
}


#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::metadata_dataset_sequence)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(crate = "rocket::serde")]
pub struct MetadataDatasetSequence {
    pub id: i64,
    pub metadata_id: i64,
    pub description: Option<String>,
    pub distance: Option<f64>,
    pub duration: Option<f64>,
    pub lat_starting_point_deg: Option<f64>,
    pub lon_starting_point_deg: Option<f64>,
    pub name: Option<String>,
    pub weather: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}


#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::metadata_setup)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(crate = "rocket::serde")]
pub struct MetadataSetup {
    pub id: i64,
    pub metadata_id: i64,
    pub name: Option<String>,
    pub platform_description_link: Option<String>,
    pub created_at: DateTime<Utc>,
}


#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::metadata_sensor)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(crate = "rocket::serde")]
pub struct MetadataSensor {
    pub id: i64,
    pub metadata_id: i64,
    pub sensor_key: String,
    pub acquisition_rate: Option<f64>,
    pub acquistion_mode: Option<String>,
    pub capture_rate: Option<f64>,
    pub channel_number: Option<i32>,
    pub channel_space: Option<String>,
    pub firmware_version: Option<String>,
    pub focus_position_m: Option<f64>,
    pub fov_horizontal_deg: Option<f64>,
    pub fov_vertical_deg: Option<f64>,
    pub frame_id: Option<String>,
    pub freetext: Option<String>,
    pub image_height: Option<i32>,
    pub image_width: Option<i32>,
    pub lens: Option<String>,
    pub manufacturer: Option<String>,
    pub max_exposure: Option<i32>,
    pub model: Option<String>,
    pub mtu: Option<i32>,
    pub optical_center_frame: Option<String>,
    pub ros_topics: Option<serde_json::Value>,
    pub sw_trigger_rate: Option<f64>,
    pub time_stamp_accuracy: Option<String>,
    pub time_sync_method: Option<String>,
    pub trigger_method: Option<String>,
    pub trigger_mode: Option<bool>,
    pub trigger_reference: Option<String>,
    pub trigger_source: Option<String>,
    pub type_: Option<String>,
    pub created_at: DateTime<Utc>,
}
