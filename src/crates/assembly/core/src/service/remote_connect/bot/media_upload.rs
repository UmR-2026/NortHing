//! Weixin bot — outbound CDN upload + workspace file send.
//!
//! Encrypts plaintext with a fresh AES-128 key, asks the iLink API for a
//! CDN upload URL, POSTs the ciphertext, and packages the result as a
//! media item that can be sent via [`super::super::weixin_bot_inbound`]'
//! outbound paths.  Also bundles a workspace file (`send_workspace_file_to_peer`)
//! — read from disk, MIME-sniffed, encrypted, uploaded, and emitted as
//! image/video/file item depending on the resolved type.
//!
//! Implementation is split across sibling files; see `weixin_bot_media.rs`
//! for the facade / module index.
use anyhow::{anyhow, Result};
use rand::RngCore;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{debug, info};

use super::media_types::{UploadUrlResult, UploadedMediaInfo};
use super::weixin_bot::{WeixinBot, API_TIMEOUT_SECS, CHANNEL_VERSION};
use super::weixin_crypto::{
    aes_ecb_ciphertext_len, build_cdn_upload_url, encrypt_aes_128_ecb_pkcs7, md5_hex_lower, MAX_WEIXIN_FILE_BYTES,
};

impl WeixinBot {
    /// `ilink/bot/getuploadurl` — returns either `upload_full_url` (preferred,
    /// when the server pre-builds the complete CDN URL) and/or
    /// `upload_param` (legacy, requires client-side URL composition).
    /// Mirrors `getUploadUrl` in `@tencent-weixin/openclaw-weixin@2.x`.
    #[allow(clippy::too_many_arguments)]
    async fn ilink_get_upload_url(
        &self,
        to_user_id: &str,
        filekey: &str,
        media_type: i64,
        rawsize: u64,
        rawfilemd5: &str,
        filesize: usize,
        aeskey_hex: &str,
    ) -> Result<UploadUrlResult> {
        let body = json!({
            "filekey": filekey,
            "media_type": media_type,
            "to_user_id": to_user_id,
            "rawsize": rawsize,
            "rawfilemd5": rawfilemd5,
            "filesize": filesize,
            "no_need_thumb": true,
            "aeskey": aeskey_hex,
            "base_info": { "channel_version": CHANNEL_VERSION }
        });
        let raw = self
            .post_ilink("ilink/bot/getuploadurl", body, Duration::from_secs(API_TIMEOUT_SECS))
            .await?;
        let v: Value = serde_json::from_str(&raw)?;
        let pick =
            |k: &str| -> Option<String> { v[k].as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) };
        let upload_full_url = pick("upload_full_url");
        let upload_param = pick("upload_param");
        if upload_full_url.is_none() && upload_param.is_none() {
            return Err(anyhow!("getuploadurl: missing both upload_full_url and upload_param"));
        }
        Ok(UploadUrlResult {
            upload_full_url,
            upload_param,
        })
    }

    async fn post_weixin_cdn_upload(&self, cdn_url: &str, ciphertext: &[u8]) -> Result<String> {
        let client = reqwest::Client::builder().timeout(Duration::from_secs(120)).build()?;
        let mut last_err: Option<anyhow::Error> = None;
        for attempt in 1..=super::weixin_crypto::CDN_UPLOAD_MAX_RETRIES {
            let resp = client
                .post(cdn_url)
                .header("Content-Type", "application/octet-stream")
                .body(ciphertext.to_vec())
                .send()
                .await;
            let resp = match resp {
                Ok(r) => r,
                Err(e) => {
                    last_err = Some(anyhow!("CDN upload attempt {attempt}: {e}"));
                    if attempt < super::weixin_crypto::CDN_UPLOAD_MAX_RETRIES {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    continue;
                }
            };
            let status = resp.status();
            if status.is_client_error() {
                let body = resp.text().await.unwrap_or_default();
                return Err(anyhow!("CDN client error {status}: {body}"));
            }
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                last_err = Some(anyhow!("CDN server error {status}: {body}"));
                if attempt < super::weixin_crypto::CDN_UPLOAD_MAX_RETRIES {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                continue;
            }
            let download_param = resp
                .headers()
                .get("x-encrypted-param")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty());
            return download_param.ok_or_else(|| anyhow!("CDN response missing x-encrypted-param header"));
        }
        Err(last_err.unwrap_or_else(|| anyhow!("CDN upload failed")))
    }

    /// Read plaintext → encrypt → getuploadurl → POST to CDN (same pipeline as OpenClaw weixin plugin).
    async fn upload_bytes_to_weixin_cdn(
        &self,
        to_user_id: &str,
        plaintext: &[u8],
        media_type: i64,
    ) -> Result<UploadedMediaInfo> {
        let rawsize = plaintext.len() as u64;
        let rawfilemd5 = md5_hex_lower(plaintext);
        let mut aeskey = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut aeskey);
        let aeskey_hex = hex::encode(aeskey);
        let filesize_cipher = aes_ecb_ciphertext_len(plaintext.len());
        let ciphertext = encrypt_aes_128_ecb_pkcs7(plaintext, &aeskey);

        let mut filekey_raw = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut filekey_raw);
        let filekey = hex::encode(filekey_raw);

        let url_resp = self
            .ilink_get_upload_url(
                to_user_id,
                &filekey,
                media_type,
                rawsize,
                &rawfilemd5,
                filesize_cipher,
                &aeskey_hex,
            )
            .await?;

        let cdn_url = if let Some(full) = url_resp.upload_full_url.as_deref() {
            full.to_string()
        } else if let Some(param) = url_resp.upload_param.as_deref() {
            build_cdn_upload_url(self.cdn_base_url(), param, &filekey)
        } else {
            return Err(anyhow!("getuploadurl: missing both upload_full_url and upload_param"));
        };
        debug!(
            "weixin CDN upload: media_type={media_type} rawsize={rawsize} cipher_len={}",
            ciphertext.len()
        );
        let download_encrypted_query_param = self.post_weixin_cdn_upload(&cdn_url, &ciphertext).await?;

        Ok(UploadedMediaInfo {
            download_encrypted_query_param,
            aeskey_hex,
            file_size_plain: rawsize,
            file_size_cipher: ciphertext.len(),
        })
    }

    async fn send_message_with_items(&self, to_user_id: &str, context_token: &str, items: Vec<Value>) -> Result<()> {
        let client_id = format!("northhing-wx-{}", uuid::Uuid::new_v4());
        let msg = json!({
            "from_user_id": "",
            "to_user_id": to_user_id,
            "client_id": client_id,
            "message_type": 2,
            "message_state": 2,
            "item_list": items,
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

    /// Upload a workspace file and send as image / video / file attachment (like Feishu `send_file_to_feishu_chat`).
    pub(super) async fn send_workspace_file_to_peer(
        &self,
        peer_id: &str,
        raw_path: &str,
        workspace_root: Option<&std::path::Path>,
    ) -> Result<()> {
        let content = super::read_workspace_file(raw_path, MAX_WEIXIN_FILE_BYTES, workspace_root).await?;
        let mime = super::detect_mime_type(std::path::Path::new(&content.name));

        let token = {
            let m = self.context_tokens.read().await;
            m.get(peer_id)
                .cloned()
                .ok_or_else(|| anyhow!("missing context_token for peer {peer_id}"))?
        };

        let item: Value = if mime.starts_with("image/") {
            let up = self.upload_bytes_to_weixin_cdn(peer_id, &content.bytes, 1).await?;
            let aes_b64 = Self::media_aes_key_b64(&up.aeskey_hex)?;
            json!({
                "type": 2,
                "image_item": {
                    "media": {
                        "encrypt_query_param": up.download_encrypted_query_param,
                        "aes_key": aes_b64,
                        "encrypt_type": 1
                    },
                    "mid_size": up.file_size_cipher
                }
            })
        } else if mime.starts_with("video/") {
            let up = self.upload_bytes_to_weixin_cdn(peer_id, &content.bytes, 2).await?;
            let aes_b64 = Self::media_aes_key_b64(&up.aeskey_hex)?;
            json!({
                "type": 5,
                "video_item": {
                    "media": {
                        "encrypt_query_param": up.download_encrypted_query_param,
                        "aes_key": aes_b64,
                        "encrypt_type": 1
                    },
                    "video_size": up.file_size_cipher
                }
            })
        } else {
            let up = self.upload_bytes_to_weixin_cdn(peer_id, &content.bytes, 3).await?;
            let aes_b64 = Self::media_aes_key_b64(&up.aeskey_hex)?;
            json!({
                "type": 4,
                "file_item": {
                    "media": {
                        "encrypt_query_param": up.download_encrypted_query_param,
                        "aes_key": aes_b64,
                        "encrypt_type": 1
                    },
                    "file_name": content.name,
                    "len": format!("{}", up.file_size_plain)
                }
            })
        };

        self.send_message_with_items(peer_id, &token, vec![item]).await?;
        info!("Weixin file sent to peer={peer_id} name={}", content.name);
        Ok(())
    }
}
