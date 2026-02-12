#![allow(unused_variables)]

use crate::AppState;
use crate::error::{Error, StorageError};
use crate::storage::models::{Entry, EntryID, Sequence, SequenceID};
use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct Metadata {
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
    pub tags: Option<Vec<String>>,
    pub topics: Option<Vec<String>>,
}
use crate::storage::storage_manager::{Map, TxID};
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put, response::status};
use tracing::debug;

fn not_found<T>(msg: String) -> Result<T, Error> {
    Err(StorageError::NotFound(msg).into())
}

//Kapitel 5.1.2 im Entwurfsheft (falls noch andere das ewig suchen)
#[get("/entries/<entry_id>/<txid>/metadata")]
pub async fn get_metadata(
    state: &State<AppState>,
    entry_id: EntryID,
    txid: TxID
) -> Result<Json<Metadata>, Error> {
    let sm = &state.storage_manager;
    
    let entry = sm.get_metadata(entry_id, txid).await?;
    match entry {
        Some(e) => {
            let md = Metadata {
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
                tags: if e.tags.is_empty() { None } else { Some(e.tags) },
                topics: if e.topics.is_empty() { None } else { Some(e.topics) },
            };
            Ok(Json(md))
        }
        None => not_found(format!("metadata for entry {entry_id} not found")),
    }
}

//Das müssen wir nochmal anschauen. Vielleicht funktioniert nicht mit JSON ????????????????????????????????????
//?????????????????????????????????????????????????????????????????????????????????????????????????????????????
#[put("/entries/<entry_id>/metadata", format = "json", data = "<metadata>")]
pub async fn update_metadata(
    state: &State<AppState>,
    entry_id: EntryID,
    metadata: Json<Metadata>,
) -> Result<status::NoContent, Error> {
    let sm = &state.storage_manager;
    let txid: TxID = 0;
    let m = metadata.into_inner();

    let mut entry = match sm.get_entry(entry_id, txid).await? {
        Some(e) => e,
        None => return not_found(format!("entry {entry_id} not found")),
    };

    if m.time_machine.is_some() {
        entry.time_machine = m.time_machine;
    }
    if m.platform_name.is_some() {
        entry.platform_name = m.platform_name.clone();
    }
    if m.platform_image_link.is_some() {
        entry.platform_image_link = m.platform_image_link.clone();
    }
    if m.scenario_name.is_some() {
        entry.scenario_name = m.scenario_name.clone();
    }
    if m.scenario_creation_time.is_some() {
        entry.scenario_creation_time = m.scenario_creation_time;
    }
    if m.scenario_description.is_some() {
        entry.scenario_description = m.scenario_description.clone();
    }
    if m.sequence_duration.is_some() {
        entry.sequence_duration = m.sequence_duration;
    }
    if m.sequence_distance.is_some() {
        entry.sequence_distance = m.sequence_distance;
    }
    if m.sequence_lat_starting_point_deg.is_some() {
        entry.sequence_lat_starting_point_deg = m.sequence_lat_starting_point_deg;
    }
    if m.sequence_lon_starting_point_deg.is_some() {
        entry.sequence_lon_starting_point_deg = m.sequence_lon_starting_point_deg;
    }
    if m.weather_cloudiness.is_some() {
        entry.weather_cloudiness = m.weather_cloudiness.clone();
    }
    if m.weather_precipitation.is_some() {
        entry.weather_precipitation = m.weather_precipitation.clone();
    }
    if m.weather_precipitation_deposits.is_some() {
        entry.weather_precipitation_deposits = m.weather_precipitation_deposits.clone();
    }
    if m.weather_wind_intensity.is_some() {
        entry.weather_wind_intensity = m.weather_wind_intensity.clone();
    }
    if m.weather_road_humidity.is_some() {
        entry.weather_road_humidity = m.weather_road_humidity.clone();
    }
    if m.weather_fog.is_some() {
        entry.weather_fog = m.weather_fog;
    }
    if m.weather_snow.is_some() {
        entry.weather_snow = m.weather_snow;
    }
    if let Some(tags) = &m.tags {
        entry.tags = tags.clone();
    }
    if let Some(topics) = &m.topics {
        entry.topics = topics.clone();
    }

    entry.updated_at = chrono::Utc::now();

    sm.update_entry(entry_id, &entry, txid).await?;
    Ok(status::NoContent)
}

#[get("/entries?<search_string>&<sort_by>&<ascending>&<page>&<page_size>")]
pub async fn get_entries(
    state: &State<AppState>,
    search_string: Option<String>,
    sort_by: Option<String>,
    ascending: Option<bool>,
    page: Option<u32>,
    page_size: Option<u32>,
)-> Result<Json<Vec<Entry>>, Error> {
    let sm = &state.storage_manager;
    let txid: TxID = 0;

    let entries = sm
        .get_entries(search_string, sort_by, ascending, page, page_size, txid)
        .await?;

    Ok(Json(entries))
}

#[get("/entries/<entry_id>/<txid>")]
pub async fn get_entry(state: &State<AppState>, 
    entry_id: EntryID,
    txid: TxID
) -> Result<Json<Entry>, Error> {
    let sm = &state.storage_manager;


    let entry = sm.get_entry(entry_id, txid).await?;
    match entry {
        Some(e) => Ok(Json(e)),
        None => not_found(format!("entry {entry_id} not found")),
    }
}

#[get("/paths/<path>/<txid>")]
pub async fn get_entry_by_path(
    state: &State<AppState>,
    path: String,
    txid: TxID
) -> Result<Json<Entry>, Error> {
    let sm = &state.storage_manager;

    let entry = sm.get_entry_by_path(path.clone(), txid).await?;
    match entry {
        Some(e) => Ok(Json(e)),
        None => not_found(format!("entry with path '{path}' not found")),
    }
}

#[get("/entries/<entry_id>/sequences")]
pub async fn get_sequences(
    state: &State<AppState>,
    entry_id: EntryID,
) -> Result<Json<Map<SequenceID, Sequence>>, Error> {
    let sm = &state.storage_manager;
    let txid: TxID = 0;
    

    let sequences = sm.get_sequences(entry_id, txid).await?;
    Ok(Json(sequences))
}

#[post("/entries/<entry_id>/sequences/<txid>", format = "json", data = "<sequence>")]
pub async fn add_sequence(
    state: &State<AppState>,
    entry_id: EntryID,
    sequence: Json<Sequence>,
    txid: TxID
) -> Result<status::Created<Json<SequenceID>>, Error> {
    let sm = &state.storage_manager;
    

    let new_id = sm
        .add_sequence(entry_id, sequence.into_inner(), txid)
        .await?;
    Ok(status::Created::new(format!("/entries/{entry_id}/sequences/{new_id}")).body(Json(new_id)))
}

#[put(
    "/entries/<entry_id>/sequences/<sequence_id>",
    format = "json",
    data = "<sequence>"
)]
pub async fn update_sequence(
    state: &State<AppState>,
    entry_id: EntryID,
    sequence_id: SequenceID,
    sequence: Json<Sequence>,
) -> Result<status::NoContent, Error> {
    let sm = &state.storage_manager;
    let txid: TxID = 0;

    let mut seq = sequence.into_inner();
    seq.id = sequence_id;
    seq.entry_id = entry_id;

    sm.update_sequence(entry_id, sequence_id, seq, txid).await?;
    Ok(status::NoContent)
}

#[delete("/entries/<entry_id>/sequences/<sequence_id>")]
pub async fn remove_sequence(
    state: &State<AppState>,
    entry_id: EntryID,
    sequence_id: SequenceID,
) -> Result<status::NoContent, Error> {
    let sm = &state.storage_manager;
    let txid: TxID = 0;
    sm.remove_sequence(entry_id, sequence_id, txid).await?;
    Ok(status::NoContent)
}

#[put("/entries/<entry_id>/tags", format = "json", data = "<tag>")]
pub async fn add_tag(
    state: &State<AppState>,
    entry_id: EntryID,
    tag: String,
) -> Result<status::NoContent, Error> {
    let sm = &state.storage_manager;
    let txid: TxID = 0;
    sm.add_tag(entry_id, tag, txid).await?;
    Ok(status::NoContent)
}

#[delete("/entries/<entry_id>/tags", format = "json", data = "<tag>")]
pub async fn remove_tag(
    state: &State<AppState>,
    entry_id: EntryID,
    tag: String,
) -> Result<status::NoContent, Error> {
    let sm = &state.storage_manager;
    let txid: TxID = 0;
    sm.remove_tag(entry_id, tag, txid).await?;
    Ok(status::NoContent)
}
