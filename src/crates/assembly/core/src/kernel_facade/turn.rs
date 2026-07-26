//! KernelTurnApi implementation.

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::turn::{DialogSubmitOutcomeDto, TurnId, TurnInputDto, TurnStateDto};

use crate::agentic::coordination::global_scheduler;
use crate::agentic::coordination::DialogSubmissionPolicy;
use crate::agentic::coordination::DialogTriggerSource;

fn session_owns_turn(dialog_turn_ids: &[String], turn_id: &str) -> bool {
    dialog_turn_ids.iter().any(|id| id == turn_id)
}

impl super::KernelFacade {
    /// Best-effort lookup of the session that owns a given turn. The in-memory
    /// session map is authoritative for both active and queued scheduler turns.
    async fn find_session_for_turn(&self, turn_id: &str) -> Option<String> {
        let coordinator = self.coordinator().ok()?;
        coordinator
            .session_manager()
            .sessions
            .iter()
            .find(|session| session_owns_turn(&session.dialog_turn_ids, turn_id))
            .map(|session| session.session_id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::session_owns_turn;

    #[test]
    fn turn_lookup_matches_active_and_queued_turn_ids() {
        let ids = vec!["active-turn".to_string(), "queued-turn".to_string()];
        assert!(session_owns_turn(&ids, "active-turn"));
        assert!(session_owns_turn(&ids, "queued-turn"));
        assert!(!session_owns_turn(&ids, "other-turn"));
    }
}

#[async_trait]
impl northhing_kernel_api::KernelTurnApi for super::KernelFacade {
    async fn submit_turn(&self, input: TurnInputDto) -> Result<DialogSubmitOutcomeDto, KernelError> {
        // Workspace resolution priority:
        // 1. input.workspace_path (explicit, from caller)
        // 2. resolve_session_workspace_path (session record; needed for scheduler restore)
        // 3. default_workspace_path (last resort)
        let scheduler = global_scheduler()
            .ok_or_else(|| KernelError::Runtime("global scheduler not available — init_core not called".to_string()))?;
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
        let session_id = self
            .find_session_for_turn(turn_id)
            .await
            .ok_or_else(|| KernelError::NotFound(format!("turn not found: {turn_id}")))?;
        self.coordinator()?
            .cancel_dialog_turn(&session_id, turn_id)
            .await
            .map_err(|e| KernelError::Runtime(format!("stop_turn failed: {e}")))?;
        Ok(())
    }

    async fn get_turn_state(&self, turn_id: &TurnId) -> Result<TurnStateDto, KernelError> {
        // Core does not expose a direct turn-state query. Best-effort: scan
        // the in-memory session's dialog_turn_ids to find the owning session,
        // then read the persisted turn and map status → TurnStateKind.
        // duration_ms is None when unavailable (flagged in report).
        let session_id = self
            .find_session_for_turn(turn_id)
            .await
            .ok_or_else(|| KernelError::NotFound(format!("turn not found: {turn_id}")))?;
        let workspace = self
            .coordinator()?
            .resolve_session_workspace_path(&session_id)
            .await
            .ok_or_else(|| KernelError::NotFound(format!("session not found: {session_id}")))?;
        let session = self
            .coordinator()?
            .session_manager()
            .get_session(&session_id)
            .ok_or_else(|| KernelError::NotFound(format!("session not found: {session_id}")))?;
        let turn_index = session
            .dialog_turn_ids
            .iter()
            .position(|t| t == turn_id)
            .ok_or_else(|| KernelError::NotFound(format!("turn not found in session: {turn_id}")))?;
        let turn = self
            .coordinator()?
            .session_manager()
            .persistence_manager
            .load_dialog_turn(&workspace, &session_id, turn_index)
            .await
            .map_err(|e| KernelError::Runtime(format!("load_dialog_turn failed: {e}")))?
            .ok_or_else(|| KernelError::NotFound(format!("turn not found in storage: {turn_id}")))?;
        Ok(TurnStateDto {
            state: crate::kernel_facade::dto::turn_status_to_kind(&turn.status),
            duration_ms: turn.duration_ms,
        })
    }
}
