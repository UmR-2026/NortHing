//! Model-interaction content types: [`ModelRoundData`] and the per-round item
//! DTOs ([`TextItemData`], [`ThinkingItemData`], [`ToolItemData`]) plus their
//! payload wrappers ([`UserMessageData`], [`ToolCallData`], [`ToolResultData`]).
//!
//! This sibling has no dependency on other session types — it stands alone so
//! both [`super::dialog_turn`] and the facade can pull from it without
//! inverting the split.

use serde::{Deserialize, Serialize};

/// User message data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMessageData {
    pub id: String,
    pub content: String,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Model interaction round data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRoundData {
    pub id: String,
    #[serde(alias = "turn_id")]
    pub turn_id: String,
    #[serde(alias = "round_index")]
    pub round_index: usize,
    pub timestamp: u64,

    /// Text item entries
    #[serde(default, alias = "text_items")]
    pub text_items: Vec<TextItemData>,

    /// Tool call entries
    #[serde(default, alias = "tool_items")]
    pub tool_items: Vec<ToolItemData>,

    /// Thinking item entries
    #[serde(default, alias = "thinking_items")]
    pub thinking_items: Vec<ThinkingItemData>,

    #[serde(alias = "start_time")]
    pub start_time: u64,
    #[serde(skip_serializing_if = "Option::is_none", alias = "end_time")]
    pub end_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "duration_ms")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "provider_id")]
    pub provider_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "model_id")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "model_alias")]
    pub model_alias: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "first_chunk_ms")]
    pub first_chunk_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "first_visible_output_ms")]
    pub first_visible_output_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "stream_duration_ms")]
    pub stream_duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "attempt_count")]
    pub attempt_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "failure_category")]
    pub failure_category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "token_details")]
    pub token_details: Option<serde_json::Value>,
    pub status: String,
}

/// Text item data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextItemData {
    pub id: String,
    pub content: String,
    #[serde(alias = "is_streaming")]
    pub is_streaming: bool,
    pub timestamp: u64,
    /// Whether Markdown format (default `true`)
    #[serde(default = "default_is_markdown", alias = "is_markdown")]
    pub is_markdown: bool,

    /// Original order index (to restore the correct insertion order)
    #[serde(skip_serializing_if = "Option::is_none", alias = "order_index")]
    pub order_index: Option<usize>,

    /// Subagent marker field
    #[serde(skip_serializing_if = "Option::is_none", alias = "is_subagent_item")]
    pub is_subagent_item: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none", alias = "parent_task_tool_id")]
    pub parent_task_tool_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", alias = "subagent_session_id")]
    pub subagent_session_id: Option<String>,

    /// Status field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

fn default_is_markdown() -> bool {
    true
}

/// Thinking item data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingItemData {
    pub id: String,
    pub content: String,
    #[serde(alias = "is_streaming")]
    pub is_streaming: bool,
    #[serde(alias = "is_collapsed")]
    pub is_collapsed: bool,
    pub timestamp: u64,

    /// Original order index (to restore the correct insertion order)
    #[serde(skip_serializing_if = "Option::is_none", alias = "order_index")]
    pub order_index: Option<usize>,

    /// Status field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Subagent marker field (fixes incorrect placement of subagent thinking content after restart)
    #[serde(skip_serializing_if = "Option::is_none", alias = "is_subagent_item")]
    pub is_subagent_item: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none", alias = "parent_task_tool_id")]
    pub parent_task_tool_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", alias = "subagent_session_id")]
    pub subagent_session_id: Option<String>,
}

/// Tool item data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolItemData {
    pub id: String,
    #[serde(alias = "tool_name")]
    pub tool_name: String,
    #[serde(alias = "tool_call")]
    pub tool_call: ToolCallData,
    #[serde(skip_serializing_if = "Option::is_none", alias = "tool_result")]
    pub tool_result: Option<ToolResultData>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "ai_intent")]
    pub ai_intent: Option<String>,
    #[serde(alias = "start_time")]
    pub start_time: u64,
    #[serde(skip_serializing_if = "Option::is_none", alias = "end_time")]
    pub end_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "duration_ms")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "queue_wait_ms")]
    pub queue_wait_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "preflight_ms")]
    pub preflight_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "confirmation_wait_ms")]
    pub confirmation_wait_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "execution_ms")]
    pub execution_ms: Option<u64>,

    /// Original order index (to restore the correct insertion order)
    #[serde(skip_serializing_if = "Option::is_none", alias = "order_index")]
    pub order_index: Option<usize>,

    /// Subagent marker field
    #[serde(skip_serializing_if = "Option::is_none", alias = "is_subagent_item")]
    pub is_subagent_item: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none", alias = "parent_task_tool_id")]
    pub parent_task_tool_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", alias = "subagent_session_id")]
    pub subagent_session_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", alias = "subagent_model_id")]
    pub subagent_model_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", alias = "subagent_model_alias")]
    pub subagent_model_alias: Option<String>,

    /// Status field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", alias = "interruption_reason")]
    pub interruption_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallData {
    pub input: serde_json::Value,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultData {
    pub result: serde_json::Value,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none", alias = "result_for_assistant")]
    pub result_for_assistant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "duration_ms")]
    pub duration_ms: Option<u64>,
}
