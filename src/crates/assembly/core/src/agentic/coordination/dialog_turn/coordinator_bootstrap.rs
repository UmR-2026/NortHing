//! Sub-domain: bootstrap.
//! Spec §2.1 — facade method extracted from dialog_turn/mod.rs (R44a refactor).
//! Contains the public `ensure_assistant_bootstrap` method that triggers a
//! bootstrap turn for empty sessions.

use super::super::coordinator::*;
use super::super::ports::*;
use super::super::scheduler::*;
use super::super::turn_outcome::TurnOutcome;
use super::DialogTriggerSource;

use crate::agentic::core::{InternalReminderKind, Message, SessionState};
use crate::agentic::WorkspaceBinding;
use crate::service::bootstrap::{ensure_workspace_persona_files_for_prompt, is_workspace_bootstrap_pending};
use crate::util::errors::NortHingResult;
use std::path::PathBuf;

impl ConversationCoordinator {
    /// Ensure the completed/failed/cancelled turn is persisted to the workspace
    /// session storage. If the frontend already saved a richer version
    /// during streaming, we only update the final status; otherwise we create
    /// a minimal record with the user message so the turn is never lost.
    /// Safety-net persistence: only creates a minimal record when the frontend
    /// has not saved anything yet.  The frontend's PersistenceModule is the
    /// authoritative writer for turn content (model rounds, text, tools, etc.)
    /// and final status.  This function must NOT overwrite frontend-managed
    /// data, because the spawned task always runs before the frontend receives
    /// the DialogTurnCompleted event via the transport layer, and the existing
    /// disk data from debounced saves may have incomplete model rounds.
    pub async fn ensure_assistant_bootstrap(
        &self,
        session_id: String,
        workspace_path: String,
    ) -> NortHingResult<AssistantBootstrapEnsureOutcome> {
        let workspace_root = PathBuf::from(&workspace_path);
        // Empty or partial assistant dirs may never have run create_assistant_workspace; fill only
        // missing persona stubs (never overwrite), while preserving completed bootstrap state.
        ensure_workspace_persona_files_for_prompt(&workspace_root).await?;
        let bootstrap_pending = is_workspace_bootstrap_pending(&workspace_root);
        if !bootstrap_pending {
            return Ok(AssistantBootstrapEnsureOutcome::Skipped {
                session_id,
                reason: AssistantBootstrapSkipReason::BootstrapNotRequired,
            });
        }

        let session = match self.session_manager.get_session(&session_id) {
            Some(session) => session,
            None => {
                self.session_manager
                    .restore_session(&workspace_root, &session_id)
                    .await?
            }
        };

        let turn_count = self.session_manager.get_turn_count(&session_id);

        if turn_count > 0 {
            return Ok(AssistantBootstrapEnsureOutcome::Skipped {
                session_id,
                reason: AssistantBootstrapSkipReason::SessionHasExistingTurns,
            });
        }

        if !matches!(session.state, SessionState::Idle) {
            return Ok(AssistantBootstrapEnsureOutcome::Skipped {
                session_id,
                reason: AssistantBootstrapSkipReason::SessionNotIdle,
            });
        }

        let is_chinese = Self::is_chinese_locale().await;
        let kickoff_query = Self::assistant_bootstrap_kickoff_query(is_chinese);
        let expected_reply_language = if is_chinese { "Chinese" } else { "English" };
        let workspace_binding = WorkspaceBinding::new(None, workspace_root.clone());
        let model_id = self
            .execution_engine
            .resolve_model_id_for_turn(
                &session,
                ASSISTANT_BOOTSTRAP_AGENT_TYPE,
                Some(&workspace_binding),
                kickoff_query,
                0,
            )
            .await?;

        let ai_client_factory = match crate::infrastructure::ai::get_global_ai_client_factory().await {
            Ok(factory) => factory,
            Err(error) => {
                return Ok(AssistantBootstrapEnsureOutcome::Blocked {
                    session_id,
                    reason: AssistantBootstrapBlockReason::ModelUnavailable,
                    detail: format!("Failed to get AI client factory: {error}"),
                });
            }
        };

        if let Err(error) = ai_client_factory.get_client_resolved(&model_id).await {
            return Ok(AssistantBootstrapEnsureOutcome::Blocked {
                session_id,
                reason: AssistantBootstrapBlockReason::ModelUnavailable,
                detail: format!("Failed to get AI client (model_id={model_id}): {error}"),
            });
        }

        let kickoff_reminder = Self::assistant_bootstrap_system_reminder(kickoff_query, expected_reply_language);

        let turn_id = format!("assistant-bootstrap-{}", uuid::Uuid::new_v4());
        let metadata = serde_json::json!({
            "assistant_bootstrap": {
                "trigger": "lazy_auto",
                "system_generated": true,
                "workspace_path": workspace_path,
            }
        });

        self.start_dialog_turn_internal(
            session_id.clone(),
            kickoff_query.to_string(),
            Some(kickoff_query.to_string()),
            None,
            Some(turn_id.clone()),
            ASSISTANT_BOOTSTRAP_AGENT_TYPE.to_string(),
            Some(workspace_root.to_string_lossy().to_string()),
            DialogSubmissionPolicy::for_source(DialogTriggerSource::DesktopApi).with_skip_tool_confirmation(true),
            Some(metadata),
            vec![Message::internal_reminder(
                InternalReminderKind::Generic,
                kickoff_reminder,
            )],
            true,
        )
        .await?;

        Ok(AssistantBootstrapEnsureOutcome::Started { session_id, turn_id })
    }
}
