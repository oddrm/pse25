use crate::AppState;
use crate::error::{Error, StorageError};
use crate::plugin_manager::plugin::Plugin;
use crate::storage::models::{Entry, EntryID, Metadata, Sequence, SequenceID};
use crate::storage::storage_manager::{Map, TxID};
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put, response::status};
use tracing::debug;

use tokio::time::{timeout, Duration};

const PM_LOCK_TIMEOUT: Duration = Duration::from_secs(1);

// Optional (Route-Level): harte Obergrenze für "langsame" Operationen.
// Das ist unabhängig vom Lock-Timeout und schützt euch vor ewig laufenden Requests.
const ROUTE_OP_TIMEOUT: Duration = Duration::from_secs(10);

async fn lock_plugin_manager(
    state: &State<AppState>,
) -> Result<tokio::sync::MutexGuard<'_, crate::plugin_manager::manager::PluginManager>, Error> {
    timeout(PM_LOCK_TIMEOUT, state.plugin_manager.lock())
        .await
        .map_err(|_| {
            Error::CustomError(format!(
                "Plugin manager is busy (lock timeout after {:?}). Please retry.",
                PM_LOCK_TIMEOUT
            ))
        })
}
#[derive(serde::Serialize)]
pub struct PluginInfo {
    name: String,
    description: String,
    trigger: String,
    path: String,
    enabled: bool,
    valid: bool,
}

#[put("/plugins/<plugin_name>/start")]
pub async fn start_plugin_instance(
    state: &State<AppState>,
    plugin_name: String,
) -> Result<Json<u64>, Error> {
    let instance_id = chrono::Utc::now().timestamp_millis().max(0) as u64;

    // Phase 1: global lock kurz halten (prüfen + Daten holen)
    let (plugin_index, plugin_path) = {
        let pm = lock_plugin_manager(state).await?;
        pm.prepare_start(&plugin_name)?
    }; // <- pm DROPPED hier, bevor wir irgendwas .await-en

    // Phase 2: langsame Arbeit ohne globalen lock
    let inst = timeout(
        ROUTE_OP_TIMEOUT,
        async {
            // build braucht &PluginManager, aber hält keinen Mutex
            let pm = lock_plugin_manager(state).await?; // nur um &self zu bekommen
            // WICHTIG: wir rufen hier NUR die "build" Methode auf, die keine Map mutiert.
            pm.build_started_instance(plugin_index, &plugin_path, instance_id).await
        },
    )
        .await
        .map_err(|_| Error::CustomError(format!("start timed out after {:?}", ROUTE_OP_TIMEOUT)))??;

    // Phase 3: commit wieder kurz unter globalem lock
    {
        let mut pm = lock_plugin_manager(state).await?;
        pm.commit_started_instance(instance_id, inst)?;
    }

    Ok(Json(instance_id))
}

#[post("/plugins/register")]
pub async fn register_plugins(state: &State<AppState>) -> Result<status::NoContent, Error> {
    let mut pm = lock_plugin_manager(state).await?;
    pm.register_plugins(std::path::PathBuf::from("/plugins"))?;
    Ok(status::NoContent)
}

#[post("/plugins/<plugin_id>/register")]
pub async fn register_plugin(
    state: &State<AppState>,
    plugin_id: String,
) -> Result<status::NoContent, Error> {
    let mut pm = lock_plugin_manager(state).await?;
    pm.register_plugin(std::path::PathBuf::from(plugin_id))?;
    Ok(status::NoContent)
}

#[put("/plugins/<instance_id>/stop")]
pub async fn stop_plugin_instance(
    state: &State<AppState>,
    instance_id: u64,
) -> Result<status::NoContent, Error> {
    // take() entfernt die Instanz sofort aus running => UI blockiert nicht auf "running"-Liste
    let handle = {
        let mut pm = lock_plugin_manager(state).await?;
        pm.take_instance_handle(instance_id)?
    };

    timeout(ROUTE_OP_TIMEOUT, crate::plugin_manager::manager::PluginManager::stop_instance_handle(handle, instance_id))
        .await
        .map_err(|_| Error::CustomError(format!("stop timed out after {:?}", ROUTE_OP_TIMEOUT)))??;

    Ok(status::NoContent)
}

#[put("/plugins/<instance_id>/pause")]
pub async fn pause_plugin_instance(
    state: &State<AppState>,
    instance_id: u64,
) -> Result<status::NoContent, Error> {
    // Handle unter globalem lock holen, dann lock droppen
    let handle = {
        let pm = lock_plugin_manager(state).await?;
        pm.get_instance_handle(instance_id)?
    };

    timeout(ROUTE_OP_TIMEOUT, crate::plugin_manager::manager::PluginManager::pause_instance_handle(handle, instance_id))
        .await
        .map_err(|_| Error::CustomError(format!("pause timed out after {:?}", ROUTE_OP_TIMEOUT)))??;

    Ok(status::NoContent)
}

#[put("/plugins/<instance_id>/resume")]
pub async fn resume_plugin_instance(
    state: &State<AppState>,
    instance_id: u64,
) -> Result<status::NoContent, Error> {
    let handle = {
        let pm = lock_plugin_manager(state).await?;
        pm.get_instance_handle(instance_id)?
    };

    timeout(ROUTE_OP_TIMEOUT, crate::plugin_manager::manager::PluginManager::resume_instance_handle(handle, instance_id))
        .await
        .map_err(|_| Error::CustomError(format!("resume timed out after {:?}", ROUTE_OP_TIMEOUT)))??;

    Ok(status::NoContent)
}

#[get("/plugins/running")]
pub async fn get_running_instances(
    state: &State<AppState>,
) -> Result<Json<Vec<PluginInfo>>, Error> {
    let pm = lock_plugin_manager(state).await?;

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

// Die GETs bleiben ok: sie locken kurz und awaiten nichts "Langsames".
#[get("/plugins/registered")]
pub async fn get_registered_plugins(
    state: &State<AppState>,
) -> Result<Json<Vec<PluginInfo>>, Error> {
    let pm = lock_plugin_manager(state).await?;

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
    let mut pm = lock_plugin_manager(state).await?;
    pm.enable_plugin(plugin_name)?;
    Ok(status::NoContent)
}

#[put("/plugins/<plugin_name>/disable")]
pub async fn disable_plugin(
    state: &State<AppState>,
    plugin_name: String,
) -> Result<status::NoContent, Error> {
    let mut pm = lock_plugin_manager(state).await?;
    pm.disable_plugin(&plugin_name)?;
    Ok(status::NoContent)
}