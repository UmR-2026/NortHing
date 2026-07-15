//! Trace-response builders + token-usage and failed-partial event emission for
//! `RoundExecutor`.
//!
//! Sibling module to `round_executor/mod.rs` (Round 47c split). Holds the per-round
//! emit helpers + the full `ModelExchangeResponseTrace` builder chain
//! (`trace_response` + 4 convenience wrappers + `stream_result_reasoning`).
//!
//! Uses sibling-visible `self.emit_event(...)` from `rexec_state.rs`.

use super::super::stream_processor::StreamResult;
use super::super::types::RoundContext;
use super::rexec_types::token_details_from_usage;
use super::RoundExecutor;
use crate::agentic::core::ToolCall;
use crate::agentic::events::{AgenticEvent, EventPriority, ToolEventData};
use northhing_ai_adapters::{ModelExchangeRequestTraceHandle, ModelExchangeResponseTrace, ModelExchangeTraceConfig};
use tracing::debug;

impl RoundExecutor {
    pub(in crate::agentic::execution) async fn emit_token_usage_update(
        &self,
        context: &RoundContext,
        usage: &crate::util::types::ai::GeminiUsage,
        context_window: Option<usize>,
        is_subagent: bool,
    ) {
        debug!(
            "Updating token stats from model response: input={}, output={}, total={}, is_subagent={}",
            usage.prompt_token_count, usage.candidates_token_count, usage.total_token_count, is_subagent
        );

        self.emit_event(
            AgenticEvent::TokenUsageUpdated {
                session_id: context.session_id.clone(),
                turn_id: context.dialog_turn_id.clone(),
                model_id: context.model_name.clone(),
                input_tokens: usage.prompt_token_count as usize,
                output_tokens: Some(usage.candidates_token_count as usize),
                total_tokens: usage.total_token_count as usize,
                max_context_tokens: context_window,
                is_subagent,
                cached_tokens: usage.cached_content_token_count.map(|v| v as usize),
                token_details: token_details_from_usage(usage),
            },
            EventPriority::Normal,
        )
        .await;
    }

    pub(in crate::agentic::execution) async fn emit_failed_partial_tool_calls(
        &self,
        context: &RoundContext,
        round_id: &str,
        tool_calls: &[ToolCall],
        error: &str,
    ) {
        for tool_call in tool_calls {
            self.emit_event(
                AgenticEvent::ToolEvent {
                    session_id: context.session_id.clone(),
                    turn_id: context.dialog_turn_id.clone(),
                    round_id: round_id.to_string(),
                    tool_event: ToolEventData::Failed {
                        tool_id: tool_call.tool_id.clone(),
                        tool_name: tool_call.tool_name.clone(),
                        error: format!("Tool arguments stream interrupted: {}", error),
                        duration_ms: None,
                        queue_wait_ms: None,
                        preflight_ms: None,
                        confirmation_wait_ms: None,
                        execution_ms: None,
                    },
                },
                EventPriority::High,
            )
            .await;
        }
    }

    pub(in crate::agentic::execution) async fn complete_model_exchange_trace(
        trace_config: Option<&ModelExchangeTraceConfig>,
        trace_handle: Option<&ModelExchangeRequestTraceHandle>,
        response: ModelExchangeResponseTrace,
    ) {
        let (Some(trace_config), Some(trace_handle)) = (trace_config, trace_handle) else {
            return;
        };

        trace_config
            .sink
            .request_attempt_completed(trace_handle, &response)
            .await;
    }

    pub(in crate::agentic::execution) fn final_trace_response(result: &StreamResult) -> ModelExchangeResponseTrace {
        let kind = if result.partial_recovery_reason.is_some() {
            "partial"
        } else {
            "completed"
        };
        Self::trace_response(kind, Some(result), None)
    }

    pub(in crate::agentic::execution) fn trace_response_from_stream_result(
        kind: &str,
        result: &StreamResult,
    ) -> ModelExchangeResponseTrace {
        Self::trace_response(kind, Some(result), None)
    }

    pub(in crate::agentic::execution) fn error_trace_response_from_stream_result(
        kind: &str,
        error: String,
        result: &StreamResult,
    ) -> ModelExchangeResponseTrace {
        Self::trace_response(kind, Some(result), Some(error))
    }

    pub(in crate::agentic::execution) fn error_trace_response(kind: &str, error: String) -> ModelExchangeResponseTrace {
        Self::trace_response(kind, None, Some(error))
    }

    fn trace_response(kind: &str, result: Option<&StreamResult>, error: Option<String>) -> ModelExchangeResponseTrace {
        let (assistant_text, thinking, tool_calls, usage, provider_metadata, partial_recovery_reason) =
            if let Some(result) = result {
                (
                    Some(result.full_text.clone()),
                    Self::stream_result_reasoning(result),
                    serde_json::to_value(&result.tool_calls).ok(),
                    result.usage.as_ref().and_then(|usage| serde_json::to_value(usage).ok()),
                    result.provider_metadata.clone(),
                    result.partial_recovery_reason.clone(),
                )
            } else {
                (None, None, None, None, None, None)
            };

        ModelExchangeResponseTrace {
            kind: kind.to_string(),
            assistant_text,
            thinking,
            tool_calls,
            usage,
            provider_metadata,
            partial_recovery_reason,
            error,
        }
    }

    fn stream_result_reasoning(result: &StreamResult) -> Option<String> {
        if result.full_thinking.is_empty() {
            result.reasoning_content_present.then(String::new)
        } else {
            Some(result.full_thinking.clone())
        }
    }
}
