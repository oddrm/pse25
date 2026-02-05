use std::path::Path;

use crate::error::StorageError;
use tokio::io::AsyncReadExt;
use tracing::instrument;

const CUSTOM_METADATA_IDENTIFIER: &str = r"definitions:
  info:
    data_spec_version: '0.2'
    dataset_license: MIT license
    meta_data_spec_version: '0.2'";

#[instrument]
pub async fn file_is_mcap(path: &Path) -> bool {
    path.extension()
        .map_or(false, |ext| ext.to_string_lossy().to_lowercase() == "mcap")
}

#[instrument]
pub async fn file_is_custom_metadata(path: &Path) -> Result<bool, StorageError> {
    let correct_extension = match path.extension() {
        Some(ext) => {
            let ext_lc = ext.to_string_lossy().to_lowercase();
            ext_lc == "yaml" || ext_lc == "yml"
        }
        None => false,
    };
    if correct_extension {
        let mut file = tokio::fs::File::open(path)
            .await
            .map_err(|e| StorageError::IoError(e.into()))?;
        let mut buffer = [0; 256];
        let read_bytes = file
            .read(&mut buffer)
            .await
            .map_err(|e| StorageError::IoError(e.into()))?;
        let content = String::from_utf8_lossy(&buffer[..read_bytes]);
        if content.contains(CUSTOM_METADATA_IDENTIFIER) {
            return Ok(true);
        }
    }
    Ok(false)
}
