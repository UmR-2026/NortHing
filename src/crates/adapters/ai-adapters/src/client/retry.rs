//! Retry classification + backoff helpers for `send_message*` retry loop.
//!
//! - `is_transient_stream_error`  inspects error message text and decides whether
//!                                 the aggregated send loop should retry.
//! - `send_message_retry_delay_ms` computes exponential backoff for the next
//!                                 attempt.

use super::types::SEND_MESSAGE_RETRY_BASE_DELAY_MS;

pub(super) fn send_message_retry_delay_ms(attempt_index: usize) -> u64 {
    SEND_MESSAGE_RETRY_BASE_DELAY_MS * (1u64 << attempt_index.min(3))
}

pub(super) fn is_transient_stream_error(error_message: &str) -> bool {
    let msg = error_message.to_lowercase();

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

    if non_retryable_keywords.iter().any(|k| msg.contains(k)) {
        return false;
    }

    [
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
    ]
    .iter()
    .any(|k| msg.contains(k))
}
