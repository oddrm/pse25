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
        Self {
            name,
            description,
            trigger,
            path,

            enabled: true,
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
        self.enabled = enabled;
    }

    pub fn valid(&self) -> bool {
        self.valid
    }

    pub fn set_valid(&mut self, valid: bool) {
        self.valid = valid;
    }

    pub fn validation_warnings(&self) -> &Vec<String> {
        &self.validation_warnings
    }

    pub fn set_validation_warnings(&mut self, warnings: Vec<String>) {
        self.validation_warnings = warnings;
    }
}

#[derive(Debug, Clone)]
pub enum Trigger {
    OnEntryCreate,
    OnEntryUpdate,
    OnEntryDelete,
    OnSchedule(String),
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
