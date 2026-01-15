#[derive(Debug, Clone)]
pub struct Plugin {}

impl Plugin {
    pub fn new() -> Self {
        todo!()
    }

    pub fn name(&self) -> &String {
        todo!()
    }

    pub fn description(&self) -> &String {
        todo!()
    }

    pub fn trigger(&self) -> &Trigger {
        todo!()
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
