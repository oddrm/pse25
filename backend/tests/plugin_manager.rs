mod common;

use diesel::prelude::*;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs;
use std::str::FromStr;
use chrono::Utc;
use backend::plugin_manager::manager::PluginManager;
use backend::plugin_manager::plugin::{Plugin, Trigger};
use backend::storage::storage_manager::StorageManager;


fn unique_temp_plugins_dir(dir_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("pse25_{nanos}_{dir_name}"))
}

fn write_minimal_python_plugin(dir: &PathBuf, file_name: &str, extra_constants: &str) -> PathBuf {
    let path = dir.join(file_name);

    // Minimal plugin: satisfies python_bridge::validate_plugin_module:
    // - PluginImpl exists and is callable (class)
    // - PluginImpl has a callable run() method
    //
    // Constants are optional; if missing, PluginManager falls back to filename stem as name.
    let content = format!(
        r#"
{extra_constants}

class PluginImpl:
    def __init__(self, path: str):
        self.path = path

    def run(self, data: str) -> str:
        return "ok"
"#,
    );

    fs::write(&path, content).expect("failed to write minimal python plugin file");
    path
}


#[test]
fn register_plugins_called_twice_on_same_directory_returns_error_and_does_not_duplicate() {
    // Goal:
    // - Calling register_plugins(dir) twice should not duplicate plugins.
    // - Second call should return Err due to duplicate path prevention.
    // - Registered plugin count must remain stable.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    let dir = unique_temp_plugins_dir("register_plugins_twice");
    fs::create_dir_all(&dir).expect("failed to create temp plugins dir");

    let _p1 = write_minimal_python_plugin(
        &dir,
        "dup_a.py",
        r#"PLUGIN_NAME = "dup_a"
PLUGIN_DESCRIPTION = "dup a"
PLUGIN_TRIGGER = "manual"
"#,
    );
    let _p2 = write_minimal_python_plugin(
        &dir,
        "dup_b.py",
        r#"PLUGIN_NAME = "dup_b"
PLUGIN_DESCRIPTION = "dup b"
PLUGIN_TRIGGER = "manual"
"#,
    );

    pm.register_plugins(dir.clone())
        .expect("first register_plugins should succeed");

    let count_after_first = pm.get_registered_plugins().len();
    assert_eq!(count_after_first, 2, "expected two plugins after first registration");

    let err = pm
        .register_plugins(dir.clone())
        .expect_err("second register_plugins should fail due to duplicate registration");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("already registered"),
        "error should mention already registered, got: {msg}"
    );

    let count_after_second = pm.get_registered_plugins().len();
    assert_eq!(
        count_after_second, count_after_first,
        "plugin count must not increase on duplicate registration attempt"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_config_and_apply_is_idempotent_for_same_config() {
    // Goal:
    // - Applying the same config twice should produce the same final state without errors.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    let yaml = r#"
plugins:
  - name: example_plugin
    enabled: false
"#;
    let cfg_path = common::create_yaml_config(yaml);

    pm.load_config_and_apply(cfg_path.to_string_lossy().as_ref())
        .expect("first load_config_and_apply failed");

    pm.load_config_and_apply(cfg_path.to_string_lossy().as_ref())
        .expect("second load_config_and_apply should also succeed");

    let _ = fs::remove_file(&cfg_path);

    let p = pm
        .get_registered_plugins()
        .into_iter()
        .find(|p| p.name().as_str() == "example_plugin")
        .expect("example_plugin should exist");
    assert!(!p.enabled(), "example_plugin should remain disabled after applying config twice");
}

#[test]
fn register_plugins_registers_all_plugins_in_directory() {
    // Goal:
    // - PluginManager::register_plugins(directory) iterates directory entries
    // - registers every plugin file in that directory
    // - and makes them visible via get_registered_plugins().

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    // Arrange: temp plugin directory with 2 valid plugins
    let dir = unique_temp_plugins_dir("register_plugins");
    fs::create_dir_all(&dir).expect("failed to create temp plugins dir");

    // Use unique module names to avoid Python import caching collisions.
    let _p1 = write_minimal_python_plugin(
        &dir,
        "test_plugin_one.py",
        r#"PLUGIN_NAME = "plugin_one"
PLUGIN_DESCRIPTION = "test plugin one"
PLUGIN_TRIGGER = "manual"
"#,
    );
    let _p2 = write_minimal_python_plugin(
        &dir,
        "test_plugin_two.py",
        r#"PLUGIN_NAME = "plugin_two"
PLUGIN_DESCRIPTION = "test plugin two"
PLUGIN_TRIGGER = "manual"
"#,
    );

    // Act
    pm.register_plugins(dir.clone())
        .expect("register_plugins failed");

    // Assert
    let registered = pm.get_registered_plugins();
    assert_eq!(registered.len(), 2, "should register exactly 2 plugins");

    let names: Vec<&str> = registered.iter().map(|p| p.name().as_str()).collect();
    assert!(
        names.contains(&"plugin_one"),
        "plugin_one should be registered, got: {names:?}"
    );
    assert!(
        names.contains(&"plugin_two"),
        "plugin_two should be registered, got: {names:?}"
    );

    // Cleanup (best-effort)
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_config_and_apply_is_partially_applied_before_error_is_returned() {
    // Goal:
    // - Document current behavior:
    //   If config has multiple entries and one is unknown,
    //   the function returns Err, but earlier entries may already have been applied.

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    // Arrange: first entry exists, second one does not
    let yaml = r#"
plugins:
  - name: example_plugin
    enabled: false
  - name: does_not_exist
    enabled: true
"#;
    let config_path = common::create_yaml_config(yaml);

    // Act: should error due to does_not_exist
    let err = pm
        .load_config_and_apply(config_path.to_string_lossy().as_ref())
        .expect_err("expected error because does_not_exist is not registered");

    let _ = fs::remove_file(&config_path);

    // Assert: error mentions not found
    assert!(
        format!("{err:?}").contains("not found"),
        "error should mention not found"
    );

    // Assert: example_plugin was already applied (disabled)
    let p = pm
        .get_registered_plugins()
        .into_iter()
        .find(|p| p.name().as_str() == "example_plugin")
        .expect("example_plugin should still be registered");
    assert!(
        !p.enabled(),
        "example_plugin should already be disabled (partial application behavior)"
    );
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

    common::init_test_logging();

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
    let config_path = common::create_yaml_config(yaml);

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
    let config_path = common::create_yaml_config(yaml);

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
        common::unique_temp_file_path("this_file_should_not_exist.yaml");
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
    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    // Arrange: PluginManager + Plugin registrieren
    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    // temp dir ist aktuell zwar ungenutzt, aber API verlangt ihn
    let temp_dir = unique_temp_plugins_dir("plugin_instance_lifecycle");
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
    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    pm.disable_plugin("example_plugin")
        .expect("disable_plugin failed");

    let temp_dir = unique_temp_plugins_dir("plugin_disabled_start");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let err = pm
        .start_plugin_instance("example_plugin",
                               vec![], temp_dir.clone(),
                               42)
        .await
        .expect_err("expected start_plugin_instance to fail for disabled plugin");

    let msg = format!("{err:?}");
    assert!(
        msg.contains("disabled"),
        "error should mention disabled plugin, got: {msg}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn start_fails_for_unknown_plugin_name() {
    // Goal:
    // - starting a plugin that is not registered must return Err.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    let temp_dir = unique_temp_plugins_dir("start_unknown_plugin");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let err = pm
        .start_plugin_instance("does_not_exist", vec![],
                               temp_dir.clone(), 100)
        .await
        .expect_err("expected error when starting unknown plugin");

    let msg = format!("{err:?}");
    assert!(
        msg.contains("not registered") || msg.contains("registered"),
        "error should mention not registered, got: {msg}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn start_fails_for_duplicate_instance_id() {
    // Goal:
    // - starting twice with the same instance_id must fail on the second call.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    let temp_dir = unique_temp_plugins_dir("duplicate_instance_id");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let instance_id = 777_u64;

    // First start should succeed
    pm.start_plugin_instance("example_plugin", vec![],
                             temp_dir.clone(), instance_id)
        .await
        .expect("first start should succeed");

    // Second start with same instance_id should fail
    let err = pm
        .start_plugin_instance("example_plugin", vec![],
                               temp_dir.clone(), instance_id)
        .await
        .expect_err("expected error on duplicate instance_id");

    let msg = format!("{err:?}");
    assert!(
        msg.contains("already running"),
        "error should mention already running, got: {msg}"
    );

    // Cleanup: stop instance to avoid orphan process
    pm.stop_plugin_instance(instance_id)
        .await
        .expect("stop after duplicate-instance-id test failed");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn stop_pause_resume_fail_for_non_running_instance() {
    // Goal:
    // - stop/pause/resume on an instance_id that is not running must return Err.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    let err = pm
        .stop_plugin_instance(9999)
        .await
        .expect_err("expected error on stop for non-running instance");
    assert!(
        format!("{err:?}").contains("not running"),
        "stop error should mention not running"
    );

    let err = pm
        .pause_plugin_instance(9999)
        .await
        .expect_err("expected error on pause for non-running instance");
    assert!(
        format!("{err:?}").contains("not running"),
        "pause error should mention not running"
    );

    let err = pm
        .resume_plugin_instance(9999)
        .await
        .expect_err("expected error on resume for non-running instance");
    assert!(
        format!("{err:?}").contains("not running"),
        "resume error should mention not running"
    );
}

#[tokio::test]
async fn pause_and_resume_are_idempotent() {
    // Goal:
    // - pause twice should be Ok both times
    // - resume twice should be Ok both times
    // - and state transitions match get_running_instances() filtering behavior.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    let temp_dir = unique_temp_plugins_dir("pause_resume_idempotent");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let instance_id = 778_u64;

    pm.start_plugin_instance("example_plugin",
                             vec![], temp_dir.clone(), instance_id)
        .await
        .expect("start_plugin_instance failed");

    // pause twice
    pm.pause_plugin_instance(instance_id)
        .await
        .expect("first pause failed");
    pm.pause_plugin_instance(instance_id)
        .await
        .expect("second pause should be idempotent");

    assert_eq!(
        pm.get_running_instances().len(),
        0,
        "paused instances are filtered out by get_running_instances()"
    );

    // resume twice
    pm.resume_plugin_instance(instance_id)
        .await
        .expect("first resume failed");
    pm.resume_plugin_instance(instance_id)
        .await
        .expect("second resume should be idempotent");

    assert_eq!(
        pm.get_running_instances().len(),
        1,
        "after resume, instance should be reported as running"
    );

    pm.stop_plugin_instance(instance_id)
        .await
        .expect("stop_plugin_instance failed");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn plugin_manager_load_config_and_apply_errors_on_invalid_yaml() {
    // Goal:
    // - invalid YAML must return Err with parse prefix.

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    let yaml = "plugins: [ this is : not valid yaml";
    let config_path = common::create_yaml_config(yaml);

    let err = pm
        .load_config_and_apply(config_path.to_string_lossy().as_ref())
        .expect_err("expected error for invalid YAML");

    let _ = fs::remove_file(&config_path);

    assert!(
        format!("{err:?}").contains("Failed to parse config"),
        "should fail with 'Failed to parse config' prefix"
    );
}

#[test]
fn register_plugin_prevents_duplicates_even_with_different_path_forms() {
    // Goal:
    // - Duplicate registration should be prevented even if the same file is passed
    //   once as relative path and once as canonical/absolute path.
    //
    // This verifies the canonicalize-based dedup logic.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    let rel = PathBuf::from("src/plugin_manager/plugins/plugin.py");
    pm.register_plugin(rel.clone())
        .expect("first register_plugin(relative) should succeed");

    let abs = rel
        .canonicalize()
        .expect("canonicalize should work for existing plugin file");

    let err = pm
        .register_plugin(abs)
        .expect_err("second register_plugin(absolute) should fail due to duplicate");

    let msg = format!("{err:?}");
    assert!(
        msg.contains("already registered"),
        "error should mention already registered, got: {msg}"
    );

    assert_eq!(
        pm.get_registered_plugins().len(),
        1,
        "duplicate registration must not create a second entry"
    );
}

#[test]
fn load_config_and_apply_can_reenable_plugin_after_disabling() {
    // Goal:
    // - load_config_and_apply can both disable and re-enable a plugin across runs.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    // Step 1: disable via config
    let yaml_disable = r#"
plugins:
  - name: example_plugin
    enabled: false
"#;
    let cfg_disable = common::create_yaml_config(yaml_disable);
    pm.load_config_and_apply(cfg_disable.to_string_lossy().as_ref())
        .expect("disabling config should apply");
    let _ = fs::remove_file(&cfg_disable);

    let p = pm
        .get_registered_plugins()
        .into_iter()
        .find(|p| p.name().as_str() == "example_plugin")
        .expect("example_plugin should exist");
    assert!(!p.enabled(), "plugin should be disabled after disable-config");

    // Step 2: re-enable via config
    let yaml_enable = r#"
plugins:
  - name: example_plugin
    enabled: true
"#;
    let cfg_enable = common::create_yaml_config(yaml_enable);
    pm.load_config_and_apply(cfg_enable.to_string_lossy().as_ref())
        .expect("enabling config should apply");
    let _ = fs::remove_file(&cfg_enable);

    let p = pm
        .get_registered_plugins()
        .into_iter()
        .find(|p| p.name().as_str() == "example_plugin")
        .expect("example_plugin should exist");
    assert!(p.enabled(), "plugin should be enabled again after enable-config");
}

#[tokio::test]
async fn plugin_manager_two_instances_run_independently() {
    // Goal:
    // - Two instances with different IDs can run in parallel.
    // - Stopping one does not affect the other.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    let temp_dir = unique_temp_plugins_dir("two_instances_parallel");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let id1 = 1001_u64;
    let id2 = 1002_u64;

    pm.start_plugin_instance("example_plugin", vec![],
                             temp_dir.clone(), id1)
        .await
        .expect("start instance 1 failed");
    pm.start_plugin_instance("example_plugin", vec![],
                             temp_dir.clone(), id2)
        .await
        .expect("start instance 2 failed");

    let running = pm.get_running_instances();
    assert_eq!(running.len(), 2, "both instances should be running");

    pm.stop_plugin_instance(id1)
        .await
        .expect("stop instance 1 failed");

    let running = pm.get_running_instances();
    assert_eq!(running.len(), 1, "instance 2 should still be running");
    assert_eq!(running[0].1, id2, "remaining running instance should be id2");

    pm.stop_plugin_instance(id2)
        .await
        .expect("stop instance 2 failed");

    assert_eq!(
        pm.get_running_instances().len(),
        0,
        "after stopping both, none should be running"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn register_plugin_twice_returns_error_and_does_not_duplicate() {
    // Goal:
    // - registering the same plugin file twice should be prevented
    // - second call must return Err
    // - and plugin list length must stay 1

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    let plugin_path = PathBuf::from("src/plugin_manager/plugins/plugin.py");

    pm.register_plugin(plugin_path.clone())
        .expect("first register_plugin should succeed");

    let err = pm
        .register_plugin(plugin_path)
        .expect_err("second register_plugin should fail due to duplicate registration");

    let msg = format!("{err:?}");
    assert!(
        msg.contains("already registered"),
        "error should mention already registered, got: {msg}"
    );

    assert_eq!(
        pm.get_registered_plugins().len(),
        1,
        "duplicate registration must not add another entry"
    );
}

#[tokio::test]
async fn plugin_manager_pause_then_stop_works() {
    // Goal:
    // - Stopping a paused instance should still succeed and clean up the process.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    let temp_dir = unique_temp_plugins_dir("pause_then_stop");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let id = 2001_u64;

    pm.start_plugin_instance("example_plugin", vec![],
                             temp_dir.clone(), id)
        .await
        .expect("start failed");

    pm.pause_plugin_instance(id)
        .await
        .expect("pause failed");

    // paused instances are filtered out by get_running_instances()
    assert_eq!(pm.get_running_instances().len(), 0);

    pm.stop_plugin_instance(id)
        .await
        .expect("stop failed (pause -> stop should work)");

    assert_eq!(pm.get_running_instances().len(), 0);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn register_plugins_skips_non_py_files_and_directories() {
    // Goal:
    // - register_plugins should ignore non-.py files and directories
    // - and still succeed when directory contains "noise".

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    let dir = unique_temp_plugins_dir("register_plugins_skip_noise");
    fs::create_dir_all(&dir).expect("failed to create temp plugins dir");

    // valid plugin
    let _p = write_minimal_python_plugin(
        &dir,
        "only_this_one.py",
        r#"PLUGIN_NAME = "only_this_one"
PLUGIN_DESCRIPTION = "valid plugin"
PLUGIN_TRIGGER = "manual"
"#,
    );

    // noise file
    fs::write(dir.join("README.txt"), "hello").expect("failed to write noise file");

    // noise directory
    fs::create_dir_all(dir.join("subdir")).expect("failed to create noise dir");

    pm.register_plugins(dir.clone())
        .expect("register_plugins should succeed even with noise");

    let registered = pm.get_registered_plugins();
    assert_eq!(registered.len(), 1, "should register exactly one *.py plugin");
    assert_eq!(registered[0].name().as_str(), "only_this_one");

    let _ = fs::remove_dir_all(&dir);
}

fn write_minimal_python_plugin_without_constants(dir: &PathBuf, file_name: &str) -> PathBuf {
    let path = dir.join(file_name);

    // Minimal plugin that is VALID for validation, but provides NO PLUGIN_* constants.
    // Expected behavior:
    // - name falls back to filename stem
    // - description falls back to "Plugin loaded from ..."
    // - trigger defaults to Manual
    let content = r#"
class PluginImpl:
    def __init__(self, path: str):
        self.path = path

    def run(self, data: str) -> str:
        return "ok"
"#;

    fs::write(&path, content).expect("failed to write minimal python plugin file (no constants)");
    path
}

#[tokio::test]
async fn stop_is_not_idempotent_and_resume_after_stop_fails() {
    // Goal:
    // - Start -> Stop succeeds
    // - Stop again returns Err (not running)
    // - Resume after stop returns Err (not running)
    //
    // This catches "double click" / race conditions typical for UI clients.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    let temp_dir = unique_temp_plugins_dir("stop_twice_resume_after_stop");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let id = 3001_u64;

    pm.start_plugin_instance("example_plugin", vec![],
                             temp_dir.clone(), id)
        .await
        .expect("start failed");

    pm.stop_plugin_instance(id).await.expect("stop failed");

    let err = pm
        .stop_plugin_instance(id)
        .await
        .expect_err("second stop must fail (instance not running)");
    assert!(
        format!("{err:?}").contains("not running"),
        "second stop error should mention not running, got: {err:?}"
    );

    let err = pm
        .resume_plugin_instance(id)
        .await
        .expect_err("resume after stop must fail (instance not running)");
    assert!(
        format!("{err:?}").contains("not running"),
        "resume-after-stop error should mention not running, got: {err:?}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn pause_only_affects_target_instance() {
    // Goal:
    // - With two running instances, pausing one should NOT pause the other.
    // - get_running_instances() should still report the other instance.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin failed");

    let temp_dir = unique_temp_plugins_dir("pause_only_one_instance");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let id1 = 4001_u64;
    let id2 = 4002_u64;

    pm.start_plugin_instance("example_plugin", vec![],
                             temp_dir.clone(), id1)
        .await
        .expect("start id1 failed");
    pm.start_plugin_instance("example_plugin", vec![],
                             temp_dir.clone(), id2)
        .await
        .expect("start id2 failed");

    assert_eq!(pm.get_running_instances().len(), 2);

    pm.pause_plugin_instance(id1)
        .await
        .expect("pause id1 failed");

    // Only id2 should still be reported as running (paused instances are filtered out)
    let running = pm.get_running_instances();
    assert_eq!(running.len(), 1, "only one instance should remain running");
    assert_eq!(running[0].1, id2, "id2 should still be running");

    pm.resume_plugin_instance(id1)
        .await
        .expect("resume id1 failed");

    assert_eq!(
        pm.get_running_instances().len(),
        2,
        "after resuming id1, both should be running"
    );

    pm.stop_plugin_instance(id1).await.expect("stop id1 failed");
    pm.stop_plugin_instance(id2).await.expect("stop id2 failed");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn register_plugin_uses_fallbacks_when_python_constants_are_missing() {
    // Goal:
    // - If PLUGIN_NAME/DESCRIPTION/TRIGGER are missing in python module:
    //   - name should fall back to filename stem
    //   - trigger should default to Manual
    //
    // This makes "minimal plugins" easier and documents the contract.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    let dir = unique_temp_plugins_dir("plugin_missing_constants");
    fs::create_dir_all(&dir).expect("failed to create temp plugins dir");

    let file_name = "no_constants_plugin.py";
    let path = write_minimal_python_plugin_without_constants(&dir, file_name);

    pm.register_plugin(path).expect("register_plugin failed");

    let registered = pm.get_registered_plugins();
    assert_eq!(registered.len(), 1);

    assert_eq!(
        registered[0].name().as_str(),
        "no_constants_plugin",
        "name should fall back to filename stem when PLUGIN_NAME is missing"
    );
    assert!(
        matches!(registered[0].trigger(), Trigger::Manual),
        "trigger should default to Manual when PLUGIN_TRIGGER is missing"
    );

    let _ = fs::remove_dir_all(&dir);
}

fn write_minimal_python_plugin_with_trigger(
    dir: &PathBuf,
    file_name: &str,
    plugin_name: &str,
    plugin_trigger: &str,
) -> PathBuf {
    let path = dir.join(file_name);

    // Valid plugin + explicit constants so we can test trigger mapping deterministically.
    let content = format!(
        r#"
PLUGIN_NAME = "{plugin_name}"
PLUGIN_DESCRIPTION = "test plugin"
PLUGIN_TRIGGER = "{plugin_trigger}"

class PluginImpl:
    def __init__(self, path: str):
        self.path = path

    def run(self, data: str) -> str:
        return "ok"
"#,
    );

    fs::write(&path, content).expect("failed to write python plugin file (with trigger)");
    path
}

fn write_invalid_python_plugin_missing_plugin_impl(dir: &PathBuf, file_name: &str) -> PathBuf {
    let path = dir.join(file_name);

    // No PluginImpl -> should fail validation in register_plugin().
    let content = r#"
PLUGIN_NAME = "invalid_missing_plugin_impl"

def something_else():
    return "nope"
"#;

    fs::write(&path, content).expect("failed to write invalid python plugin file");
    path
}

fn write_invalid_python_plugin_missing_run(dir: &PathBuf, file_name: &str) -> PathBuf {
    let path = dir.join(file_name);

    // PluginImpl exists, but no run() -> should fail validation in register_plugin().
    let content = r#"
PLUGIN_NAME = "invalid_missing_run"

class PluginImpl:
    def __init__(self, path: str):
        self.path = path
"#;

    fs::write(&path, content).expect("failed to write invalid python plugin file");
    path
}

#[test]
fn register_plugin_maps_all_supported_triggers_and_fallbacks() {
    // Goal:
    // - Verify parse_trigger mapping via register_plugin() + python constants:
    //   on_entry_create/update/delete, on_schedule:<pattern>, manual, and unknown -> Manual fallback.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    let dir = unique_temp_plugins_dir("trigger_mapping");
    fs::create_dir_all(&dir).expect("failed to create temp plugins dir");

    let _p1 = write_minimal_python_plugin_with_trigger(
        &dir,
        "t_create.py",
        "t_create",
        "on_entry_create",
    );
    let _p2 = write_minimal_python_plugin_with_trigger(
        &dir,
        "t_update.py",
        "t_update",
        "on_entry_update",
    );
    let _p3 = write_minimal_python_plugin_with_trigger(
        &dir,
        "t_delete.py",
        "t_delete",
        "on_entry_delete",
    );
    let _p4 = write_minimal_python_plugin_with_trigger(
        &dir,
        "t_schedule.py",
        "t_schedule",
        "on_schedule: */5 * * * *",
    );
    let _p5 = write_minimal_python_plugin_with_trigger(
        &dir,
        "t_manual.py",
        "t_manual",
        "manual",
    );
    let _p6 = write_minimal_python_plugin_with_trigger(
        &dir,
        "t_unknown.py",
        "t_unknown",
        "some_unknown_trigger",
    );

    pm.register_plugins(dir.clone())
        .expect("register_plugins failed (trigger mapping setup)");

    let reg = pm.get_registered_plugins();

    let find = |name: &str| {
        reg.iter()
            .find(|p| p.name().as_str() == name)
            .unwrap_or_else(|| panic!("plugin '{name}' not found among registered"))
    };

    assert!(
        matches!(find("t_create").trigger(), Trigger::OnEntryCreate),
        "on_entry_create should map to Trigger::OnEntryCreate"
    );
    assert!(
        matches!(find("t_update").trigger(), Trigger::OnEntryUpdate),
        "on_entry_update should map to Trigger::OnEntryUpdate"
    );
    assert!(
        matches!(find("t_delete").trigger(), Trigger::OnEntryDelete),
        "on_entry_delete should map to Trigger::OnEntryDelete"
    );

    match find("t_schedule").trigger() {
        Trigger::OnSchedule(schedule) => {
            let next = schedule
                .upcoming(Utc)
                .next()
                .expect("schedule should yield at least one datetime");

            // grobe Plausibilitätsprüfung
            assert!(next > Utc::now());
        }
        other => panic!("expected Trigger::OnSchedule(..), got: {other:?}"),
    }

    assert!(
        matches!(find("t_manual").trigger(), Trigger::Manual),
        "manual should map to Trigger::Manual"
    );
    assert!(
        matches!(find("t_unknown").trigger(), Trigger::Manual),
        "unknown trigger should fall back to Manual"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn register_plugin_fails_validation_when_plugin_impl_missing_or_run_missing() {
    // Goal:
    // - register_plugin must return Err for hard validation failures:
    //   - PluginImpl missing
    //   - PluginImpl.run missing

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    let dir = unique_temp_plugins_dir("validation_errors");
    fs::create_dir_all(&dir).expect("failed to create temp plugins dir");

    let p_missing_impl = write_invalid_python_plugin_missing_plugin_impl(&dir,
                                                                         "missing_impl.py");
    let err = pm
        .register_plugin(p_missing_impl)
        .expect_err("expected error for missing PluginImpl");
    assert!(
        format!("{err:?}").contains("PluginImpl"),
        "error should mention PluginImpl, got: {err:?}"
    );

    let p_missing_run = write_invalid_python_plugin_missing_run(&dir, "missing_run.py");
    let err = pm
        .register_plugin(p_missing_run)
        .expect_err("expected error for PluginImpl without run()");
    assert!(
        format!("{err:?}").contains("run"),
        "error should mention run(), got: {err:?}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_config_and_apply_applies_multiple_known_plugins() {
    // Goal:
    // - A config file containing multiple known plugins should apply all toggles.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    // Arrange: register two plugins from a temp directory
    let dir = unique_temp_plugins_dir("config_multiple_known");
    fs::create_dir_all(&dir).expect("failed to create temp plugins dir");

    let _p1 = write_minimal_python_plugin_with_trigger(&dir, "a.py",
                                                       "plugin_a_cfg", "manual");
    let _p2 = write_minimal_python_plugin_with_trigger(&dir, "b.py",
                                                       "plugin_b_cfg", "manual");

    pm.register_plugins(dir.clone())
        .expect("register_plugins failed");

    // sanity: both are enabled by default
    let mut enabled_before: Vec<(&str, bool)> = pm
        .get_registered_plugins()
        .into_iter()
        .map(|p| (p.name().as_str(), p.enabled()))
        .collect();
    enabled_before.sort_by_key(|(n, _)| *n);
    assert!(
        enabled_before.iter().any(|(n, e)| *n == "plugin_a_cfg" && *e),
        "plugin_a_cfg should be enabled by default"
    );
    assert!(
        enabled_before.iter().any(|(n, e)| *n == "plugin_b_cfg" && *e),
        "plugin_b_cfg should be enabled by default"
    );

    // Act: config toggles both
    let yaml = r#"
plugins:
  - name: plugin_a_cfg
    enabled: false
  - name: plugin_b_cfg
    enabled: false
"#;
    let cfg_path = common::create_yaml_config(yaml);

    pm.load_config_and_apply(cfg_path.to_string_lossy().as_ref())
        .expect("load_config_and_apply failed for multi-plugin config");

    let _ = fs::remove_file(&cfg_path);
    let _ = fs::remove_dir_all(&dir);

    // Assert: both are disabled
    let reg = pm.get_registered_plugins();
    let a = reg
        .iter()
        .find(|p| p.name().as_str() == "plugin_a_cfg")
        .expect("plugin_a_cfg missing after registration");
    let b = reg
        .iter()
        .find(|p| p.name().as_str() == "plugin_b_cfg")
        .expect("plugin_b_cfg missing after registration");

    assert!(!a.enabled(), "plugin_a_cfg should be disabled by config");
    assert!(!b.enabled(), "plugin_b_cfg should be disabled by config");
}

#[test]
fn mixed_registration_directory_then_single_plugin_results_in_combined_set() {
    // Goal:
    // - register_plugins(dir) and register_plugin(single) can be mixed.
    // - all plugins are present exactly once (no accidental overwrite).
    //
    // We register 2 temp plugins from a directory, then additionally register the built-in
    // example plugin.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    // 1) Register two plugins via directory
    let dir = unique_temp_plugins_dir("mixed_registration");
    fs::create_dir_all(&dir).expect("failed to create temp plugins dir");

    let _p1 = write_minimal_python_plugin(
        &dir,
        "mixed_one.py",
        r#"PLUGIN_NAME = "mixed_one"
PLUGIN_DESCRIPTION = "mixed one"
PLUGIN_TRIGGER = "manual"
"#,
    );
    let _p2 = write_minimal_python_plugin(
        &dir,
        "mixed_two.py",
        r#"PLUGIN_NAME = "mixed_two"
PLUGIN_DESCRIPTION = "mixed two"
PLUGIN_TRIGGER = "manual"
"#,
    );

    pm.register_plugins(dir.clone())
        .expect("register_plugins(dir) failed");

    // 2) Then register the built-in example plugin individually
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin(example) failed");

    // Assert: we have 3 distinct plugins by name
    let names: Vec<String> = pm
        .get_registered_plugins()
        .into_iter()
        .map(|p| p.name().clone())
        .collect();

    assert!(
        names.iter().any(|n| n == "mixed_one"),
        "mixed_one should be registered, got: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "mixed_two"),
        "mixed_two should be registered, got: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "example_plugin"),
        "example_plugin should be registered, got: {names:?}"
    );

    assert_eq!(
        pm.get_registered_plugins().len(),
        3,
        "expected exactly 3 plugins total (2 from dir + 1 single)"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn start_two_different_plugins_reports_correct_running_pairs() {
    // Goal:
    // - Register multiple plugins.
    // - Start two instances of *different* plugins.
    // - get_running_instances() must return correct (&Plugin, InstanceID) pairs.
    //
    // This specifically validates that RunningInstance.plugin_index maps to the correct plugin.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    // Arrange: temp dir with one minimal plugin + built-in example plugin
    let dir = unique_temp_plugins_dir("two_different_plugins");
    fs::create_dir_all(&dir).expect("failed to create temp plugins dir");

    let _p = write_minimal_python_plugin(
        &dir,
        "other_plugin.py",
        r#"PLUGIN_NAME = "other_plugin"
PLUGIN_DESCRIPTION = "other"
PLUGIN_TRIGGER = "manual"
"#,
    );

    pm.register_plugins(dir.clone())
        .expect("register_plugins failed");
    pm.register_plugin(PathBuf::from("src/plugin_manager/plugins/plugin.py"))
        .expect("register_plugin(example) failed");

    // Act: start one instance per plugin
    let temp_dir = unique_temp_plugins_dir("two_different_plugins_instances");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let id1 = 9101_u64;
    let id2 = 9102_u64;

    pm.start_plugin_instance("other_plugin",
                             vec![], temp_dir.clone(), id1)
        .await
        .expect("start other_plugin failed");
    pm.start_plugin_instance("example_plugin",
                             vec![], temp_dir.clone(), id2)
        .await
        .expect("start example_plugin failed");

    // Assert: running contains both and maps IDs to correct names
    let mut running: Vec<(String, u64)> = pm
        .get_running_instances()
        .into_iter()
        .map(|(p, id)| (p.name().clone(), id))
        .collect();

    running.sort_by_key(|(_, id)| *id);

    assert_eq!(running.len(), 2, "expected exactly two running instances");
    assert_eq!(running[0], ("other_plugin".to_string(), id1));
    assert_eq!(running[1], ("example_plugin".to_string(), id2));

    // Cleanup
    pm.stop_plugin_instance(id1).await.expect("stop id1 failed");
    pm.stop_plugin_instance(id2).await.expect("stop id2 failed");

    let _ = fs::remove_dir_all(&temp_dir);
    let _ = fs::remove_dir_all(&dir);
}

fn write_python_plugin_that_ignores_stop(dir: &PathBuf, file_name: &str,
                                         plugin_name: &str) -> PathBuf {
    // This plugin ACKs stop(), but run() never ends -> forces PluginManager stop_plugin_instance()
    // into the kill path after the soft-stop wait timeout.
    let path = dir.join(file_name);

    let content = format!(
        r#"
PLUGIN_NAME = "{plugin_name}"
PLUGIN_DESCRIPTION = "ignores stop; used for kill-path test"
PLUGIN_TRIGGER = "manual"

import time

class PluginImpl:
    def __init__(self, path: str):
        self.path = path

    def run(self, data: str) -> str:
        # Never returns, never checks a stop flag.
        while True:
            time.sleep(0.1)

    def stop(self) -> str:
        # Pretend we are stopping, but do not affect run().
        return "stopping"

    def pause(self) -> str:
        return "paused"

    def resume(self) -> str:
        return "resumed"
"#,
    );

    fs::write(&path, content).expect("failed to write ignore-stop python plugin");
    path
}

fn write_python_plugin_with_slow_pause(dir: &PathBuf, file_name: &str,
                                       plugin_name: &str) -> PathBuf {
    // This plugin makes pause() block longer than TIMEOUT_PAUSE_ACK (2s),
    // so PluginManager::pause_plugin_instance should time out.
    let path = dir.join(file_name);

    let content = format!(
        r#"
PLUGIN_NAME = "{plugin_name}"
PLUGIN_DESCRIPTION = "slow pause; used for timeout test"
PLUGIN_TRIGGER = "manual"

import time
import threading

class PluginImpl:
    def __init__(self, path: str):
        self.path = path
        self._stop = threading.Event()

    def run(self, data: str) -> str:
        while not self._stop.is_set():
            time.sleep(0.1)
        return "stopped"

    def stop(self) -> str:
        self._stop.set()
        return "stopping"

    def pause(self) -> str:
        # Block longer than the manager's TIMEOUT_PAUSE_ACK (2s)
        time.sleep(5)
        return "paused"

    def resume(self) -> str:
        return "resumed"
"#,
    );

    fs::write(&path, content).expect("failed to write slow-pause python plugin");
    path
}

#[tokio::test]
async fn start_actually_executes_run_function() {
    // Goal:
    // - Prove that the python process starts AND PluginImpl.run() is executed.
    // - We detect this via a marker file written by run().

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    let dir = unique_temp_plugins_dir("run_marker_plugin");
    fs::create_dir_all(&dir).expect("failed to create temp plugins dir");

    let plugin_path =
        write_python_plugin_that_writes_marker_and_exits(&dir, "marker.py",
                                                         "marker_plugin");
    let marker_path = plugin_path.with_extension("ran");

    // Ensure clean start
    let _ = fs::remove_file(&marker_path);

    pm.register_plugin(plugin_path.clone())
        .expect("register_plugin failed");

    // Start instance
    let temp_dir = unique_temp_plugins_dir("run_marker_instance");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let id = 9910_u64;
    pm.start_plugin_instance("marker_plugin", vec![],
                             temp_dir.clone(), id)
        .await
        .expect("start marker_plugin failed");

    // Wait briefly for run() to execute and write the marker
    // (run() returns immediately after writing, but scheduling is async)
    for _ in 0..30 {
        if marker_path.exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    assert!(
        marker_path.exists(),
        "marker file should exist => proves PluginImpl.run() executed"
    );

    // Cleanup: even if run already exited, stop should cleanly remove instance/process
    pm.stop_plugin_instance(id).await.expect("stop failed");

    let _ = fs::remove_dir_all(&temp_dir);
    let _ = fs::remove_dir_all(&dir);
}


fn write_python_plugin_that_writes_marker_and_exits(
    dir: &PathBuf,
    file_name: &str,
    plugin_name: &str,
) -> PathBuf {
    let path = dir.join(file_name);

    // run() writes a marker file next to the plugin file and then exits.
    // This gives us a deterministic proof that run() executed.
    let content = format!(
        r#"
PLUGIN_NAME = "{plugin_name}"
PLUGIN_DESCRIPTION = "writes marker and exits"
PLUGIN_TRIGGER = "manual"

from pathlib import Path

class PluginImpl:
    def __init__(self, path: str):
        self.path = Path(path)

    def run(self, data: str) -> str:
        marker = self.path.with_suffix(".ran")
        marker.write_text("ran", encoding="utf-8")
        return "done"

    def stop(self) -> str:
        return "stopping"

    def pause(self) -> str:
        return "paused"

    def resume(self) -> str:
        return "resumed"
"#,
    );

    fs::write(&path, content).expect("failed to write marker python plugin");
    path
}

#[tokio::test]
async fn stop_kills_runner_when_soft_stop_does_not_exit() {
    // Goal:
    // - stop_plugin_instance should not hang if a plugin ignores stop and never exits.
    // - It should go through the "soft stop ack ok, but still running => kill" path and return Ok.

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    // Arrange: register special plugin
    let dir = unique_temp_plugins_dir("kill_path_plugin");
    fs::create_dir_all(&dir).expect("failed to create temp plugins dir");

    let plugin_path = write_python_plugin_that_ignores_stop(&dir, "ignore_stop.py",
                                                            "ignore_stop");
    pm.register_plugin(plugin_path).expect("register_plugin failed");

    let temp_dir = unique_temp_plugins_dir("kill_path_instance");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let id = 9901_u64;

    // Act: start then stop (should force kill internally)
    pm.start_plugin_instance("ignore_stop", vec![], temp_dir.clone(), id)
        .await
        .expect("start ignore_stop failed");

    pm.stop_plugin_instance(id)
        .await
        .expect("stop should succeed even if it had to kill the process");

    // Assert: no running instances
    assert_eq!(pm.get_running_instances().len(), 0);

    let _ = fs::remove_dir_all(&temp_dir);
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn pause_times_out_when_runner_does_not_ack_in_time() {
    // Goal:
    // - pause_plugin_instance should time out if the runner does not ACK within TIMEOUT_PAUSE_ACK.
    // - IMPORTANT: on timeout, the instance state must remain Running (no partial state change).

    common::init_test_logging();

    let Some(storage_manager) = try_storage_manager_from_env() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    let mut pm = PluginManager::new(storage_manager);

    // Arrange: register plugin with slow pause()
    let dir = unique_temp_plugins_dir("slow_pause_plugin");
    fs::create_dir_all(&dir).expect("failed to create temp plugins dir");

    let plugin_path = write_python_plugin_with_slow_pause(&dir, "slow_pause.py",
                                                          "slow_pause");
    pm.register_plugin(plugin_path).expect("register_plugin failed");

    let temp_dir = unique_temp_plugins_dir("slow_pause_instance");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let id = 9902_u64;
    pm.start_plugin_instance("slow_pause", vec![], temp_dir.clone(),
                             id)
        .await
        .expect("start slow_pause failed");

    // Sanity: running before pause attempt
    assert_eq!(
        pm.get_running_instances().len(),
        1,
        "instance should be running before pause attempt"
    );

    // Act: pause should time out
    let err = pm
        .pause_plugin_instance(id)
        .await
        .expect_err("pause should time out");

    let msg = format!("{err:?}");
    assert!(
        msg.contains("timed out"),
        "error should mention timed out, got: {msg}"
    );

    // Assert: still running (state should NOT become Paused if pause failed)
    let running = pm.get_running_instances();
    assert_eq!(
        running.len(),
        1,
        "after pause timeout, instance must still be reported as running"
    );
    assert_eq!(running[0].1, id, "running instance id should remain the same");

    // Cleanup: stop instance (plugin respects stop)
    pm.stop_plugin_instance(id).await.expect("stop failed");

    let _ = fs::remove_dir_all(&temp_dir);
    let _ = fs::remove_dir_all(&dir);
}