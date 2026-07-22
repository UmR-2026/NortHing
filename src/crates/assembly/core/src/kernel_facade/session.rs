//! KernelSessionApi implementation.

use std::path::Path;

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::session::{
    BranchId, SessionBranchDto, SessionConfigDto, SessionDto, SessionId, SessionSummaryDto,
};

use crate::agentic::core::SessionConfig;

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
        let name = format!("session-{}", crate::kernel_facade::helpers::system_time_to_ms());
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
            None => Err(KernelError::NotFound(format!(
                "session metadata not found: {id}"
            ))),
        }
    }

    async fn create_branch(&self, request: SessionBranchDto) -> Result<BranchId, KernelError> {
        let workspace = self
            .coordinator()?
            .resolve_session_workspace_path(&request.parent_session_id)
            .await
            .ok_or_else(|| {
                KernelError::NotFound(format!(
                    "parent session not found: {}",
                    request.parent_session_id
                ))
            })?;
        let branch_name = request
            .name
            .unwrap_or_else(|| format!("branch-{}", crate::kernel_facade::helpers::system_time_to_ms()));
        let result = northhing_services_integrations::git::GitService::create_branch(
            &workspace,
            &branch_name,
            None,
        )
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
        Err(KernelError::Internal(
            "not yet wired: get_persistence_handle — PersistenceManager folding deferred (K4b)".to_string(),
        ))
    }
}
