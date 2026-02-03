use pyo3::prelude::*;
use std::path::Path;

use crate::error::Error;

// NOTE:
// Runtime-Steuerung (run/stop/pause/resume) läuft ab jetzt über den separaten Python Runner Prozess
// (plugin_runner.py) mit JSON-Commands + ACKs. Diese Bridge ist nur noch für
// validate_plugin_module() und read_module_constants() gedacht.

// pub fn load_plugin_instance(...) { ... }  // optional: entfernen, wenn nicht mehr genutzt
// pub fn call_run(...) { ... }              // entfernen
// pub fn call_stop(...) { ... }             // entfernen
// pub fn call_pause(...) { ... }            // entfernen
// pub fn call_resume(...) { ... }           // entfernen

pub fn read_module_constants(
    plugin_file: &Path,
) -> Result<(Option<String>, Option<String>, Option<String>), Error> {
    Python::attach(|py| {
        let parent = plugin_file
            .parent()
            .ok_or_else(|| Error::CustomError("Plugin path has no parent directory".to_string()))?;

        let module_name = plugin_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::CustomError("Invalid plugin filename".to_string()))?;

        let sys = py
            .import("sys")
            .map_err(|e| Error::CustomError(format!("Python import sys failed: {e}")))?;

        let sys_path = sys
            .getattr("path")
            .map_err(|e| Error::CustomError(format!("Python sys.path access failed: {e}")))?;

        sys_path
            .call_method1("insert", (0, parent.to_string_lossy().as_ref()))
            .map_err(|e| Error::CustomError(format!("Python sys.path insert failed: {e}")))?;

        let module = py.import(module_name).map_err(|e| {
            Error::CustomError(format!("Python import '{module_name}' failed: {e}"))
        })?;

        let name = module
            .getattr("PLUGIN_NAME")
            .ok()
            .and_then(|v| v.extract::<String>().ok());

        let description = module
            .getattr("PLUGIN_DESCRIPTION")
            .ok()
            .and_then(|v| v.extract::<String>().ok());

        let trigger = module
            .getattr("PLUGIN_TRIGGER")
            .ok()
            .and_then(|v| v.extract::<String>().ok());

        Ok((name, description, trigger))
    })
}

pub fn validate_plugin_module(plugin_file: &Path) -> Result<Vec<String>, Error> {
    Python::attach(|py| {
        let mut warnings: Vec<String> = Vec::new();

        let parent = plugin_file.parent().ok_or_else(|| {
            Error::CustomError("Plugin path has no parent directory".to_string())
        })?;

        let module_name = plugin_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::CustomError("Invalid plugin filename".to_string()))?;

        let sys = py
            .import("sys")
            .map_err(|e| Error::CustomError(format!("Python import sys failed: {e}")))?;

        let sys_path = sys
            .getattr("path")
            .map_err(|e| Error::CustomError(format!("Python sys.path access failed: {e}")))?;

        sys_path
            .call_method1("insert", (0, parent.to_string_lossy().as_ref()))
            .map_err(|e| Error::CustomError(format!(
                "Python sys.path insert failed: {e}")))?;

        let module = py
            .import(module_name)
            .map_err(|e| Error::CustomError(format!(
                "Python import '{module_name}' failed: {e}")))?;

        // Hard requirements (sonst nicht startbar)
        let plugin_impl = module.getattr("PluginImpl").map_err(|e| {
            Error::CustomError(format!("Plugin '{module_name}' has no PluginImpl: {e}"))
        })?;

        if !plugin_impl.is_callable() {
            return Err(Error::CustomError(
                "PluginImpl exists but is not callable (expected class or factory function)"
                    .to_string(),
            ));
        }

        // Neu: run() muss vorhanden und callable sein (wir nutzen kein step() mehr)
        let run_attr = plugin_impl.getattr("run").map_err(|e| {
            Error::CustomError(format!(
                "Plugin '{module_name}': PluginImpl has no run() method: {e}"
            ))
        })?;
        
        if !run_attr.is_callable() {
            return Err(Error::CustomError(format!(
                "Plugin '{module_name}': PluginImpl.run exists but is not callable"
            )));
        }

        // Soft requirements (nur Warnungen)
        if module.getattr("PLUGIN_NAME").is_err() {
            warnings.push(format!(
                "Plugin '{module_name}': missing PLUGIN_NAME constant (will use filename fallback)"
            ));
        }
        if module.getattr("PLUGIN_DESCRIPTION").is_err() {
            warnings.push(format!(
                "Plugin '{module_name}': \
                missing PLUGIN_DESCRIPTION constant (will use fallback description)"
            ));
        }
        if module.getattr("PLUGIN_TRIGGER").is_err() {
            warnings.push(format!(
                "Plugin '{module_name}': missing PLUGIN_TRIGGER constant (will default to 'manual')"
            ));
        }

        Ok(warnings)
    })
}