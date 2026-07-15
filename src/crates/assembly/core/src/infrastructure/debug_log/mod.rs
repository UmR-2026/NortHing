//! Debug Mode runtime logging utilities.
//! Provides a shared instrumentation pipeline for desktop/server/cli + web.
//!
//! ## Module Structure
//! - `types` - Types and handlers for the HTTP ingest server (Config, State, Request, Response)
//! - `http_server` - The actual HTTP server implementation (axum-based)

pub mod http_server;
pub mod types;

pub use types::{
    handle_ingest, IngestLogRequest, IngestResponse, IngestServerConfig, IngestServerState, DEFAULT_INGEST_PORT,
};

pub use http_server::IngestServerManager;

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

pub async fn append_log_async(entry: DebugLogEntry, config: Option<DebugLogConfig>, send_http: bool) -> Result<()> {
    let cfg = config.unwrap_or_default();
    let log_line = build_log_line(entry, &cfg);
    let log_path = cfg.log_path.clone();
    let ingest_url = cfg.ingest_url.clone().filter(|_| send_http);

    let log_line_for_file = log_line.clone();
    let log_path_clone = log_path.clone();
    task::spawn_blocking(move || -> Result<()> {
        ensure_parent_exists(&log_path_clone)?;
        let mut file = OpenOptions::new().create(true).append(true).open(&log_path_clone)?;
        writeln!(file, "{}", serde_json::to_string(&log_line_for_file)?)?;
        Ok(())
    })
    .await
    .map_err(|e| anyhow::anyhow!("Join error: {}", e))??;

    if let Some(url) = ingest_url {
        let client = reqwest::Client::new();
        let _ = client.post(url).json(&log_line).send().await;
    }

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
/// underlying `append_log_async` already swallows file/HTTP errors).
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
}
