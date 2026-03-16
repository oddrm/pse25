use rocket::get;
use rocket::serde::json::Json;
use serde::Serialize;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub location: Option<String>,
}

#[get("/logs?<level>&<limit>")]
pub fn get_logs(level: Option<String>, limit: Option<usize>) -> Json<Vec<LogEntry>> {
    let log_dir = "/logs";
    let mut entries = Vec::new();

    let path = Path::new(log_dir);
    if !path.exists() {
        return Json(entries);
    }

    let mut files: Vec<_> = fs::read_dir(path)
        .ok()
        .map(|dir| {
            dir.filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_file())
                .collect()
        })
        .unwrap_or_default();

    files.sort_by_key(|f| f.file_name());
    let files_count = files.len();
    let files_to_process = files.into_iter().skip(files_count.saturating_sub(3));

    let limit = limit.unwrap_or(100);
    let level_filter = level.map(|l| l.to_uppercase());


    let mut all_entries = Vec::new();

    for entry in files_to_process {
        if let Ok(file) = fs::File::open(entry.path()) {
            let reader = BufReader::new(file);
            let lines = reader.lines().filter_map(|l| l.ok());

            let mut current_entry: Option<LogEntry> = None;

            for line in lines {
                if let Some(log_entry) = parse_log_line(&line) {
                    // New log entry started
                    if let Some(completed) = current_entry.take() {
                        if level_matches(&completed, &level_filter) && !is_noise(&completed) {
                            all_entries.push(completed);
                        }
                    }
                    current_entry = Some(log_entry);
                } else if let Some(ref mut entry) = current_entry {
                    // This is a continuation line
                    entry.message.push('\n');
                    entry.message.push_str(line.trim_end());
                }
            }
            if let Some(completed) = current_entry {
                if level_matches(&completed, &level_filter) && !is_noise(&completed) {
                    all_entries.push(completed);
                }
            }
        }
    }

    // Newest first
    all_entries.reverse();
    entries = all_entries.into_iter().take(limit).collect();

    Json(entries)
}

fn is_noise(entry: &LogEntry) -> bool {
    // Filter out logs related to the log-fetching route itself to avoid recursive clutter
    let msg = &entry.message;

    // Check if it's from known noisy targets
    if msg.contains("hyper") {
        return true;
    }

    if msg.contains("rocket") {
        if entry.level == "DEBUG" {
            return true;
        }
    }

    if msg.contains("/logs")
        || msg.contains("(get_logs)")
        || msg.contains("Response succeeded")
        || msg.contains("Outcome: Success")
        || msg.contains("hyper::proto::h1::io: flushed")
    {
        return true;
    }

    false
}

fn level_to_val(level: &str) -> u8 {
    match level.to_uppercase().as_str() {
        "ERROR" => 1,
        "WARN" => 2,
        "INFO" => 3,
        "DEBUG" => 4,
        "TRACE" => 5,
        _ => 0,
    }
}

fn level_matches(entry: &LogEntry, filter: &Option<String>) -> bool {
    if let Some(f) = filter {
        let entry_val = level_to_val(&entry.level);
        let filter_val = level_to_val(f);
        entry_val <= filter_val
    } else {
        true
    }
}

fn parse_log_line(line: &str) -> Option<LogEntry> {

    if line.len() < 30 || !line.as_bytes()[0].is_ascii_digit() {
        return None;
    }

    let first_space = line.find(' ')?;
    let timestamp = line[..first_space].to_string();

    let remaining = line[first_space..].trim_start();
    let next_space = remaining.find(' ')?;
    let level = remaining[..next_space].to_string();

    let is_valid_level = matches!(
        level.to_uppercase().as_str(),
        "ERROR" | "WARN" | "INFO" | "DEBUG" | "TRACE"
    );
    if !is_valid_level {
        return None;
    }

    let mut message = remaining[next_space..].trim_start();

    // Strip ThreadId if present to keep the message cleaner
    if message.starts_with("ThreadId") {
        if let Some(after_tid) = message.find(' ') {
            message = message[after_tid..].trim_start();
        }
    }

    let mut location = None;

    // Tracing with with_file(true) usually looks like: path/to/file.rs:123: message
    // We look for the first colon that is followed by a space
    if let Some(first_colon_space) = message.find(": ") {
        let prefix = &message[..first_colon_space];
        // Heuristic to check if this prefix looks like a file location (has .rs and :)
        if prefix.contains(".rs") && prefix.contains(':') {
            location = Some(prefix.to_string());
            message = message[first_colon_space + 2..].trim_start();
        }
    }

    Some(LogEntry {
        timestamp,
        level,
        message: message.to_string(),
        location,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_log_line_extracts_timestamp_level_message_and_location() {
        let parsed =
            parse_log_line("2026-03-16T19:00:00Z INFO src/main.rs:42: service started").unwrap();

        assert_eq!(parsed.timestamp, "2026-03-16T19:00:00Z");
        assert_eq!(parsed.level, "INFO");
        assert_eq!(parsed.location.as_deref(), Some("src/main.rs:42"));
        assert_eq!(parsed.message, "service started");
    }

    #[test]
    fn parse_log_line_strips_thread_id_prefix() {
        let parsed =
            parse_log_line("2026-03-16T19:00:00Z DEBUG ThreadId(7) worker loop started").unwrap();

        assert_eq!(parsed.level, "DEBUG");
        assert_eq!(parsed.location, None);
        assert_eq!(parsed.message, "worker loop started");
    }

    #[test]
    fn parse_log_line_rejects_short_or_invalid_lines() {
        assert!(parse_log_line("short").is_none());
        assert!(parse_log_line("not-a-date INFO message").is_none());
        assert!(parse_log_line("2026-03-16T19:00:00Z OTHER unknown").is_none());
    }

    #[test]
    fn level_to_val_orders_known_levels_and_unknowns() {
        assert_eq!(level_to_val("ERROR"), 1);
        assert_eq!(level_to_val("WARN"), 2);
        assert_eq!(level_to_val("INFO"), 3);
        assert_eq!(level_to_val("DEBUG"), 4);
        assert_eq!(level_to_val("TRACE"), 5);
        assert_eq!(level_to_val("unknown"), 0);
    }

    #[test]
    fn level_matches_obeys_filter_threshold() {
        let entry = LogEntry {
            timestamp: "2026-03-16T19:00:00Z".to_string(),
            level: "INFO".to_string(),
            message: "message".to_string(),
            location: None,
        };

        assert!(level_matches(&entry, &None));
        assert!(level_matches(&entry, &Some("INFO".to_string())));
        assert!(level_matches(&entry, &Some("DEBUG".to_string())));
        assert!(!level_matches(&entry, &Some("WARN".to_string())));
    }

    #[test]
    fn is_noise_filters_known_log_fetch_noise() {
        let hyper = LogEntry {
            timestamp: "t".to_string(),
            level: "INFO".to_string(),
            message: "hyper request".to_string(),
            location: None,
        };
        let rocket_debug = LogEntry {
            timestamp: "t".to_string(),
            level: "DEBUG".to_string(),
            message: "rocket internal".to_string(),
            location: None,
        };
        let route_noise = LogEntry {
            timestamp: "t".to_string(),
            level: "INFO".to_string(),
            message: "GET /logs Response succeeded".to_string(),
            location: None,
        };
        let normal = LogEntry {
            timestamp: "t".to_string(),
            level: "INFO".to_string(),
            message: "application message".to_string(),
            location: None,
        };

        assert!(is_noise(&hyper));
        assert!(is_noise(&rocket_debug));
        assert!(is_noise(&route_noise));
        assert!(!is_noise(&normal));
    }
}
