//! Trim-mode request-body preservation tests across all four providers.
//!
//! When `custom_request_body_mode == "trim"`, the adapter protects a small set
//! of essential fields from being overwritten by the user-supplied
//! `custom_request_body`. These tests verify the protected field set holds
//! across OpenAI Chat, OpenAI Responses, Anthropic, and Gemini providers.

#![cfg(test)]

use crate::providers::{anthropic, gemini, gemini::GeminiMessageConverter, openai};
use crate::types::{ReasoningMode, ToolDefinition};
use serde_json::json;

use super::helpers::make_trim_test_client;

#[test]
fn build_openai_request_body_trim_mode_preserves_essential_fields() {
    let mut client = make_trim_test_client("openai");
    client.config.base_url = "https://api.deepseek.com/v1".to_string();
    client.config.request_url = "https://api.deepseek.com/v1/chat/completions".to_string();
    client.config.model = "deepseek-v4-pro".to_string();
    client.config.max_tokens = Some(8192);
    client.config.reasoning_mode = ReasoningMode::Enabled;
    client.config.reasoning_effort = Some("high".to_string());
    let messages = vec![json!({ "role": "user", "content": "hello" })];

    let request_body = openai::chat::build_request_body(
        &client,
        &client.config.request_url,
        messages.clone(),
        None,
        Some(json!({
            "model": "override-model",
            "messages": [{ "role": "user", "content": "override" }],
            "stream": false,
            "max_tokens": 1,
            "temperature": 0.7,
            "response_format": { "type": "json_object" }
        })),
    );

    assert_eq!(request_body["model"], "deepseek-v4-pro");
    assert_eq!(request_body["messages"], json!(messages));
    assert_eq!(request_body["stream"], true);
    assert_eq!(request_body["max_tokens"], 8192);
    assert_eq!(request_body["temperature"], 0.7);
    assert_eq!(request_body["response_format"]["type"], "json_object");
    assert!(request_body.get("thinking").is_none());
    assert!(request_body.get("reasoning_effort").is_none());
}

#[test]
fn build_responses_request_body_trim_mode_preserves_essential_fields() {
    let mut client = make_trim_test_client("responses");
    client.config.max_tokens = Some(4096);
    let input = vec![json!({
        "role": "user",
        "content": [{ "type": "input_text", "text": "hello" }]
    })];

    let request_body = openai::responses::build_request_body(
        &client,
        Some("Be concise".to_string()),
        input.clone(),
        None,
        Some(json!({
            "instructions": "override me",
            "input": [{ "role": "user", "content": [{ "type": "input_text", "text": "override" }] }],
            "stream": false,
            "max_output_tokens": 1,
            "temperature": 0.1
        })),
    );

    assert_eq!(request_body["model"], "test-model");
    assert_eq!(request_body["input"], json!(input));
    assert_eq!(request_body["instructions"], "Be concise");
    assert_eq!(request_body["stream"], true);
    assert_eq!(request_body["max_output_tokens"], 4096);
    assert_eq!(request_body["temperature"], 0.1);
    assert!(request_body.get("reasoning").is_none());
}

#[test]
fn build_anthropic_request_body_trim_mode_preserves_essential_fields() {
    let mut client = make_trim_test_client("anthropic");
    client.config.max_tokens = Some(8192);
    let messages = vec![json!({
        "role": "user",
        "content": [{ "type": "text", "text": "hello" }]
    })];

    let request_body = anthropic::request::build_request_body(
        &client,
        &client.config.request_url,
        Some("Use the system prompt".to_string()),
        messages.clone(),
        None,
        Some(json!({
            "system": "override me",
            "messages": [{ "role": "user", "content": [{ "type": "text", "text": "override" }] }],
            "max_tokens": 1,
            "stream": false,
            "metadata": { "tag": "kept" }
        })),
    );

    assert_eq!(request_body["model"], "test-model");
    assert_eq!(request_body["messages"], json!(messages));
    assert_eq!(request_body["system"], "Use the system prompt");
    assert_eq!(request_body["stream"], true);
    assert_eq!(request_body["max_tokens"], 8192);
    assert_eq!(request_body["metadata"]["tag"], "kept");
    assert!(request_body.get("thinking").is_none());
}

#[test]
fn build_gemini_request_body_trim_mode_preserves_essential_fields() {
    let mut client = make_trim_test_client("gemini");
    client.config.model = "gemini-2.5-pro".to_string();
    client.config.max_tokens = Some(4096);

    let contents = vec![json!({
        "role": "user",
        "parts": [{ "text": "hello" }]
    })];
    let system_instruction = json!({
        "parts": [{ "text": "system" }]
    });
    let gemini_tools = GeminiMessageConverter::convert_tools(Some(vec![ToolDefinition {
        name: "lookup".to_string(),
        description: "Look up data".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" }
            },
            "required": ["query"]
        }),
    }]));

    let request_body = gemini::request::build_request_body(
        &client,
        Some(system_instruction.clone()),
        contents.clone(),
        gemini_tools,
        Some(json!({
            "contents": [{ "role": "user", "parts": [{ "text": "override" }] }],
            "systemInstruction": { "parts": [{ "text": "override system" }] },
            "generationConfig": {
                "maxOutputTokens": 1,
                "candidateCount": 2
            },
            "tools": [],
            "toolConfig": {
                "functionCallingConfig": {
                    "mode": "NONE"
                }
            },
            "temperature": 0.3
        })),
    );

    assert_eq!(request_body["contents"], json!(contents));
    assert_eq!(request_body["systemInstruction"], system_instruction);
    assert_eq!(request_body["generationConfig"]["maxOutputTokens"], 4096);
    assert_eq!(request_body["generationConfig"]["candidateCount"], 2);
    assert_eq!(request_body["generationConfig"]["temperature"], 0.3);
    assert_eq!(request_body["toolConfig"]["functionCallingConfig"]["mode"], "AUTO");
    assert_eq!(request_body["tools"][0]["functionDeclarations"][0]["name"], "lookup");
}
