use cron::Schedule;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct Plugin {
    name: String,
    description: String,
    trigger: Trigger,
    path: std::path::PathBuf,

    // möchte ich es nutzen?
    enabled: bool,
    // ist es funktionsfähig?
    valid: bool,
    validation_warnings: Vec<String>,
}

impl Plugin {
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

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn trigger(&self) -> &Trigger {
        &self.trigger
    }

    pub fn path(&self) -> &std::path::PathBuf {
        &self.path
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        debug!("Set enabled={} for plugin '{}'", enabled, self.name);
        self.enabled = enabled;
    }

    pub fn valid(&self) -> bool {
        self.valid
    }

    pub fn set_valid(&mut self, valid: bool) {
        debug!("Set valid={} for plugin '{}'", valid, self.name);
        self.valid = valid;
    }

    pub fn validation_warnings(&self) -> &Vec<String> {
        &self.validation_warnings
    }

    pub fn set_validation_warnings(&mut self, warnings: Vec<String>) {
        debug!(
            "Set validation warnings for plugin '{}' => {} warnings",
            self.name,
            warnings.len()
        );
        self.validation_warnings = warnings;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerKind {
    OnEntryCreate,
    OnEntryUpdate,
    OnEntryDelete,
}

#[derive(Debug, Clone)]
pub enum BackendEvent {
    EntryCreated { path: String },
    EntryUpdated { path: String },
    EntryDeleted { path: String },
    Manual { plugin_name: String }, // optional
}

impl BackendEvent {
    pub fn trigger_kind(&self) -> Option<TriggerKind> {
        match self {
            BackendEvent::EntryCreated { .. } => Some(TriggerKind::OnEntryCreate),
            BackendEvent::EntryUpdated { .. } => Some(TriggerKind::OnEntryUpdate),
            BackendEvent::EntryDeleted { .. } => Some(TriggerKind::OnEntryDelete),
            BackendEvent::Manual { .. } => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Trigger {
    OnEntryCreate,
    OnEntryUpdate, // implementiert
    OnEntryDelete,
    OnSchedule(Schedule),
    Manual,
}

impl Trigger {
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
