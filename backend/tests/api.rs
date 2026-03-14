
mod common;

use std::env;
use std::sync::Arc;

use backend::routes::database::{get_entries, get_entry, get_entry_by_path};
use backend::routes::health_check::health;
use backend::storage::models::Entry;
use backend::storage::storage_manager::StorageManager;
use backend::AppState;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use rocket::serde::json::serde_json;

const TXID: u64 = 0;

fn skip_if_no_db() -> bool {
    if env::var("DATABASE_URL").is_err() {
        eprintln!("Skipping API tests: DATABASE_URL not set");
        return true;
    }
    false
}

async fn build_test_rocket() -> rocket::Rocket<rocket::Build> {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let storage_manager = StorageManager::new(&db_url).expect("failed to create StorageManager");
    let plugin_manager = Arc::new(tokio::sync::Mutex::new(
        backend::plugin_manager::manager::PluginManager::new(),
    ));

    rocket::build()
        .mount(
            "/",
            rocket::routes![health, get_entries, get_entry, get_entry_by_path],
        )
        .manage(AppState {
            storage_manager,
            plugin_manager,
        })
}

#[tokio::test]
async fn test_health_endpoint_ok() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let client = Client::tracked(build_test_rocket().await)
        .await
        .expect("failed to build rocket client");

    let resp = client.get("/health").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let body = resp.into_string().await.unwrap();
    assert_eq!(body, "OK");
}

#[tokio::test]
async fn test_get_entries_empty_list_ok() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    // Ensure DB is empty for entries/files.
    let db_url = env::var("DATABASE_URL").unwrap();
    let storage = StorageManager::new(&db_url).unwrap();
    common::remove_all_data(&storage).await.unwrap();

    let client = Client::tracked(build_test_rocket().await)
        .await
        .expect("failed to build rocket client");

    let resp = client
        .get("/entries?txid=0&page=0&page_size=10")
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let body = resp.into_string().await.unwrap();
    // get_entries returns Json<(Vec<Entry>, u32)>
    let parsed: (Vec<Entry>, u32) =
        serde_json::from_str(&body).expect("response body should be valid JSON");
    assert!(parsed.0.is_empty(), "expected empty entries list");
    // When there are 0 entries, num_pages currently comes back as 1 (implementation detail),
    // but at minimum we assert it is >= 1.
    assert!(parsed.1 >= 1);
}

#[tokio::test]
async fn test_get_entry_404_for_unknown_id() {
    if skip_if_no_db() {
        return;
    }
    common::init_test_logging();

    let client = Client::tracked(build_test_rocket().await)
        .await
        .expect("failed to build rocket client");

    let resp = client.get("/entries/999999/tx/0").dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

