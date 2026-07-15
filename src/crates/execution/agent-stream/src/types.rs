//! Provider-neutral stream DTOs and the `StreamEventSink` trait.
//!
//! These types are the public, replay-stable contract used by callers of
//! `northhing-agent-stream`. Provider-specific wire parsing lives in
//! `northhing-ai-adapters` and converts into these portable shapes.
//!
//! Internal helpers shared with sibling modules (`elapsed_ms_u64`,
//! `UNKNOWN_TOOL_PLACEHOLDER`, `is_token_limit_finish_reason`) are exposed at
//! `pub(crate)` so that `stream_context` and `stream_processor` can reach them
//! without leaking past the crate root.

use crate::unified::UnifiedTokenUsage;
use northhing_events::{AgenticEvent, AgenticEventPriority as EventPriority};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::time::Instant;

/// Minimal tool-call value emitted by the stream processor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    /// Original provider-emitted argument JSON, preserved for replay stability when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_arguments: Option<String>,
    /// Record whether tool parameters are valid.
    pub is_error: bool,
    /// True when truncated raw JSON arguments were repaired into a partial tool call.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub recovered_from_truncation: bool,
}

impl ToolCall {
    pub fn is_valid(&self) -> bool {
        !self.tool_id.is_empty() && !self.tool_name.is_empty() && !self.is_error
    }
}

/// Stream-processor specific error that avoids depending on core runtime errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamProcessorError {
    AiClient(String),
    Cancelled(String),
}

impl fmt::Display for StreamProcessorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AiClient(msg) => write!(f, "AI client error: {}", msg),
            Self::Cancelled(msg) => write!(f, "Operation cancelled: {}", msg),
        }
    }
}

impl std::error::Error for StreamProcessorError {}

/// Event sink abstraction used by stream processing. Product crates can adapt
/// their own queue implementation without making this crate depend on core.
#[async_trait::async_trait]
pub trait StreamEventSink: Send + Sync {
    async fn enqueue(&self, event: AgenticEvent, priority: Option<EventPriority>);
}

/// Whether a provider finish_reason means the response was cut by the model's
/// output token limit rather than completed naturally.
/// Covers OpenAI-compatible "length", Anthropic "max_tokens", and Gemini
/// "MAX_TOKENS".
pub(crate) fn is_token_limit_finish_reason(reason: &str) -> bool {
    let normalized = reason.trim().to_ascii_lowercase();
    normalized == "length" || normalized == "max_tokens"
}

pub(crate) fn elapsed_ms_u64(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

//==============================================================================
// SSE Log Collector - Outputs raw SSE data on error
//==============================================================================

/// SSE log collector configuration
#[derive(Debug, Clone, Default)]
pub struct SseLogConfig {
    /// Maximum number of SSE data entries to output on error, None means unlimited
    pub max_output: Option<usize>,
}

/// Placeholder name for tool calls whose name was not received before the stream terminated.
pub(crate) const UNKNOWN_TOOL_PLACEHOLDER: &str = "unknown_tool";

/// Stream processing result
#[derive(Debug, Clone)]
pub struct StreamResult {
    pub full_thinking: String,
    /// Whether the provider emitted a reasoning/thinking field even if its content was empty.
    pub reasoning_content_present: bool,
    /// Signature of Anthropic extended thinking (passed back in multi-turn conversations)
    pub thinking_signature: Option<String>,
    pub full_text: String,
    pub tool_calls: Vec<ToolCall>,
    /// Token usage statistics (from model response)
    pub usage: Option<UnifiedTokenUsage>,
    /// Provider-specific metadata captured from the stream tail.
    pub provider_metadata: Option<Value>,
    /// Whether this stream produced any user-visible output (text/thinking/tool events)
    pub has_effective_output: bool,
    /// Milliseconds from stream processing start to the first upstream response item.
    pub first_chunk_ms: Option<u64>,
    /// Milliseconds from stream processing start to the first event visible to the UI.
    pub first_visible_output_ms: Option<u64>,
    /// When set, the stream terminated abnormally but was recovered with partial output.
    /// Contains a human-readable reason (e.g. "Stream processing error: ..." or
    /// "Stream processor watchdog timeout ...").
    pub partial_recovery_reason: Option<String>,
}

/// Stream processing error with output diagnostics.
#[derive(Debug)]
pub struct StreamProcessError {
    pub error: StreamProcessorError,
    pub has_effective_output: bool,
}

impl StreamProcessError {
    pub(crate) fn new(error: StreamProcessorError, has_effective_output: bool) -> Self {
        Self {
            error,
            has_effective_output,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StreamProcessOptions {
    pub recover_partial_on_cancel: bool,
}
