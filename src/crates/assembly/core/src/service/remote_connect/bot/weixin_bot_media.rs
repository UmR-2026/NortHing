//! Weixin bot — outbound media + messaging pipeline (facade).
//!
//! Owns every `impl WeixinBot` method that pushes bytes / messages to the
//! peer, plus the inbound CDN download pipeline that the dialog turn
//! pipeline relies on.  Implementation is split across sibling files for
//! navigation; this module is purely a facade + module index — the actual
//! `impl` blocks live in the sibling files declared next to it in
//! [`super::mod`]:
//!
//!   * [`media_types`](super::media_types) — DTOs returned by the upload
//!     pipeline (`UploadedMediaInfo`, `UploadUrlResult`).
//!   * [`media_validate`](super::media_validate) — encoding helpers
//!     ([`media_aes_key_b64`](super::media_validate::media_aes_key_b64),
//!     [`chunk_text_for_weixin`](super::media_validate::chunk_text_for_weixin),
//!     [`is_context_token_error`](super::media_validate::is_context_token_error))
//!     plus the corresponding unit tests.
//!   * [`media_download`](super::media_download) — CDN download + AES
//!     decryption for inbound images
//!     ([`inbound_image_attachments_from_message`]).
//!   * [`media_upload`](super::media_upload) — CDN upload pipeline
//!     ([`upload_bytes_to_weixin_cdn`]) + workspace file send
//!     ([`send_workspace_file_to_peer`]).
//!   * [`media_send_text`](super::media_send_text) — outbound text message
//!     pipeline ([`send_text`], [`try_send_text`]) +
//!     `ilink/bot/getupdates` poll ([`get_updates_once`]).
//!   * [`media_typing`](super::media_typing) — typing indicator
//!     ([`start_typing`] returning the RAII guard from `weixin_bot.rs`).
//!
//! Methods that *receive* / dispatch inbound messages live in the sibling
//! [`super::weixin_bot_inbound`].  Rust merges all `impl WeixinBot { ... }`
//! blocks at link time, so the split is purely organisational.
