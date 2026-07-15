//! Weixin bot — inbound CDN download + image decryption.
//!
//! Pulls bytes from the WeChat CDN for inbound images, decrypts them with
//! AES-128-ECB-PKCS7, and packages them as Feishu-style `ImageAttachment`
//! data URLs so the rest of the pipeline (image analyzer, dialog turn,
//! workspace bridge) can consume them without any Weixin-specific glue.
//!
//! Implementation is split across sibling files; see `weixin_bot_media.rs`
//! for the facade / module index.
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde_json::Value;
use std::time::Duration;
use tracing::warn;

use super::weixin_bot::{WeixinBot, MAX_INBOUND_IMAGES};
use super::weixin_crypto::{
    build_cdn_download_url, decrypt_aes_128_ecb_pkcs7, parse_weixin_cdn_aes_key, sniff_image_mime, DEFAULT_CDN_BASE_URL,
};
use crate::service::remote_connect::remote_server::ImageAttachment;

impl WeixinBot {
    pub(super) fn cdn_base_url(&self) -> &'static str {
        DEFAULT_CDN_BASE_URL
    }

    /// Download CDN bytes.  Prefers `full_url` (when the server pre-builds the
    /// complete URL, matching `@tencent-weixin/openclaw-weixin@2.x`'s
    /// `CDNMedia.full_url`); otherwise falls back to building the URL from
    /// `encrypted_query_param`.
    async fn fetch_weixin_cdn_bytes(&self, encrypted_query_param: &str, full_url: Option<&str>) -> Result<Vec<u8>> {
        let url = match full_url.map(str::trim).filter(|s: &&str| !s.is_empty()) {
            Some(u) => u.to_string(),
            None => build_cdn_download_url(self.cdn_base_url(), encrypted_query_param),
        };
        let client = reqwest::Client::builder().timeout(Duration::from_secs(120)).build()?;
        let resp = client.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("weixin CDN GET {status}: {body}"));
        }
        Ok(resp.bytes().await?.to_vec())
    }

    /// Decrypt one inbound `image_item` (CDN download + AES-128-ECB), matching OpenClaw `downloadMediaFromItem`.
    async fn inbound_image_bytes_from_item(&self, item: &Value) -> Result<Vec<u8>> {
        let img = &item["image_item"];
        let param = img["media"]["encrypt_query_param"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("image: missing encrypt_query_param"))?;
        let full_url = img["media"]["full_url"].as_str();

        let key: Option<[u8; 16]> = if let Some(hex_s) = img["aeskey"].as_str().filter(|s| !s.is_empty()) {
            let bytes = hex::decode(hex_s.trim()).map_err(|e| anyhow!("image aeskey hex: {e}"))?;
            if bytes.len() != 16 {
                return Err(anyhow!("image aeskey must decode to 16 bytes"));
            }
            let mut k = [0u8; 16];
            k.copy_from_slice(&bytes);
            Some(k)
        } else if let Some(b64) = img["media"]["aes_key"].as_str().filter(|s| !s.is_empty()) {
            Some(parse_weixin_cdn_aes_key(b64)?)
        } else {
            None
        };

        let enc = self.fetch_weixin_cdn_bytes(param, full_url).await?;
        match key {
            Some(k) => decrypt_aes_128_ecb_pkcs7(&enc, &k),
            None => Ok(enc),
        }
    }

    /// Collect up to [`MAX_INBOUND_IMAGES`] images from `item_list` as Feishu-style `ImageAttachment` data URLs.
    pub(super) async fn inbound_image_attachments_from_message(&self, msg: &Value) -> (Vec<ImageAttachment>, usize) {
        const MAX_BYTES: usize = 1024 * 1024;
        let Some(items) = msg["item_list"].as_array() else {
            return (vec![], 0);
        };
        let total_with_param = items
            .iter()
            .filter(|i| {
                i["type"].as_i64() == Some(2)
                    && i["image_item"]["media"]["encrypt_query_param"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty())
            })
            .count();
        let skipped = total_with_param.saturating_sub(MAX_INBOUND_IMAGES);

        let mut attachments = Vec::new();
        for item in items {
            if attachments.len() >= MAX_INBOUND_IMAGES {
                break;
            }
            if item["type"].as_i64() != Some(2) {
                continue;
            }
            match self.inbound_image_bytes_from_item(item).await {
                Ok(raw) => {
                    let mime = sniff_image_mime(&raw);
                    let data_url = if raw.len() <= MAX_BYTES {
                        let b64 = B64.encode(&raw);
                        format!("data:{mime};base64,{b64}")
                    } else {
                        let raw_fallback = raw.clone();
                        match crate::agentic::image_analysis::optimize_image_with_size_limit(
                            raw,
                            "openai",
                            Some(mime),
                            Some(MAX_BYTES),
                        ) {
                            Ok(processed) => {
                                let b64 = B64.encode(&processed.data);
                                format!("data:{};base64,{}", processed.mime_type, b64)
                            }
                            Err(e) => {
                                warn!("Weixin image compression failed: {e}");
                                let b64 = B64.encode(&raw_fallback);
                                format!("data:{mime};base64,{b64}")
                            }
                        }
                    };
                    attachments.push(ImageAttachment {
                        name: format!("weixin_image_{}.jpg", attachments.len() + 1),
                        data_url,
                    });
                }
                Err(e) => warn!("Weixin inbound image download failed: {e}"),
            }
        }
        (attachments, skipped)
    }
}
