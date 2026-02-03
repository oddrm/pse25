mod common;

use backend::schema::files;
use diesel::prelude::*;

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

    // Clean up after test
    common::cleanup_test_data(&mut conn);
}
