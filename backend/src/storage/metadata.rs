#![allow(unused)]

use crate::error::Error;
use rocket::serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Metadata {}

impl Metadata {
    // includes validation
    // TODO: Anpassung  str-> String Entwurfsheft nötig?
    pub fn from_yaml(yaml_str: &str) -> Result<Self, Error> {
        todo!()
    }

    pub fn to_yaml(&self) -> Result<String, Error> {
        todo!()
    }
}