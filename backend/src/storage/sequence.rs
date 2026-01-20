use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::sequences;

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::schema::sequences)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Sequence {
    pub sequence_id: i64,
    pub title: String,
    // brauchen wir den wirklich!?
    pub timestamp: i64,
    // i64 passt wohl besser zu PostgreSQL;
    // Frontend kümmert sich um schöne Darstellung hh:mm:ss:msms; hier nicht relevant
    pub start_time: i64,
    pub end_time: i64,
    pub description: String,
}

// INSERT: ohne sequence_id, wenn die DB den PK erzeugt
#[derive(Insertable, Deserialize)]
#[diesel(table_name = crate::schema::sequences)]
pub struct NewSequence {
    pub title: String,
    pub timestamp: i64,
    pub start_time: i64,
    pub end_time: i64,
    pub description: String,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = crate::schema::sequences)]
pub struct UpdateSequence {
    pub title: Option<String>,
    //pub timestamp: Option<i64>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub description: Option<String>,
}