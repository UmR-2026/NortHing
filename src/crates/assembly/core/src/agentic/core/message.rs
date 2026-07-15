pub use msg_types::{
    CompressedMessage, CompressedMessageRole, CompressedTodoItem, CompressedTodoSnapshot, CompressedToolCall,
    CompressionEntry, CompressionPayload, InternalReminderKind, Message, MessageContent, MessageMetadata, MessageRole,
    MessageSemanticKind, ToolCall, ToolResult,
};
pub use northhing_runtime_ports::{CompressionContract, CompressionContractItem};

#[path = "msg_build.rs"]
mod msg_build;
#[path = "msg_convert.rs"]
mod msg_convert;
#[path = "msg_types.rs"]
mod msg_types;

#[cfg(test)]
mod tests {
    use super::Message;
    use crate::util::types::Message as AIMessage;

    #[test]
    fn preserves_empty_reasoning_content_for_provider_replay() {
        let msg = Message::assistant_with_reasoning(Some(String::new()), String::new(), vec![])
            .with_thinking_signature(Some("sig_1".to_string()));

        let ai_msg = AIMessage::from(msg);

        assert_eq!(ai_msg.reasoning_content.as_deref(), Some(""));
        assert_eq!(ai_msg.thinking_signature.as_deref(), Some("sig_1"));
    }
}
