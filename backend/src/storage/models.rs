use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Post {
    pub path: String,
    pub last_modified: chrono::NaiveDateTime,
    pub created: chrono::NaiveDateTime,
    pub size: i64,
    pub last_checked: chrono::NaiveDateTime,
}
