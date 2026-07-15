//! Shared test helpers for AIClient tests.
//!
//! `make_test_client` builds an `AIClient` from an `AIConfig` template with the
//! given `format` string and optional `custom_request_body`. `make_trim_test_client`
//! layers in `custom_request_body_mode = "trim"` so the trim-mode tests can
//! verify the protection behavior in `build_request_body_subset`.

#![cfg(test)]

use crate::client::AIClient;
use crate::types::{AIConfig, ReasoningMode};
use serde_json::Value;

pub(super) fn make_test_client(format: &str, custom_request_body: Option<Value>) -> AIClient {
    AIClient::new(AIConfig {
        name: format!("{}-test", format),
        base_url: "https://example.com/v1".to_string(),
        request_url: "https://example.com/v1/chat/completions".to_string(),
        api_key: "test-key".to_string(),
        model: "test-model".to_string(),
        format: format.to_string(),
        context_window: 128000,
        max_tokens: Some(8192),
        temperature: None,
        top_p: None,
        reasoning_mode: ReasoningMode::Default,
        inline_think_in_text: false,
        custom_headers: None,
        custom_headers_mode: None,
        skip_ssl_verify: false,
        reasoning_effort: None,
        thinking_budget_tokens: None,
        custom_request_body,
        custom_request_body_mode: None,
    })
}

pub(super) fn make_trim_test_client(format: &str) -> AIClient {
    let mut client = make_test_client(format, None);
    client.config.custom_request_body_mode = Some("trim".to_string());
    client
}
