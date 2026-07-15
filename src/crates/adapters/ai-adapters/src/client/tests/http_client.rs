//! Tests that the shared `reqwest::Client` does NOT apply a global request
//! timeout — streamed responses need indefinite body timeouts, and the
//! TTFT/idle timeouts are enforced separately via `tokio::time::timeout` in
//! `client::sse::send_stream_request`.

#![cfg(test)]

use super::helpers::make_test_client;

#[test]
fn streaming_http_client_does_not_apply_global_request_timeout() {
    let client = make_test_client("openai", None);
    let request = client
        .client
        .get("https://example.com/stream")
        .build()
        .expect("request should build");

    assert_eq!(request.timeout(), None);
}
