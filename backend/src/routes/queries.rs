#![allow(unused_variables)]

use crate::error::Error;
use crate::storage::entry::Entry;
use crate::storage::sequence::Sequence;
use crate::storage::storage_manager::Map;
use crate::{AppState, storage::sequence::SequenceID, storage::storage_manager::EntryID};
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put, response::status};

//Kapitel 5.1.2 im Entwurfsheft (falls noch andere das ewig suchen)
#[get("/entries/<entry_id>/metadata")]
pub async fn get_metadata(
    state: &State<AppState>,
    entry_id: EntryID,
) -> Result<Json<crate::storage::metadata::Metadata>, Error> {
    todo!()
}

#[put("/entries/<entry_id>/metadata", format = "json", data = "<metadata>")]
pub async fn update_metadata(
    state: &State<AppState>,
    entry_id: EntryID,
    metadata: String,
) -> Result<status::NoContent, Error> {
    todo!()
}

#[get("/entries?<search_string>&<sort_by>&<ascending>&<page>&<page_size>")]
pub async fn get_entries(
    state: &State<AppState>,
    search_string: Option<String>,
    sort_by: Option<String>,
    ascending: Option<bool>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<Json<Vec<Entry>>, Error> {
    todo!()
}

#[get("/entries/<entry_id>")]
pub async fn get_entry(state: &State<AppState>, entry_id: EntryID) -> Result<Json<Entry>, Error> {
    todo!()
}

#[get("/paths/<path>")]
pub async fn get_entry_by_path(
    state: &State<AppState>,
    path: String,
) -> Result<Json<Entry>, Error> {
    todo!()
}

#[get("/entries/<entry_id>/sequences")]
pub async fn get_sequences(
    state: &State<AppState>,
    entry_id: EntryID,
) -> Result<Json<Map<SequenceID, Sequence>>, Error> {
    todo!()
}

#[post("/entries/<entry_id>/sequences", format = "json", data = "<sequence>")]
pub async fn add_sequence(
    state: &State<AppState>,
    entry_id: EntryID,
    sequence: String,
) -> Result<status::Created<Json<SequenceID>>, Error> {
    todo!()
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
    sequence: String,
) -> Result<status::NoContent, Error> {
    todo!()
}

#[delete("/entries/<entry_id>/sequences/<sequence_id>")]
pub async fn remove_sequence(
    state: &State<AppState>,
    entry_id: EntryID,
    sequence_id: SequenceID,
) -> Result<status::NoContent, Error> {
    todo!()
}

#[put("/entries/<entry_id>/tags", format = "json", data = "<tag>")]
pub async fn add_tag(
    state: &State<AppState>,
    entry_id: EntryID,
    tag: String,
) -> Result<status::NoContent, Error> {
    todo!()
}

#[delete("/entries/<entry_id>/tags", format = "json", data = "<tag>")]
pub async fn remove_tag(
    state: &State<AppState>,
    entry_id: EntryID,
    tag: String,
) -> Result<status::NoContent, Error> {
    todo!()
}
