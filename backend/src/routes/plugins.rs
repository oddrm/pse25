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

#[post("/plugins/register")]
pub async fn register_plugins(state: &State<AppState>) -> Result<status::NoContent, Error> {
    let mut pm = state.plugin_manager.lock().await;
    pm.register_plugins(std::path::PathBuf::from("/plugins"))?;
    Ok(status::NoContent)
}

#[post("/plugins/<plugin_id>/register")]
pub async fn register_plugin(
    state: &State<AppState>,
    plugin_id: String,
) -> Result<status::NoContent, Error> {
    let mut pm = state.plugin_manager.lock().await;
    pm.register_plugin(std::path::PathBuf::from(plugin_id))?;
    Ok(status::NoContent)
}

#[put("/plugins/<plugin_name>/start")]
pub async fn start_plugin_instance(
    state: &State<AppState>,
    plugin_name: String,
) -> Result<Json<u64>, Error> {
    let mut pm = state.plugin_manager.lock().await;
    // id entsteht aus Zeitstempel in Millisekunden
    let instance_id = chrono::Utc::now().timestamp_millis().max(0) as u64;

    pm.start_plugin_instance(
        &plugin_name,
        std::path::PathBuf::from("/tmp"),
        instance_id,
    )
    .await?;

    Ok(Json(instance_id))
}

#[put("/plugins/<instance_id>/stop")]
pub async fn stop_plugin_instance(
    state: &State<AppState>,
    instance_id: u64,
) -> Result<status::NoContent, Error> {
    let mut pm = state.plugin_manager.lock().await;
    pm.stop_plugin_instance(instance_id).await?;
    Ok(status::NoContent)
}

#[put("/plugins/<instance_id>/pause")]
pub async fn pause_plugin_instance(
    state: &State<AppState>,
    instance_id: u64,
) -> Result<status::NoContent, Error> {
    let mut pm = state.plugin_manager.lock().await;
    pm.pause_plugin_instance(instance_id).await?;
    Ok(status::NoContent)
}

#[put("/plugins/<instance_id>/resume")]
pub async fn resume_plugin_instance(
    state: &State<AppState>,
    instance_id: u64,
) -> Result<status::NoContent, Error> {
    let mut pm = state.plugin_manager.lock().await;
    pm.resume_plugin_instance(instance_id).await?;
    Ok(status::NoContent)
}

#[get("/plugins/running")]
pub async fn get_running_instances(
    state: &State<AppState>,
) -> Result<Json<Vec<PluginInfo>>, Error> {
    let pm = state.plugin_manager.lock().await;

    let plugins = pm
        .get_running_instances()
        .into_iter()
        .map(|(p, _instance_id)| PluginInfo {
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

#[get("/plugins/registered")]
pub async fn get_registered_plugins(
    state: &State<AppState>,
) -> Result<Json<Vec<PluginInfo>>, Error> {
    let pm = state.plugin_manager.lock().await;

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

#[put("/plugins/<plugin_name>/enable")]
pub async fn enable_plugin(
    state: &State<AppState>,
    plugin_name: &str,
) -> Result<status::NoContent, Error> {
    let mut pm = state.plugin_manager.lock().await;
    pm.enable_plugin(plugin_name)?;
    Ok(status::NoContent)
}

#[put("/plugins/<plugin_name>/disable")]
pub async fn disable_plugin(
    state: &State<AppState>,
    plugin_name: String,
) -> Result<status::NoContent, Error> {
    let mut pm = state.plugin_manager.lock().await;
    pm.disable_plugin(&plugin_name)?;
    Ok(status::NoContent)
}