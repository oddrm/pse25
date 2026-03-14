//! Integration tests for storage manager: pagination, sorting, sequences, tags.
//! Require DATABASE_URL. Use unique entry ids/paths to avoid collisions with other test crates.

mod common;

use std::env;

use backend::routes::database::MetadataWeb;
use backend::storage::models::{Entry, Sequence, Topic, Sensor};
use backend::storage::storage_manager::StorageManager;
use chrono::{SubsecRound, Utc};
use diesel::prelude::*;
use backend::schema;

const INTEGRATION_ENTRY_ID_BASE: i64 = 80_000;
const TXID: u64 = 0;

fn skip_if_no_db() -> bool {
    if std::env::var("DATABASE_URL").is_err() {
        eprintln!("Skipping storage_integration tests: DATABASE_URL not set");
        return true;
    }
    false
}

fn minimal_entry(id: i64, name: &str, path: &str) -> Entry {
    let now = Utc::now().trunc_subsecs(3);
    Entry {
        id,
        name: name.to_string(),
        created_at: now,
        updated_at: now,
        path: path.to_string(),
        size: 0,
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
        tags: vec![],
    }
}

/// Insert an entry via raw diesel and return the row (with DB-assigned id if applicable).
async fn insert_entry(storage: &StorageManager, entry: Entry) -> Entry {
    let conn = storage.db_connection_pool().get().await.unwrap();
    let inserted = conn
        .interact(move |conn| {
            diesel::insert_into(schema::entries::dsl::entries)
                .values(entry)
                .returning(backend::storage::models::Entry::as_select())
                .get_result::<Entry>(conn)
        })
        .await
        .unwrap()
        .unwrap();
    inserted
}

#[tokio::test]
async fn test_get_entries_pagination() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    // Insert 5 entries with unique ids/paths
    let paths = [
        "/test/integration/pagination_a",
        "/test/integration/pagination_b",
        "/test/integration/pagination_c",
        "/test/integration/pagination_d",
        "/test/integration/pagination_e",
    ];
    let mut inserted = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let id = INTEGRATION_ENTRY_ID_BASE + i as i64;
        let entry = minimal_entry(id, &format!("Entry {}", i), path);
        inserted.push(insert_entry(&storage, entry).await);
    }

    // Page size 2: expect 3 pages
    let (page0, num_pages) = storage
        .get_entries(None, None, None, Some(0), Some(2), TXID)
        .await
        .unwrap();
    assert_eq!(num_pages, 3, "5 items / page_size 2 => 3 pages");
    assert_eq!(page0.len(), 2);

    let (page1, _) = storage
        .get_entries(None, None, None, Some(1), Some(2), TXID)
        .await
        .unwrap();
    assert_eq!(page1.len(), 2);

    let (page2, _) = storage
        .get_entries(None, None, None, Some(2), Some(2), TXID)
        .await
        .unwrap();
    assert_eq!(page2.len(), 1);

    // All returned entries should be among our inserted ones (by path)
    let inserted_paths: std::collections::HashSet<_> =
        inserted.iter().map(|e| e.path.as_str()).collect();
    for e in page0.iter().chain(page1.iter()).chain(page2.iter()) {
        assert!(inserted_paths.contains(e.path.as_str()), "stray entry: {}", e.path);
    }
}

#[tokio::test]
async fn test_get_entries_sorting() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    let ids = [INTEGRATION_ENTRY_ID_BASE + 10, INTEGRATION_ENTRY_ID_BASE + 11, INTEGRATION_ENTRY_ID_BASE + 12];
    let names = ["Zebra", "Alpha", "Midi"];
    let paths = [
        "/test/integration/sort_z",
        "/test/integration/sort_a",
        "/test/integration/sort_m",
    ];
    for i in 0..3 {
        let entry = minimal_entry(ids[i], names[i], paths[i]);
        insert_entry(&storage, entry).await;
    }

    // Sort by name ascending: filter to only our 3 entries and check order
    let (entries_asc, _) = storage
        .get_entries(None, Some("Name".to_string()), Some(true), None, None, TXID)
        .await
        .unwrap();
    let our_asc: Vec<&str> = entries_asc
        .iter()
        .filter(|e| e.path.starts_with("/test/integration/sort_"))
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(our_asc.len(), 3, "our 3 sort entries should be present");
    assert_eq!(our_asc, ["Alpha", "Midi", "Zebra"], "sort by name ascending");

    // Sort by name descending
    let (entries_desc, _) = storage
        .get_entries(None, Some("Name".to_string()), Some(false), None, None, TXID)
        .await
        .unwrap();
    let our_desc: Vec<&str> = entries_desc
        .iter()
        .filter(|e| e.path.starts_with("/test/integration/sort_"))
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(our_desc.len(), 3);
    assert_eq!(our_desc, ["Zebra", "Midi", "Alpha"], "sort by name descending");
}

#[tokio::test]
async fn test_sequences_add_get_remove() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    let entry = minimal_entry(
        INTEGRATION_ENTRY_ID_BASE + 20,
        "SeqEntry",
        "/test/integration/entry_sequences",
    );
    let inserted = insert_entry(&storage, entry).await;
    let entry_id = inserted.id;
    let txid = storage.start_transaction();

    let now = Utc::now().trunc_subsecs(3);
    let seq = Sequence {
        id: 0,
        entry_id,
        description: "drive_1".to_string(),
        start_timestamp: 1000,
        end_timestamp: 2000,
        created_at: now,
        updated_at: now,
        tags: vec!["seq_tag".to_string()],
    };

    let seq_id = storage.add_sequence(entry_id, seq.clone(), txid).await.unwrap();
    assert!(seq_id > 0);

    let sequences = storage.get_sequences(entry_id, TXID).await.unwrap();
    assert_eq!(sequences.len(), 1);
    let s = sequences.get(&seq_id).unwrap();
    assert_eq!(s.description, "drive_1");
    assert_eq!(s.start_timestamp, 1000);

    storage.remove_sequence(entry_id, seq_id, txid).await.unwrap();
    let sequences_after = storage.get_sequences(entry_id, TXID).await.unwrap();
    assert!(sequences_after.is_empty());
}

#[tokio::test]
async fn test_tags_add_remove() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    let mut entry = minimal_entry(
        INTEGRATION_ENTRY_ID_BASE + 30,
        "TagEntry",
        "/test/integration/entry_tags",
    );
    entry.tags = vec!["existing".to_string()];
    let inserted = insert_entry(&storage, entry).await;
    let entry_id = inserted.id;
    let txid = storage.start_transaction();

    storage.add_tag(entry_id, "new_tag".to_string(), txid).await.unwrap();

    let updated = storage.get_entry(entry_id, TXID).await.unwrap().unwrap();
    assert!(updated.tags.contains(&"existing".to_string()));
    assert!(updated.tags.contains(&"new_tag".to_string()));

    storage.remove_tag(entry_id, "new_tag".to_string(), txid).await.unwrap();

    let after_remove = storage.get_entry(entry_id, TXID).await.unwrap().unwrap();
    assert!(after_remove.tags.contains(&"existing".to_string()));
    assert!(!after_remove.tags.contains(&"new_tag".to_string()));
}

#[tokio::test]
async fn test_get_entry_not_found() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    // Non-existent id (use a high id that we don't insert)
    let result = storage.get_entry(99_999_999, TXID).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_entry_by_path_not_found() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    let result = storage
        .get_entry_by_path("/nonexistent/path/entry".to_string(), TXID)
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_entries_search_by_path() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    const SEARCH_TOKEN: &str = "xyzsearchtoken";
    let entry = minimal_entry(
        INTEGRATION_ENTRY_ID_BASE + 40,
        "SearchEntry",
        "/test/integration/search_xyzsearchtoken",
    );
    let inserted = insert_entry(&storage, entry).await;

    let (entries, _pages) = storage
        .get_entries(Some(SEARCH_TOKEN.to_string()), None, None, None, None, TXID)
        .await
        .unwrap();

    assert!(
        entries.iter().any(|e| e.path == inserted.path),
        "search results must contain the inserted entry"
    );
}

#[tokio::test]
async fn test_topics_add_get_update_remove() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    let entry = minimal_entry(
        INTEGRATION_ENTRY_ID_BASE + 50,
        "TopicEntry",
        "/test/integration/entry_topics",
    );
    let inserted = insert_entry(&storage, entry).await;
    let entry_id = inserted.id;
    let txid = storage.start_transaction();

    let now = Utc::now().trunc_subsecs(3);
    let topic = Topic {
        id: 0,
        entry_id,
        topic_name: "topic_a".to_string(),
        topic_type: Some("type_a".to_string()),
        message_count: 10,
        frequency: Some(5.0),
        created_at: now,
        updated_at: now,
    };

    let topic_id = storage.add_topic(topic.clone(), txid).await.unwrap();
    assert!(topic_id > 0);

    let topics = storage.get_topics(entry_id, TXID).await.unwrap();
    assert_eq!(topics.len(), 1);
    let t = topics.get(&topic_id).unwrap();
    assert_eq!(t.topic_name, "topic_a");
    assert_eq!(t.message_count, 10);

    let updated_topic = Topic {
        id: topic_id,
        entry_id,
        topic_name: "topic_b".to_string(),
        topic_type: Some("type_b".to_string()),
        message_count: 20,
        frequency: Some(10.0),
        created_at: t.created_at,
        updated_at: Utc::now().trunc_subsecs(3),
    };

    storage.update_topic(updated_topic.clone(), txid).await.unwrap();

    let topics_after = storage.get_topics(entry_id, TXID).await.unwrap();
    let t2 = topics_after.get(&topic_id).unwrap();
    assert_eq!(t2.topic_name, "topic_b");
    assert_eq!(t2.message_count, 20);

    storage.remove_topic(topic_id, txid).await.unwrap();
    let topics_final = storage.get_topics(entry_id, TXID).await.unwrap();
    assert!(topics_final.is_empty());
}

#[tokio::test]
async fn test_sensors_add_get_all_update_remove() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    let entry = minimal_entry(
        INTEGRATION_ENTRY_ID_BASE + 60,
        "SensorEntry",
        "/test/integration/entry_sensors",
    );
    let inserted = insert_entry(&storage, entry).await;
    let entry_id = inserted.id;
    let txid = storage.start_transaction();

    let sensor = Sensor {
        id: 0,
        entry_id,
        sensor_name: "Lidar".to_string(),
        manufacturer: Some("ACME".to_string()),
        sensor_type: Some("lidar".to_string()),
        ros_topics: vec!["/lidar".to_string()],
        custom_parameters: None,
    };

    let sensor_id = storage.add_sensor(sensor.clone(), txid).await.unwrap();
    assert!(sensor_id > 0);

    let sensors = storage.get_sensors(entry_id, TXID).await.unwrap();
    assert_eq!(sensors.len(), 1);
    let s = sensors.get(&sensor_id).unwrap();
    assert_eq!(s.sensor_name, "Lidar");
    assert_eq!(s.ros_topics, vec!["/lidar".to_string()]);

    let all_sensors = storage.get_all_sensors(TXID).await.unwrap();
    assert!(
        all_sensors.contains_key(&sensor_id),
        "all_sensors must contain our sensor"
    );

    let updated_sensor = Sensor {
        id: sensor_id,
        entry_id,
        sensor_name: "Camera".to_string(),
        manufacturer: Some("OtherCorp".to_string()),
        sensor_type: Some("camera".to_string()),
        ros_topics: vec!["/camera".to_string()],
        custom_parameters: None,
    };

    storage.update_sensor(updated_sensor.clone(), txid).await.unwrap();

    let sensors_after = storage.get_sensors(entry_id, TXID).await.unwrap();
    let s2 = sensors_after.get(&sensor_id).unwrap();
    assert_eq!(s2.sensor_name, "Camera");
    assert_eq!(s2.manufacturer.as_deref(), Some("OtherCorp"));

    storage.remove_sensor(sensor_id, txid).await.unwrap();
    let sensors_final = storage.get_sensors(entry_id, TXID).await.unwrap();
    assert!(sensors_final.is_empty());
}

#[tokio::test]
async fn test_get_entries_search_by_tag() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    const TAG_TOKEN: &str = "tagsearchtoken";
    let mut entry = minimal_entry(
        INTEGRATION_ENTRY_ID_BASE + 70,
        "TagSearchEntry",
        "/test/integration/search_tag",
    );
    entry.tags = vec![TAG_TOKEN.to_string()];
    let inserted = insert_entry(&storage, entry).await;

    let (entries, _pages) = storage
        .get_entries(Some(TAG_TOKEN.to_string()), None, None, None, None, TXID)
        .await
        .unwrap();

    assert!(
        entries.iter().any(|e| e.path == inserted.path),
        "search by tag must return the entry"
    );
}

#[tokio::test]
async fn test_get_entries_search_by_topic_name() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    const TOPIC_TOKEN: &str = "topicsearchtoken";
    let entry = minimal_entry(
        INTEGRATION_ENTRY_ID_BASE + 80,
        "TopicSearchEntry",
        "/test/integration/search_topic",
    );
    let inserted = insert_entry(&storage, entry).await;
    let entry_id = inserted.id;
    let txid = storage.start_transaction();

    let now = Utc::now().trunc_subsecs(3);
    let topic = Topic {
        id: 0,
        entry_id,
        topic_name: TOPIC_TOKEN.to_string(),
        topic_type: None,
        message_count: 1,
        frequency: None,
        created_at: now,
        updated_at: now,
    };
    storage.add_topic(topic, txid).await.unwrap();

    let (entries, _pages) = storage
        .get_entries(Some(TOPIC_TOKEN.to_string()), None, None, None, None, TXID)
        .await
        .unwrap();

    assert!(
        entries.iter().any(|e| e.path == inserted.path),
        "search by topic name must return the entry"
    );
}

#[tokio::test]
async fn test_get_entries_search_by_date() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    let entry = minimal_entry(
        INTEGRATION_ENTRY_ID_BASE + 90,
        "DateSearchEntry",
        "/test/integration/search_date",
    );
    let inserted = insert_entry(&storage, entry).await;

    let date_str = inserted.created_at.date_naive().to_string(); // \"YYYY-MM-DD\"

    let (entries, _pages) = storage
        .get_entries(Some(date_str), None, None, None, None, TXID)
        .await
        .unwrap();

    assert!(
        entries.iter().any(|e| e.path == inserted.path),
        "search by date must return the entry"
    );
}

#[tokio::test]
async fn test_add_entry_and_get_entry() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    let mut entry = minimal_entry(
        INTEGRATION_ENTRY_ID_BASE + 100,
        "AddEntry",
        "/test/integration/add_entry",
    );
    entry.status = "Complete".to_string();
    entry.tags = vec!["from_add_entry".to_string()];

    let txid = storage.start_transaction();
    let new_id = storage.add_entry(entry.clone(), txid).await.unwrap();

    let fetched = storage.get_entry(new_id, TXID).await.unwrap().unwrap();
    assert_eq!(fetched.path, entry.path);
    assert_eq!(fetched.name, entry.name);
    assert_eq!(fetched.status, entry.status);
    assert!(fetched.tags.contains(&"from_add_entry".to_string()));
}

#[tokio::test]
async fn test_update_entry_metadata_fields() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    let entry = minimal_entry(
        INTEGRATION_ENTRY_ID_BASE + 110,
        "UpdateMetadataEntry",
        "/test/integration/update_metadata",
    );
    let inserted = insert_entry(&storage, entry).await;
    let entry_id = inserted.id;

    let now = Utc::now().trunc_subsecs(3);
    let md = MetadataWeb {
        time_machine: Some(1.23),
        platform_name: Some("PlatformX".to_string()),
        platform_image_link: Some("http://image".to_string()),
        scenario_name: Some("ScenarioA".to_string()),
        scenario_creation_time: Some(now),
        scenario_description: Some("Desc".to_string()),
        sequence_duration: Some(42.0),
        sequence_distance: Some(1000.0),
        sequence_lat_starting_point_deg: Some(1.0),
        sequence_lon_starting_point_deg: Some(2.0),
        weather_cloudiness: Some("cloudy".to_string()),
        weather_precipitation: Some("rain".to_string()),
        weather_precipitation_deposits: Some("wet".to_string()),
        weather_wind_intensity: Some("strong".to_string()),
        weather_road_humidity: Some("humid".to_string()),
        weather_fog: Some(true),
        weather_snow: Some(false),
        topics: None,
    };

    storage.update_entry(entry_id, md.clone(), TXID).await.unwrap();

    let updated = storage.get_entry(entry_id, TXID).await.unwrap().unwrap();
    assert_eq!(updated.platform_name.as_deref(), Some("PlatformX"));
    assert_eq!(updated.scenario_name.as_deref(), Some("ScenarioA"));
    assert_eq!(updated.sequence_duration, Some(42.0));
    assert_eq!(updated.weather_cloudiness.as_deref(), Some("cloudy"));
    assert_eq!(updated.weather_fog, Some(true));
}

#[tokio::test]
async fn test_start_and_commit_transaction() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();

    let txid = storage.start_transaction();
    // commit_transaction currently only affects in-memory active_transactions,
    // but this test ensures it at least succeeds for a valid txid.
    storage.commit_transaction(txid).await.unwrap();
}
