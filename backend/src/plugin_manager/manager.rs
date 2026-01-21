#![allow(unused)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;

use tokio::sync::mpsc::Sender;

use crate::{error::Error, plugin_manager::plugin::Plugin, storage::storage_instance::Event};
use crate::plugin_manager::manager::InstanceState::Running;
use crate::plugin_manager::plugin::Trigger;

// Debug: {:?} in println!; textuelle Darstellung Typ bei Debuggen
// Clone: Kopie erstellen; Copy: Typ bei Zuweisen/ Übergeben kopiert nicht bewegt
// PartialEq: Vergleich == wird möglich
// Eq: Gleichheit vollständig definiert (kein NaN-Sonderfall)
//TODO: gehört evtl. eher in Plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceState {
    Running,
    Paused,
}

#[derive(Debug, Clone)]
pub struct PluginManager {
    //Sender Messages Typ Event; Kanal Events an andere Threads
    event_tx: Sender<Event>,
    //Liste registrierte Plugins
    registered: Vec<Plugin>,
    //HashMap Instanz auf Index in registriert mit Zustand Instanz
    running: HashMap<InstanceID, (usize, InstanceState)>,
}

// TODO use PluginError later, now too much refactoring needed
impl PluginManager {
    pub fn new(event_transmitter: Sender<Event>) -> Self {
        Self {
            event_tx: event_transmitter,
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
        let name = path
            .file_stem()//Dateinamen ohne Endung
            .and_then(|s| s.to_str()) // in &str umwandeln
            .unwrap_or("unknown") // worstcasename: unknown
            .to_string();

        // dieses Schreiben für Log-Datei
        let description = format!("Plugin loaded from {:?}", path);

        // TODO: später aus Metadaten/Config ableiten
        let trigger = Trigger::Manual;

        let plugin = Plugin::new(name, description, trigger, path);
        self.registered.push(plugin);
        Ok(())
    }

    pub fn start_plugin_instance(
        &mut self,
        plugin: &Plugin,
        parameters: Vec<(String, String)>,
        temp_directory: PathBuf,
        instance_id: InstanceID,
    ) -> Result<(), Error> {
        //läuft schon?
        if self.running.contains_key(&instance_id) {
            return Err(Error::CustomError(format!(
                "Instance {} is already running",
                instance_id
            )));
        }
        //Suche in Liste Registrierter
        let plugin_index = self
            .registered
            .iter() // suche gleichen Namen
            .position(|p| p.name() == plugin.name())//Closure
            .ok_or_else(|| {
                Error::CustomError(format!(
                    "Plugin '{}' is not registered",
                    plugin.name()
                ))
            })?;
        //TODO: in Phyton Process starten
        //Liste hinzufügen
        self.running.insert(instance_id, (plugin_index, Running));
        Ok(())
    }

    pub fn stop_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        //Entfernen aus Liste; match auf Fälle
        let plugin_index = match self.running.remove(&instance_id) {
            Some(idx) => idx, //diese Instanz gefunden
            None => {
                // Instanz existiert nicht / läuft nicht
                return Err(Error::CustomError(format!(
                    "Instance {} is not running",
                    instance_id
                )));
            }
        };
        //TODO: in Python stoppen
        Ok(())
    }

    pub fn pause_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        // Eintrag holen
        let entry = self
            .running
            .get_mut(&instance_id) // Suche Key InstanceId
            .ok_or_else(|| Error::CustomError(format!(
                "Instance {} is not running",
                instance_id
            )))?;

        // entry: &mut (usize, InstanceState) => (plugin_index, state)
        // entry.1 ist state
        if entry.1 == InstanceState::Paused {
            return Ok(());
        }

        // Status auf Paused setzen
        entry.1 = InstanceState::Paused;

        // TODO: hier später Prozess pausieren

        Ok(())
    }

    //Gegenstück Pause
    pub fn resume_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        // Eintrag zur Instanz holen
        let entry = self
            .running
            .get_mut(&instance_id)
            .ok_or_else(|| Error::CustomError(format!(
                "Instance {} is not running",
                instance_id
            )))?;

        // entry: &mut (usize, InstanceState)
        if entry.1 == InstanceState::Running {
            return Ok(());
        }

        // Status auf Running setzen
        entry.1 = InstanceState::Running;

        // TODO: hier Prozess wieder anstoßen

        Ok(())
    }

    pub fn get_running_instances(&self) -> Vec<(&Plugin, InstanceID)> {
        self.running
            .iter()
            .filter_map(|(instance_id, (plugin_index, state))| {
                // | Closure Syntax übrigens; *state ist Wert von state
                if *state == InstanceState::Running {
                    let plugin = &self.registered[(*plugin_index)];
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
}

type InstanceID = u64;
