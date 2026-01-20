use diesel::prelude::*;
use serde::{Deserialize, Serialize};

// Queryable: Diesel generiert Implementierungen, damit Ergebnisse aus SQL‑Abfragen direkt in
// deinen Struct gemappt werden können (Entry als Ergebnis von posts::table.load(conn) usw.).
// Selectable: Erlaubt dir, Entry::as_select() in Query‑DSLs zu nutzen, z.B. bei Joins oder
// Teilprojektionen; Diesel weiß dann, wie er diese Struct aus der Abfrage befüllen soll.
// Serialize: Typ direkt als JSON o.Ä. nach außen gegeben werden soll, z.B. als HTTP‑Response im
// Webserver

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::schema::posts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Entry {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub platform: String,
    pub size: i64,
    pub tags: Vec<String>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = crate::schema::posts)]
pub struct NewEntry {
    pub name: String,
    pub path: String,
    pub platform: String,
    pub size: i64,
    pub tags: Vec<String>,
}

// Für PATCH/PUT: alles optional
#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = crate::schema::posts)]
pub struct UpdateEntry {
    pub name: Option<String>,
    pub path: Option<String>,
    pub platform: Option<String>,
    pub tags: Option<Vec<String>>,
}