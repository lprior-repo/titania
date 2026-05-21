//! JSONL structured logging per crate/lane.
//!
//! Writes per-run logs to target/xtask-proof/&lt;run-id&gt;/&lt;crate&gt;/&lt;lane&gt;.jsonl

use chrono::Utc;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct LaneLogEntry {
    pub crate_name: String,
    pub lane: String,
    pub command: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub status: String,
    pub timestamp: String,
}

pub struct RunLogger {
    pub run_id: String,
    pub base_dir: PathBuf,
}

impl RunLogger {
    pub fn new(run_id: &str) -> Self {
        let base_dir = PathBuf::from("target").join("xtask-proof").join(run_id);
        RunLogger {
            run_id: run_id.to_string(),
            base_dir,
        }
    }

    pub fn log_entry(
        &self,
        crate_name: &str,
        lane: &str,
        command: &str,
        exit_code: Option<i32>,
        duration_ms: u64,
        status: &str,
    ) -> anyhow::Result<()> {
        let entry = LaneLogEntry {
            crate_name: crate_name.to_string(),
            lane: lane.to_string(),
            command: command.to_string(),
            exit_code,
            duration_ms,
            status: status.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };

        let dir = self.base_dir.join(crate_name);
        fs::create_dir_all(&dir)?;

        let file_path = dir.join(format!("{lane}.jsonl"));
        let json = serde_json::to_string(&entry)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;
        writeln!(file, "{json}")?;

        Ok(())
    }
}

pub fn generate_run_id() -> String {
    let now = Utc::now();
    format!("{}", now.format("%Y%m%d-%H%M%S"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_serialization() {
        let entry = LaneLogEntry {
            crate_name: "vb_core".to_string(),
            lane: "test".to_string(),
            command: "cargo test -p vb_core".to_string(),
            exit_code: Some(0),
            duration_ms: 1234,
            status: "pass".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(json.contains("vb_core"));
        assert!(json.contains("test"));
        assert!(json.contains("pass"));
    }

    #[test]
    fn test_generate_run_id() {
        let id = generate_run_id();
        assert!(!id.is_empty());
        assert!(id.len() > 10);
    }
}
