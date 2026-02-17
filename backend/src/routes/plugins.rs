use crate::AppState;
use crate::error::Error;
use rocket::serde::json::Json;
use rocket::{State, get, post, put, response::status};

use tokio::time::{Duration, timeout};
use tracing::{debug, instrument};

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
    instance_id: Option<u64>,
    state: Option<crate::plugin_manager::manager::InstanceState>,
}

#[post("/plugins/<plugin_name>/start")]
pub async fn start_plugin_instance(
    state: &State<AppState>,
    plugin_name: &str,
) -> Result<Json<u64>, Error> {
    let instance_id = chrono::Utc::now().timestamp_micros().max(0) as u64;

    // Phase 1: global lock kurz halten
    let (plugin_index, plugin_path) = {
        let pm = lock_plugin_manager(state).await?;
        pm.prepare_start(plugin_name)?
    };

    // Phase 2: langsame Arbeit ohne globalen lock
    let handle = timeout(ROUTE_OP_TIMEOUT, async {
        let pm = lock_plugin_manager(state).await?;
        pm.build_started_instance(plugin_index, &plugin_path, instance_id)
            .await
    })
    .await
    .map_err(|_| Error::CustomError(format!("start timed out after {:?}", ROUTE_OP_TIMEOUT)))??;

    // Phase 3: commit wieder kurz unter globalem lock
    {
        let mut pm = lock_plugin_manager(state).await?;
        pm.commit_started_instance(instance_id, handle)?;
    }

    Ok(Json(instance_id))
}

#[put("/plugins/register")]
pub async fn register_plugins(state: &State<AppState>) -> Result<status::NoContent, Error> {
    // First: grab running handles under lock so we can stop them without holding the global lock
    let running_handles = {
        let pm = lock_plugin_manager(state).await?;
        pm.get_running_handles()
    };

    // Attempt to stop each running instance (best-effort, without holding the lock)
    for (instance_id, handle) in running_handles {
        let stop_res = timeout(
            ROUTE_OP_TIMEOUT,
            crate::plugin_manager::manager::PluginManager::stop_instance_handle(
                handle.clone(),
                instance_id,
            ),
        )
        .await;

        if let Err(_) = stop_res {
            tracing::warn!(
                "stop timed out for instance {} while rescanning plugins",
                instance_id
            );
        } else if let Ok(Err(e)) = stop_res {
            tracing::warn!(
                "stop failed for instance {} while rescanning plugins: {:?}",
                instance_id,
                e
            );
        }
    }

    // Now acquire lock and clear registered plugins, running map and history before re-registering
    {
        let mut pm = lock_plugin_manager(state).await?;
        pm.running.clear();
        pm.history.clear();
        pm.registered.clear();
        // perform registration into the now-empty manager
        pm.register_plugins(std::path::PathBuf::from("/plugins"))?;
    }

    Ok(status::NoContent)
}

#[put("/plugins/<plugin_name>/register")]
pub async fn register_plugin(
    state: &State<AppState>,
    plugin_name: &str,
) -> Result<status::NoContent, Error> {
    let mut pm = lock_plugin_manager(state).await?;
    pm.register_plugin(std::path::PathBuf::from(plugin_name))?;
    Ok(status::NoContent)
}

#[put("/plugins/<instance_id>/stop")]
pub async fn stop_plugin_instance(
    state: &State<AppState>,
    instance_id: u64,
) -> Result<status::NoContent, Error> {
    // Acquire a clone of the handle so we can call the async stop without holding the lock.
    let handle = {
        let pm = lock_plugin_manager(state).await?;
        pm.get_instance_handle(instance_id)?
    };

    timeout(
        ROUTE_OP_TIMEOUT,
        crate::plugin_manager::manager::PluginManager::stop_instance_handle(handle, instance_id),
    )
    .await
    .map_err(|_| Error::CustomError(format!("stop timed out after {:?}", ROUTE_OP_TIMEOUT)))??;

    // Record stopped instance in history and remove running handle under lock
    {
        let mut pm = lock_plugin_manager(state).await?;
        if let Ok(handle) = pm.take_instance_handle(instance_id) {
            // store plugin_index and Stopped state in history
            pm.record_history(
                instance_id,
                handle.plugin_index,
                crate::plugin_manager::manager::InstanceState::Stopped,
            );
        }
    }

    Ok(status::NoContent)
}

#[put("/plugins/<instance_id>/pause")]
pub async fn pause_plugin_instance(
    state: &State<AppState>,
    instance_id: u64,
) -> Result<status::NoContent, Error> {
    let handle = {
        let pm = lock_plugin_manager(state).await?;
        pm.get_instance_handle(instance_id)?
    };

    timeout(
        ROUTE_OP_TIMEOUT,
        crate::plugin_manager::manager::PluginManager::pause_instance_handle(handle, instance_id),
    )
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

    timeout(
        ROUTE_OP_TIMEOUT,
        crate::plugin_manager::manager::PluginManager::resume_instance_handle(handle, instance_id),
    )
    .await
    .map_err(|_| Error::CustomError(format!("resume timed out after {:?}", ROUTE_OP_TIMEOUT)))??;

    Ok(status::NoContent)
}

#[get("/plugin/instances")]
pub async fn get_plugin_instances(state: &State<AppState>) -> Result<Json<Vec<PluginInfo>>, Error> {
    let pm = lock_plugin_manager(state).await?;

    let mut results: Vec<PluginInfo> = Vec::new();

    // currently running (including Paused/Completed/Failed)
    for (p, instance_id, status) in pm.get_running_instances() {
        // treat disabled plugins as non-registered => skip their instances
        if !p.enabled() {
            continue;
        }
        results.push(PluginInfo {
            name: p.name().clone(),
            description: p.description().clone(),
            trigger: p.trigger().to_string(),
            path: p.path().to_string_lossy().into_owned(),
            enabled: p.enabled(),
            valid: p.valid(),
            instance_id: Some(instance_id),
            state: Some(status),
        });
    }

    // include stopped/recorded instances from history
    for (p, instance_id, status) in pm.get_history_instances() {
        // skip instances for disabled plugins as well
        if !p.enabled() {
            continue;
        }
        results.push(PluginInfo {
            name: p.name().clone(),
            description: p.description().clone(),
            trigger: p.trigger().to_string(),
            path: p.path().to_string_lossy().into_owned(),
            enabled: p.enabled(),
            valid: p.valid(),
            instance_id: Some(instance_id),
            state: Some(status),
        });
    }

    Ok(Json(results))
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
        .filter(|p| p.enabled())
        .map(|p| PluginInfo {
            name: p.name().clone(),
            description: p.description().clone(),
            trigger: p.trigger().to_string(),
            path: p.path().to_string_lossy().into_owned(),
            enabled: p.enabled(),
            valid: p.valid(),
            instance_id: None,
            state: None,
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
    plugin_name: &str,
) -> Result<status::NoContent, Error> {
    let mut pm = lock_plugin_manager(state).await?;
    pm.disable_plugin(plugin_name)?;
    Ok(status::NoContent)
}
