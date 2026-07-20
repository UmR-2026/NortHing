//! Kernel platform API and platform DTOs.

use crate::error::KernelError;
use crate::session::SessionId;
use crate::settings::MCPServerStatusDto;

// ── Platform DTOs ─────────────────────────────────────────────────────────────

/// Terminal config DTO (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TerminalConfigDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cols: Option<u16>,
}

/// Image context DTO.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageContextDto {
    pub image_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

/// Analysis result DTO (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalysisDto {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Core health DTO (F2 new).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoreHealthDto {
    pub healthy: bool,
    #[serde(default)]
    pub details: Vec<String>,
}

/// Panel DTO (F3 new).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PanelDto {
    pub name: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// Panels config DTO (F3 new).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PanelsConfigDto {
    #[serde(default)]
    pub panels: Vec<PanelDto>,
}

/// Skill status DTO (F2 new).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillStatusDto {
    pub skill_id: String,
    pub name: String,
    pub enabled: bool,
    pub status: String,
}

/// Inspector data DTO (F2 new: model display name + MCP/skills status).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InspectorDataDto {
    pub model_name: String,
    #[serde(default)]
    pub mcp_status: Vec<MCPServerStatusDto>,
    #[serde(default)]
    pub skills_status: Vec<SkillStatusDto>,
}

/// Artifact DTO (F2 new; F2 review decision: if no existing core support, F2 review decides).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactDto {
    pub id: String,
    pub kind: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

// ── KernelPlatformApi ──────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait KernelPlatformApi: Send + Sync {
    /// Open terminal (#73 RuntimeManager::new attributed here; #74 TerminalApi/TerminalConfig).
    /// Source: #73 #74
    async fn open_terminal(&self, config: TerminalConfigDto) -> Result<(), KernelError>;

    /// Analyze image context.
    /// Source: #47
    async fn analyze_image(&self, context: ImageContextDto) -> Result<AnalysisDto, KernelError>;

    /// Get core health status (F2 new).
    /// Source: F2 core_health
    async fn get_core_health(&self) -> Result<CoreHealthDto, KernelError>;

    /// Read panels config (F3 new).
    /// Source: F3 panels.json
    async fn read_panels_config(&self) -> Result<PanelsConfigDto, KernelError>;

    /// Query if onboarding is complete (F2 new).
    /// Source: F2 onboarding
    async fn is_onboarding_complete(&self) -> Result<bool, KernelError>;

    /// Complete onboarding (F2 new).
    /// Source: F2 onboarding
    async fn complete_onboarding(&self) -> Result<(), KernelError>;

    /// Get Inspector data (F2 new: model display name + MCP/skills status).
    /// Source: F2 Inspector
    async fn get_inspector_data(&self) -> Result<InspectorDataDto, KernelError>;

    /// List artifacts (F2 review decision: if no existing core support).
    /// Source: F2 artifact panel
    async fn list_artifacts(&self, session_id: &SessionId) -> Result<Vec<ArtifactDto>, KernelError>;
}
