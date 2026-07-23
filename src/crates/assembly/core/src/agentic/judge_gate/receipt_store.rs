//! Persistent consumed-receipt store (append-only JSONL).
//!
//! Crash-safe: file is source of truth on restart. Each line records a
//! consume or release action; startup replays the log to rebuild the
//! in-memory set.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use tracing::{debug, warn};

const RECEIPT_LOG_FILENAME: &str = "consumed_receipts.jsonl";

#[derive(Debug, Serialize, Deserialize)]
struct ReceiptAction {
    receipt_id: String,
    action: String, // "consumed" | "released"
    ts: u64,
}

/// Get the path to the consumed receipts log file.
pub(crate) fn receipt_log_path() -> Option<PathBuf> {
    let pm = crate::infrastructure::app_paths::path_manager_arc();
    let dir = pm.user_data_dir().join("judge-gate");
    Some(dir.join(RECEIPT_LOG_FILENAME))
}

/// Load consumed receipts from the append-only log.
/// Replays consumed/released actions to rebuild the set.
pub(crate) fn load_consumed_receipts() -> HashSet<String> {
    let mut set = HashSet::new();
    let Some(path) = receipt_log_path() else {
        return set;
    };
    if !path.exists() {
        return set;
    }
    let Ok(file) = std::fs::File::open(&path) else {
        return set;
    };
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<ReceiptAction>(trimmed) {
            Ok(entry) => match entry.action.as_str() {
                "consumed" => {
                    set.insert(entry.receipt_id);
                }
                "released" => {
                    set.remove(&entry.receipt_id);
                }
                _ => {}
            },
            Err(e) => {
                warn!(error = %e, "skipping malformed consumed_receipts line");
            }
        }
    }
    debug!(count = set.len(), "loaded consumed receipts from disk");
    set
}

/// Append a receipt action to the log file (best-effort, non-fatal on failure).
pub(crate) fn persist_receipt_action(receipt_id: &str, action: &str) {
    let Some(path) = receipt_log_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let entry = ReceiptAction {
        receipt_id: receipt_id.to_string(),
        action: action.to_string(),
        ts: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    };
    let Ok(json) = serde_json::to_string(&entry) else {
        return;
    };
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(mut file) => {
            let _ = writeln!(file, "{}", json);
        }
        Err(e) => {
            warn!(error = %e, "failed to persist receipt action");
        }
    }
}
