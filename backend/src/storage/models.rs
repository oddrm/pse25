#![allow(unused)]

//! # Storage Models Module
//!
//! This module defines all data models used in the storage system. These models serve **dual purposes**:
//! - **Database operations**: Used directly with Diesel ORM for querying and inserting data
//! - **API serialization**: The same models are serialized to JSON for API responses
//!
//! ## Models Overview
//!
//! All models in this module:
//! - Match the database schema exactly (one-to-one mapping with database tables)
//! - Support Diesel operations (`Queryable`, `Selectable`, `Insertable`)
//! - Support JSON serialization (`Serialize`, `Deserialize`) for API responses
//!
//! ## Usage
//!
//! ```rust
//! // Query from database
//! let entry: Entry = entries::table
//!     .filter(entries::id.eq(entry_id))
//!     .first(conn)?;
//!
//! // Use directly in API response
//! Ok(Json(entry))
//! ```
//!
//! **Note**: For `Entry` responses that need tags, load tags separately and include them
//! in the API response structure at the route level, as tags are stored in a separate table.

use diesel::prelude::*;
use rocket::serde::{Deserialize, Serialize};
use crate::error::Error;
use chrono::{DateTime, Utc};

// Type aliases
pub type EntryID = i64;
pub type SequenceID = i64;
pub type MetadataID = i64;
pub type TagID = i64;
pub type TopicID = i64;
pub type Timestamp = i64;


/// Database model for the `files` table.
///
/// This table tracks filesystem-level information about files that are being monitored.
/// It's separate from the `entries` table because:
/// - `files` tracks raw filesystem events (file creation, modification, etc.)
/// - `entries` represents processed/logical entries that may be created from files
///
/// The `File` model is used by the filesystem watcher to track when files are created,
/// modified, or need to be re-checked. This is part of the initial file discovery process
/// before entries are created from these files.
#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct File {
    pub path: String,
    pub last_modified: chrono::NaiveDateTime,
    pub created: chrono::NaiveDateTime,
    pub size: i64,
    pub last_checked: chrono::NaiveDateTime,
}

/// Model for the `entries` table.
///
/// Represents a logical entry (e.g., a rosbag/MCAP file) with all its metadata.
/// This is the main entity in the system - it contains information about data files
/// including timing information, compression details, and message counts.
///
/// Used for both database operations and API serialization.
#[derive(Queryable, Selectable, Insertable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::schema::entries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(crate = "rocket::serde")]
pub struct Entry {
    pub id: i64,
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

/// Model for the `sequences` table.
///
/// Represents a time sequence within an entry. Sequences define time ranges
/// (start_timestamp to end_timestamp) with a description, allowing users to
/// mark and organize specific time periods within data files.
///
/// Used for both database operations and API serialization.
#[derive(Queryable, Selectable, Insertable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::schema::sequences)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(crate = "rocket::serde")]
pub struct Sequence {
    pub id: i64,
    pub entry_id: i64,
    pub description: String,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Model for the `metadata` table.
///
/// Stores arbitrary JSON metadata associated with an entry. This allows storing
/// flexible, entry-specific metadata that doesn't fit into the structured fields
/// of the entries table. The metadata is stored as JSONB for efficient querying.
///
/// Used for both database operations and API serialization.
#[derive(Queryable, Selectable, Insertable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::schema::metadata)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(crate = "rocket::serde")]
pub struct Metadata {
    pub id: i64,
    pub entry_id: i64,
    pub metadata_json: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Model for the `tags` table.
///
/// Represents a tag associated with an entry. Tags are used for categorizing
/// and organizing entries. Multiple tags can be associated with a single entry.
#[derive(Queryable, Selectable, Insertable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::schema::tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(crate = "rocket::serde")]
pub struct Tag {
    pub id: i64,
    pub entry_id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

/// Model for the `topics` table.
///
/// Represents a topic (e.g., ROS topic) within an entry. Topics contain
/// information about message types, serialization formats, and message counts.
/// Used for tracking what topics are present in rosbag/MCAP files.
#[derive(Queryable, Selectable, Insertable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::schema::topics)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(crate = "rocket::serde")]
pub struct Topic {
    pub id: i64,
    pub entry_id: i64,
    pub topic_name: String,
    pub message_count: i64,
    pub type_: Option<String>,
    pub type_description_hash: Option<String>,
    pub serialization_format: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Metadata {
    // includes validation
    pub fn from_yaml(yaml_str: &String) -> Result<Self, Error> {
        todo!()
    }

    pub fn to_yaml(&self) -> Result<String, Error> {
        todo!()
    }
}

impl Sequence {
    pub fn new(description: String, start: Timestamp, end: Timestamp) -> Self {
        let now = Utc::now();
        Sequence {
            id: 0, // Will be set by database
            entry_id: 0, // Will be set when associating with entry
            description,
            start_timestamp: start,
            end_timestamp: end,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn start(&self) -> Timestamp {
        self.start_timestamp
    }

    pub fn end(&self) -> Timestamp {
        self.end_timestamp
    }
}
