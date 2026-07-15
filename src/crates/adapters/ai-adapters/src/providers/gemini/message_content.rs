//! Convert northhing `Message` values into Gemini `system_instruction` and
//! `contents` arrays.
//!
//! The main entry point is [`convert_messages`]. Helpers in this file are
//! module-private — they only support message conversion.

use crate::types::Message;
use serde_json::{json, Map, Value};
use tracing::warn;

/// Convert a list of northhing messages into Gemini's `(system_instruction,
/// contents)` shape.
pub fn convert_messages(messages: Vec<Message>, model_name: &str) -> (Option<Value>, Vec<Value>) {
    let mut system_texts = Vec::new();
    let mut contents = Vec::new();
    let is_gemini_3 = model_name.contains("gemini-3");

    for msg in messages {
        match msg.role.as_str() {
            "system" => {
                if let Some(content) = msg.content.filter(|content| !content.trim().is_empty()) {
                    system_texts.push(content);
                }
            }
            "user" => {
                let parts = convert_content_parts(msg.content.as_deref(), false);
                push_content(&mut contents, "user", parts);
            }
            "assistant" => {
                let mut parts = Vec::new();

                let mut pending_thought_signature = msg.thinking_signature.filter(|value| !value.trim().is_empty());
                let has_tool_calls = msg
                    .tool_calls
                    .as_ref()
                    .map(|tool_calls| !tool_calls.is_empty())
                    .unwrap_or(false);

                if let Some(content) = msg.content.as_deref().filter(|value| !value.trim().is_empty()) {
                    if !has_tool_calls {
                        if let Some(signature) = pending_thought_signature.take() {
                            parts.push(json!({
                                "thoughtSignature": signature,
                            }));
                        }
                    }
                    parts.extend(convert_content_parts(Some(content), true));
                }

                if let Some(tool_calls) = msg.tool_calls {
                    for (tool_call_index, tool_call) in tool_calls.into_iter().enumerate() {
                        let mut part = Map::new();
                        part.insert(
                            "functionCall".to_string(),
                            json!({
                                "name": tool_call.name,
                                "args": tool_call.arguments,
                            }),
                        );

                        match pending_thought_signature.take() {
                            Some(signature) => {
                                part.insert("thoughtSignature".to_string(), Value::String(signature));
                            }
                            None if is_gemini_3 && tool_call_index == 0 => {
                                part.insert(
                                    "thoughtSignature".to_string(),
                                    Value::String("skip_thought_signature_validator".to_string()),
                                );
                            }
                            None => {}
                        }

                        parts.push(Value::Object(part));
                    }
                }

                if let Some(signature) = pending_thought_signature {
                    parts.push(json!({
                        "thoughtSignature": signature,
                    }));
                }

                push_content(&mut contents, "model", parts);
            }
            "tool" => {
                let tool_name = msg.name.unwrap_or_default();
                if tool_name.is_empty() {
                    warn!("Skipping Gemini tool response without tool name");
                    continue;
                }

                let is_error = msg.is_error.unwrap_or(false);
                let response = if is_error {
                    let error_text = msg
                        .content
                        .as_deref()
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or("Tool execution failed");
                    json!({ "error": error_text })
                } else {
                    parse_tool_response(msg.content.as_deref())
                };
                let parts = vec![json!({
                    "functionResponse": {
                        "name": tool_name,
                        "response": response,
                    }
                })];

                push_content(&mut contents, "user", parts);
            }
            _ => {
                warn!("Unknown Gemini message role: {}", msg.role);
            }
        }
    }

    let system_instruction = if system_texts.is_empty() {
        None
    } else {
        Some(json!({
            "parts": [{
                "text": system_texts.join("\n\n")
            }]
        }))
    };

    (system_instruction, contents)
}

fn push_content(contents: &mut Vec<Value>, role: &str, parts: Vec<Value>) {
    if parts.is_empty() {
        return;
    }

    if let Some(last) = contents.last_mut() {
        let last_role = last.get("role").and_then(Value::as_str).unwrap_or_default();
        if last_role == role {
            if let Some(existing_parts) = last.get_mut("parts").and_then(Value::as_array_mut) {
                existing_parts.extend(parts);
                return;
            }
        }
    }

    contents.push(json!({
        "role": role,
        "parts": parts,
    }));
}

fn convert_content_parts(content: Option<&str>, is_model_role: bool) -> Vec<Value> {
    let Some(content) = content else {
        return Vec::new();
    };

    if content.trim().is_empty() {
        return Vec::new();
    }

    let parsed = match serde_json::from_str::<Value>(content) {
        Ok(parsed) if parsed.is_array() => parsed,
        _ => return vec![json!({ "text": content })],
    };

    let mut parts = Vec::new();

    if let Some(items) = parsed.as_array() {
        for item in items {
            let item_type = item.get("type").and_then(Value::as_str);
            match item_type {
                Some("text") | Some("input_text") | Some("output_text") => {
                    if let Some(text) = item.get("text").and_then(Value::as_str) {
                        if !text.is_empty() {
                            parts.push(json!({ "text": text }));
                        }
                    }
                }
                Some("image_url") if !is_model_role => {
                    if let Some(url) = item
                        .get("image_url")
                        .and_then(|value| value.get("url").and_then(Value::as_str).or_else(|| value.as_str()))
                    {
                        if let Some(part) = convert_image_url_to_part(url) {
                            parts.push(part);
                        }
                    }
                }
                Some("image") if !is_model_role => {
                    let source = item.get("source");
                    let mime_type = source.and_then(|value| value.get("media_type")).and_then(Value::as_str);
                    let data = source.and_then(|value| value.get("data")).and_then(Value::as_str);

                    if let (Some(mime_type), Some(data)) = (mime_type, data) {
                        parts.push(json!({
                            "inlineData": {
                                "mimeType": mime_type,
                                "data": data,
                            }
                        }));
                    }
                }
                _ => {}
            }
        }
    }

    if parts.is_empty() {
        vec![json!({ "text": content })]
    } else {
        parts
    }
}

fn convert_image_url_to_part(url: &str) -> Option<Value> {
    let prefix = "data:";
    if !url.starts_with(prefix) {
        warn!("Gemini currently supports inline data URLs for image parts; skipping unsupported image URL");
        return None;
    }

    let rest = &url[prefix.len()..];
    let (mime_type, data) = rest.split_once(";base64,")?;
    if mime_type.is_empty() || data.is_empty() {
        return None;
    }

    Some(json!({
        "inlineData": {
            "mimeType": mime_type,
            "data": data,
        }
    }))
}

fn parse_tool_response(content: Option<&str>) -> Value {
    let Some(content) = content.filter(|value| !value.trim().is_empty()) else {
        return json!({ "content": "Tool execution completed" });
    };

    match serde_json::from_str::<Value>(content) {
        Ok(Value::Object(map)) => Value::Object(map),
        Ok(value) => json!({ "content": value }),
        Err(_) => json!({ "content": content }),
    }
}
