#[derive(Debug, Clone)]
pub struct Plugin {
    name: String,
    description: String,
    trigger: Trigger,
    path: std::path::PathBuf, // optional, aber meist praktisch
}

impl Plugin {
    pub fn new(name: String, description: String, trigger: Trigger, path: std::path::PathBuf)
        -> Self {
        Self {
            name,
            description,
            trigger,
            path,
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
}

#[derive(Debug, Clone)]
pub enum Trigger {
    OnEntryCreate,
    OnEntryUpdate,
    OnEntryDelete,
    OnSchedule(String),
    Manual,
}
