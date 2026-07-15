//! Stream processing context plus the watchdog-aware stream item adapter.
//!
//! `StreamContext` accumulates the per-stream state (text/thinking/tool
//! bookkeeping, timing markers, recovery flags) consumed by `StreamProcessor`.
//! `next_stream_item` wraps the upstream `Stream` with an optional idle
//! timeout and reports its outcome as `TimedStreamItem`.
//!
//! Items here are crate-private; only `StreamProcessor` drives them.

use crate::tool_call_accumulator::{FinalizedToolCall, PendingToolCalls, ToolCallBoundary};
use crate::types::{elapsed_ms_u64, StreamProcessOptions, StreamResult, ToolCall, UNKNOWN_TOOL_PLACEHOLDER};
use crate::unified::UnifiedTokenUsage;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::collections::HashSet;
use std::time::Instant;
use tracing::{debug, error};

/// Stream processing context, encapsulates state during stream processing
pub(crate) struct StreamContext {
    pub(crate) session_id: String,
    pub(crate) dialog_turn_id: String,
    pub(crate) round_id: String,

    // Accumulated results
    pub(crate) full_thinking: String,
    pub(crate) reasoning_content_present: bool,
    /// Signature of Anthropic extended thinking (passed back in multi-turn conversations)
    pub(crate) thinking_signature: Option<String>,
    pub(crate) full_text: String,
    pub(crate) tool_calls: Vec<ToolCall>,
    pub(crate) usage: Option<UnifiedTokenUsage>,
    pub(crate) provider_metadata: Option<Value>,

    // Current tool call state
    pub(crate) pending_tool_calls: PendingToolCalls,
    pub(crate) finalized_tool_call_ids: HashSet<String>,

    // Counters and flags
    pub(crate) stream_started_at: Instant,
    pub(crate) first_chunk_ms: Option<u64>,
    pub(crate) first_visible_output_ms: Option<u64>,
    pub(crate) text_chunks_count: usize,
    pub(crate) thinking_chunks_count: usize,
    pub(crate) thinking_completed_sent: bool,
    pub(crate) has_effective_output: bool,
    pub(crate) partial_recovery_reason: Option<String>,
    /// Provider finish_reason indicating the response was cut by the model's
    /// output token limit (e.g. "length", "max_tokens", "MAX_TOKENS").
    pub(crate) token_limit_finish_reason: Option<String>,
}

impl StreamContext {
    pub(crate) fn new(
        session_id: String,
        dialog_turn_id: String,
        round_id: String,
        _options: StreamProcessOptions,
    ) -> Self {
        Self {
            session_id,
            dialog_turn_id,
            round_id,
            full_thinking: String::new(),
            reasoning_content_present: false,
            thinking_signature: None,
            full_text: String::new(),
            tool_calls: Vec::new(),
            usage: None,
            provider_metadata: None,
            pending_tool_calls: PendingToolCalls::new(),
            finalized_tool_call_ids: HashSet::new(),
            stream_started_at: Instant::now(),
            first_chunk_ms: None,
            first_visible_output_ms: None,
            text_chunks_count: 0,
            thinking_chunks_count: 0,
            thinking_completed_sent: false,
            has_effective_output: false,
            partial_recovery_reason: None,
            token_limit_finish_reason: None,
        }
    }

    pub(crate) fn into_result(self) -> StreamResult {
        StreamResult {
            full_thinking: self.full_thinking,
            reasoning_content_present: self.reasoning_content_present,
            thinking_signature: self.thinking_signature,
            full_text: self.full_text,
            tool_calls: self.tool_calls,
            usage: self.usage,
            provider_metadata: self.provider_metadata,
            has_effective_output: self.has_effective_output,
            first_chunk_ms: self.first_chunk_ms,
            first_visible_output_ms: self.first_visible_output_ms,
            partial_recovery_reason: self.partial_recovery_reason,
        }
    }

    pub(crate) fn mark_first_stream_chunk(&mut self) {
        if self.first_chunk_ms.is_none() {
            self.first_chunk_ms = Some(elapsed_ms_u64(self.stream_started_at));
        }
    }

    pub(crate) fn mark_first_visible_output(&mut self) {
        if self.first_visible_output_ms.is_none() {
            self.first_visible_output_ms = Some(elapsed_ms_u64(self.stream_started_at));
        }
    }

    pub(crate) fn can_recover_as_partial_result(&self) -> bool {
        self.has_effective_output
    }

    pub(crate) fn record_finalized_tool_call(&mut self, finalized: &FinalizedToolCall) {
        let tool_name = if finalized.tool_name.is_empty() {
            UNKNOWN_TOOL_PLACEHOLDER.to_string()
        } else {
            finalized.tool_name.clone()
        };
        let tool_id = if finalized.tool_id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            finalized.tool_id.clone()
        };
        if !self.finalized_tool_call_ids.insert(tool_id.clone()) {
            debug!(
                "Skipping duplicate finalized tool call in stream: tool_id={}, tool_name={}",
                tool_id, tool_name
            );
            return;
        }
        self.tool_calls.push(ToolCall {
            tool_id,
            tool_name,
            arguments: finalized.arguments.clone(),
            raw_arguments: (!finalized.raw_arguments.is_empty()).then_some(finalized.raw_arguments.clone()),
            is_error: finalized.is_error,
            recovered_from_truncation: finalized.recovered_from_truncation,
        });
    }

    pub(crate) fn finalize_all_pending_tool_calls(&mut self, boundary: ToolCallBoundary) -> Vec<FinalizedToolCall> {
        let finalized = self.pending_tool_calls.finalize_all(boundary);
        for tool_call in &finalized {
            self.record_finalized_tool_call(tool_call);
        }
        finalized
    }

    /// Force finish pending tool calls, used when the stream is shutting down before a natural tool boundary.
    pub(crate) fn force_finish_pending_tool_calls(&mut self) {
        for finalized in self.finalize_all_pending_tool_calls(ToolCallBoundary::GracefulShutdown) {
            error!(
                "force finish pending tool call: tool_id={}, tool_name={}, raw_len={}, is_error={}",
                finalized.tool_id,
                finalized.tool_name,
                finalized.raw_arguments.len(),
                finalized.is_error
            );
        }
    }
}

pub(crate) enum TimedStreamItem<T> {
    Item(T),
    End,
    TimedOut,
}

pub(crate) async fn next_stream_item<S>(
    stream: &mut S,
    watchdog_timeout: Option<std::time::Duration>,
) -> TimedStreamItem<S::Item>
where
    S: Stream + Unpin,
{
    match watchdog_timeout {
        Some(timeout) => match tokio::time::timeout(timeout, stream.next()).await {
            Ok(Some(item)) => TimedStreamItem::Item(item),
            Ok(None) => TimedStreamItem::End,
            Err(_) => TimedStreamItem::TimedOut,
        },
        None => match stream.next().await {
            Some(item) => TimedStreamItem::Item(item),
            None => TimedStreamItem::End,
        },
    }
}
