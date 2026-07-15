//! Remote-connect file IO helpers (Round 11 split).
//!
//! Owns the `read_remote_*` and `remote_file_*` prefix cluster. The
//! `handle_remote_workspace_file_command` dispatcher lives in
//! `command_handlers` since `handle_remote_*` prefix is owned there, but it
//! composes the readers + response builders in this module.

use std::path::Path;

use base64::Engine as _;

use super::remote_workspace_resolver::{
    detect_remote_mime_type, resolve_remote_file_chunk_range, resolve_remote_workspace_path,
};
use super::RemoteResponse;
use northhing_runtime_ports::{RemoteWorkspaceFileChunk, RemoteWorkspaceFileContent, RemoteWorkspaceFileInfo};

pub fn remote_file_display_name(name: Option<&str>) -> String {
    match name {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => "file".to_string(),
    }
}

pub async fn read_remote_workspace_file(
    raw_path: &str,
    max_size: u64,
    workspace_root: Option<&Path>,
) -> Result<RemoteWorkspaceFileContent, String> {
    let abs_path = resolve_remote_workspace_path(raw_path, workspace_root)
        .ok_or_else(|| format!("Remote file path could not be resolved: {raw_path}"))?;

    if !abs_path.exists() {
        return Err(format!("File not found: {}", abs_path.display()));
    }
    if !abs_path.is_file() {
        return Err(format!("Path is not a regular file: {}", abs_path.display()));
    }

    let metadata = tokio::fs::metadata(&abs_path)
        .await
        .map_err(|e| format!("Cannot read file metadata for {}: {e}", abs_path.display()))?;

    if metadata.len() > max_size {
        return Err(format!(
            "File too large ({} bytes, limit {max_size} bytes): {}",
            metadata.len(),
            abs_path.display()
        ));
    }

    let bytes = tokio::fs::read(&abs_path)
        .await
        .map_err(|e| format!("Cannot read file {}: {e}", abs_path.display()))?;

    Ok(RemoteWorkspaceFileContent {
        name: remote_file_display_name(abs_path.file_name().and_then(|n| n.to_str())),
        bytes,
        mime_type: detect_remote_mime_type(&abs_path),
        size: metadata.len(),
    })
}

pub async fn read_remote_workspace_file_chunk(
    raw_path: &str,
    workspace_root: Option<&Path>,
    offset: u64,
    limit: u64,
) -> Result<RemoteWorkspaceFileChunk, String> {
    let abs_path = resolve_remote_workspace_path(raw_path, workspace_root)
        .ok_or_else(|| format!("Remote file path could not be resolved: {raw_path}"))?;

    if !abs_path.exists() || !abs_path.is_file() {
        return Err(format!("File not found or not a regular file: {}", abs_path.display()));
    }

    let total_size = tokio::fs::metadata(&abs_path)
        .await
        .map_err(|e| format!("Cannot read file metadata: {e}"))?
        .len();

    let bytes = tokio::fs::read(&abs_path)
        .await
        .map_err(|e| format!("Cannot read file: {e}"))?;
    let range = resolve_remote_file_chunk_range(bytes.len(), offset, limit);
    let chunk = bytes[range.start..range.end].to_vec();

    Ok(RemoteWorkspaceFileChunk {
        name: remote_file_display_name(abs_path.file_name().and_then(|n| n.to_str())),
        bytes: chunk,
        offset,
        chunk_size: range.chunk_size,
        total_size,
        mime_type: detect_remote_mime_type(&abs_path),
    })
}

pub async fn read_remote_workspace_file_info(
    raw_path: &str,
    workspace_root: Option<&Path>,
) -> Result<RemoteWorkspaceFileInfo, String> {
    let abs_path = resolve_remote_workspace_path(raw_path, workspace_root)
        .ok_or_else(|| format!("Remote file path could not be resolved: {raw_path}"))?;

    if !abs_path.exists() {
        return Err(format!("File not found: {}", abs_path.display()));
    }
    if !abs_path.is_file() {
        return Err(format!("Path is not a regular file: {}", abs_path.display()));
    }

    let size = tokio::fs::metadata(&abs_path)
        .await
        .map_err(|e| format!("Cannot read file metadata: {e}"))?
        .len();

    Ok(RemoteWorkspaceFileInfo {
        name: remote_file_display_name(abs_path.file_name().and_then(|n| n.to_str())),
        size,
        mime_type: detect_remote_mime_type(&abs_path),
    })
}

pub fn remote_file_content_response(result: Result<RemoteWorkspaceFileContent, String>) -> RemoteResponse {
    match result {
        Ok(content) => RemoteResponse::FileContent {
            name: content.name,
            content_base64: base64::engine::general_purpose::STANDARD.encode(&content.bytes),
            mime_type: content.mime_type.to_string(),
            size: content.size,
        },
        Err(message) => RemoteResponse::Error { message },
    }
}

pub fn remote_file_chunk_response(result: Result<RemoteWorkspaceFileChunk, String>) -> RemoteResponse {
    match result {
        Ok(chunk) => RemoteResponse::FileChunk {
            name: chunk.name,
            chunk_base64: base64::engine::general_purpose::STANDARD.encode(&chunk.bytes),
            offset: chunk.offset,
            chunk_size: chunk.chunk_size,
            total_size: chunk.total_size,
            mime_type: chunk.mime_type.to_string(),
        },
        Err(message) => RemoteResponse::Error { message },
    }
}

pub fn remote_file_info_response(result: Result<RemoteWorkspaceFileInfo, String>) -> RemoteResponse {
    match result {
        Ok(info) => RemoteResponse::FileInfo {
            name: info.name,
            size: info.size,
            mime_type: info.mime_type.to_string(),
        },
        Err(message) => RemoteResponse::Error { message },
    }
}
