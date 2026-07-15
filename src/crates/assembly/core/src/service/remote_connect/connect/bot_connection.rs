//! Bot connection lifecycle: Feishu, Telegram, Weixin.
//!
//! Carries the `start_bot_connection`, `restore_bot`, and `stop_bots` methods
//! of `RemoteConnectService`. Bot connections run independently of relay
//! state: stopping the relay (`stop_relay`) does not affect them, and
//! `stop_bots` does not affect the relay.

use std::sync::Arc;
use tracing::info;

use super::*;

impl super::RemoteConnectService {
    pub(crate) async fn start_bot_connection(&self, method: &ConnectionMethod) -> Result<ConnectionResult> {
        let pairing_code = PairingProtocol::generate_bot_pairing_code();

        let bot_link = match method {
            ConnectionMethod::BotTelegram => {
                match &self.config.bot_telegram {
                    Some(bot::BotConfig::Telegram { bot_token }) if !bot_token.is_empty() => {
                        // Stop any existing Telegram bot
                        if let Some(handle) = self.bot_telegram_handle.write().await.take() {
                            handle.stop();
                        }

                        let tg_bot = Arc::new(bot::telegram::TelegramBot::new(bot::telegram::TelegramConfig {
                            bot_token: bot_token.clone(),
                        }));
                        tg_bot.register_pairing(&pairing_code).await?;

                        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);

                        let bot_connected_info = self.bot_connected_info.clone();
                        let bot_for_pair = tg_bot.clone();
                        let bot_for_loop = tg_bot.clone();
                        let tg_bot_ref = self.telegram_bot.clone();

                        *tg_bot_ref.write().await = Some(tg_bot.clone());

                        tokio::spawn(async move {
                            let mut stop_rx = stop_rx;
                            match bot_for_pair.wait_for_pairing(&mut stop_rx).await {
                                Ok(chat_id) => {
                                    // Guard against the race where stop_bots() cleared
                                    // bot_connected_info between pairing completing and
                                    // this task running.
                                    if !*stop_rx.borrow() {
                                        *bot_connected_info.write().await = Some(format!("Telegram({chat_id})"));
                                        info!("Telegram bot paired, starting message loop");
                                        bot_for_loop.run_message_loop(stop_rx).await;
                                    } else {
                                        info!("Telegram pairing completed but bot was stopped; discarding");
                                    }
                                }
                                Err(e) => {
                                    info!("Telegram pairing ended: {e}");
                                }
                            }
                        });

                        *self.bot_telegram_handle.write().await = Some(BotHandle { stop_tx });

                        "https://t.me/BotFather".to_string()
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Telegram bot token not configured. Please set bot token first."
                        ));
                    }
                }
            }
            ConnectionMethod::BotFeishu => {
                match &self.config.bot_feishu {
                    Some(bot::BotConfig::Feishu { app_id, app_secret })
                        if !app_id.is_empty() && !app_secret.is_empty() =>
                    {
                        if let Some(handle) = self.bot_feishu_handle.write().await.take() {
                            handle.stop();
                        }

                        let fs_bot = Arc::new(bot::feishu::FeishuBot::new(bot::feishu::FeishuConfig {
                            app_id: app_id.clone(),
                            app_secret: app_secret.clone(),
                        }));
                        fs_bot.register_pairing(&pairing_code).await?;

                        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);

                        let bot_connected_info = self.bot_connected_info.clone();
                        let bot_for_pair = fs_bot.clone();
                        let bot_for_loop = fs_bot.clone();
                        let fs_bot_ref = self.feishu_bot.clone();

                        *fs_bot_ref.write().await = Some(fs_bot.clone());

                        tokio::spawn(async move {
                            let mut stop_rx = stop_rx;
                            match bot_for_pair.wait_for_pairing(&mut stop_rx).await {
                                Ok(chat_id) => {
                                    // Guard against the race where stop_bots() cleared
                                    // bot_connected_info between pairing completing and
                                    // this task running.
                                    if !*stop_rx.borrow() {
                                        *bot_connected_info.write().await = Some(format!("Feishu({chat_id})"));
                                        info!("Feishu bot paired, starting message loop");
                                        bot_for_loop.run_message_loop(stop_rx).await;
                                    } else {
                                        info!("Feishu pairing completed but bot was stopped; discarding");
                                    }
                                }
                                Err(e) => {
                                    info!("Feishu pairing ended: {e}");
                                }
                            }
                        });

                        *self.bot_feishu_handle.write().await = Some(BotHandle { stop_tx });

                        "https://open.feishu.cn/app".to_string()
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Feishu bot credentials not configured. \
                             Please set App ID and App Secret first."
                        ));
                    }
                }
            }
            ConnectionMethod::BotWeixin => match &self.config.bot_weixin {
                Some(bot::BotConfig::Weixin {
                    ilink_token,
                    base_url,
                    bot_account_id,
                }) if !ilink_token.is_empty() && !bot_account_id.is_empty() => {
                    if let Some(handle) = self.bot_weixin_handle.write().await.take() {
                        handle.stop();
                    }

                    let wx_cfg = bot::weixin::WeixinConfig {
                        ilink_token: ilink_token.clone(),
                        base_url: if base_url.trim().is_empty() {
                            "https://ilinkai.weixin.qq.com".to_string()
                        } else {
                            base_url.clone()
                        },
                        bot_account_id: bot_account_id.clone(),
                    };

                    let wx_bot = Arc::new(bot::weixin::WeixinBot::new(wx_cfg));
                    wx_bot.register_pairing(&pairing_code).await?;

                    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);

                    let bot_connected_info = self.bot_connected_info.clone();
                    let bot_for_pair = wx_bot.clone();
                    let bot_for_loop = wx_bot.clone();
                    let wx_bot_ref = self.weixin_bot.clone();

                    *wx_bot_ref.write().await = Some(wx_bot.clone());

                    tokio::spawn(async move {
                        let mut stop_rx = stop_rx;
                        match bot_for_pair.wait_for_pairing(&mut stop_rx).await {
                            Ok(peer_id) => {
                                if !*stop_rx.borrow() {
                                    *bot_connected_info.write().await = Some(format!("Weixin({peer_id})"));
                                    info!("Weixin bot paired, starting message loop");
                                    bot_for_loop.run_message_loop(stop_rx).await;
                                } else {
                                    info!("Weixin pairing completed but bot was stopped; discarding");
                                }
                            }
                            Err(e) => {
                                info!("Weixin pairing ended: {e}");
                            }
                        }
                    });

                    *self.bot_weixin_handle.write().await = Some(BotHandle { stop_tx });

                    "https://www.wechat.com".to_string()
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Weixin not linked. Complete WeChat QR login in Remote Connect first."
                    ));
                }
            },
            _ => {
                return Err(anyhow::anyhow!("start_bot_connection: unsupported method {method:?}"));
            }
        };

        Ok(ConnectionResult {
            method: method.clone(),
            qr_data: None,
            qr_svg: None,
            qr_url: None,
            bot_pairing_code: Some(pairing_code),
            bot_link: Some(bot_link),
            pairing_state: PairingState::WaitingForScan,
        })
    }

    /// Restore a previously paired bot from persistence.
    /// Skips the pairing step and directly starts the message loop.
    pub async fn restore_bot(&self, saved: &bot::SavedBotConnection) -> Result<()> {
        match saved.config {
            bot::BotConfig::Telegram { ref bot_token } => {
                if let Some(handle) = self.bot_telegram_handle.write().await.take() {
                    handle.stop();
                }

                let tg_bot = Arc::new(bot::telegram::TelegramBot::new(bot::telegram::TelegramConfig {
                    bot_token: bot_token.clone(),
                }));

                let chat_id: i64 = saved
                    .chat_id
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid saved telegram chat_id: {}", saved.chat_id))?;
                tg_bot.restore_chat_state(chat_id, saved.chat_state.clone()).await;

                let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
                *self.telegram_bot.write().await = Some(tg_bot.clone());
                *self.bot_connected_info.write().await = Some(format!("Telegram({chat_id})"));

                let bot_for_loop = tg_bot.clone();
                tokio::spawn(async move {
                    info!("Telegram bot restored from persistence, starting message loop");
                    bot_for_loop.run_message_loop(stop_rx).await;
                });

                *self.bot_telegram_handle.write().await = Some(BotHandle { stop_tx });
                info!("Telegram bot restored for chat_id={chat_id}");
            }
            bot::BotConfig::Feishu {
                ref app_id,
                ref app_secret,
            } => {
                if let Some(handle) = self.bot_feishu_handle.write().await.take() {
                    handle.stop();
                }

                let fs_bot = Arc::new(bot::feishu::FeishuBot::new(bot::feishu::FeishuConfig {
                    app_id: app_id.clone(),
                    app_secret: app_secret.clone(),
                }));

                fs_bot
                    .restore_chat_state(&saved.chat_id, saved.chat_state.clone())
                    .await;

                let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
                *self.feishu_bot.write().await = Some(fs_bot.clone());

                let cid = saved.chat_id.clone();
                *self.bot_connected_info.write().await = Some(format!("Feishu({cid})"));

                let bot_for_loop = fs_bot.clone();
                tokio::spawn(async move {
                    info!("Feishu bot restored from persistence, starting message loop");
                    bot_for_loop.run_message_loop(stop_rx).await;
                });

                *self.bot_feishu_handle.write().await = Some(BotHandle { stop_tx });
                info!("Feishu bot restored for chat_id={}", saved.chat_id);
            }
            bot::BotConfig::Weixin {
                ref ilink_token,
                ref base_url,
                ref bot_account_id,
            } => {
                if let Some(handle) = self.bot_weixin_handle.write().await.take() {
                    handle.stop();
                }

                let wx_cfg = bot::weixin::WeixinConfig {
                    ilink_token: ilink_token.clone(),
                    base_url: if base_url.trim().is_empty() {
                        "https://ilinkai.weixin.qq.com".to_string()
                    } else {
                        base_url.clone()
                    },
                    bot_account_id: bot_account_id.clone(),
                };

                let wx_bot = Arc::new(bot::weixin::WeixinBot::new(wx_cfg));
                wx_bot
                    .restore_chat_state(&saved.chat_id, saved.chat_state.clone())
                    .await;

                let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
                *self.weixin_bot.write().await = Some(wx_bot.clone());

                let cid = saved.chat_id.clone();
                *self.bot_connected_info.write().await = Some(format!("Weixin({cid})"));

                let bot_for_loop = wx_bot.clone();
                tokio::spawn(async move {
                    info!("Weixin bot restored from persistence, starting message loop");
                    bot_for_loop.run_message_loop(stop_rx).await;
                });

                *self.bot_weixin_handle.write().await = Some(BotHandle { stop_tx });
                info!("Weixin bot restored for chat_id={}", saved.chat_id);
            }
        }
        Ok(())
    }

    /// Stop all bot connections.
    pub async fn stop_bots(&self) {
        if let Some(handle) = self.bot_telegram_handle.write().await.take() {
            handle.stop();
        }
        *self.telegram_bot.write().await = None;

        if let Some(handle) = self.bot_feishu_handle.write().await.take() {
            handle.stop();
        }
        *self.feishu_bot.write().await = None;

        if let Some(handle) = self.bot_weixin_handle.write().await.take() {
            handle.stop();
        }
        *self.weixin_bot.write().await = None;
        *self.bot_connected_info.write().await = None;

        info!("Bot connections stopped");
    }
}
