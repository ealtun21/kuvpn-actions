//! Connection event history.
//!
//! Persists connect/disconnect events to a JSON file in the user data directory.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Connected,
    Reconnected,
    Disconnected,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionEvent {
    /// Unix timestamp (seconds) when the event occurred.
    pub timestamp_unix: u64,
    pub kind: EventKind,
    /// Duration in seconds (set on Disconnected / Error).
    pub duration_secs: Option<u64>,
    /// Error message (set on Error).
    pub message: Option<String>,
}

impl ConnectionEvent {
    pub fn now(kind: EventKind) -> Self {
        Self {
            timestamp_unix: now_unix(),
            kind,
            duration_secs: None,
            message: None,
        }
    }
}

pub(crate) fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn history_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(crate::utils::kuvpn_data_dir()?.join("history.json"))
}

/// Appends one event to the on-disk history file.
pub fn append_event(event: &ConnectionEvent) -> Result<(), Box<dyn std::error::Error>> {
    let path = history_path()?;
    let mut events: Vec<ConnectionEvent> = if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };
    events.push(event.clone());
    std::fs::write(&path, serde_json::to_string_pretty(&events)?)?;
    Ok(())
}

/// Loads all events from the on-disk history file.
pub fn load_events() -> Result<Vec<ConnectionEvent>, Box<dyn std::error::Error>> {
    let path = history_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content).unwrap_or_default())
}

/// Removes the history file.
pub fn clear_events() -> Result<(), Box<dyn std::error::Error>> {
    let path = history_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}
