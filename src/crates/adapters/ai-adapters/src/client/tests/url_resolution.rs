//! Tests for `crate::providers::openai::common::resolve_models_url` and
//! `crate::providers::anthropic::discovery::resolve_models_url` — i.e. the
//! provider-specific URL resolver behavior.

#![cfg(test)]

use crate::client::AIClient;
use crate::providers::{anthropic, openai};
use crate::types::{AIConfig, ReasoningMode};

#[test]
fn resolves_openai_models_url_from_completion_endpoint() {
    let client = AIClient::new(AIConfig {
        name: "test".to_string(),
        base_url: "https://api.openai.com/v1/chat/completions".to_string(),
        request_url: "https://api.openai.com/v1/chat/completions".to_string(),
        api_key: "test-key".to_string(),
        model: "gpt-4.1".to_string(),
        format: "openai".to_string(),
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
        custom_request_body: None,
        custom_request_body_mode: None,
    });

    assert_eq!(
        openai::common::resolve_models_url(&client),
        "https://api.openai.com/v1/models"
    );
}

#[test]
fn resolves_anthropic_models_url_from_messages_endpoint() {
    let client = AIClient::new(AIConfig {
        name: "test".to_string(),
        base_url: "https://api.anthropic.com/v1/messages".to_string(),
        request_url: "https://api.anthropic.com/v1/messages".to_string(),
        api_key: "test-key".to_string(),
        model: "claude-sonnet-4-5".to_string(),
        format: "anthropic".to_string(),
        context_window: 200000,
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
        custom_request_body: None,
        custom_request_body_mode: None,
    });

    assert_eq!(
        anthropic::discovery::resolve_models_url(&client),
        "https://api.anthropic.com/v1/models"
    );
}
