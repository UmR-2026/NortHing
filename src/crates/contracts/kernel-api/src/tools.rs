//! Kernel tools API and tool DTOs.

use crate::error::KernelError;
use std::sync::Arc;

// ── Tool DTOs ─────────────────────────────────────────────────────────────────

/// Tool info DTO (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolInfoDto {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
}

/// Tool render options DTO.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolRenderOptionsDto {
    pub render_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// Tool result DTO (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolResultDto {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Tool use context DTO (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolUseContextDto {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Validation result DTO.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationResultDto {
    pub valid: bool,
    #[serde(default)]
    pub errors: Vec<String>,
}

/// User input request DTO (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserInputRequestDto {
    pub request_id: String,
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

/// User input response DTO (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserInputResponseDto {
    pub request_id: String,
    pub approved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Value>,
}

// ── KernelToolsApi ─────────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait KernelToolsApi: Send + Sync {
    /// List registered tools.
    /// Source: #30
    async fn list_tools(&self) -> Result<Vec<ToolInfoDto>, KernelError>;

    /// Register a tool (abnormal item 6 solution: ACP implements ToolPort not core Tool).
    /// Source: #31 #32 #33 #34 #35
    async fn register_tool(&self, tool: Arc<dyn ToolPort>) -> Result<(), KernelError>;

    /// Request user input (permission/confirm).
    /// Source: #36
    async fn request_user_input(&self, request: UserInputRequestDto) -> Result<UserInputResponseDto, KernelError>;
}

// ── ToolPort (ACP tool boundary trait) ─────────────────────────────────────────

/// ACP tool boundary trait (replaces core Tool trait); #[async_trait]统一风格.
#[async_trait::async_trait]
pub trait ToolPort: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn validate(&self, input: &serde_json::Value) -> ValidationResultDto;
    async fn execute(&self, input: serde_json::Value, ctx: &ToolUseContextDto) -> ToolResultDto;
    fn render_options(&self) -> ToolRenderOptionsDto;
}
