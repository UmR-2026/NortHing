//! Remote-connect workspace/path resolver helpers (Round 11 split).
//!
//! Owns the path/mime/agent-type `resolve_remote_*` helpers used by the
//! remote-connect request_builders / file_io / session_tracker siblings.
//! Pure path/mime helpers stay private; only the public `resolve_remote_*`
//! surface is part of the module API. `resolve_remote_cancel_decision` lives
//! in `command_handlers` next to the cancel types it returns.

use std::path::{Path, PathBuf};

pub const REMOTE_FILE_MAX_READ_BYTES: u64 = 30 * 1024 * 1024;
pub const REMOTE_FILE_MAX_CHUNK_BYTES: u64 = 3 * 1024 * 1024;

pub fn resolve_remote_file_chunk_range(
    file_len: usize,
    offset: u64,
    limit: u64,
) -> northhing_runtime_ports::RemoteFileChunkRange {
    let actual_limit = limit.min(REMOTE_FILE_MAX_CHUNK_BYTES);
    let start = (offset as usize).min(file_len);
    let end = start.saturating_add(actual_limit as usize).min(file_len);

    northhing_runtime_ports::RemoteFileChunkRange {
        start,
        end,
        chunk_size: (end - start) as u64,
    }
}

fn strip_remote_workspace_path_prefix(raw: &str) -> &str {
    raw.strip_prefix("computer://")
        .or_else(|| raw.strip_prefix("file://"))
        .unwrap_or(raw)
}

fn is_remote_absolute_workspace_path(path: &str) -> bool {
    path.starts_with('/') || (path.len() >= 3 && path.as_bytes()[1] == b':')
}

pub fn resolve_remote_workspace_path(raw: &str, workspace_root: Option<&Path>) -> Option<PathBuf> {
    let stripped = strip_remote_workspace_path_prefix(raw);

    if is_remote_absolute_workspace_path(stripped) {
        return Some(PathBuf::from(stripped));
    }

    let workspace_root = workspace_root?;
    let canonical_root = std::fs::canonicalize(workspace_root).ok()?;
    let candidate = canonical_root.join(stripped);
    let canonical_candidate = std::fs::canonicalize(candidate).ok()?;

    if canonical_candidate.starts_with(&canonical_root) {
        Some(canonical_candidate)
    } else {
        None
    }
}

pub fn detect_remote_mime_type(path: &Path) -> &'static str {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    match ext.as_str() {
        "txt" | "log" => "text/plain",
        "md" => "text/markdown",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "text/javascript",
        "ts" | "tsx" | "jsx" | "rs" | "py" | "go" | "java" | "c" | "cpp" | "h" | "sh" | "toml" | "yaml" | "yml" => {
            "text/plain"
        }
        "json" => "application/json",
        "xml" => "application/xml",
        "csv" => "text/csv",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "zip" => "application/zip",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "mp4" => "video/mp4",
        "opus" => "audio/opus",
        _ => "application/octet-stream",
    }
}

pub fn resolve_remote_agent_type(mobile_type: Option<&str>) -> &'static str {
    match mobile_type {
        Some("code") | Some("agentic") | Some("Agentic") => "agentic",
        Some("multitask") | Some("Multitask") => "Multitask",
        Some("cowork") | Some("Cowork") => "Cowork",
        Some("plan") | Some("Plan") => "Plan",
        Some("debug") | Some("Debug") => "debug",
        _ => "agentic",
    }
}
