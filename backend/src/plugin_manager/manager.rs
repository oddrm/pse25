#![allow(unused)]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, warn};

use crate::plugin_manager::manager::InstanceState::Running;
use crate::plugin_manager::plugin::Trigger;
use crate::plugin_manager::python_bridge;
use crate::{
    error::Error, plugin_manager::plugin::Plugin, storage::storage_manager::StorageManager,
};

// -------------------- constants --------------------

const TRIGGER_MANUAL: &str = "manual";
const TRIGGER_ON_ENTRY_CREATE: &str = "on_entry_create";
const TRIGGER_ON_ENTRY_UPDATE: &str = "on_entry_update";
const TRIGGER_ON_ENTRY_DELETE: &str = "on_entry_delete";

const FALLBACK_PLUGIN_NAME: &str = "unknown";

const TRIGGER_ON_SCHEDULE_PREFIX: &str = "on_schedule:";

const JSON_KEY_INSTANCE_ID: &str = "instance_id";
const JSON_KEY_REQUEST_ID: &str = "request_id";
const JSON_KEY_CMD: &str = "cmd";


const PYTHON_EXECUTABLE: &str = "python";
const PYTHON_UNBUFFERED_FLAG: &str = "-u";

// Achtung: Pfad muss zu deiner echten Datei passen.
const RUNNER_PATH: &str = "src/plugin_manager/plugins/plugin_runner.py";

const ARG_PLUGIN_PATH: &str = "--plugin-path";
const ARG_INSTANCE_ID: &str = "--instance-id";

const ERR_FAILED_READ_CONFIG_PREFIX: &str = "Failed to read config: ";
const ERR_FAILED_PARSE_CONFIG_PREFIX: &str = "Failed to parse config: ";
const ERR_PLUGIN_NOT_FOUND_PREFIX: &str = "Plugin '";
const ERR_PLUGIN_NOT_REGISTERED_PREFIX: &str = "Plugin '";
const ERR_INSTANCE_ALREADY_RUNNING_PREFIX: &str = "Instance ";
const ERR_INSTANCE_NOT_RUNNING_PREFIX: &str = "Instance ";
const ERR_FAILED_SPAWN_PY_PREFIX: &str = "Failed to spawn python runner: ";
const ERR_FAILED_OPEN_STDIN: &str = "Failed to open stdin for python runner";
const ERR_FAILED_OPEN_STDOUT: &str = "Failed to open stdout for python runner";
const ERR_PY_STDOUT_CLOSED: &str = "Python runner stdout closed";
const ERR_UNKNOWN_ERROR: &str = "unknown_error";
const ERR_FAILED_KILL_PY_PREFIX: &str = "Failed to kill python runner: ";
const ERR_FAILED_SEND_CMD_PREFIX: &str = "Failed to send cmd to python runner: ";
const ERR_FAILED_FLUSH_CMD_PREFIX: &str = "Failed to flush cmd to python runner: ";

const CMD_START: &str = "start";
const CMD_STOP: &str = "stop";
const CMD_PAUSE: &str = "pause";
const CMD_RESUME: &str = "resume";

const LOG_PY_STDERR_PREFIX: &str = "python stderr: {}";
const LOG_PY_STDOUT_NON_JSON: &str = "python stdout (non-json): {} (parse err: {})";
const LOG_RUNNER_EVENT: &str = "runner event (instance {}): {}";

const LOG_SOFT_STOP_FORCE_KILL: &str = "Soft stop ACK ok, but process did not exit quickly; forcing kill.";
const LOG_SOFT_STOP_FAILED_FORCE_KILL: &str = "Soft stop failed/timeout; forcing kill. err={:?}";

const TIMEOUT_START_ACK: Duration = Duration::from_secs(5);
const TIMEOUT_SOFT_STOP_ACK: Duration = Duration::from_secs(2);
const TIMEOUT_PAUSE_ACK: Duration = Duration::from_secs(2);
const TIMEOUT_RESUME_ACK: Duration = Duration::from_secs(2);
const TIMEOUT_WAIT_EXIT_AFTER_SOFT_STOP: Duration = Duration::from_secs(2);
const TIMEOUT_WAIT_EXIT_AFTER_KILL: Duration = Duration::from_secs(2);

type InstanceID = u64;

// ---------- helpers (module-internal) ----------
fn parse_trigger(py_trigger: Option<&str>) -> Trigger {
    match py_trigger {
        // Trigger extrahieren
        Some(TRIGGER_MANUAL) | None => Trigger::Manual,
        Some(TRIGGER_ON_ENTRY_CREATE) => Trigger::OnEntryCreate,
        Some(TRIGGER_ON_ENTRY_UPDATE) => Trigger::OnEntryUpdate,
        Some(TRIGGER_ON_ENTRY_DELETE) => Trigger::OnEntryDelete,
        Some(other) if other.starts_with(TRIGGER_ON_SCHEDULE_PREFIX) => Trigger::OnSchedule(
            other
                .trim_start_matches(TRIGGER_ON_SCHEDULE_PREFIX)
                .trim()
                .to_string(),
        ),
        _ => Trigger::Manual,
    }
}
// Rückgabe ob pausiert
fn is_paused(inst: &RunningInstance) -> bool { inst.state == InstanceState::Paused }

// Rückgabe ob laufend
fn is_running(inst: &RunningInstance) -> bool { inst.state == Running }

// baut json aus instanz/request_id und cmd
fn build_cmd_request(instance_id: InstanceID, request_id: &str, cmd: &str) -> serde_json::Value {
    let mut req = serde_json::Map::new();
    req.insert(JSON_KEY_INSTANCE_ID.to_string(), serde_json::Value::from(instance_id));
    req.insert(JSON_KEY_REQUEST_ID.to_string(), serde_json::Value::from(request_id));
    req.insert(JSON_KEY_CMD.to_string(), serde_json::Value::from(cmd));
    serde_json::Value::Object(req)
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceState {
    Running,
    Paused,
}

#[derive(Debug, Deserialize)]
struct RunnerMsg {
    instance_id: u64,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    ok: Option<bool>,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    trace: Option<String>,
    #[serde(default)]
    event: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct PluginsConfig {
    pub plugins: Vec<PluginConfig>,
}

#[derive(Debug)]
struct RunningInstance {
    plugin_index: usize, // welches Plugin auf Liste
    state: InstanceState, // Running/Paused
    child: Child, // Handle auf gestarteten Python Prozess
    child_stdin: ChildStdin, // Schreibkanal zum Python Prozess
    stdout_rx: mpsc::Receiver<RunnerMsg>, // Empfangschannel vom Python Prozess
    next_request_seq: u64, // Zähler für eindeutige Request-IDs
}

#[derive(Debug)]
pub struct PluginManager {
    storage_manager: StorageManager,
    registered: Vec<Plugin>,
    running: HashMap<InstanceID, RunningInstance>,
}

impl PluginManager {
    // async Wrapper der Befehle an Python Prozess
    async fn cmd_start(inst: &mut RunningInstance, id: InstanceID) -> Result<RunnerMsg, Error> {
        Self::send_cmd_ack(inst, id, CMD_START, TIMEOUT_START_ACK).await
    }
    
    async fn cmd_stop(inst: &mut RunningInstance, id: InstanceID) -> Result<RunnerMsg, Error> {
        Self::send_cmd_ack(inst, id, CMD_STOP, TIMEOUT_SOFT_STOP_ACK).await
    }
    async fn cmd_pause(inst: &mut RunningInstance, id: InstanceID) -> Result<RunnerMsg, Error> {
        Self::send_cmd_ack(inst, id, CMD_PAUSE, TIMEOUT_PAUSE_ACK).await
    }
    async fn cmd_resume(inst: &mut RunningInstance, id: InstanceID) -> Result<RunnerMsg, Error> {
        Self::send_cmd_ack(inst, id, CMD_RESUME, TIMEOUT_RESUME_ACK).await
    }

    // alles notwendige neu erzeugt
    pub fn new(storage_manager: StorageManager) -> Self {
        Self {
            storage_manager,
            registered: Vec::new(),
            running: HashMap::new(),
        }
    }

    // returned laufende Instanz oder Error
    fn get_running_instance_mut(
        &mut self,
        instance_id: InstanceID,
    ) -> Result<&mut RunningInstance, Error> {
        self.running.get_mut(&instance_id).ok_or_else(|| {
            Error::CustomError(format!(
                "{ERR_INSTANCE_NOT_RUNNING_PREFIX}{} is not running",
                instance_id
            ))
        })
    }

    // TODO eventuell if-Anweisung überarbeiten
    pub fn load_config_and_apply(&mut self, config_path: &str) -> Result<(), Error> {
        // YAML-Datei lesen
        let content = fs::read_to_string(config_path).map_err(|e| {
            Error::CustomError(format!("{ERR_FAILED_READ_CONFIG_PREFIX}{e}"))
        })?;

        // Parsen zu PluginsConfig
        let config: PluginsConfig = serde_yaml::from_str(&content).map_err(|e| {
            Error::CustomError(format!("{ERR_FAILED_PARSE_CONFIG_PREFIX}{e}"))
        })?;

        // suche entsprechende Plugins und setze enabled-Flag
        for plugin_cfg in config.plugins {
            if let Ok(plugin) = self
                .registered
                .iter_mut()
                .find(|p| p.name().as_str() == plugin_cfg.name)
                .ok_or_else(|| {
                    Error::CustomError(
                        format!("{ERR_PLUGIN_NOT_FOUND_PREFIX}{}' not found", plugin_cfg.name))
                })
            {
                plugin.set_enabled(plugin_cfg.enabled);
            }
        }

        Ok(())
    }

    // Start Python Runner
    async fn spawn_runner(
        &self,
        plugin_path: &PathBuf,
        instance_id: InstanceID,
    ) -> Result<(Child, ChildStdin, mpsc::Receiver<RunnerMsg>), Error> {
        let runner_path = PathBuf::from(RUNNER_PATH);

        // Geburt für Python-Prozess
        let mut child = Command::new(PYTHON_EXECUTABLE)
            .arg(PYTHON_UNBUFFERED_FLAG)
            .arg(runner_path)
            .arg(ARG_PLUGIN_PATH)
            .arg(plugin_path)
            .arg(ARG_INSTANCE_ID)
            .arg(instance_id.to_string())
            // notwendig für Rust:
            .stdin(Stdio::piped()) // an Rust
            .stdout(Stdio::piped()) // von Python
            .stderr(Stdio::piped()) // von Python Fehler
            .spawn()
            .map_err(|e| Error::CustomError(
                format!("{ERR_FAILED_SPAWN_PY_PREFIX}{e}")))?;

        // Um Commands an Runner schicken zu kömmem
        let child_stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::CustomError(ERR_FAILED_OPEN_STDIN.to_string()))?;

        // wie oben
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::CustomError(ERR_FAILED_OPEN_STDOUT.to_string()))?;

        // stderr drainen, damit der Prozess nie wegen voller Pipe blockiert (Deadlock)
        // -> alles als Error loggen
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    error!(LOG_PY_STDERR_PREFIX, line);
                }
            });
        }

        // Channel anlegen, um Runner-Events zu empfangen -> Hintergrundtask liest stdout/ parst
        let (tx, rx) = mpsc::channel::<RunnerMsg>(128);
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                match serde_json::from_str::<RunnerMsg>(&line) {
                    // Weiterleiten -> async integrierbar
                    Ok(msg) => {
                        if tx.send(msg).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        debug!(line = %line, error = %e, "python stdout (non-json)");
                    }
                }
            }
        });

        Ok((child, child_stdin, rx))
    }

    pub fn register_plugins(&mut self, directory: PathBuf) -> Result<(), Error> {
        // iterieren
        for entry in fs::read_dir(&directory)
            .map_err(|e| Error::CustomError(e.to_string()))? {
            let entry = entry.map_err(|e| Error::CustomError(e.to_string()))?;
            let path = entry.path();
            // jeweils registrieren
            self.register_plugin(path)?;
        }
        Ok(())
    }

    pub fn register_plugin(&mut self, path: PathBuf) -> Result<(), Error> {
        let warnings = python_bridge::validate_plugin_module(path.as_path())?;
        for w in &warnings {
            warn!("{w}");
        }

        // Dateiname ohne Endung
        let fallback_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(FALLBACK_PLUGIN_NAME)
            .to_string();

        let fallback_description = format!("Plugin loaded from {:?}", path);

        // Auslesen aus Python-Modul
        let (py_name, py_description, py_trigger) =
            python_bridge::read_module_constants(path.as_path()).unwrap_or((None, None, None));

        let name = py_name.unwrap_or(fallback_name);
        let description = py_description.unwrap_or(fallback_description);

        let trigger = parse_trigger(py_trigger.as_deref());

        let mut plugin = Plugin::new(name, description, trigger, path);
        plugin.set_valid(true);
        plugin.set_validation_warnings(warnings);

        self.registered.push(plugin);
        Ok(())
    }

    pub async fn start_plugin_instance_by_name(
        &mut self,
        plugin_name: &str,
        _parameters: Vec<(String, String)>,
        _temp_directory: PathBuf,
        instance_id: InstanceID,
    ) -> Result<(), Error> {
        // noch nicht am laufen
        if self.running.contains_key(&instance_id) {
            return Err(Error::CustomError(format!(
                "{ERR_INSTANCE_ALREADY_RUNNING_PREFIX}{} is already running",
                instance_id
            )));
        }

        // anhand des Namens suchen
        let plugin_index = self.registered.iter()
            .position(|p| p.name().as_str() == plugin_name).ok_or_else(|| {
            Error::CustomError(
                format!("{ERR_PLUGIN_NOT_REGISTERED_PREFIX}{}' is not registered", plugin_name))
        })?;

        let reg_plugin = &self.registered[plugin_index];

        // muss valid sein
        if !reg_plugin.valid() {
            return Err(Error::CustomError(format!(
                "Plugin '{}' is invalid and cannot be started",
                reg_plugin.name()
            )));
        }
        // muss aktiviert sein
        if !reg_plugin.enabled() {
            return Err(Error::CustomError(format!("Plugin '{}' is disabled", reg_plugin.name())));
        }

        // Python-Prozess starten
        let (child, child_stdin, stdout_rx) 
            = self.spawn_runner(reg_plugin.path(), instance_id).await?;

        // Instanz erstellen
        let mut inst = RunningInstance {
            plugin_index,
            state: Running,
            child,
            child_stdin,
            stdout_rx,
            next_request_seq: 1,
        };

        // Sendet start an Python-Runner
        let _ = Self::send_cmd_ack(&mut inst, instance_id, CMD_START, TIMEOUT_START_ACK).await?;
        // Eintrag in running speichern
        self.running.insert(instance_id, inst);
        Ok(())
    }

    pub async fn stop_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        // Entfernt Instanz aus running
        let mut entry = self.running.remove(&instance_id).ok_or_else(|| {
            Error::CustomError(
                format!("{ERR_INSTANCE_NOT_RUNNING_PREFIX}{} is not running", instance_id))
        })?;

        // 1) Soft stop mit Bestätigung (damit es nicht "ignoriert" wird)
        let soft =
            Self::send_cmd_ack(&mut entry, instance_id, CMD_STOP, TIMEOUT_SOFT_STOP_ACK).await;

        if soft.is_ok() {
            // wirklich okay
            if timeout(TIMEOUT_WAIT_EXIT_AFTER_SOFT_STOP,
                       entry.child.wait()).await.is_ok() {
                return Ok(());
            }
            // sonst Warnung
            warn!(LOG_SOFT_STOP_FORCE_KILL);
        } else {
            warn!(error = ?soft.err(), "{LOG_SOFT_STOP_FAILED_FORCE_KILL}");
        }

        // kill
        entry
            .child
            .kill()
            .await
            .map_err(|e| Error::CustomError(format!("{ERR_FAILED_KILL_PY_PREFIX}{e}")))?;

        // kurz warten
        let _ = timeout(TIMEOUT_WAIT_EXIT_AFTER_KILL, entry.child.wait()).await;
        Ok(())
    }

    pub async fn pause_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        let entry = self.get_running_instance_mut(instance_id)?;

        // schon pausiert
        if entry.state == InstanceState::Paused {
            return Ok(());
        }

        // sendet pause, wartet ack
        let _ = Self::send_cmd_ack(entry, instance_id, CMD_PAUSE, TIMEOUT_PAUSE_ACK).await?;
        entry.state = InstanceState::Paused;
        Ok(())
    }

    pub async fn resume_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        let entry = self.get_running_instance_mut(instance_id)?;
        // schon running?
        if entry.state == Running {
            return Ok(());
        }

        let _ = Self::send_cmd_ack(entry, instance_id, CMD_RESUME, TIMEOUT_RESUME_ACK).await?;
        entry.state = Running;
        Ok(())
    }

    // Herz der Kommunikation
    async fn send_cmd_ack(
        inst: &mut RunningInstance,
        instance_id: InstanceID,
        cmd: &str,
        wait: Duration,
    ) -> Result<RunnerMsg, Error> {
        // RequestID bauen
        let request_id = format!("{}-{}", instance_id, inst.next_request_seq);
        inst.next_request_seq += 1;

        // json request bauen
        let mut req = serde_json::Map::new();
        req.insert(
            JSON_KEY_INSTANCE_ID.to_string(),
            serde_json::Value::from(instance_id),
        );
        req.insert(
            JSON_KEY_REQUEST_ID.to_string(),
            serde_json::Value::from(request_id.clone()), // clone weil String
        );
        req.insert(JSON_KEY_CMD.to_string(), serde_json::Value::from(cmd));

        let req = serde_json::Value::Object(req);

        // Weiterleiten an Python-Prozess
        let line = req.to_string() + "\n";
        inst.child_stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| Error::CustomError(format!("{ERR_FAILED_SEND_CMD_PREFIX}{e}")))?;
        inst.child_stdin
            .flush() // sonst bleibt es in Buffer
            .await
            .map_err(|e| Error::CustomError(format!("{ERR_FAILED_FLUSH_CMD_PREFIX}{e}")))?;

        let fut = async {
            loop {
                // asynchrones Warten auf Nachricht von Python
                let msg = inst
                    .stdout_rx
                    .recv()
                    .await
                    .ok_or_else(|| Error::CustomError(ERR_PY_STDOUT_CLOSED.to_string()))?;

                // Instanz nicht betroffen
                if msg.instance_id != instance_id {
                    continue;
                }

                // Request muss übereinstimmen, sonst nicht gesuchte Antwort
                if msg.request_id.as_deref()
                    == Some(req[JSON_KEY_REQUEST_ID].as_str().unwrap())
                {
                    // Erfolg -> Rückgabe
                    if msg.ok.unwrap_or(false) {
                        return Ok(msg);
                    }

                    // Baue Fehlermeldung
                    let err = msg.error.unwrap_or_else(|| ERR_UNKNOWN_ERROR.to_string());
                    let trace = msg.trace.unwrap_or_default();

                    if trace.is_empty() {
                        return Err(Error::CustomError(format!("Runner cmd '{cmd}' failed: {err}")));
                    }

                    return Err(Error::CustomError(format!(
                        "Runner cmd '{cmd}' failed: {err}\nPython traceback:\n{trace}"
                    )));
                }

                // falls Events -> loggen und weiter warten
                if let Some(ev) = &msg.event {
                    debug!(LOG_RUNNER_EVENT, instance_id, ev);
                }
            }
        };

        // hängt nicht ewig -> timeout
        timeout(wait, fut)
            .await
            .map_err(|_| Error::CustomError(
                format!("Runner cmd '{cmd}' timed out after {:?}", wait)))?
    }

    // Ausgabe running Instanzen als Liste von (&Plugin, InstanceID)
    pub fn get_running_instances(&self) -> Vec<(&Plugin, InstanceID)> {
        self.running
            .iter()
            .filter_map(|(instance_id, entry)| {
                if entry.state == Running {
                    let plugin = &self.registered[entry.plugin_index];
                    Some((plugin, *instance_id))
                } else {
                    None
                }
            })
            .collect()
    }

    // Ausgabe aller registrierten Plugins als Liste von &Plugin
    pub fn get_registered_plugins(&self) -> Vec<&Plugin> {
        self.registered.iter().collect()
    }

    // finden und enable
    pub fn enable_plugin(&mut self, name: &str) -> Result<(), Error> {
        let plugin = self
            .registered
            .iter_mut()
            .find(|p| p.name().as_str() == name)
            .ok_or_else(|| Error::CustomError(format!("Plugin '{name}' not found")))?;

        plugin.set_enabled(true);
        Ok(())
    }

    // finden und disable
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