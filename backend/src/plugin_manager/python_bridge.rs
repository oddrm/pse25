use pyo3::prelude::*;
use std::path::Path;

use crate::error::Error;

// -------------------- constants --------------------
const ERR_PLUGIN_NO_PARENT_DIR: &str = "Plugin path has no parent directory";
const ERR_INVALID_PLUGIN_FILENAME: &str = "Invalid plugin filename";

const PY_MOD_SYS: &str = "sys";
const PY_SYS_PATH_ATTR: &str = "path";
const PY_SYS_PATH_INSERT: &str = "insert";

const PY_IMPORT_SYS_FAILED_PREFIX: &str = "Python import sys failed: ";
const PY_SYS_PATH_ACCESS_FAILED_PREFIX: &str = "Python sys.path access failed: ";
const PY_SYS_PATH_INSERT_FAILED_PREFIX: &str = "Python sys.path insert failed: ";
const PY_IMPORT_MODULE_FAILED_PREFIX: &str = "Python import '";
const PY_IMPORT_MODULE_FAILED_SUFFIX: &str = "' failed: ";

const PY_ATTR_PLUGIN_NAME: &str = "PLUGIN_NAME";
const PY_ATTR_PLUGIN_DESCRIPTION: &str = "PLUGIN_DESCRIPTION";
const PY_ATTR_PLUGIN_TRIGGER: &str = "PLUGIN_TRIGGER";

const PY_ATTR_PLUGIN_IMPL: &str = "PluginImpl";
const PY_ATTR_RUN: &str = "run";

const ERR_PLUGIN_IMPL_NOT_CALLABLE: &str =
    "PluginImpl exists but is not callable (expected class or factory function)";

const ERR_PLUGIN_HAS_NO_PLUGIN_IMPL_PREFIX: &str = "Plugin '";
const ERR_PLUGIN_HAS_NO_PLUGIN_IMPL_MID: &str = "' has no PluginImpl: ";

const ERR_PLUGIN_IMPL_HAS_NO_RUN_PREFIX: &str = "Plugin '";
const ERR_PLUGIN_IMPL_HAS_NO_RUN_MID: &str = "': PluginImpl has no run() method: ";

const ERR_PLUGIN_RUN_NOT_CALLABLE_PREFIX: &str = "Plugin '";
const ERR_PLUGIN_RUN_NOT_CALLABLE_SUFFIX: &str =
    "': PluginImpl.run exists but is not callable";

const WARN_MISSING_PLUGIN_NAME_PREFIX: &str = "Plugin '";
const WARN_MISSING_PLUGIN_NAME_SUFFIX: &str =
    "': missing PLUGIN_NAME constant (will use filename fallback)";

const WARN_MISSING_PLUGIN_DESCRIPTION_PREFIX: &str = "Plugin '";
const WARN_MISSING_PLUGIN_DESCRIPTION_SUFFIX: &str =
    "': missing PLUGIN_DESCRIPTION constant (will use fallback description)";

const WARN_MISSING_PLUGIN_TRIGGER_PREFIX: &str = "Plugin '";
const WARN_MISSING_PLUGIN_TRIGGER_SUFFIX: &str =
    "': missing PLUGIN_TRIGGER constant (will default to 'manual')";

// NOTE:
// Runtime-Steuerung (run/stop/pause/resume) läuft ab jetzt über den separaten Python Runner Prozess
// (plugin_runner.py) mit JSON-Commands + ACKs. Diese Bridge ist nur noch für
// validate_plugin_module() und read_module_constants() gedacht.

// --- helpers ---
fn prepare_module_import<'py>(
    py: Python<'py>,
    plugin_file: &Path,
) -> Result<(Bound<'py, PyModule>, String), Error> {
    // Verzeichnis wo Plugin liegt
    let parent = plugin_file
        .parent()
        .ok_or_else(|| Error::CustomError(ERR_PLUGIN_NO_PARENT_DIR.to_string()))?;

    // Dateiname ohne Endung
    let module_name = plugin_file
        .file_stem() // ohne Dateiendung
        .and_then(|s| s.to_str())
        .ok_or_else(|| Error::CustomError(ERR_INVALID_PLUGIN_FILENAME.to_string()))?
        .to_string();

    // Modul sys
    let sys = py
        .import(PY_MOD_SYS)
        .map_err(|e| Error::CustomError(format!("{PY_IMPORT_SYS_FAILED_PREFIX}{e}")))?;

    // Liste in Python mit Suchpfaden für Imports
    let sys_path = sys
        .getattr(PY_SYS_PATH_ATTR)
        .map_err(|e| Error::CustomError(format!("{PY_SYS_PATH_ACCESS_FAILED_PREFIX}{e}")))?;

    // Plugin-Ordner für Python sichtbar -> richtiger Code/ keine Namenskollisionen
    sys_path
        .call_method1(PY_SYS_PATH_INSERT, (0, parent.to_string_lossy().as_ref()))
        .map_err(|e| Error::CustomError(format!("{PY_SYS_PATH_INSERT_FAILED_PREFIX}{e}")))?;

    // Import des Plugins in Python-Form
    let module = py.import(&module_name).map_err(|e| {
        Error::CustomError(format!(
            "{PY_IMPORT_MODULE_FAILED_PREFIX}{module_name}{PY_IMPORT_MODULE_FAILED_SUFFIX}{e}"
        ))
    })?;

    Ok((module, module_name))
}

pub fn read_module_constants(
    plugin_file: &Path,
) -> Result<(Option<String>, Option<String>, Option<String>), Error> {
    // im Wesentlichen Aktion in Python (Closure)
    Python::attach(|py| {
        // Vorbereitung Plugin-Import in Rust
        let (module, _module_name) = prepare_module_import(py, plugin_file)?;

        // bei allen: wenn nicht funktioniert -> None
        let name = module
            .getattr(PY_ATTR_PLUGIN_NAME)
            .ok()
            .and_then(|v| v.extract::<String>().ok());

        let description = module
            .getattr(PY_ATTR_PLUGIN_DESCRIPTION)
            .ok()
            .and_then(|v| v.extract::<String>().ok());

        let trigger = module
            .getattr(PY_ATTR_PLUGIN_TRIGGER)
            .ok()
            .and_then(|v| v.extract::<String>().ok());

        Ok((name, description, trigger))
    })
}

pub fn validate_plugin_module(plugin_file: &Path) -> Result<Vec<String>, Error> {
    // wieder in Python (Closure)
    Python::attach(|py| {
        // Liste der Warnings
        let mut warnings: Vec<String> = Vec::new();

        let (module, module_name) = prepare_module_import(py, plugin_file)?;

        // Hard requirements (sonst nicht startbar)
        // -> Implementierung von PluginImpl und run()
        let plugin_impl = module.getattr(PY_ATTR_PLUGIN_IMPL).map_err(|e| {
            Error::CustomError(format!(
                "{ERR_PLUGIN_HAS_NO_PLUGIN_IMPL_PREFIX}\
                {module_name}{ERR_PLUGIN_HAS_NO_PLUGIN_IMPL_MID}{e}"
            ))
        })?;

        // Richtiges Format -> Klasse oder Factory-Funktion
        if !plugin_impl.is_callable() {
            return Err(Error::CustomError(ERR_PLUGIN_IMPL_NOT_CALLABLE.to_string()));
        }

        // Zugriff auf run() Methode
        let run_attr = plugin_impl.getattr(PY_ATTR_RUN).map_err(|e| {
            Error::CustomError(format!(
                "{ERR_PLUGIN_IMPL_HAS_NO_RUN_PREFIX}{module_name}{ERR_PLUGIN_IMPL_HAS_NO_RUN_MID}{e}"
            ))
        })?;

        if !run_attr.is_callable() {
            return Err(Error::CustomError(format!(
                "{ERR_PLUGIN_RUN_NOT_CALLABLE_PREFIX}{module_name}{ERR_PLUGIN_RUN_NOT_CALLABLE_SUFFIX}"
            )));
        }

        // Soft requirements (nur Warnungen) -> Existenz von Konstanten
        if module.getattr(PY_ATTR_PLUGIN_NAME).is_err() {
            warnings.push(format!(
                "{WARN_MISSING_PLUGIN_NAME_PREFIX}{module_name}{WARN_MISSING_PLUGIN_NAME_SUFFIX}"
            ));
        }
        if module.getattr(PY_ATTR_PLUGIN_DESCRIPTION).is_err() {
            warnings.push(format!(
                "{WARN_MISSING_PLUGIN_DESCRIPTION_PREFIX}{module_name}{WARN_MISSING_PLUGIN_DESCRIPTION_SUFFIX}"
            ));
        }
        if module.getattr(PY_ATTR_PLUGIN_TRIGGER).is_err() {
            warnings.push(format!(
                "{WARN_MISSING_PLUGIN_TRIGGER_PREFIX}{module_name}{WARN_MISSING_PLUGIN_TRIGGER_SUFFIX}"
            ));
        }

        Ok(warnings)
    })
}