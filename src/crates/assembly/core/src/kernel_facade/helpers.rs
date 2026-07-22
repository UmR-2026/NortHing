//! Misc helper functions.

use std::time::Duration;

use tokio::time::error::Elapsed;

/// Returns the default workspace path.
pub(crate) fn default_workspace_path() -> String {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string())
}

/// Returns current Unix timestamp in milliseconds.
pub(crate) fn system_time_to_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Converts a SystemTime to Unix timestamp in milliseconds (i64).
pub(crate) fn system_time_to_ms_i64(t: std::time::SystemTime) -> i64 {
    t.duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Shared first-line error helper for provider test results.
/// Takes first line, trims, caps at 120 chars, falls back to "connection failed" if empty.
pub(crate) fn first_line_error(detail: &str) -> String {
    let first_line = detail.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        "connection failed".to_string()
    } else {
        first_line.chars().take(120).collect()
    }
}

/// Extracts first line, trims, and caps at 120 chars. Returns empty string if input is empty.
pub(crate) fn first_line_truncated(s: &str) -> String {
    s.lines().next().unwrap_or("").trim().chars().take(120).collect()
}

/// Truncates a string to at most 4000 characters (by char count).
pub(crate) fn truncate_4000(s: &str) -> String {
    s.chars().take(4000).collect()
}

/// Extracts a human-readable summary from tool-call params JSON.
/// Tries "command", "path", "file_path", "content", "query" keys in order;
/// falls back to the full params string. Result is first-line truncated to 120 chars.
pub(crate) fn extract_summary_from_params(params: &serde_json::Value) -> String {
    let candidates = ["command", "path", "file_path", "content", "query"];
    for key in candidates {
        if let Some(val) = params.get(key).and_then(|v| v.as_str()) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return first_line_truncated(trimmed);
            }
        }
    }
    first_line_truncated(&params.to_string())
}

/// Maps `MCPServerStatus` to `MCPServerStatusKind` DTO.
pub(crate) fn map_mcp_status_kind(
    status: crate::service::mcp::MCPServerStatus,
) -> northhing_kernel_api::settings::MCPServerStatusKind {
    match status {
        crate::service::mcp::MCPServerStatus::Connected
        | crate::service::mcp::MCPServerStatus::Healthy => {
            northhing_kernel_api::settings::MCPServerStatusKind::Connected
        }
        crate::service::mcp::MCPServerStatus::Starting
        | crate::service::mcp::MCPServerStatus::Uninitialized
        | crate::service::mcp::MCPServerStatus::Reconnecting => {
            northhing_kernel_api::settings::MCPServerStatusKind::Starting
        }
        crate::service::mcp::MCPServerStatus::NeedsAuth => {
            northhing_kernel_api::settings::MCPServerStatusKind::Failed {
                message: "needs authentication".to_string(),
            }
        }
        crate::service::mcp::MCPServerStatus::Failed => {
            northhing_kernel_api::settings::MCPServerStatusKind::Failed {
                message: "runtime reported failure".to_string(),
            }
        }
        crate::service::mcp::MCPServerStatus::Stopping
        | crate::service::mcp::MCPServerStatus::Stopped => {
            northhing_kernel_api::settings::MCPServerStatusKind::Disabled
        }
    }
}

/// Maps a probe result to `MCPServerStatusKind`.
#[allow(clippy::type_complexity)]
pub(crate) fn map_mcp_probe_status(
    probe_status: Result<
        Result<crate::service::mcp::MCPServerStatus, crate::util::errors::NortHingError>,
        Elapsed,
    >,
) -> northhing_kernel_api::settings::MCPServerStatusKind {
    match probe_status {
        Ok(Ok(status)) => map_mcp_status_kind(status),
        Ok(Err(_)) => northhing_kernel_api::settings::MCPServerStatusKind::Failed {
            message: "status probe failed".to_string(),
        },
        Err(_) => northhing_kernel_api::settings::MCPServerStatusKind::ProbeTimeout,
    }
}
