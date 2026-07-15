//! Gemini message format converter
//!
//! This module is a thin facade that re-exports the three sub-domain
//! implementations as static methods on `GeminiMessageConverter`:
//!
//! - [`message_content`] — northhing `Message` -> Gemini `contents`
//! - [`tool_conversion`] — northhing `ToolDefinition` -> Gemini native tools
//!   and/or function declarations
//! - [`schema_sanitizer`] — strip unsupported JSON-schema fields from tool
//!   and response schemas

use crate::types::Message;
use serde_json::Value;

use super::message_content;
use super::schema_sanitizer;
use super::tool_conversion;

pub struct GeminiMessageConverter;

impl GeminiMessageConverter {
    /// Convert northhing messages to Gemini `system_instruction` and `contents`.
    pub fn convert_messages(messages: Vec<Message>, model_name: &str) -> (Option<Value>, Vec<Value>) {
        message_content::convert_messages(messages, model_name)
    }

    /// Convert northhing tool definitions to Gemini native tools and/or
    /// function declarations.
    pub fn convert_tools(tools: Option<Vec<crate::types::ToolDefinition>>) -> Option<Vec<Value>> {
        tool_conversion::convert_tools(tools)
    }

    /// Strip JSON-schema fields that Gemini's tool/response schema does not
    /// accept.
    pub fn sanitize_schema(value: Value) -> Value {
        schema_sanitizer::sanitize_schema(value)
    }
}

#[cfg(test)]
mod tests {
    use super::GeminiMessageConverter;
    use crate::types::{Message, ToolCall, ToolDefinition};
    use serde_json::json;

    #[test]
    fn converts_messages_to_gemini_format() {
        let messages = vec![
            Message::system("You are helpful".to_string()),
            Message::user("Hello".to_string()),
            Message {
                role: "assistant".to_string(),
                content: Some("Working on it".to_string()),
                reasoning_content: Some("Let me think".to_string()),
                thinking_signature: Some("sig_1".to_string()),
                tool_calls: Some(vec![ToolCall {
                    id: "call_1".to_string(),
                    name: "get_weather".to_string(),
                    arguments: json!({"city": "Beijing"}),
                    raw_arguments: None,
                }]),
                tool_call_id: None,
                name: None,
                is_error: None,
                tool_image_attachments: None,
            },
            Message {
                role: "tool".to_string(),
                content: Some("Sunny".to_string()),
                reasoning_content: None,
                thinking_signature: None,
                tool_calls: None,
                tool_call_id: Some("call_1".to_string()),
                name: Some("get_weather".to_string()),
                is_error: None,
                tool_image_attachments: None,
            },
        ];

        let (system_instruction, contents) = GeminiMessageConverter::convert_messages(messages, "gemini-2.5-pro");

        assert_eq!(
            system_instruction.unwrap()["parts"][0]["text"],
            json!("You are helpful")
        );
        assert_eq!(contents.len(), 3);
        assert_eq!(contents[0]["role"], json!("user"));
        assert_eq!(contents[1]["role"], json!("model"));
        assert_eq!(contents[1]["parts"][0]["text"], json!("Working on it"));
        assert_eq!(contents[1]["parts"][1]["functionCall"]["name"], json!("get_weather"));
        assert_eq!(contents[1]["parts"][1]["thoughtSignature"], json!("sig_1"));
        assert_eq!(
            contents[2]["parts"][0]["functionResponse"]["name"],
            json!("get_weather")
        );
    }

    #[test]
    fn injects_skip_signature_for_first_synthetic_gemini_3_tool_call() {
        let messages = vec![Message {
            role: "assistant".to_string(),
            content: None,
            reasoning_content: None,
            thinking_signature: None,
            tool_calls: Some(vec![ToolCall {
                id: "call_1".to_string(),
                name: "get_weather".to_string(),
                arguments: json!({"city": "Paris"}),
                raw_arguments: None,
            }]),
            tool_call_id: None,
            name: None,
            is_error: None,
            tool_image_attachments: None,
        }];

        let (_, contents) = GeminiMessageConverter::convert_messages(messages, "gemini-3-flash-preview");

        assert_eq!(contents.len(), 1);
        assert_eq!(
            contents[0]["parts"][0]["thoughtSignature"],
            json!("skip_thought_signature_validator")
        );
    }

    #[test]
    fn converts_data_url_images_to_inline_data() {
        let messages = vec![Message {
            role: "user".to_string(),
            content: Some(
                json!([
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": "data:image/png;base64,abc"
                        }
                    },
                    {
                        "type": "text",
                        "text": "Describe this image"
                    }
                ])
                .to_string(),
            ),
            reasoning_content: None,
            thinking_signature: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            is_error: None,
            tool_image_attachments: None,
        }];

        let (_, contents) = GeminiMessageConverter::convert_messages(messages, "gemini-2.5-pro");

        assert_eq!(contents[0]["parts"][0]["inlineData"]["mimeType"], json!("image/png"));
        assert_eq!(contents[0]["parts"][1]["text"], json!("Describe this image"));
    }

    #[test]
    fn strips_unsupported_fields_from_tool_schema() {
        let tools = Some(vec![ToolDefinition {
            name: "get_weather".to_string(),
            description: "Get weather".to_string(),
            parameters: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "city": { "type": "string" },
                    "timezone": {
                        "type": ["string", "null"]
                    },
                    "link": {
                        "anyOf": [
                            {
                                "type": "object",
                                "properties": {
                                    "url": { "type": "string" }
                                },
                                "required": ["url"]
                            },
                            { "type": "null" }
                        ]
                    },
                    "items": {
                        "allOf": [
                            {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string" }
                                },
                                "required": ["name"]
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "count": { "type": "integer" }
                                },
                                "required": ["count"]
                            }
                        ]
                    }
                },
                "required": ["city"],
                "additionalProperties": false,
                "items": {
                    "type": "object",
                    "additionalProperties": false
                }
            }),
        }]);

        let converted = GeminiMessageConverter::convert_tools(tools).expect("converted tools");
        let schema = &converted[0]["functionDeclarations"][0]["parameters"];

        assert!(schema.get("$schema").is_none());
        assert!(schema.get("additionalProperties").is_none());
        assert!(schema["items"].get("additionalProperties").is_none());
        assert_eq!(schema["properties"]["timezone"]["type"], json!("string"));
        assert_eq!(schema["properties"]["timezone"]["nullable"], json!(true));
        assert_eq!(schema["properties"]["link"]["type"], json!("object"));
        assert_eq!(schema["properties"]["link"]["nullable"], json!(true));
        assert_eq!(schema["properties"]["items"]["type"], json!("object"));
        assert_eq!(schema["properties"]["items"]["required"], json!(["name", "count"]));
    }

    #[test]
    fn maps_web_search_to_native_google_search_tool() {
        let tools = Some(vec![ToolDefinition {
            name: "WebSearch".to_string(),
            description: "Search the web".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                },
                "required": ["query"]
            }),
        }]);

        let converted = GeminiMessageConverter::convert_tools(tools).expect("converted tools");
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0]["googleSearch"], json!({}));
        assert!(converted[0].get("functionDeclarations").is_none());
    }

    #[test]
    fn falls_back_to_function_declarations_when_native_and_custom_tools_mix() {
        let tools = Some(vec![
            ToolDefinition {
                name: "WebSearch".to_string(),
                description: "Search the web".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" }
                    }
                }),
            },
            ToolDefinition {
                name: "get_weather".to_string(),
                description: "Get weather".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "city": { "type": "string" }
                    },
                    "required": ["city"]
                }),
            },
        ]);

        let converted = GeminiMessageConverter::convert_tools(tools).expect("converted tools");
        assert_eq!(converted.len(), 1);
        assert!(converted[0].get("googleSearch").is_none());
        assert_eq!(converted[0]["functionDeclarations"][0]["name"], json!("get_weather"));
        assert_eq!(converted[0]["functionDeclarations"][1]["name"], json!("WebSearch"));
    }

    #[test]
    fn maps_web_fetch_to_native_url_context_tool() {
        let tools = Some(vec![ToolDefinition {
            name: "WebFetch".to_string(),
            description: "Fetch a URL".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string" }
                },
                "required": ["url"]
            }),
        }]);

        let converted = GeminiMessageConverter::convert_tools(tools).expect("converted tools");
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0]["urlContext"], json!({}));
    }
}
