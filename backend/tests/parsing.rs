use tokio::fs;
use tokio::process::Command;
use tracing::debug;
use tracing::instrument;
mod common;

#[instrument]
#[tokio::test]
async fn test_mcap_reading() {
    common::init_test_logging();
    // let path = std::path::Path::new(
    //     "/data/goose/alice/alice_scenario06_sequence02/alice_scenario06_sequence02.mcap",
    // );
    let path = std::path::Path::new("/data/excavator_drive.mcap");
    // prefer resolving via `which mcap` (checks PATH)
    let mut found = false;
    let out = Command::new("which").arg("mcap").output().await;
    debug!("which mcap output: {:?}", out);
    match out {
        Ok(out) => {
            if out.status.success() {
                let which_path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !which_path.is_empty() {
                    debug!("which mcap => {}", which_path);
                    if fs::metadata(&which_path).await.is_ok() {
                        found = true;
                    }
                }
            }
        }
        Err(err) => {
            debug!("Error running `which mcap`: {}", err);
        }
    };

    // fallback: check common install locations
    let candidates = ["/usr/local/bin/mcap", "/usr/local/cargo/bin/mcap"];
    for p in candidates.iter() {
        if fs::metadata(p).await.is_ok() {
            debug!("Found mcap at {}", p);
            found = true;
            break;
        }
    }

    // no diagnostic listings in CI

    assert!(found, "mcap binary not found in expected locations or PATH");

    // run `mcap version` to ensure executable works (subcommand `version` exists)
    let out = Command::new("mcap")
        .arg("version")
        .output()
        .await
        .expect("failed to spawn mcap");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    debug!("mcap version stdout: {}", stdout);
    debug!("mcap version stderr: {}", stderr);

    let entry = backend::storage::parsing::get_entry_from_mcap(path)
        .await
        .expect("Failed to read MCAP file");
}
