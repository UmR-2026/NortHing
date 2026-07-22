//! KernelTurnApi implementation.

use std::path::Path;

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::turn::{DialogSubmitOutcomeDto, TurnId, TurnInputDto, TurnStateDto};

use crate::agentic::coordination::global_scheduler;
use crate::agentic::coordination::DialogSubmissionPolicy;
use crate::agentic::coordination::DialogTriggerSource;

/// Best-effort lookup of the session that owns a given turn.
pub(crate) async fn find_session_for_turn(
    coordinator: &crate::agentic::coordination::ConversationCoordinator,
    turn_id: &str,
) -> Option<String> {
    let workspace = crate::kernel_facade::helpers::default_workspace_path();
    let Ok(summaries) = coordinator
        .list_sessions(Path::new(&workspace))
        .await
    else {
        return None;
    };
    for summary in summaries {
        if let Some(session) = coordinator
            .session_manager()
            .get_session(&summary.session_id)
        {
            if session.dialog_turn_ids.iter().any(|t| t == turn_id) {
                return Some(session.session_id);
            }
        }
    }
    None
}

#[async_trait]
impl northhing_kernel_api::KernelTurnApi for super::KernelFacade {
    async fn submit_turn(&self, input: TurnInputDto) -> Result<DialogSubmitOutcomeDto, KernelError> {
        let scheduler = global_scheduler().ok_or_else(|| {
            KernelError::Runtime("global scheduler not available — init_core not called".to_string())
        })?;
        let workspace = if let Some(ref wp) = input.workspace_path {
            wp.clone()
        } else {
            match self.coordinator().ok() {
                Some(c) => match c.resolve_session_workspace_path(&input.session_id).await {
                    Some(p) => p.to_string_lossy().to_string(),
                    None => crate::kernel_facade::helpers::default_workspace_path(),
                },
                None => crate::kernel_facade::helpers::default_workspace_path(),
            }
        };
        let policy = DialogSubmissionPolicy::for_source(DialogTriggerSource::DesktopApi);
        let outcome = scheduler
            .submit(
                input.session_id.clone(),
                input.text,
                None,
                None,
                input.mode,
                Some(workspace),
                policy,
                None,
                None,
                None,
            )
            .await
            .map_err(|e| KernelError::Runtime(format!("submit_turn failed: {e}")))?;
        Ok(crate::kernel_facade::dto::outcome_to_dto(outcome))
    }

    async fn stop_turn(&self, turn_id: &TurnId) -> Result<(), KernelError> {
        let coordinator = self.coordinator()?;
        let session_id = find_session_for_turn(coordinator, turn_id)
            .await
            .ok_or_else(|| KernelError::NotFound(format!("turn not found: {turn_id}")))?;
        coordinator
            .cancel_dialog_turn(&session_id, turn_id)
            .await
            .map_err(|e| KernelError::Runtime(format!("stop_turn failed: {e}")))?;
        Ok(())
    }

    async fn get_turn_state(&self, turn_id: &TurnId) -> Result<TurnStateDto, KernelError> {
        let coordinator = self.coordinator()?;
        let session_id = find_session_for_turn(coordinator, turn_id)
            .await
            .ok_or_else(|| KernelError::NotFound(format!("turn not found: {turn_id}")))?;
        let workspace = coordinator
            .resolve_session_workspace_path(&session_id)
            .await
            .ok_or_else(|| KernelError::NotFound(format!("session not found: {session_id}")))?;
        let session = coordinator
            .session_manager()
            .get_session(&session_id)
            .ok_or_else(|| KernelError::NotFound(format!("session not found: {session_id}")))?;
        let turn_index = session
            .dialog_turn_ids
            .iter()
            .position(|t| t == turn_id)
            .ok_or_else(|| {
                KernelError::NotFound(format!("turn not found in session: {turn_id}"))
            })?;
        let turn = coordinator
            .session_manager()
            .persistence_manager
            .load_dialog_turn(&workspace, &session_id, turn_index)
            .await
            .map_err(|e| KernelError::Runtime(format!("load_dialog_turn failed: {e}")))?
            .ok_or_else(|| {
                KernelError::NotFound(format!("turn not found in storage: {turn_id}"))
            })?;
        Ok(TurnStateDto {
            state: crate::kernel_facade::dto::turn_status_to_kind(&turn.status),
            duration_ms: turn.duration_ms,
        })
    }
}
