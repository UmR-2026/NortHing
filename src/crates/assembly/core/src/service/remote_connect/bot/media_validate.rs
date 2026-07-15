//! Weixin bot — outbound encoding helpers + small validators.
//!
//! Pure / near-pure helpers that the other `bot` siblings lean on but that
//! don't deserve their own file:
//!   * [`media_aes_key_b64`] — base64-of-ASCII-hex quirk used by every
//!     outbound media item (matches `@tencent-weixin/openclaw-weixin@2.x`).
//!   * [`is_context_token_error`] — heuristic used by `send_text` to decide
//!     whether to drop a stale `context_token` after a failed send.
//!   * [`chunk_text_for_weixin`] — outbound text splitter at the
//!     `MAX_TEXT_CHUNK` boundary (UTF-8 safe).
//!
//! Implementation is split across sibling files; see `weixin_bot_media.rs`
//! for the facade / module index.
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

use super::weixin_bot::{WeixinBot, MAX_TEXT_CHUNK};

impl WeixinBot {
    /// `aes_key` in JSON for outbound media items.
    ///
    /// Quirk match with the official `@tencent-weixin/openclaw-weixin@2.x`
    /// reference plugin: it does `Buffer.from(aeskey.toString("hex")).toString("base64")`,
    /// which treats the 32-char hex *string* as UTF-8 bytes and base64-encodes
    /// **those ASCII bytes** — NOT the raw 16 binary bytes.  The downstream
    /// WeChat client decodes the value, sees 32 ASCII hex chars, and hex-
    /// decodes back to the original 16-byte AES key.  We were previously
    /// shipping `base64(raw 16 bytes)` (the "obvious" interpretation), which
    /// the WeChat client cannot decrypt — the file appeared in the chat but
    /// every download attempt failed with "下载失败".  Stay bug-compatible
    /// with the reference so the client can decrypt the CDN payload.
    pub(super) fn media_aes_key_b64(aeskey_hex: &str) -> Result<String> {
        let trimmed = aeskey_hex.trim();
        if trimmed.len() != 32 || !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(anyhow!("aeskey must be 32 ascii hex chars"));
        }
        Ok(B64.encode(trimmed.as_bytes()))
    }

    /// Heuristic: treat any send error mentioning an iLink application error
    /// (or a ret/errcode payload) as a context_token-expiration signal.
    /// We invalidate aggressively because the only thing we can do with a
    /// bad token is stop using it.
    pub(super) fn is_context_token_error(err: &anyhow::Error) -> bool {
        let s = err.to_string();
        s.contains("application error") || s.contains("context_token") || s.contains("errcode=")
    }
}

/// Split an outbound reply into chunks no larger than [`MAX_TEXT_CHUNK`]
/// bytes while staying on UTF-8 character boundaries (WeChat rejects any
/// payload that ends mid-codepoint).  The cap mirrors the iLink API limit
/// and is intentionally character-agnostic — it counts bytes, not graphemes.
pub(super) fn chunk_text_for_weixin(text: &str) -> Vec<String> {
    if text.len() <= MAX_TEXT_CHUNK {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        if rest.len() <= MAX_TEXT_CHUNK {
            out.push(rest.to_string());
            break;
        }
        let mut cut = MAX_TEXT_CHUNK;
        while cut > 0 && !rest.is_char_boundary(cut) {
            cut -= 1;
        }
        if cut == 0 {
            cut = rest.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        }
        out.push(rest[..cut].to_string());
        rest = &rest[cut..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use base64::engine::general_purpose::STANDARD as B64;
    #[allow(unused_imports)]
    use base64::Engine as _;

    /// Sanity-check the heuristic used by `send_text` to decide whether a
    /// failed `send_message_raw` indicates the cached `context_token` has
    /// gone bad. Application errors and explicit `errcode=` strings must
    /// trigger token invalidation; pure transport errors (network/HTTP)
    /// must NOT, so we don't drop a perfectly good token after a transient
    /// blip.
    #[test]
    fn context_token_error_heuristic() {
        let app_err =
            anyhow!("ilink ilink/bot/sendmessage application error ret=0 errcode=12345 errmsg=context_token expired");
        assert!(WeixinBot::is_context_token_error(&app_err));

        let app_err_short = anyhow!("upstream returned errcode=42 unauthorized");
        assert!(WeixinBot::is_context_token_error(&app_err_short));

        let net_err = anyhow!("error sending request: connection refused");
        assert!(!WeixinBot::is_context_token_error(&net_err));

        let http_err = anyhow!("ilink ilink/bot/sendmessage HTTP 500 Internal Server Error");
        assert!(!WeixinBot::is_context_token_error(&http_err));
    }

    /// Outbound `aes_key` MUST be base64 of the 32-char hex *string* (its
    /// ASCII bytes), NOT base64 of the 16 raw key bytes.  This matches the
    /// official `@tencent-weixin/openclaw-weixin@2.x` reference plugin and
    /// is what the WeChat client expects when it pulls the file from CDN —
    /// otherwise every download fails with "下载失败" even though the bot
    /// successfully delivers the message itself.
    #[test]
    fn media_aes_key_b64_matches_openclaw_hex_ascii_format() {
        let raw = [
            0x01u8, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10,
        ];
        let aeskey_hex = hex::encode(raw);
        let produced = WeixinBot::media_aes_key_b64(&aeskey_hex).unwrap();
        let expected = B64.encode(aeskey_hex.as_bytes());
        assert_eq!(
            produced, expected,
            "media_aes_key_b64 must base64-encode the hex string ASCII bytes (OpenClaw quirk)"
        );
        let decoded = B64.decode(&produced).unwrap();
        assert_eq!(
            decoded.len(),
            32,
            "decoded value must be 32 ASCII chars, not 16 raw bytes"
        );
        assert!(
            std::str::from_utf8(&decoded)
                .map(|s| s.chars().all(|c| c.is_ascii_hexdigit()))
                .unwrap_or(false),
            "decoded payload must be the original hex string"
        );
    }

    #[test]
    fn media_aes_key_b64_rejects_non_hex_input() {
        assert!(WeixinBot::media_aes_key_b64("not_hex_at_all").is_err());
        assert!(WeixinBot::media_aes_key_b64("zz".repeat(16).as_str()).is_err());
        assert!(WeixinBot::media_aes_key_b64("ab").is_err());
    }
}
