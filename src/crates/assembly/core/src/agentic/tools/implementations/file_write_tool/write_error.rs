use crate::agentic::tools::file_read_state_runtime::{
    assert_file_not_unexpectedly_modified, get_stored_file_read_state, local_file_modification_time_ms,
    read_current_file_content, read_state_tracking_enabled, validate_existing_file_read_before_write,
    FILE_UNEXPECTEDLY_MODIFIED_ERROR,
};
use crate::agentic::tools::file_tool_guidance::file_tool_guidance_message;
use crate::agentic::tools::framework::{ToolPathResolution, ToolUseContext};
use crate::agentic::tools::ToolPathOperation;
use crate::util::errors::{NortHingError, NortHingResult};
use std::path::Path;
use tokio::fs;

pub fn format_write_freshness_guidance(logical_path: &str, error: String) -> String {
    if error == FILE_UNEXPECTEDLY_MODIFIED_ERROR || error.contains("unexpectedly modified") {
        format!(
            "The file {} changed since it was last read. Use Read again, then retry Write.",
            logical_path
        )
    } else if error.contains("modified since read") {
        format!(
            "The file {} changed after it was last read. Use Read again, then retry Write.",
            logical_path
        )
    } else {
        error
    }
}

pub async fn file_exists(context: &ToolUseContext, resolved: &ToolPathResolution) -> bool {
    if resolved.uses_remote_workspace_backend() {
        if let Some(ws_fs) = context.ws_fs() {
            ws_fs.exists(&resolved.resolved_path).await.unwrap_or(false)
        } else {
            false
        }
    } else {
        Path::new(&resolved.resolved_path).exists()
    }
}

pub async fn existing_file_matches_content(
    context: &ToolUseContext,
    resolved: &ToolPathResolution,
    content: &str,
) -> Option<bool> {
    let existing = if resolved.uses_remote_workspace_backend() {
        context.ws_fs()?.read_file(&resolved.resolved_path).await.ok()?
    } else {
        fs::read(&resolved.resolved_path).await.ok()?
    };

    Some(existing == content.as_bytes())
}

pub async fn existing_file_write_freshness_error(
    context: &ToolUseContext,
    resolved: &ToolPathResolution,
) -> Option<String> {
    if !file_exists(context, resolved).await {
        return None;
    }
    if !read_state_tracking_enabled(context) {
        return None;
    }

    let current_content = match read_current_file_content(context, resolved).await {
        Ok(content) => content,
        Err(error) => return Some(error.to_string()),
    };
    let read_state = get_stored_file_read_state(context, resolved);
    let current_mtime_ms = if resolved.uses_remote_workspace_backend() {
        None
    } else {
        Some(local_file_modification_time_ms(Path::new(&resolved.resolved_path)))
    };

    assert_file_not_unexpectedly_modified(read_state.as_ref(), &current_content, current_mtime_ms)
        .err()
        .map(|error| format_write_freshness_guidance(&resolved.logical_path, error))
}

pub async fn assert_atomic_write_freshness_if_exists(
    context: &ToolUseContext,
    resolved: &ToolPathResolution,
) -> NortHingResult<()> {
    if let Some(error) = existing_file_write_freshness_error(context, resolved).await {
        return Err(NortHingError::tool(file_tool_guidance_message(error)));
    }

    Ok(())
}

pub async fn write_guardrail_preflight_error(
    context: &ToolUseContext,
    resolved: &ToolPathResolution,
) -> Option<String> {
    if !file_exists(context, resolved).await {
        return None;
    }

    if let Some(message) = validate_existing_file_read_before_write(context, resolved).await {
        return Some(file_tool_guidance_message(message));
    }

    existing_file_write_freshness_error(context, resolved)
        .await
        .map(file_tool_guidance_message)
}

pub async fn preflight_write_error(context: &ToolUseContext, file_path: &str) -> Option<String> {
    let resolved = match context.resolve_tool_path(file_path) {
        Ok(resolved) => resolved,
        Err(err) => return Some(err.to_string()),
    };

    if let Err(err) = context.enforce_path_operation(ToolPathOperation::Write, &resolved) {
        return Some(err.to_string());
    }

    write_guardrail_preflight_error(context, &resolved).await
}
