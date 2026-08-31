//! KernelSessionApi implementation.

use std::path::Path;

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::session::{
    BranchId, SessionBranchDto, SessionConfigDto, SessionDto, SessionId, SessionSearchHitDto, SessionSummaryDto,
    WorkspaceSessionsDto,
};

use crate::agentic::core::SessionConfig;

fn extract_search_snippet(text: &str, query: &str) -> Option<String> {
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    if lower_query.is_empty() {
        return None;
    }
    let byte_pos = lower_text.find(&lower_query)?;
    let char_pos = lower_text[..byte_pos].chars().count();
    let text_chars: Vec<char> = text.chars().collect();
    let query_char_len = query.chars().count();

    let match_start = char_pos.min(text_chars.len());
    let match_end = (match_start + query_char_len).min(text_chars.len());

    let snippet_start = match_start.saturating_sub(40);
    let snippet_end = (match_end + 40).min(text_chars.len());

    Some(text_chars[snippet_start..snippet_end].iter().collect())
}

#[async_trait]
impl northhing_kernel_api::KernelSessionApi for super::KernelFacade {
    async fn create_session(&self, config: SessionConfigDto) -> Result<SessionId, KernelError> {
        let workspace = config
            .workspace_path
            .clone()
            .unwrap_or_else(crate::kernel_facade::helpers::default_workspace_path);
        let mut core_config = SessionConfig {
            workspace_path: Some(workspace),
            ..Default::default()
        };
        if !config.model_name.is_empty() {
            core_config.model_id = Some(config.model_name.clone());
        }
        let name = config
            .name
            .unwrap_or_else(|| format!("session-{}", crate::kernel_facade::helpers::system_time_to_ms()));
        let session = self
            .coordinator()?
            .create_session(name, config.agent_type, core_config)
            .await
            .map_err(|e| KernelError::Runtime(format!("create_session failed: {e}")))?;
        Ok(session.session_id)
    }

    async fn list_sessions(&self) -> Result<Vec<SessionSummaryDto>, KernelError> {
        let workspace = crate::kernel_facade::helpers::default_workspace_path();
        let summaries = self
            .coordinator()?
            .list_sessions(Path::new(&workspace))
            .await
            .map_err(|e| KernelError::Runtime(format!("list_sessions failed: {e}")))?;
        Ok(summaries
            .into_iter()
            .map(crate::kernel_facade::events::summary_to_dto)
            .collect())
    }

    async fn archive_session(&self, id: &SessionId) -> Result<(), KernelError> {
        let workspace = self
            .coordinator()?
            .resolve_session_workspace_path(id)
            .await
            .ok_or_else(|| KernelError::NotFound(format!("session not found: {id}")))?;
        let archived = self
            .coordinator()?
            .session_manager()
            .persistence_manager
            .archive_session(&workspace, id)
            .await
            .map_err(|e| KernelError::Runtime(format!("archive_session failed: {e}")))?;
        if archived {
            Ok(())
        } else {
            Err(KernelError::NotFound(format!("session metadata not found: {id}")))
        }
    }

    async fn list_sessions_all_workspaces(&self) -> Result<Vec<WorkspaceSessionsDto>, KernelError> {
        let coordinator = self.coordinator()?;
        let mut workspace_paths: Vec<String> = match crate::service::workspace::global_workspace_service() {
            Some(service) => {
                let mut infos = service.list_workspace_infos().await;
                infos.sort_by(|left, right| right.last_accessed.cmp(&left.last_accessed));
                infos
                    .into_iter()
                    .map(|info| info.root_path.to_string_lossy().to_string())
                    .collect()
            }
            None => Vec::new(),
        };
        let default_workspace = crate::kernel_facade::helpers::default_workspace_path();
        if !workspace_paths.iter().any(|path| path == &default_workspace) {
            workspace_paths.push(default_workspace);
        }

        let mut grouped = Vec::with_capacity(workspace_paths.len());
        for workspace_path in workspace_paths {
            let summaries = match coordinator
                .session_manager()
                .persistence_manager
                .list_sessions(Path::new(&workspace_path))
                .await
            {
                Ok(summaries) => summaries,
                Err(err) => {
                    tracing::warn!(
                        workspace_path = %workspace_path,
                        error = %err,
                        "Failed to list sessions for workspace; returning empty session list"
                    );
                    Vec::new()
                }
            };
            grouped.push(WorkspaceSessionsDto {
                workspace_path,
                sessions: summaries
                    .into_iter()
                    .map(crate::kernel_facade::events::summary_to_dto)
                    .collect(),
            });
        }
        Ok(grouped)
    }

    async fn get_session(&self, id: &SessionId) -> Result<SessionDto, KernelError> {
        let session = self
            .coordinator()?
            .session_manager()
            .get_session(id)
            .ok_or_else(|| KernelError::NotFound(format!("session not found: {id}")))?;
        Ok(crate::kernel_facade::events::session_to_dto(&session))
    }

    async fn delete_session(&self, id: &SessionId) -> Result<(), KernelError> {
        let workspace = self
            .coordinator()?
            .resolve_session_workspace_path(id)
            .await
            .ok_or_else(|| KernelError::NotFound(format!("session not found: {id}")))?;
        self.coordinator()?
            .delete_session(&workspace, id)
            .await
            .map_err(|e| KernelError::Runtime(format!("delete_session failed: {e}")))?;
        Ok(())
    }

    async fn rename_session(&self, id: &SessionId, name: &str) -> Result<(), KernelError> {
        self.coordinator()?
            .update_session_title(id, name)
            .await
            .map_err(|e| KernelError::Runtime(format!("rename_session failed: {e}")))?;
        Ok(())
    }

    async fn get_messages(&self, session_id: &SessionId) -> Result<Vec<super::MessageDto>, KernelError> {
        let messages = self
            .coordinator()?
            .get_messages(session_id)
            .await
            .map_err(|e| KernelError::Runtime(format!("get_messages failed: {e}")))?;
        Ok(messages
            .into_iter()
            .map(crate::kernel_facade::dto::message_to_dto)
            .collect())
    }

    async fn search_sessions(
        &self,
        query: &str,
        workspace: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<SessionSearchHitDto>, KernelError> {
        let query_trimmed = query.trim();
        if query_trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let max_hits = limit.unwrap_or(50) as usize;
        if max_hits == 0 {
            return Ok(Vec::new());
        }

        let workspace_path = workspace
            .map(|s| s.to_string())
            .unwrap_or_else(crate::kernel_facade::helpers::default_workspace_path);

        let coordinator = self.coordinator()?;
        let summaries = match coordinator
            .session_manager()
            .persistence_manager
            .list_sessions(Path::new(&workspace_path))
            .await
        {
            Ok(summaries) => summaries,
            Err(err) => {
                tracing::warn!(
                    workspace_path = %workspace_path,
                    error = %err,
                    "Failed to list sessions for workspace in search_sessions; returning empty results"
                );
                return Ok(Vec::new());
            }
        };

        // ponytail: 全量扫描 O(会话数 × 消息数)，无索引；会话到百级或消息到万级需升级（复用 transcript index 或引入 SQLite FTS）
        let mut hits = Vec::new();
        for summary in summaries {
            if hits.len() >= max_hits {
                break;
            }
            let messages = match coordinator.get_messages(&summary.session_id).await {
                Ok(messages) => messages,
                Err(err) => {
                    tracing::warn!(
                        session_id = %summary.session_id,
                        error = %err,
                        "Failed to load messages for session in search_sessions; skipping session"
                    );
                    continue;
                }
            };

            let mut session_hit_count = 0;
            for msg in messages {
                if session_hit_count >= 2 || hits.len() >= max_hits {
                    break;
                }

                let (role_str, text_content) = match &msg.role {
                    crate::agentic::core::MessageRole::User => {
                        let text = match &msg.content {
                            crate::agentic::core::MessageContent::Text(t) => Some(t.as_str()),
                            crate::agentic::core::MessageContent::Multimodal { text, .. } => Some(text.as_str()),
                            crate::agentic::core::MessageContent::Mixed { text, .. } => Some(text.as_str()),
                            _ => None,
                        };
                        ("user", text)
                    }
                    crate::agentic::core::MessageRole::Assistant => {
                        let text = match &msg.content {
                            crate::agentic::core::MessageContent::Text(t) => Some(t.as_str()),
                            crate::agentic::core::MessageContent::Multimodal { text, .. } => Some(text.as_str()),
                            crate::agentic::core::MessageContent::Mixed { text, .. } => Some(text.as_str()),
                            _ => None,
                        };
                        ("assistant", text)
                    }
                    _ => continue,
                };

                let Some(text) = text_content else {
                    continue;
                };

                if let Some(snippet) = extract_search_snippet(text, query_trimmed) {
                    hits.push(SessionSearchHitDto {
                        session_id: summary.session_id.clone(),
                        session_name: summary.session_name.clone(),
                        message_id: msg.id.clone(),
                        role: role_str.to_string(),
                        snippet,
                        timestamp_ms: crate::kernel_facade::helpers::system_time_to_ms_i64(msg.timestamp),
                    });
                    session_hit_count += 1;
                }
            }
        }

        Ok(hits)
    }

    async fn get_session_metadata(&self, id: &SessionId) -> Result<super::SessionMetadataDto, KernelError> {
        let workspace = self
            .coordinator()?
            .resolve_session_workspace_path(id)
            .await
            .ok_or_else(|| KernelError::NotFound(format!("session not found: {id}")))?;
        let metadata = self
            .coordinator()?
            .session_manager()
            .load_session_metadata(&workspace, id)
            .await
            .map_err(|e| KernelError::Runtime(format!("load_session_metadata failed: {e}")))?;
        match metadata {
            Some(m) => Ok(crate::kernel_facade::dto::metadata_to_dto(&m)),
            None => Err(KernelError::NotFound(format!("session metadata not found: {id}"))),
        }
    }

    async fn create_branch(&self, request: SessionBranchDto) -> Result<BranchId, KernelError> {
        let workspace = self
            .coordinator()?
            .resolve_session_workspace_path(&request.parent_session_id)
            .await
            .ok_or_else(|| KernelError::NotFound(format!("parent session not found: {}", request.parent_session_id)))?;
        let branch_name = request
            .name
            .unwrap_or_else(|| format!("branch-{}", crate::kernel_facade::helpers::system_time_to_ms()));
        let result = northhing_services_integrations::git::GitService::create_branch(&workspace, &branch_name, None)
            .await
            .map_err(|e| KernelError::Runtime(format!("create_branch failed: {e}")))?;
        if result.success {
            Ok(branch_name)
        } else {
            Err(KernelError::Runtime(
                result.error.unwrap_or_else(|| "git create_branch failed".to_string()),
            ))
        }
    }

    async fn get_persistence_handle(&self) -> Result<super::PersistenceHandleDto, KernelError> {
        // NEEDS_CONTEXT: PersistenceManager folding deferred to K4b.
        Err(KernelError::Internal(
            "not yet wired: get_persistence_handle — PersistenceManager folding deferred (K4b)".to_string(),
        ))
    }
}
