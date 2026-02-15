#![allow(unused)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{mpsc, Mutex};
use tokio::time::{Duration, timeout};
use tracing::{debug, error, warn};

use crate::plugin_manager::manager::InstanceState::Running;
use crate::plugin_manager::plugin::Trigger;
use crate::plugin_manager::python_bridge;
use crate::{
    error::Error, plugin_manager::plugin::Plugin,
};
use std::sync::Arc;

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

const PYTHON_UNBUFFERED_FLAG: &str = "-u";

#[cfg(windows)]
const PYTHON_EXECUTABLE: &str = "python";

#[cfg(not(windows))]
const PYTHON_EXECUTABLE: &str = "python3";

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

const LOG_SOFT_STOP_FORCE_KILL: &str =
    "Soft stop ACK ok, but process did not exit quickly; forcing kill.";
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
fn is_paused(inst: &RunningInstance) -> bool {
    inst.state == InstanceState::Paused
}

// Rückgabe ob laufend
fn is_running(inst: &RunningInstance) -> bool {
    inst.state == Running
}

// baut json aus instanz/request_id und cmd
fn build_cmd_request(instance_id: InstanceID, request_id: &str, cmd: &str) -> serde_json::Value {
    let mut req = serde_json::Map::new();
    req.insert(
        JSON_KEY_INSTANCE_ID.to_string(),
        serde_json::Value::from(instance_id),
    );
    req.insert(
        JSON_KEY_REQUEST_ID.to_string(),
        serde_json::Value::from(request_id),
    );
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
    plugin_index: usize,                  // welches Plugin auf Liste
    state: InstanceState,                 // Running/Paused
    child: Child,                         // Handle auf gestarteten Python Prozess
    child_stdin: ChildStdin,              // Schreibkanal zum Python Prozess
    stdout_rx: mpsc::Receiver<RunnerMsg>, // Empfangschannel vom Python Prozess
    next_request_seq: u64,                // Zähler für eindeutige Request-IDs
}

impl RunningInstance {
    async fn cmd_start(&mut self, id: InstanceID) -> Result<RunnerMsg, Error> {
        PluginManager::send_cmd_ack(self, id, CMD_START, TIMEOUT_START_ACK).await
    }

    async fn cmd_stop(&mut self, id: InstanceID) -> Result<RunnerMsg, Error> {
        PluginManager::send_cmd_ack(self, id, CMD_STOP, TIMEOUT_SOFT_STOP_ACK).await
    }

    async fn cmd_pause(&mut self, id: InstanceID) -> Result<RunnerMsg, Error> {
        PluginManager::send_cmd_ack(self, id, CMD_PAUSE, TIMEOUT_PAUSE_ACK).await
    }

    async fn cmd_resume(&mut self, id: InstanceID) -> Result<RunnerMsg, Error> {
        PluginManager::send_cmd_ack(self, id, CMD_RESUME, TIMEOUT_RESUME_ACK).await
    }
}

#[derive(Debug)]
pub struct PluginManager {
    registered: Vec<Plugin>,
    running: HashMap<InstanceID, Arc<Mutex<RunningInstance>>>,
}

impl PluginManager {
    // alles notwendige neu erzeugt
    pub fn new() -> Self {
        Self {
            registered: Vec::new(),
            running: HashMap::new(),
        }
    }

    pub fn load_config_and_apply(&mut self, config_path: &str) -> Result<(), Error> {
        // YAML-Datei lesen

        let content = fs::read_to_string(config_path).map_err(|e| {
            error!("config file not set/cannot be read: {}", e);
            Error::CustomError(format!("{ERR_FAILED_READ_CONFIG_PREFIX}{e}"))
        })?;

        // Parsen zu PluginsConfig
        let config: PluginsConfig = serde_yaml::from_str(&content)
            .map_err(|e| Error::CustomError(format!("{ERR_FAILED_PARSE_CONFIG_PREFIX}{e}")))?;

        // suche entsprechende Plugins und setze enabled-Flag
        for plugin_cfg in config.plugins {
            let plugin = self
                .registered
                .iter_mut()
                .find(|p| p.name().as_str() == plugin_cfg.name)
                .ok_or_else(|| {
                    Error::CustomError(format!(
                        "{ERR_PLUGIN_NOT_FOUND_PREFIX}{}' not found",
                        plugin_cfg.name
                    ))
                })?;

            plugin.set_enabled(plugin_cfg.enabled);
        }

        Ok(())
    }

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

    /// Liefert einen Handle (Arc) auf eine laufende Instanz.
    /// Wichtig: Der globale PluginManager-Lock kann danach gedroppt werden,
    /// und die eigentliche Arbeit läuft über den Instance-Mutex.
    pub fn get_instance_handle(
        &self,
        instance_id: InstanceID,
    ) -> Result<Arc<Mutex<RunningInstance>>, Error> {
        self.running.get(&instance_id).cloned().ok_or_else(|| {
            Error::CustomError(format!(
                "{ERR_INSTANCE_NOT_RUNNING_PREFIX}{} is not running",
                instance_id
            ))
        })
    }

    /// Entfernt die Instanz aus der Map und gibt den Handle zurück.
    /// Für `stop`: wir wollen nach außen keine "running" Instanz mehr reporten,
    /// während wir ggf. noch soft-stop/kill abarbeiten.
    pub fn take_instance_handle(
        &mut self,
        instance_id: InstanceID,
    ) -> Result<Arc<Mutex<RunningInstance>>, Error> {
        self.running.remove(&instance_id).ok_or_else(|| {
            Error::CustomError(format!(
                "{ERR_INSTANCE_NOT_RUNNING_PREFIX}{} is not running",
                instance_id
            ))
        })
    }

    /// Validiert Plugin-Startbedingungen und liefert die Daten, die man zum Start braucht,
    /// ohne async/await zu machen (damit man den globalen Lock droppen kann).
    pub fn prepare_start(&self, plugin_name: &str) -> Result<(usize, PathBuf), Error> {
        let plugin_index = self
            .registered
            .iter()
            .position(|p| p.name().as_str() == plugin_name)
            .ok_or_else(|| {
                Error::CustomError(format!(
                    "{ERR_PLUGIN_NOT_REGISTERED_PREFIX}{}' is not registered",
                    plugin_name
                ))
            })?;

        let reg_plugin = &self.registered[plugin_index];

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

        Ok((plugin_index, reg_plugin.path().clone()))
    }

    /// Finalisiert den Start: trägt die Instanz in `running` ein.
    /// Sollte erst aufgerufen werden, nachdem der Runner erfolgreich gestartet wurde.
    pub fn commit_started_instance(
        &mut self,
        instance_id: InstanceID,
        inst: RunningInstance,
    ) -> Result<(), Error> {
        if self.running.contains_key(&instance_id) {
            return Err(Error::CustomError(format!(
                "{ERR_INSTANCE_ALREADY_RUNNING_PREFIX}{} is already running",
                instance_id
            )));
        }

        self.running.insert(instance_id, Arc::new(Mutex::new(inst)));
        Ok(())
    }

    // Start Python Runner
    async fn spawn_runner(
        &self,
        plugin_path: &PathBuf,
        instance_id: InstanceID,
    ) -> Result<(Child, ChildStdin, mpsc::Receiver<RunnerMsg>), Error> {
        let runner_path = PathBuf::from(RUNNER_PATH);

        let mut child = Command::new(PYTHON_EXECUTABLE)
            .arg(PYTHON_UNBUFFERED_FLAG)
            .arg(runner_path)
            .arg(ARG_PLUGIN_PATH)
            .arg(plugin_path)
            .arg(ARG_INSTANCE_ID)
            .arg(instance_id.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::CustomError(format!("{ERR_FAILED_SPAWN_PY_PREFIX}{e}")))?;

        let child_stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::CustomError(ERR_FAILED_OPEN_STDIN.to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::CustomError(ERR_FAILED_OPEN_STDOUT.to_string()))?;

        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    error!(LOG_PY_STDERR_PREFIX, line);
                }
            });
        }

        let (tx, rx) = mpsc::channel::<RunnerMsg>(128);
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                match serde_json::from_str::<RunnerMsg>(&line) {
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

    // Herz der Kommunikation
    async fn send_cmd_ack(
        inst: &mut RunningInstance,
        instance_id: InstanceID,
        cmd: &str,
        wait: Duration,
    ) -> Result<RunnerMsg, Error> {
        let request_id = format!("{}-{}", instance_id, inst.next_request_seq);
        inst.next_request_seq += 1;

        let mut req = serde_json::Map::new();
        req.insert(
            JSON_KEY_INSTANCE_ID.to_string(),
            serde_json::Value::from(instance_id),
        );
        req.insert(
            JSON_KEY_REQUEST_ID.to_string(),
            serde_json::Value::from(request_id.clone()),
        );
        req.insert(JSON_KEY_CMD.to_string(), serde_json::Value::from(cmd));

        let req = serde_json::Value::Object(req);

        let line = req.to_string() + "\n";
        inst.child_stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| Error::CustomError(format!("{ERR_FAILED_SEND_CMD_PREFIX}{e}")))?;
        inst.child_stdin
            .flush()
            .await
            .map_err(|e| Error::CustomError(format!("{ERR_FAILED_FLUSH_CMD_PREFIX}{e}")))?;

        let fut = async {
            loop {
                let msg = inst
                    .stdout_rx
                    .recv()
                    .await
                    .ok_or_else(|| Error::CustomError(ERR_PY_STDOUT_CLOSED.to_string()))?;

                if msg.instance_id != instance_id {
                    continue;
                }

                if msg.request_id.as_deref() == Some(req[JSON_KEY_REQUEST_ID].as_str().unwrap()) {
                    if msg.ok.unwrap_or(false) {
                        return Ok(msg);
                    }

                    let err = msg.error.unwrap_or_else(|| ERR_UNKNOWN_ERROR.to_string());
                    let trace = msg.trace.unwrap_or_default();

                    if trace.is_empty() {
                        return Err(Error::CustomError(format!(
                            "Runner cmd '{cmd}' failed: {err}"
                        )));
                    }

                    return Err(Error::CustomError(format!(
                        "Runner cmd '{cmd}' failed: {err}\nPython traceback:\n{trace}"
                    )));
                }

                if let Some(ev) = &msg.event {
                    debug!(LOG_RUNNER_EVENT, instance_id, ev);
                }
            }
        };

        timeout(wait, fut).await.map_err(|_| {
            Error::CustomError(format!("Runner cmd '{cmd}' timed out after {:?}", wait))
        })?
    }

    /// Ausführung: Runner starten + start-ACK abwarten.
    /// Achtung: Diese Methode hält KEINEN globalen Lock – sie baut nur die Instanz.
    pub async fn build_started_instance(
        &self,
        plugin_index: usize,
        plugin_path: &PathBuf,
        instance_id: InstanceID,
    ) -> Result<RunningInstance, Error> {
        let (child, child_stdin, stdout_rx) = self.spawn_runner(plugin_path, instance_id).await?;

        let mut inst = RunningInstance {
            plugin_index,
            state: Running,
            child,
            child_stdin,
            stdout_rx,
            next_request_seq: 1,
        };

        inst.cmd_start(instance_id).await?;
        Ok(inst)
    }

    /// Stop-Logik auf Instance-Ebene (wird typischerweise über einen Instance-Mutex ausgeführt).
    pub async fn stop_instance_handle(
        instance: Arc<Mutex<RunningInstance>>,
        instance_id: InstanceID,
    ) -> Result<(), Error> {
        let mut entry = instance.lock().await;

        let soft = entry.cmd_stop(instance_id).await;

        if soft.is_ok() {
            if timeout(TIMEOUT_WAIT_EXIT_AFTER_SOFT_STOP, entry.child.wait())
                .await
                .is_ok()
            {
                return Ok(());
            }
            warn!(LOG_SOFT_STOP_FORCE_KILL);
        } else {
            warn!(error = ?soft.err(), "{LOG_SOFT_STOP_FAILED_FORCE_KILL}");
        }

        entry
            .child
            .kill()
            .await
            .map_err(|e| Error::CustomError(format!("{ERR_FAILED_KILL_PY_PREFIX}{e}")))?;

        let _ = timeout(TIMEOUT_WAIT_EXIT_AFTER_KILL, entry.child.wait()).await;
        Ok(())
    }

    pub async fn pause_instance_handle(
        instance: Arc<Mutex<RunningInstance>>,
        instance_id: InstanceID,
    ) -> Result<(), Error> {
        let mut entry = instance.lock().await;

        if entry.state == InstanceState::Paused {
            return Ok(());
        }

        entry.cmd_pause(instance_id).await?;
        entry.state = InstanceState::Paused;
        Ok(())
    }

    pub async fn resume_instance_handle(
        instance: Arc<Mutex<RunningInstance>>,
        instance_id: InstanceID,
    ) -> Result<(), Error> {
        let mut entry = instance.lock().await;

        if entry.state == Running {
            return Ok(());
        }

        entry.cmd_resume(instance_id).await?;
        entry.state = Running;
        Ok(())
    }

    fn canonical_or_original(path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }

    fn is_path_already_registered(&self, path: &Path) -> bool {
        let cand = Self::canonical_or_original(path);

        self.registered.iter().any(|p| {
            let existing = Self::canonical_or_original(p.path());
            existing == cand
        })
    }

    pub fn register_plugin(&mut self, plugin_path: PathBuf) -> Result<(), Error> {
        let plugin_path = Self::canonical_or_original(&plugin_path);

        if self.is_path_already_registered(&plugin_path) {
            return Err(Error::CustomError(format!(
                "Plugin at path '{}' is already registered",
                plugin_path.to_string_lossy()
            )));
        }

        // Python-Seite: validieren + Konstanten lesen
        let warnings = python_bridge::validate_plugin_module(&plugin_path)?;
        let (name_opt, desc_opt, trigger_opt) = python_bridge::read_module_constants(&plugin_path)?;

        let file_stem_fallback = plugin_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(FALLBACK_PLUGIN_NAME);

        let name = name_opt.unwrap_or_else(|| file_stem_fallback.to_string());
        let description = desc_opt.unwrap_or_else(|| {
            format!("Plugin loaded from {}", plugin_path.to_string_lossy())
        });
        let trigger = parse_trigger(trigger_opt.as_deref());

        let mut plugin = Plugin::new(name, description, trigger, plugin_path);
        plugin.set_validation_warnings(warnings);
        plugin.set_valid(true);
        plugin.set_enabled(true);

        self.registered.push(plugin);
        Ok(())
    }

    pub fn register_plugins(&mut self, directory: PathBuf) -> Result<(), Error> {
        let dir = directory;

        let entries = fs::read_dir(&dir).map_err(|e| {
            Error::CustomError(format!(
                "Failed to read plugin directory '{}': {e}",
                dir.to_string_lossy()
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                Error::CustomError(format!(
                    "Failed to read plugin directory entry in '{}': {e}",
                    dir.to_string_lossy()
                ))
            })?;

            let path = entry.path();

            // Skip: Unterordner / Nicht-.py
            if path.is_dir() {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("py") {
                continue;
            }

            // Fehler bei einem Plugin => brich ab (passt zu euren Tests: duplicates sollen Err geben).
            self.register_plugin(path)?;
        }

        Ok(())
    }


    // Ausgabe running Instanzen als Liste von (&Plugin, InstanceID)
    pub fn get_running_instances(&self) -> Vec<(&Plugin, InstanceID)> {
        // Best-effort ohne awaits: wenn Instance gerade gelockt ist (stop/pause), skippen wir sie.
        // Das verhindert, dass "Übersicht" wegen einer einzelnen langsamen Instanz hängen bleibt.
        self.running
            .iter()
            .filter_map(|(instance_id, entry)| {
                let Ok(guard) = entry.try_lock() else {
                    return None;
                };

                if guard.state == Running {
                    let plugin = &self.registered[guard.plugin_index];
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
