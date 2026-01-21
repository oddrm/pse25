use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable, Debug)]
#[diesel(table_name = crate::schema::files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct File {
    pub path: String,
    pub last_modified: chrono::NaiveDateTime,
    pub created: chrono::NaiveDateTime,
    pub size: i64,
    pub last_checked: chrono::NaiveDateTime,
}
