//! Anthropic request-body construction tests (non-trim-mode).
//!
//! Exercises reasoning-mode / effort / budget_tokens mapping for adaptive
//! models (claude-sonnet-4-6, claude-opus-4-8), pre-adaptive models
//! (claude-sonnet-4-5), DeepSeek-Anthropic compatibility, and mythos quirks.

#![cfg(test)]

use crate::client::AIClient;
use crate::providers::anthropic;
use crate::types::{AIConfig, ReasoningMode};
use serde_json::json;

#[test]
fn build_anthropic_request_body_uses_adaptive_reasoning_and_effort() {
    let client = AIClient::new(AIConfig {
        name: "anthropic".to_string(),
        base_url: "https://api.anthropic.com".to_string(),
        request_url: "https://api.anthropic.com/v1/messages".to_string(),
        api_key: "test-key".to_string(),
        model: "claude-sonnet-4-6".to_string(),
        format: "anthropic".to_string(),
        context_window: 200000,
        max_tokens: Some(8192),
        temperature: None,
        top_p: None,
        reasoning_mode: ReasoningMode::Adaptive,
        inline_think_in_text: false,
        custom_headers: None,
        custom_headers_mode: None,
        skip_ssl_verify: false,
        reasoning_effort: Some("high".to_string()),
        thinking_budget_tokens: None,
        custom_request_body: None,
        custom_request_body_mode: None,
    });

    let request_body = anthropic::request::build_request_body(
        &client,
        &client.config.request_url,
        None,
        vec![json!({ "role": "user", "content": [{ "type": "text", "text": "hello" }] })],
        None,
        None,
    );

    assert_eq!(request_body["thinking"]["type"], "adaptive");
    assert_eq!(request_body["output_config"]["effort"], "high");
}

#[test]
fn build_anthropic_request_body_maps_enabled_to_adaptive_for_adaptive_models() {
    let client = AIClient::new(AIConfig {
        name: "anthropic".to_string(),
        base_url: "https://api.anthropic.com".to_string(),
        request_url: "https://api.anthropic.com/v1/messages".to_string(),
        api_key: "test-key".to_string(),
        model: "claude-sonnet-4-6".to_string(),
        format: "anthropic".to_string(),
        context_window: 200000,
        max_tokens: Some(8192),
        temperature: None,
        top_p: None,
        reasoning_mode: ReasoningMode::Enabled,
        inline_think_in_text: false,
        custom_headers: None,
        custom_headers_mode: None,
        skip_ssl_verify: false,
        reasoning_effort: None,
        thinking_budget_tokens: None,
        custom_request_body: None,
        custom_request_body_mode: None,
    });

    let request_body = anthropic::request::build_request_body(
        &client,
        &client.config.request_url,
        None,
        vec![json!({ "role": "user", "content": [{ "type": "text", "text": "hello" }] })],
        None,
        None,
    );

    assert_eq!(request_body["thinking"]["type"], "adaptive");
    assert!(request_body["thinking"].get("budget_tokens").is_none());
    assert_eq!(request_body["output_config"]["effort"], "medium");
}

#[test]
fn build_anthropic_request_body_keeps_manual_thinking_for_pre_adaptive_models() {
    let client = AIClient::new(AIConfig {
        name: "anthropic".to_string(),
        base_url: "https://api.anthropic.com".to_string(),
        request_url: "https://api.anthropic.com/v1/messages".to_string(),
        api_key: "test-key".to_string(),
        model: "claude-sonnet-4-5".to_string(),
        format: "anthropic".to_string(),
        context_window: 200000,
        max_tokens: Some(8192),
        temperature: None,
        top_p: None,
        reasoning_mode: ReasoningMode::Enabled,
        inline_think_in_text: false,
        custom_headers: None,
        custom_headers_mode: None,
        skip_ssl_verify: false,
        reasoning_effort: Some("high".to_string()),
        thinking_budget_tokens: None,
        custom_request_body: None,
        custom_request_body_mode: None,
    });

    let request_body = anthropic::request::build_request_body(
        &client,
        &client.config.request_url,
        None,
        vec![json!({ "role": "user", "content": [{ "type": "text", "text": "hello" }] })],
        None,
        None,
    );

    assert_eq!(request_body["thinking"]["type"], "enabled");
    assert_eq!(request_body["thinking"]["budget_tokens"], 6144);
    assert!(request_body.get("output_config").is_none());
}

#[test]
fn build_anthropic_request_body_uses_adaptive_for_opus_4_7_and_newer() {
    let client = AIClient::new(AIConfig {
        name: "anthropic".to_string(),
        base_url: "https://api.anthropic.com".to_string(),
        request_url: "https://api.anthropic.com/v1/messages".to_string(),
        api_key: "test-key".to_string(),
        model: "claude-opus-4-8".to_string(),
        format: "anthropic".to_string(),
        context_window: 200000,
        max_tokens: Some(8192),
        temperature: None,
        top_p: None,
        reasoning_mode: ReasoningMode::Enabled,
        inline_think_in_text: false,
        custom_headers: None,
        custom_headers_mode: None,
        skip_ssl_verify: false,
        reasoning_effort: Some("high".to_string()),
        thinking_budget_tokens: Some(2048),
        custom_request_body: None,
        custom_request_body_mode: None,
    });

    let request_body = anthropic::request::build_request_body(
        &client,
        &client.config.request_url,
        None,
        vec![json!({ "role": "user", "content": [{ "type": "text", "text": "hello" }] })],
        None,
        None,
    );

    assert_eq!(request_body["thinking"]["type"], "adaptive");
    assert!(request_body["thinking"].get("budget_tokens").is_none());
    assert_eq!(request_body["output_config"]["effort"], "high");
}

#[test]
fn build_anthropic_request_body_omits_disabled_for_mythos() {
    let client = AIClient::new(AIConfig {
        name: "anthropic".to_string(),
        base_url: "https://api.anthropic.com".to_string(),
        request_url: "https://api.anthropic.com/v1/messages".to_string(),
        api_key: "test-key".to_string(),
        model: "claude-mythos-preview".to_string(),
        format: "anthropic".to_string(),
        context_window: 200000,
        max_tokens: Some(8192),
        temperature: None,
        top_p: None,
        reasoning_mode: ReasoningMode::Disabled,
        inline_think_in_text: false,
        custom_headers: None,
        custom_headers_mode: None,
        skip_ssl_verify: false,
        reasoning_effort: None,
        thinking_budget_tokens: None,
        custom_request_body: None,
        custom_request_body_mode: None,
    });

    let request_body = anthropic::request::build_request_body(
        &client,
        &client.config.request_url,
        None,
        vec![json!({ "role": "user", "content": [{ "type": "text", "text": "hello" }] })],
        None,
        None,
    );

    assert!(request_body.get("thinking").is_none());
    assert!(request_body.get("output_config").is_none());
}

#[test]
fn build_anthropic_request_body_adds_deepseek_reasoning_effort() {
    let client = AIClient::new(AIConfig {
        name: "deepseek".to_string(),
        base_url: "https://api.deepseek.com/anthropic".to_string(),
        request_url: "https://api.deepseek.com/anthropic/v1/messages".to_string(),
        api_key: "test-key".to_string(),
        model: "deepseek-v4-pro".to_string(),
        format: "anthropic".to_string(),
        context_window: 200000,
        max_tokens: Some(8192),
        temperature: None,
        top_p: None,
        reasoning_mode: ReasoningMode::Enabled,
        inline_think_in_text: false,
        custom_headers: None,
        custom_headers_mode: None,
        skip_ssl_verify: false,
        reasoning_effort: Some("xhigh".to_string()),
        thinking_budget_tokens: None,
        custom_request_body: None,
        custom_request_body_mode: None,
    });

    let request_body = anthropic::request::build_request_body(
        &client,
        &client.config.request_url,
        None,
        vec![json!({ "role": "user", "content": [{ "type": "text", "text": "hello" }] })],
        None,
        None,
    );

    assert_eq!(request_body["thinking"]["type"], "enabled");
    assert!(request_body["thinking"].get("budget_tokens").is_none());
    assert_eq!(request_body["output_config"]["effort"], "max");
}

#[test]
fn build_anthropic_request_body_enabled_reasoning_always_has_budget_tokens() {
    let client = AIClient::new(AIConfig {
        name: "anthropic-proxy".to_string(),
        base_url: "https://proxy.example.com/anthropic".to_string(),
        request_url: "https://proxy.example.com/anthropic/v1/messages".to_string(),
        api_key: "test-key".to_string(),
        model: "vendor-model-alias".to_string(),
        format: "anthropic".to_string(),
        context_window: 200000,
        max_tokens: Some(4000),
        temperature: None,
        top_p: None,
        reasoning_mode: ReasoningMode::Enabled,
        inline_think_in_text: false,
        custom_headers: None,
        custom_headers_mode: None,
        skip_ssl_verify: false,
        reasoning_effort: None,
        thinking_budget_tokens: None,
        custom_request_body: None,
        custom_request_body_mode: None,
    });

    let request_body = anthropic::request::build_request_body(
        &client,
        &client.config.request_url,
        None,
        vec![json!({ "role": "user", "content": [{ "type": "text", "text": "hello" }] })],
        None,
        None,
    );

    assert_eq!(request_body["thinking"]["type"], "enabled");
    assert_eq!(request_body["thinking"]["budget_tokens"], 3000);
}

#[test]
fn build_anthropic_request_body_default_deepseek_reasoning_omits_thinking_fields() {
    let client = AIClient::new(AIConfig {
        name: "deepseek".to_string(),
        base_url: "https://api.deepseek.com/anthropic".to_string(),
        request_url: "https://api.deepseek.com/anthropic/v1/messages".to_string(),
        api_key: "test-key".to_string(),
        model: "deepseek-v4-flash".to_string(),
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
        reasoning_effort: Some("high".to_string()),
        thinking_budget_tokens: None,
        custom_request_body: None,
        custom_request_body_mode: None,
    });

    let request_body = anthropic::request::build_request_body(
        &client,
        &client.config.request_url,
        None,
        vec![json!({ "role": "user", "content": [{ "type": "text", "text": "hello" }] })],
        None,
        None,
    );

    assert!(request_body.get("thinking").is_none());
    assert!(request_body.get("output_config").is_none());
}

#[test]
fn build_anthropic_request_body_disabled_deepseek_reasoning_omits_effort() {
    let client = AIClient::new(AIConfig {
        name: "deepseek".to_string(),
        base_url: "https://api.deepseek.com/anthropic".to_string(),
        request_url: "https://api.deepseek.com/anthropic/v1/messages".to_string(),
        api_key: "test-key".to_string(),
        model: "deepseek-v4-flash".to_string(),
        format: "anthropic".to_string(),
        context_window: 200000,
        max_tokens: Some(8192),
        temperature: None,
        top_p: None,
        reasoning_mode: ReasoningMode::Disabled,
        inline_think_in_text: false,
        custom_headers: None,
        custom_headers_mode: None,
        skip_ssl_verify: false,
        reasoning_effort: Some("high".to_string()),
        thinking_budget_tokens: None,
        custom_request_body: None,
        custom_request_body_mode: None,
    });

    let request_body = anthropic::request::build_request_body(
        &client,
        &client.config.request_url,
        None,
        vec![json!({ "role": "user", "content": [{ "type": "text", "text": "hello" }] })],
        None,
        None,
    );

    assert_eq!(request_body["thinking"]["type"], "disabled");
    assert!(request_body.get("output_config").is_none());
}

#[test]
fn build_anthropic_request_body_adaptive_deepseek_reasoning_falls_back_to_enabled() {
    let client = AIClient::new(AIConfig {
        name: "deepseek".to_string(),
        base_url: "https://api.deepseek.com/anthropic".to_string(),
        request_url: "https://api.deepseek.com/anthropic/v1/messages".to_string(),
        api_key: "test-key".to_string(),
        model: "deepseek-v4-flash".to_string(),
        format: "anthropic".to_string(),
        context_window: 200000,
        max_tokens: Some(8192),
        temperature: None,
        top_p: None,
        reasoning_mode: ReasoningMode::Adaptive,
        inline_think_in_text: false,
        custom_headers: None,
        custom_headers_mode: None,
        skip_ssl_verify: false,
        reasoning_effort: Some("high".to_string()),
        thinking_budget_tokens: Some(4096),
        custom_request_body: None,
        custom_request_body_mode: None,
    });

    let request_body = anthropic::request::build_request_body(
        &client,
        &client.config.request_url,
        None,
        vec![json!({ "role": "user", "content": [{ "type": "text", "text": "hello" }] })],
        None,
        None,
    );

    assert_eq!(request_body["thinking"]["type"], "enabled");
    assert!(request_body["thinking"].get("budget_tokens").is_none());
    assert_eq!(request_body["output_config"]["effort"], "high");
}
