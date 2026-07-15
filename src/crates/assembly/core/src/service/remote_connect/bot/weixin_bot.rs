//! Weixin iLink bot core: struct definitions, configuration, and the small
//! set of methods that all other siblings depend on (`new`,
//! `restore_chat_state`, `base_url`, `is_session_paused`, `pause_session`,
//! `build_auth_headers`, `post_ilink`).  All sibling files implement
//! additional `impl WeixinBot { ... }` blocks (`weixin_bot_media.rs`,
////! `weixin_bot_inbound.rs`) — Rust merges them at link time.
//!
//! `TypingHandle` (RAII guard for the WeChat "正在输入" indicator) lives
//! here because both `start_typing` (in `weixin_bot_media.rs`) and
//! `deliver_interaction` (in `weixin_bot_inbound.rs`) need direct access to
//! the same struct layout.  `PendingPairing` is `pub(super)` because
//! `register_pairing` / `verify_pairing_code` use it from
//! `weixin_bot_inbound.rs`.
use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::warn;

use super::command_router::BotChatState;
use super::weixin_qr_login::{ensure_trailing_slash, now_ms, random_wechat_uin_header};

// ── Cross-sibling tunables ────────────────────────────────────────────────────

/// Default HTTP timeout for short iLink calls; used everywhere in `weixin_*`.
pub(super) const API_TIMEOUT_SECS: u64 = 20;
/// Channel version stamped into every outbound `base_info.channel_version`
/// payload (matches OpenClaw).  Referenced from `weixin_bot_media.rs`
/// (`send_message_*`, `ilink_get_upload_url`, `get_updates_once`,
/// `fetch_typing_ticket`) and `weixin_bot_inbound.rs` (`run_message_loop`).
pub(super) const CHANNEL_VERSION: &str = "1.0.2";
/// Long-poll timeout for `ilink/bot/getupdates` (server keeps the call open
/// up to this long).  Used by `wait_for_pairing` / `run_message_loop`.
pub(super) const LONG_POLL_TIMEOUT_SECS: u64 = 36;
/// iLink application error code for "session expired — re-login required";
/// triggers a session pause window in `is_session_paused`.
pub(super) const SESSION_EXPIRED_ERRCODE: i64 = -14;
/// How long the bot refuses all iLink calls after seeing `SESSION_EXPIRED_ERRCODE`.
pub(super) const SESSION_PAUSE_SECS: u64 = 3600;
/// WeChat `sendmessage` rejects strings over this length; `chunk_text_for_weixin`
/// splits outbound replies at this boundary.
pub(super) const MAX_TEXT_CHUNK: usize = 3500;
/// Hard cap on inbound images per user-message (matches Feishu — referenced by
/// `weixin_bot_media.rs::inbound_image_attachments_from_message` and the
/// truncation notice in `weixin_bot_inbound.rs::run_message_loop`).
pub(super) const MAX_INBOUND_IMAGES: usize = 5;

// ── Public configuration + struct ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeixinConfig {
    pub ilink_token: String,
    pub base_url: String,
    /// Normalized ilink bot id (filesystem-safe); used for sync buffer path.
    pub bot_account_id: String,
}

#[derive(Debug, Clone)]
pub(super) struct PendingPairing {
    pub(super) created_at: i64,
}

pub struct WeixinBot {
    pub(super) config: WeixinConfig,
    pub(super) pending_pairings: Arc<RwLock<HashMap<String, PendingPairing>>>,
    pub(super) chat_states: Arc<RwLock<HashMap<String, BotChatState>>>,
    pub(super) context_tokens: Arc<RwLock<HashMap<String, String>>>,
    /// Per-peer typing ticket cache (returned by `ilink/bot/getconfig`,
    /// required by `ilink/bot/sendtyping`).  Refreshed lazily and dropped
    /// whenever a typing API call signals an invalid/expired ticket.
    pub(super) typing_tickets: Arc<RwLock<HashMap<String, String>>>,
    pub(super) session_pause_until_ms: Arc<RwLock<HashMap<String, i64>>>,
}

/// RAII guard returned by [`WeixinBot::start_typing`].  Dropping or calling
/// [`TypingHandle::stop`] cancels the periodic refresher and best-effort
/// emits a `sendtyping(status=2)` to clear the "正在输入" UI on the peer side.
///
/// Cancellation uses an [`AtomicBool`] (not `tokio::sync::Notify`) on purpose:
/// `Notify::notify_waiters` only wakes tasks that are *currently* parked on
/// `.notified()`, so signalling while the loop is mid-`send_typing` HTTP call
/// silently drops the wake-up and the task would refresh "正在输入" forever.
/// An atomic flag plus short-grained polling makes the cancel deterministic.
pub struct TypingHandle {
    pub(super) cancel: Arc<std::sync::atomic::AtomicBool>,
    pub(super) handle: Option<tokio::task::JoinHandle<()>>,
    pub(super) bot: Arc<WeixinBot>,
    pub(super) peer_id: String,
    pub(super) stopped: bool,
}

impl TypingHandle {
    /// Stop the typing loop and explicitly send a cancel event. Awaiting this
    /// gives callers visibility into the cancel attempt; not awaiting (i.e.
    /// just dropping) still cancels the loop and fires a best-effort cancel
    /// from the Drop impl.
    pub async fn stop(mut self) {
        self.stopped = true;
        self.cancel.store(true, std::sync::atomic::Ordering::Release);
        if let Some(h) = self.handle.take() {
            let _ = h.await;
        }
        if let Err(e) = self.bot.send_typing(&self.peer_id, 2).await {
            warn!(
                "weixin: send typing(cancel) failed for peer {peer}: {e}",
                peer = self.peer_id
            );
        }
    }
}

impl Drop for TypingHandle {
    fn drop(&mut self) {
        if self.stopped {
            return;
        }
        self.cancel.store(true, std::sync::atomic::Ordering::Release);
        if let Some(h) = self.handle.take() {
            h.abort();
        }
        // Fire-and-forget cancel: we can't await in Drop, but we still want
        // the peer's "正在输入" indicator to clear in case the future was
        // dropped without `stop().await`.
        let bot = self.bot.clone();
        let peer = self.peer_id.clone();
        tokio::spawn(async move {
            if let Err(e) = bot.send_typing(&peer, 2).await {
                warn!("weixin: drop-cancel typing failed for peer {peer}: {e}");
            }
        });
    }
}

impl WeixinBot {
    pub fn new(config: WeixinConfig) -> Self {
        Self {
            config,
            pending_pairings: Arc::new(RwLock::new(HashMap::new())),
            chat_states: Arc::new(RwLock::new(HashMap::new())),
            context_tokens: Arc::new(RwLock::new(HashMap::new())),
            typing_tickets: Arc::new(RwLock::new(HashMap::new())),
            session_pause_until_ms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn restore_chat_state(&self, peer_id: &str, state: BotChatState) {
        self.chat_states.write().await.insert(peer_id.to_string(), state);
    }

    pub(super) fn base_url(&self) -> String {
        ensure_trailing_slash(&self.config.base_url)
    }

    pub(super) async fn is_session_paused(&self) -> bool {
        let id = &self.config.bot_account_id;
        let mut m = self.session_pause_until_ms.write().await;
        let now = now_ms();
        if let Some(until) = m.get(id).copied() {
            if now >= until {
                m.remove(id);
                return false;
            }
            return true;
        }
        false
    }

    pub(super) async fn pause_session(&self) {
        let until = now_ms() + (SESSION_PAUSE_SECS as i64) * 1000;
        self.session_pause_until_ms
            .write()
            .await
            .insert(self.config.bot_account_id.clone(), until);
        warn!(
            "weixin: session expired (err -14), pausing API for {}s",
            SESSION_PAUSE_SECS
        );
    }

    pub(super) fn build_auth_headers(&self, body: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        );
        h.insert(
            HeaderName::from_static("authorizationtype"),
            HeaderValue::from_static("ilink_bot_token"),
        );
        h.insert(
            HeaderName::from_static("content-length"),
            HeaderValue::from_str(&body.len().to_string()).unwrap_or(HeaderValue::from_static("0")),
        );
        h.insert(
            HeaderName::from_static("x-wechat-uin"),
            HeaderValue::from_str(&random_wechat_uin_header()).unwrap_or(HeaderValue::from_static("MA==")),
        );
        if let Ok(v) = HeaderValue::from_str(&format!("Bearer {}", self.config.ilink_token.trim())) {
            h.insert(HeaderName::from_static("authorization"), v);
        }
        h
    }

    pub(super) async fn post_ilink(&self, endpoint: &str, body: Value, timeout: Duration) -> Result<String> {
        let url = format!("{}{}", self.base_url(), endpoint.trim_start_matches('/'));
        let body_str = serde_json::to_string(&body)?;
        let client = reqwest::Client::builder().timeout(timeout).build()?;
        let resp = client
            .post(&url)
            .headers(self.build_auth_headers(&body_str))
            .body(body_str)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(anyhow!("ilink {endpoint} HTTP {status}: {text}"));
        }
        // WeChat iLink returns HTTP 200 even on application errors. The actual
        // status lives in the JSON body's `ret` / `errcode` fields. We MUST
        // surface those as errors here so callers (e.g. `send_message_raw`)
        // notice failures like expired `context_token` instead of silently
        // dropping messages. `getupdates` callers parse the body themselves
        // and tolerate `ret == -14`, so we only enforce this for the
        // `sendmessage` endpoint where the body is well-defined.
        if endpoint.contains("sendmessage") || endpoint.contains("sendtyping") || endpoint.contains("getconfig") {
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                let ret = v["ret"].as_i64().unwrap_or(0);
                let errcode = v["errcode"].as_i64().unwrap_or(0);
                if ret != 0 || errcode != 0 {
                    let errmsg = v["errmsg"]
                        .as_str()
                        .or_else(|| v["msg"].as_str())
                        .unwrap_or("")
                        .to_string();
                    return Err(anyhow!(
                        "ilink {endpoint} application error ret={ret} errcode={errcode} errmsg={errmsg}"
                    ));
                }
            }
        }
        Ok(text)
    }
}
