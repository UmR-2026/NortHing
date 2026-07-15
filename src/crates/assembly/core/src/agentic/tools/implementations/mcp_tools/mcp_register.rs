//! List MCP resources and prompts tools.

use crate::agentic::tools::framework::{
    Tool, ToolExposure, ToolRenderOptions, ToolResult, ToolUseContext, ValidationResult,
};
use crate::util::errors::NortHingResult;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::mcp_state::{
    ensure_mcp_server_available_for_context, get_mcp_server_manager, list_prompts_for_server,
    list_resources_for_server, validate_required_string,
};
use super::mcp_types::{render_prompt_catalog, render_resource_catalog};

pub struct ListMCPResourcesTool;

impl Default for ListMCPResourcesTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ListMCPResourcesTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ListMCPResourcesTool {
    fn name(&self) -> &str {
        "ListMCPResources"
    }

    async fn description(&self) -> NortHingResult<String> {
        Ok("Lists MCP resources exposed by a connected MCP server. Use this before ReadMCPResource when you need to inspect available MCP-hosted files, docs, or structured context.".to_string())
    }

    fn short_description(&self) -> String {
        "List MCP resources exposed by a connected MCP server.".to_string()
    }

    fn default_exposure(&self) -> ToolExposure {
        ToolExposure::Collapsed
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "server_id": {
                    "type": "string",
                    "description": "The MCP server ID to inspect."
                },
                "refresh": {
                    "type": "boolean",
                    "description": "When true, refresh the server catalog before returning resources.",
                    "default": false
                }
            },
            "required": ["server_id"],
            "additionalProperties": false
        })
    }

    fn is_readonly(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: Option<&Value>) -> bool {
        true
    }

    fn needs_permissions(&self, _input: Option<&Value>) -> bool {
        false
    }

    async fn validate_input(&self, input: &Value, _context: Option<&ToolUseContext>) -> ValidationResult {
        validate_required_string(input, "server_id")
    }

    fn render_tool_use_message(&self, input: &Value, options: &ToolRenderOptions) -> String {
        let server_id = input
            .get("server_id")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        if options.verbose {
            format!("Listing MCP resources from server: {}", server_id)
        } else {
            format!("List MCP resources from {}", server_id)
        }
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        let server_id = input
            .get("server_id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| super::mcp_types::tool_error("server_id is required"))?;
        let refresh = input.get("refresh").and_then(|value| value.as_bool()).unwrap_or(false);

        let manager = get_mcp_server_manager().await?;
        ensure_mcp_server_available_for_context(&manager, server_id, context).await?;
        let resources = list_resources_for_server(&manager, server_id, refresh).await?;
        let count = resources.len();
        let rendered = render_resource_catalog(&resources);

        Ok(vec![ToolResult::ok(
            json!({
                "server_id": server_id,
                "resources": resources,
                "count": count,
            }),
            Some(rendered),
        )])
    }
}

pub struct ListMCPPromptsTool;

impl Default for ListMCPPromptsTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ListMCPPromptsTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ListMCPPromptsTool {
    fn name(&self) -> &str {
        "ListMCPPrompts"
    }

    async fn description(&self) -> NortHingResult<String> {
        Ok("Lists MCP prompts exposed by a connected MCP server. Use this before GetMCPPrompt when you need reusable server-provided prompt templates.".to_string())
    }

    fn short_description(&self) -> String {
        "List MCP prompts exposed by a connected MCP server.".to_string()
    }

    fn default_exposure(&self) -> ToolExposure {
        ToolExposure::Collapsed
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "server_id": {
                    "type": "string",
                    "description": "The MCP server ID to inspect."
                },
                "refresh": {
                    "type": "boolean",
                    "description": "When true, refresh the server catalog before returning prompts.",
                    "default": false
                }
            },
            "required": ["server_id"],
            "additionalProperties": false
        })
    }

    fn is_readonly(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: Option<&Value>) -> bool {
        true
    }

    fn needs_permissions(&self, _input: Option<&Value>) -> bool {
        false
    }

    async fn validate_input(&self, input: &Value, _context: Option<&ToolUseContext>) -> ValidationResult {
        validate_required_string(input, "server_id")
    }

    fn render_tool_use_message(&self, input: &Value, options: &ToolRenderOptions) -> String {
        let server_id = input
            .get("server_id")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        if options.verbose {
            format!("Listing MCP prompts from server: {}", server_id)
        } else {
            format!("List MCP prompts from {}", server_id)
        }
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        let server_id = input
            .get("server_id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| super::mcp_types::tool_error("server_id is required"))?;
        let refresh = input.get("refresh").and_then(|value| value.as_bool()).unwrap_or(false);

        let manager = get_mcp_server_manager().await?;
        ensure_mcp_server_available_for_context(&manager, server_id, context).await?;
        let prompts = list_prompts_for_server(&manager, server_id, refresh).await?;
        let count = prompts.len();
        let rendered = render_prompt_catalog(&prompts);

        Ok(vec![ToolResult::ok(
            json!({
                "server_id": server_id,
                "prompts": prompts,
                "count": count,
            }),
            Some(rendered),
        )])
    }
}
