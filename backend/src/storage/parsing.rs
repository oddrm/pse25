use std::path::Path;

use crate::storage::storage_manager::StorageManager;
use crate::{
    error::StorageError,
    storage::models::{Entry, Sensor, Sequence},
};
use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use serde_json;
use serde_yaml;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tracing::debug;
use tracing::error;
use tracing::instrument;

const CUSTOM_METADATA_IDENTIFIER: &str = r"title: ";

#[derive(serde::Deserialize, Debug, Clone)]
pub struct TopicInfo {
    pub topic: String,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub message_count: u64,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct McapInfo {
    pub topics: Vec<TopicInfo>,
    pub start_time_ns: Option<i64>,
    pub end_time_ns: Option<i64>,
    pub duration_seconds: Option<f64>,
}

async fn get_mcap_info(path: &Path) -> Result<McapInfo, StorageError> {
    // Use the `mcap` CLI plaintext output: `mcap info <path>` and parse it.
    let mut cmd = Command::new("mcap");
    cmd.arg("info").arg(path);
    // debug!("Running command: mcap info {:?}", path);
    let output = match cmd.output().await {
        Ok(o) => o,
        Err(e) => {
            debug!("Failed to spawn mcap: {:?}", e);
            return Err(StorageError::IoError(e.into()));
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        debug!("mcap stderr: {}", stderr);
        return Err(StorageError::CustomError(format!(
            "mcap info failed: status={:?} stderr={}",
            output.status, stderr
        )));
    }
    // debug!("mcap stdout length: {}", output.stdout.len());
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    debug!("mcap stdout: {}", stdout);
    let mut duration_seconds: Option<f64> = None;
    let mut start_time_ns: Option<i64> = None;
    let mut end_time_ns: Option<i64> = None;
    let mut topics: Vec<TopicInfo> = Vec::new();

    let mut in_channels = false;
    for raw_line in stdout.lines() {
        let line = raw_line.trim_end();
        if line.starts_with("duration:") {
            if let Some(val) = line.splitn(2, ':').nth(1) {
                let s = val.trim();
                // e.g. "31.877158155s"
                if let Some(num) = s.strip_suffix('s') {
                    if let Ok(f) = num.trim().parse::<f64>() {
                        duration_seconds = Some(f);
                    }
                } else if let Ok(f) = s.parse::<f64>() {
                    duration_seconds = Some(f);
                }
            }
            continue;
        }
        if line.starts_with("start:") {
            if let Some(idx1) = line.find('(') {
                if let Some(idx2) = line[idx1 + 1..].find(')') {
                    let inside = &line[idx1 + 1..idx1 + 1 + idx2];
                    if let Ok(sec) = inside.trim().parse::<f64>() {
                        start_time_ns = Some((sec * 1e9) as i64);
                    }
                }
            }
            continue;
        }
        if line.starts_with("end:") {
            if let Some(idx1) = line.find('(') {
                if let Some(idx2) = line[idx1 + 1..].find(')') {
                    let inside = &line[idx1 + 1..idx1 + 1 + idx2];
                    if let Ok(sec) = inside.trim().parse::<f64>() {
                        end_time_ns = Some((sec * 1e9) as i64);
                    }
                }
            }
            continue;
        }
        if line.starts_with("channels:") {
            in_channels = true;
            continue;
        }
        if in_channels {
            // look for lines like: (1)  /rosout   142 msgs (...)
            if line.contains("msgs") {
                // parse message count
                let msg_count = line
                    .split("msgs")
                    .next()
                    .and_then(|left| {
                        // left contains '(1)  /topic   142 '
                        // try to find the last token which should be the number
                        let parts: Vec<&str> = left.split_whitespace().collect();
                        for part in parts.iter().rev() {
                            if let Ok(n) = part.trim().parse::<u64>() {
                                return Some(n);
                            }
                        }
                        None
                    })
                    .unwrap_or(0u64);

                // parse topic name: take substring after first ')' up to the message count
                let topic_name = line
                    .splitn(2, ')')
                    .nth(1)
                    .map(|s| s.trim())
                    .map(|s| {
                        // take first whitespace-separated token (topics are paths without spaces)
                        s.split_whitespace().next().unwrap_or("").to_string()
                    })
                    .unwrap_or_default();

                // parse type/name after last ':' if present
                let topic_type = line.rfind(':').and_then(|idx| {
                    let after = &line[idx + 1..];
                    let t = after.split('[').next().unwrap_or(after).trim();
                    if t.is_empty() {
                        None
                    } else {
                        Some(t.to_string())
                    }
                });

                if !topic_name.is_empty() {
                    topics.push(TopicInfo {
                        topic: topic_name,
                        r#type: topic_type,
                        message_count: msg_count,
                    });
                }
            }
        }
    }

    Ok(McapInfo {
        topics,
        start_time_ns,
        end_time_ns,
        duration_seconds,
    })
}

#[instrument]
pub fn file_is_mcap(path: &Path) -> bool {
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
    // debug!(
    //     "Checking custom metadata for {:?}, extension_ok={}",
    //     path, correct_extension
    // );
    if correct_extension {
        let mut file = tokio::fs::File::open(path)
            .await
            .map_err(|e| StorageError::IoError(e.into()))?;
        let mut buffer = [0; 256];
        let read_bytes = file
            .read(&mut buffer)
            .await
            .map_err(|e| StorageError::IoError(e.into()))?;
        // debug!(
        //     "Read {} bytes from metadata candidate {:?}",
        //     read_bytes, path
        // );
        let content = String::from_utf8_lossy(&buffer[..read_bytes]);
        if content.contains(CUSTOM_METADATA_IDENTIFIER) {
            // debug!("Custom metadata identifier found in {:?}", path);
            return Ok(true);
        } else {
            // debug!("Custom metadata identifier NOT found in {:?}", path);
        }
    }
    Ok(false)
}

#[instrument]
pub async fn get_entry_from_mcap(path: &Path) -> Result<Entry, StorageError> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|e| StorageError::IoError(e.into()))?;
    debug!("Reading MCAP file: {:?}", path);
    // debug!("File metadata: {:?}", file.metadata().await);
    // debug!("Extracting topics from MCAP file: {:?}", path);
    let path = path.to_owned();

    // Use the `mcap` CLI to extract topics/duration (get_mcap_info parses the JSON)
    let mcap_info = match get_mcap_info(&path).await {
        Ok(c) => c,
        Err(e) => {
            debug!("mcap info failed: {:?}", e);
            McapInfo {
                topics: vec![],
                start_time_ns: None,
                end_time_ns: None,
                duration_seconds: None,
            }
        }
    };

    // look for custom metadata file in same directory as the mcap
    let parent = path
        .parent()
        .ok_or(StorageError::CustomError(
            "MCAP has no parent directory".into(),
        ))?
        .to_path_buf();

    let mut metadata_path: Option<std::path::PathBuf> = None;
    let mut dir = tokio::fs::read_dir(&parent)
        .await
        .map_err(|e| StorageError::IoError(e.into()))?;
    while let Some(entry) = dir
        .next_entry()
        .await
        .map_err(|e| StorageError::IoError(e.into()))?
    {
        let p = entry.path();
        if file_is_custom_metadata(&p).await? {
            metadata_path = Some(p);
            break;
        }
    }
    // debug!("Metadata path for {:?}: {:?}", path, metadata_path);

    // parse metadata yaml if present (for optional metadata)
    let yaml: Option<serde_yaml::Value> = match metadata_path {
        Some(md) => parse_metadata_yaml(&md).await.unwrap_or(None),
        None => None,
    };
    // debug!("Parsed YAML present: {}", yaml.is_some());

    // determine sequence duration: prefer MCAP-derived duration, fall back to YAML
    let sequence_duration: Option<f64> = mcap_info.duration_seconds.or_else(|| {
        yaml.as_ref().and_then(|y| {
            y.get("definitions")
                .and_then(|d| d.get("sequence"))
                .and_then(|s| s.get("duration"))
                .and_then(|v| v.as_f64())
        })
    });

    // collect tags from yaml if present
    let tags: Vec<String> = yaml
        .as_ref()
        .and_then(|y| {
            y.get("definitions")
                .and_then(|d| d.get("sequence"))
                .and_then(|s| s.get("tags"))
        })
        .and_then(|t| {
            t.as_sequence().map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
        })
        .unwrap_or_else(|| vec![]);
    // debug!("Extracted tags: {:?}", tags);

    // file metadata
    let meta = tokio::fs::metadata(&path)
        .await
        .map_err(|e| StorageError::IoError(e.into()))?;
    let size = meta.len() as i64;

    // basic entry construction - many fields are optional and will be filled from yaml when available
    let now = Utc::now();

    // helper closures to extract strings and numbers from yaml
    let yaml_get_string = |y: &serde_yaml::Value, path: &[&str]| -> Option<String> {
        let mut cur = y;
        for p in path.iter() {
            cur = cur.get(*p)?;
        }
        cur.as_str().map(|s| s.to_string())
    };

    let yaml_get_f64 = |y: &serde_yaml::Value, path: &[&str]| -> Option<f64> {
        let mut cur = y;
        for p in path.iter() {
            cur = cur.get(*p)?;
        }
        cur.as_f64()
    };

    let time_machine = yaml.as_ref().and_then(|y| {
        y.get("definitions")
            .and_then(|d| d.get("info"))
            .and_then(|i| i.get("time_machine"))
            .and_then(|v| v.as_f64())
    });

    let platform_name = yaml
        .as_ref()
        .and_then(|y| yaml_get_string(y, &["definitions", "setup", "name"]));
    let platform_image_link = yaml
        .as_ref()
        .and_then(|y| yaml_get_string(y, &["definitions", "setup", "platform_image_link"]));
    let scenario_name = yaml
        .as_ref()
        .and_then(|y| yaml_get_string(y, &["definitions", "scenario", "name"]));

    let scenario_creation_time = yaml.as_ref().and_then(|y| {
        yaml_get_string(y, &["definitions", "sequence", "creation_time_utc"]).and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        })
    });

    let scenario_description = yaml
        .as_ref()
        .and_then(|y| yaml_get_string(y, &["definitions", "scenario", "description"]));

    let sequence_distance = yaml
        .as_ref()
        .and_then(|y| yaml_get_f64(y, &["definitions", "sequence", "distance"]));
    let sequence_lat_starting_point_deg = yaml
        .as_ref()
        .and_then(|y| yaml_get_f64(y, &["definitions", "sequence", "lat_starting_point_deg"]));
    let sequence_lon_starting_point_deg = yaml
        .as_ref()
        .and_then(|y| yaml_get_f64(y, &["definitions", "sequence", "lon_starting_point_deg"]));

    let weather = yaml.as_ref().and_then(|y| {
        y.get("definitions")
            .and_then(|d| d.get("sequence"))
            .and_then(|s| s.get("weather"))
    });

    let weather_cloudiness = weather
        .and_then(|w| w.get("cloudiness"))
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    let weather_precipitation = weather
        .and_then(|w| w.get("precipitation"))
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    let weather_precipitation_deposits = weather
        .and_then(|w| w.get("precipitation_deposits"))
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    let weather_wind_intensity = weather
        .and_then(|w| w.get("wind_intensity"))
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    let weather_road_humidity = weather
        .and_then(|w| w.get("road_humidity"))
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    let weather_fog = weather.and_then(|w| w.get("fog")).and_then(|v| v.as_bool());
    let weather_snow = weather
        .and_then(|w| w.get("snow"))
        .and_then(|v| v.as_bool());

    // check if all fields from mcap info could be read and at least one topic has a message
    let status = if mcap_info.duration_seconds.is_some()
        && mcap_info.start_time_ns.is_some()
        && mcap_info.end_time_ns.is_some()
        && !mcap_info.topics.is_empty()
        && mcap_info.topics.iter().any(|t| t.message_count > 0)
    {
        "Complete"
    } else if !mcap_info.topics.is_empty() {
        "Partial MCAP Info"
    } else {
        "No MCAP Info"
    }
    .to_string();

    let entry = Entry {
        id: 0,
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string()),
        path: path.to_string_lossy().to_string(),
        size,
        created_at: now,
        updated_at: now,
        status,
        time_machine,
        platform_name,
        platform_image_link,
        scenario_name,
        scenario_creation_time,
        scenario_description,
        sequence_duration,
        sequence_distance,
        sequence_lat_starting_point_deg,
        sequence_lon_starting_point_deg,
        weather_cloudiness,
        weather_precipitation,
        weather_precipitation_deposits,
        weather_wind_intensity,
        weather_road_humidity,
        weather_fog,
        weather_snow,
        tags,
        // topics are stored in separate table now
    };
    // debug!(
    //     "Constructed Entry: id=0 name={} path={} size={} tags_count={}",
    //     entry.name,
    //     entry.path,
    //     entry.size,
    //     entry.tags.len()
    // );

    Ok(entry)
}

// Parse a metadata YAML file and return the serde_yaml::Value if successful.
// This function is forgiving: on any IO or parse error it returns Ok(None).
#[instrument]
pub async fn parse_metadata_yaml(path: &Path) -> Result<Option<serde_yaml::Value>, StorageError> {
    if !path.exists() {
        return Ok(None);
    }

    serde_yaml::from_str::<serde_yaml::Value>(&tokio::fs::read_to_string(path).await?)
        .map(Some)
        .map_err(|e| {
            error!("Failed to parse YAML file {:?}: {}", path, e);
            StorageError::CustomError(format!("Failed to parse YAML: {}", e))
        })
}

/// Build entry from an MCAP and insert entry + sequences + sensors into DB.
/// Uses `storage_manager` for DB access. Non-fatal YAML parsing errors are ignored.
#[instrument]
pub async fn insert_entry_into_db(
    storage_manager: &StorageManager,
    path: &Path,
) -> Result<Entry, StorageError> {
    // build Entry from mcap (this is forgiving)
    let mut entry = get_entry_from_mcap(path).await?;

    // determine metadata yaml again (for sequences/sensors)
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut metadata_path: Option<std::path::PathBuf> = None;
    if let Ok(mut dir) = tokio::fs::read_dir(parent).await {
        while let Ok(Some(e)) = dir.next_entry().await {
            let p = e.path();
            if file_is_custom_metadata(&p).await.unwrap_or(false) {
                metadata_path = Some(p);
                break;
            }
        }
    }

    let yaml = match metadata_path {
        Some(p) => parse_metadata_yaml(&p).await.unwrap_or(None),
        None => None,
    };

    // insert entry into DB and get new id (idempotent)
    let txid = storage_manager.start_transaction();

    // Check if entry with same path already exists
    if let Ok(Some(existing)) = storage_manager
        .get_entry_by_path(entry.path.clone(), txid)
        .await
    {
        debug!(
            "Entry with same path already exists with id {}. Updating it.",
            existing.id
        );
        // debug!("Existing entry: {:?}", existing);
        // update metadata for existing entry
        entry.id = existing.id;
        let md = crate::routes::database::MetadataWeb {
            time_machine: entry.time_machine,
            platform_name: entry.platform_name.clone(),
            platform_image_link: entry.platform_image_link.clone(),
            scenario_name: entry.scenario_name.clone(),
            scenario_creation_time: entry.scenario_creation_time,
            scenario_description: entry.scenario_description.clone(),
            sequence_duration: entry.sequence_duration,
            sequence_distance: entry.sequence_distance,
            sequence_lat_starting_point_deg: entry.sequence_lat_starting_point_deg,
            sequence_lon_starting_point_deg: entry.sequence_lon_starting_point_deg,
            weather_cloudiness: entry.weather_cloudiness.clone(),
            weather_precipitation: entry.weather_precipitation.clone(),
            weather_precipitation_deposits: entry.weather_precipitation_deposits.clone(),
            weather_wind_intensity: entry.weather_wind_intensity.clone(),
            weather_road_humidity: entry.weather_road_humidity.clone(),
            weather_fog: entry.weather_fog,
            weather_snow: entry.weather_snow,
            topics: None,
        };
        if let Err(e) = storage_manager.update_entry(entry.id, md, txid).await {
            error!("Failed to update existing entry {}: {:?}", entry.id, e);
        }
        // also ensure tags are present
        for tag in entry.tags.clone().into_iter() {
            if let Err(e) = storage_manager.add_tag(entry.id, tag, txid).await {
                error!("Failed to add tag for entry {}: {:?}", entry.id, e);
            }
        }
        let pool = storage_manager.db_connection_pool();
        let entry_id = entry.id;
        let entry_size = entry.size;
        let entry_updated_at = entry.updated_at;
        let entry_status = entry.status.clone();
        if let Ok(conn2) = pool.get().await {
            if let Err(e) = conn2
                .interact(move |conn| {
                    diesel::update(
                        crate::schema::entries::dsl::entries
                            .filter(crate::schema::entries::dsl::id.eq(entry_id)),
                    )
                    .set((
                        crate::schema::entries::dsl::size.eq(entry_size),
                        crate::schema::entries::dsl::updated_at.eq(entry_updated_at),
                        crate::schema::entries::dsl::status.eq(entry_status),
                    ))
                    .execute(conn)
                })
                .await
            {
                error!(
                    "Failed to update size/updated_at/status for entry {}: {:?}",
                    entry_id, e
                );
            }
        }
    } else {
        // insert new entry (keep previous insertion approach)
        let entry_clone = entry.clone();
        debug!(
            "Inserting entry into DB: name={} path={} size={} tags_count={}",
            entry.name,
            entry.path,
            entry.size,
            entry.tags.len()
        );
        let new_id = storage_manager.add_entry(entry_clone, txid).await?;
        entry.id = new_id;
        // add tags for new entry
        for tag in entry.tags.clone().into_iter() {
            if let Err(e) = storage_manager.add_tag(entry.id, tag, txid).await {
                error!("Failed to add tag for entry {}: {:?}", entry.id, e);
            }
        }
    }

    // insert topics into topics table: run mcap info to get topics and duration
    let mcap_info = match get_mcap_info(path).await {
        Ok(c) => c,
        Err(err) => {
            error!("Failed to get MCAP info for topics: {:?}", err);
            McapInfo {
                topics: vec![],
                start_time_ns: None,
                end_time_ns: None,
                duration_seconds: None,
            }
        }
    };
    let topics_list = mcap_info.topics;
    let sequence_duration: Option<f64> = mcap_info.duration_seconds.or_else(|| {
        yaml.as_ref().and_then(|y| {
            y.get("definitions")
                .and_then(|d| d.get("sequence"))
                .and_then(|s| s.get("duration"))
                .and_then(|v| v.as_f64())
        })
    });
    // upsert topics: update existing topics by name, add new ones
    let existing_topics_map = storage_manager.get_topics(entry.id, txid).await.ok();
    for t in topics_list.iter() {
        let freq = sequence_duration.and_then(|d| {
            if d > 0.0 {
                Some((t.message_count as f64) / d)
            } else {
                None
            }
        });
        let now = Utc::now();
        let topic = crate::storage::models::Topic {
            id: 0,
            entry_id: entry.id,
            topic_name: t.topic.clone(),
            topic_type: t.r#type.clone(),
            message_count: t.message_count as i64,
            frequency: freq,
            created_at: now,
            updated_at: now,
        };
        // try to find existing topic with same name
        if let Some(map) = existing_topics_map.as_ref() {
            let mut found: Option<crate::storage::models::Topic> = None;
            for (_id, existing_topic) in map.iter() {
                if existing_topic.topic_name == topic.topic_name {
                    found = Some(existing_topic.clone());
                    break;
                }
            }
            if let Some(mut et) = found {
                et.topic_type = topic.topic_type.clone();
                et.message_count = topic.message_count;
                et.frequency = topic.frequency;
                et.updated_at = topic.updated_at;
                if let Err(e) = storage_manager.update_topic(et, txid).await {
                    error!("Failed to update topic for entry {}: {:?}", entry.id, e);
                }
                continue;
            }
        }
        if let Err(e) = storage_manager.add_topic(topic, txid).await {
            error!("Failed to add topic for entry {}: {:?}", entry.id, e);
        }
    }
    // insert sequences from YAML: main sequence (if duration present) and subsequences
    if let Some(y) = yaml.as_ref() {
        // main sequence: if there is duration or description
        if let Some(seq_node) = y.get("definitions").and_then(|d| d.get("sequence")) {
            let desc = seq_node
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let duration = seq_node
                .get("duration")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let start_ts = seq_node
                .get("start_time_machine")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let end_ts = if duration > 0.0 {
                start_ts + (duration as i64)
            } else {
                start_ts
            };
            let now = Utc::now();
            let sequence = Sequence {
                id: 0,
                entry_id: entry.id,
                description: desc,
                start_timestamp: start_ts,
                end_timestamp: end_ts,
                created_at: now,
                updated_at: now,
                tags: Vec::new(),
            };
            // upsert main sequence: match by description + timestamps
            let existing_seqs = storage_manager.get_sequences(entry.id, txid).await.ok();
            let mut matched = false;
            if let Some(map) = existing_seqs.as_ref() {
                for (id, es) in map.iter() {
                    if es.description == sequence.description
                        && es.start_timestamp == sequence.start_timestamp
                        && es.end_timestamp == sequence.end_timestamp
                    {
                        let mut seq_to_update = sequence.clone();
                        seq_to_update.id = *id;
                        if let Err(e) = storage_manager
                            .update_sequence(entry.id, *id, seq_to_update, txid)
                            .await
                        {
                            error!("Failed to update sequence for entry {}: {:?}", entry.id, e);
                        }
                        matched = true;
                        break;
                    }
                }
            }
            if !matched {
                if let Err(e) = storage_manager.add_sequence(entry.id, sequence, txid).await {
                    error!("Failed to add sequence for entry {}: {:?}", entry.id, e);
                }
            }
        }

        // subsequences
        if let Some(subs) = y.get("definitions").and_then(|d| d.get("subsequence")) {
            if let Some(arr) = subs.as_sequence() {
                for sub in arr.iter() {
                    let desc = sub
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let start_ts = sub
                        .get("start_time_machine")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    let end_ts = sub.get("end_time").and_then(|v| v.as_i64()).unwrap_or(0);
                    let now = Utc::now();
                    let sequence = Sequence {
                        id: 0,
                        entry_id: entry.id,
                        description: desc,
                        start_timestamp: start_ts,
                        end_timestamp: end_ts,
                        created_at: now,
                        updated_at: now,
                        tags: Vec::new(),
                    };
                    // subsequence upsert: match by description + timestamps
                    let existing_seqs = storage_manager.get_sequences(entry.id, txid).await.ok();
                    let mut matched = false;
                    if let Some(map) = existing_seqs.as_ref() {
                        for (id, es) in map.iter() {
                            if es.description == sequence.description
                                && es.start_timestamp == sequence.start_timestamp
                                && es.end_timestamp == sequence.end_timestamp
                            {
                                let mut seq_to_update = sequence.clone();
                                seq_to_update.id = *id;
                                if let Err(e) = storage_manager
                                    .update_sequence(entry.id, *id, seq_to_update, txid)
                                    .await
                                {
                                    error!(
                                        "Failed to update subsequence for entry {}: {:?}",
                                        entry.id, e
                                    );
                                }
                                matched = true;
                                break;
                            }
                        }
                    }
                    if !matched {
                        if let Err(e) = storage_manager.add_sequence(entry.id, sequence, txid).await
                        {
                            error!("Failed to add subsequence for entry {}: {:?}", entry.id, e);
                        }
                    }
                }
            }
        }

        // sensors
        if let Some(sensors_node) = y.get("definitions").and_then(|d| d.get("sensors")) {
            if let Some(map) = sensors_node.as_mapping() {
                for (k, v) in map.iter() {
                    let sensor_name = k.as_str().unwrap_or("").to_string();
                    let manufacturer = v
                        .get(&serde_yaml::Value::from("manufacturer"))
                        .and_then(|vv| vv.as_str())
                        .map(|s| s.to_string());
                    let sensor_type = v
                        .get(&serde_yaml::Value::from("type"))
                        .and_then(|vv| vv.as_str())
                        .map(|s| s.to_string());
                    let ros_topics = v
                        .get(&serde_yaml::Value::from("ros_topics"))
                        .and_then(|vv| vv.as_sequence())
                        .map(|seq| {
                            seq.iter()
                                .filter_map(|it| it.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_else(|| vec![]);
                    // custom parameters: keep everything except manufacturer/type/ros_topics
                    let mut custom = serde_json::Map::new();
                    if let Some(map_v) = v.as_mapping() {
                        for (kk, vv) in map_v.iter() {
                            if let Some(kstr) = kk.as_str() {
                                if kstr == "manufacturer" || kstr == "type" || kstr == "ros_topics"
                                {
                                    continue;
                                }
                                custom.insert(
                                    kstr.to_string(),
                                    serde_json::to_value(vv).unwrap_or(serde_json::Value::Null),
                                );
                            }
                        }
                    }
                    let custom_parameters = if custom.is_empty() {
                        None
                    } else {
                        Some(serde_json::Value::Object(custom))
                    };
                    let sensor = Sensor {
                        id: 0,
                        entry_id: entry.id,
                        sensor_name,
                        manufacturer,
                        sensor_type,
                        ros_topics,
                        custom_parameters,
                    };
                    // upsert sensor by name
                    let existing_sensors = storage_manager.get_sensors(entry.id, txid).await.ok();
                    let mut matched = false;
                    if let Some(map) = existing_sensors.as_ref() {
                        for (id, es) in map.iter() {
                            if es.sensor_name == sensor.sensor_name {
                                let mut s_to_update = sensor.clone();
                                s_to_update.id = *id;
                                if let Err(e) =
                                    storage_manager.update_sensor(s_to_update, txid).await
                                {
                                    error!(
                                        "Failed to update sensor for entry {}: {:?}",
                                        entry.id, e
                                    );
                                }
                                matched = true;
                                break;
                            }
                        }
                    }
                    if !matched {
                        if let Err(e) = storage_manager.add_sensor(sensor, txid).await {
                            error!("Failed to add sensor for entry {}: {:?}", entry.id, e);
                        }
                    }
                }
            }
        }
    }

    Ok(entry)
}
