//! Kernel settings API and settings DTOs.

use crate::error::{KernelError, KernelResult};
use crate::session::SessionId;

// ── Settings DTOs ─────────────────────────────────────────────────────────────

/// Global config DTO (enumerated from core GlobalConfig at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GlobalConfigDto {
    pub providers: Vec<ProviderConfigDto>,
    pub default_provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderConfigDto {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// Global config patch DTO.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GlobalConfigPatchDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<Vec<ProviderConfigDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_config: Option<serde_json::Value>,
}

/// AI model config DTO (enumerated from core at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AIModelConfigDto {
    pub id: String,
    pub provider_id: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
}

/// Config location DTO (I1 decision: User | Project | BuiltIn).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigLocationDto {
    User,
    Project,
    BuiltIn,
}

/// MCP server config DTO.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MCPServerConfigDto {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MCPServerDto {
    pub id: String,
    pub name: String,
    pub config: MCPServerConfigDto,
    pub location: ConfigLocationDto,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MCPServerStatusDto {
    pub id: String,
    pub status: MCPServerStatusKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum MCPServerStatusKind {
    Connected,
    Starting,
    Disabled,
    Failed { message: String },
    ProbeTimeout,
}

/// Provider test result DTO.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderTestResultDto {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Provider form DTO for testing provider config (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderFormDto {
    pub provider_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

// ── KernelSettingsApi ──────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait KernelSettingsApi: Send + Sync {
    /// Get global config.
    /// Source: #49 #50 #51
    async fn get_global_config(&self) -> Result<GlobalConfigDto, KernelError>;

    /// Update global config (patch semantics).
    /// Source: #52 (GlobalConfigManager is alias mapping for get_global_config_service, not separately exposed)
    async fn update_global_config(&self, patch: GlobalConfigPatchDto) -> Result<(), KernelError>;

    /// List model configs.
    /// Source: #53
    async fn list_model_configs(&self) -> Result<Vec<AIModelConfigDto>, KernelError>;

    /// Add/update model config.
    /// Source: #53
    async fn upsert_model_config(&self, config: AIModelConfigDto) -> Result<(), KernelError>;

    /// Delete model config.
    /// Source: #53
    async fn delete_model_config(&self, id: &str) -> Result<(), KernelError>;

    /// Set default provider.
    /// Source: #53
    async fn set_default_provider(&self, id: &str) -> Result<(), KernelError>;

    /// List MCP server configs.
    /// Source: #60 #61 #65
    async fn list_mcp_servers(&self) -> Result<Vec<MCPServerDto>, KernelError>;

    /// Add/update MCP server.
    /// Source: #61
    async fn upsert_mcp_server(&self, config: MCPServerDto) -> Result<(), KernelError>;

    /// Delete MCP server.
    /// Source: #61
    async fn delete_mcp_server(&self, id: &str) -> Result<(), KernelError>;

    /// Query MCP server connection status.
    /// Source: #62
    async fn get_mcp_status(&self, id: &str) -> Result<MCPServerStatusDto, KernelError>;

    /// Test provider connection (F2 new; abnormal item 3 solution: replaces AIClient::new direct connection).
    /// Source: F2 test_provider(id)
    async fn test_provider(&self, id: &str) -> Result<ProviderTestResultDto, KernelError>;

    /// Test provider config (F2 new).
    /// Source: F2 test_provider_config(form)
    async fn test_provider_config(&self, form: ProviderFormDto) -> Result<ProviderTestResultDto, KernelError>;

    // F2-conditional placeholder (occupies 2 method slots; if F2 doesn't need per-server start/stop, do not implement):
    // start_mcp_server(&self, id: &str) -> Result<(), KernelError>
    // stop_mcp_server(&self, id: &str) -> Result<(), KernelError>
}
