//! Tests for `is_transient_stream_error` — the heuristic that decides whether
//! the aggregated `send_message*` retry loop should retry after a stream
//! aggregation failure.

#![cfg(test)]

use crate::client::retry::is_transient_stream_error;

#[test]
fn aggregated_send_message_retries_transient_stream_errors() {
    for msg in [
        "SSE Error: stream closed before response completed",
        "Transport Error: error decoding response body",
        "Anthropic API is temporarily overloaded",
        "Gemini SSE stream timeout after 60s",
        "OpenAI Streaming API error 503: service unavailable",
    ] {
        assert!(is_transient_stream_error(msg), "expected transient stream error: {msg}");
    }
}

#[test]
fn aggregated_send_message_does_not_retry_permanent_errors() {
    for msg in [
        "OpenAI Streaming API client error 401: unauthorized",
        "SSE Parsing Error: missing field choices",
        "Provider error: provider=glm, code=1113, message=余额不足或无可用资源包",
    ] {
        assert!(
            !is_transient_stream_error(msg),
            "expected permanent stream error: {msg}"
        );
    }
}
