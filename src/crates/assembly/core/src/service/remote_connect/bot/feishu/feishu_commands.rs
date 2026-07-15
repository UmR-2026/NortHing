use super::super::{load_bot_persistence, save_bot_persistence, BotConfig};
use super::feishu_types::ParsedMessage;
use super::FeishuBot;
use crate::service::remote_connect::bot::command_router::{
    complete_im_bot_pairing, current_bot_language, execute_forwarded_turn, handle_command, parse_command,
    welcome_message, BotChatState, BotInteractionHandler, BotInteractiveRequest, BotMessageSender, HandleResult,
};

use chrono::Utc;
use serde_json::Value;
use tracing::warn;

// =====================================================================
// Event parsing
// =====================================================================

impl FeishuBot {
    /// Parse a Feishu message event into text + image keys.
    /// Supports `text`, `post` (rich text with images), and `image` message types.
    pub(super) fn parse_message_event_full(event: &Value) -> Option<ParsedMessage> {
        let event_type = event.pointer("/header/event_type").and_then(|v| v.as_str())?;
        if event_type != "im.message.receive_v1" {
            return None;
        }

        let chat_id = event
            .pointer("/event/message/chat_id")
            .and_then(|v| v.as_str())?
            .to_string();
        let message_id = event
            .pointer("/event/message/message_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let msg_type = event.pointer("/event/message/message_type").and_then(|v| v.as_str())?;
        let content_str = event.pointer("/event/message/content").and_then(|v| v.as_str())?;
        let content: Value = serde_json::from_str(content_str).ok()?;

        match msg_type {
            "text" => {
                let text = content["text"].as_str()?.trim().to_string();
                if text.is_empty() {
                    return None;
                }
                Some(ParsedMessage {
                    chat_id,
                    message_id,
                    text,
                    image_keys: vec![],
                })
            }
            "post" => {
                let (text, image_keys) = Self::extract_from_post(&content);
                if text.is_empty() && image_keys.is_empty() {
                    return None;
                }
                Some(ParsedMessage {
                    chat_id,
                    message_id,
                    text,
                    image_keys,
                })
            }
            "image" => {
                let image_key = content["image_key"].as_str()?.to_string();
                Some(ParsedMessage {
                    chat_id,
                    message_id,
                    text: String::new(),
                    image_keys: vec![image_key],
                })
            }
            _ => None,
        }
    }

    /// Backward-compatible wrapper: returns (chat_id, text) only for text/post with text content.
    #[cfg(test)]
    pub(super) fn parse_message_event(event: &Value) -> Option<(String, String)> {
        let parsed = Self::parse_message_event_full(event)?;
        if parsed.text.is_empty() {
            return None;
        }
        Some((parsed.chat_id, parsed.text))
    }

    /// Extract text and image keys from a Feishu `post` (rich-text) message.
    pub(super) fn extract_from_post(content: &Value) -> (String, Vec<String>) {
        let root = if content["content"].is_array() {
            content
        } else {
            content
                .get("zh_cn")
                .or_else(|| content.get("en_us"))
                .or_else(|| content.as_object().and_then(|obj| obj.values().next()))
                .unwrap_or(content)
        };

        let paragraphs = match root["content"].as_array() {
            Some(p) => p,
            None => return (String::new(), vec![]),
        };

        let mut text_parts: Vec<String> = Vec::new();
        let mut image_keys: Vec<String> = Vec::new();

        for para in paragraphs {
            if let Some(elements) = para.as_array() {
                for elem in elements {
                    match elem["tag"].as_str().unwrap_or("") {
                        "text" | "a" => {
                            if let Some(t) = elem["text"].as_str() {
                                let trimmed = t.trim();
                                if !trimmed.is_empty() {
                                    text_parts.push(trimmed.to_string());
                                }
                            }
                        }
                        "img" => {
                            if let Some(key) = elem["image_key"].as_str() {
                                if !key.is_empty() {
                                    image_keys.push(key.to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let title = root["title"].as_str().unwrap_or("").trim();
        if !title.is_empty() {
            text_parts.insert(0, title.to_string());
        }

        (text_parts.join(" "), image_keys)
    }

    /// Extract (chat_id, command) from a Feishu card action callback.
    pub(super) fn parse_card_action_event(event: &Value) -> Option<(String, String)> {
        let event_type = event.pointer("/header/event_type").and_then(|v| v.as_str())?;
        if event_type != "card.action.trigger" {
            return None;
        }

        let chat_id = event
            .pointer("/event/action/value/chat_id")
            .and_then(|v| v.as_str())
            .or_else(|| event.pointer("/event/context/open_chat_id").and_then(|v| v.as_str()))?
            .to_string();
        let command = event
            .pointer("/event/action/value/command")
            .and_then(|v| v.as_str())?
            .trim()
            .to_string();

        Some((chat_id, command))
    }

    /// Extract chat_id from any im.message.receive_v1 event (regardless of msg_type).
    pub(super) fn extract_message_chat_id(event: &Value) -> Option<String> {
        let event_type = event.pointer("/header/event_type").and_then(|v| v.as_str())?;
        if event_type != "im.message.receive_v1" {
            return None;
        }
        event
            .pointer("/event/message/chat_id")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    #[cfg(test)]
    pub(super) fn parse_ws_event(event: &Value) -> Option<(String, String)> {
        Self::parse_message_event(event).or_else(|| Self::parse_card_action_event(event))
    }
}

// =====================================================================
// Message dispatch and state persistence
// =====================================================================

impl FeishuBot {
    /// Handle an incoming message after parsing.
    pub(super) async fn handle_incoming_message(
        &self,
        chat_id: &str,
        text: &str,
        images: Vec<crate::service::remote_connect::remote_server::ImageAttachment>,
    ) {
        let mut states = self.chat_states.write().await;
        let state = states.entry(chat_id.to_string()).or_insert_with(|| {
            let mut s = BotChatState::new(chat_id.to_string());
            s.paired = true;
            s
        });
        let language = current_bot_language().await;

        if !state.paired {
            let trimmed = text.trim();
            if trimmed == "/start" {
                self.send_message(chat_id, welcome_message(language)).await.ok();
                return;
            }
            if trimmed.len() == 6 && trimmed.chars().all(|c| c.is_ascii_digit()) {
                if self.verify_pairing_code(trimmed).await {
                    let result = complete_im_bot_pairing(state).await;
                    self.send_handle_result(chat_id, &result).await.ok();
                    self.persist_chat_state(chat_id, state).await;
                    return;
                } else {
                    self.send_message(chat_id, Self::invalid_pairing_code_message(language))
                        .await
                        .ok();
                    return;
                }
            }
            self.send_message(chat_id, Self::enter_pairing_code_message(language))
                .await
                .ok();
            return;
        }

        let cmd = parse_command(text);
        let result = handle_command(state, cmd, images).await;

        self.persist_chat_state(chat_id, state).await;
        drop(states);

        self.send_handle_result(chat_id, &result).await.ok();

        if let Some(forward) = result.forward_to_session {
            let bot = self.clone();
            let cid = chat_id.to_string();
            tokio::spawn(async move {
                let interaction_bot = bot.clone();
                let interaction_chat_id = cid.clone();
                let handler: BotInteractionHandler = std::sync::Arc::new(move |interaction: BotInteractiveRequest| {
                    let interaction_bot = interaction_bot.clone();
                    let interaction_chat_id = interaction_chat_id.clone();
                    Box::pin(async move {
                        interaction_bot
                            .deliver_interaction(&interaction_chat_id, interaction)
                            .await;
                    })
                });
                let msg_bot = bot.clone();
                let msg_cid = cid.clone();
                let sender: BotMessageSender = std::sync::Arc::new(move |text: String| {
                    let msg_bot = msg_bot.clone();
                    let msg_cid = msg_cid.clone();
                    Box::pin(async move {
                        if let Err(err) = msg_bot.send_message(&msg_cid, &text).await {
                            warn!("Failed to send Feishu intermediate message to {msg_cid}: {err}");
                        }
                    })
                });
                let verbose_mode = load_bot_persistence().verbose_mode;
                let result = execute_forwarded_turn(forward, Some(handler), Some(sender), verbose_mode).await;
                if !result.display_text.is_empty() {
                    if let Err(err) = bot.send_message(&cid, &result.display_text).await {
                        warn!("Failed to send Feishu final message to {cid}: {err}");
                    }
                }
                bot.notify_files_ready(&cid, &result.full_text).await;
            });
        }
    }

    pub(super) async fn deliver_interaction(&self, chat_id: &str, interaction: BotInteractiveRequest) {
        let mut states = self.chat_states.write().await;
        let state = states.entry(chat_id.to_string()).or_insert_with(|| {
            let mut s = BotChatState::new(chat_id.to_string());
            s.paired = true;
            s
        });
        crate::service::remote_connect::bot::command_router::apply_interactive_request(state, &interaction);
        self.persist_chat_state(chat_id, state).await;
        drop(states);

        let result = HandleResult {
            reply: interaction.reply,
            actions: interaction.actions,
            forward_to_session: None,
            menu: interaction.menu,
        };
        self.send_handle_result(chat_id, &result).await.ok();
    }

    pub(super) async fn persist_chat_state(&self, chat_id: &str, state: &BotChatState) {
        let mut data = load_bot_persistence();
        data.upsert(crate::service::remote_connect::bot::SavedBotConnection {
            bot_type: "feishu".to_string(),
            chat_id: chat_id.to_string(),
            config: BotConfig::Feishu {
                app_id: self.config.app_id.clone(),
                app_secret: self.config.app_secret.clone(),
            },
            chat_state: state.clone(),
            connected_at: Utc::now().timestamp(),
        });
        save_bot_persistence(&data);
    }
}
