use async_trait::async_trait;
use std::sync::Arc;

use crate::service::config::ConfigService;
use crate::service::mcp::server::MCPServerConfig;
use crate::util::errors::NortHingResult;

pub struct MCPConfigService {
    pub(super) config_service: Arc<ConfigService>,
    inner: northhing_services_integrations::mcp::config::MCPConfigService,
}

struct CoreMCPConfigStore {
    config_service: Arc<ConfigService>,
}

#[async_trait]
impl northhing_services_integrations::mcp::config::MCPConfigStore for CoreMCPConfigStore {
    async fn get_config_value(
        &self,
        key: &str,
    ) -> northhing_services_integrations::mcp::MCPRuntimeResult<Option<serde_json::Value>> {
        match self.config_service.config::<serde_json::Value>(Some(key)).await {
            Ok(value) => Ok(Some(value)),
            Err(_) => Ok(None),
        }
    }

    async fn set_config_value(
        &self,
        key: &str,
        value: serde_json::Value,
    ) -> northhing_services_integrations::mcp::MCPRuntimeResult<()> {
        self.config_service
            .set_config(key, value)
            .await
            .map_err(|e| northhing_services_integrations::mcp::MCPRuntimeError::configuration(e.to_string()))
    }
}

impl MCPConfigService {
    pub fn get_remote_authorization_value(config: &MCPServerConfig) -> Option<String> {
        northhing_services_integrations::mcp::config::MCPConfigService::get_remote_authorization_value(config)
    }

    pub fn get_remote_authorization_source(config: &MCPServerConfig) -> Option<&'static str> {
        northhing_services_integrations::mcp::config::MCPConfigService::get_remote_authorization_source(config)
    }

    pub fn has_remote_authorization(config: &MCPServerConfig) -> bool {
        northhing_services_integrations::mcp::config::MCPConfigService::has_remote_authorization(config)
    }

    pub fn has_remote_oauth(config: &MCPServerConfig) -> bool {
        northhing_services_integrations::mcp::config::MCPConfigService::has_remote_oauth(config)
    }

    pub fn has_remote_xaa(config: &MCPServerConfig) -> bool {
        northhing_services_integrations::mcp::config::MCPConfigService::has_remote_xaa(config)
    }

    pub fn new(config_service: Arc<ConfigService>) -> NortHingResult<Self> {
        let store = Arc::new(CoreMCPConfigStore {
            config_service: config_service.clone(),
        });
        Ok(Self {
            config_service,
            inner: northhing_services_integrations::mcp::config::MCPConfigService::new(store),
        })
    }

    pub async fn load_all_configs(&self) -> NortHingResult<Vec<MCPServerConfig>> {
        Ok(self.inner.load_all_configs().await?)
    }

    pub async fn get_server_config(&self, server_id: &str) -> NortHingResult<Option<MCPServerConfig>> {
        Ok(self.inner.get_server_config(server_id).await?)
    }

    pub async fn save_server_config(&self, config: &MCPServerConfig) -> NortHingResult<()> {
        Ok(self.inner.save_server_config(config).await?)
    }

    pub async fn set_remote_authorization(
        &self,
        server_id: &str,
        authorization_value: &str,
    ) -> NortHingResult<MCPServerConfig> {
        Ok(self
            .inner
            .set_remote_authorization(server_id, authorization_value)
            .await?)
    }

    pub async fn clear_remote_authorization(&self, server_id: &str) -> NortHingResult<MCPServerConfig> {
        Ok(self.inner.clear_remote_authorization(server_id).await?)
    }

    pub async fn delete_server_config(&self, server_id: &str) -> NortHingResult<()> {
        Ok(self.inner.delete_server_config(server_id).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::mcp::config::ConfigLocation;
    use crate::service::mcp::server::MCPServerType;
    use std::collections::HashMap;

    fn make_config(
        id: &str,
        location: ConfigLocation,
        server_type: MCPServerType,
        command: Option<&str>,
        url: Option<&str>,
    ) -> MCPServerConfig {
        MCPServerConfig {
            id: id.to_string(),
            name: id.to_string(),
            server_type,
            transport: None,
            command: command.map(str::to_string),
            args: Vec::new(),
            env: HashMap::new(),
            headers: HashMap::new(),
            url: url.map(str::to_string),
            auto_start: true,
            enabled: true,
            location,
            capabilities: Vec::new(),
            settings: Default::default(),
            oauth: None,
            xaa: None,
        }
    }

    #[test]
    fn remote_authorization_prefers_headers_and_normalizes_tokens() {
        let mut config = make_config(
            "remote-auth",
            ConfigLocation::User,
            MCPServerType::Remote,
            None,
            Some("https://example.com/mcp"),
        );
        config
            .env
            .insert("Authorization".to_string(), "legacy-token".to_string());
        config
            .headers
            .insert("Authorization".to_string(), "Bearer header-token".to_string());

        assert_eq!(
            MCPConfigService::get_remote_authorization_value(&config).as_deref(),
            Some("Bearer header-token")
        );
        assert_eq!(
            MCPConfigService::get_remote_authorization_source(&config),
            Some("headers")
        );
        assert_eq!(
            northhing_services_integrations::mcp::config::normalize_mcp_authorization_value("plain-token").as_deref(),
            Some("Bearer plain-token")
        );
    }
}
