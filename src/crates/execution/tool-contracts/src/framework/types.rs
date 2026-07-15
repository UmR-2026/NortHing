//! Tool contract DTOs, restrictions, validation, and result types.
//!
//! R37b sibling: portable DTOs (dynamic tool info, context facts, workspace
//! kind), collapsed-usage / access errors, runtime restrictions, validation
//! result, and tool execution result. Split verbatim from `framework.rs`.

use super::*;
use northhing_core_types::ToolImageAttachment;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Dynamic MCP tool subtype metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DynamicMcpToolInfo {
    pub server_id: String,
    pub server_name: String,
    pub tool_name: String,
}

/// Dynamic tool provider metadata used by registry and boundary adapters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DynamicToolInfo {
    pub provider_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp: Option<DynamicMcpToolInfo>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolWorkspaceKind {
    Local,
    Remote,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolContextFacts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialog_turn_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_kind: Option<ToolWorkspaceKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_root: Option<String>,
    #[serde(default)]
    pub runtime_tool_restrictions: ToolRuntimeRestrictions,
}

pub trait PortableToolContextProvider: Send + Sync {
    fn tool_context_facts(&self) -> ToolContextFacts;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollapsedToolUsageError {
    RequiresGetToolSpec {
        tool_name: String,
        get_tool_spec_tool_name: String,
    },
}

impl fmt::Display for CollapsedToolUsageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequiresGetToolSpec {
                tool_name,
                get_tool_spec_tool_name,
            } => write!(
                formatter,
                "Tool '{tool_name}' is collapsed. Call {get_tool_spec_tool_name} first with {{\"tool_name\":\"{tool_name}\"}} to read its full usage instructions and input schema, then try again."
            ),
        }
    }
}

impl std::error::Error for CollapsedToolUsageError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolExecutionAccessError {
    NotInAllowedList {
        tool_name: String,
        allowed_tools: Vec<String>,
    },
}

impl fmt::Display for ToolExecutionAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInAllowedList {
                tool_name,
                allowed_tools,
            } => write!(
                formatter,
                "Tool '{tool_name}' is not in the allowed list: {allowed_tools:?}"
            ),
        }
    }
}

impl std::error::Error for ToolExecutionAccessError {}

pub fn validate_tool_allowed_by_list(
    tool_name: &str,
    allowed_tools: &[String],
) -> Result<(), ToolExecutionAccessError> {
    if allowed_tools.is_empty() || allowed_tools.iter().any(|allowed| allowed == tool_name) {
        return Ok(());
    }

    Err(ToolExecutionAccessError::NotInAllowedList {
        tool_name: tool_name.to_string(),
        allowed_tools: allowed_tools.to_vec(),
    })
}

pub fn validate_collapsed_tool_usage(
    tool_name: &str,
    collapsed_tools: &[String],
    loaded_collapsed_tools: &[String],
    get_tool_spec_tool_name: &str,
) -> Result<(), CollapsedToolUsageError> {
    if tool_name == get_tool_spec_tool_name {
        return Ok(());
    }

    if !collapsed_tools.iter().any(|collapsed_tool| collapsed_tool == tool_name) {
        return Ok(());
    }

    if loaded_collapsed_tools
        .iter()
        .any(|loaded_tool| loaded_tool == tool_name)
    {
        return Ok(());
    }

    Err(CollapsedToolUsageError::RequiresGetToolSpec {
        tool_name: tool_name.to_string(),
        get_tool_spec_tool_name: get_tool_spec_tool_name.to_string(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolExposure {
    Expanded,
    Collapsed,
}

/// Tool result rendering options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolRenderOptions {
    pub verbose: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRuntimeRestrictions {
    #[serde(default)]
    pub allowed_tool_names: BTreeSet<String>,
    #[serde(default)]
    pub denied_tool_names: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub denied_tool_messages: BTreeMap<String, String>,
    #[serde(default)]
    pub path_policy: ToolPathPolicy,
}

impl ToolRuntimeRestrictions {
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        (self.allowed_tool_names.is_empty() || self.allowed_tool_names.contains(tool_name))
            && !self.denied_tool_names.contains(tool_name)
    }

    pub fn ensure_tool_allowed(&self, tool_name: &str) -> Result<(), ToolRestrictionError> {
        if self.denied_tool_names.contains(tool_name) {
            return Err(ToolRestrictionError::Denied {
                tool_name: tool_name.to_string(),
                message: self.denied_tool_messages.get(tool_name).cloned(),
            });
        }

        if !self.allowed_tool_names.is_empty() && !self.allowed_tool_names.contains(tool_name) {
            return Err(ToolRestrictionError::NotAllowed {
                tool_name: tool_name.to_string(),
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolRestrictionError {
    Denied { tool_name: String, message: Option<String> },
    NotAllowed { tool_name: String },
}

impl fmt::Display for ToolRestrictionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Denied { tool_name, message } => {
                if let Some(message) = message.as_deref() {
                    write!(formatter, "{message}")
                } else {
                    write!(formatter, "Tool '{}' is denied by runtime restrictions", tool_name)
                }
            }
            Self::NotAllowed { tool_name } => {
                write!(formatter, "Tool '{}' is not allowed by runtime restrictions", tool_name)
            }
        }
    }
}

impl std::error::Error for ToolRestrictionError {}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub result: bool,
    pub message: Option<String>,
    pub error_code: Option<i32>,
    pub meta: Option<Value>,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self {
            result: true,
            message: None,
            error_code: None,
            meta: None,
        }
    }
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolResult {
    #[serde(rename = "result")]
    Result {
        data: Value,
        #[serde(default)]
        result_for_assistant: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        image_attachments: Option<Vec<ToolImageAttachment>>,
    },
    #[serde(rename = "progress")]
    Progress {
        content: Value,
        normalized_messages: Option<Vec<Value>>,
        tools: Option<Vec<String>>,
    },
    #[serde(rename = "stream_chunk")]
    StreamChunk {
        data: Value,
        chunk_index: usize,
        is_final: bool,
    },
}

impl ToolResult {
    /// Get content (for display)
    pub fn content(&self) -> Value {
        match self {
            ToolResult::Result { data, .. } => data.clone(),
            ToolResult::Progress { content, .. } => content.clone(),
            ToolResult::StreamChunk { data, .. } => data.clone(),
        }
    }

    /// Standard tool success without images.
    pub fn ok(data: Value, result_for_assistant: Option<String>) -> Self {
        Self::Result {
            data,
            result_for_assistant,
            image_attachments: None,
        }
    }

    /// Tool success with optional images for multimodal tool results (Anthropic).
    pub fn ok_with_images(
        data: Value,
        result_for_assistant: Option<String>,
        image_attachments: Vec<ToolImageAttachment>,
    ) -> Self {
        Self::Result {
            data,
            result_for_assistant,
            image_attachments: Some(image_attachments),
        }
    }
}
