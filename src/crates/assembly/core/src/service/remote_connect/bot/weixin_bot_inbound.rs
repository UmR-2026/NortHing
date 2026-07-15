//! Weixin bot — inbound parsing + pairing + message loop.
//!
//! Owns every `impl WeixinBot` method that *receives* from the peer:
//!   * Item-list parsing (body, peer id, context token, image sentinel)
//!   * Pairing store (`register_pairing` / `verify_pairing_code`)
//!   * `send_handle_result` (command_router reply), `notify_files_ready`
//!     (auto-push), `persist_chat_state` (disk save)
//!   * `wait_for_pairing` (pre-pairing long-poll), `run_message_loop`
//!     (post-pairing long-poll), `handle_incoming_message`,
//!     `deliver_interaction`
//!
//! `PendingPairing` is imported from `weixin_bot.rs` (R38 lesson —
//! cross-sibling `pub(super)` field access).  Outbound CDN / send_* helpers
//! live in `weixin_bot_media.rs`; structure / auth / `post_ilink` live in
//! `weixin_bot.rs`.
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

use super::command_router::{
    apply_interactive_request, complete_im_bot_pairing, current_bot_language, execute_forwarded_turn, handle_command,
    parse_command, welcome_message, BotChatState, BotInteractionHandler, BotInteractiveRequest, BotMessageSender,
    HandleResult,
};
use super::weixin_bot::{
    PendingPairing, WeixinBot, CHANNEL_VERSION, LONG_POLL_TIMEOUT_SECS, MAX_INBOUND_IMAGES, SESSION_EXPIRED_ERRCODE,
};
use super::weixin_qr_login::{load_sync_buf, save_sync_buf};
use super::{load_bot_persistence, save_bot_persistence, BotConfig, SavedBotConnection};
use crate::service::remote_connect::remote_server::ImageAttachment;

impl WeixinBot {
    fn is_weixin_media_item_type(type_id: i64) -> bool {
        matches!(type_id, 2..=5)
    }

    fn body_from_item_list(items: &[Value]) -> String {
        for item in items {
            let t = item["type"].as_i64().unwrap_or(0);
            if t == 1 {
                if let Some(tx) = item["text_item"]["text"].as_str() {
                    let text = tx.to_string();
                    let ref_msg = &item["ref_msg"];
                    if !ref_msg.is_object() {
                        return text;
                    }
                    let ref_title = ref_msg["title"].as_str();
                    let ref_item = &ref_msg["message_item"];
                    if ref_item.is_object() {
                        let mt = ref_item["type"].as_i64().unwrap_or(0);
                        if Self::is_weixin_media_item_type(mt) {
                            return text;
                        }
                        let ref_body = Self::body_from_item_list(std::slice::from_ref(ref_item));
                        if ref_title.is_none() && ref_body.is_empty() {
                            return text;
                        }
                        let mut parts: Vec<String> = Vec::new();
                        if let Some(tt) = ref_title {
                            parts.push(tt.to_string());
                        }
                        if !ref_body.is_empty() {
                            parts.push(ref_body);
                        }
                        if parts.is_empty() {
                            return text;
                        }
                        let joined = parts.join(" | ");
                        return format!("[引用: {joined}]\n{text}");
                    }
                    if let Some(tt) = ref_title {
                        return format!("[引用: {tt}]\n{text}");
                    }
                    return text;
                }
            }
            if t == 3 {
                if let Some(tx) = item["voice_item"]["text"].as_str() {
                    return tx.to_string();
                }
            }
        }
        String::new()
    }

    fn body_from_message(msg: &Value) -> String {
        let Some(items) = msg["item_list"].as_array() else {
            return String::new();
        };
        Self::body_from_item_list(items)
    }

    /// True if the message carries at least one `image_item` (pairing wait UX / guards).
    fn has_inbound_image_items(msg: &Value) -> bool {
        let Some(items) = msg["item_list"].as_array() else {
            return false;
        };
        items.iter().any(|i| {
            i["type"].as_i64() == Some(2)
                && i["image_item"]["media"]["encrypt_query_param"]
                    .as_str()
                    .is_some_and(|s| !s.is_empty())
        })
    }

    fn is_user_message(msg: &Value) -> bool {
        msg["message_type"].as_i64() == Some(1)
    }

    fn peer_id(msg: &Value) -> Option<String> {
        msg["from_user_id"]
            .as_str()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
    }

    fn context_token(msg: &Value) -> Option<String> {
        msg["context_token"]
            .as_str()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
    }

    pub async fn register_pairing(&self, pairing_code: &str) -> Result<()> {
        self.pending_pairings.write().await.insert(
            pairing_code.to_string(),
            PendingPairing {
                created_at: chrono::Utc::now().timestamp(),
            },
        );
        Ok(())
    }

    pub async fn verify_pairing_code(&self, code: &str) -> bool {
        let mut pairings = self.pending_pairings.write().await;
        if let Some(p) = pairings.remove(code) {
            let age = chrono::Utc::now().timestamp() - p.created_at;
            return age < 300;
        }
        false
    }

    async fn send_handle_result(&self, peer_id: &str, result: &HandleResult) {
        let language = current_bot_language().await;
        let text = if result.menu.items.is_empty() && result.menu.title.is_empty() {
            result.reply.clone()
        } else {
            result.menu.render_plain_text(language)
        };
        if text.trim().is_empty() {
            return;
        }
        if let Err(e) = self.send_text(peer_id, &text).await {
            warn!("weixin send_handle_result: {e}");
        }
    }

    /// Scan `text` for downloadable file references and push each matching
    /// file directly to the peer via the iLink CDN pipeline.  Files exceeding
    /// `MAX_WEIXIN_FILE_BYTES` are skipped with a brief notice; per-file
    /// failures are reported as plain-text replies.
    async fn notify_files_ready(&self, peer_id: &str, text: &str) {
        use super::weixin_crypto::MAX_WEIXIN_FILE_BYTES;
        let language = current_bot_language().await;
        let workspace_root = {
            let states = self.chat_states.read().await;
            states.get(peer_id).and_then(|s| s.active_workspace_path())
        };
        let files = super::collect_auto_push_files(text, workspace_root.as_deref().map(std::path::Path::new));
        if files.is_empty() {
            return;
        }

        // Intentionally do NOT send a "正在为你发送 N 个文件……" intro: the
        // file message itself already shows up in the chat, and the intro
        // line just adds noise (and on WeChat costs a context_token slot
        // per send). Errors / size-skips below still surface as their own
        // notice messages so the user is informed when something is wrong.
        let root_path = workspace_root.as_deref().map(std::path::Path::new);
        for file in files {
            if file.size > MAX_WEIXIN_FILE_BYTES {
                let notice =
                    super::auto_push_skip_too_large_message(language, &file.name, file.size, MAX_WEIXIN_FILE_BYTES);
                if let Err(e) = self.send_text(peer_id, &notice).await {
                    warn!("Weixin auto-push skip notice failed for peer {peer_id}: {e}");
                }
                continue;
            }
            match self
                .send_workspace_file_to_peer(peer_id, &file.abs_path, root_path)
                .await
            {
                Ok(()) => info!("Weixin auto-pushed file to peer {peer_id}: {}", file.abs_path),
                Err(e) => {
                    warn!("Weixin auto-push failed for {} to peer {peer_id}: {e}", file.name);
                    let notice = super::auto_push_failed_message(language, &file.name, &e.to_string());
                    if let Err(send_err) = self.send_text(peer_id, &notice).await {
                        warn!("Weixin auto-push failure notice failed for peer {peer_id}: {send_err}");
                    }
                }
            }
        }
    }

    async fn persist_chat_state(&self, peer_id: &str, state: &BotChatState) {
        let mut data = load_bot_persistence();
        data.upsert(SavedBotConnection {
            bot_type: "weixin".to_string(),
            chat_id: peer_id.to_string(),
            config: BotConfig::Weixin {
                ilink_token: self.config.ilink_token.clone(),
                base_url: self.config.base_url.clone(),
                bot_account_id: self.config.bot_account_id.clone(),
            },
            chat_state: state.clone(),
            connected_at: chrono::Utc::now().timestamp(),
        });
        save_bot_persistence(&data);
    }

    /// Pairing + message loop: long-poll getupdates.
    pub async fn wait_for_pairing(&self, stop_rx: &mut tokio::sync::watch::Receiver<bool>) -> Result<String> {
        info!("Weixin bot waiting for pairing code (getupdates)...");
        let mut buf = load_sync_buf(&self.config.bot_account_id);

        loop {
            if *stop_rx.borrow() {
                return Err(anyhow!("bot stop requested"));
            }

            let poll = tokio::select! {
                _ = stop_rx.changed() => {
                    return Err(anyhow!("bot stop requested"));
                }
                r = self.get_updates_once(
                    &buf,
                    Duration::from_secs(LONG_POLL_TIMEOUT_SECS),
                ) => r,
            };

            let resp = match poll {
                Ok(v) => v,
                Err(e) => {
                    error!("weixin getupdates: {e}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let ret = resp["ret"].as_i64().unwrap_or(0);
            let errcode = resp["errcode"].as_i64().unwrap_or(0);
            if (ret != 0 && ret != SESSION_EXPIRED_ERRCODE) || (errcode != 0 && errcode != SESSION_EXPIRED_ERRCODE) {
                if errcode == SESSION_EXPIRED_ERRCODE || ret == SESSION_EXPIRED_ERRCODE {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
                warn!("weixin getupdates ret={ret} errcode={errcode}");
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }

            if let Some(new_buf) = resp["get_updates_buf"].as_str() {
                buf = new_buf.to_string();
                save_sync_buf(&self.config.bot_account_id, &buf);
            }

            if let Some(msgs) = resp["msgs"].as_array() {
                for msg in msgs {
                    if !Self::is_user_message(msg) {
                        continue;
                    }
                    let Some(peer) = Self::peer_id(msg) else {
                        continue;
                    };
                    if let Some(ct) = Self::context_token(msg) {
                        self.context_tokens.write().await.insert(peer.clone(), ct);
                    }
                    let text = Self::body_from_message(msg).trim().to_string();
                    let language = current_bot_language().await;

                    if text == "/start" {
                        self.try_send_text(&peer, welcome_message(language), "welcome").await;
                        continue;
                    }

                    if text.len() == 6 && text.chars().all(|c| c.is_ascii_digit()) {
                        if self.verify_pairing_code(&text).await {
                            info!("Weixin pairing successful peer={peer}");
                            let mut state = BotChatState::new(peer.clone());
                            let result = complete_im_bot_pairing(&mut state).await;
                            self.chat_states.write().await.insert(peer.clone(), state.clone());
                            self.persist_chat_state(&peer, &state).await;

                            self.send_handle_result(&peer, &result).await;
                            return Ok(peer);
                        } else {
                            let err = if language.is_chinese() {
                                "配对码无效或已过期，请重试。"
                            } else {
                                "Invalid or expired pairing code."
                            };
                            self.try_send_text(&peer, err, "pairing-invalid").await;
                        }
                    } else if !text.is_empty() {
                        let err = if language.is_chinese() {
                            "请输入 northhing 桌面端远程连接中显示的 6 位配对码。"
                        } else {
                            "Please send the 6-digit pairing code from northhing Desktop Remote Connect."
                        };
                        self.try_send_text(&peer, err, "pairing-prompt").await;
                    } else if Self::has_inbound_image_items(msg) {
                        let err = if language.is_chinese() {
                            "配对请直接发送 6 位数字配对码；完成配对后再发送图片与助手对话。"
                        } else {
                            "To pair, send the 6-digit code only. After pairing you can send images to chat."
                        };
                        self.try_send_text(&peer, err, "pairing-image-hint").await;
                    }
                }
            }
        }
    }

    pub async fn run_message_loop(self: Arc<Self>, stop_rx: tokio::sync::watch::Receiver<bool>) {
        info!("Weixin message loop started");
        let mut stop = stop_rx;
        let mut buf = load_sync_buf(&self.config.bot_account_id);

        loop {
            if *stop.borrow() {
                break;
            }

            let poll = tokio::select! {
                _ = stop.changed() => break,
                r = self.get_updates_once(
                    &buf,
                    Duration::from_secs(LONG_POLL_TIMEOUT_SECS),
                ) => r,
            };

            let resp = match poll {
                Ok(v) => v,
                Err(e) => {
                    error!("weixin getupdates (loop): {e}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let ret = resp["ret"].as_i64().unwrap_or(0);
            let errcode = resp["errcode"].as_i64().unwrap_or(0);
            if (ret != 0 && ret != SESSION_EXPIRED_ERRCODE) || (errcode != 0 && errcode != SESSION_EXPIRED_ERRCODE) {
                if errcode == SESSION_EXPIRED_ERRCODE || ret == SESSION_EXPIRED_ERRCODE {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }

            if let Some(new_buf) = resp["get_updates_buf"].as_str() {
                buf = new_buf.to_string();
                save_sync_buf(&self.config.bot_account_id, &buf);
            }

            let Some(msgs) = resp["msgs"].as_array() else {
                continue;
            };

            for msg in msgs {
                if !Self::is_user_message(msg) {
                    continue;
                }
                let Some(peer) = Self::peer_id(msg) else {
                    continue;
                };
                if let Some(ct) = Self::context_token(msg) {
                    self.context_tokens.write().await.insert(peer.clone(), ct);
                }
                let msg_value = msg.clone();
                let bot = self.clone();
                tokio::spawn(async move {
                    let (images, skipped_images) = bot.inbound_image_attachments_from_message(&msg_value).await;
                    let language = current_bot_language().await;
                    // Match Feishu: truncation is a separate user-visible message, not mixed into command text.
                    if skipped_images > 0 {
                        let note = if language.is_chinese() {
                            format!(
                                "仅会处理前 {} 张图片，其余 {} 张已丢弃。",
                                MAX_INBOUND_IMAGES, skipped_images
                            )
                        } else {
                            format!(
                                "Only the first {} images will be processed; the remaining {} were discarded.",
                                MAX_INBOUND_IMAGES, skipped_images
                            )
                        };
                        bot.try_send_text(&peer, &note, "image-truncation-notice").await;
                    }
                    let body = WeixinBot::body_from_message(&msg_value);
                    let text = if body.trim().is_empty() && !images.is_empty() {
                        if language.is_chinese() {
                            "[用户发送了一张图片]".to_string()
                        } else {
                            "[User sent an image]".to_string()
                        }
                    } else {
                        body
                    };
                    bot.handle_incoming_message(peer, &text, images).await;
                });
            }
        }
        info!("Weixin message loop stopped");
    }

    async fn handle_incoming_message(self: &Arc<Self>, peer_id: String, text: &str, images: Vec<ImageAttachment>) {
        let mut states = self.chat_states.write().await;
        let state = states.entry(peer_id.clone()).or_insert_with(|| {
            let mut s = BotChatState::new(peer_id.clone());
            s.paired = true;
            s
        });
        let language = current_bot_language().await;

        if !state.paired {
            let trimmed = text.trim();
            if trimmed == "/start" {
                drop(states);
                self.try_send_text(&peer_id, welcome_message(language), "welcome").await;
                return;
            }
            if trimmed.len() == 6 && trimmed.chars().all(|c| c.is_ascii_digit()) {
                if self.verify_pairing_code(trimmed).await {
                    let result = complete_im_bot_pairing(state).await;
                    self.persist_chat_state(&peer_id, state).await;
                    drop(states);
                    self.send_handle_result(&peer_id, &result).await;
                    return;
                } else {
                    let err = if language.is_chinese() {
                        "配对码无效或已过期。"
                    } else {
                        "Invalid or expired pairing code."
                    };
                    drop(states);
                    self.try_send_text(&peer_id, err, "pairing-invalid").await;
                    return;
                }
            }
            drop(states);
            let err = if language.is_chinese() {
                "请输入 6 位配对码。"
            } else {
                "Please send the 6-digit pairing code."
            };
            self.try_send_text(&peer_id, err, "pairing-prompt").await;
            return;
        }

        let cmd = parse_command(text);
        let result = handle_command(state, cmd, images).await;
        self.persist_chat_state(&peer_id, state).await;
        drop(states);

        self.send_handle_result(&peer_id, &result).await;

        if let Some(forward) = result.forward_to_session {
            let bot = self.clone();
            let peer = peer_id.clone();
            // Only show "正在输入" when there's an actual agentic turn to run.
            // Local command/menu replies are already sent synchronously above,
            // so a typing indicator there would either flash for a few ms or,
            // worse, linger if the cancel call is delayed — both look broken
            // to the user.  Agentic turns are the long-running case where
            // typing genuinely tells the user "the bot is still working".
            let typing_for_turn = self.start_typing(peer_id.clone());
            tokio::spawn(async move {
                let interaction_bot = bot.clone();
                let peer_c = peer.clone();
                let handler: BotInteractionHandler = Arc::new(move |interaction: BotInteractiveRequest| {
                    let interaction_bot = interaction_bot.clone();
                    let peer_i = peer_c.clone();
                    Box::pin(async move {
                        interaction_bot.deliver_interaction(peer_i, interaction).await;
                    })
                });
                let msg_bot = bot.clone();
                let peer_m = peer.clone();
                let sender: BotMessageSender = Arc::new(move |t: String| {
                    let msg_bot = msg_bot.clone();
                    let peer_s = peer_m.clone();
                    Box::pin(async move {
                        if let Err(e) = msg_bot.send_text(&peer_s, &t).await {
                            warn!("weixin: send intermediate message to peer {peer_s} failed: {e}");
                        }
                    })
                });
                let verbose_mode = load_bot_persistence().verbose_mode;
                let turn_result = execute_forwarded_turn(forward, Some(handler), Some(sender), verbose_mode).await;
                if !turn_result.display_text.is_empty() {
                    if let Err(e) = bot.send_text(&peer, &turn_result.display_text).await {
                        warn!("weixin: send final reply to peer {peer} failed: {e}");
                    }
                }
                bot.notify_files_ready(&peer, &turn_result.full_text).await;
                // Stop typing AFTER both the final reply and any auto-pushed
                // files have been dispatched, so the indicator does not flap
                // off between the text answer and its attachments.
                typing_for_turn.stop().await;
            });
        }
    }

    async fn deliver_interaction(&self, peer_id: String, interaction: BotInteractiveRequest) {
        let mut states = self.chat_states.write().await;
        let state = states.entry(peer_id.clone()).or_insert_with(|| {
            let mut s = BotChatState::new(peer_id.clone());
            s.paired = true;
            s
        });
        apply_interactive_request(state, &interaction);
        self.persist_chat_state(&peer_id, state).await;
        drop(states);

        let result = HandleResult {
            reply: interaction.reply,
            actions: interaction.actions,
            forward_to_session: None,
            menu: interaction.menu,
        };
        self.send_handle_result(&peer_id, &result).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn body_from_message_plain_text() {
        let msg = json!({
            "item_list": [{ "type": 1, "text_item": { "text": "hi" } }]
        });
        assert_eq!(WeixinBot::body_from_message(&msg), "hi");
    }

    #[test]
    fn body_from_message_quoted_text() {
        let msg = json!({
            "item_list": [{
                "type": 1,
                "text_item": { "text": "reply" },
                "ref_msg": { "title": " earlier ", "message_item": { "type": 1, "text_item": { "text": "orig" } } }
            }]
        });
        let b = WeixinBot::body_from_message(&msg);
        assert!(b.contains("[引用:"));
        assert!(b.contains("reply"));
    }
}
