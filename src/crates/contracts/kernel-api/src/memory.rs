//! Kernel memory API: episode log listing.

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

/// Kernel memory API trait for reading growth/learning data.
#[async_trait::async_trait]
pub trait KernelMemoryApi {
    /// List episodes for a workspace, ordered by timestamp descending.
    async fn list_episodes(
        &self,
        workspace_slug: &str,
        limit: Option<u32>,
    ) -> Result<Vec<EpisodeDto>, KernelError>;
}
