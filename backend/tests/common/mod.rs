mod plugin_manager;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::env;
use std::path::PathBuf;
use std::sync::Once;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

static INIT: Once = Once::new();

/// Initialize logging for tests (only runs once)
pub fn init_test_logging() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_writer(std::io::stdout)
            .pretty()
            .with_max_level(tracing::Level::DEBUG)
            .try_init()
            .ok(); // Ignore error if already initialized
    });
}

/// Set up a test database connection
pub fn establish_test_connection() -> PgConnection {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".to_string());

    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to test database"))
}

/// Clean up test data from database
pub fn cleanup_test_data(conn: &mut PgConnection) {
    use diesel::sql_query;

    // Clean up in reverse order of foreign key dependencies
    sql_query("TRUNCATE TABLE tags, sequences, metadata, topics, entries, files CASCADE")
        .execute(conn)
        .expect("Failed to clean up test data");
}

/// Unique temp file path for tests.
pub fn unique_temp_file_path(file_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();

    env::temp_dir().join(format!("pse25_{nanos}_{file_name}"))
}

/// Creates a temporary YAML config file and returns its path.
pub fn create_yaml_config(contents: &str) -> PathBuf {
    let path = unique_temp_file_path("plugins.yaml");
    fs::write(&path, contents).expect("failed to write temp yaml config");
    path
}
