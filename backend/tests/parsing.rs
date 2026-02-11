use tracing::debug;
use tracing::instrument;
mod common;

// #[instrument]
// #[tokio::test]
// async fn test_mcap_reading() {
//     common::init_test_logging();
//     // let path = std::path::Path::new(
//     //     "/data/goose/alice/alice_scenario06_sequence02/alice_scenario06_sequence02.mcap",
//     // );
//     let path = std::path::Path::new("/data/excavator_drive.mcap");
//     let entry = backend::storage::parsing::insert_entry_from_mcap(path)
//         .await
//         .expect("Failed to read MCAP file");
// }
