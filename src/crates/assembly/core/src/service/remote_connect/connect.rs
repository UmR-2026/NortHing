//! Sub-domain: SSH/remote connection lifecycle.
//!
//! Spec step-3.7 — extracted from remote_connect/mod.rs (R55e refactor).
//!
//! # R73-5 layout (Mavis M3 take-over)
//!
//! The original single 741-line file carried a single `impl RemoteConnectService`
//! block that mixed 4 concerns: ctor / device identity, mobile identity helpers,
//! relay connection lifecycle, and bot connection lifecycle. R73-5 splits
//! these into:
//!
//! - `connect.rs` (this file) — entry facade: struct, `new`, `device_identity`,
//!   `update_bot_config`, `available_methods`, `stop`, `stop_all`, plus the
//!   `BotHandle` and `TrustedMobileIdentity` helper types.
//! - `connect/mobile_identity.rs` — `validate_mobile_identity` +
//!   `persist_mobile_identity` (static-style helpers used by the pairing flow).
//! - `connect/relay_connection.rs` — `start` (relay methods: LAN / ngrok /
//!   northhing Server / Custom Server) + `stop_relay`.
//! - `connect/bot_connection.rs` — `start_bot_connection` (Feishu / Telegram /
//!   Weixin) + `restore_bot` + `stop_bots`.
//!
//! All three siblings use the inherent-impl pattern
//! `impl super::RemoteConnectService { ... }`. Unlike `impl Trait for Type`,
//! inherent impls can be split across multiple files in the same crate
//! without triggering E0119. The 4 entry + 3 siblings preserve the public
//! API byte-for-byte; `service/remote_connect/mod.rs` continues to re-export
//! `pub mod connect;` and consumers see no change.

mod bot_connection;
mod mobile_identity;
mod relay_connection;

use super::*;
use std::sync::Arc;
use tokio::sync::RwLock;

impl RemoteConnectService {
    pub fn new(config: RemoteConnectConfig) -> Result<Self> {
        let device_identity = DeviceIdentity::from_current_machine()?;
        let pairing = PairingProtocol::new(device_identity.clone());

        Ok(Self {
            config,
            device_identity,
            pairing: Arc::new(RwLock::new(pairing)),
            relay_client: Arc::new(RwLock::new(None)),
            remote_server: Arc::new(RwLock::new(None)),
            active_method: Arc::new(RwLock::new(None)),
            ngrok_tunnel: Arc::new(RwLock::new(None)),
            embedded_relay: Arc::new(RwLock::new(None)),
            bot_telegram_handle: Arc::new(RwLock::new(None)),
            bot_feishu_handle: Arc::new(RwLock::new(None)),
            bot_weixin_handle: Arc::new(RwLock::new(None)),
            telegram_bot: Arc::new(RwLock::new(None)),
            feishu_bot: Arc::new(RwLock::new(None)),
            weixin_bot: Arc::new(RwLock::new(None)),
            bot_connected_info: Arc::new(RwLock::new(None)),
            trusted_mobile_identity: Arc::new(RwLock::new(None)),
        })
    }

    pub fn device_identity(&self) -> &DeviceIdentity {
        &self.device_identity
    }

    pub fn update_bot_config(&mut self, bot_config: bot::BotConfig) {
        match bot_config {
            bot::BotConfig::Feishu { app_id, app_secret } => {
                self.config.bot_feishu = Some(bot::BotConfig::Feishu { app_id, app_secret });
            }
            bot::BotConfig::Telegram { bot_token } => {
                self.config.bot_telegram = Some(bot::BotConfig::Telegram { bot_token });
            }
            bot::BotConfig::Weixin {
                ilink_token,
                base_url,
                bot_account_id,
            } => {
                self.config.bot_weixin = Some(bot::BotConfig::Weixin {
                    ilink_token,
                    base_url,
                    bot_account_id,
                });
            }
        }
    }

    pub async fn available_methods(&self) -> Vec<ConnectionMethod> {
        vec![
            ConnectionMethod::Lan,
            ConnectionMethod::Ngrok,
            ConnectionMethod::NortHingServer,
            ConnectionMethod::CustomServer {
                url: self.config.custom_server_url.clone().unwrap_or_default(),
            },
            ConnectionMethod::BotFeishu,
            ConnectionMethod::BotTelegram,
            ConnectionMethod::BotWeixin,
        ]
    }

    /// Legacy `stop()` — only stops relay for backward compatibility.
    /// Bot connections persist independently.
    pub async fn stop(&self) {
        self.stop_relay().await;
    }

    /// Stop everything (relay + bots).
    pub async fn stop_all(&self) {
        self.stop_relay().await;
        self.stop_bots().await;
    }
}

/// Handle to a running bot (Telegram, Feishu, or Weixin).
pub(crate) struct BotHandle {
    stop_tx: tokio::sync::watch::Sender<bool>,
}

impl BotHandle {
    fn stop(&self) {
        let _ = self.stop_tx.send(true);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TrustedMobileIdentity {
    pub(crate) mobile_install_id: String,
    pub(crate) user_id: String,
}

/// Unified Remote Connect service that orchestrates all connection methods.
pub struct RemoteConnectService {
    pub(crate) config: RemoteConnectConfig,
    pub(crate) device_identity: DeviceIdentity,
    pub(crate) pairing: Arc<RwLock<PairingProtocol>>,
    pub(crate) relay_client: Arc<RwLock<Option<RelayClient>>>,
    pub(crate) remote_server: Arc<RwLock<Option<RemoteServer>>>,
    pub(crate) active_method: Arc<RwLock<Option<ConnectionMethod>>>,
    pub(crate) ngrok_tunnel: Arc<RwLock<Option<ngrok::NgrokTunnel>>>,
    pub(crate) embedded_relay: Arc<RwLock<Option<embedded_relay::EmbeddedRelayHandle>>>,
    // Bot handles live independently of relay connections
    pub(crate) bot_telegram_handle: Arc<RwLock<Option<BotHandle>>>,
    pub(crate) bot_feishu_handle: Arc<RwLock<Option<BotHandle>>>,
    pub(crate) bot_weixin_handle: Arc<RwLock<Option<BotHandle>>>,
    // Keep Arc references to bots for send_message etc.
    pub(crate) telegram_bot: Arc<RwLock<Option<Arc<bot::telegram::TelegramBot>>>>,
    pub(crate) feishu_bot: Arc<RwLock<Option<Arc<bot::feishu::FeishuBot>>>>,
    pub(crate) weixin_bot: Arc<RwLock<Option<Arc<bot::weixin::WeixinBot>>>>,
    /// Independent bot connection state — not tied to PairingProtocol.
    /// Stores the peer description (e.g. "Telegram(7096812005)") when a bot is active.
    pub(crate) bot_connected_info: Arc<RwLock<Option<String>>>,
    /// Trusted mobile identity for the current relay lifecycle only.
    pub(crate) trusted_mobile_identity: Arc<RwLock<Option<TrustedMobileIdentity>>>,
}
