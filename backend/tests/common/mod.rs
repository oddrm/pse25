use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::env;
use std::sync::Once;

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
