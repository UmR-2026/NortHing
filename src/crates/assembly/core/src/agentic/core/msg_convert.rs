use super::msg_types::{Message, MessageContent, ToolCall};
use crate::util::types::{Message as AIMessage, ToolCall as AIToolCall};
use std::fmt::{self, Display};
use tracing::warn;

impl From<Message> for AIMessage {
    fn from(msg: Message) -> Self {
        let role = match msg.role {
            super::msg_types::MessageRole::User => "user",
            super::msg_types::MessageRole::Assistant => "assistant",
            super::msg_types::MessageRole::Tool => "tool",
            super::msg_types::MessageRole::System => "system",
        };
        let thinking_signature = msg.metadata.thinking_signature.clone();

        match msg.content {
            MessageContent::Text(text) => {
                // Check if text is empty to avoid sending empty content to API
                let content = if text.trim().is_empty() {
                    // Should not have empty text messages, but provide default value for defensive programming
                    warn!("Empty text message detected: role={}", role);
                    if role == "user" {
                        Some("(empty message)".to_string())
                    } else if role == "system" {
                        Some("You are a helpful assistant.".to_string())
                    } else {
                        Some(" ".to_string()) // Minimum valid value
                    }
                } else {
                    Some(text)
                };

                Self {
                    role: role.to_string(),
                    content,
                    reasoning_content: None,
                    thinking_signature: None,
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    is_error: None,
                    tool_image_attachments: None,
                }
            }
            MessageContent::Multimodal { text, images } => {
                let mut content = text;
                if !images.is_empty() {
                    content.push_str("\n\n[Attached image(s):\n");
                    for image in images {
                        let name = image
                            .metadata
                            .as_ref()
                            .and_then(|m| m.get("name"))
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(str::to_string)
                            .or_else(|| image.image_path.as_ref().filter(|s| !s.is_empty()).cloned())
                            .unwrap_or_else(|| image.id.clone());

                        content.push_str(&format!("- {} ({})\n", name, image.mime_type));
                    }
                    content.push(']');
                }

                Self {
                    role: "user".to_string(),
                    content: Some(content),
                    reasoning_content: None,
                    thinking_signature: None,
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    is_error: None,
                    tool_image_attachments: None,
                }
            }
            MessageContent::Mixed {
                reasoning_content,
                text,
                tool_calls,
            } => {
                let converted_tool_calls = if tool_calls.is_empty() {
                    // Set to None when tool_call is empty to avoid deepseek model errors
                    None
                } else {
                    Some(
                        tool_calls
                            .into_iter()
                            .map(|tc| AIToolCall {
                                id: tc.tool_id,
                                name: tc.tool_name,
                                arguments: tc.arguments,
                                raw_arguments: tc.raw_arguments,
                            })
                            .collect(),
                    )
                };

                // When there are tool_calls, empty text should use None
                let content = if text.trim().is_empty() {
                    None // OpenAI API allows content to be null when assistant + tool_calls
                } else {
                    Some(text)
                };

                // Reasoning content (interleaved thinking mode)
                Self {
                    role: "assistant".to_string(),
                    content,
                    reasoning_content,
                    thinking_signature: thinking_signature.clone(),
                    tool_calls: converted_tool_calls,
                    tool_call_id: None,
                    name: None,
                    is_error: None,
                    tool_image_attachments: None,
                }
            }
            MessageContent::ToolResult {
                tool_id,
                tool_name,
                result,
                result_for_assistant,
                is_error,
                image_attachments,
            } => {
                // Tool messages must include tool_call_id
                // Prefer result_for_assistant (text specifically for AI), if None or empty then use result (data field)
                let content_for_ai = if let Some(assistant_text) = result_for_assistant {
                    // Check if empty string
                    if assistant_text.trim().is_empty() {
                        // If empty, use serialized result
                        serde_json::to_string(&result).unwrap_or(format!("Tool {} execution completed", tool_name))
                    } else {
                        assistant_text
                    }
                } else {
                    // If no result_for_assistant, use serialized result
                    serde_json::to_string(&result).unwrap_or(format!("Tool {} execution completed", tool_name))
                };

                Self {
                    role: "tool".to_string(),
                    content: Some(content_for_ai),
                    reasoning_content: None,
                    thinking_signature: None,
                    tool_calls: None,
                    tool_call_id: Some(tool_id),
                    name: Some(tool_name),
                    is_error: Some(is_error),
                    tool_image_attachments: image_attachments.clone(),
                }
            }
        }
    }
}

impl From<&Message> for AIMessage {
    fn from(msg: &Message) -> Self {
        // Reference version calls owned version after clone to avoid duplicate logic
        AIMessage::from(msg.clone())
    }
}

impl Display for MessageContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageContent::Text(text) => write!(f, "{}", text),
            MessageContent::Multimodal { text, images } => {
                write!(f, "Multimodal: text_length={}, images={}", text.len(), images.len())
            }
            MessageContent::ToolResult {
                tool_id,
                tool_name,
                result,
                result_for_assistant,
                is_error,
                image_attachments,
            } => write!(
                f,
                "ToolResult: tool_id={}, tool_name={}, result={}, result_for_assistant={:?}, is_error={}, images={}",
                tool_id,
                tool_name,
                result,
                result_for_assistant,
                is_error,
                image_attachments.as_ref().map(|v| v.len()).unwrap_or(0)
            ),
            MessageContent::Mixed {
                reasoning_content,
                text,
                tool_calls,
            } => write!(
                f,
                "Mixed: reasoning_content={:?}, text={}, tool_calls={}",
                reasoning_content,
                text,
                tool_calls
                    .iter()
                    .map(|tc| format!(
                        "ToolCall: tool_id={}, tool_name={}, arguments={}",
                        tc.tool_id, tc.tool_name, tc.arguments
                    ))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

impl From<ToolCall> for AIToolCall {
    fn from(tc: ToolCall) -> Self {
        Self {
            id: tc.tool_id.clone(),
            name: tc.tool_name.clone(),
            arguments: tc.arguments,
            raw_arguments: tc.raw_arguments,
        }
    }
}
