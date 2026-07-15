use super::msg_types::{
    InternalReminderKind, Message, MessageContent, MessageMetadata, MessageRole, MessageSemanticKind, ToolCall,
    ToolResult,
};
use crate::util::TokenCounter;

impl Message {
    pub fn system(text: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::System,
            content: MessageContent::Text(text),
            timestamp: std::time::SystemTime::now(),
            metadata: MessageMetadata::default(),
        }
    }

    pub fn user(text: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content: MessageContent::Text(text),
            timestamp: std::time::SystemTime::now(),
            metadata: MessageMetadata::default(),
        }
    }

    pub fn internal_reminder(reminder_kind: InternalReminderKind, text: impl Into<String>) -> Self {
        let text = text.into();
        let rendered = if crate::agentic::core::has_prompt_markup(&text) {
            text
        } else {
            crate::agentic::core::render_system_reminder(&text)
        };
        Self::user(rendered)
            .with_semantic_kind(MessageSemanticKind::InternalReminder)
            .with_internal_reminder_kind(reminder_kind)
    }

    pub fn user_multimodal(text: String, images: Vec<crate::agentic::image_analysis::ImageContextData>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content: MessageContent::Multimodal { text, images },
            timestamp: std::time::SystemTime::now(),
            metadata: MessageMetadata::default(),
        }
    }

    pub fn assistant(text: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content: MessageContent::Text(text),
            timestamp: std::time::SystemTime::now(),
            metadata: MessageMetadata::default(),
        }
    }

    pub fn assistant_with_tools(text: String, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content: MessageContent::Mixed {
                reasoning_content: None,
                text,
                tool_calls,
            },
            timestamp: std::time::SystemTime::now(),
            metadata: MessageMetadata::default(),
        }
    }

    /// Create assistant message with reasoning content (supports interleaved thinking mode)
    pub fn assistant_with_reasoning(
        reasoning_content: Option<String>,
        text: String,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content: MessageContent::Mixed {
                reasoning_content,
                text,
                tool_calls,
            },
            timestamp: std::time::SystemTime::now(),
            metadata: MessageMetadata::default(),
        }
    }

    pub fn tool_result(result: ToolResult) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::Tool,
            content: MessageContent::ToolResult {
                tool_id: result.tool_id.clone(),
                tool_name: result.tool_name.clone(),
                result: result.result.clone(),
                result_for_assistant: result.result_for_assistant.clone(),
                is_error: result.is_error,
                image_attachments: result.image_attachments.clone(),
            },
            timestamp: std::time::SystemTime::now(),
            metadata: MessageMetadata::default(),
        }
    }

    /// Check if message should be treated as an actual user-turn boundary.
    pub fn is_actual_user_message(&self) -> bool {
        if self.role != MessageRole::User {
            return false;
        }
        if let Some(semantic_kind) = self.metadata.semantic_kind {
            return semantic_kind == MessageSemanticKind::ActualUserInput;
        }
        let text = match &self.content {
            MessageContent::Text(text) => Some(text.as_str()),
            MessageContent::Multimodal { text, .. } => Some(text.as_str()),
            _ => None,
        };
        if text.is_some_and(crate::agentic::core::prompt_markup::is_system_reminder_only) {
            return false;
        }
        true
    }

    /// Set message's turn_id (to identify which dialog turn the message belongs to)
    pub fn with_turn_id(mut self, turn_id: String) -> Self {
        self.metadata.turn_id = Some(turn_id);
        self
    }

    /// Set message's round_id (to identify which model round the message belongs to)
    pub fn with_round_id(mut self, round_id: String) -> Self {
        self.metadata.round_id = Some(round_id);
        self
    }

    pub fn with_semantic_kind(mut self, semantic_kind: MessageSemanticKind) -> Self {
        self.metadata.semantic_kind = Some(semantic_kind);
        self
    }

    pub fn with_internal_reminder_kind(mut self, reminder_kind: InternalReminderKind) -> Self {
        self.metadata.internal_reminder_kind = Some(reminder_kind);
        self
    }

    pub fn internal_reminder_kind(&self) -> Option<InternalReminderKind> {
        self.metadata.internal_reminder_kind
    }

    pub fn with_compression_payload(mut self, compression_payload: super::msg_types::CompressionPayload) -> Self {
        self.metadata.compression_payload = Some(compression_payload);
        self.metadata.tokens = None;
        self
    }

    /// Set message's thinking_signature (for Anthropic extended thinking multi-turn conversations)
    pub fn with_thinking_signature(mut self, signature: Option<String>) -> Self {
        self.metadata.thinking_signature = signature;
        self
    }

    /// Get message's token count
    pub fn tokens(&mut self) -> usize {
        if let Some(tokens) = self.metadata.tokens {
            return tokens;
        }
        let tokens = self.estimate_tokens();
        self.metadata.tokens = Some(tokens);
        tokens
    }

    fn estimate_image_tokens(metadata: Option<&serde_json::Value>) -> usize {
        let (width, height) = metadata
            .and_then(|m| {
                let w = m.get("width").and_then(|v| v.as_u64());
                let h = m.get("height").and_then(|v| v.as_u64());
                match (w, h) {
                    (Some(w), Some(h)) if w > 0 && h > 0 => Some((w as u32, h as u32)),
                    _ => None,
                }
            })
            .unwrap_or((1024, 1024));

        let tiles_w = width.div_ceil(512);
        let tiles_h = height.div_ceil(512);
        let tiles = (tiles_w.max(1) * tiles_h.max(1)) as usize;
        50 + tiles * 200
    }

    pub fn estimate_tokens_with_reasoning(&self, include_reasoning: bool) -> usize {
        let mut total = 0usize;
        total += 4;

        match &self.content {
            MessageContent::Text(text) => {
                total += TokenCounter::estimate_tokens(text);
            }
            MessageContent::Multimodal { text, images } => {
                total += TokenCounter::estimate_tokens(text);
                for image in images {
                    total += Self::estimate_image_tokens(image.metadata.as_ref());
                }
            }
            MessageContent::Mixed {
                reasoning_content,
                text,
                tool_calls,
            } => {
                if include_reasoning {
                    if let Some(reasoning) = reasoning_content.as_ref() {
                        total += TokenCounter::estimate_tokens(reasoning);
                    }
                }
                total += TokenCounter::estimate_tokens(text);

                for tool_call in tool_calls {
                    total += TokenCounter::estimate_tokens(&tool_call.tool_name);
                    let serialized_arguments = tool_call
                        .raw_arguments
                        .clone()
                        .filter(|raw| serde_json::from_str::<serde_json::Value>(raw).is_ok())
                        .unwrap_or_else(|| {
                            serde_json::to_string(&tool_call.arguments).unwrap_or_else(|_| "{}".to_string())
                        });
                    total += TokenCounter::estimate_tokens(&serialized_arguments);
                    total += 10;
                }
            }
            MessageContent::ToolResult {
                tool_name,
                result,
                result_for_assistant,
                image_attachments,
                ..
            } => {
                if let Some(text) = result_for_assistant.as_ref().filter(|s| !s.is_empty()) {
                    total += TokenCounter::estimate_tokens(text);
                } else if let Ok(json_str) = serde_json::to_string(result) {
                    total += TokenCounter::estimate_tokens(&json_str);
                } else {
                    total += TokenCounter::estimate_tokens(tool_name);
                }
                if let Some(imgs) = image_attachments {
                    for _ in imgs {
                        total += Self::estimate_image_tokens(None);
                    }
                }
            }
        }

        total
    }

    fn estimate_tokens(&self) -> usize {
        self.estimate_tokens_with_reasoning(true)
    }
}
