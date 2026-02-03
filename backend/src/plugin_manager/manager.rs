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

#[derive(Debug)]
struct RunningInstance {
    plugin_index: usize,
    state: InstanceState,
    child: Child,
    child_stdin: ChildStdin,
    stdout_rx: mpsc::Receiver<RunnerMsg>,
    next_request_seq: u64,
}

#[derive(Debug)]
pub struct PluginManager {
    storage_manager: StorageManager,
    registered: Vec<Plugin>,
    running: HashMap<InstanceID, RunningInstance>,
}

impl PluginManager {
    pub fn new(storage_manager: StorageManager) -> Self {
        Self {
            storage_manager,
            registered: Vec::new(),
            running: HashMap::new(),
        }
    }

    async fn spawn_runner(
        &self,
        plugin_path: &PathBuf,
        instance_id: InstanceID,
    ) -> Result<(Child, ChildStdin, mpsc::Receiver<RunnerMsg>), Error> {
        // Achtung: Pfad muss zu deiner echten Datei passen.
        // Aktuell liegt sie bei dir unter plugin_manager/plugins/plugin_runner.py
        let runner_path = PathBuf::from("src/plugin_manager/plugins/plugin_runner.py");

        let mut child = Command::new("python")
            .arg("-u")
            .arg(runner_path)
            .arg("--plugin-path")
            .arg(plugin_path)
            .arg("--instance-id")
            .arg(instance_id.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::CustomError(format!("Failed to spawn python runner: {e}")))?;

        let child_stdin = child
            .stdin
            .take()
            .ok_or_else(
                || Error::CustomError("Failed to open stdin for python runner".to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(
                || Error::CustomError("Failed to open stdout for python runner".to_string()))?;

        // stderr drainen, damit der Prozess nie wegen voller Pipe blockiert
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    error!("python stderr: {}", line);
                }
            });
        }

        let (tx, rx) = mpsc::channel::<RunnerMsg>(128);
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                match serde_json::from_str::<RunnerMsg>(&line) {
                    Ok(msg) => {
                        // Wenn Channel voll/geschlossen ist, einfach abbrechen
                        if tx.send(msg).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        debug!("python stdout (non-json): {} (parse err: {})", line, e);
                    }
                }
            }
        });

        Ok((child, child_stdin, rx))
    }

    pub fn register_plugins(&mut self, directory: PathBuf) -> Result<(), Error> {
        for entry in fs::read_dir(&directory)
            .map_err(|e| Error::CustomError(e.to_string()))? {
            let entry = entry.map_err(|e| Error::CustomError(e.to_string()))?;
            let path = entry.path();
            self.register_plugin(path)?;
        }
        Ok(())
    }

    pub fn register_plugin(&mut self, path: PathBuf) -> Result<(), Error> {
        // 1) Validieren (harte Fehler -> nicht registrieren)
        // 2) Warnings merken (soft)
        let warnings = python_bridge::validate_plugin_module(path.as_path())?;
        for w in &warnings {
            warn!("{w}");
        }

        // 2) Constants lesen (best-effort)
        let fallback_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let fallback_description = format!("Plugin loaded from {:?}", path);

        let (py_name, py_description, py_trigger) =
            python_bridge::read_module_constants(path.as_path()).unwrap_or((None, None, None));

        let name = py_name.unwrap_or(fallback_name);
        let description = py_description.unwrap_or(fallback_description);

        let trigger = match py_trigger.as_deref() {
            Some("manual") | None => Trigger::Manual,
            Some("on_entry_create") => Trigger::OnEntryCreate,
            Some("on_entry_update") => Trigger::OnEntryUpdate,
            Some("on_entry_delete") => Trigger::OnEntryDelete,
            Some(other) if other.starts_with("on_schedule:") => {
                Trigger::OnSchedule(other.trim_start_matches("on_schedule:").trim().to_string())
            }
            _ => Trigger::Manual,
        };

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
        if self.running.contains_key(&instance_id) {
            return Err(Error::CustomError(format!(
                "Instance {} is already running",
                instance_id
            )));
        }

        let plugin_index = self
            .registered
            .iter()
            .position(|p| p.name().as_str() == plugin_name)
            .ok_or_else(|| Error::CustomError(
                format!("Plugin '{}' is not registered", plugin_name)))?;

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

        let (child, child_stdin, stdout_rx) =
            self.spawn_runner(reg_plugin.path(), instance_id).await?;

        let mut inst = RunningInstance {
            plugin_index,
            state: Running,
            child,
            child_stdin,
            stdout_rx,
            next_request_seq: 1,
        };

        // Runner startet run()-Thread erst nach "start" -> mit ACK warten
        let _ = 
            Self::send_cmd_ack(&mut inst, instance_id, "start", Duration::from_secs(5))
            .await?;

        self.running.insert(instance_id, inst);
        Ok(())
    }

    pub async fn stop_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        let mut entry = self.running.remove(&instance_id).ok_or_else(|| {
            Error::CustomError(format!("Instance {} is not running", instance_id))
        })?;

        // 1) Soft stop mit Bestätigung (damit es nicht "ignoriert" wird)
        let soft = 
            Self::send_cmd_ack(&mut entry, instance_id, "stop", Duration::from_secs(2))
            .await;

        if soft.is_ok() {
            // 2) kurz warten, ob sauber beendet
            if timeout(Duration::from_secs(2), entry.child.wait()).await.is_ok() {
                return Ok(());
            }
            warn!("Soft stop ACK ok, but process did not exit quickly; forcing kill.");
        } else {
            warn!("Soft stop failed/timeout; forcing kill. err={:?}", soft.err());
        }

        // 3) Hard stop (Not-Aus)
        entry
            .child
            .kill()
            .await
            .map_err(|e| Error::CustomError(format!("Failed to kill python runner: {e}")))?;

        let _ = timeout(Duration::from_secs(2), entry.child.wait()).await;
        Ok(())
    }

    pub async fn pause_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        let entry = self.running.get_mut(&instance_id).ok_or_else(|| {
            Error::CustomError(format!("Instance {} is not running", instance_id))
        })?;

        if entry.state == InstanceState::Paused {
            return Ok(());
        }

        let _ = Self::send_cmd_ack(entry, instance_id, "pause", Duration::from_secs(2))
            .await?;
        entry.state = InstanceState::Paused;
        Ok(())
    }

    pub async fn resume_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        let entry = self.running.get_mut(&instance_id).ok_or_else(|| {
            Error::CustomError(format!("Instance {} is not running", instance_id))
        })?;

        if entry.state == Running {
            return Ok(());
        }

        let _ = Self::send_cmd_ack(entry, instance_id, "resume", Duration::from_secs(2))
            .await?;
        entry.state = Running;
        Ok(())
    }

    async fn send_cmd_ack(
        inst: &mut RunningInstance,
        instance_id: InstanceID,
        cmd: &str,
        wait: Duration,
    ) -> Result<RunnerMsg, Error> {
        let request_id = format!("{}-{}", instance_id, inst.next_request_seq);
        inst.next_request_seq += 1;

        let req = serde_json::json!({
            "instance_id": instance_id,
            "request_id": request_id,
            "cmd": cmd
        });

        let line = req.to_string() + "\n";
        inst.child_stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| 
                Error::CustomError(format!("Failed to send cmd to python runner: {e}")))?;
        inst.child_stdin
            .flush()
            .await
            .map_err(|e| 
                Error::CustomError(format!("Failed to flush cmd to python runner: {e}")))?;

        let fut = async {
            loop {
                let msg = inst
                    .stdout_rx
                    .recv()
                    .await
                    .ok_or_else(|| Error::CustomError("Python runner stdout closed".to_string()))?;

                if msg.instance_id != instance_id {
                    continue;
                }

                if msg.request_id.as_deref() == Some(req["request_id"].as_str().unwrap()) {
                    if msg.ok.unwrap_or(false) {
                        return Ok(msg);
                    }

                    let err = msg.error.unwrap_or_else(|| "unknown_error".to_string());
                    let trace = msg.trace.unwrap_or_default();

                    if trace.is_empty() {
                        return Err(Error::CustomError(format!("Runner cmd '{cmd}' failed: {err}")));
                    }

                    return Err(Error::CustomError(format!(
                        "Runner cmd '{cmd}' failed: {err}\nPython traceback:\n{trace}"
                    )));
                }

                if let Some(ev) = &msg.event {
                    debug!("runner event (instance {}): {}", instance_id, ev);
                }
            }
        };

        timeout(wait, fut)
            .await
            .map_err(|_| Error::CustomError(
                format!("Runner cmd '{cmd}' timed out after {:?}", wait)))?
    }

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
