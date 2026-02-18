use cron::Schedule;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::str::FromStr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::{Duration, timeout};
use tracing::{debug, error, info, instrument, warn};
use tracing_subscriber::field::debug;

use crate::plugin_manager::plugin::Trigger;
use crate::plugin_manager::python_bridge;
use crate::{error::Error, plugin_manager::plugin::Plugin};
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
#[instrument]
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
// baut json aus instanz/request_id und cmd
#[instrument]
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
pub enum PluginCommand {
    Stop(oneshot::Sender<Result<(), Error>>),
    Pause(oneshot::Sender<Result<(), Error>>),
    Resume(oneshot::Sender<Result<(), Error>>),
}

#[derive(Debug, Clone)]
pub struct PluginHandle {
    pub plugin_index: usize,
    pub command_tx: mpsc::Sender<PluginCommand>,
    pub status_rx: watch::Receiver<InstanceState>,
}

#[derive(Debug)]
pub struct PluginManager {
    registered: Vec<Plugin>,
    running: HashMap<InstanceID, PluginHandle>,
}

impl PluginManager {
    // alles notwendige neu erzeugt
    #[instrument]
    pub fn new() -> Self {
        Self {
            registered: Vec::new(),
            running: HashMap::new(),
        }
    }

    #[instrument(skip(self))]
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

    /// Liefert einen Handle auf eine laufende Instanz.

    #[instrument]
    pub fn get_instance_handle(&self, instance_id: InstanceID) -> Result<PluginHandle, Error> {
        self.running.get(&instance_id).cloned().ok_or_else(|| {
            Error::CustomError(format!(
                "{ERR_INSTANCE_NOT_RUNNING_PREFIX}{} is not running",
                instance_id
            ))
        })
    }

    /// Entfernt die Instanz aus der Map und gibt den Handle zurück.
    #[instrument]
    pub fn take_instance_handle(&mut self, instance_id: InstanceID) -> Result<PluginHandle, Error> {
        self.running.remove(&instance_id).ok_or_else(|| {
            Error::CustomError(format!(
                "{ERR_INSTANCE_NOT_RUNNING_PREFIX}{} is not running",
                instance_id
            ))
        })
    }

    /// Validiert Plugin-Startbedingungen und liefert die Daten, die man zum Start braucht.
    #[instrument]
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
    pub fn commit_started_instance(
        &mut self,
        instance_id: InstanceID,
        handle: PluginHandle,
    ) -> Result<(), Error> {
        if self.running.contains_key(&instance_id) {
            return Err(Error::CustomError(format!(
                "{ERR_INSTANCE_ALREADY_RUNNING_PREFIX}{} is already running",
                instance_id
            )));
        }
        self.running.insert(instance_id, handle);
        debug!("Committed started instance {}", instance_id);
        Ok(())
    }

    // Start Python Runner
    #[instrument]
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

    #[instrument]
    pub async fn build_started_instance(
        &self,
        plugin_index: usize,
        plugin_path: &PathBuf,
        instance_id: InstanceID,
    ) -> Result<PluginHandle, Error> {
        let (child, mut child_stdin, mut stdout_rx) =
            self.spawn_runner(plugin_path, instance_id).await?;
        let (command_tx, command_rx) = mpsc::channel(32);
        let (status_tx, status_rx) = watch::channel(InstanceState::Running);

        // Perform initial start handshake before spawning the actor
        let request_id = format!("{}-0", instance_id);
        send_runner_cmd(instance_id, &mut child_stdin, CMD_START, &request_id).await?;

        // Wait for CMD_START ACK
        timeout(TIMEOUT_START_ACK, async {
            while let Some(msg) = stdout_rx.recv().await {
                if msg.instance_id == instance_id && msg.request_id == Some(request_id.clone()) {
                    return if msg.ok.unwrap_or(false) {
                        Ok(())
                    } else {
                        Err(Error::CustomError(
                            msg.error.unwrap_or_else(|| ERR_UNKNOWN_ERROR.to_string()),
                        ))
                    };
                }
            }
            Err(Error::CustomError(ERR_PY_STDOUT_CLOSED.to_string()))
        })
        .await
        .map_err(|_| {
            Error::CustomError(format!(
                "Start handshake timed out after {:?}",
                TIMEOUT_START_ACK
            ))
        })??;

        tokio::spawn(run_instance_actor(
            instance_id,
            child,
            child_stdin,
            stdout_rx,
            command_rx,
            status_tx,
        ));
        debug!("Spawned instance actor for instance {}", instance_id);

        Ok(PluginHandle {
            plugin_index,
            command_tx,
            status_rx,
        })
    }

    #[instrument]
    pub fn register_plugins(&mut self, directory: PathBuf) -> Result<(), Error> {
        debug!("Registering plugins from '{:?}'", directory);
        // ... (remaining code unchanged)

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

            // jeweils registrieren — ignoriere Duplikate statt Fehler zu werfen
            match self.register_plugin(path.clone()) {
                Ok(()) => {}
                Err(Error::CustomError(ref s)) if s.contains("already registered") => {
                    debug!("Plugin {:?} already registered, skipping", path);
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    #[instrument]
    pub fn register_plugin(&mut self, path: PathBuf) -> Result<(), Error> {
        // Duplikate verhindern: gleicher Plugin-Pfad darf nicht zweimal registriert werden.
        // Canonicalize macht es robuster gegen ./foo.py vs foo.py vs absolute Pfade.
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.clone());
        if canonical_path.ends_with("plugin_base.py") {
            debug!("Skipping registration of 'plugin_base.py'");
            return Ok(());
        }
        debug!("Registering plugin at path {:?}", canonical_path);
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
        debug!(
            "Plugin '{}' validated and prepared for registration",
            plugin.name()
        );
        plugin.set_valid(true);
        plugin.set_validation_warnings(warnings);

        self.registered.push(plugin);
        Ok(())
    }

    #[instrument]
    pub async fn start_plugin_instance(
        &mut self,
        plugin_name: &str,
        _temp_directory: PathBuf,
        instance_id: InstanceID,
    ) -> Result<(), Error> {
        debug!(
            "start_plugin_instance: plugin='{}' instance={}",
            plugin_name, instance_id
        );
        if self.running.contains_key(&instance_id) {
            return Err(Error::CustomError(format!(
                "{ERR_INSTANCE_ALREADY_RUNNING_PREFIX}{} is already running",
                instance_id
            )));
        }

        let (plugin_index, path) = self.prepare_start(plugin_name)?;
        let handle = self
            .build_started_instance(plugin_index, &path, instance_id)
            .await?;
        self.running.insert(instance_id, handle);
        debug!(
            "Started instance {} for plugin '{}'",
            instance_id, plugin_name
        );
        Ok(())
    }

    #[instrument]
    pub async fn stop_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        debug!("stop_plugin_instance: instance={}", instance_id);
        let handle = self.running.remove(&instance_id).ok_or_else(|| {
            Error::CustomError(format!(
                "{ERR_INSTANCE_NOT_RUNNING_PREFIX}{} is not running",
                instance_id
            ))
        })?;

        Self::stop_instance_handle(handle, instance_id).await
    }

    #[instrument]
    pub async fn pause_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        let handle = self.get_instance_handle(instance_id)?;
        Self::pause_instance_handle(handle, instance_id).await
    }

    #[instrument]
    pub async fn resume_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        let handle = self.get_instance_handle(instance_id)?;
        Self::resume_instance_handle(handle, instance_id).await
    }

    #[instrument]
    pub async fn stop_instance_handle(
        handle: PluginHandle,
        _instance_id: InstanceID,
    ) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();
        handle
            .command_tx
            .send(PluginCommand::Stop(tx))
            .await
            .map_err(|_| Error::CustomError("Actor dead".to_string()))?;
        debug!("Sent stop command to instance {}", _instance_id);
        match timeout(TIMEOUT_SOFT_STOP_ACK, rx).await {
            Ok(Ok(res)) => res,
            _ => {
                warn!("Soft stop failed or timed out, process will be killed by actor cleanup");
                Ok(())
            }
        }
    }

    #[instrument]
    pub async fn pause_instance_handle(
        handle: PluginHandle,
        _instance_id: InstanceID,
    ) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();
        handle
            .command_tx
            .send(PluginCommand::Pause(tx))
            .await
            .map_err(|_| Error::CustomError("Actor dead".to_string()))?;
        debug!("Sent pause command to instance {}", _instance_id);
        let res = timeout(TIMEOUT_PAUSE_ACK, rx)
            .await
            .map_err(|_| Error::CustomError("Pause timeout".to_string()))?;

        res.map_err(|e| Error::CustomError(format!("Actor dropped response: {e}")))?
    }

    #[instrument]
    pub async fn resume_instance_handle(
        handle: PluginHandle,
        _instance_id: InstanceID,
    ) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();
        handle
            .command_tx
            .send(PluginCommand::Resume(tx))
            .await
            .map_err(|_| Error::CustomError("Actor dead".to_string()))?;
        debug!("Sent resume command to instance {}", _instance_id);
        let res = timeout(TIMEOUT_RESUME_ACK, rx)
            .await
            .map_err(|_| Error::CustomError("Resume timeout".to_string()))?;

        res.map_err(|e| Error::CustomError(format!("Actor dropped response: {e}")))?
    }

    #[instrument]
    pub fn get_running_instances(&self) -> Vec<(&Plugin, InstanceID, InstanceState)> {
        self.running
            .iter()
            .map(|(instance_id, handle)| {
                let plugin = &self.registered[handle.plugin_index];
                let state = *handle.status_rx.borrow();
                (plugin, *instance_id, state)
            })
            .collect()
    }

    // Ausgabe aller registrierten Plugins als Liste von &Plugin
    #[instrument]
    pub fn get_registered_plugins(&self) -> Vec<&Plugin> {
        self.registered.iter().collect()
    }

    // finden und enable
    #[instrument]
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
    #[instrument]
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

async fn send_runner_cmd(
    instance_id: InstanceID,
    stdin: &mut ChildStdin,
    cmd: &str,
    request_id: &str,
) -> Result<(), Error> {
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
    let req = serde_json::Value::Object(req);
    let line = req.to_string() + "\n";
    stdin
        .write_all(line.as_bytes())
        .await
        .map_err(|e| Error::CustomError(format!("{ERR_FAILED_SEND_CMD_PREFIX}{e}")))?;
    stdin
        .flush()
        .await
        .map_err(|e| Error::CustomError(format!("{ERR_FAILED_FLUSH_CMD_PREFIX}{e}")))?;
    Ok(())
}

#[instrument(skip(child, child_stdin, stdout_rx, command_rx, status_tx))]
async fn run_instance_actor(
    instance_id: InstanceID,
    mut child: Child,
    mut child_stdin: ChildStdin,
    mut stdout_rx: mpsc::Receiver<RunnerMsg>,
    mut command_rx: mpsc::Receiver<PluginCommand>,
    status_tx: watch::Sender<InstanceState>,
) {
    let mut pending_acks: HashMap<String, oneshot::Sender<Result<(), Error>>> = HashMap::new();
    let mut next_request_seq = 1u64;

    loop {
        tokio::select! {
            cmd = command_rx.recv() => {
                match cmd {
                    Some(PluginCommand::Stop(reply)) => {
                        let request_id = format!("{}-{}", instance_id, next_request_seq);
                        next_request_seq += 1;
                        if let Err(e) = send_runner_cmd(instance_id, &mut child_stdin, CMD_STOP, &request_id).await {
                            let _ = reply.send(Err(e));
                        } else {
                            pending_acks.insert(request_id, reply);
                        }
                    }
                    Some(PluginCommand::Pause(reply)) => {
                        let request_id = format!("{}-{}", instance_id, next_request_seq);
                        next_request_seq += 1;
                        if let Err(e) = send_runner_cmd(instance_id, &mut child_stdin, CMD_PAUSE, &request_id).await {
                            let _ = reply.send(Err(e));
                        } else {
                            pending_acks.insert(request_id, reply);
                        }
                    }
                    Some(PluginCommand::Resume(reply)) => {
                        let request_id = format!("{}-{}", instance_id, next_request_seq);
                        next_request_seq += 1;
                        if let Err(e) = send_runner_cmd(instance_id, &mut child_stdin, CMD_RESUME, &request_id).await {
                            let _ = reply.send(Err(e));
                        } else {
                            pending_acks.insert(request_id, reply);
                        }
                    }
                    None => break,
                }
            }
            msg = stdout_rx.recv() => {
                match msg {
                    Some(msg) => {
                        if msg.instance_id != instance_id { continue; }
                        if let Some(request_id) = msg.request_id {
                            if let Some(reply) = pending_acks.remove(&request_id) {
                                if msg.ok.unwrap_or(false) {
                                    let _ = reply.send(Ok(()));
                                    // Update state if it was a pause/resume command
                                    if request_id.contains(CMD_PAUSE) {
                                        status_tx.send(InstanceState::Paused).ok();
                                    } else if request_id.contains(CMD_RESUME) {
                                        status_tx.send(InstanceState::Running).ok();
                                    }
                                } else {
                                    let err = msg.error.unwrap_or_else(|| ERR_UNKNOWN_ERROR.to_string());
                                    let _ = reply.send(Err(Error::CustomError(err)));
                                }
                            }
                        }
                        if let Some(ev) = &msg.event {
                            // Special handling for logs emitted from plugin (via Python logging)
                            if ev == "log" {
                                if let Some(val) = &msg.result {
                                    if let Some(level) = val.get("level").and_then(|v| v.as_str()) {
                                        let message = val.get("msg").and_then(|v| v.as_str()).unwrap_or_default();
                                        // plugin::plugin_name.instance_id
                                        let logger_name = format!("plugin.{}.{}", instance_id, level.to_lowercase());
                                        match level {
                                            "DEBUG" => debug!("{} {}", logger_name, message),
                                            "INFO" => info!("{} {}", logger_name, message),
                                            "WARN" | "WARNING" => warn!("{} {}", logger_name, message),
                                            "ERROR" | "CRITICAL" => error!("{} {}", logger_name, message),
                                            _ => debug!("{} {}", logger_name, message),
                                        }
                                    }
                                }
                                continue;
                            }

                            debug!(LOG_RUNNER_EVENT, instance_id, ev);
                            if ev == "exited" {
                                status_tx.send(if msg.ok.unwrap_or(false) { InstanceState::Completed } else { InstanceState::Failed }).ok();
                                break;
                            }
                        }
                    }
                    None => break,
                }
            }
            exit_status = child.wait() => {
                let s = match exit_status {
                    Ok(s) if s.success() => InstanceState::Completed,
                    _ => InstanceState::Failed,
                };
                status_tx.send(s).ok();
                break;
            }
        }
    }
    let _ = child.kill().await;
}
