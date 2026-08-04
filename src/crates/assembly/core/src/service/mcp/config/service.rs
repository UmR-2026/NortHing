use async_trait::async_trait;
use std::sync::Arc;

use crate::service::config::ConfigService;
use crate::service::mcp::server::MCPServerConfig;
use crate::util::errors::{NortHingError, NortHingResult};

pub struct MCPConfigService {
    pub(super) config_service: Arc<ConfigService>,
    inner: northhing_services_integrations::mcp::config::MCPConfigService,
}

struct CoreMCPConfigStore {
    config_service: Arc<ConfigService>,
}

/// Classifies a `ConfigService` read for the MCP config store.
///
/// A missing key (`NortHingError::NotFound`) is the legitimate empty state and
/// maps to `Ok(None)` so read-modify-write callers can start from an empty
/// baseline. Any other failure (IO, parse, serialization) must surface as an
/// error: treating it as an empty config would let callers clobber the existing
/// value on the subsequent write.
fn classify_config_read(
    key: &str,
    result: NortHingResult<serde_json::Value>,
) -> northhing_services_integrations::mcp::MCPRuntimeResult<Option<serde_json::Value>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(NortHingError::NotFound(_)) => Ok(None),
        Err(e) => Err(northhing_services_integrations::mcp::MCPRuntimeError::configuration(
            format!("Failed to read MCP config store key '{}': {}", key, e),
        )),
    }
}

#[async_trait]
impl northhing_services_integrations::mcp::config::MCPConfigStore for CoreMCPConfigStore {
    async fn get_config_value(
        &self,
        key: &str,
    ) -> northhing_services_integrations::mcp::MCPRuntimeResult<Option<serde_json::Value>> {
        classify_config_read(key, self.config_service.config::<serde_json::Value>(Some(key)).await)
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

    #[test]
    fn classify_config_read_maps_missing_key_to_none_and_real_failures_to_error() {
        let present = classify_config_read("mcp_servers", Ok(serde_json::json!({ "mcpServers": {} })));
        assert_eq!(present.unwrap(), Some(serde_json::json!({ "mcpServers": {} })));

        let missing = classify_config_read(
            "mcp_servers",
            Err(NortHingError::NotFound(
                "Config path 'mcp_servers' not found".to_string(),
            )),
        );
        assert_eq!(missing.unwrap(), None);

        let failed = classify_config_read(
            "mcp_servers",
            Err(NortHingError::config("Failed to read config file: disk error")),
        );
        let error = failed.expect_err("real read failures must not be treated as an empty config");
        assert_eq!(
            error.kind(),
            northhing_services_integrations::mcp::MCPRuntimeErrorKind::Configuration
        );
        assert!(
            error.message().contains("mcp_servers"),
            "error should identify the failing key, got: {}",
            error.message()
        );
    }

    #[tokio::test]
    async fn core_mcp_config_store_returns_none_for_missing_key_on_real_config_service() {
        use northhing_services_integrations::mcp::config::MCPConfigStore;

        let temp_root = std::env::temp_dir().join(format!("northhing-core-mcp-config-store-{}", uuid::Uuid::new_v4()));
        let path_manager = Arc::new(crate::infrastructure::PathManager::with_user_root_for_tests(
            temp_root.join("user-root"),
        ));
        let settings = crate::service::config::ConfigManagerSettings {
            path_manager: Some(path_manager),
            auto_save: true,
            backup_count: 5,
        };
        let config_service = Arc::new(
            crate::service::config::ConfigService::with_settings(settings)
                .await
                .expect("config service builds against an isolated temp root"),
        );

        let store = CoreMCPConfigStore { config_service };
        let value = store
            .get_config_value("mcp_servers")
            .await
            .expect("a key that was never written is the legitimate empty state");
        assert_eq!(value, None);
    }
}
