use rocket::serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub platform: String,
    pub size: u64,
    pub tags: Vec<String>,
}
