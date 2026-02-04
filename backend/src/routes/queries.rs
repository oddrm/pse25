

#![allow(unused_variables)]

use crate::{AppState, storage};
use crate::error::Error;
use crate::schema::entries::updated_at;
use crate::schema::sequences::end_timestamp;
use crate::storage::models::{Entry, EntryID, Sequence, SequenceID};
use crate::storage::storage_manager::{Map, StorageManager, TxID};
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put, response::status};

//Kapitel 5.1.2 im Entwurfsheft (falls noch andere das ewig suchen)
#[get("/entries/<entry_id>/metadata")]
pub async fn get_metadata(
    state: &State<AppState>,
    entry_id: EntryID,
) -> Result<Json<Metadata>, Error> {
    let storage = &state.storage_manager;
    let txid = storage.get_transaction_id();

    let meta_opt = storage.get_metadata(entry_id, txid).await?;
    storage.end_transaction(txid)?;

    match meta_opt {
        Some(m) => Ok(Json(m)),
        None => not_found(format!("metadata for entry {entry_id} not found")),
    }
}

#[put("/entries/<entry_id>/metadata", format = "json", data = "<metadata>")]
pub async fn update_metadata(
    state: &State<AppState>,
    entry_id: EntryID,
    metadata: String,
) -> Result<status::NoContent, Error> {
    let storage: &StorageManager = &state.storage_manager;
    let txid = storage.get_transaction_id();

    let value: serde_json::Value =
        serde_json::from_str(&metadata_str).map_err(|e| Error::ParsingError(e.to_string()))?;

   
    let metadata = Metadata {
        id: 0,                    
        entry_id,
        metadata_json: Some(value),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let _updated_entry_id = storage.update_metadata(entry_id, &metadata, txid).await?;
    storage.end_transaction(txid)?;

    Ok(status::NoContent)}



#[get("/entries?<search_string>&<sort_by>&<ascending>&<page>&<page_size>")]
pub async fn get_entries(
    state: &State<AppState>,
    search_string: Option<String>,
    sort_by: Option<String>,
    ascending: Option<bool>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<Json<Vec<Entry>>, Error> {
    let storage = &state.storage_manager;
    let txid = storage.get_transaction_id();

    
    let pairs = storage
        .get_entries(search_string, sort_by, ascending, page, page_size, txid)
        .await?;

    storage.end_transaction(txid)?;
}

#[get("/entries/<entry_id>")]
pub async fn get_entry(state: &State<AppState>, entry_id: EntryID) -> Result<Json<Entry>, Error> {
    let storage: &StorageManager = &state.storage_manager;
    let txid = storage.get_transaction_id();

    let entry = storage.get_entry(entry_id, txid).await?;
    storage.end_transaction(txid);

}

#[get("/paths/<path>")]
pub async fn get_entry_by_path(
    state: &State<AppState>,
    path: String,
) -> Result<Json<Entry>, Error> {
    let storage: &StorageManager  = &state.storage_manager ;
    let txid: u64 = storage.get_transaction_id();

    let get_entry_by_path: Option<Entry> = storage.get_entry_by_path(&path, txid).await?;

    storage.end_transaction(txid);

}

#[get("/entries/<entry_id>/sequences")]
pub async fn get_sequences(
    state: &State<AppState>,
    entry_id: EntryID,
) -> Result<Json<Map<SequenceID, Sequence>>, Error> {
    let storage: &StorageManager  = &state.storage_manager ;
    let txid: u64 = storage.get_transaction_id();

    let sequence = storage.get_sequences(id, txid).await?;

    storage.end_transaction(txid);

}
/*
#[post("/entries/<entry_id>/sequences", format = "json", data = "<sequence_request>")]
pub async fn add_sequence(
    state: &State<AppState>,
    entry_id: EntryID,
    sequence_request: Json<NewSequenceRequest>,
) -> Result<status::Created<Json<SequenceID>>, Error> {
    let storage: &StorageManager  = &state.storage_manager ;
    let txid: u64 = storage.get_transaction_id();

    if sequence_request.end_timestamp < sequence_request.start_timestamp {
        storage.end_transaction(txid)?;
        return Err(Error::CustomError(
            "end_timestamp must be >= start_timestamp".to_string(),
        ));
    }

    let seq_id = storage
        .add_sequence(
            ,
            &sequence_request.description,
            sequence_request.start_timestamp,
            sequence_request.end_timestamp,
        )
        .await?;

    storage.end_transaction(txid);
    Ok(status::Created::new(format!(
        "/entries/{}/sequences/{}",
        entry_id, seq_id
    ))
    .body(Json(seq_id)))
    
}
 */


 #[put("/entries/<entry_id>/sequences/<sequence_id>", format = "json", data = "<sequence_str>")]
 pub async fn update_sequence(
     state: &State<AppState>,
     entry_id: EntryID,
     sequence_id: SequenceID,
     sequence_str: String,
 ) -> Result<status::NoContent, Error> {
     let storage: &StorageManager = &state.storage_manager;
     let txid = storage.get_transaction_id();
 
     // JSON body -> Sequence parse
     let mut new_sequence_value: Sequence = serde_json::from_str(&sequence_str)
         .map_err(|e| Error::ParsingError(e.to_string()))?;
 
     
     new_sequence_value.id = sequence_id;
     new_sequence_value.entry_id = entry_id;
 
     
     if new_sequence_value.end_timestamp < new_sequence_value.start_timestamp {
         storage.end_transaction(txid)?;
         return Err(Error::CustomError("end_timestamp must be > start_timestamp".to_string()));
     }
 
     storage.update_sequence(entry_id, sequence_id, new_sequence_value, txid).await?;
     storage.end_transaction(txid)?;
 
     Ok(status::NoContent)
 }

#[delete("/entries/<entry_id>/sequences/<sequence_id>")]
pub async fn remove_sequence(
    state: &State<AppState>,
    entry_id: EntryID,
    sequence_id: SequenceID,
) -> Result<status::NoContent, Error> {
    let storage: &StorageManager = &state.storage_manager;
    let txid: u64 = storage.get_transaction_id();

    storage.remove_sequence(entry_id, sequence_id, txid).await?;
    storage.end_transaction(txid)?;

    Ok(status::NoContent);
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

