//! Provider-neutral tool-call data shapes and helper predicates.
//!
//! The `PendingToolCall` / `PendingToolCalls` types plus the surrounding DTOs
//! (`FinalizedToolCall`, `EarlyDetectedToolCall`, `ToolCallParamsChunk`,
//! `ToolCallDeltaOutcome`) describe the streaming accumulation state and the
//! outcomes produced at each delta. They are the public, replay-stable contract
//! used by callers of `northhing-agent-stream`.
//!
//! This sibling only owns type definitions and the small predicate helpers
//! (`is_write_like_tool_name`, `is_truncation_safe_to_recover`). JSON repair
//! lives in `tool_call_repair.rs`; method bodies (parse, finalize, apply_delta)
//! live in `tool_call_state.rs`.

use serde_json::Value;
use std::collections::BTreeMap;

/// Marker describing why a `PendingToolCall` was finalized.
///
/// `FinalizedToolCall` is the value emitted at the boundary; `ToolCallBoundary`
/// records *why* the boundary fired so logs and diagnostics can distinguish
/// "stream ended cleanly" from "model hit `max_tokens` mid-call".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallBoundary {
    NewTool,
    FinishReason,
    StreamEnd,
    GracefulShutdown,
    EndOfAggregation,
}

impl ToolCallBoundary {
    /// Stable string label used in logs and diagnostics.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NewTool => "new_tool",
            Self::FinishReason => "finish_reason",
            Self::StreamEnd => "stream_end",
            Self::GracefulShutdown => "graceful_shutdown",
            Self::EndOfAggregation => "end_of_aggregation",
        }
    }
}

/// Key under which a `PendingToolCall` is tracked inside `PendingToolCalls`.
///
/// Providers either give an explicit `index` for the delta (in which case the
/// `Indexed` variant is used) or stream only an id with no index, in which
/// case `Unindexed` is used.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToolCallStreamKey {
    Indexed(usize),
    Unindexed,
}

impl From<Option<usize>> for ToolCallStreamKey {
    fn from(value: Option<usize>) -> Self {
        match value {
            Some(index) => Self::Indexed(index),
            None => Self::Unindexed,
        }
    }
}

/// Mutable, per-call state held while a tool call is being streamed.
///
/// One `PendingToolCall` corresponds to a single tool invocation being
/// accumulated from provider deltas. When the stream signals a boundary
/// (`finish_reason`, `stream_end`, …) the pending state is converted into a
/// `FinalizedToolCall` via [`PendingToolCall::finalize`].
#[derive(Debug, Clone, Default)]
pub struct PendingToolCall {
    pub(crate) tool_id: String,
    pub(crate) tool_name: String,
    pub(crate) raw_arguments: String,
    pub(crate) early_detected_emitted: bool,
}

/// Output emitted by [`PendingToolCall::finalize`] (and batched by
/// `PendingToolCalls::finalize_*`).
#[derive(Debug, Clone)]
pub struct FinalizedToolCall {
    pub tool_id: String,
    pub tool_name: String,
    pub raw_arguments: String,
    pub arguments: Value,
    pub is_error: bool,
    /// True when the raw stream produced unparseable JSON (e.g. truncated by
    /// `max_tokens`) and we successfully patched the trailing brackets/strings
    /// to make it parse. The recovered call still executes, but downstream
    /// consumers should warn the model that the content may be incomplete.
    pub recovered_from_truncation: bool,
}

/// Emitted by `PendingToolCalls::apply_delta` the first time we observe a
/// non-empty tool id + tool name for a given stream key.
///
/// Lets the consumer surface "tool detected" events before the full
/// arguments have streamed in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EarlyDetectedToolCall {
    pub tool_id: String,
    pub tool_name: String,
}

/// Chunk of streaming arguments emitted by `PendingToolCalls::apply_delta`
/// alongside the (possibly partial) `arguments` JSON for one tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallParamsChunk {
    pub tool_id: String,
    pub tool_name: String,
    pub params_chunk: String,
}

/// Aggregate outcome of a single `apply_delta` call.
#[derive(Debug, Clone, Default)]
pub struct ToolCallDeltaOutcome {
    pub finalized_previous: Option<FinalizedToolCall>,
    pub early_detected: Option<EarlyDetectedToolCall>,
    pub params_partial: Option<ToolCallParamsChunk>,
}

/// Collection of in-flight tool calls being assembled from streaming deltas.
///
/// One `PendingToolCalls` instance typically lives for the duration of one
/// provider response; it is reset / dropped when the stream ends.
#[derive(Debug, Clone, Default)]
pub struct PendingToolCalls {
    pub(crate) pending: BTreeMap<ToolCallStreamKey, PendingToolCall>,
}

/// Tools where executing a truncated tool call is **safe and meaningful** —
/// the model intended to write content and a partial file is strictly more
/// useful than a hard failure. For everything else (Bash, Edit, Task, ...) we
/// surface the truncation as an error: a partial shell command or a partial
/// `old_string`/`new_string` for Edit can change semantics destructively.
pub fn is_write_like_tool_name(tool_name: &str) -> bool {
    matches!(tool_name, "Write" | "file_write" | "write_notebook")
}

/// Tools for which we will *attempt* to recover a truncated tool call by
/// closing brackets/strings. Combines write-like tools (where partial output
/// is genuinely useful) with `AskUserQuestion` / `TodoWrite` where the user
/// already sees a UI and we can render the partial state.
pub(crate) fn is_truncation_safe_to_recover(tool_name: &str) -> bool {
    is_write_like_tool_name(tool_name) || matches!(tool_name, "AskUserQuestion" | "TodoWrite")
}
