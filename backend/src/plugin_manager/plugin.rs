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
