use crate::AppState;
use crate::error::{Error, StorageError};
use crate::storage::models::{
    Entry, EntryID, Sensor, SensorID, Sequence, SequenceID, Topic, TopicID,
};
use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct MetadataWeb {
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
    pub topics: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct SensorWeb {
    pub sensor_name: String,
    pub manufacturer: Option<String>,
    pub sensor_type: Option<String>,
    pub ros_topics: Vec<String>,
    pub custom_parameters: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct SequenceWeb {
    pub description: String,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub tags: Vec<String>,
}
use crate::storage::storage_manager::{Map, TxID};
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put, response::status};

fn not_found<T>(msg: String) -> Result<T, Error> {
    Err(StorageError::NotFound(msg).into())
}

//Kapitel 5.1.2 im Entwurfsheft (falls noch andere das ewig suchen)
#[get("/entries/<entry_id>/metadata/tx/<txid>")]
pub async fn get_metadata(
    state: &State<AppState>,
    entry_id: EntryID,
    txid: TxID,
) -> Result<Json<MetadataWeb>, Error> {
    let sm = &state.storage_manager;

    let entry = sm.get_metadata(entry_id, txid).await?;
    match entry {
        Some(e) => {
            let md = MetadataWeb {
                time_machine: e.time_machine,
                platform_name: e.platform_name,
                platform_image_link: e.platform_image_link,
                scenario_name: e.scenario_name,
                scenario_creation_time: e.scenario_creation_time,
                scenario_description: e.scenario_description,
                sequence_duration: e.sequence_duration,
                sequence_distance: e.sequence_distance,
                sequence_lat_starting_point_deg: e.sequence_lat_starting_point_deg,
                sequence_lon_starting_point_deg: e.sequence_lon_starting_point_deg,
                weather_cloudiness: e.weather_cloudiness,
                weather_precipitation: e.weather_precipitation,
                weather_precipitation_deposits: e.weather_precipitation_deposits,
                weather_wind_intensity: e.weather_wind_intensity,
                weather_road_humidity: e.weather_road_humidity,
                weather_fog: e.weather_fog,
                weather_snow: e.weather_snow,
                topics: {
                    // fetch topic names from topics table
                    let txid: crate::storage::storage_manager::TxID = 0;
                    match state.storage_manager.get_topics(e.id, txid).await {
                        Ok(map) => {
                            let names: Vec<String> =
                                map.values().map(|t| t.topic_name.clone()).collect();
                            if names.is_empty() { None } else { Some(names) }
                        }
                        Err(_) => None,
                    }
                },
            };
            Ok(Json(md))
        }
        None => not_found(format!("metadata for entry {entry_id} not found")),
    }
}

//Das müssen wir nochmal anschauen. Vielleicht funktioniert nicht mit JSON ????????????????????????????????????
//?????????????????????????????????????????????????????????????????????????????????????????????????????????????
#[put(
    "/entries/<entry_id>/metadata/tx/<txid>",
    format = "json",
    data = "<metadata>"
)]
pub async fn update_metadata(
    state: &State<AppState>,
    entry_id: EntryID,
    metadata: Json<MetadataWeb>,
    txid: TxID,
) -> Result<status::NoContent, Error> {
    let sm = &state.storage_manager;
    let m = metadata.into_inner();
    sm.update_entry(entry_id, m, txid).await?;
    Ok(status::NoContent)
}

#[get("/entries?<search_string>&<sort_by>&<ascending>&<page>&<page_size>&<txid>")]
pub async fn get_entries(
    state: &State<AppState>,
    search_string: Option<String>,
    sort_by: Option<String>,
    ascending: Option<bool>,
    page: Option<u32>,
    page_size: Option<u32>,
    txid: Option<TxID>,
) -> Result<Json<Vec<Entry>>, Error> {
    let sm = &state.storage_manager;
    let txid = txid.unwrap_or(0);

    let entries = sm
        .get_entries(search_string, sort_by, ascending, page, page_size, txid)
        .await?;

    Ok(Json(entries))
}

#[get("/entries/<entry_id>/tx/<txid>")]
pub async fn get_entry(
    state: &State<AppState>,
    entry_id: EntryID,
    txid: TxID,
) -> Result<Json<Entry>, Error> {
    let sm = &state.storage_manager;

    let entry = sm.get_entry(entry_id, txid).await?;
    match entry {
        Some(e) => Ok(Json(e)),
        None => not_found(format!("entry {entry_id} not found")),
    }
}

#[get("/paths/<path>/tx/<txid>")]
pub async fn get_entry_by_path(
    state: &State<AppState>,
    path: String,
    txid: TxID,
) -> Result<Json<Entry>, Error> {
    let sm = &state.storage_manager;

    let entry = sm.get_entry_by_path(path.clone(), txid).await?;
    match entry {
        Some(e) => Ok(Json(e)),
        None => not_found(format!("entry with path '{path}' not found")),
    }
}

#[get("/entries/<entry_id>/sequences/tx/<txid>")]
pub async fn get_sequences(
    state: &State<AppState>,
    entry_id: EntryID,
    txid: TxID,
) -> Result<Json<Map<SequenceID, Sequence>>, Error> {
    let sm = &state.storage_manager;

    let sequences = sm.get_sequences(entry_id, txid).await?;
    Ok(Json(sequences))
}

#[get("/entries/<entry_id>/topics/tx/<txid>")]
pub async fn get_topics(
    state: &State<AppState>,
    entry_id: EntryID,
    txid: TxID,
) -> Result<Json<Map<TopicID, Topic>>, Error> {
    let sm = &state.storage_manager;

    let topics = sm.get_topics(entry_id, txid).await?;
    Ok(Json(topics))
}

#[get("/entries/<entry_id>/sensors/tx/<txid>")]
pub async fn get_sensors(
    state: &State<AppState>,
    entry_id: EntryID,
    txid: TxID,
) -> Result<Json<Map<SensorID, Sensor>>, Error> {
    let sm = &state.storage_manager;

    let sensors = sm.get_sensors(entry_id, txid).await?;
    Ok(Json(sensors))
}

#[get("/sensors/tx/<txid>")]
pub async fn get_all_sensors(
    state: &State<AppState>,
    txid: TxID,
) -> Result<Json<Map<SensorID, Sensor>>, Error> {
    let sm = &state.storage_manager;

    let sensors = sm.get_all_sensors(txid).await?;
    Ok(Json(sensors))
}

#[post(
    "/entries/<entry_id>/sensors/tx/<txid>",
    format = "json",
    data = "<sensor>"
)]
pub async fn add_sensor(
    state: &State<AppState>,
    entry_id: EntryID,
    sensor: Json<SensorWeb>,
    txid: TxID,
) -> Result<status::Created<Json<SensorID>>, Error> {
    let sm = &state.storage_manager;

    let s = sensor.into_inner();
    let storage_sensor = Sensor {
        id: 0, // will be generated by storage manager
        entry_id,
        sensor_name: s.sensor_name,
        manufacturer: s.manufacturer,
        sensor_type: s.sensor_type,
        ros_topics: s.ros_topics,
        custom_parameters: s.custom_parameters,
    };

    let new_id = sm.add_sensor(storage_sensor, txid).await?;
    Ok(status::Created::new(format!("/entries/{entry_id}/sensors/{new_id}")).body(Json(new_id)))
}

#[put(
    "/entries/<entry_id>/sensors/<sensor_id>/tx/<txid>",
    format = "json",
    data = "<sensor>"
)]
pub async fn update_sensor(
    state: &State<AppState>,
    entry_id: EntryID,
    sensor_id: SensorID,
    sensor: Json<SensorWeb>,
    txid: TxID,
) -> Result<status::NoContent, Error> {
    let sm = &state.storage_manager;
    let s = sensor.into_inner();

    let storage_sensor = Sensor {
        id: sensor_id,
        entry_id,
        sensor_name: s.sensor_name,
        manufacturer: s.manufacturer,
        sensor_type: s.sensor_type,
        ros_topics: s.ros_topics,
        custom_parameters: s.custom_parameters,
    };

    sm.update_sensor(storage_sensor, txid).await?;
    Ok(status::NoContent)
}

#[delete("/sensors/<sensor_id>/tx/<txid>")]
pub async fn remove_sensor(
    state: &State<AppState>,
    sensor_id: SensorID,
    txid: TxID,
) -> Result<status::NoContent, Error> {
    let sm = &state.storage_manager;
    sm.remove_sensor(sensor_id, txid).await?;
    Ok(status::NoContent)
}

#[post(
    "/entries/<entry_id>/sequences/tx/<txid>",
    format = "json",
    data = "<sequence>"
)]
pub async fn add_sequence(
    state: &State<AppState>,
    entry_id: EntryID,
    sequence: Json<SequenceWeb>,
    txid: TxID,
) -> Result<status::Created<Json<SequenceID>>, Error> {
    let sm = &state.storage_manager;
    let s = sequence.into_inner();
    let storage_sequence = Sequence {
        id: 0,
        entry_id,
        description: s.description,
        start_timestamp: s.start_timestamp,
        end_timestamp: s.end_timestamp,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        tags: s.tags,
    };

    let new_id = sm.add_sequence(entry_id, storage_sequence, txid).await?;
    Ok(status::Created::new(format!("/entries/{entry_id}/sequences/{new_id}")).body(Json(new_id)))
}

#[put(
    "/entries/<entry_id>/sequences/<sequence_id>/tx/<txid>",
    format = "json",
    data = "<sequence>"
)]
pub async fn update_sequence(
    state: &State<AppState>,
    entry_id: EntryID,
    sequence_id: SequenceID,
    sequence: Json<SequenceWeb>,
    txid: TxID,
) -> Result<status::NoContent, Error> {
    let sm = &state.storage_manager;
    let s = sequence.into_inner();

    let storage_sequence = Sequence {
        id: sequence_id,
        entry_id,
        description: s.description,
        start_timestamp: s.start_timestamp,
        end_timestamp: s.end_timestamp,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        tags: s.tags,
    };

    sm.update_sequence(entry_id, sequence_id, storage_sequence, txid)
        .await?;
    Ok(status::NoContent)
}

#[delete("/entries/<entry_id>/sequences/<sequence_id>/tx/<txid>")]
pub async fn remove_sequence(
    state: &State<AppState>,
    entry_id: EntryID,
    sequence_id: SequenceID,
    txid: TxID,
) -> Result<status::NoContent, Error> {
    let sm = &state.storage_manager;
    sm.remove_sequence(entry_id, sequence_id, txid).await?;
    Ok(status::NoContent)
}

#[put("/entries/<entry_id>/tags/tx/<txid>", data = "<tag>")]
pub async fn add_tag(
    state: &State<AppState>,
    entry_id: EntryID,
    tag: String,
    txid: TxID,
) -> Result<status::NoContent, Error> {
    let sm = &state.storage_manager;
    sm.add_tag(entry_id, tag, txid).await?;
    Ok(status::NoContent)
}

#[delete("/entries/<entry_id>/tags/tx/<txid>", data = "<tag>")]
pub async fn remove_tag(
    state: &State<AppState>,
    entry_id: EntryID,
    tag: String,
    txid: TxID,
) -> Result<status::NoContent, Error> {
    let sm = &state.storage_manager;
    sm.remove_tag(entry_id, tag, txid).await?;
    Ok(status::NoContent)
}

#[get("/transaction")]
pub async fn start_transaction(state: &State<AppState>) -> Result<Json<TxID>, Error> {
    let sm = &state.storage_manager;
    let txid = sm.start_transaction();
    Ok(Json(txid))
}

#[get("/transaction/<txid>/commit")]
pub async fn commit_transaction(
    state: &State<AppState>,
    txid: TxID,
) -> Result<status::NoContent, Error> {
    let sm = &state.storage_manager;
    sm.commit_transaction(txid).await?;
    Ok(status::NoContent)
}
