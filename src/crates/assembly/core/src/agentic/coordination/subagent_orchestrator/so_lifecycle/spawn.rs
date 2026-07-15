//! Sub-domain: subagent spawn — session creation, concurrency, timeout registration.
//! Spec step-3.7 — extracted from so_lifecycle.rs (R54a refactor).

use super::super::super::coordinator::*;
use crate::agentic::core::Message;
use crate::agentic::events::AgenticEvent;
use crate::agentic::goal_mode::effective_subagent_timeout_seconds;
use crate::util::errors::{NortHingError, NortHingResult};
use std::sync::Arc;
use tokio::sync::watch;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

impl ConversationCoordinator {
    /// Phase 1: derive metadata, acquire concurrency permit, create hidden subagent
    /// session, link to parent, register timeout handle.
    pub(crate) async fn execute_hidden_subagent_phase1(
        &self,
        request: HiddenSubagentExecutionRequest,
        cancel_token: Option<&CancellationToken>,
        timeout_seconds: Option<u64>,
    ) -> NortHingResult<SubagentPhase1Output> {
        let HiddenSubagentExecutionRequest {
            session_name,
            agent_type,
            session_config,
            initial_messages,
            user_input_text,
            created_by,
            subagent_parent_info,
            context,
            delegation_policy,
            runtime_tool_restrictions,
            prompt_cache_source_session_id,
        } = request;

        let requested_timeout_seconds = timeout_seconds.filter(|seconds| *seconds > 0);
        let parent_thread_goal_active = if let Some(parent_info) = subagent_parent_info.as_ref() {
            matches!(self.load_active_thread_goal(&parent_info.session_id).await, Ok(Some(_)))
        } else {
            false
        };
        if parent_thread_goal_active {
            let parent_session_id = subagent_parent_info
                .as_ref()
                .map(|info| info.session_id.as_str())
                .unwrap_or("-");
            debug!(
                "Subagent timeout disabled by default for active goal mode: agent_type={}, parent_session_id={}",
                agent_type, parent_session_id
            );
        }
        let timeout_seconds = effective_subagent_timeout_seconds(requested_timeout_seconds, parent_thread_goal_active);
        let timeout_error_message = match timeout_seconds.or(requested_timeout_seconds) {
            Some(seconds) => format!("Subagent '{}' timed out after {} seconds", agent_type, seconds),
            None => format!("Subagent '{}' timed out", agent_type),
        };

        // Create dynamic deadline via watch channel so it can be adjusted at runtime.
        let initial_deadline = timeout_seconds.map(|seconds| Instant::now() + Duration::from_secs(seconds));
        let (deadline_tx, deadline_rx) = watch::channel(initial_deadline);
        let subagent_started_at = Instant::now();
        // Clone subagent_parent_info before using .as_ref() for string extraction
        // so the original can still be moved into the struct later.
        let subagent_parent_info_borrow = subagent_parent_info.clone();
        let parent_session_id = subagent_parent_info_borrow
            .as_ref()
            .map(|info| info.session_id.as_str())
            .unwrap_or("-");
        let parent_dialog_turn_id = subagent_parent_info_borrow
            .as_ref()
            .map(|info| info.dialog_turn_id.as_str())
            .unwrap_or("-");
        let parent_tool_call_id = subagent_parent_info_borrow
            .as_ref()
            .map(|info| info.tool_call_id.as_str())
            .unwrap_or("-");

        let context_profile_policy = self.context_profile_policy_for_subagent(
            &agent_type,
            &session_config,
            subagent_parent_info_borrow.as_ref(),
        );
        debug!(
            "Subagent context profile policy selected: agent_type={}, profile={:?}, profile_concurrency_cap={}",
            agent_type, context_profile_policy.profile, context_profile_policy.subagent_concurrency_cap
        );

        // Check cancel token (before creating session)
        if let Some(token) = cancel_token {
            if token.is_cancelled() {
                debug!("Subagent task cancelled before execution");
                return Err(NortHingError::Cancelled("Subagent task has been cancelled".to_string()));
            }
        }

        // Create independent subagent session.
        let (permits, wait_ms) = self
            .acquire_subagent_concurrency_permit(
                &agent_type,
                context_profile_policy.subagent_concurrency_cap,
                cancel_token,
                initial_deadline,
            )
            .await?;
        let _permit_guard = SubagentConcurrencyPermitGuard::new(permits, agent_type.clone());

        if let Some(token) = cancel_token {
            if token.is_cancelled() {
                debug!(
                    "Subagent task cancelled after waiting for concurrency slot: agent_type={}",
                    agent_type
                );
                return Err(NortHingError::Cancelled("Subagent task has been cancelled".to_string()));
            }
        }
        if initial_deadline.is_some_and(|expires_at| Instant::now() >= expires_at) {
            warn!(
                "Subagent timed out before session creation after waiting for concurrency slot: agent_type={}, wait_ms={}",
                agent_type, wait_ms
            );
            return Err(NortHingError::Timeout(timeout_error_message.clone()));
        }

        let session = self
            .create_hidden_subagent_session(None, session_name, agent_type.clone(), session_config, created_by)
            .await?;
        let session_id = session.session_id.clone();
        self.session_manager.refresh_session_context_window(&session_id).await?;
        if let Some(source_session_id) = prompt_cache_source_session_id.as_deref() {
            let copied = self
                .session_manager
                .clone_prompt_cache(source_session_id, &session_id)
                .await;
            debug!(
                "Forked prompt cache into subagent session: source_session_id={}, session_id={}, copied={}",
                source_session_id, session_id, copied
            );
            self.session_manager
                .seed_forked_skill_agent_listing_baselines(source_session_id, &session_id)
                .await;
        }
        self.session_manager
            .replace_context_messages(&session_id, initial_messages.clone())
            .await;
        self.session_manager
            .persist_session_lineage(
                &session_id,
                super::super::so_types::build_subagent_session_relationship(subagent_parent_info.as_ref(), &agent_type),
            )
            .await?;

        if let Some(parent_info) = subagent_parent_info.as_ref() {
            self.emit_event(AgenticEvent::SubagentSessionLinked {
                session_id: session_id.clone(),
                parent_session_id: parent_info.session_id.clone(),
                parent_dialog_turn_id: parent_info.dialog_turn_id.clone(),
                parent_tool_call_id: parent_info.tool_call_id.clone(),
                agent_type: Some(agent_type.clone()),
            })
            .await;
        }

        // Register timeout handle so it can be adjusted at runtime.
        let timeout_handle = Arc::new(SubagentTimeoutHandle {
            deadline_tx: deadline_tx.clone(),
            session_id: session_id.clone(),
            original_timeout_seconds: requested_timeout_seconds,
            remaining_at_pause: std::sync::Mutex::new(None),
        });
        {
            let mut registry = self.subagent_timeout_registry.write().await;
            registry.insert(session_id.clone(), timeout_handle);
        }

        // Check cancel token (after creating session, before execution)
        if let Some(token) = cancel_token {
            if token.is_cancelled() {
                debug!("Subagent task cancelled before AI call, cleaning up resources");
                let _ = self.cleanup_subagent_resources(&session_id).await;
                let mut registry = self.subagent_timeout_registry.write().await;
                registry.remove(&session_id);
                return Err(NortHingError::Cancelled("Subagent task has been cancelled".to_string()));
            }
        }
        if initial_deadline.is_some_and(|expires_at| Instant::now() >= expires_at) {
            warn!(
                "Subagent timed out before AI call after session creation: agent_type={}, session={}, wait_ms={}",
                agent_type, session_id, wait_ms
            );
            let _ = self.cleanup_subagent_resources(&session_id).await;
            let mut registry = self.subagent_timeout_registry.write().await;
            registry.remove(&session_id);
            return Err(NortHingError::Timeout(timeout_error_message.clone()));
        }

        Ok(SubagentPhase1Output {
            agent_type,
            session_id,
            initial_messages,
            user_input_text,
            subagent_parent_info,
            context,
            delegation_policy,
            runtime_tool_restrictions,
            turn_index: self.session_manager.get_turn_count(&session.session_id),
            dialog_turn_id: format!("subagent-{}", uuid::Uuid::new_v4()),
            subagent_cancel_token: cancel_token.map(CancellationToken::child_token).unwrap_or_default(),
            deadline_rx,
            requested_timeout_seconds,
            timeout_seconds,
            timeout_error_message,
            parent_session_id: parent_session_id.to_string(),
            parent_dialog_turn_id: parent_dialog_turn_id.to_string(),
            parent_tool_call_id: parent_tool_call_id.to_string(),
            subagent_workspace: Self::build_workspace_binding(&session.config).await,
            subagent_started_at,
        })
    }
}
