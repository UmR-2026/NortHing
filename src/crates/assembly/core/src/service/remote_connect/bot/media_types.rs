//! Weixin bot — media pipeline DTOs.
//!
//! Small data carriers returned by the CDN upload pipeline
//! ([`UploadedMediaInfo`]) and the `ilink/bot/getuploadurl` call
//! ([`UploadUrlResult`]).  These are `pub(super)` so the other `bot` module
//! siblings can reference them via `super::media_types::Type`.
//!
//! See `weixin_bot_media.rs` for the facade / module index.

/// Successful upload record produced by
/// [`super::media_upload::upload_bytes_to_weixin_cdn`].  Carries everything a
/// downstream caller needs to embed the media in an outbound message item:
/// the encrypted query param returned by the CDN `x-encrypted-param` header
/// (used by WeChat to download the bytes later), the AES key in hex form
/// (the value embedded in the message item is a base64-of-ASCII-hex quirk —
/// see [`super::media_validate::media_aes_key_b64`]), and both the plaintext
/// and ciphertext sizes for the `*_size` fields the iLink API expects.
#[derive(Debug)]
pub(super) struct UploadedMediaInfo {
    pub(super) download_encrypted_query_param: String,
    pub(super) aeskey_hex: String,
    pub(super) file_size_plain: u64,
    pub(super) file_size_cipher: usize,
}

/// Result of `ilink/bot/getuploadurl`: the server may return either a
/// pre-built complete CDN URL (`upload_full_url`, preferred) or just the
/// `upload_param` to be combined with `cdn_base_url` and `filekey`.
#[derive(Debug, Clone)]
pub(super) struct UploadUrlResult {
    pub(super) upload_full_url: Option<String>,
    pub(super) upload_param: Option<String>,
}
