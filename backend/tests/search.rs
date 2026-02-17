use std::env;

use backend::{schema, storage::storage_manager::StorageManager};
use chrono::SubsecRound;
use diesel::RunQueryDsl;
use tracing::{debug, instrument};
use tracing_subscriber::field::debug;

mod common;

#[instrument]
#[tokio::test]
async fn test_search() {
    common::init_test_logging();

    // create a minimal entry
    let now = chrono::Utc::now().trunc_subsecs(3);

    let test_entry = backend::storage::models::Entry {
        id: 0,
        name: "Test Entry".to_string(),
        created_at: now,
        updated_at: now,
        path: "/test/path/entry".to_string(),
        size: 123,
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
        // topics: vec!["/topic1".to_string(), "/topic2".to_string()],
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

    let entries = storage_manager
        .get_entries(Some("Test".to_string()), None, None, None, None, 0)
        .await
        .unwrap();
    debug!("searched entries: {:?}", entries);
    assert_eq!(entries.len(), 1);
}
