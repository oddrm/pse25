#![allow(unused)]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use pyo3::prelude::*;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};
use tokio::sync::mpsc::Sender;
use tracing::warn;

use crate::plugin_manager::manager::InstanceState::Running;
use crate::plugin_manager::plugin::Trigger;
use crate::plugin_manager::python_bridge;
use crate::{
    error::Error, plugin_manager::plugin::Plugin, storage::storage_manager::StorageManager,
};

// Debug: {:?} in println!; textuelle Darstellung Typ bei Debuggen
// Clone: Kopie erstellen; Copy: Typ bei Zuweisen/ Übergeben kopiert nicht bewegt
// PartialEq: Vergleich == wird möglich
// Eq: Gleichheit vollständig definiert (kein NaN-Sonderfall)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceState {
    Running,
    Paused,
}

#[derive(Debug)]
struct RunningInstance {
    plugin_index: usize,
    state: InstanceState,
    py_instance: Py<PyAny>,
    run_task: JoinHandle<()>,
}

#[derive(Debug)]
pub struct PluginManager {
    //Sender Messages Typ Event; Kanal Events an andere Threads
    storage_manager: StorageManager,
    //Liste registrierte Plugins
    registered: Vec<Plugin>,
    //HashMap Instanz auf Index in registriert mit Zustand Instanz
    running: HashMap<InstanceID, RunningInstance>,
}

// TODO: use PluginError later, now too much refactoring needed
impl PluginManager {
    pub fn new(storage_manager: StorageManager) -> Self {
        Self {
            storage_manager,
            registered: Vec::new(),
            running: HashMap::new(),
        }
    }

    pub fn register_plugins(&mut self, directory: PathBuf) -> Result<(), Error> {
        // über Verzeichnis
        for entry in fs::read_dir(&directory).map_err(Error::from)? {
            let entry = entry.map_err(Error::from)?;
            //bekommt passenden PathBuf
            let path = entry.path();

            self.register_plugin(path)?;
        }
        Ok(())
    }

    pub fn register_plugin(&mut self, path: PathBuf) -> Result<(), Error> {
        // 1) Validieren: importierbar + PluginImpl vorhanden
        let warnings = python_bridge::validate_plugin_module(path.as_path())?;
        for w in &warnings {
            warn!("{w}");
        }

        // 2) Constants lesen (best-effort, fallback wenn nicht vorhanden)
        let fallback_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let fallback_description = format!("Plugin loaded from {:?}", path);

        let (py_name, py_description, py_trigger) =
            python_bridge::read_module_constants(path.as_path())
                .unwrap_or((None, None, None));

        let name = py_name.unwrap_or(fallback_name);
        let description = py_description.unwrap_or(fallback_description);

        let trigger = match py_trigger.as_deref() {
            Some("manual") | None => Trigger::Manual,
            _ => Trigger::Manual, // TODO: später weitere Trigger mappen
        };

        let mut plugin = Plugin::new(name, description, trigger, path);
        plugin.set_valid(true);
        plugin.set_validation_warnings(warnings);

        self.registered.push(plugin);
        Ok(())
    }

    pub fn start_plugin_instance_by_name(
        &mut self,
        plugin_name: &str,
        parameters: Vec<(String, String)>,
        temp_directory: PathBuf,
        instance_id: InstanceID,
    ) -> Result<(), Error> {
        // läuft schon?
        if self.running.contains_key(&instance_id) {
            return Err(Error::CustomError(format!(
                "Instance {} is already running",
                instance_id
            )));
        }

        // registriertes Plugin finden
        let plugin_index = self
            .registered
            .iter()
            .position(|p| p.name().as_str() == plugin_name)
            .ok_or_else(|| Error::CustomError(
                format!("Plugin '{}' is not registered", plugin_name)))?;

        let reg_plugin = &self.registered[plugin_index];

        // Statuscheck
        if !reg_plugin.valid() {
            return Err(Error::CustomError(format!(
                "Plugin '{}' is invalid and cannot be started",
                reg_plugin.name()
            )));
        }
        if !reg_plugin.enabled() {
            return Err(Error::CustomError(format!(
                "Plugin '{}' is disabled",
                reg_plugin.name()
            )));
        }

        // Python-Instanz erzeugen und speichern
        let py_instance = python_bridge::load_plugin_instance(
            reg_plugin.path().as_path())?;

        // run() im Hintergrund ausführen
        let py_instance_for_task = Python::with_gil(
            |py| py_instance.clone_ref(py));
        let run_task = tokio::task::spawn_blocking(move || {
            let _ = python_bridge::call_run(&py_instance_for_task, "start");
        });

        self.running.insert(
            instance_id,
            RunningInstance {
                plugin_index,
                state: InstanceState::Running,
                py_instance,
                run_task,
            },
        );

        Ok(())
    }

    // Optional: bestehende API delegiert auf by_name
    pub fn start_plugin_instance(
        &mut self,
        plugin: &Plugin,
        parameters: Vec<(String, String)>,
        temp_directory: PathBuf,
        instance_id: InstanceID,
    ) -> Result<(), Error> {
        self.start_plugin_instance_by_name(
            plugin.name().as_str(),
            parameters,
            temp_directory,
            instance_id,
        )
    }

    // AUFRUF: plugin_manager.stop_plugin_instance(instance_id).await?;
    pub async fn stop_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        let entry = self.running.remove(&instance_id).ok_or_else(|| {
            Error::CustomError(format!("Instance {} is not running", instance_id))
        })?;

        // 1) Python kooperativ stoppen (setzt im Plugin ein Flag/Event)
        let _stop_result = python_bridge::call_stop(&entry.py_instance)?;

        // 2) Warten bis run() wirklich beendet ist
        // Ohne Timeout:
        // let _ = entry.run_task.await;

        // Mit Timeout (empfohlen, damit du nicht ewig hängst):
        match timeout(Duration::from_secs(10), entry.run_task).await {
            Ok(_join_result) => Ok(()),
            Err(_elapsed) => Err(Error::CustomError(
                "Stop timed out: plugin did not exit within 10s".to_string(),
            )),
        }
    }

    pub fn pause_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        // Eintrag holen
        let entry = self
            .running
            .get_mut(&instance_id) // Suche Key InstanceId
            .ok_or_else(|| {
                Error::CustomError(format!("Instance {} is not running", instance_id))
            })?;

        if entry.state == InstanceState::Paused {
            return Ok(());
        }

        // Python kooperativ pausieren
        let _pause_result = python_bridge::call_pause(&entry.py_instance)?;

        entry.state = InstanceState::Paused;

        Ok(())
    }

    //Gegenstück Pause
    pub fn resume_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        // Eintrag zur Instanz holen
        let entry = self.running.get_mut(&instance_id).ok_or_else(|| {
            Error::CustomError(format!("Instance {} is not running", instance_id))
        })?;

        if entry.state == InstanceState::Running {
            return Ok(());
        }

        // Python fortsetzen
        let _resume_result = python_bridge::call_resume(&entry.py_instance)?;

        entry.state = InstanceState::Running;

        Ok(())
    }

    pub fn get_running_instances(&self) -> Vec<(&Plugin, InstanceID)> {
        self.running
            .iter()
            .filter_map(|(instance_id, entry)| {
                if entry.state == InstanceState::Running {
                    let plugin = &self.registered[entry.plugin_index];
                    Some((plugin, *instance_id))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_registered_plugins(&self) -> Vec<&Plugin> {
        self.registered.iter().collect()
    }

    pub fn enable_plugin(&mut self, name: &str) -> Result<(), Error> {
        let plugin = self
            .registered
            .iter_mut()
            .find(|p| p.name().as_str() == name)
            .ok_or_else(|| Error::CustomError(format!("Plugin '{name}' not found")))?;

        plugin.set_enabled(true);
        Ok(())
    }

    pub fn disable_plugin(&mut self, name: &str) -> Result<(), Error> {
        let plugin = self
            .registered
            .iter_mut()
            .find(|p| p.name().as_str() == name)
            .ok_or_else(|| Error::CustomError(format!("Plugin '{name}' not found")))?;

        plugin.set_enabled(false);
        Ok(())
    }

}

type InstanceID = u64;