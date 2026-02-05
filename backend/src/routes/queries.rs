#![allow(unused_variables)]

use crate::AppState;
use crate::error::{Error, StorageError};
use crate::storage::models::{Entry, EntryID, Sequence, SequenceID, Metadata};
use crate::storage::storage_manager::{Map, TxID};
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put, response::status};

fn not_found<T>(msg: String) -> Result<T, Error> {
    Err(StorageError::NotFound(msg).into())
}

//Kapitel 5.1.2 im Entwurfsheft (falls noch andere das ewig suchen)
#[get("/entries/<entry_id>/metadata")]
pub async fn get_metadata(
    state: &State<AppState>,
    entry_id: EntryID,
) -> Result<Json<Metadata>, Error> {
    let sm = &state.storage_manager;
    let txid: TxID = 0;

    let meta = sm.get_metadata(entry_id, txid).await?;
    match meta {
        Some(m) => Ok(Json(m)),
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

    let mut m = metadata.into_inner();

    m.entry_id = entry_id;

    sm.update_metadata(entry_id, &m, txid).await?;
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
) -> Result<Json<Vec<(EntryID, Metadata)>>, Error> {
    let sm = &state.storage_manager;
    let txid: TxID = 0;

    let entries = sm
        .get_entries(search_string, sort_by, ascending, page, page_size, txid)
        .await?;

    Ok(Json(entries))
}

#[get("/entries/<entry_id>")]
pub async fn get_entry(
    state: &State<AppState>,
    entry_id: EntryID,
) -> Result<Json<Entry>, Error> {
    let sm = &state.storage_manager;
    let txid: TxID = 0;

    let entry = sm.get_entry(entry_id, txid).await?;
    match entry {
        Some(e) => Ok(Json(e)),
        None => not_found(format!("entry {entry_id} not found")),
    }
}

#[get("/paths/<path>")]
pub async fn get_entry_by_path(
    state: &State<AppState>,
    path: String,
) -> Result<Json<Entry>, Error> {
    let sm = &state.storage_manager;
    let txid: TxID = 0;

    let entry = sm.get_entry_by_path(&path, txid).await?;
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

#[post("/entries/<entry_id>/sequences", format = "json", data = "<sequence>")]
pub async fn add_sequence(
    state: &State<AppState>,
    entry_id: EntryID,
    sequence: Json<Sequence>,
) -> Result<status::Created<Json<SequenceID>>, Error> {
    let sm = &state.storage_manager;
    let txid: TxID = 0;

    let new_id = sm.add_sequence(entry_id, sequence.into_inner(), txid).await?;
    Ok(
        status::Created::new(format!("/entries/{entry_id}/sequences/{new_id}"))
            .body(Json(new_id)),
    )
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
