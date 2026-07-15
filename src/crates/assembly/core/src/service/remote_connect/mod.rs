//! Remote Connect service module.
//!
//! Provides phone-to-desktop remote connection capabilities with E2E encryption.
//! Supports multiple connection methods: LAN, ngrok, relay server, and bots.
//!
//! Bot connections (Telegram / Feishu / Weixin) run independently of relay connections
//! (LAN / ngrok / northhing Server / Custom Server).  Calling `stop()` only
//! tears down the relay side; bots keep running.  Use `stop_bot()` or
//! `stop_all()` to shut everything down.

pub mod bot;
pub mod embedded_relay;
pub mod lan;
pub mod ngrok;
pub mod remote_server;

pub mod command;
pub mod connect;
pub mod session;
pub mod sync;

pub mod device {
    pub use northhing_services_integrations::remote_connect::device::*;
}

pub mod encryption {
    pub use northhing_services_integrations::remote_connect::encryption::*;
}

pub mod pairing {
    pub use northhing_services_integrations::remote_connect::pairing::*;
}

pub mod qr_generator {
    pub use northhing_services_integrations::remote_connect::qr_generator::*;
}

pub mod relay_client {
    pub use northhing_services_integrations::remote_connect::relay_client::*;
}

pub use connect::RemoteConnectService;
pub use device::DeviceIdentity;
pub use encryption::{decrypt_from_base64, encrypt_to_base64, KeyPair};
pub use pairing::{PairingProtocol, PairingState};
pub use qr_generator::QrGenerator;
pub use relay_client::ensure_rustls_crypto_provider;
pub use relay_client::RelayClient;
pub use remote_server::RemoteServer;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// Supported connection methods.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionMethod {
    Lan,
    Ngrok,
    NortHingServer,
    CustomServer { url: String },
    BotFeishu,
    BotTelegram,
    BotWeixin,
}

/// Configuration for Remote Connect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConnectConfig {
    pub lan_port: u16,
    pub northhing_server_url: String,
    pub web_app_url: String,
    pub custom_server_url: Option<String>,
    pub bot_feishu: Option<bot::BotConfig>,
    pub bot_telegram: Option<bot::BotConfig>,
    pub bot_weixin: Option<bot::BotConfig>,
    pub mobile_web_dir: Option<String>,
}

impl Default for RemoteConnectConfig {
    fn default() -> Self {
        Self {
            lan_port: 9700,
            northhing_server_url: "https://remote.openagentapp.com/relay".to_string(),
            web_app_url: "https://remote.openagentapp.com/relay".to_string(),
            custom_server_url: None,
            bot_feishu: None,
            bot_telegram: None,
            bot_weixin: None,
            mobile_web_dir: None,
        }
    }
}

/// Result of starting a remote connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionResult {
    pub method: ConnectionMethod,
    pub qr_data: Option<String>,
    pub qr_svg: Option<String>,
    pub qr_url: Option<String>,
    pub bot_pairing_code: Option<String>,
    pub bot_link: Option<String>,
    pub pairing_state: PairingState,
}
