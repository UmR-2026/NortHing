//! KernelTurnApi implementation.

use std::path::Path;

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::turn::{DialogSubmitOutcomeDto, TurnId, TurnInputDto, TurnStateDto};

use crate::agentic::coordination::global_scheduler;
use crate::agentic::coordination::DialogSubmissionPolicy;
use crate::agentic::coordination::DialogTriggerSource;

impl super::KernelFacade {
    /// Best-effort lookup of the session that owns a given turn. Scans the
    /// in-memory session list for a session whose `dialog_turn_ids` contains
    /// the target turn id.
    async fn find_session_for_turn(&self, turn_id: &str) -> Option<String> {
        // The coordinator does not expose a turn→session index, so we scan
        // by listing all sessions and checking dialog_turn_ids on each.
        let coordinator = match self.coordinator() {
            Ok(c) => c,
            Err(_) => return None,
        };
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
}

#[async_trait]
impl northhing_kernel_api::KernelTurnApi for super::KernelFacade {
    async fn submit_turn(&self, input: TurnInputDto) -> Result<DialogSubmitOutcomeDto, KernelError> {
        // Workspace resolution priority:
        // 1. input.workspace_path (explicit, from caller)
        // 2. resolve_session_workspace_path (session record; needed for scheduler restore)
        // 3. default_workspace_path (last resort)
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
            .ok_or_else(|| {
                KernelError::NotFound(format!("turn not found in session: {turn_id}"))
            })?;
        let turn = self
            .coordinator()?
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
