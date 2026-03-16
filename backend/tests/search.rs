use std::env;

use backend::{schema, storage::storage_manager::StorageManager};
use chrono::SubsecRound;
use diesel::prelude::*;
use tracing::{debug, instrument};
use tracing_subscriber::field::debug;

mod common;

#[instrument]
#[tokio::test]
async fn test_search() {
    if std::env::var("DATABASE_URL").is_err() {
        eprintln!("Skipping test_search: DATABASE_URL not set");
        return;
    }
    common::init_test_logging();

    // create a minimal entry (unique id/path to avoid collision with database::test_get_entry which uses id=0)
    let now = chrono::Utc::now().trunc_subsecs(3);
    const SEARCH_TEST_ENTRY_ID: i64 = 999_999;
    const SEARCH_TEST_ENTRY_PATH: &str = "/test/path/entry_search";

    let test_entry = backend::storage::models::Entry {
        id: SEARCH_TEST_ENTRY_ID,
        name: "Test Entry".to_string(),
        created_at: now,
        updated_at: now,
        path: SEARCH_TEST_ENTRY_PATH.to_string(),
        size: 123,
        status: "Complete".to_string(),
        time_machine: None,
        platform_name: None,
        platform_image_link: None,
        scenario_name: None,
        scenario_creation_time: None,
        scenario_description: None,
        sequence_duration: None,
        sequence_distance: None,
        sequence_lat_starting_point_deg: None,
        sequence_lon_starting_point_deg: None,
        weather_cloudiness: None,
        weather_precipitation: None,
        weather_precipitation_deposits: None,
        weather_wind_intensity: None,
        weather_road_humidity: None,
        weather_fog: None,
        weather_snow: None,
        tags: vec!["test".to_string(), "entry".to_string()],
    };

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let storage_manager = StorageManager::new(&db_url).unwrap();

    let conn = storage_manager.db_connection_pool().get().await.unwrap();
    let test_entry_clone = test_entry.clone();
    let inserted_entry = conn
        .interact(move |conn| {
            diesel::insert_into(schema::entries::dsl::entries)
                .values(test_entry_clone)
                .returning(backend::storage::models::Entry::as_select())
                .get_result::<backend::storage::models::Entry>(conn)
        })
        .await
        .unwrap()
        .unwrap();
    let inserted_id = inserted_entry.id;
    debug!("Inserted entry: {:?}", inserted_entry);

    let entry_by_id = storage_manager.get_entry(inserted_id, 0).await.unwrap().unwrap();
    assert_eq!(entry_by_id.path, test_entry.path);
    assert_eq!(entry_by_id.name, test_entry.name);

    let entry_by_path = storage_manager
        .get_entry_by_path(SEARCH_TEST_ENTRY_PATH.to_string(), 0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(entry_by_path.id, inserted_id);

    let (entries, _num_pages) = storage_manager
        .get_entries(Some("Test".to_string()), None, None, None, None, 0)
        .await
        .unwrap();
    debug!("searched entries: {:?}", entries);
    assert!(
        entries.iter().any(|e| e.path == test_entry.path),
        "search results must contain the test entry"
    );
}
