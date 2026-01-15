use rocket::serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Sequence {}

impl Sequence {
    pub fn new(description: String, start: Timestamp, end: Timestamp) -> Self {
        todo!()
    }

    pub fn description(&self) -> &String {
        todo!()
    }

    pub fn start(&self) -> Timestamp {
        todo!()
    }

    pub fn end(&self) -> Timestamp {
        todo!()
    }
}

pub type SequenceID = u64;
// in milliseconds since UNIX EPOCH
pub type Timestamp = u64;
