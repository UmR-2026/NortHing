use super::write_error::{assert_atomic_write_freshness_if_exists, existing_file_matches_content, file_exists};
use super::write_format::write_success_result;
use super::write_validate::parse_mode_value;
use crate::agentic::tools::file_read_state_runtime::{
    file_mutation_timestamp_ms, read_current_file_content, update_file_read_state_after_mutation,
};
use crate::agentic::tools::framework::{ToolResult, ToolUseContext};
use crate::agentic::tools::ToolPathOperation;
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::Value;
use std::path::Path;
use tool_runtime::fs::{write_local_file, WriteLocalFileMode, WriteLocalFileRequest};

pub async fn call_impl(input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
    let file_path = input
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| NortHingError::tool("file_path is required".to_string()))?;

    let resolved = context.resolve_tool_path(file_path)?;
    context.enforce_path_operation(ToolPathOperation::Write, &resolved)?;
    context
        .record_light_checkpoint("Write", &resolved.logical_path, vec![resolved.logical_path.clone()])
        .await;

    let content = input
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| NortHingError::tool("content is required".to_string()))?;
    let content = content.to_string();
    let mode = parse_mode_value(input.get("mode").and_then(|v| v.as_str())).map_err(NortHingError::tool)?;

    let file_already_exists = file_exists(context, &resolved).await;
    if mode == WriteLocalFileMode::Write
        && file_already_exists
        && existing_file_matches_content(context, &resolved, &content).await == Some(true)
    {
        let result = write_success_result(
            &resolved.logical_path,
            mode,
            0,
            0,
            "already_exists_same_content",
            format!(
                "Write skipped because {} already exists with identical content.",
                resolved.logical_path
            ),
        );
        return Ok(vec![result]);
    }

    assert_atomic_write_freshness_if_exists(context, &resolved).await?;
    let final_content = match (mode, file_already_exists) {
        (WriteLocalFileMode::Append, true) => {
            let mut existing = read_current_file_content(context, &resolved).await?;
            existing.push_str(&content);
            existing
        }
        _ => content.clone(),
    };

    if resolved.uses_remote_workspace_backend() {
        let ws_fs = context
            .ws_fs()
            .ok_or_else(|| NortHingError::tool("Remote workspace file system is unavailable".to_string()))?;
        ws_fs
            .write_file(&resolved.resolved_path, final_content.as_bytes())
            .await
            .map_err(|e| NortHingError::tool(format!("Failed to write file: {}", e)))?;
        let timestamp_ms = file_mutation_timestamp_ms(context, &resolved).await;
        update_file_read_state_after_mutation(context, &resolved, &final_content, timestamp_ms);

        let (status, assistant_message) = match (mode, file_already_exists) {
            (WriteLocalFileMode::Write, true) => (
                "overwritten",
                format!(
                    "Successfully overwrote {} ({} bytes).",
                    resolved.logical_path,
                    content.len()
                ),
            ),
            (WriteLocalFileMode::Write, false) => (
                "created",
                format!(
                    "Successfully created {} ({} bytes).",
                    resolved.logical_path,
                    content.len()
                ),
            ),
            (WriteLocalFileMode::Append, true) => (
                "appended",
                format!(
                    "Successfully appended to {} ({} bytes).",
                    resolved.logical_path,
                    content.len()
                ),
            ),
            (WriteLocalFileMode::Append, false) => (
                "created",
                format!(
                    "Successfully created {} ({} bytes).",
                    resolved.logical_path,
                    content.len()
                ),
            ),
        };

        let result = write_success_result(
            &resolved.logical_path,
            mode,
            content.len(),
            if content.is_empty() {
                0
            } else {
                content.lines().count().max(1)
            },
            status,
            assistant_message,
        );
        return Ok(vec![result]);
    }

    let write_request = WriteLocalFileRequest {
        logical_path: resolved.logical_path.clone(),
        resolved_path: Path::new(&resolved.resolved_path).to_path_buf(),
        content: content.clone(),
        mode,
    };
    let outcome = tokio::task::spawn_blocking(move || write_local_file(write_request))
        .await
        .map_err(|error| NortHingError::tool(format!("Write task failed: {}", error)))?
        .map_err(NortHingError::tool)?;

    let timestamp_ms = file_mutation_timestamp_ms(context, &resolved).await;
    update_file_read_state_after_mutation(context, &resolved, &final_content, timestamp_ms);

    let result = write_success_result(
        &resolved.logical_path,
        mode,
        outcome.bytes_written,
        outcome.lines_written,
        outcome.status.as_str(),
        outcome.assistant_message,
    );

    Ok(vec![result])
}
