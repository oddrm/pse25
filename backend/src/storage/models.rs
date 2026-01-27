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
    pub last_modified: chrono::NaiveDateTime,
    pub created: chrono::NaiveDateTime,
    pub size: i64,
    pub last_checked: chrono::NaiveDateTime,
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
    pub id: MetadataID,
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
    pub created_at: DateTime<Utc>,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::topics)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TopicDb {
    pub id: TopicID,
    pub entry_id: i64,
    pub topic_name: String,
    pub message_count: i64,
    pub type_: Option<String>,
    pub type_description_hash: Option<String>,
    pub serialization_format: Option<String>,
    pub created_at: DateTime<Utc>,
}
