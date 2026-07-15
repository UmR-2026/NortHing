//! Weixin (微信) iLink bot integration — Round 39a facade.
//!
//! The original `weixin.rs` was a 2157-line god-file.  After the R39a god-
//! split it became a thin facade: every cross-crate consumer still reaches
//! the public API through `bot::weixin::XYZ`, but the body is now spread
//! across the following siblings (all `pub mod` declared in `bot/mod.rs`):
//!
//!   * [`weixin_crypto`]    — AES-128-ECB + CDN URL helpers (`pub(super)`)
//!   * [`weixin_qr_login`]  — QR session store + `weixin_qr_start` / `weixin_qr_poll`
//!                            + helpers used by the bot driver
//!                            (`load_sync_buf` / `now_ms` / `random_wechat_uin_header` /
//!                            `ensure_trailing_slash`)
//!   * [`weixin_bot`]       — `WeixinConfig`, `WeixinBot` struct, `TypingHandle` +
//!                            Drop, `PendingPairing`, `new` / `restore_chat_state` /
//!                            `base_url` / `is_session_paused` / `pause_session` /
//!                            `build_auth_headers` / `post_ilink`.  Cross-sibling
//!                            tunables (`API_TIMEOUT_SECS`, `CHANNEL_VERSION`,
//!                            `LONG_POLL_TIMEOUT_SECS`, `SESSION_EXPIRED_ERRCODE`,
//!                            `SESSION_PAUSE_SECS`, `MAX_TEXT_CHUNK`,
//!                            `MAX_INBOUND_IMAGES`) live here.
//!   * [`weixin_bot_media`]  — outbound `send_message_*`, `send_workspace_*`,
//!                             `send_text`, `cdn_base_url`, `fetch_weixin_cdn_bytes`,
//!                             `ilink_get_upload_url`, `upload_bytes_to_weixin_cdn`,
//!                             `media_aes_key_b64`, typing indicator (`start_typing`,
//!                             `fetch_typing_ticket`, `send_typing`),
//!                             `chunk_text_for_weixin`.  Tests for
//!                             `context_token_error_heuristic` +
//!                             `media_aes_key_b64_*`.
//!   * [`weixin_bot_inbound`] — inbound parsing (`body_from_message`,
//!                              `peer_id`, `context_token`, …), pairing store
//!                              (`register_pairing`, `verify_pairing_code`),
//!                              `wait_for_pairing`, `run_message_loop`,
//!                              `handle_incoming_message`, `deliver_interaction`,
//!                              `send_handle_result`, `notify_files_ready`,
//!                              `persist_chat_state`.  Tests for
//!                              `body_from_message_*`.
//!
//! The classic `bot::weixin::WeixinConfig` / `bot::weixin::WeixinBot` /
//! `bot::weixin::weixin_qr_start` / `bot::weixin::weixin_qr_poll` paths
//! are kept alive via wildcard re-exports of every sibling below.

pub use super::weixin_bot::WeixinBot;
pub use super::weixin_bot::WeixinConfig;
pub use super::weixin_qr_login::{
    weixin_qr_poll, weixin_qr_start, WeixinQrPollResponse, WeixinQrPollStatus, WeixinQrStartResponse,
};
