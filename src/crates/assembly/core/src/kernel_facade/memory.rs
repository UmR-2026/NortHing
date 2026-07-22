//! KernelMemoryApi implementation.

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::memory::{EpisodeDto, ToolFailureRecordDto, ToolUseRecordDto};

#[async_trait]
impl northhing_kernel_api::memory::KernelMemoryApi for super::KernelFacade {
    async fn list_episodes(
        &self,
        workspace_slug: &str,
        limit: Option<u32>,
    ) -> Result<Vec<EpisodeDto>, KernelError> {
        let limit = limit.unwrap_or(100) as usize;
        let episodes = crate::agentic::episodes::read_episodes(workspace_slug, limit)
            .await
            .map_err(|e| KernelError::Runtime(format!("list_episodes failed: {}", e)))?;

        Ok(episodes
            .into_iter()
            .map(|ep| EpisodeDto {
                schema_version: ep.schema_version,
                turn_id: ep.turn_id,
                session_id: ep.session_id,
                workspace_slug: ep.workspace_slug,
                agent_type: ep.agent_type,
                task_summary: ep.task_summary,
                tools_used: ep
                    .tools_used
                    .into_iter()
                    .map(|t| ToolUseRecordDto { name: t.name, ok: t.ok })
                    .collect(),
                failures: ep
                    .failures
                    .into_iter()
                    .map(|f| ToolFailureRecordDto {
                        tool: f.tool,
                        error: f.error,
                        repair: f.repair,
                    })
                    .collect(),
                outcome: match ep.outcome {
                    crate::agentic::episodes::types::EpisodeOutcome::Completed => "completed".to_string(),
                    crate::agentic::episodes::types::EpisodeOutcome::Failed => "failed".to_string(),
                    crate::agentic::episodes::types::EpisodeOutcome::Cancelled => "cancelled".to_string(),
                },
                duration_ms: ep.duration_ms,
                ts: ep.ts,
                redline_verdicts: ep
                    .redline_verdicts
                    .into_iter()
                    .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
                    .collect(),
            })
            .collect())
    }
}
