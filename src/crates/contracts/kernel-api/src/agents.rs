//! Kernel agents API and agent/skill DTOs.

use crate::error::{KernelError, KernelResult};
use crate::session::SessionId;

// ── Agent/Skill DTOs ───────────────────────────────────────────────────────────

/// Agent info DTO (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentInfoDto {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,
}

/// Subagent scope DTO.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubagentScopeDto {
    pub scope_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<SessionId>,
}

/// Subagent DTO (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubagentDto {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<SessionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Skill scope DTO.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillScopeDto {
    pub scope_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
}

/// Skill info DTO (includes mode field from #40 ModeSkillInfo folding).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillInfoDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Skill overrides DTO (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillOverridesDto {
    #[serde(default)]
    pub overrides: Vec<SkillOverrideEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillOverrideEntry {
    pub skill_id: String,
    pub key: String,
    pub value: serde_json::Value,
}

/// Project skills DTO (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectSkillsDto {
    pub workspace_path: String,
    #[serde(default)]
    pub skills: Vec<ProjectSkillEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectSkillEntry {
    pub skill_id: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

// ── KernelAgentsApi ────────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait KernelAgentsApi: Send + Sync {
    /// List agents.
    /// Source: #23 #24
    async fn list_agents(&self) -> Result<Vec<AgentInfoDto>, KernelError>;

    /// List subagents.
    /// Source: #25 #26 #27
    async fn list_subagents(&self, scope: SubagentScopeDto) -> Result<Vec<SubagentDto>, KernelError>;

    /// List skills (abnormal item 5 solution: encapsulates deep internal paths).
    /// Source: #37 #38 #39 #40 (ModeSkillInfo folded into SkillInfoDto.mode field)
    async fn list_skills(&self) -> Result<Vec<SkillInfoDto>, KernelError>;

    /// Get a single skill detail.
    /// Source: #39
    async fn get_skill(&self, id: &str) -> Result<SkillInfoDto, KernelError>;

    /// Set skill enabled state.
    /// Source: #42 #46
    async fn set_skill_enabled(&self, id: &str, scope: SkillScopeDto, enabled: bool) -> Result<(), KernelError>;

    /// Load skill override config.
    /// Source: #43
    async fn load_skill_overrides(&self) -> Result<SkillOverridesDto, KernelError>;

    /// Load project skill docs.
    /// Source: #44
    async fn load_project_skills(&self) -> Result<ProjectSkillsDto, KernelError>;

    /// Save project skill docs.
    /// Source: #45
    async fn save_project_skills(&self, doc: ProjectSkillsDto) -> Result<(), KernelError>;

    /// Resolve skill default enabled state.
    /// Source: #41
    async fn resolve_skill_default_enabled(&self, skill_id: &str, mode: &str) -> Result<bool, KernelError>;
}
