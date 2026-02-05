mod common;

use std::env;

use backend::{
    schema::{self, files},
    storage::models::*,
    storage::storage_manager::StorageManager,
};
use chrono::{SubsecRound, Utc};
use diesel::prelude::*;
use tracing::{debug, instrument};
use tracing_subscriber::field::debug;

// tests are run in parallel => collisions

#[test]
fn test_database_connection() {
    common::init_test_logging();

    let mut conn = common::establish_test_connection();

    // Clean up before test
    common::cleanup_test_data(&mut conn);

    // Insert a test file record
    let test_file = backend::storage::models::File {
        path: "/test/path/file.txt".to_string(),
        is_custom_metadata: false,
        is_mcap: false,
    };

    diesel::insert_into(files::table)
        .values(&test_file)
        .execute(&mut conn)
        .expect("Failed to insert test file");

    // Query the inserted file
    let result: Vec<backend::storage::models::File> = files::table
        .filter(files::path.eq("/test/path/file.txt"))
        .load(&mut conn)
        .expect("Failed to query test file");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].path, "/test/path/file.txt");
}

#[instrument]
#[tokio::test]
async fn test_get_entry() {
    common::init_test_logging();

    // create a minimal entry
    let now = chrono::Utc::now().trunc_subsecs(3);

    let test_entry = backend::storage::models::Entry {
        compression_format: None,
        id: 0,
        name: "test_entry".to_string(),
        path: "/test/path/entry".to_string(),
        platform: "test_platform".to_string(),
        size: 123,
        start_time_ns: None,
        duration_ns: None,
        compression_mode: None,
        storage_identifier: None,
        total_message_count: None,
        created_at: now,
        updated_at: now,
    };

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let storage_manager = StorageManager::new(&db_url).unwrap();

    let conn = storage_manager.db_connection_pool().get().await.unwrap();
    let all_entries = conn
        .interact(|conn| {
            schema::entries::dsl::entries.load::<backend::storage::models::Entry>(conn)
        })
        .await
        .unwrap()
        .unwrap();
    debug!("All entries in database: {:?}", all_entries);
    assert_eq!(all_entries.len(), 0);
    let test_entry_clone = test_entry.clone();
    let rows_inserted = conn
        .interact(move |conn| {
            diesel::insert_into(schema::entries::dsl::entries)
                .values(test_entry_clone)
                .execute(conn)
        })
        .await
        .unwrap()
        .unwrap();
    debug!(
        "Inserted test entry: {:?}, rows inserted: {}",
        test_entry, rows_inserted
    );
    assert_eq!(rows_inserted, 1);
    let all_entries = conn
        .interact(|conn| {
            schema::entries::dsl::entries.load::<backend::storage::models::Entry>(conn)
        })
        .await
        .unwrap()
        .unwrap();
    debug!("All entries in database: {:?}", all_entries);

    let entry_by_id = storage_manager.get_entry(0, 0).await.unwrap().unwrap();
    assert_eq!(entry_by_id, test_entry);
    let entry_by_path = storage_manager
        .get_entry_by_path("/test/path/entry".to_string(), 0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(entry_by_path, test_entry);
}
