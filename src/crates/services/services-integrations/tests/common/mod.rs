#![allow(dead_code, unused_imports)]

pub use async_trait::async_trait;
pub use serde_json::json;
pub use std::path::PathBuf;
pub use std::sync::{Arc, Mutex};

pub use northhing_services_integrations::mcp::auth::{MCPRemoteOAuthSessionSnapshot, MCPRemoteOAuthStatus};
pub use northhing_services_integrations::mcp::config::ConfigLocation;
pub use northhing_services_integrations::mcp::config::{
    config_to_cursor_format, format_mcp_json_config_value, get_mcp_remote_authorization_source,
    get_mcp_remote_authorization_value, has_mcp_remote_authorization, has_mcp_remote_oauth, has_mcp_remote_xaa,
    merge_mcp_server_config_sources, normalize_mcp_authorization_value, parse_cursor_format,
    remove_mcp_authorization_keys, validate_mcp_json_config, MCPConfigService, MCPConfigStore,
};
pub use northhing_services_integrations::mcp::protocol::{
    create_initialize_request, create_ping_request, create_tools_call_request, create_tools_list_request,
    default_protocol_version, MCPCapability, MCPError, MCPPrompt, MCPPromptArgument, MCPPromptContent,
    MCPPromptMessage, MCPPromptMessageContent, MCPPromptMessageContentBlock, MCPRequest, MCPResource,
    MCPResourceContent, MCPTool, MCPToolAnnotations, MCPToolResult, MCPToolResultContent,
};
pub use northhing_services_integrations::mcp::server::{
    compute_mcp_backoff_delay, detect_mcp_list_changed_kind, is_mcp_auth_error_message, merge_mcp_remote_headers,
    MCPCatalogCache, MCPConnectionPool, MCPListChangedKind, MCPRuntimeErrorKind, MCPRuntimeResult, MCPServerConfig,
    MCPServerProcess, MCPServerStatus, MCPServerTransport, MCPServerType,
};
pub use northhing_services_integrations::mcp::{
    build_mcp_tool_descriptor, build_mcp_tool_name, normalize_name_for_mcp, render_mcp_tool_result_for_assistant,
    MCPContextEnhancer, MCPContextEnhancerConfig, MCPDynamicToolProvider, MCPToolCatalogClient,
    McpDynamicToolDescriptor, McpToolInfo, PromptAdapter, ResourceAdapter, MCP_TOOL_DELIMITER, MCP_TOOL_PREFIX,
};
pub use std::collections::HashMap;
pub use std::time::Duration;

pub fn make_mcp_config(
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

pub fn make_resource(name: &str, description: Option<&str>, uri: &str) -> MCPResource {
    MCPResource {
        uri: uri.to_string(),
        name: name.to_string(),
        title: None,
        description: description.map(str::to_string),
        mime_type: Some("text/plain".to_string()),
        icons: None,
        size: Some(12),
        annotations: None,
        metadata: None,
    }
}

#[derive(Default)]
pub struct InMemoryMCPConfigStore {
    pub values: tokio::sync::Mutex<HashMap<String, serde_json::Value>>,
}

#[async_trait::async_trait]
impl MCPConfigStore for InMemoryMCPConfigStore {
    async fn get_config_value(&self, key: &str) -> MCPRuntimeResult<Option<serde_json::Value>> {
        Ok(self.values.lock().await.get(key).cloned())
    }

    async fn set_config_value(&self, key: &str, value: serde_json::Value) -> MCPRuntimeResult<()> {
        self.values.lock().await.insert(key.to_string(), value);
        Ok(())
    }
}

pub struct FailingMCPConfigStore;

#[async_trait::async_trait]
impl MCPConfigStore for FailingMCPConfigStore {
    async fn get_config_value(&self, key: &str) -> MCPRuntimeResult<Option<serde_json::Value>> {
        Err(northhing_services_integrations::mcp::MCPRuntimeError::configuration(
            format!("backend unavailable for {key}"),
        ))
    }

    async fn set_config_value(&self, key: &str, _value: serde_json::Value) -> MCPRuntimeResult<()> {
        Err(northhing_services_integrations::mcp::MCPRuntimeError::configuration(
            format!("backend unavailable for {key}"),
        ))
    }
}

pub struct FakeMCPToolCatalogClient {
    pub tools: Vec<MCPTool>,
}

#[async_trait::async_trait]
impl MCPToolCatalogClient for FakeMCPToolCatalogClient {
    async fn list_mcp_tools(&self) -> MCPRuntimeResult<Vec<MCPTool>> {
        Ok(self.tools.clone())
    }
}
