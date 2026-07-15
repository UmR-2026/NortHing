//! Stream-result validation + transient-error classification.
//!
//! Sibling module to `round_executor/mod.rs` (Round 47c split). Holds:
//! - `RETRY_BASE_DELAY_MS` private constant
//! - `is_transient_network_error` retryable-error classifier (large keyword tables)
//! - `retry_delay_ms` exponential-backoff calculator
//! - `has_interrupted_invalid_tool_calls` and `is_invalid_tool_only_without_text`
//!   stream-result predicates used by the dispatch sub-handler.

use super::super::stream_processor::StreamResult;
use super::RoundExecutor;

impl RoundExecutor {
    const RETRY_BASE_DELAY_MS: u64 = 500;

    pub(in crate::agentic::execution) fn has_interrupted_invalid_tool_calls(result: &StreamResult) -> bool {
        result.partial_recovery_reason.is_some()
            && !result.tool_calls.is_empty()
            && result.tool_calls.iter().any(|tool_call| !tool_call.is_valid())
    }

    pub(in crate::agentic::execution) fn is_invalid_tool_only_without_text(result: &StreamResult) -> bool {
        result.partial_recovery_reason.is_none()
            && !Self::has_user_visible_assistant_text(&result.full_text)
            && !result.tool_calls.is_empty()
            && result.tool_calls.iter().all(|tool_call| !tool_call.is_valid())
    }

    pub(in crate::agentic::execution) fn retry_delay_ms(attempt_index: usize) -> u64 {
        Self::RETRY_BASE_DELAY_MS * (1u64 << attempt_index.min(3))
    }

    /// Check whether an error message represents a transient (retryable) condition.
    ///
    /// Errors that already exhausted the SSE-layer retry budget (e.g. "failed
    /// after N attempts:" or "Stream retry budget exhausted") are **not**
    /// transient from the round-executor perspective — the SSE transport layer
    /// already retried with exponential backoff and `Retry-After` parsing.
    /// Re-entering the send loop would multiply attempts (10 × 10 = 100) and
    /// hold the user in a long silent stall.
    pub(in crate::agentic::execution) fn is_transient_network_error(error_message: &str) -> bool {
        let msg = error_message.to_lowercase();

        // The SSE layer already exhausted its own retry budget — do not
        // re-enter another round of attempts from the round executor.
        // We require BOTH "failed after " and "attempts:" to co-occur,
        // which uniquely identifies the SSE/round-executor budget-exhausted
        // format without catching generic errors like "failed after timeout".
        if msg.contains("failed after ") && msg.contains("attempts:") {
            return false;
        }
        if msg.contains("retry budget exhausted") {
            return false;
        }

        let non_retryable_keywords = [
            "invalid api key",
            "unauthorized",
            "forbidden",
            "model not found",
            "unsupported model",
            "invalid request",
            "bad request",
            "prompt is too long",
            "content policy",
            "proxy authentication required",
            "provider quota",
            "provider billing",
            "insufficient_quota",
            "insufficient quota",
            "insufficient balance",
            "not_enough_balance",
            "not enough balance",
            "余额不足",
            "无可用资源包",
            "账户已欠费",
            "code=1113",
            "\"code\":\"1113\"",
            "client error 400",
            "client error 401",
            "client error 402",
            "client error 403",
            "client error 404",
            "client error 413",
            "client error 422",
            "sse parsing error",
            "schema error",
            "unknown api format",
        ];

        let transient_keywords = [
            "transport error",
            "error decoding response body",
            "stream closed before response completed",
            "stream processing error",
            "sse stream error",
            "sse error",
            "sse timeout",
            "stream data timeout",
            "timeout",
            "request timeout",
            "deadline exceeded",
            "connection reset",
            "connection closed",
            "broken pipe",
            "unexpected eof",
            "connection refused",
            "socket closed",
            "temporarily unavailable",
            "service unavailable",
            "bad gateway",
            "gateway timeout",
            "overloaded",
            "proxy",
            "tunnel",
            "dns",
            "network",
            "econnreset",
            "econnrefused",
            "etimedout",
            "rate limit",
            "too many requests",
            "408",
            "409",
            "425",
            "429",
            "502",
            "503",
            "504",
        ];

        if non_retryable_keywords.iter().any(|k| msg.contains(k)) {
            return false;
        }

        transient_keywords.iter().any(|k| msg.contains(k))
    }
}
