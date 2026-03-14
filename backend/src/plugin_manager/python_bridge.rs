use pyo3::prelude::*;
use std::path::Path;
use tracing::{debug, warn};

use crate::error::Error;

// -------------------- constants --------------------
// Fehlertexte und Python-Konstanten sind bewusst zentral definiert,
// damit sie an einer Stelle gepflegt werden können und die eigentliche
// Logik darunter besser lesbar bleibt.
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
const ERR_PLUGIN_RUN_NOT_CALLABLE_SUFFIX: &str = "': PluginImpl.run exists but is not callable";

const WARN_MISSING_PLUGIN_NAME_PREFIX: &str = "Plugin '";
const WARN_MISSING_PLUGIN_NAME_SUFFIX: &str =
    "': missing PLUGIN_NAME constant (will use filename fallback)";

const WARN_MISSING_PLUGIN_DESCRIPTION_PREFIX: &str = "Plugin '";
const WARN_MISSING_PLUGIN_DESCRIPTION_SUFFIX: &str =
    "': missing PLUGIN_DESCRIPTION constant (will use fallback description)";

const WARN_MISSING_PLUGIN_TRIGGER_PREFIX: &str = "Plugin '";
const WARN_MISSING_PLUGIN_TRIGGER_SUFFIX: &str =
    "': missing PLUGIN_TRIGGER constant (will default to 'manual')";

/// Bereitet den Import eines Python-Plugin-Moduls vor.
///
/// Aufgabe dieser Funktion:
/// 1. Modulnamen aus dem Dateinamen ableiten
/// 2. Plugin-Verzeichnis in `sys.path` eintragen
/// 3. evtl. gecachte Modulversion aus `sys.modules` entfernen
/// 4. Modul frisch importieren
///
/// Das ist wichtig, damit Änderungen an Plugin-Dateien auch ohne Neustart
/// korrekt neu eingelesen werden können.
fn prepare_module_import<'py>(
    py: Python<'py>,
    plugin_file: &Path,
) -> Result<(Bound<'py, PyModule>, String), Error> {
    // Verzeichnis, in dem die Plugin-Datei liegt.
    // Dieses Verzeichnis muss später in `sys.path`, damit Python das Modul findet.
    let parent = plugin_file
        .parent()
        .ok_or_else(|| Error::CustomError(ERR_PLUGIN_NO_PARENT_DIR.to_string()))?;

    // Dateiname ohne Endung, z. B. `my_plugin.py` -> `my_plugin`.
    // Dieser Name wird als Python-Modulname verwendet.
    let module_name = plugin_file
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| Error::CustomError(ERR_INVALID_PLUGIN_FILENAME.to_string()))?
        .to_string();

    debug!(
        "prepare_module_import: module_name='{}' parent='{}'",
        module_name,
        parent.display()
    );

    // Importiere das Python-Systemmodul `sys`.
    let sys = py
        .import(PY_MOD_SYS)
        .map_err(|e| Error::CustomError(format!("{PY_IMPORT_SYS_FAILED_PREFIX}{e}")))?;

    // Hole `sys.path`, also die Liste der Suchpfade für Python-Imports.
    let sys_path = sys
        .getattr(PY_SYS_PATH_ATTR)
        .map_err(|e| Error::CustomError(format!("{PY_SYS_PATH_ACCESS_FAILED_PREFIX}{e}")))?;

    // Füge den Plugin-Ordner an den Anfang ein, damit genau dieses Modul
    // bevorzugt gefunden wird.
    sys_path
        .call_method1(PY_SYS_PATH_INSERT, (0, parent.to_string_lossy().as_ref()))
        .map_err(|e| Error::CustomError(format!("{PY_SYS_PATH_INSERT_FAILED_PREFIX}{e}")))?;

    // Wichtig für Hot-Reload-ähnliches Verhalten:
    // Entferne das Modul aus `sys.modules`, damit Python es frisch lädt
    // und nicht eine alte gecachte Version verwendet.
    if let Ok(modules) = sys.getattr("modules") {
        let _ = modules.call_method1("pop", (module_name.as_str(), py.None()));
        debug!(
            "Removed module '{}' from sys.modules to force fresh import",
            module_name
        );
    }

    // Jetzt das Plugin-Modul wirklich importieren.
    let module = py.import(&module_name).map_err(|e| {
        Error::CustomError(format!(
            "{PY_IMPORT_MODULE_FAILED_PREFIX}{module_name}{PY_IMPORT_MODULE_FAILED_SUFFIX}{e}"
        ))
    })?;

    debug!("Imported python module '{}' successfully", module_name);

    Ok((module, module_name))
}

/// Liest optionale Konstanten aus dem Python-Modul aus.
///
/// Diese Funktion ist bewusst tolerant:
/// Fehlt eine Konstante, wird `None` zurückgegeben statt eines Fehlers.
/// So können Fallback-Werte verwendet werden.
pub fn read_module_constants(
    plugin_file: &Path,
) -> Result<(Option<String>, Option<String>, Option<String>), Error> {
    // im Wesentlichen Aktion in Python (Closure)
    Python::attach(|py| {
        // Vorbereitung Plugin-Import in Rust
        let (module, module_name) = prepare_module_import(py, plugin_file)?;

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

        debug!(
            "read_module_constants {}: name={:?} description={:?} trigger={:?}",
            module_name, name, description, trigger
        );

        Ok((name, description, trigger))
    })
}

pub fn validate_plugin_module(plugin_file: &Path) -> Result<Vec<String>, Error> {
    Python::attach(|py| {
        debug!("validate_plugin_module: validating {:?}", plugin_file);

        let mut warnings: Vec<String> = Vec::new();

        let (module, module_name) = prepare_module_import(py, plugin_file)?;

        // ---------------- Hard requirements ----------------
        // Das Plugin muss eine `PluginImpl` bereitstellen.
        let plugin_impl = module.getattr(PY_ATTR_PLUGIN_IMPL).map_err(|e| {
            Error::CustomError(format!(
                "{ERR_PLUGIN_HAS_NO_PLUGIN_IMPL_PREFIX}\
                {module_name}{ERR_PLUGIN_HAS_NO_PLUGIN_IMPL_MID}{e}"
            ))
        })?;

        // `PluginImpl` muss aufrufbar sein, also typischerweise eine Klasse
        // oder Factory-Funktion.
        if !plugin_impl.is_callable() {
            return Err(Error::CustomError(ERR_PLUGIN_IMPL_NOT_CALLABLE.to_string()));
        }

        // Danach prüfen wir, ob `run()` vorhanden ist.
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

        // ---------------- Soft requirements ----------------
        // Diese Angaben sind nützlich für Anzeige und Konfiguration,
        // aber kein Start-Hindernis.
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

        if !warnings.is_empty() {
            for w in &warnings {
                warn!("{}", w);
            }
        }

        debug!(
            "validate_plugin_module {}: returning {} warnings",
            module_name,
            warnings.len()
        );

        Ok(warnings)
    })
}
