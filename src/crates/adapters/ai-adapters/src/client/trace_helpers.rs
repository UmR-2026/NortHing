//! Aggregated-trace plumbing helpers for the `send_message*` retry loop.
//!
//! These functions convert `GeminiResponse` payloads into
//! `ModelExchangeResponseTrace` records and finalize per-attempt trace handles.
//! They are siblings of `send.rs` and the `AIClient` facade.

use crate::trace::{ModelExchangeRequestTraceHandle, ModelExchangeResponseTrace, ModelExchangeTraceConfig};
use crate::types::GeminiResponse;

pub(super) async fn complete_aggregated_trace(
    trace_config: Option<&ModelExchangeTraceConfig>,
    trace_handle: Option<&ModelExchangeRequestTraceHandle>,
    response: &GeminiResponse,
) {
    let (Some(trace_config), Some(trace_handle)) = (trace_config, trace_handle) else {
        return;
    };

    trace_config
        .sink
        .request_attempt_completed(trace_handle, &gemini_response_to_trace(response))
        .await;
}

pub(super) async fn fail_aggregated_trace(
    trace_config: Option<&ModelExchangeTraceConfig>,
    trace_handle: Option<&ModelExchangeRequestTraceHandle>,
    error: &str,
) {
    let Some(trace_config) = trace_config else {
        return;
    };

    trace_config.sink.request_attempt_failed(trace_handle, error).await;
}

fn gemini_response_to_trace(response: &GeminiResponse) -> ModelExchangeResponseTrace {
    ModelExchangeResponseTrace {
        kind: "completed".to_string(),
        assistant_text: Some(response.text.clone()),
        thinking: response.reasoning_content.clone(),
        tool_calls: response
            .tool_calls
            .as_ref()
            .and_then(|tool_calls| serde_json::to_value(tool_calls).ok()),
        usage: response
            .usage
            .as_ref()
            .and_then(|usage| serde_json::to_value(usage).ok()),
        provider_metadata: response.provider_metadata.clone(),
        partial_recovery_reason: None,
        error: None,
    }
}
