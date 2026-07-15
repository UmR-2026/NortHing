use super::feishu_types::{pb, PendingPairing};
use super::FeishuBot;
use crate::service::remote_connect::bot::command_router::{
    complete_im_bot_pairing, current_bot_language, welcome_message, BotChatState, HandleResult,
};
use crate::service::remote_connect::bot::feishu::feishu_types::SharedFeishuWsWrite;
use crate::util::truncate_at_char_boundary;

use anyhow::{anyhow, Result};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{watch::Receiver, RwLock};
use tokio::time::Duration;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, error, info, warn};

// =====================================================================
// WebSocket endpoint and pairing
// =====================================================================

impl FeishuBot {
    /// Obtain a WebSocket URL from Feishu for long-connection event delivery.
    /// Uses direct AppID/AppSecret auth per Feishu SDK protocol (no bearer token).
    pub(super) async fn get_ws_endpoint(&self) -> Result<(String, serde_json::Value)> {
        let client = reqwest::Client::new();
        let resp = client
            .post("https://open.feishu.cn/callback/ws/endpoint")
            .json(&serde_json::json!({
                "AppID": self.config.app_id,
                "AppSecret": self.config.app_secret,
            }))
            .send()
            .await
            .map_err(|e| anyhow!("feishu ws endpoint request: {e}"))?;

        let ws_resp_text = resp.text().await.unwrap_or_default();
        let body: serde_json::Value = serde_json::from_str(&ws_resp_text).map_err(|e| {
            anyhow!(
                "feishu ws endpoint parse error: {e}, body: {}",
                truncate_at_char_boundary(&ws_resp_text, 300)
            )
        })?;
        let code = body["code"].as_i64().unwrap_or(-1);
        if code != 0 {
            let msg = body["msg"].as_str().unwrap_or("unknown error");
            return Err(anyhow!("feishu ws endpoint error {code}: {msg}"));
        }

        let url = body
            .pointer("/data/URL")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("missing WebSocket URL in feishu response"))?
            .to_string();
        let client_config = body.pointer("/data/ClientConfig").cloned().unwrap_or_default();

        Ok((url, client_config))
    }

    /// Start polling for pairing codes.  Returns the chat_id on success.
    pub async fn wait_for_pairing(&self, stop_rx: &mut Receiver<bool>) -> Result<String> {
        info!("Feishu bot waiting for pairing code via WebSocket...");

        if *stop_rx.borrow() {
            return Err(anyhow!("bot stop requested"));
        }

        let (ws_url, config) = self.get_ws_endpoint().await?;

        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .map_err(|e| anyhow!("feishu ws connect: {e}"))?;

        let (write, mut read) = ws_stream.split();
        let write: SharedFeishuWsWrite = Arc::new(RwLock::new(write));
        info!("Feishu WebSocket connected (binary proto), waiting for pairing...");

        let service_id = Self::extract_service_id_from_url(&ws_url);

        let ping_interval = config.get("PingInterval").and_then(|v| v.as_u64()).unwrap_or(120);

        let mut ping_timer = tokio::time::interval(Duration::from_secs(ping_interval));

        loop {
            tokio::select! {
                _ = stop_rx.changed() => {
                    info!("Feishu wait_for_pairing stopped by signal");
                    return Err(anyhow!("bot stop requested"));
                }
                msg = read.next() => {
                    match msg {
                        Some(Ok(WsMessage::Binary(data))) => {
                            let frame = match pb::decode_frame(&data) {
                                Some(f) => f,
                                None => continue,
                            };
                            match frame.method {
                                pb::FRAME_TYPE_DATA => {
                                    if let Some(chat_id) = self.handle_data_frame_for_pairing(&frame, &write).await {
                                        return Ok(chat_id);
                                    }
                                }
                                pb::FRAME_TYPE_CONTROL => {
                                    debug!("Feishu WS control frame: type={}", frame.get_header("type").unwrap_or("?"));
                                }
                                _ => {}
                            }
                        }
                        Some(Ok(WsMessage::Ping(data))) => {
                            let _ = write.write().await.send(WsMessage::Pong(data)).await;
                        }
                        Some(Err(e)) => {
                            error!("Feishu WebSocket error during pairing: {e}");
                            return Err(anyhow!("feishu ws error: {e}"));
                        }
                        None => {
                            return Err(anyhow!("feishu ws connection closed during pairing"));
                        }
                        _ => {}
                    }
                }
                _ = ping_timer.tick() => {
                    let ping = pb::Frame::new_ping(service_id);
                    let _ = write
                        .write()
                        .await
                        .send(WsMessage::Binary(pb::encode_frame(&ping).into()))
                        .await;
                }
            }
        }
    }

    fn extract_service_id_from_url(url: &str) -> i32 {
        url.split('?')
            .nth(1)
            .and_then(|qs| {
                qs.split('&').find_map(|pair| {
                    let mut kv = pair.splitn(2, '=');
                    match (kv.next(), kv.next()) {
                        (Some("service_id"), Some(v)) => v.parse::<i32>().ok(),
                        _ => None,
                    }
                })
            })
            .unwrap_or(0)
    }

    /// Main message loop that runs after pairing is complete.
    /// Connects to Feishu WebSocket (binary protobuf protocol) and routes
    /// incoming messages through the command router.
    pub async fn run_message_loop(self: Arc<Self>, stop_rx: Receiver<bool>) {
        info!("Feishu bot message loop started");
        let mut stop = stop_rx;

        loop {
            if *stop.borrow() {
                info!("Feishu bot message loop stopped by signal");
                break;
            }

            let ws_result = self.get_ws_endpoint().await;
            let (ws_url, config) = match ws_result {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to get Feishu WS endpoint: {e}");
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    continue;
                }
            };

            let ping_interval = config.get("PingInterval").and_then(|v| v.as_u64()).unwrap_or(120);

            let service_id = Self::extract_service_id_from_url(&ws_url);

            let ws_conn = tokio_tungstenite::connect_async(&ws_url).await;
            let (ws_stream, _) = match ws_conn {
                Ok(v) => v,
                Err(e) => {
                    error!("Feishu WS connect failed: {e}");
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    continue;
                }
            };
            info!("Feishu WebSocket connected for message loop (binary proto)");

            let (write, mut read) = ws_stream.split();
            let write: SharedFeishuWsWrite = Arc::new(RwLock::new(write));

            let mut ping_timer = tokio::time::interval(Duration::from_secs(ping_interval));

            loop {
                tokio::select! {
                    _ = stop.changed() => {
                        info!("Feishu bot message loop stopped by signal");
                        return;
                    }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(WsMessage::Binary(data))) => {
                                let frame = match pb::decode_frame(&data) {
                                    Some(f) => f,
                                    None => continue,
                                };

                                match frame.method {
                                    pb::FRAME_TYPE_DATA => {
                                        let msg_type = frame.get_header("type").unwrap_or("");
                                        if msg_type == "event" {
                                            if let Ok(event) = serde_json::from_slice::<serde_json::Value>(&frame.payload) {
                                                // Send ack
                                                let resp = pb::Frame::new_response(&frame, 200);
                                                let _ = write
                                                    .write()
                                                    .await
                                                    .send(WsMessage::Binary(pb::encode_frame(&resp).into()))
                                                    .await;

                                                if let Some(parsed) = Self::parse_message_event_full(&event) {
                                                    let bot = self.clone();
                                                    tokio::spawn(async move {
                                                        const MAX_IMAGES: usize = 5;
                                                        let language = current_bot_language().await;
                                                        let truncated = parsed.image_keys.len() > MAX_IMAGES;
                                                        let keys_to_use = if truncated {
                                                            &parsed.image_keys[..MAX_IMAGES]
                                                        } else {
                                                            &parsed.image_keys
                                                        };
                                                        let images = if keys_to_use.is_empty() {
                                                            vec![]
                                                        } else {
                                                            bot.download_images(&parsed.message_id, keys_to_use).await
                                                        };
                                                        if truncated {
                                                            let msg = format!(
                                                                "{} {} {}",
                                                                if language.is_chinese() {
                                                                    "仅会处理前"
                                                                } else {
                                                                    "Only the first"
                                                                },
                                                                MAX_IMAGES,
                                                                if language.is_chinese() {
                                                                    format!(
                                                                        "张图片，其余 {} 张已丢弃。",
                                                                        parsed.image_keys.len() - MAX_IMAGES
                                                                    )
                                                                } else {
                                                                    format!(
                                                                        "images will be processed; the remaining {} were discarded.",
                                                                        parsed.image_keys.len() - MAX_IMAGES
                                                                    )
                                                                },
                                                            );
                                                            let _ = bot.send_message(&parsed.chat_id, &msg).await;
                                                        }
                                                        let text = if parsed.text.is_empty() && !images.is_empty() {
                                                            if language.is_chinese() {
                                                                "[用户发送了一张图片]".to_string()
                                                            } else {
                                                                "[User sent an image]".to_string()
                                                            }
                                                        } else {
                                                            parsed.text
                                                        };
                                                        bot.handle_incoming_message(
                                                            &parsed.chat_id,
                                                            &text,
                                                            images,
                                                        )
                                                        .await;
                                                    });
                                                } else if let Some((chat_id, cmd)) = Self::parse_card_action_event(&event) {
                                                    let bot = self.clone();
                                                    tokio::spawn(async move {
                                                        bot.handle_incoming_message(
                                                            &chat_id,
                                                            &cmd,
                                                            vec![],
                                                        )
                                                        .await;
                                                    });
                                                } else if let Some(chat_id) = Self::extract_message_chat_id(&event) {
                                                    let bot = self.clone();
                                                    tokio::spawn(async move {
                                                        let language = current_bot_language().await;
                                                        bot.send_message(
                                                            &chat_id,
                                                            Self::unsupported_message_type_message(language),
                                                        ).await.ok();
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    pb::FRAME_TYPE_CONTROL => {
                                        debug!("Feishu WS control: type={}", frame.get_header("type").unwrap_or("?"));
                                    }
                                    _ => {}
                                }
                            }
                            Some(Ok(WsMessage::Ping(data))) => {
                                let _ = write.write().await.send(WsMessage::Pong(data)).await;
                            }
                            Some(Err(e)) => {
                                error!("Feishu WS error: {e}");
                                break;
                            }
                            None => {
                                warn!("Feishu WS closed, reconnecting...");
                                break;
                            }
                            _ => {}
                        }
                    }
                    _ = ping_timer.tick() => {
                        let ping = pb::Frame::new_ping(service_id);
                        let _ = write
                            .write()
                            .await
                            .send(WsMessage::Binary(pb::encode_frame(&ping).into()))
                            .await;
                    }
                }
            }

            let reconnect_interval = config.get("ReconnectInterval").and_then(|v| v.as_u64()).unwrap_or(3);
            tokio::time::sleep(Duration::from_secs(reconnect_interval)).await;
        }
    }

    /// Handle a single incoming protobuf data frame.
    /// Returns Some(chat_id) if pairing succeeded, None to continue waiting.
    pub(super) async fn handle_data_frame_for_pairing(
        &self,
        frame: &pb::Frame,
        write: &SharedFeishuWsWrite,
    ) -> Option<String> {
        let msg_type = frame.get_header("type").unwrap_or("");
        if msg_type != "event" {
            return None;
        }

        let event: serde_json::Value = serde_json::from_slice(&frame.payload).ok()?;

        // Send ack response for this frame
        let resp_frame = pb::Frame::new_response(frame, 200);
        let _ = write
            .write()
            .await
            .send(WsMessage::Binary(pb::encode_frame(&resp_frame).into()))
            .await;

        if let Some(parsed) = Self::parse_message_event_full(&event) {
            let language = current_bot_language().await;
            let chat_id = parsed.chat_id;
            let msg_text = parsed.text;
            let trimmed = msg_text.trim();

            if trimmed == "/start" {
                self.send_message(&chat_id, welcome_message(language)).await.ok();
            } else if trimmed.len() == 6 && trimmed.chars().all(|c| c.is_ascii_digit()) {
                if self.verify_pairing_code(trimmed).await {
                    info!("Feishu pairing successful, chat_id={chat_id}");
                    let mut state = BotChatState::new(chat_id.clone());
                    let result = complete_im_bot_pairing(&mut state).await;
                    self.send_handle_result(&chat_id, &result).await.ok();
                    self.chat_states.write().await.insert(chat_id.clone(), state.clone());
                    self.persist_chat_state(&chat_id, &state).await;

                    return Some(chat_id);
                } else {
                    self.send_message(&chat_id, Self::invalid_pairing_code_message(language))
                        .await
                        .ok();
                }
            } else {
                self.send_message(&chat_id, Self::enter_pairing_code_message(language))
                    .await
                    .ok();
            }
        } else if let Some(chat_id) = Self::extract_message_chat_id(&event) {
            let language = current_bot_language().await;
            self.send_message(&chat_id, Self::enter_pairing_code_message(language))
                .await
                .ok();
        }
        None
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
}
