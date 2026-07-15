//! Sub-domain: BTW, fork, cleanup, session title, events, accessors.
//! Spec step-3.7 — extracted from subagent_orchestrator.rs (R50b refactor).

use super::super::coordinator::*;
use super::super::ports::*;
use crate::agentic::agents::agent_registry;
use crate::agentic::context_profile::ContextProfilePolicy;
use crate::agentic::core::{Session, SessionConfig, SessionKind, SessionState};
use crate::agentic::events::{AgenticEvent, DeepReviewQueueState, EventPriority};
use crate::agentic::fork_agent::ForkAgentContextSnapshot;
use crate::agentic::goal_mode::{
    effective_subagent_timeout_seconds, is_usage_limit_error, maybe_build_continuation_after_turn,
    should_skip_goal_continuation_after_turn, should_skip_goal_for_turn, thread_goal_status_is_resumable,
    user_facing_thread_goal_error, ThreadGoalRuntime, ThreadGoalStore,
};
use crate::agentic::session::SessionManager;
use crate::agentic::side_question::build_btw_user_input;
use crate::service::bootstrap::{ensure_workspace_persona_files_for_prompt, is_workspace_bootstrap_pending};
use crate::service::config::global::GlobalConfigManager;
use crate::service::remote_ssh::normalize_remote_workspace_path;
use crate::service::session::{SessionRelationship, SessionRelationshipKind};
use crate::service::workspace::{global_workspace_service, WorkspaceCreateOptions, WorkspaceKind};
use crate::service_agent_runtime::CoreServiceAgentRuntime;
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_runtime_ports::DialogSubmissionPolicy;
use std::sync::Arc;
use tokio::time::Instant;
use tracing::{debug, warn};

impl ConversationCoordinator {
    pub async fn capture_fork_agent_context_snapshot(
        &self,
        parent_session_id: &str,
    ) -> NortHingResult<ForkAgentContextSnapshot> {
        let parent_session = self
            .session_manager
            .get_session(parent_session_id)
            .ok_or_else(|| NortHingError::NotFound(format!("Parent session not found: {}", parent_session_id)))?;
        let context_messages = self.load_session_context_messages(&parent_session).await?;
        ForkAgentContextSnapshot::from_parent_session(&parent_session, context_messages)
    }

    pub(crate) async fn ensure_hidden_btw_session(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        child_session_name: Option<&str>,
    ) -> NortHingResult<Session> {
        if let Some(session) = self.session_manager.get_session(child_session_id) {
            return Ok(session);
        }

        let snapshot = self.capture_fork_agent_context_snapshot(parent_session_id).await?;
        let session_name = child_session_name
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or("Side thread")
            .to_string();
        let child_session = self
            .session_manager
            .create_session_with_id_and_details(
                Some(child_session_id.to_string()),
                session_name,
                snapshot.parent_agent_type.clone(),
                snapshot.build_child_session_config(None),
                Some(format!("session-{}", snapshot.parent_session_id)),
                SessionKind::EphemeralChild,
            )
            .await?;

        let copied = self
            .session_manager
            .clone_prompt_cache(parent_session_id, &child_session.session_id)
            .await;
        debug!(
            "Forked prompt cache into /btw child session: parent_session_id={}, child_session_id={}, copied={}",
            parent_session_id, child_session.session_id, copied
        );
        self.session_manager
            .seed_forked_skill_agent_listing_baselines(parent_session_id, &child_session.session_id)
            .await;

        self.session_manager
            .replace_context_messages(&child_session.session_id, snapshot.messages)
            .await;

        Ok(child_session)
    }

    pub async fn start_hidden_btw_turn(
        &self,
        request_id: &str,
        parent_session_id: &str,
        child_session_id: &str,
        child_session_name: Option<&str>,
        question: &str,
        model_id: Option<&str>,
    ) -> NortHingResult<String> {
        if request_id.trim().is_empty() {
            return Err(NortHingError::Validation("request_id is required".to_string()));
        }
        if parent_session_id.trim().is_empty() {
            return Err(NortHingError::Validation("parent_session_id is required".to_string()));
        }
        if child_session_id.trim().is_empty() {
            return Err(NortHingError::Validation("child_session_id is required".to_string()));
        }
        if question.trim().is_empty() {
            return Err(NortHingError::Validation("question is required".to_string()));
        }

        let child_session = self
            .ensure_hidden_btw_session(parent_session_id, child_session_id, child_session_name)
            .await?;

        if let Some(model_id) = model_id.map(str::trim).filter(|model_id| !model_id.is_empty()) {
            self.session_manager
                .update_session_model_id(child_session_id, model_id)
                .await?;
        }

        let turn_id = format!("btw-turn-{}", request_id.trim());
        let user_message_metadata = Some(serde_json::json!({
            "kind": "btw",
            "parentSessionId": parent_session_id,
        }));

        let (user_input, prepended_messages) = build_btw_user_input(question);

        self.start_dialog_turn_internal(
            child_session_id.to_string(),
            user_input,
            Some(question.trim().to_string()),
            None,
            Some(turn_id.clone()),
            child_session.agent_type.clone(),
            child_session.config.workspace_path.clone(),
            DialogSubmissionPolicy::for_source(DialogTriggerSource::DesktopApi).with_skip_tool_confirmation(true),
            user_message_metadata,
            prepended_messages,
            true,
        )
        .await?;

        Ok(turn_id)
    }

    /// Clean up runtime-only subagent resources.
    ///
    /// Subagent sessions are now persisted so users can reopen them from the UI.
    /// This cleanup path must only release ephemeral runtime resources such as
    /// snapshot bookkeeping; it must not delete the persisted session itself.
    pub(crate) async fn cleanup_subagent_resources(&self, session_id: &str) -> NortHingResult<()> {
        let cleanup_started_at = Instant::now();
        debug!("Starting subagent resource cleanup: session_id={}", session_id);

        // Clean up snapshot system resources
        if let Some(workspace_path) = self
            .session_manager
            .get_session(session_id)
            .and_then(|session| session.config.workspace_path.map(std::path::PathBuf::from))
        {
            debug!(
                "Subagent cleanup stage starting: session_id={}, stage=snapshot_cleanup, workspace_path={}",
                session_id,
                workspace_path.display()
            );
            let stage_started_at = Instant::now();
            if let Ok(snapshot_manager) =
                crate::service::snapshot::ensure_snapshot_manager_for_workspace(&workspace_path)
            {
                let snapshot_service = snapshot_manager.snapshot_service();
                let snapshot_service = snapshot_service.read().await;
                if let Err(e) = snapshot_service.accept_session(session_id).await {
                    warn!(
                        "Failed to cleanup snapshot system resources: session={}, error={}",
                        session_id, e
                    );
                } else {
                    debug!("Snapshot system resources cleaned up: session={}", session_id);
                }
            }
            debug!(
                "Subagent cleanup stage completed: session_id={}, stage=snapshot_cleanup, duration_ms={}",
                session_id,
                stage_started_at.elapsed().as_millis()
            );
        }

        debug!(
            "Subagent resource cleanup completed: session_id={}, duration_ms={}",
            session_id,
            cleanup_started_at.elapsed().as_millis()
        );
        Ok(())
    }

    /// Generate session title
    ///
    /// Use AI to generate a concise and accurate session title based on user message content.
    /// Also persists the title to the session backend. Callers that go through
    /// `start_dialog_turn` do NOT need to call this separately — first-message
    /// title generation is handled automatically inside `start_dialog_turn`.
    pub async fn generate_session_title(
        &self,
        session_id: &str,
        user_message: &str,
        max_length: Option<usize>,
    ) -> NortHingResult<String> {
        let allow_ai = is_ai_session_title_generation_enabled().await;
        let resolved = self
            .session_manager
            .resolve_session_title(user_message, max_length, allow_ai)
            .await;

        self.session_manager
            .update_session_title(session_id, &resolved.title)
            .await?;

        let event = AgenticEvent::SessionTitleGenerated {
            session_id: session_id.to_string(),
            title: resolved.title.clone(),
            method: resolved.method.as_str().to_string(),
        };
        self.emit_event(event).await;

        debug!(
            "Session title generation event sent: session_id={}, title={}",
            session_id, resolved.title
        );

        Ok(resolved.title)
    }

    pub async fn update_session_title(&self, session_id: &str, title: &str) -> NortHingResult<String> {
        let normalized = title.trim().to_string();
        if normalized.is_empty() {
            return Err(NortHingError::validation("Session title must not be empty".to_string()));
        }

        self.session_manager
            .update_session_title(session_id, &normalized)
            .await?;

        Ok(normalized)
    }

    pub async fn update_session_agent_type(&self, session_id: &str, agent_type: &str) -> NortHingResult<()> {
        let normalized = Self::normalize_agent_type(agent_type);
        self.session_manager
            .update_session_agent_type(session_id, &normalized)
            .await
    }

    /// Update the session-level prompt-cache guard mode for the latest
    /// scheduler-accepted user submission.
    pub async fn update_last_submitted_agent_type(&self, session_id: &str, agent_type: &str) -> NortHingResult<()> {
        let normalized = Self::normalize_agent_type(agent_type);
        self.session_manager
            .update_last_submitted_agent_type(session_id, &normalized)
            .await
    }

    /// Emit event
    pub(crate) async fn emit_event(&self, event: AgenticEvent) {
        let _ = self.event_queue.enqueue(event, Some(EventPriority::Normal)).await;
    }

    /// Emit a `SessionModelAutoMigrated` event with `High` priority so the
    /// frontend can refresh its model selector and surface a notice promptly.
    ///
    /// Callers (e.g. `SessionManager`) reach this method via
    /// [`get_global_coordinator`] so they don't need to thread an
    /// `Arc<EventQueue>` through every constructor.
    pub async fn emit_session_model_auto_migrated(
        &self,
        session_id: &str,
        previous_model_id: &str,
        new_model_id: &str,
        reason: &str,
    ) {
        let event = AgenticEvent::SessionModelAutoMigrated {
            session_id: session_id.to_string(),
            previous_model_id: previous_model_id.to_string(),
            new_model_id: new_model_id.to_string(),
            reason: reason.to_string(),
        };
        let _ = self.event_queue.enqueue(event, Some(EventPriority::High)).await;
    }

    pub async fn emit_deep_review_queue_state_changed(
        &self,
        session_id: &str,
        turn_id: &str,
        queue_state: DeepReviewQueueState,
    ) {
        let event = AgenticEvent::DeepReviewQueueStateChanged {
            session_id: session_id.to_string(),
            turn_id: turn_id.to_string(),
            queue_state,
        };
        let _ = self.event_queue.enqueue(event, Some(EventPriority::High)).await;
    }

    /// Get SessionManager reference (for advanced features like mode management)
    pub fn session_manager(&self) -> &Arc<SessionManager> {
        &self.session_manager
    }

    /// Set global coordinator (called during initialization)
    ///
    /// Skips if global coordinator already exists
    pub fn set_global(coordinator: Arc<ConversationCoordinator>) {
        match GLOBAL_COORDINATOR.set(coordinator) {
            Ok(_) => {
                debug!("Global coordinator set");
            }
            Err(_) => {
                debug!("Global coordinator already exists, skipping set");
            }
        }
    }
}
