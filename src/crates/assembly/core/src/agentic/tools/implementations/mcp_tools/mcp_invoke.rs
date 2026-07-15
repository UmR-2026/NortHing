//! Read MCP resource and get prompt tools.

use crate::agentic::tools::framework::{
    Tool, ToolExposure, ToolRenderOptions, ToolResult, ToolUseContext, ValidationResult,
};
use crate::service::mcp::adapter::PromptAdapter;
use crate::service::mcp::protocol::MCPPromptContent;
use crate::util::errors::NortHingResult;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

use super::mcp_state::{ensure_mcp_server_available_for_context, get_mcp_server_manager, validate_required_string};
use super::mcp_types::{render_resource_contents, truncate_text, DEFAULT_RENDER_CHAR_LIMIT};

pub struct ReadMCPResourceTool {
    max_render_chars: usize,
}

impl Default for ReadMCPResourceTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadMCPResourceTool {
    pub fn new() -> Self {
        Self {
            max_render_chars: DEFAULT_RENDER_CHAR_LIMIT,
        }
    }
}

#[async_trait]
impl Tool for ReadMCPResourceTool {
    fn name(&self) -> &str {
        "ReadMCPResource"
    }

    async fn description(&self) -> NortHingResult<String> {
        Ok("Reads a specific MCP resource by URI from a connected MCP server. Use ListMCPResources first if you do not already know the resource URI.".to_string())
    }

    fn short_description(&self) -> String {
        "Read a specific MCP resource by URI from a connected MCP server.".to_string()
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
                    "description": "The MCP server ID that owns the resource."
                },
                "uri": {
                    "type": "string",
                    "description": "The full MCP resource URI to read."
                }
            },
            "required": ["server_id", "uri"],
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
        let server_validation = validate_required_string(input, "server_id");
        if !server_validation.result {
            return server_validation;
        }
        validate_required_string(input, "uri")
    }

    fn render_tool_use_message(&self, input: &Value, options: &ToolRenderOptions) -> String {
        let uri = input.get("uri").and_then(|value| value.as_str()).unwrap_or("unknown");
        if options.verbose {
            format!("Reading MCP resource: {}", uri)
        } else {
            format!("Read MCP resource {}", uri)
        }
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        let server_id = input
            .get("server_id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| super::mcp_types::tool_error("server_id is required"))?;
        let uri = input
            .get("uri")
            .and_then(|value| value.as_str())
            .ok_or_else(|| super::mcp_types::tool_error("uri is required"))?;

        let manager = get_mcp_server_manager().await?;
        ensure_mcp_server_available_for_context(&manager, server_id, context).await?;
        let connection = manager
            .get_connection(server_id)
            .await
            .ok_or_else(|| super::mcp_types::tool_error(format!("MCP server not connected: {}", server_id)))?;
        let result = connection.read_resource(uri).await?;
        let content_count = result.contents.len();
        let rendered = render_resource_contents(&result.contents, self.max_render_chars);

        Ok(vec![ToolResult::ok(
            json!({
                "server_id": server_id,
                "uri": uri,
                "contents": result.contents,
                "content_count": content_count,
            }),
            Some(rendered),
        )])
    }
}

pub struct GetMCPPromptTool {
    max_render_chars: usize,
}

impl Default for GetMCPPromptTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GetMCPPromptTool {
    pub fn new() -> Self {
        Self {
            max_render_chars: DEFAULT_RENDER_CHAR_LIMIT,
        }
    }
}

#[async_trait]
impl Tool for GetMCPPromptTool {
    fn name(&self) -> &str {
        "GetMCPPrompt"
    }

    async fn description(&self) -> NortHingResult<String> {
        Ok("Fetches a named MCP prompt template from a connected MCP server and renders it into plain text for the model. Pass prompt arguments when the server requires them.".to_string())
    }

    fn short_description(&self) -> String {
        "Fetch and render a named MCP prompt template from a connected MCP server.".to_string()
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
                    "description": "The MCP server ID that owns the prompt."
                },
                "name": {
                    "type": "string",
                    "description": "The MCP prompt name."
                },
                "arguments": {
                    "type": "object",
                    "description": "Optional string arguments for the prompt template.",
                    "additionalProperties": {
                        "type": "string"
                    }
                }
            },
            "required": ["server_id", "name"],
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
        let server_validation = validate_required_string(input, "server_id");
        if !server_validation.result {
            return server_validation;
        }

        let name_validation = validate_required_string(input, "name");
        if !name_validation.result {
            return name_validation;
        }

        if let Some(arguments) = input.get("arguments") {
            let Some(object) = arguments.as_object() else {
                return ValidationResult {
                    result: false,
                    message: Some("arguments must be an object".to_string()),
                    error_code: Some(400),
                    meta: None,
                };
            };

            let invalid_keys = object
                .iter()
                .filter_map(|(key, value)| (!value.is_string()).then_some(key.clone()))
                .collect::<HashSet<_>>();
            if !invalid_keys.is_empty() {
                return ValidationResult {
                    result: false,
                    message: Some(format!(
                        "arguments values must be strings: {}",
                        invalid_keys.into_iter().collect::<Vec<_>>().join(", ")
                    )),
                    error_code: Some(400),
                    meta: None,
                };
            }
        }

        ValidationResult::default()
    }

    fn render_tool_use_message(&self, input: &Value, options: &ToolRenderOptions) -> String {
        let name = input.get("name").and_then(|value| value.as_str()).unwrap_or("unknown");
        if options.verbose {
            format!("Fetching MCP prompt: {}", name)
        } else {
            format!("Get MCP prompt {}", name)
        }
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        let server_id = input
            .get("server_id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| super::mcp_types::tool_error("server_id is required"))?;
        let name = input
            .get("name")
            .and_then(|value| value.as_str())
            .ok_or_else(|| super::mcp_types::tool_error("name is required"))?;

        let arguments = input.get("arguments").and_then(|value| {
            value.as_object().map(|object| {
                object
                    .iter()
                    .filter_map(|(key, value)| value.as_str().map(|string| (key.clone(), string.to_string())))
                    .collect::<HashMap<String, String>>()
            })
        });

        let manager = get_mcp_server_manager().await?;
        ensure_mcp_server_available_for_context(&manager, server_id, context).await?;
        let connection = manager
            .get_connection(server_id)
            .await
            .ok_or_else(|| super::mcp_types::tool_error(format!("MCP server not connected: {}", server_id)))?;
        let result = connection.get_prompt(name, arguments.clone()).await?;
        let prompt_text = PromptAdapter::to_system_prompt(&MCPPromptContent {
            name: name.to_string(),
            messages: result.messages.clone(),
        });
        let (rendered_text, truncated) = truncate_text(&prompt_text, self.max_render_chars);
        let mut rendered = rendered_text;
        if truncated {
            rendered.push_str("\n\n[Output truncated after reaching the MCP prompt tool size limit.]");
        }

        Ok(vec![ToolResult::ok(
            json!({
                "server_id": server_id,
                "name": name,
                "arguments": arguments,
                "description": result.description,
                "messages": result.messages,
                "prompt_text": prompt_text,
            }),
            Some(rendered),
        )])
    }
}
