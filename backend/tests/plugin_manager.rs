mod common;

use diesel::prelude::*;
use std::path::PathBuf;
use std::sync::Once;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs;

static INIT: Once = Once::new();

use backend::plugin_manager::manager::PluginManager;
use backend::plugin_manager::plugin::{Plugin, Trigger};
use backend::storage::storage_manager::StorageManager;


fn unique_temp_dir_path(dir_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("pse25_{nanos}_{dir_name}"))
}

fn try_storage_manager_from_env() -> Option<StorageManager> {
    let db_url = std::env::var("DATABASE_URL").ok()?;
    StorageManager::new(&db_url).ok()
}

#[test]
fn plugin_new_sets_defaults_and_fields() {
    // Goal:
    // - `Plugin::new` copies provided fields
    // - and sets sane defaults for flags and warnings.

    // Arrange
    let p = Plugin::new(
        "p1".to_string(),
        "desc".to_string(),
        Trigger::Manual,
        PathBuf::from("some/path/plugin.py"),
    );

    // Assert
    assert_eq!(p.name().as_str(), "p1", "name should be stored unchanged");
    assert_eq!(
        p.description().as_str(),
        "desc",
        "description should be stored unchanged"
    );
    assert!(
        matches!(p.trigger(), Trigger::Manual),
        "trigger should be stored unchanged"
    );
    assert!(p.enabled(), "Plugin should be enabled by default");
    assert!(p.valid(), "Plugin should be valid by default");
    assert!(
        p.validation_warnings().is_empty(),
        "Plugin should start without warnings"
    );
}

#[test]
fn plugin_enable_disable_and_valid_flags_work() {
    // Goal:
    // - setters mutate only the intended flags
    // - warnings vector is stored and retrievable

    // Arrange
    let mut p = Plugin::new(
        "p1".to_string(),
        "desc".to_string(),
        Trigger::Manual,
        PathBuf::from("some/path/plugin.py"),
    );

    // Act + Assert: enabled
    p.set_enabled(false);
    assert!(!p.enabled(), "set_enabled(false) must disable the plugin");

    p.set_enabled(true);
    assert!(p.enabled(), "set_enabled(true) must enable the plugin");

    // Act + Assert: valid
    p.set_valid(false);
    assert!(!p.valid(), "set_valid(false) must mark the plugin invalid");

    // Act + Assert: warnings
    p.set_validation_warnings(vec!["w1".to_string(), "w2".to_string()]);
    assert_eq!(
        p.validation_warnings().len(),
        2,
        "warnings list length should match input"
    );
    assert_eq!(p.validation_warnings()[0], "w1");
    assert_eq!(p.validation_warnings()[1], "w2");
}

#[test]
fn plugin_manager_new_starts_empty() {
    // Goal:
    // - a fresh manager has no registered plugins and no running instances.

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let pm = PluginManager::new(storage_manager);

    assert_eq!(pm.get_registered_plugins().len(), 0);
    assert_eq!(pm.get_running_instances().len(), 0);
}

#[test]
fn plugin_manager_register_plugin_registers_builtin_python_plugin() {
    // Goal:
    // - register_plugin() reads the python module constants
    // - validates module structure
    // - registers the plugin with expected name/trigger and defaults.

    crate::common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    // Arrange
    let mut pm = PluginManager::new(storage_manager);
    let plugin_path = PathBuf::from("src/plugin_manager/plugins/plugin.py");

    // Act
    pm.register_plugin(plugin_path)
        .expect("register_plugin failed (python/pyo3 import or validation issue)");

    // Assert
    let registered = pm.get_registered_plugins();
    assert_eq!(registered.len(), 1, "exactly one plugin should be registered");

    // This checks the *observable output* of reading constants from the python module.
    assert_eq!(
        registered[0].name().as_str(),
        "example_plugin",
        "python constant PLUGIN_NAME should be applied"
    );
    assert!(
        matches!(registered[0].trigger(), Trigger::Manual),
        "python constant PLUGIN_TRIGGER=manual should map to Trigger::Manual"
    );
    assert!(registered[0].enabled(), "registered plugin should default to enabled");
    assert!(registered[0].valid(), "registered plugin should be marked valid after validation");
}

#[test]
fn plugin_manager_enable_disable_by_name() {
    // Goal:
    // - enable_plugin/disable_plugin toggle only the targeted plugin by name.

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    // Arrange
    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    // Act: disable
    pm.disable_plugin("example_plugin")
        .expect("disable_plugin failed");

    // Assert: disabled
    let p = pm
        .get_registered_plugins()
        .into_iter()
        .find(|p| p.name().as_str() == "example_plugin")
        .expect("plugin not found after registration");
    assert!(!p.enabled(), "disable_plugin must set enabled=false");

    // Act: enable
    pm.enable_plugin("example_plugin")
        .expect("enable_plugin failed");

    // Assert: enabled again
    let p = pm
        .get_registered_plugins()
        .into_iter()
        .find(|p| p.name().as_str() == "example_plugin")
        .expect("plugin not found after registration");
    assert!(p.enabled(), "enable_plugin must set enabled=true");
}

#[test]
fn plugin_manager_enable_disable_unknown_plugin_returns_error() {
    // Goal:
    // - calling enable/disable for an unknown name returns an error (no silent success).

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    let err = pm
        .disable_plugin("does_not_exist")
        .expect_err("expected error when disabling unknown plugin");
    assert!(
        format!("{err:?}").contains("not found"),
        "error should indicate missing plugin"
    );

    let err = pm
        .enable_plugin("does_not_exist")
        .expect_err("expected error when enabling unknown plugin");
    assert!(
        format!("{err:?}").contains("not found"),
        "error should indicate missing plugin"
    );
}

#[test]
fn plugin_manager_load_config_and_apply_toggles_enabled_flag() {
    // Goal:
    // - YAML config is parsed
    // - matching plugin is found
    // - enabled flag is applied.

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    // Arrange
    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    let yaml = r#"
plugins:
  - name: example_plugin
    enabled: false
"#;
    let config_path = crate::common::create_yaml_config(yaml);

    // Act
    pm.load_config_and_apply(config_path.to_string_lossy().as_ref())
        .expect("load_config_and_apply failed");

    // Cleanup (best-effort)
    let _ = fs::remove_file(&config_path);

    // Assert
    let p = pm
        .get_registered_plugins()
        .into_iter()
        .find(|p| p.name().as_str() == "example_plugin")
        .expect("plugin not found");
    assert!(!p.enabled(), "config should disable example_plugin");
}

#[test]
fn plugin_manager_load_config_and_apply_errors_on_unknown_plugin() {
    // Goal:
    // - config referring to a plugin name that isn't registered must error.

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    let yaml = r#"
plugins:
  - name: does_not_exist
    enabled: true
"#;
    let config_path = crate::common::create_yaml_config(yaml);

    let err = pm
        .load_config_and_apply(config_path.to_string_lossy().as_ref())
        .expect_err("expected error for unknown plugin");

    let _ = fs::remove_file(&config_path);

    let msg = format!("{err:?}");
    assert!(
        msg.contains("not found") || msg.contains("Plugin"),
        "error message should indicate missing plugin, got: {msg}"
    );
}

#[test]
fn plugin_manager_load_config_and_apply_errors_on_missing_file() {
    // Goal:
    // - missing config path should be a readable error, not a panic.

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    let missing_path =
        crate::common::unique_temp_file_path("this_file_should_not_exist.yaml");
    let err = pm
        .load_config_and_apply(missing_path.to_string_lossy().as_ref())
        .expect_err("expected error for missing config file");

    assert!(
        format!("{err:?}").contains("Failed to read config"),
        "should fail with 'Failed to read config' prefix"
    );
}

/// Runtime-Integrationtest:
/// Prüft den kompletten Lifecycle über den echten Python-Runner:
/// register -> start -> pause -> resume -> stop
#[tokio::test]
async fn plugin_instance_lifecycle_start_pause_resume_stop() {
    crate::common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    // Arrange: PluginManager + Plugin registrieren
    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    // temp dir ist aktuell zwar ungenutzt, aber API verlangt ihn
    let temp_dir = unique_temp_dir_path("plugin_instance_lifecycle");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let instance_id = 1_u64;

    // Act: Start
    pm.start_plugin_instance("example_plugin",
                             vec![], temp_dir.clone(), instance_id)
        .await
        .expect("start_plugin_instance failed");

    // Assert: Instanz ist als running sichtbar
    let running = pm.get_running_instances();
    assert_eq!(running.len(), 1, "exactly one instance should be running");
    assert_eq!(
        running[0].0.name().as_str(),
        "example_plugin",
        "running instance should belong to example_plugin"
    );
    assert_eq!(
        running[0].1, instance_id,
        "running instance id should match"
    );

    // Act: Pause
    pm.pause_plugin_instance(instance_id)
        .await
        .expect("pause_plugin_instance failed");

    // Assert: paused Instanz soll NICHT in get_running_instances auftauchen
    // (Filter ist state==Running)
    let running = pm.get_running_instances();
    assert_eq!(
        running.len(),
        0,
        "paused instances must not be reported as running"
    );

    // Act: Resume
    pm.resume_plugin_instance(instance_id)
        .await
        .expect("resume_plugin_instance failed");

    // Assert: wieder running sichtbar
    let running = pm.get_running_instances();
    assert_eq!(running.len(), 1, "instance should be running after resume");
    assert_eq!(running[0].1, instance_id);

    // Act: Stop
    pm.stop_plugin_instance(instance_id)
        .await
        .expect("stop_plugin_instance failed");

    // Assert: keine running instances mehr
    let running = pm.get_running_instances();
    assert_eq!(running.len(), 0, "after stop there must be no running instances");

    // Cleanup (best-effort)
    let _ = fs::remove_dir_all(&temp_dir);
}

/// Runtime-Integrationtest:
/// Prüft, dass ein deaktiviertes Plugin nicht gestartet werden kann.
#[tokio::test]
async fn start_fails_when_plugin_is_disabled() {
    crate::common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    pm.disable_plugin("example_plugin")
        .expect("disable_plugin failed");

    let temp_dir = unique_temp_dir_path("plugin_disabled_start");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let err = pm
        .start_plugin_instance("example_plugin",
                               vec![], temp_dir.clone(), 42)
        .await
        .expect_err("expected start_plugin_instance to fail for disabled plugin");

    let msg = format!("{err:?}");
    assert!(
        msg.contains("disabled"),
        "error should mention disabled plugin, got: {msg}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}