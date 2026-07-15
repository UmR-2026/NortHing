use super::FeishuBot;
use crate::service::remote_connect::bot::command_router::{
    current_bot_language, BotAction, BotActionStyle, BotLanguage, HandleResult,
};

use anyhow::{anyhow, Result};
use serde_json::json;
use tracing::{debug, warn};

// =====================================================================
// Language helpers
// =====================================================================

impl FeishuBot {
    pub(super) fn invalid_pairing_code_message(language: BotLanguage) -> &'static str {
        if language.is_chinese() {
            "配对码无效或已过期，请重试。"
        } else {
            "Invalid or expired pairing code. Please try again."
        }
    }

    pub(super) fn enter_pairing_code_message(language: BotLanguage) -> &'static str {
        if language.is_chinese() {
            "请输入 northhing Desktop 中显示的 6 位配对码。"
        } else {
            "Please enter the 6-digit pairing code from northhing Desktop."
        }
    }

    pub(super) fn unsupported_message_type_message(language: BotLanguage) -> &'static str {
        if language.is_chinese() {
            "暂不支持这种消息类型，请发送文本或图片。"
        } else {
            "This message type is not supported. Please send text or images."
        }
    }
}

// =====================================================================
// Card / text formatting and send
// =====================================================================

impl FeishuBot {
    pub(super) fn build_markdown_card(content: &str) -> serde_json::Value {
        json!({
            "schema": "2.0",
            "config": {
                "wide_screen_mode": true,
            },
            "body": {
                "elements": [
                    {
                        "tag": "markdown",
                        "content": content,
                        "text_align": "left",
                        "text_size": "normal",
                        "margin": "0px 0px 0px 0px",
                        "element_id": "northhing_remote_reply_markdown",
                    }
                ],
            },
        })
    }

    pub async fn send_message(&self, chat_id: &str, content: &str) -> Result<()> {
        let token = self.get_access_token().await?;
        let card = Self::build_markdown_card(content);
        let client = reqwest::Client::new();
        let resp = client
            .post("https://open.feishu.cn/open-apis/im/v1/messages")
            .query(&[("receive_id_type", "chat_id")])
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "receive_id": chat_id,
                "msg_type": "interactive",
                "content": serde_json::to_string(&card)?,
            }))
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(anyhow!("feishu send_message HTTP {status}: {body}"));
        }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(code) = parsed.get("code").and_then(|c| c.as_i64()) {
                if code != 0 {
                    let msg = parsed.get("msg").and_then(|m| m.as_str()).unwrap_or("unknown");
                    warn!("Feishu send_message API error: code={code}, msg={msg}");
                    return Err(anyhow!("feishu send_message API error: code={code}, msg={msg}"));
                }
            }
        }
        debug!("Feishu message sent to {chat_id}");
        Ok(())
    }

    pub async fn send_action_card(
        &self,
        chat_id: &str,
        language: BotLanguage,
        content: &str,
        actions: &[BotAction],
    ) -> Result<()> {
        let token = self.get_access_token().await?;
        let client = reqwest::Client::new();
        let card = Self::build_action_card(chat_id, language, content, actions);
        let resp = client
            .post("https://open.feishu.cn/open-apis/im/v1/messages")
            .query(&[("receive_id_type", "chat_id")])
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "receive_id": chat_id,
                "msg_type": "interactive",
                "content": serde_json::to_string(&card)?,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("feishu send_action_card failed: {body}"));
        }
        debug!("Feishu action card sent to {chat_id}");
        Ok(())
    }

    pub(super) async fn send_handle_result(&self, chat_id: &str, result: &HandleResult) -> Result<()> {
        let language = current_bot_language().await;
        let text = if result.menu.items.is_empty() && result.menu.title.is_empty() {
            result.reply.clone()
        } else {
            result.menu.render_text_block()
        };
        // Empty replies (e.g. the silent "forward only" result returned by
        // `handle_chat`) must not be sent — they would surface as a blank
        // message in the user's Feishu chat.
        if text.trim().is_empty() {
            return Ok(());
        }
        if result.actions.is_empty() {
            self.send_message(chat_id, &text).await
        } else {
            self.send_action_card(chat_id, language, &text, &result.actions).await
        }
    }

    pub(super) fn build_action_card(
        chat_id: &str,
        language: BotLanguage,
        content: &str,
        actions: &[BotAction],
    ) -> serde_json::Value {
        let body = Self::card_body_text(language, content);
        let mut elements = vec![json!({
            "tag": "markdown",
            "content": body,
        })];

        for chunk in actions.chunks(2) {
            let buttons: Vec<_> = chunk
                .iter()
                .map(|action| {
                    let button_type = match action.style {
                        BotActionStyle::Primary => "primary",
                        BotActionStyle::Default => "default",
                    };
                    json!({
                        "tag": "button",
                        "text": {
                            "tag": "plain_text",
                            "content": action.label,
                        },
                        "type": button_type,
                        "value": {
                            "chat_id": chat_id,
                            "command": action.command,
                        }
                    })
                })
                .collect();
            elements.push(json!({
                "tag": "action",
                "actions": buttons,
            }));
        }

        json!({
            "config": {
                "wide_screen_mode": true,
            },
            "header": {
                "title": {
                    "tag": "plain_text",
                    "content": "northhing Remote Connect",
                }
            },
            "elements": elements,
        })
    }

    pub(super) fn card_body_text(language: BotLanguage, content: &str) -> String {
        let mut removed_command_lines = false;
        let mut lines = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('/') && trimmed.contains(" - ") {
                removed_command_lines = true;
                continue;
            }
            if trimmed.contains("/cancel_task ") {
                lines.push(if language.is_chinese() {
                    "如需停止本次请求，请使用下方的\"取消任务\"按钮。".to_string()
                } else {
                    "If needed, use the Cancel Task button below to stop this request.".to_string()
                });
                continue;
            }
            lines.push(Self::replace_command_tokens(language, line));
        }

        let mut body = lines.join("\n").trim().to_string();
        if removed_command_lines {
            if !body.is_empty() {
                body.push_str("\n\n");
            }
            body.push_str(if language.is_chinese() {
                "请选择下方操作。"
            } else {
                "Choose an action below."
            });
        }

        if body.is_empty() {
            if language.is_chinese() {
                "请选择下方操作。".to_string()
            } else {
                "Choose an action below.".to_string()
            }
        } else {
            body
        }
    }

    pub(super) fn replace_command_tokens(language: BotLanguage, line: &str) -> String {
        let replacements = [
            (
                "/switch_workspace",
                if language.is_chinese() {
                    "切换工作区"
                } else {
                    "Switch Workspace"
                },
            ),
            (
                "/pro",
                if language.is_chinese() {
                    "专业模式"
                } else {
                    "Expert Mode"
                },
            ),
            (
                "/assistant",
                if language.is_chinese() {
                    "助理模式"
                } else {
                    "Assistant Mode"
                },
            ),
            (
                "/resume_session",
                if language.is_chinese() {
                    "恢复会话"
                } else {
                    "Resume Session"
                },
            ),
            (
                "/new_code_session",
                if language.is_chinese() {
                    "新建编码会话"
                } else {
                    "New Code Session"
                },
            ),
            (
                "/new_cowork_session",
                if language.is_chinese() {
                    "新建协作会话"
                } else {
                    "New Cowork Session"
                },
            ),
            (
                "/new_claw_session",
                if language.is_chinese() {
                    "新建助理会话"
                } else {
                    "New Claw Session"
                },
            ),
            (
                "/cancel_task",
                if language.is_chinese() {
                    "取消任务"
                } else {
                    "Cancel Task"
                },
            ),
            ("/help", if language.is_chinese() { "帮助" } else { "Help" }),
        ];

        replacements
            .iter()
            .fold(line.to_string(), |acc, (from, to)| acc.replace(from, to))
    }
}
