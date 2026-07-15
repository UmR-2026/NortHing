//! Feishu (Lark) bot integration for Remote Connect.
//!
//! Users create their own Feishu bot on the Feishu Open Platform and provide
//! App ID + App Secret.  The desktop receives messages via Feishu's WebSocket
//! long connection and routes them through the shared command router.

mod feishu_actions;
mod feishu_commands;
mod feishu_messages;
mod feishu_types;
mod feishu_webhook;

pub use feishu_actions::*;
pub use feishu_commands::*;
pub use feishu_messages::*;
pub use feishu_types::*;
pub use feishu_webhook::*;

use super::command_router::BotChatState;

#[derive(Debug, Clone)]
pub struct FeishuBot {
    pub(super) config: FeishuConfig,
    pub(super) token: std::sync::Arc<tokio::sync::RwLock<Option<FeishuToken>>>,
    pub(super) pending_pairings: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, PendingPairing>>>,
    pub(super) chat_states: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, BotChatState>>>,
}

impl FeishuBot {
    pub fn new(config: FeishuConfig) -> Self {
        Self {
            config,
            token: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            pending_pairings: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            chat_states: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn restore_chat_state(&self, chat_id: &str, state: BotChatState) {
        self.chat_states.write().await.insert(chat_id.to_string(), state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::remote_connect::bot::command_router::BotLanguage;

    #[test]
    fn parse_text_message_event() {
        let event = serde_json::json!({
            "header": { "event_type": "im.message.receive_v1" },
            "event": {
                "message": {
                    "message_type": "text",
                    "chat_id": "oc_test_chat",
                    "content": "{\"text\":\"/help\"}"
                }
            }
        });

        let parsed = FeishuBot::parse_ws_event(&event);
        assert_eq!(parsed, Some(("oc_test_chat".to_string(), "/help".to_string())));
    }

    #[test]
    fn parse_card_action_event_uses_embedded_chat_id() {
        let event = serde_json::json!({
            "header": { "event_type": "card.action.trigger" },
            "event": {
                "context": {
                    "open_chat_id": "oc_fallback"
                },
                "action": {
                    "value": {
                        "chat_id": "oc_actual",
                        "command": "/switch_workspace"
                    }
                }
            }
        });

        let parsed = FeishuBot::parse_ws_event(&event);
        assert_eq!(parsed, Some(("oc_actual".to_string(), "/switch_workspace".to_string())));
    }

    #[test]
    fn card_body_removes_slash_command_list() {
        let body = FeishuBot::card_body_text(
            BotLanguage::EnUS,
            "Available commands:\n/switch_workspace - List and switch workspaces\n/help - Show this help message",
        );

        assert_eq!(body, "Available commands:\n\nChoose an action below.");
    }
}
