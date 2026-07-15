//! OpenAI Chat / Responses + Gemini request-body construction tests.
//!
//! Covers the non-trim-mode `build_request_body` paths for OpenAI Chat, OpenAI
//! Responses, and Gemini provider adapters. Anthropic body tests live in
//! `request_bodies_anthropic.rs`; trim-mode tests live in
//! `request_bodies_trim.rs`.

#![cfg(test)]

use crate::client::AIClient;
use crate::providers::{gemini, gemini::GeminiMessageConverter, openai};
use crate::types::{AIConfig, ReasoningMode, ToolDefinition};
use serde_json::json;

#[test]
fn build_gemini_request_body_translates_response_format_and_merges_generation_config() {
    let client = AIClient::new(AIConfig {
        name: "gemini".to_string(),
        base_url: "https://example.com".to_string(),
        request_url: "https://example.com/models/gemini-2.5-pro:streamGenerateContent?alt=sse".to_string(),
        api_key: "test-key".to_string(),
        model: "gemini-2.5-pro".to_string(),
        format: "gemini".to_string(),
        context_window: 128000,
        max_tokens: Some(4096),
        temperature: Some(0.2),
        top_p: Some(0.8),
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

    let request_body = gemini::request::build_request_body(
        &client,
        None,
        vec![json!({
            "role": "user",
            "parts": [{ "text": "hello" }]
        })],
        None,
        Some(json!({
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "schema": {
                        "type": "object",
                        "properties": {
                            "answer": { "type": "string" }
                        },
                        "required": ["answer"],
                        "additionalProperties": false
                    }
                }
            },
            "stop": ["END"],
            "generationConfig": {
                "candidateCount": 1
            }
        })),
    );

    assert_eq!(request_body["generationConfig"]["maxOutputTokens"], 4096);
    assert_eq!(request_body["generationConfig"]["temperature"], 0.2);
    assert_eq!(request_body["generationConfig"]["topP"], 0.8);
    assert_eq!(
        request_body["generationConfig"]["thinkingConfig"]["includeThoughts"],
        true
    );
    assert_eq!(request_body["generationConfig"]["responseMimeType"], "application/json");
    assert_eq!(request_body["generationConfig"]["candidateCount"], 1);
    assert_eq!(request_body["generationConfig"]["stopSequences"], json!(["END"]));
    assert_eq!(
        request_body["generationConfig"]["responseJsonSchema"]["required"],
        json!(["answer"])
    );
    assert!(request_body["generationConfig"]["responseJsonSchema"]
        .get("additionalProperties")
        .is_none());
    assert!(request_body.get("response_format").is_none());
    assert!(request_body.get("stop").is_none());
}

#[test]
fn build_gemini_request_body_omits_function_calling_config_for_native_only_tools() {
    let client = AIClient::new(AIConfig {
        name: "gemini".to_string(),
        base_url: "https://example.com".to_string(),
        request_url: "https://example.com/models/gemini-2.5-pro:streamGenerateContent?alt=sse".to_string(),
        api_key: "test-key".to_string(),
        model: "gemini-2.5-pro".to_string(),
        format: "gemini".to_string(),
        context_window: 128000,
        max_tokens: Some(4096),
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

    let gemini_tools = GeminiMessageConverter::convert_tools(Some(vec![ToolDefinition {
        name: "WebSearch".to_string(),
        description: "Search the web".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" }
            }
        }),
    }]));

    let request_body = gemini::request::build_request_body(
        &client,
        None,
        vec![json!({
            "role": "user",
            "parts": [{ "text": "hello" }]
        })],
        gemini_tools,
        None,
    );

    assert_eq!(request_body["tools"][0]["googleSearch"], json!({}));
    assert!(request_body.get("toolConfig").is_none());
}

#[test]
fn build_openai_request_body_uses_generic_thinking_object_when_enabled() {
    let client = AIClient::new(AIConfig {
        name: "openai-compatible".to_string(),
        base_url: "https://example.com/v1".to_string(),
        request_url: "https://example.com/v1/chat/completions".to_string(),
        api_key: "test-key".to_string(),
        model: "test-model".to_string(),
        format: "openai".to_string(),
        context_window: 128000,
        max_tokens: Some(4096),
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

    let request_body = openai::chat::build_request_body(
        &client,
        &client.config.request_url,
        vec![json!({ "role": "user", "content": "hello" })],
        None,
        None,
    );

    assert_eq!(request_body["thinking"]["type"], "enabled");
    assert!(request_body.get("enable_thinking").is_none());
    assert!(request_body.get("reasoning_effort").is_none());
    assert!(request_body.get("reasoning_split").is_none());
}

#[test]
fn build_openai_request_body_adds_deepseek_reasoning_effort() {
    let client = AIClient::new(AIConfig {
        name: "deepseek".to_string(),
        base_url: "https://api.deepseek.com/v1".to_string(),
        request_url: "https://api.deepseek.com/v1/chat/completions".to_string(),
        api_key: "test-key".to_string(),
        model: "deepseek-v4-pro".to_string(),
        format: "openai".to_string(),
        context_window: 128000,
        max_tokens: Some(4096),
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

    let request_body = openai::chat::build_request_body(
        &client,
        &client.config.request_url,
        vec![json!({ "role": "user", "content": "hello" })],
        None,
        None,
    );

    assert_eq!(request_body["thinking"]["type"], "enabled");
    assert_eq!(request_body["reasoning_effort"], "max");
}

#[test]
fn build_openai_request_body_omits_deepseek_reasoning_effort_when_disabled() {
    let client = AIClient::new(AIConfig {
        name: "deepseek".to_string(),
        base_url: "https://api.deepseek.com/v1".to_string(),
        request_url: "https://api.deepseek.com/v1/chat/completions".to_string(),
        api_key: "test-key".to_string(),
        model: "deepseek-v4-flash".to_string(),
        format: "openai".to_string(),
        context_window: 128000,
        max_tokens: Some(4096),
        temperature: None,
        top_p: None,
        reasoning_mode: ReasoningMode::Disabled,
        inline_think_in_text: false,
        custom_headers: None,
        custom_headers_mode: None,
        skip_ssl_verify: false,
        reasoning_effort: Some("max".to_string()),
        thinking_budget_tokens: None,
        custom_request_body: None,
        custom_request_body_mode: None,
    });

    let request_body = openai::chat::build_request_body(
        &client,
        &client.config.request_url,
        vec![json!({ "role": "user", "content": "hello" })],
        None,
        None,
    );

    assert_eq!(request_body["thinking"]["type"], "disabled");
    assert!(request_body.get("reasoning_effort").is_none());
}

#[test]
fn build_openai_request_body_uses_enable_thinking_for_siliconflow() {
    let client = AIClient::new(AIConfig {
        name: "siliconflow".to_string(),
        base_url: "https://api.siliconflow.cn/v1".to_string(),
        request_url: "https://api.siliconflow.cn/v1/chat/completions".to_string(),
        api_key: "test-key".to_string(),
        model: "Qwen/Qwen3-Coder-480B-A35B-Instruct".to_string(),
        format: "openai".to_string(),
        context_window: 128000,
        max_tokens: Some(4096),
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

    let request_body = openai::chat::build_request_body(
        &client,
        &client.config.request_url,
        vec![json!({ "role": "user", "content": "hello" })],
        None,
        None,
    );

    assert_eq!(request_body["enable_thinking"], true);
    assert!(request_body.get("thinking").is_none());
}

#[test]
fn build_responses_request_body_maps_disabled_mode_to_none_effort() {
    let client = AIClient::new(AIConfig {
        name: "responses".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        request_url: "https://api.openai.com/v1/responses".to_string(),
        api_key: "test-key".to_string(),
        model: "gpt-5".to_string(),
        format: "responses".to_string(),
        context_window: 128000,
        max_tokens: Some(4096),
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

    let request_body = openai::responses::build_request_body(
        &client,
        Some("Be concise".to_string()),
        vec![json!({
            "role": "user",
            "content": [{ "type": "input_text", "text": "hello" }]
        })],
        None,
        None,
    );

    assert_eq!(request_body["reasoning"]["effort"], "none");
}
