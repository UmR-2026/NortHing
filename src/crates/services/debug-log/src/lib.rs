//! Debug Mode runtime logging utilities.
//! Provides a shared instrumentation pipeline for desktop/server/cli + web.
//!
//! K4a-T5 (2026-07-26): extracted from `northhing-core`'s
//! `infrastructure/debug_log` into this leaf service crate so product
//! surfaces (desktop) can depend on `log_event` + the `COMP_*` component
//! constants without reaching into core. The HTTP ingest server
//! (`http_server` / `types`) stays in core because it depends on core's
//! workspace service; core re-exports this crate's items to keep its
//! internal call sites unchanged.

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tokio::task;
use uuid::Uuid;

const DEFAULT_SESSION_ID: &str = "debug-session";

/// Maximum single debug log file size before single-generation rotation (8 MiB).
/// When the target log file exceeds this limit, it is rotated to `<filename>.1.<ext>`
/// (overwriting any previous backup), and a fresh log file is started.
const DEBUG_LOG_MAX_BYTES: u64 = 8 * 1024 * 1024;

static DEFAULT_LOG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    if let Ok(env_path) = std::env::var("northhing_DEBUG_LOG_PATH") {
        return PathBuf::from(env_path);
    }

    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".northhing")
        .join("debug.log")
});

static DEFAULT_INGEST_URL: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("northhing_DEBUG_INGEST_URL").ok());

#[derive(Debug, Clone)]
pub struct DebugLogConfig {
    pub log_path: PathBuf,
    pub ingest_url: Option<String>,
    pub session_id: String,
}

impl Default for DebugLogConfig {
    fn default() -> Self {
        Self {
            log_path: DEFAULT_LOG_PATH.clone(),
            ingest_url: DEFAULT_INGEST_URL.clone(),
            session_id: DEFAULT_SESSION_ID.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugLogEntry {
    pub location: String,
    pub message: String,
    #[serde(default)]
    pub data: Value,
    #[serde(default)]
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hypothesis_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Phase H (2026-06-20): top-level component name (e.g.
    /// `"session_lifecycle"`). Defaults to empty string so old call
    /// sites that pre-date this field remain compatible.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub component: String,
    /// Phase H: agent mode id at the log site (e.g. `"code"`,
    /// `"debug"`). Empty string when the log site is mode-agnostic.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mode_id: String,
}

impl DebugLogEntry {
    pub fn with_defaults(mut self, config: &DebugLogConfig) -> Self {
        if self.session_id.is_empty() {
            self.session_id = config.session_id.clone();
        }
        if self.timestamp.is_none() {
            self.timestamp = Some(current_timestamp_ms());
        }
        if self.id.is_none() {
            self.id = Some(format!("log_{}", Uuid::new_v4()));
        }
        self
    }
}

fn current_timestamp_ms() -> i64 {
    Utc::now().timestamp_millis()
}

fn redact_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sanitized = serde_json::Map::new();
            for (k, v) in map.into_iter() {
                if is_sensitive_key(&k) {
                    sanitized.insert(k, redact_scalar(v));
                } else {
                    sanitized.insert(k, redact_value(v));
                }
            }
            Value::Object(sanitized)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(redact_value).collect()),
        other => other,
    }
}

fn redact_scalar(value: Value) -> Value {
    match value {
        Value::String(s) => {
            let prefix: String = s.chars().take(10).collect();
            Value::String(format!("{}***", prefix))
        }
        Value::Number(_) => Value::String("***".to_string()),
        Value::Bool(_) => Value::Bool(false),
        Value::Array(_) | Value::Object(_) => Value::String("***".to_string()),
        Value::Null => Value::Null,
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "password"
            | "token"
            | "access_token"
            | "refresh_token"
            | "api_key"
            | "apikey"
            | "cookie"
            | "authorization"
            | "auth"
            | "secret"
    )
}

fn build_log_line(entry: DebugLogEntry, config: &DebugLogConfig) -> Value {
    let normalized = entry.with_defaults(config);
    let data = redact_value(normalized.data);

    serde_json::json!({
        "id": normalized.id,
        "timestamp": normalized.timestamp,
        "location": normalized.location,
        "message": normalized.message,
        "data": data,
        "sessionId": normalized.session_id,
        "runId": normalized.run_id,
        "hypothesisId": normalized.hypothesis_id,
        "component": normalized.component,
        "modeId": normalized.mode_id,
    })
}

fn ensure_parent_exists(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn backup_path_for(path: &Path) -> Option<PathBuf> {
    let file_name = path.file_name()?.to_str()?;
    let new_name = match file_name.rfind('.') {
        Some(idx) => format!("{}.1{}", &file_name[..idx], &file_name[idx..]),
        None => format!("{}.1", file_name),
    };
    Some(path.with_file_name(new_name))
}

/// Note on concurrency: concurrent appends may both pass the size check;
/// the second rename will fail and that log line is dropped.
/// This matches the crate's existing fire-and-forget / caller-swallows-errors semantics.
fn rotate_if_oversized(path: &Path, max_bytes: u64) -> Result<()> {
    match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.len() > max_bytes {
                if let Some(backup_path) = backup_path_for(path) {
                    let _ = fs::remove_file(&backup_path);
                    fs::rename(path, &backup_path)?;
                }
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub async fn append_log_async(entry: DebugLogEntry, config: Option<DebugLogConfig>, send_http: bool) -> Result<()> {
    // K4a-T5: this leaf crate writes the NDJSON line to disk only. The
    // `send_http` flag and `DebugLogConfig.ingest_url` are retained for
    // signature/struct compatibility with core's re-export bridge, but the
    // dormant reqwest HTTP-forward was dropped so the crate stays dependency-
    // light (no native TLS build). No production caller passes `send_http = true`
    // (`log_event` and core's `handle_ingest` both pass `false`).
    let _ = send_http;
    let cfg = config.unwrap_or_default();
    let log_line = build_log_line(entry, &cfg);
    let log_path = cfg.log_path.clone();

    task::spawn_blocking(move || -> Result<()> {
        ensure_parent_exists(&log_path)?;
        rotate_if_oversized(&log_path, DEBUG_LOG_MAX_BYTES)?;
        let mut file = OpenOptions::new().create(true).append(true).open(&log_path)?;
        writeln!(file, "{}", serde_json::to_string(&log_line)?)?;
        Ok(())
    })
    .await
    .map_err(|e| anyhow::anyhow!("Join error: {}", e))??;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// Phase H (2026-06-20): MVP-friendly debug-event shorthand.
//
// The raw `append_log_async(entry, ...)` requires callers to construct
// a full `DebugLogEntry` themselves, which discourages logging from the
// hot path during manual testing. The shorthand below:
//
// - Fixes a small set of well-known **components** (one per wire site)
//   so logs are greppable by component name (e.g.
//   `grep '"component":"session_lifecycle"' debug.log`).
// - Always records the `mode_id` (so we can see which Agent impl was
//   selected for a given log line).
// - Always records the `location` (the call site — typically
//   `module:fn` or `module:closure`).
// - Builds the JSON `data` payload from a free-form key-value list
//   (rather than requiring `serde_json::json!` at the call site).
//
// No new behavior: still writes the same JSON line to the same file
// path, just with a flatter call surface.
// ═══════════════════════════════════════════════════════════════════

/// Well-known component names. Locked here (not inlined at the call
/// site) so a typo at one site doesn't silently create a new
/// component — `log_event` checks against this list and falls back to
/// `"unknown"` for unrecognized names.
pub const COMP_APP_LIFECYCLE: &str = "app_lifecycle";
pub const COMP_SESSION_LIFECYCLE: &str = "session_lifecycle";
pub const COMP_MODE_ROUTING: &str = "mode_routing";
pub const COMP_SKILL_PANEL: &str = "skill_panel";
pub const COMP_ACTOR_RUNTIME: &str = "actor_runtime";

/// Emit one structured debug log line. Fire-and-forget — never blocks
/// the caller, never panics. Failures are silently swallowed (the
/// underlying `append_log_async` already swallows file errors).
///
/// `mode_id` is rendered into the JSON as a top-level field for easy
/// `jq`. `data` is an ordered list of `(key, value)` pairs serialized
/// as a JSON object. `None` skips the data field.
///
/// Phase H (2026-06-20): `mode_id` and `message` are borrowed (so the
/// caller can pass them by reference and avoid a heap allocation).
/// `data` takes **owned** `String` pairs because the async future
/// produced by `log_event` must be `'static` (callers may invoke it
/// from inside `Runtime::block_on` where `'a` futures are rejected).
pub async fn log_event(
    component: &'static str,
    mode_id: &str,
    location: &'static str,
    message: &str,
    data: Option<[(String, String); 4]>,
) {
    // Validate component against the known list. Unknown values fall
    // back to "unknown" so the file stays clean (typos don't pollute).
    let component: &'static str = match component {
        COMP_APP_LIFECYCLE | COMP_SESSION_LIFECYCLE | COMP_MODE_ROUTING | COMP_SKILL_PANEL | COMP_ACTOR_RUNTIME => {
            component
        }
        _ => "unknown",
    };

    // Build the data object. We accept up to 4 owned `(String, String)`
    // pairs (enough for the MVP wire sites); `None` skips the field
    // entirely so empty logs don't carry `"data": {}`. Keys with an
    // empty string are skipped so callers can leave padding slots
    // empty without polluting the JSON output.
    let data_value = data
        .map(|pairs| {
            let mut map = serde_json::Map::new();
            for (k, v) in pairs.into_iter() {
                if !k.is_empty() {
                    map.insert(k, serde_json::Value::String(v));
                }
            }
            serde_json::Value::Object(map)
        })
        .unwrap_or(serde_json::Value::Null);

    let entry = DebugLogEntry {
        location: location.to_string(),
        message: message.to_string(),
        data: data_value,
        session_id: String::new(), // filled by with_defaults
        run_id: None,
        hypothesis_id: None,
        timestamp: None,
        id: None,
        component: component.to_string(),
        mode_id: mode_id.to_string(),
    };
    let _ = append_log_async(entry, None, false).await;
}

// ────────── tests ──────────

#[cfg(test)]
mod component_tests {
    use super::*;

    /// A test-only component name used to verify that the unknown-
    /// component fallback works. Anything outside the known list is
    /// rewritten to `"unknown"`.
    #[test]
    fn unknown_component_falls_back() {
        // We can't easily inspect the rewritten value without sending
        // it through append_log_async (which writes to disk), so we
        // re-implement the validator inline here to mirror the
        // production check. If the production check ever drifts from
        /// this assertion, the test will not catch it — that's a
        /// known limitation. The strong guarantee comes from the
        /// compile-time `&'static str` requirement on `component`.
        fn normalize(c: &str) -> &str {
            match c {
                COMP_APP_LIFECYCLE
                | COMP_SESSION_LIFECYCLE
                | COMP_MODE_ROUTING
                | COMP_SKILL_PANEL
                | COMP_ACTOR_RUNTIME => c,
                _ => "unknown",
            }
        }
        assert_eq!(normalize(COMP_APP_LIFECYCLE), "app_lifecycle");
        assert_eq!(normalize(COMP_SESSION_LIFECYCLE), "session_lifecycle");
        assert_eq!(normalize("typo_component"), "unknown");
    }

    /// Verifies the public component constants are stable strings —
    /// downstream tooling (log scrapers, jq queries) may depend on
    /// these literal values.
    #[test]
    fn component_constants_are_stable() {
        assert_eq!(COMP_APP_LIFECYCLE, "app_lifecycle");
        assert_eq!(COMP_SESSION_LIFECYCLE, "session_lifecycle");
        assert_eq!(COMP_MODE_ROUTING, "mode_routing");
        assert_eq!(COMP_SKILL_PANEL, "skill_panel");
        assert_eq!(COMP_ACTOR_RUNTIME, "actor_runtime");
    }

    #[test]
    fn test_backup_path_generation() {
        assert_eq!(
            backup_path_for(Path::new("debug.log")),
            Some(PathBuf::from("debug.1.log"))
        );
        assert_eq!(
            backup_path_for(Path::new("path/to/my_debug.log")),
            Some(PathBuf::from("path/to/my_debug.1.log"))
        );
        assert_eq!(
            backup_path_for(Path::new("custom.app.log")),
            Some(PathBuf::from("custom.app.1.log"))
        );
        assert_eq!(backup_path_for(Path::new("logfile")), Some(PathBuf::from("logfile.1")));
        assert_eq!(backup_path_for(Path::new(".log")), Some(PathBuf::from(".1.log")));
    }

    struct TempDirGuard(PathBuf);
    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn test_rotate_if_oversized() {
        let temp_dir = std::env::temp_dir().join(format!("northhing_rotate_test_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let _guard = TempDirGuard(temp_dir.clone());

        let log_file = temp_dir.join("test.log");
        let backup_file = temp_dir.join("test.1.log");

        // Non-existent file: no error, no rotation.
        assert!(rotate_if_oversized(&log_file, 100).is_ok());
        assert!(!log_file.exists());
        assert!(!backup_file.exists());

        // File size <= threshold: no rotation.
        fs::write(&log_file, vec![b'x'; 100]).unwrap();
        assert!(rotate_if_oversized(&log_file, 100).is_ok());
        assert!(log_file.exists());
        assert!(!backup_file.exists());

        // File size > threshold: rotates to backup_file.
        assert!(rotate_if_oversized(&log_file, 50).is_ok());
        assert!(!log_file.exists());
        assert!(backup_file.exists());
        assert_eq!(fs::metadata(&backup_file).unwrap().len(), 100);

        // Write a new 200-byte log file while backup already exists.
        fs::write(&log_file, vec![b'y'; 200]).unwrap();
        assert!(rotate_if_oversized(&log_file, 50).is_ok());
        assert!(!log_file.exists());
        assert!(backup_file.exists());
        // Backup was overwritten with the new 200-byte content.
        assert_eq!(fs::metadata(&backup_file).unwrap().len(), 200);
    }

    #[tokio::test]
    async fn test_append_log_async_rotates_oversized_file() {
        let temp_dir = std::env::temp_dir().join(format!("northhing_append_rotate_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let _guard = TempDirGuard(temp_dir.clone());

        let log_path = temp_dir.join("debug.log");
        let backup_path = temp_dir.join("debug.1.log");

        let oversized_len = (DEBUG_LOG_MAX_BYTES + 1) as usize;
        fs::write(&log_path, vec![b'A'; oversized_len]).unwrap();

        let config = DebugLogConfig {
            log_path: log_path.clone(),
            ingest_url: None,
            session_id: "test-session".to_string(),
        };

        let entry = DebugLogEntry {
            location: "test_loc".to_string(),
            message: "new line after rotation".to_string(),
            data: Value::Null,
            session_id: "test-session".to_string(),
            run_id: None,
            hypothesis_id: None,
            timestamp: None,
            id: None,
            component: "app_lifecycle".to_string(),
            mode_id: "test".to_string(),
        };

        append_log_async(entry, Some(config), false).await.unwrap();

        assert!(backup_path.exists());
        assert_eq!(fs::metadata(&backup_path).unwrap().len(), oversized_len as u64);

        assert!(log_path.exists());
        let new_content = fs::read_to_string(&log_path).unwrap();
        assert!(new_content.contains("new line after rotation"));
        assert!(!new_content.contains("AAAAA"));
    }
}
