use northhing_services_integrations::mcp::server::{
    compute_mcp_backoff_delay, detect_mcp_list_changed_kind, MCPListChangedKind,
};
use std::time::Duration;

#[test]
fn backoff_delay_grows_exponentially_and_caps() {
    let base = Duration::from_secs(2);
    let max = Duration::from_secs(60);

    assert_eq!(compute_mcp_backoff_delay(base, max, 1), Duration::from_secs(2));
    assert_eq!(compute_mcp_backoff_delay(base, max, 2), Duration::from_secs(4));
    assert_eq!(compute_mcp_backoff_delay(base, max, 5), Duration::from_secs(32));
    assert_eq!(compute_mcp_backoff_delay(base, max, 10), Duration::from_secs(60));
}

#[test]
fn detect_list_changed_kind_supports_three_catalogs() {
    assert_eq!(
        detect_mcp_list_changed_kind("notifications/tools/list_changed"),
        Some(MCPListChangedKind::Tools)
    );
    assert_eq!(
        detect_mcp_list_changed_kind("notifications/prompts/list_changed"),
        Some(MCPListChangedKind::Prompts)
    );
    assert_eq!(
        detect_mcp_list_changed_kind("notifications/resources/list_changed"),
        Some(MCPListChangedKind::Resources)
    );
    assert_eq!(detect_mcp_list_changed_kind("notifications/unknown"), None);
}

// ===== Sentinel resolution tests for runtime_server_config (W26-5, P1-8) =====

use super::MCPServerManager;
use crate::service::mcp::server::MCPServerConfig;
use std::collections::HashMap;
use std::sync::Arc;

/// Build an [`MCPServerManager`] whose config service reads from an isolated
/// temp config root (same shape as the
/// `core_mcp_config_store_returns_none_for_missing_key_on_real_config_service`
/// precedent in `service/mcp/config/service.rs`), so `runtime_server_config`
/// misses the persisted store and falls through to the ephemeral configs the
/// tests seed.
async fn isolated_manager(test_tag: &str) -> MCPServerManager {
    let temp_root = std::env::temp_dir().join(format!("northhing-mcp-resolve-{}-{}", test_tag, uuid::Uuid::new_v4()));
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
    MCPServerManager::new(Arc::new(
        crate::service::mcp::config::MCPConfigService::new(config_service)
            .expect("mcp config service wraps the isolated config service"),
    ))
}

#[derive(Default)]
struct TestCoreCredStore {
    map: std::sync::Mutex<HashMap<String, String>>,
}

#[async_trait::async_trait]
impl northhing_runtime_ports::McpCredentialStore for TestCoreCredStore {
    async fn store(&self, account: &str, secret: &str) -> northhing_runtime_ports::PortResult<()> {
        self.map.lock().unwrap().insert(account.to_string(), secret.to_string());
        Ok(())
    }

    async fn get(&self, account: &str) -> northhing_runtime_ports::PortResult<Option<String>> {
        Ok(self.map.lock().unwrap().get(account).cloned())
    }

    async fn delete(&self, account: &str) -> northhing_runtime_ports::PortResult<()> {
        self.map.lock().unwrap().remove(account);
        Ok(())
    }
}

#[tokio::test]
async fn runtime_server_config_resolves_sentinel_to_real_secret() {
    use crate::infrastructure::credentials::{lock_global_credential_store_for_test, set_global_credential_store};
    use northhing_runtime_ports::{mcp_remote_authorization_account, MCP_AUTH_SENTINEL};

    let _global_guard = lock_global_credential_store_for_test();
    let server_id = "test-remote-srv";
    let account = mcp_remote_authorization_account(server_id);
    let store = Arc::new(TestCoreCredStore::default());
    store
        .map
        .lock()
        .unwrap()
        .insert(account, "Bearer test-secret-token".to_string());
    set_global_credential_store(store);

    let manager = isolated_manager("sentinel-resolve").await;

    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), MCP_AUTH_SENTINEL.to_string());
    let config = MCPServerConfig {
        id: server_id.to_string(),
        name: "Test Server".to_string(),
        server_type: crate::service::mcp::server::MCPServerType::Remote,
        transport: None,
        command: None,
        args: Vec::new(),
        env: HashMap::new(),
        headers,
        url: Some("https://example.com/mcp".to_string()),
        auto_start: true,
        enabled: true,
        location: crate::service::mcp::config::ConfigLocation::User,
        capabilities: Vec::new(),
        settings: Default::default(),
        oauth: None,
        xaa: None,
    };

    manager
        .ephemeral_configs
        .write()
        .await
        .insert(server_id.to_string(), config);

    let resolved = manager.runtime_server_config(server_id).await.unwrap();
    assert_eq!(
        resolved.headers.get("Authorization").map(String::as_str),
        Some("Bearer test-secret-token")
    );
}

#[tokio::test]
async fn runtime_server_config_fails_closed_when_credential_missing() {
    use crate::infrastructure::credentials::{lock_global_credential_store_for_test, set_global_credential_store};
    use northhing_runtime_ports::MCP_AUTH_SENTINEL;

    let _global_guard = lock_global_credential_store_for_test();
    let server_id = "missing-remote-srv";
    let store = Arc::new(TestCoreCredStore::default());
    set_global_credential_store(store);

    let manager = isolated_manager("credential-missing").await;

    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), MCP_AUTH_SENTINEL.to_string());
    let config = MCPServerConfig {
        id: server_id.to_string(),
        name: "Missing Server".to_string(),
        server_type: crate::service::mcp::server::MCPServerType::Remote,
        transport: None,
        command: None,
        args: Vec::new(),
        env: HashMap::new(),
        headers,
        url: Some("https://example.com/mcp".to_string()),
        auto_start: true,
        enabled: true,
        location: crate::service::mcp::config::ConfigLocation::User,
        capabilities: Vec::new(),
        settings: Default::default(),
        oauth: None,
        xaa: None,
    };

    manager
        .ephemeral_configs
        .write()
        .await
        .insert(server_id.to_string(), config);

    let err = manager.runtime_server_config(server_id).await.unwrap_err();
    assert!(
        err.to_string().contains("Credential not found"),
        "expected Credential not found error, got: {err}"
    );
}
