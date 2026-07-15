use super::super::{
    auto_push_failed_message, auto_push_skip_too_large_message, collect_auto_push_files, read_workspace_file,
};
use super::feishu_types::{FeishuToken, MAX_FEISHU_FILE_BYTES};
use super::FeishuBot;
use crate::agentic::image_analysis::optimize_image_with_size_limit;
use crate::service::remote_connect::bot::command_router::current_bot_language;
use crate::service::remote_connect::remote_server::ImageAttachment;
use crate::util::truncate_at_char_boundary;

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::multipart::{Form, Part};
use tracing::{debug, info, warn};

// =====================================================================
// Token management
// =====================================================================

impl FeishuBot {
    pub(super) async fn get_access_token(&self) -> Result<String> {
        {
            let guard = self.token.read().await;
            if let Some(t) = guard.as_ref() {
                if t.expires_at > chrono::Utc::now().timestamp() + 60 {
                    return Ok(t.access_token.clone());
                }
            }
        }

        let client = reqwest::Client::new();
        let resp = client
            .post("https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal")
            .json(&serde_json::json!({
                "app_id": self.config.app_id,
                "app_secret": self.config.app_secret,
            }))
            .send()
            .await
            .map_err(|e| anyhow!("feishu token request: {e}"))?;

        let token_resp_text = resp.text().await.unwrap_or_default();
        let body: serde_json::Value = serde_json::from_str(&token_resp_text).map_err(|e| {
            anyhow!(
                "feishu token response parse error: {e}, body: {}",
                truncate_at_char_boundary(&token_resp_text, 200)
            )
        })?;
        let access_token = body["tenant_access_token"]
            .as_str()
            .ok_or_else(|| anyhow!("missing tenant_access_token in response"))?
            .to_string();
        let expire = body["expire"].as_i64().unwrap_or(7200);

        *self.token.write().await = Some(FeishuToken {
            access_token: access_token.clone(),
            expires_at: chrono::Utc::now().timestamp() + expire,
        });

        info!("Feishu access token refreshed");
        Ok(access_token)
    }
}

// =====================================================================
// Image download
// =====================================================================

impl FeishuBot {
    /// Download a user-sent image from a Feishu message using the message resources API.
    /// The returned data-URL is compressed to at most 1 MB.
    pub(super) async fn download_image_as_data_url(&self, message_id: &str, file_key: &str) -> Result<String> {
        let token = match self.get_access_token().await {
            Ok(t) => t,
            Err(e) => {
                return Err(e);
            }
        };
        let client = reqwest::Client::new();
        let url = format!(
            "https://open.feishu.cn/open-apis/im/v1/messages/{}/resources/{}?type=image",
            message_id, file_key
        );
        let resp = client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| anyhow!("feishu download image: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("feishu image download failed: HTTP {status} — {body}"));
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/png")
            .to_string();
        let raw_bytes = resp.bytes().await?;

        const MAX_BYTES: usize = 1024 * 1024;
        if raw_bytes.len() <= MAX_BYTES {
            let b64 = B64.encode(&raw_bytes);
            return Ok(format!("data:{};base64,{}", content_type, b64));
        }

        tracing::info!(
            "Feishu image exceeds {}KB ({}KB), compressing",
            MAX_BYTES / 1024,
            raw_bytes.len() / 1024
        );
        match optimize_image_with_size_limit(raw_bytes.to_vec(), "openai", Some(&content_type), Some(MAX_BYTES)) {
            Ok(processed) => {
                let b64 = B64.encode(&processed.data);
                Ok(format!("data:{};base64,{}", processed.mime_type, b64))
            }
            Err(e) => {
                tracing::warn!("Feishu image compression failed, using original: {e}");
                let b64 = B64.encode(&raw_bytes);
                Ok(format!("data:{};base64,{}", content_type, b64))
            }
        }
    }

    /// Download multiple images and convert to ImageAttachment list.
    pub(super) async fn download_images(&self, message_id: &str, image_keys: &[String]) -> Vec<ImageAttachment> {
        let mut attachments = Vec::new();
        for (i, key) in image_keys.iter().enumerate() {
            match self.download_image_as_data_url(message_id, key).await {
                Ok(data_url) => {
                    attachments.push(ImageAttachment {
                        name: format!("image_{}.png", i + 1),
                        data_url,
                    });
                }
                Err(e) => {
                    warn!("Failed to download Feishu image {key}: {e}");
                }
            }
        }
        attachments
    }
}

// =====================================================================
// File upload and send
// =====================================================================

impl FeishuBot {
    /// Upload a local file to Feishu and return its `file_key`.
    /// Caller is expected to pre-check size against `MAX_FEISHU_FILE_BYTES`.
    pub(super) async fn upload_file_to_feishu(&self, file_path: &str) -> Result<String> {
        let token = self.get_access_token().await?;

        let content = read_workspace_file(file_path, MAX_FEISHU_FILE_BYTES, None).await?;

        // Feishu uses its own file_type enum rather than MIME types.
        let ext = std::path::Path::new(&content.name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let file_type = match ext.as_str() {
            "pdf" => "pdf",
            "doc" | "docx" => "doc",
            "xls" | "xlsx" => "xls",
            "ppt" | "pptx" => "ppt",
            "mp4" => "mp4",
            _ => "stream",
        };

        let part = Part::bytes(content.bytes)
            .file_name(content.name.clone())
            .mime_str("application/octet-stream")?;

        let form = Form::new()
            .text("file_type", file_type.to_string())
            .text("file_name", content.name)
            .part("file", part);

        let client = reqwest::Client::new();
        let resp = client
            .post("https://open.feishu.cn/open-apis/im/v1/files")
            .bearer_auth(&token)
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Feishu file upload failed: {body}"));
        }

        let body: serde_json::Value = resp.json().await?;
        body.pointer("/data/file_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("Feishu upload response missing file_key"))
    }

    /// Upload a local file and send it to a Feishu chat as a file message.
    pub(super) async fn send_file_to_feishu_chat(&self, chat_id: &str, file_path: &str) -> Result<()> {
        let file_key = self.upload_file_to_feishu(file_path).await?;
        let token = self.get_access_token().await?;

        let client = reqwest::Client::new();
        let resp = client
            .post("https://open.feishu.cn/open-apis/im/v1/messages")
            .query(&[("receive_id_type", "chat_id")])
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "receive_id": chat_id,
                "msg_type": "file",
                "content": serde_json::to_string(&serde_json::json!({"file_key": file_key}))?,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Feishu file message failed: {body}"));
        }
        debug!("Feishu file sent to {chat_id}: {file_path}");
        Ok(())
    }

    /// Scan `text` for downloadable file references and push every matching
    /// file directly to the Feishu chat as a `file` message.  Files exceeding
    /// `MAX_FEISHU_FILE_BYTES` are skipped with a brief notice; per-file
    /// failures are reported as plain-text replies.
    pub(super) async fn notify_files_ready(&self, chat_id: &str, text: &str) {
        let language = current_bot_language().await;
        let workspace_root = {
            let states = self.chat_states.read().await;
            states.get(chat_id).and_then(|s| s.active_workspace_path())
        };
        let files = collect_auto_push_files(text, workspace_root.as_deref().map(std::path::Path::new));
        if files.is_empty() {
            return;
        }

        // Skip the "正在为你发送 N 个文件……" intro: the file card itself is
        // visible in the chat; only error / size-skip notices below need to
        // surface to the user.
        for file in files {
            if file.size > MAX_FEISHU_FILE_BYTES {
                let notice = auto_push_skip_too_large_message(language, &file.name, file.size, MAX_FEISHU_FILE_BYTES);
                let _ = self.send_message(chat_id, &notice).await;
                continue;
            }
            match self.send_file_to_feishu_chat(chat_id, &file.abs_path).await {
                Ok(()) => info!("Feishu auto-pushed file to chat {chat_id}: {}", file.abs_path),
                Err(e) => {
                    warn!("Feishu auto-push failed for {} in chat {chat_id}: {e}", file.name);
                    let notice = auto_push_failed_message(language, &file.name, &e.to_string());
                    let _ = self.send_message(chat_id, &notice).await;
                }
            }
        }
    }
}
