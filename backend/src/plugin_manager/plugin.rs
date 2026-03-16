use cron::Schedule;
use tracing::debug;

/// Beschreibt ein registriertes Plugin mit allen Metadaten,
/// die der Rust-Teil der Anwendung darüber wissen muss.
///
/// Ein `Plugin` ist hier **nicht die laufende Instanz**, sondern
/// die statische Beschreibung eines Plugins:
/// - Name
/// - Beschreibung
/// - Trigger
/// - Dateipfad
/// - Aktivierungs-/Validierungsstatus
#[derive(Debug, Clone)]
pub struct Plugin {
    /// Anzeigename des Plugins.
    name: String,
    /// Freitext-Beschreibung des Plugins.
    description: String,
    /// Legt fest, wann das Plugin automatisch ausgelöst werden soll.
    trigger: Trigger,
    /// Dateipfad zur Python-Datei des Plugins.
    path: std::path::PathBuf,

    /// Gibt an, ob das Plugin vom Benutzer aktiviert wurde.
    enabled: bool,
    /// Gibt an, ob das Plugin technisch gültig/startbar ist.
    valid: bool,
    /// Nicht-kritische Probleme aus der Validierung.
    validation_warnings: Vec<String>,
}

impl Plugin {
    /// Erzeugt ein neues Plugin-Objekt mit Standardwerten:
    /// - `enabled = false` → erst nach Konfiguration aktiv
    /// - `valid = true` → kann später durch Validierung angepasst werden
    pub fn new(
        name: String,
        description: String,
        trigger: Trigger,
        path: std::path::PathBuf,
    ) -> Self {
        debug!("Creating plugin '{}' trigger={}", name, trigger.to_string());

        Self {
            name,
            description,
            trigger,
            path,

            enabled: false,
            valid: true,
            validation_warnings: Vec::new(),
        }
    }

    /// Liefert den Namen des Plugins.
    pub fn name(&self) -> &String {
        &self.name
    }

    /// Liefert die Beschreibung des Plugins.
    pub fn description(&self) -> &String {
        &self.description
    }

    /// Liefert den Trigger des Plugins.
    pub fn trigger(&self) -> &Trigger {
        &self.trigger
    }

    /// Liefert den Dateipfad der Plugin-Datei.
    pub fn path(&self) -> &std::path::PathBuf {
        &self.path
    }

    /// Gibt zurück, ob das Plugin aktiviert ist.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Aktiviert oder deaktiviert das Plugin.
    pub fn set_enabled(&mut self, enabled: bool) {
        debug!("Set enabled={} for plugin '{}'", enabled, self.name);
        self.enabled = enabled;
    }

    /// Gibt zurück, ob das Plugin als gültig/startbar markiert ist.
    pub fn valid(&self) -> bool {
        self.valid
    }

    /// Setzt den Validitätsstatus des Plugins.
    pub fn set_valid(&mut self, valid: bool) {
        debug!("Set valid={} for plugin '{}'", valid, self.name);
        self.valid = valid;
    }

    /// Liefert die gesammelten Warnungen aus der Validierung.
    pub fn validation_warnings(&self) -> &Vec<String> {
        &self.validation_warnings
    }

    /// Ersetzt die bisher gespeicherten Validierungswarnungen.
    pub fn set_validation_warnings(&mut self, warnings: Vec<String>) {
        debug!(
            "Set validation warnings for plugin '{}' => {} warnings",
            self.name,
            warnings.len()
        );
        self.validation_warnings = warnings;
    }
}

/// Vereinfachte Trigger-Art ohne zusätzliche Daten.
/// Wird z. B. verwendet, um Events auf einen gemeinsamen Typ zu mappen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerKind {
    OnEntryCreate,
    OnEntryUpdate,
    OnEntryDelete,
    OnSchedule,
}

/// Ereignisse aus dem Backend, die Plugins auslösen können.
#[derive(Debug, Clone)]
pub enum BackendEvent {
    /// Ein neuer Eintrag wurde erzeugt.
    EntryCreated { path: String },
    /// Ein bestehender Eintrag wurde verändert.
    EntryUpdated { path: String },
    /// Ein Eintrag wurde gelöscht.
    EntryDeleted { path: String },
    /// Zeitgesteuertes Event auf Basis eines Cron-Schedules.
    OnSchedule { schedule: Schedule, path: String },
    /// Manueller Start eines Plugins.
    ///
    /// Wichtig:
    /// Dieses Event liefert absichtlich **keinen** `TriggerKind`,
    /// weil ein manueller Start nicht über die normale Event-Matching-
    /// Logik laufen soll.
    Manual { plugin_name: String },
}

impl BackendEvent {
    /// Ordnet ein konkretes Event einer allgemeinen Trigger-Art zu.
    ///
    /// `Manual` gibt `None` zurück, weil dafür normalerweise ein direkter
    /// Plugin-Start genutzt wird und kein automatisches Trigger-Matching.
    pub fn trigger_kind(&self) -> Option<TriggerKind> {
        match self {
            BackendEvent::EntryCreated { .. } => Some(TriggerKind::OnEntryCreate),
            BackendEvent::EntryUpdated { .. } => Some(TriggerKind::OnEntryUpdate),
            BackendEvent::EntryDeleted { .. } => Some(TriggerKind::OnEntryDelete),
            BackendEvent::OnSchedule { .. } => Some(TriggerKind::OnSchedule),
            BackendEvent::Manual { .. } => None,
        }
    }
}

/// Konkreter Trigger eines Plugins.
///
/// Im Unterschied zu `TriggerKind` kann dieser Typ zusätzliche Daten tragen,
/// z. B. ein `Schedule` bei zeitgesteuerten Plugins.
#[derive(Debug, Clone)]
pub enum Trigger {
    OnEntryCreate,
    OnEntryUpdate,
    OnEntryDelete,
    OnSchedule(Schedule),
    Manual,
}

impl Trigger {
    /// Gibt eine menschenlesbare String-Repräsentation zurück,
    /// z. B. für API-Antworten oder Logs.
    pub fn to_string(&self) -> String {
        match self {
            Trigger::OnEntryCreate => "OnEntryCreate".to_string(),
            Trigger::OnEntryUpdate => "OnEntryUpdate".to_string(),
            Trigger::OnEntryDelete => "OnEntryDelete".to_string(),
            Trigger::OnSchedule(schedule) => format!("OnSchedule({schedule})"),
            Trigger::Manual => "Manual".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::str::FromStr;

    #[test]
    fn plugin_new_sets_defaults_and_exposes_fields() {
        let path = PathBuf::from("plugins/example.py");
        let plugin = Plugin::new(
            "example".to_string(),
            "Example plugin".to_string(),
            Trigger::Manual,
            path.clone(),
        );

        assert_eq!(plugin.name(), "example");
        assert_eq!(plugin.description(), "Example plugin");
        assert!(matches!(plugin.trigger(), Trigger::Manual));
        assert_eq!(plugin.path(), &path);
        assert!(!plugin.enabled());
        assert!(plugin.valid());
        assert!(plugin.validation_warnings().is_empty());
    }

    #[test]
    fn plugin_setters_update_mutable_state() {
        let mut plugin = Plugin::new(
            "example".to_string(),
            "Example plugin".to_string(),
            Trigger::OnEntryCreate,
            PathBuf::from("plugins/example.py"),
        );

        plugin.set_enabled(true);
        plugin.set_valid(false);
        plugin.set_validation_warnings(vec!["warn-1".to_string(), "warn-2".to_string()]);

        assert!(plugin.enabled());
        assert!(!plugin.valid());
        assert_eq!(
            plugin.validation_warnings(),
            &vec!["warn-1".to_string(), "warn-2".to_string()]
        );
    }

    #[test]
    fn backend_event_trigger_kind_maps_non_manual_variants() {
        assert_eq!(
            BackendEvent::EntryCreated {
                path: "/data/file".to_string()
            }
                .trigger_kind(),
            Some(TriggerKind::OnEntryCreate)
        );
        assert_eq!(
            BackendEvent::EntryUpdated {
                path: "/data/file".to_string()
            }
                .trigger_kind(),
            Some(TriggerKind::OnEntryUpdate)
        );
        assert_eq!(
            BackendEvent::EntryDeleted {
                path: "/data/file".to_string()
            }
                .trigger_kind(),
            Some(TriggerKind::OnEntryDelete)
        );
        assert_eq!(
            BackendEvent::OnSchedule {
                schedule: Schedule::from_str("0 * * * * *").unwrap(),
                path: "/data/file".to_string()
            }
                .trigger_kind(),
            Some(TriggerKind::OnSchedule)
        );
        assert_eq!(
            BackendEvent::Manual {
                plugin_name: "example".to_string()
            }
                .trigger_kind(),
            None
        );
    }

    #[test]
    fn trigger_to_string_returns_human_readable_variants() {
        assert_eq!(Trigger::OnEntryCreate.to_string(), "OnEntryCreate");
        assert_eq!(Trigger::OnEntryUpdate.to_string(), "OnEntryUpdate");
        assert_eq!(Trigger::OnEntryDelete.to_string(), "OnEntryDelete");
        assert_eq!(Trigger::Manual.to_string(), "Manual");
        assert!(
            Trigger::OnSchedule(Schedule::from_str("0 * * * * *").unwrap())
                .to_string()
                .starts_with("OnSchedule(")
        );
    }
}
