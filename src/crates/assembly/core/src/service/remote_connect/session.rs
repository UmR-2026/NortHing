//! Sub-domain: session + workspace projection state queries.
//! Spec step-3.7 — extracted from remote_connect/mod.rs (R55e refactor).

use super::connect::RemoteConnectService;
use super::*;

impl RemoteConnectService {
    pub async fn pairing_state(&self) -> PairingState {
        self.pairing.read().await.state().await
    }

    pub async fn is_connected(&self) -> bool {
        self.pairing.read().await.state().await == PairingState::Connected
    }

    pub async fn active_method(&self) -> Option<ConnectionMethod> {
        self.active_method.read().await.clone()
    }

    pub async fn peer_device_name(&self) -> Option<String> {
        self.pairing.read().await.peer_device_name().map(String::from)
    }

    /// Check whether a specific bot type is currently running.
    pub async fn is_bot_running(&self, bot_type: &str) -> bool {
        match bot_type {
            "telegram" => self.bot_telegram_handle.read().await.is_some(),
            "feishu" => self.bot_feishu_handle.read().await.is_some(),
            "weixin" => self.bot_weixin_handle.read().await.is_some(),
            _ => false,
        }
    }

    pub async fn bot_connected_info(&self) -> Option<String> {
        self.bot_connected_info.read().await.clone()
    }

    pub async fn trusted_mobile_user_id(&self) -> Option<String> {
        self.trusted_mobile_identity
            .read()
            .await
            .as_ref()
            .map(|identity| identity.user_id.clone())
    }
}
