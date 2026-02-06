use crate::AppState;
use crate::error::{Error, StorageError};
use crate::plugin_manager::plugin::Plugin;
use crate::storage::models::{Entry, EntryID, Metadata, Sequence, SequenceID};
use crate::storage::storage_manager::{Map, TxID};
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put, response::status};
use tracing::debug;

#[derive(serde::Serialize)]
pub struct PluginInfo {
    name: String,
    description: String,
    trigger: String,
    path: String,
    enabled: bool,
    valid: bool,
}

#[get("/plugins")]
pub fn list_plugins(state: &State<AppState>) -> Result<Json<Vec<PluginInfo>>, Error> {
    let pm = &state.plugin_manager;
    let plugins = pm
        .get_registered_plugins()
        .into_iter()
        .map(|p| PluginInfo {
            name: p.name().clone(),
            description: p.description().clone(),
            trigger: p.trigger().to_string(),
            path: p.path().to_string_lossy().into_owned(),
            enabled: p.enabled(),
            valid: p.valid(),
        })
        .collect();
    Ok(Json(plugins))
}
