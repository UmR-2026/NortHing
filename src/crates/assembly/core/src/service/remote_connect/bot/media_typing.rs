//! Weixin bot — typing indicator (ilink/bot/getconfig + ilink/bot/sendtyping).
//!
//! Per `@tencent-weixin/openclaw-weixin` (`src/api/api.ts`), driving the
//! "对方正在输入" hint above the WeChat chat input requires two calls:
//!   1. `POST ilink/bot/getconfig`   → returns a base64 `typing_ticket`
//!      bound to the `(bot, ilink_user_id, context_token)` triple.
//!   2. `POST ilink/bot/sendtyping`  → with `status=1` to start typing and
//!      `status=2` to cancel (also auto-times out server-side after a few
//!      seconds, hence the 5-second refresh cadence used below).
//!
//! [`fetch_typing_ticket`] always invokes `ilink/bot/getconfig` (does NOT
//! consult the cache) so the caller can recover from a stale ticket by
//! clearing it and calling here again.
//!
//! Implementation is split across sibling files; see `weixin_bot_media.rs`
//! for the facade / module index.
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

use super::weixin_bot::{TypingHandle, WeixinBot, API_TIMEOUT_SECS, CHANNEL_VERSION};

impl WeixinBot {
    /// Fetch a fresh typing_ticket for `peer_id`. Always invokes
    /// `ilink/bot/getconfig` (does NOT consult the cache) so the caller can
    /// recover from a stale ticket by clearing it and calling here again.
    async fn fetch_typing_ticket(&self, peer_id: &str) -> Result<String> {
        let context_token = {
            let m = self.context_tokens.read().await;
            m.get(peer_id).cloned()
        };
        let mut body = json!({
            "ilink_user_id": peer_id,
            "base_info": { "channel_version": CHANNEL_VERSION }
        });
        if let Some(ct) = context_token {
            body["context_token"] = json!(ct);
        }
        let raw = self
            .post_ilink("ilink/bot/getconfig", body, Duration::from_secs(API_TIMEOUT_SECS))
            .await?;
        let v: Value = serde_json::from_str(&raw)?;
        let ticket = v["typing_ticket"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("ilink/bot/getconfig returned empty typing_ticket"))?;
        let mut m = self.typing_tickets.write().await;
        m.insert(peer_id.to_string(), ticket.clone());
        Ok(ticket)
    }

    /// Send one typing event (`status`: 1 = start, 2 = cancel). Lazily fetches
    /// a typing_ticket on the first call per peer and refreshes once on
    /// ticket-related errors before giving up.
    pub(super) async fn send_typing(&self, peer_id: &str, status: i64) -> Result<()> {
        let cached = {
            let m = self.typing_tickets.read().await;
            m.get(peer_id).cloned()
        };
        let ticket = match cached {
            Some(t) => t,
            None => self.fetch_typing_ticket(peer_id).await?,
        };

        let send_with = |t: String| async move {
            let body = json!({
                "ilink_user_id": peer_id,
                "typing_ticket": t,
                "status": status,
                "base_info": { "channel_version": CHANNEL_VERSION }
            });
            self.post_ilink("ilink/bot/sendtyping", body, Duration::from_secs(API_TIMEOUT_SECS))
                .await
        };

        match send_with(ticket.clone()).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // Drop the stale ticket and retry once with a fresh one. We
                // can't reliably distinguish ticket errors from transient
                // failures, so we always try to recover at most once.
                {
                    let mut m = self.typing_tickets.write().await;
                    if m.get(peer_id).map(|t| t == &ticket).unwrap_or(false) {
                        m.remove(peer_id);
                    }
                }
                debug!("weixin: typing ticket retry for peer {peer_id} (prev err: {e})");
                let fresh = self.fetch_typing_ticket(peer_id).await?;
                send_with(fresh).await?;
                Ok(())
            }
        }
    }

    /// Spawn a background task that emits `sendtyping(status=1)` immediately
    /// and refreshes it every 5 seconds. The returned [`TypingHandle`] cancels
    /// the loop and emits `sendtyping(status=2)` when stopped or dropped, so
    /// the "正在输入" hint disappears on the user's side as soon as the bot
    /// finishes responding.
    pub(super) fn start_typing(self: &Arc<Self>, peer_id: String) -> TypingHandle {
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_task = cancel.clone();
        let bot = self.clone();
        let peer_for_task = peer_id.clone();
        let handle = tokio::spawn(async move {
            // Refresh interval matches OpenClaw's 6s default cadence; we use
            // 5s to leave a small safety margin against server-side timeout.
            // Each "wait" between refreshes is broken into 100ms ticks so a
            // stop signal from the main task is observed within ≤100ms even
            // mid-wait, which keeps the indicator from lingering after the
            // bot has actually finished responding.
            const TICK: Duration = Duration::from_millis(100);
            const TICKS_PER_REFRESH: u32 = 50; // 50 * 100ms = 5s
            const TICKS_AFTER_FAILURE: u32 = 100; // 100 * 100ms = 10s

            loop {
                if cancel_task.load(Ordering::Acquire) {
                    return;
                }
                let next_wait = match bot.send_typing(&peer_for_task, 1).await {
                    Ok(()) => TICKS_PER_REFRESH,
                    Err(e) => {
                        debug!("weixin: send typing(start) failed for peer {peer_for_task}: {e}");
                        TICKS_AFTER_FAILURE
                    }
                };
                for _ in 0..next_wait {
                    if cancel_task.load(Ordering::Acquire) {
                        return;
                    }
                    tokio::time::sleep(TICK).await;
                }
            }
        });
        TypingHandle {
            cancel,
            handle: Some(handle),
            bot: self.clone(),
            peer_id,
            stopped: false,
        }
    }
}
