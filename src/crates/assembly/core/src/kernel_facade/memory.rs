//! KernelMemoryApi implementation.

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::memory::{EpisodeDto, FactDto, ToolFailureRecordDto, ToolUseRecordDto};

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

    async fn list_facts(
        &self,
        workspace_slug: Option<&str>,
    ) -> Result<Vec<FactDto>, KernelError> {
        use crate::service::agent_memory::{default_memory_db_path, FactConfidence, FactScope, FactType, MemoryDb};
        let db = MemoryDb::open(&default_memory_db_path())
            .map_err(|e| KernelError::Runtime(format!("MemoryDb open failed: {}", e)))?;
        let facts = db
            .get_facts(workspace_slug)
            .map_err(|e| KernelError::Runtime(format!("list_facts failed: {}", e)))?;
        Ok(facts
            .into_iter()
            .map(|f| FactDto {
                id: f.id,
                text: f.text,
                scope: match f.scope {
                    FactScope::Workspace => "workspace".to_string(),
                    FactScope::Global => "global".to_string(),
                },
                confidence: match f.confidence {
                    FactConfidence::High => "high".to_string(),
                    FactConfidence::Med => "med".to_string(),
                    FactConfidence::Low => "low".to_string(),
                },
                fact_type: match f.fact_type {
                    FactType::User => "user".to_string(),
                    FactType::Feedback => "feedback".to_string(),
                    FactType::Project => "project".to_string(),
                    FactType::Reference => "reference".to_string(),
                },
                created_at: f.created_at,
                session_id: f.provenance.session_id,
                turn_id: f.provenance.turn_id,
            })
            .collect())
    }

    async fn search_facts(
        &self,
        query: &str,
        workspace_slug: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<FactDto>, KernelError> {
        use crate::service::agent_memory::{default_memory_db_path, FactConfidence, FactScope, FactType, MemoryDb};
        let db = MemoryDb::open(&default_memory_db_path())
            .map_err(|e| KernelError::Runtime(format!("MemoryDb open failed: {}", e)))?;
        let limit = limit.unwrap_or(20) as usize;
        let scored = db
            .search_facts(query, workspace_slug, limit)
            .map_err(|e| KernelError::Runtime(format!("search_facts failed: {}", e)))?;
        Ok(scored
            .into_iter()
            .map(|s| {
                let f = s.fact;
                FactDto {
                    id: f.id,
                    text: f.text,
                    scope: match f.scope {
                        FactScope::Workspace => "workspace".to_string(),
                        FactScope::Global => "global".to_string(),
                    },
                    confidence: match f.confidence {
                        FactConfidence::High => "high".to_string(),
                        FactConfidence::Med => "med".to_string(),
                        FactConfidence::Low => "low".to_string(),
                    },
                    fact_type: match f.fact_type {
                        FactType::User => "user".to_string(),
                        FactType::Feedback => "feedback".to_string(),
                        FactType::Project => "project".to_string(),
                        FactType::Reference => "reference".to_string(),
                    },
                    created_at: f.created_at,
                    session_id: f.provenance.session_id,
                    turn_id: f.provenance.turn_id,
                }
            })
            .collect())
    }
}
