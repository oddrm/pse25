#![allow(unused)]

use crate::error::Error;
use rocket::serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Metadata {}

impl Metadata {
    // includes validation
    pub fn from_yaml(yaml_str: &String) -> Result<Self, Error> {
        todo!()
    }

    pub fn to_yaml(&self) -> Result<String, Error> {
        todo!()
    }
}