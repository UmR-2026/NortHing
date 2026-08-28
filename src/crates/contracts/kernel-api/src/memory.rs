//! Kernel memory API: episode log listing and fact browsing.

use crate::error::KernelError;

/// DTO for episode data exposed via the kernel API.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EpisodeDto {
    pub schema_version: u32,
    pub turn_id: String,
    pub session_id: String,
    pub workspace_slug: String,
    pub agent_type: String,
    pub task_summary: String,
    pub tools_used: Vec<ToolUseRecordDto>,
    pub failures: Vec<ToolFailureRecordDto>,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    pub ts: u64,
    #[serde(default)]
    pub redline_verdicts: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ToolUseRecordDto {
    pub name: String,
    pub ok: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ToolFailureRecordDto {
    pub tool: String,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair: Option<String>,
}

/// DTO for a memory fact exposed via the kernel API.
///
/// Enums are flattened to strings; `schema_version` is omitted
/// (UI-agnostic metadata).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FactDto {
    pub id: String,
    pub text: String,
    pub scope: String,
    pub confidence: String,
    pub fact_type: String,
    pub created_at: u64,
    pub session_id: String,
    pub turn_id: String,
}

/// Kernel memory API trait for reading growth/learning data.
#[async_trait::async_trait]
pub trait KernelMemoryApi {
    /// List episodes for a workspace, ordered by timestamp descending.
    async fn list_episodes(&self, workspace_slug: &str, limit: Option<u32>) -> Result<Vec<EpisodeDto>, KernelError>;

    /// List memory facts. `workspace_slug = Some` returns global + workspace facts;
    /// `None` returns global facts only (mirrors `MemoryDb::get_facts` semantics).
    async fn list_facts(&self, workspace_slug: Option<&str>) -> Result<Vec<FactDto>, KernelError>;

    /// Full-text search memory facts. Score is not exposed in the DTO.
    /// `limit` defaults to 20 when `None`.
    async fn search_facts(&self, query: &str, workspace_slug: Option<&str>, limit: Option<u32>) -> Result<Vec<FactDto>, KernelError>;
}
