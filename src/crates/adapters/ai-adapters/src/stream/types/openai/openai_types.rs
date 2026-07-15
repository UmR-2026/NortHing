use crate::stream::types::unified::{UnifiedTokenUsage, UnifiedToolCall};
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub(crate) struct PromptTokensDetails {
    pub(crate) cached_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenAIUsage {
    #[serde(default)]
    pub(crate) prompt_tokens: u32,
    #[serde(default)]
    pub(crate) completion_tokens: u32,
    #[serde(default)]
    pub(crate) total_tokens: u32,
    pub(crate) prompt_tokens_details: Option<PromptTokensDetails>,
    /// DeepSeek extension. Subset of `prompt_tokens`. Absent on non-DeepSeek
    /// providers. Prefer this over `prompt_tokens_details.cached_tokens` when
    /// both are present — DeepSeek-native is the authoritative source.
    #[serde(default)]
    pub(crate) prompt_cache_hit_tokens: Option<u32>,
    /// DeepSeek extension. Equals `prompt_tokens - prompt_cache_hit_tokens`.
    /// Deserialized so a future strict serde lint doesn't reject the payload;
    /// not propagated (the miss count is derivable from the other two).
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) prompt_cache_miss_tokens: Option<u32>,
}

impl From<OpenAIUsage> for UnifiedTokenUsage {
    fn from(usage: OpenAIUsage) -> Self {
        let standard_cached = usage.prompt_tokens_details.and_then(|details| details.cached_tokens);
        // DeepSeek extension wins when both present.
        let cache_read = usage.prompt_cache_hit_tokens.or(standard_cached);

        Self {
            prompt_token_count: usage.prompt_tokens,
            candidates_token_count: usage.completion_tokens,
            total_token_count: usage.total_tokens,
            reasoning_token_count: None,
            cached_content_token_count: cache_read,
            cache_creation_token_count: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Choice {
    #[allow(dead_code)]
    pub(crate) index: usize,
    /// MiniMax's last SSE frame switches to non-streaming `chat.completion`
    /// shape and puts the content under `message` instead of `delta`. We don't
    /// need that frame's content (earlier chunks already streamed it), but the
    /// frame also carries the only authoritative `usage` block. Default the
    /// field so such frames deserialize cleanly and the top-level usage flows
    /// through.
    #[serde(default)]
    pub(crate) delta: Delta,
    pub(crate) finish_reason: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_stringish")]
    pub(crate) stop_reason: Option<String>,
}

/// MiniMax `reasoning_details` array element.
/// Only elements with `type == "reasoning.text"` carry thinking text.
#[derive(Debug, Deserialize)]
pub(crate) struct ReasoningDetail {
    #[serde(rename = "type")]
    pub(crate) detail_type: Option<String>,
    pub(crate) text: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct Delta {
    // reason: Delta.role is deserialized for OpenAI compat; the streaming pipeline only reads `content`/`reasoning_*` and `tool_calls`
    #[allow(dead_code)]
    pub(crate) role: Option<String>,
    /// Standard OpenAI-compatible reasoning field (DeepSeek, Qwen, etc.)
    pub(crate) reasoning_content: Option<String>,
    /// MiniMax-specific reasoning field; used as fallback when `reasoning_content` is absent.
    pub(crate) reasoning_details: Option<Vec<ReasoningDetail>>,
    pub(crate) content: Option<String>,
    pub(crate) tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct OpenAIToolCall {
    // reason: OpenAIToolCall.index is deserialized for OpenAI compat; tool_calls are flattened in stream order so index is unused downstream
    #[allow(dead_code)]
    pub(crate) index: usize,
    // reason: OpenAIToolCall.id is deserialized but not propagated to UnifiedToolCall (the unified type carries no id today)
    #[allow(dead_code)]
    pub(crate) id: Option<String>,
    // reason: OpenAIToolCall.tool_type is deserialized for the W3C-style "type" field; today only "function" tool calls are emitted by supported providers
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub(crate) tool_type: Option<String>,
    #[serde(default)]
    pub(crate) arguments_is_snapshot: bool,
    pub(crate) function: Option<FunctionCall>,
}

impl From<OpenAIToolCall> for UnifiedToolCall {
    fn from(tool_call: OpenAIToolCall) -> Self {
        Self {
            tool_call_index: Some(tool_call.index),
            id: tool_call.id,
            name: tool_call.function.as_ref().and_then(|f| f.name.clone()),
            arguments: tool_call.function.as_ref().and_then(|f| f.arguments.clone()),
            arguments_is_snapshot: tool_call.arguments_is_snapshot,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct FunctionCall {
    pub(crate) name: Option<String>,
    pub(crate) arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAISSEData {
    // reason: id is deserialized for OpenAI compat; SSE chunks are tracked via the per-stream handle, not the response id
    #[allow(dead_code)]
    pub(crate) id: String,
    // reason: created is deserialized for OpenAI compat; timestamp is intentionally not exposed on UnifiedTokenUsage
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) created: Option<u64>,
    // reason: model is deserialized for OpenAI compat; the runtime model is known from the original request and is not echoed back
    #[allow(dead_code)]
    pub(crate) model: String,
    pub(crate) choices: Vec<Choice>,
    pub(crate) usage: Option<OpenAIUsage>,
}

#[derive(Debug, Default)]
pub struct OpenAIToolCallArgumentsNormalizer;

fn deserialize_optional_stringish<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(value)) => Some(value),
        Some(serde_json::Value::Number(value)) => Some(value.to_string()),
        Some(serde_json::Value::Bool(value)) => Some(value.to_string()),
        Some(other) => Some(other.to_string()),
    })
}
