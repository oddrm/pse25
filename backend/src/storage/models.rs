use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rocket::serde::{Deserialize, Serialize};

pub type EntryID = i64;
pub type SequenceID = i64;
pub type SensorID = i64;
pub type Timestamp = i64;
pub type TopicID = i64;

#[derive(
    Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, PartialEq, Eq,
)]
#[diesel(table_name = crate::schema::files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct File {
    pub path: String,
    pub is_mcap: bool,
    pub is_custom_metadata: bool,
}

#[derive(
    Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Serialize, Deserialize, PartialEq,
)]
#[diesel(table_name = crate::schema::entries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Entry {
    pub id: EntryID,
    pub name: String,
    pub path: String,
    pub size: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: String,
    pub time_machine: Option<f64>,
    pub platform_name: Option<String>,
    pub platform_image_link: Option<String>,
    pub scenario_name: Option<String>,
    pub scenario_creation_time: Option<DateTime<Utc>>,
    pub scenario_description: Option<String>,
    pub sequence_duration: Option<f64>,
    pub sequence_distance: Option<f64>,
    pub sequence_lat_starting_point_deg: Option<f64>,
    pub sequence_lon_starting_point_deg: Option<f64>,
    pub weather_cloudiness: Option<String>,
    pub weather_precipitation: Option<String>,
    pub weather_precipitation_deposits: Option<String>,
    pub weather_wind_intensity: Option<String>,
    pub weather_road_humidity: Option<String>,
    pub weather_fog: Option<bool>,
    pub weather_snow: Option<bool>,
    pub tags: Vec<String>,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[diesel(table_name = crate::schema::topics)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(crate = "rocket::serde")]
pub struct Topic {
    pub id: TopicID,
    pub entry_id: i64,
    pub topic_name: String,
    pub topic_type: Option<String>,
    pub message_count: i64,
    pub frequency: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(
    Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, PartialEq, Eq,
)]
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
    pub tags: Vec<String>,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[diesel(table_name = crate::schema::sensors)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(crate = "rocket::serde")]
pub struct Sensor {
    pub id: i64,
    pub entry_id: i64,
    pub sensor_name: String,
    pub manufacturer: Option<String>,
    pub sensor_type: Option<String>,
    pub ros_topics: Vec<String>,
    pub custom_parameters: Option<serde_json::Value>,
}
