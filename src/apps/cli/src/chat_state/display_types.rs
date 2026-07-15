// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chat state display types
//!
//! Pure data types used by the TUI rendering layer:
//! - [`ToolDisplayStatus`], [`MessageRole`], [`display_text_for_role`]
//! - [`SubagentProgress`], [`ToolDisplayState`], [`FlowItem`]
//! - [`ChatMessage`] with `from_core_message` conversion
//! - [`ChatMetadata`]

use std::time::SystemTime;

use northhing_core::agentic::core::message::{Message as CoreMessage, MessageContent, MessageRole as CoreMessageRole};
use northhing_core::agentic::core::strip_prompt_markup;

use super::helpers::extract_fallback_summary;

// ============ Display Status Types ============

/// Tool display status (for UI rendering)
#[derive(Debug, Clone, PartialEq)]
pub enum ToolDisplayStatus {
    EarlyDetected,
    ParamsPartial,
    Queued,
    Waiting,
    ConfirmationNeeded,
    Confirmed,
    Rejected,
    Pending,
    Running,
    Streaming,
    Success,
    Failed,
    Cancelled,
}

impl ToolDisplayStatus {
    /// Returns true if the tool has entered an active execution phase
    /// (Running, Streaming, or any terminal state). Early pipeline stages
    /// (ParamsPartial, Queued, Waiting) should not overwrite these states,
    /// since priority queue ordering can cause late-arriving low-priority
    /// events to arrive after high-priority state transitions.
    pub fn is_execution_phase(&self) -> bool {
        matches!(
            self,
            ToolDisplayStatus::Running
                | ToolDisplayStatus::Streaming
                | ToolDisplayStatus::Success
                | ToolDisplayStatus::Failed
                | ToolDisplayStatus::Cancelled
                | ToolDisplayStatus::Rejected
        )
    }
}

/// Message role for display
#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl From<&CoreMessageRole> for MessageRole {
    fn from(role: &CoreMessageRole) -> Self {
        match role {
            CoreMessageRole::User => MessageRole::User,
            CoreMessageRole::Assistant => MessageRole::Assistant,
            CoreMessageRole::System => MessageRole::System,
            CoreMessageRole::Tool => MessageRole::Tool,
        }
    }
}

fn display_text_for_role(role: &MessageRole, text: &str) -> String {
    if *role == MessageRole::User {
        strip_prompt_markup(text)
    } else {
        text.to_string()
    }
}

// ============ UI Display Types ============

/// Subagent progress tracking (for Task tool real-time display)
#[derive(Debug, Clone, Default)]
pub struct SubagentProgress {
    /// Total tool calls made by the subagent so far
    pub tool_count: usize,
    /// Name of the currently executing tool in the subagent (if any)
    pub current_tool_name: Option<String>,
    /// Summary/title of the current tool (e.g. file path, command)
    pub current_tool_title: Option<String>,
}

/// Tool call display state (for rendering tool cards)
#[derive(Debug, Clone)]
pub struct ToolDisplayState {
    pub tool_id: String,
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub status: ToolDisplayStatus,
    pub result: Option<String>,
    pub progress_message: Option<String>,
    pub duration_ms: Option<u64>,
    /// Optional metadata for richer display (e.g. full diff patch, diagnostics)
    pub metadata: Option<serde_json::Value>,
    /// Subagent progress (only for Task tools)
    pub subagent_progress: Option<SubagentProgress>,
}

/// A single content block in a message (text, thinking, or tool call)
#[derive(Debug, Clone)]
pub enum FlowItem {
    /// Text content block
    Text { content: String, is_streaming: bool },
    /// AI thinking/reasoning block
    Thinking { content: String },
    /// Tool call block
    Tool { tool_state: ToolDisplayState },
}

/// A chat message for UI rendering (converted from core Message + streaming state)
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub timestamp: SystemTime,
    pub flow_items: Vec<FlowItem>,
    pub is_streaming: bool,
    /// Monotonically increasing version number; incremented on every content change.
    /// Used by render cache to detect stale entries without deep comparison.
    pub version: u64,
}

impl ChatMessage {
    /// Convert a core Message to a UI ChatMessage
    pub fn from_core_message(msg: &CoreMessage) -> Self {
        let role = MessageRole::from(&msg.role);
        let mut flow_items = Vec::new();

        match &msg.content {
            MessageContent::Text(text) => {
                if !text.is_empty() {
                    flow_items.push(FlowItem::Text {
                        content: display_text_for_role(&role, text),
                        is_streaming: false,
                    });
                }
            }
            MessageContent::Mixed {
                reasoning_content,
                text,
                tool_calls,
            } => {
                // Add reasoning/thinking block if present
                if let Some(reasoning) = reasoning_content {
                    if !reasoning.is_empty() {
                        flow_items.push(FlowItem::Thinking {
                            content: reasoning.clone(),
                        });
                    }
                }

                // Add text block if present
                if !text.is_empty() {
                    flow_items.push(FlowItem::Text {
                        content: display_text_for_role(&role, text),
                        is_streaming: false,
                    });
                }

                // Add tool call blocks
                for tc in tool_calls {
                    flow_items.push(FlowItem::Tool {
                        tool_state: ToolDisplayState {
                            tool_id: tc.tool_id.clone(),
                            tool_name: tc.tool_name.clone(),
                            parameters: tc.arguments.clone(),
                            status: ToolDisplayStatus::Success, // Historical messages are completed
                            result: None,
                            progress_message: None,
                            duration_ms: None,
                            metadata: None,
                            subagent_progress: None,
                        },
                    });
                }
            }
            MessageContent::Multimodal { text, .. } => {
                if !text.is_empty() {
                    flow_items.push(FlowItem::Text {
                        content: display_text_for_role(&role, text),
                        is_streaming: false,
                    });
                }
            }
            MessageContent::ToolResult {
                tool_id,
                tool_name,
                result,
                is_error,
                ..
            } => {
                let result_str = extract_fallback_summary(result);
                flow_items.push(FlowItem::Tool {
                    tool_state: ToolDisplayState {
                        tool_id: tool_id.clone(),
                        tool_name: tool_name.clone(),
                        parameters: serde_json::Value::Null,
                        status: if *is_error {
                            ToolDisplayStatus::Failed
                        } else {
                            ToolDisplayStatus::Success
                        },
                        result: Some(result_str),
                        progress_message: None,
                        subagent_progress: None,
                        duration_ms: None,
                        metadata: Some(result.clone()),
                    },
                });
            }
        }

        Self {
            id: msg.id.clone(),
            role,
            timestamp: msg.timestamp,
            flow_items,
            is_streaming: false,
            version: 0,
        }
    }
}

// ============ Chat Metadata ============

/// Statistics for the current chat session
#[derive(Debug, Clone, Default)]
pub struct ChatMetadata {
    pub message_count: usize,
    pub tool_calls: usize,
    pub total_rounds: usize,
    pub total_tokens: usize,
}
