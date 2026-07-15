//! MCP server connection state and shared validation for MCP resource/prompt tools.

use crate::agentic::tools::framework::{ToolUseContext, ValidationResult};
use crate::service::mcp::global_mcp_service;
use crate::service::mcp::protocol::{MCPPrompt, MCPResource};
use crate::service::mcp::MCPServerManager;
use crate::util::errors::NortHingResult;
use serde_json::Value;
use std::sync::Arc;

use super::mcp_types::tool_error;

pub(super) async fn get_mcp_server_manager() -> NortHingResult<Arc<MCPServerManager>> {
    global_mcp_service()
        .map(|service| service.server_manager())
        .ok_or_else(|| tool_error("MCP service is not initialized"))
}

pub(super) async fn list_resources_for_server(
    manager: &Arc<MCPServerManager>,
    server_id: &str,
    refresh: bool,
) -> NortHingResult<Vec<MCPResource>> {
    let mut resources = manager.get_cached_resources(server_id).await;
    if refresh || resources.is_empty() {
        manager.refresh_server_resource_catalog(server_id).await?;
        resources = manager.get_cached_resources(server_id).await;
    }
    Ok(resources)
}

pub(super) async fn list_prompts_for_server(
    manager: &Arc<MCPServerManager>,
    server_id: &str,
    refresh: bool,
) -> NortHingResult<Vec<MCPPrompt>> {
    let mut prompts = manager.get_cached_prompts(server_id).await;
    if refresh || prompts.is_empty() {
        manager.refresh_server_prompt_catalog(server_id).await?;
        prompts = manager.get_cached_prompts(server_id).await;
    }
    Ok(prompts)
}

pub(super) async fn ensure_mcp_server_available_for_context(
    manager: &Arc<MCPServerManager>,
    server_id: &str,
    _context: &ToolUseContext,
) -> NortHingResult<()> {
    manager
        .get_connection(server_id)
        .await
        .ok_or_else(|| tool_error(format!("MCP server not connected: {}", server_id)))?;

    Ok(())
}

pub(super) fn validate_required_string(input: &Value, field_name: &str) -> ValidationResult {
    match input.get(field_name).and_then(|value| value.as_str()) {
        Some(value) if !value.trim().is_empty() => ValidationResult::default(),
        Some(_) => ValidationResult {
            result: false,
            message: Some(format!("{} cannot be empty", field_name)),
            error_code: Some(400),
            meta: None,
        },
        None => ValidationResult {
            result: false,
            message: Some(format!("{} is required", field_name)),
            error_code: Some(400),
            meta: None,
        },
    }
}
