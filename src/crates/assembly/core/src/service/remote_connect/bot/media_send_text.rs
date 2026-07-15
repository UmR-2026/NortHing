//! Weixin bot — outbound text pipeline + `getupdates` long-poll.
//!
//! Owns:
//!   * `send_message_raw` / `send_text` / `try_send_text` — outbound text
//!     message pipeline, including the stale-`context_token` invalidation
//!     heuristic ([`super::media_validate::is_context_token_error`]).
//!   * `get_updates_once` — single `ilink/bot/getupdates` call, which is
//!     consumed by `weixin_bot_inbound.rs` for both pairing and the
//!     post-pairing message loop.  Lives here (rather than in inbound)
//!     because the iLink poll body shape and session-expired handling
//!     mirror the outbound send pipeline.
//!
//! Implementation is split across sibling files; see `weixin_bot_media.rs`
//! for the facade / module index.
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::warn;

use super::weixin_bot::{WeixinBot, API_TIMEOUT_SECS, CHANNEL_VERSION, SESSION_EXPIRED_ERRCODE};

impl WeixinBot {
    pub(super) async fn get_updates_once(&self, buf: &str, timeout: Duration) -> Result<Value> {
        if self.is_session_paused().await {
            tokio::time::sleep(Duration::from_secs(2)).await;
            return Ok(json!({
                "ret": 0,
                "msgs": [],
                "get_updates_buf": buf
            }));
        }

        let body = json!({
            "get_updates_buf": buf,
            "base_info": { "channel_version": CHANNEL_VERSION }
        });
        let raw = self.post_ilink("ilink/bot/getupdates", body, timeout).await?;
        let v: Value = serde_json::from_str(&raw)?;
        let ret = v["ret"].as_i64().unwrap_or(0);
        let errcode = v["errcode"].as_i64().unwrap_or(0);
        if errcode == SESSION_EXPIRED_ERRCODE || ret == SESSION_EXPIRED_ERRCODE {
            self.pause_session().await;
        }
        Ok(v)
    }

    async fn send_message_raw(&self, to_user_id: &str, context_token: &str, text: &str) -> Result<()> {
        let client_id = format!("northhing-wx-{}", uuid::Uuid::new_v4());
        let item_list = if text.is_empty() {
            None
        } else {
            Some(vec![json!({
                "type": 1,
                "text_item": { "text": text }
            })])
        };
        let msg = json!({
            "from_user_id": "",
            "to_user_id": to_user_id,
            "client_id": client_id,
            "message_type": 2,
            "message_state": 2,
            "item_list": item_list,
            "context_token": context_token,
        });
        let body = json!({
            "msg": msg,
            "base_info": { "channel_version": CHANNEL_VERSION }
        });
        self.post_ilink("ilink/bot/sendmessage", body, Duration::from_secs(API_TIMEOUT_SECS))
            .await?;
        Ok(())
    }

    /// Send text to peer; uses last known `context_token` for that peer.
    ///
    /// If the WeChat iLink API rejects the message (typically because the
    /// `context_token` has expired or exceeded its usage budget), we drop
    /// the cached token so subsequent sends fail fast with a clear error
    /// instead of silently retrying a known-bad token. The token will be
    /// refreshed automatically the next time the user sends an inbound
    /// message (see `run_message_loop` / `wait_for_pairing`).
    pub async fn send_text(&self, peer_id: &str, text: &str) -> Result<()> {
        let token = {
            let m = self.context_tokens.read().await;
            m.get(peer_id).cloned().ok_or_else(|| {
                anyhow!("context_token unavailable for peer {peer_id} (waiting for next inbound message)")
            })?
        };
        for chunk in super::media_validate::chunk_text_for_weixin(text) {
            if let Err(e) = self.send_message_raw(peer_id, &token, &chunk).await {
                if Self::is_context_token_error(&e) {
                    let mut m = self.context_tokens.write().await;
                    if m.get(peer_id).map(|t| t == &token).unwrap_or(false) {
                        m.remove(peer_id);
                        warn!("weixin: dropped stale context_token for peer {peer_id} after send error: {e}");
                    }
                }
                return Err(e);
            }
        }
        Ok(())
    }

    /// Best-effort send that logs a warning on failure instead of silently
    /// swallowing the error. Use this for non-critical replies (welcome,
    /// pairing-error hints, etc.) where we don't want to abort the caller
    /// but we DO want a log record if the send actually failed.
    pub(super) async fn try_send_text(&self, peer_id: &str, text: &str, ctx: &str) {
        if let Err(e) = self.send_text(peer_id, text).await {
            warn!("weixin: {ctx} send to peer {peer_id} failed: {e}");
        }
    }
}
