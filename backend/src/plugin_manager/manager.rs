#![allow(unused)]

use cron::Schedule;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::str::FromStr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, mpsc};
use tokio::time::{Duration, timeout};
use tracing::{debug, error, warn};

use crate::plugin_manager::manager::InstanceState::Running;
use crate::plugin_manager::plugin::Trigger;
use crate::plugin_manager::python_bridge;
use crate::{error::Error, plugin_manager::plugin::Plugin};
use rocket::futures::TryFutureExt;
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
fn parse_trigger(py_trigger: Option<&str>) -> Result<Trigger, Error> {
    match py_trigger {
        // Trigger extrahieren
        Some(TRIGGER_MANUAL) | None => Ok(Trigger::Manual),
        Some(TRIGGER_ON_ENTRY_CREATE) => Ok(Trigger::OnEntryCreate),
        Some(TRIGGER_ON_ENTRY_UPDATE) => Ok(Trigger::OnEntryUpdate),
        Some(TRIGGER_ON_ENTRY_DELETE) => Ok(Trigger::OnEntryDelete),
        Some(other) if other.starts_with(TRIGGER_ON_SCHEDULE_PREFIX) => {
            let raw = other.trim_start_matches(TRIGGER_ON_SCHEDULE_PREFIX).trim();

            // Unterstütze sowohl 5-Feld (min hour day mon dow) als auch 6-Feld
            // (sec min hour day mon dow).
            // Wenn 5 Felder angegeben sind, interpretieren wir das als "sekunden=0".
            let field_count = raw.split_whitespace().count();
            let cron_expr = match field_count {
                5 => format!("0 {raw}"),
                _ => raw.to_string(),
            };

            let schedule = Schedule::from_str(&cron_expr).map_err(|e| {
                Error::CustomError(format!(
                    "Invalid cron expression '{raw}' (parsed as '{cron_expr}'): {e}"
                ))
            })?;

            Ok(Trigger::OnSchedule(schedule))
        }
        _ => Ok(Trigger::Manual),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum InstanceState {
    Running,
    Paused,
    Completed,
    Failed,
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
pub struct RunningInstance {
    pub plugin_index: usize,  // welches Plugin auf Liste
    pub state: InstanceState, // Running/Paused
    child: Child,             // Handle auf gestarteten Python Prozess
    child_stdin: ChildStdin,  // Schreibkanal zum Python Prozess
    pending_acks: HashMap<String, tokio::sync::oneshot::Sender<RunnerMsg>>,
    next_request_seq: u64, // Zähler für eindeutige Request-IDs
}

impl RunningInstance {
    async fn cmd_start(this: Arc<Mutex<Self>>, id: InstanceID) -> Result<RunnerMsg, Error> {
        PluginManager::send_cmd_ack(this, id, CMD_START, TIMEOUT_START_ACK).await
    }

    async fn cmd_stop(this: Arc<Mutex<Self>>, id: InstanceID) -> Result<RunnerMsg, Error> {
        PluginManager::send_cmd_ack(this, id, CMD_STOP, TIMEOUT_SOFT_STOP_ACK).await
    }

    async fn cmd_pause(this: Arc<Mutex<Self>>, id: InstanceID) -> Result<RunnerMsg, Error> {
        PluginManager::send_cmd_ack(this, id, CMD_PAUSE, TIMEOUT_PAUSE_ACK).await
    }

    async fn cmd_resume(this: Arc<Mutex<Self>>, id: InstanceID) -> Result<RunnerMsg, Error> {
        PluginManager::send_cmd_ack(this, id, CMD_RESUME, TIMEOUT_RESUME_ACK).await
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
        // TODO what about disabled plugins?
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
    async fn cmd_start(
        inst: Arc<Mutex<RunningInstance>>,
        id: InstanceID,
    ) -> Result<RunnerMsg, Error> {
        Self::send_cmd_ack(inst, id, CMD_START, TIMEOUT_START_ACK).await
    }
    async fn cmd_stop(
        inst: Arc<Mutex<RunningInstance>>,
        id: InstanceID,
    ) -> Result<RunnerMsg, Error> {
        Self::send_cmd_ack(inst, id, CMD_STOP, TIMEOUT_SOFT_STOP_ACK).await
    }
    async fn cmd_pause(
        inst: Arc<Mutex<RunningInstance>>,
        id: InstanceID,
    ) -> Result<RunnerMsg, Error> {
        Self::send_cmd_ack(inst, id, CMD_PAUSE, TIMEOUT_PAUSE_ACK).await
    }
    async fn cmd_resume(
        inst: Arc<Mutex<RunningInstance>>,
        id: InstanceID,
    ) -> Result<RunnerMsg, Error> {
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

    // finalisiert den Start: trägt die Instanz in `running` ein.
    // Sollte erst aufgerufen werden, nachdem der Runner erfolgreich gestartet wurde.
    pub fn commit_started_instance(
        &mut self,
        instance_id: InstanceID,
        handle: Arc<Mutex<RunningInstance>>,
    ) -> Result<(), Error> {
        if self.running.contains_key(&instance_id) {
            return Err(Error::CustomError(format!(
                "{ERR_INSTANCE_ALREADY_RUNNING_PREFIX}{} is already running",
                instance_id
            )));
        }

        self.running.insert(instance_id, handle);
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
        instance_handle: Arc<Mutex<RunningInstance>>,
        instance_id: InstanceID,
        cmd: &str,
        wait: Duration,
    ) -> Result<RunnerMsg, Error> {
        let (request_id, next_seq) = {
            let mut inst = instance_handle.lock().await;
            let rid = format!("{}-{}", instance_id, inst.next_request_seq);
            inst.next_request_seq += 1;
            (rid, inst.next_request_seq)
        };

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

        let (tx, rx) = tokio::sync::oneshot::channel();

        {
            let mut inst = instance_handle.lock().await;
            inst.pending_acks.insert(request_id.clone(), tx);

            inst.child_stdin
                .write_all(line.as_bytes())
                .await
                .map_err(|e| Error::CustomError(format!("{ERR_FAILED_SEND_CMD_PREFIX}{e}")))?;
            inst.child_stdin
                .flush()
                .await
                .map_err(|e| Error::CustomError(format!("{ERR_FAILED_FLUSH_CMD_PREFIX}{e}")))?;
        }

        let fut = async {
            let msg = rx
                .await
                .map_err(|_| Error::CustomError(ERR_PY_STDOUT_CLOSED.to_string()))?;

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

            Err(Error::CustomError(format!(
                "Runner cmd '{cmd}' failed: {err}\nPython traceback:\n{trace}"
            )))
        };

        timeout(wait, fut).await.map_err(|_| {
            // Cleanup pending ACK on timeout
            tokio::spawn(async move {
                let mut inst = instance_handle.lock().await;
                inst.pending_acks.remove(&request_id);
            });
            Error::CustomError(format!("Runner cmd '{cmd}' timed out after {:?}", wait))
        })?
    }

    fn start_background_listener(
        instance_handle: Arc<Mutex<RunningInstance>>,
        mut rx: mpsc::Receiver<RunnerMsg>,
        instance_id: InstanceID,
    ) {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if msg.instance_id != instance_id {
                    continue;
                }

                if let Some(request_id) = &msg.request_id {
                    let mut guard = instance_handle.lock().await;
                    if let Some(tx) = guard.pending_acks.remove(request_id) {
                        let _ = tx.send(msg);
                        continue;
                    }
                }

                if let Some(ev) = &msg.event {
                    debug!(LOG_RUNNER_EVENT, instance_id, ev);
                    if ev == "exited" {
                        let mut guard = instance_handle.lock().await;
                        guard.state = if msg.ok.unwrap_or(false) {
                            InstanceState::Completed
                        } else {
                            InstanceState::Failed
                        };
                    }
                }
            }
        });
    }

    /// Ausführung: Runner starten + start-ACK abwarten.
    /// Achtung: Diese Methode hält KEINEN globalen Lock – sie baut nur die Instanz.
    pub async fn build_started_instance(
        &self,
        plugin_index: usize,
        plugin_path: &PathBuf,
        instance_id: InstanceID,
    ) -> Result<Arc<Mutex<RunningInstance>>, Error> {
        let (child, child_stdin, stdout_rx) = self.spawn_runner(plugin_path, instance_id).await?;

        let inst = RunningInstance {
            plugin_index,
            state: InstanceState::Running,
            child,
            child_stdin,
            pending_acks: HashMap::new(),
            next_request_seq: 1,
        };

        let handle = Arc::new(Mutex::new(inst));
        Self::start_background_listener(handle.clone(), stdout_rx, instance_id);

        RunningInstance::cmd_start(handle.clone(), instance_id).await?;
        Ok(handle)
    }

    pub fn register_plugins(&mut self, directory: PathBuf) -> Result<(), Error> {
        // iterieren
        for entry in fs::read_dir(&directory).map_err(|e| Error::CustomError(e.to_string()))? {
            let entry = entry.map_err(|e| Error::CustomError(e.to_string()))?;
            let path = entry.path();

            // Nur Dateien registrieren (keine Ordner)
            if !path.is_file() {
                continue;
            }

            // Nur *.py registrieren (sonst ist ein Plugin-Ordner extrem fragil)
            let is_py = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("py"));

            if !is_py {
                continue;
            }

            // jeweils registrieren
            self.register_plugin(path)?;
        }
        Ok(())
    }

    pub fn register_plugin(&mut self, path: PathBuf) -> Result<(), Error> {
        // Duplikate verhindern: gleicher Plugin-Pfad darf nicht zweimal registriert werden.
        // Canonicalize macht es robuster gegen ./foo.py vs foo.py vs absolute Pfade.
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.clone());
        if canonical_path.ends_with("plugin_base.py") {
            debug!("Skipping registration of 'plugin_base.py'");
            return Ok(());
        }
        if self.registered.iter().any(|p| p.path() == &canonical_path) {
            return Err(Error::CustomError(format!(
                "Plugin at path '{:?}' is already registered",
                canonical_path
            )));
        }

        let warnings = python_bridge::validate_plugin_module(canonical_path.as_path())?;
        for w in &warnings {
            warn!("{w}");
        }

        // Dateiname ohne Endung
        let fallback_name = canonical_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(FALLBACK_PLUGIN_NAME)
            .to_string();

        let fallback_description = format!("Plugin loaded from {:?}", canonical_path);

        // Auslesen aus Python-Modul
        let (py_name, py_description, py_trigger) =
            python_bridge::read_module_constants(canonical_path.as_path())
                .unwrap_or((None, None, None));

        let name = py_name.unwrap_or(fallback_name);
        let description = py_description.unwrap_or(fallback_description);

        let trigger = parse_trigger(py_trigger.as_deref())?;

        let mut plugin = Plugin::new(name, description, trigger, canonical_path);
        plugin.set_valid(true);
        plugin.set_validation_warnings(warnings);

        self.registered.push(plugin);
        Ok(())
    }

    pub async fn start_plugin_instance(
        &mut self,
        plugin_name: &str,
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

        // muss valid sein
        if !reg_plugin.valid() {
            return Err(Error::CustomError(format!(
                "Plugin '{}' is invalid and cannot be started",
                reg_plugin.name()
            )));
        }
        // muss aktiviert sein
        if !reg_plugin.enabled() {
            return Err(Error::CustomError(format!(
                "Plugin '{}' is disabled",
                reg_plugin.name()
            )));
        }

        let handle = self
            .build_started_instance(plugin_index, reg_plugin.path(), instance_id)
            .await?;
        self.running.insert(instance_id, handle);
        Ok(())
    }

    pub async fn stop_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        // Entfernt Instanz aus running
        let handle = self.running.remove(&instance_id).ok_or_else(|| {
            Error::CustomError(format!(
                "{ERR_INSTANCE_NOT_RUNNING_PREFIX}{} is not running",
                instance_id
            ))
        })?;

        Self::stop_instance_handle(handle, instance_id).await
    }

    pub async fn pause_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        let handle = self.get_instance_handle(instance_id)?;
        Self::pause_instance_handle(handle, instance_id).await
    }

    pub async fn resume_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        let handle = self.get_instance_handle(instance_id)?;
        Self::resume_instance_handle(handle, instance_id).await
    }

    /// Stop-Logik auf Instance-Ebene (wird typischerweise über einen Instance-Mutex ausgeführt).
    pub async fn stop_instance_handle(
        instance: Arc<Mutex<RunningInstance>>,
        instance_id: InstanceID,
    ) -> Result<(), Error> {
        {
            let entry = instance.lock().await;
            if entry.state == InstanceState::Completed || entry.state == InstanceState::Failed {
                return Ok(());
            }
        }

        let soft = RunningInstance::cmd_stop(instance.clone(), instance_id).await;

        let mut entry = instance.lock().await;
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
        {
            let entry = instance.lock().await;
            match entry.state {
                InstanceState::Paused => return Ok(()),
                InstanceState::Completed | InstanceState::Failed => {
                    return Err(Error::CustomError(format!(
                        "Cannot pause instance {} as it has already finished ({:?})",
                        instance_id, entry.state
                    )));
                }
                InstanceState::Running => {}
            }
        }

        RunningInstance::cmd_pause(instance.clone(), instance_id).await?;

        let mut entry = instance.lock().await;
        entry.state = InstanceState::Paused;
        Ok(())
    }

    pub async fn resume_instance_handle(
        instance: Arc<Mutex<RunningInstance>>,
        instance_id: InstanceID,
    ) -> Result<(), Error> {
        {
            let entry = instance.lock().await;
            match entry.state {
                InstanceState::Running => return Ok(()),
                InstanceState::Completed | InstanceState::Failed => {
                    return Err(Error::CustomError(format!(
                        "Cannot resume instance {} as it has already finished ({:?})",
                        instance_id, entry.state
                    )));
                }
                InstanceState::Paused => {}
            }
        }

        RunningInstance::cmd_resume(instance.clone(), instance_id).await?;

        let mut entry = instance.lock().await;
        entry.state = InstanceState::Running;
        Ok(())
    }

    // Ausgabe running Instanzen als Liste von (&Plugin, InstanceID, InstanceState)
    pub fn get_running_instances(&self) -> Vec<(&Plugin, InstanceID, InstanceState)> {
        // Best-effort ohne awaits: wenn Instance gerade gelockt ist (stop/pause), skippen wir sie.
        // Das verhindert, dass "Übersicht" wegen einer einzelnen langsamen Instanz hängen bleibt.
        self.running
            .iter()
            .filter_map(|(instance_id, entry)| {
                let Ok(guard) = entry.try_lock() else {
                    return None;
                };

                let plugin = &self.registered[guard.plugin_index];
                Some((plugin, *instance_id, guard.state))
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
