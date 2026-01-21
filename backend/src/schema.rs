// @generated automatically by Diesel CLI.

diesel::table! {
    files (path) {
        path -> Varchar,
        last_modified -> Timestamp,
        created -> Timestamp,
        size -> Int8,
        last_checked -> Timestamp,
    }
}
