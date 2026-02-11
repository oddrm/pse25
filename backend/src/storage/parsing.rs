use std::io::Read;
use std::io::Seek;
use std::path::Path;

use crate::{error::StorageError, storage::models::Entry};
use mcap::sans_io::SummaryReadEvent;
use mcap::sans_io::SummaryReader;
use mcap::sans_io::SummaryReaderOptions;
use tokio::io::AsyncReadExt;
use tokio::task::spawn_blocking;
use tracing::debug;
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

#[instrument]
pub async fn insert_entry_from_mcap(path: &Path) -> Result<Entry, StorageError> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|e| StorageError::IoError(e.into()))?;
    debug!("Reading MCAP file: {:?}", path);
    debug!("File metadata: {:?}", file.metadata().await);
    debug!("Extracting topics from MCAP file: {:?}", path);
    // can lead to SIGBUS if the file is modified
    // let mapped = unsafe {
    //     memmap2::MmapOptions::new()
    //         .map(&file.into_std().await)
    //         .unwrap()
    // };
    // let summary = Summary::read(&mapped).unwrap().unwrap();
    let path = path.to_owned();

    // custom version of Summary::read because the original requires a memory mapped buffer, which is unsafe
    let summary = spawn_blocking(move || -> Result<mcap::Summary, StorageError> {
        let mut f = std::fs::File::open(path)?;
        let file_size = f.metadata()?.len();
        let mut reader = SummaryReader::new_with_options(
            SummaryReaderOptions::default().with_file_size(file_size),
        );
        while let Some(event_res) = reader.next_event() {
            let event = event_res?;
            match event {
                SummaryReadEvent::ReadRequest(n) => {
                    let buf = reader.insert(n);
                    let read = f.read(buf)?;
                    reader.notify_read(read);
                }
                SummaryReadEvent::SeekRequest(to) => {
                    let pos = f.seek(to)?;
                    reader.notify_seeked(pos);
                }
            }
        }
        match reader.finish() {
            Some(s) => Ok(s),
            None => Err(StorageError::CustomError(
                "MCAP summary does not exist".to_string(),
            )),
        }
    })
    .await
    .map_err(|e| StorageError::CustomError(format!("{:?}", e)))??;
    let topics = summary
        .channels
        .iter()
        .map(|channel| channel.1.topic.clone())
        .collect::<Vec<_>>();
    debug!("Extracted topics: {:?}", topics);
    todo!()
}
