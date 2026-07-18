//! Sub-domain: turn_subhandlers output/finalization phase (R47b refactor).
//!
//! Owns `finalize_turn` and `cleanup_turn` — the third and fourth of the 4
//! sub-handler phases. Builds the `ExecutionContext`, registers the
//! `ActiveTurnRegistration` (RAII counter guard) and `SessionExecutionGuard`
//! (RAII state reset guard), emits `DialogTurnStarted`, optionally spawns the
//! session-title generator, then spawns the actual execution task that
//! calls `execute_dialog_turn` and persists the outcome.
//!
//! Spec §2.1 R47b — extracted from `turn_subhandlers.rs` god-file.
//! Sibling imports `use super::super::coordinator::*` for the struct and
//! `use super::super::scheduler::DialogSubmissionPolicy` for the policy type.

use super::super::coordinator::*;
use super::super::ports::*;
use super::super::scheduler::*;
use super::super::scheduler::{
    abort_thread_goal_continuation_for_session, clear_thread_goal_continuation_abort, DialogSubmissionPolicy,
};

use super::sub_handle_types::TurnContext;

use crate::agentic::core::{ProcessingPhase, SessionState};
use crate::agentic::events::{AgenticEvent, EventPriority};
use crate::agentic::execution::ExecutionContext;
use crate::agentic::remote_file_delivery::needs_computer_links_for_source;
use crate::agentic::session::SessionManager;
use crate::agentic::tools::{
    is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
};
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_runtime_ports::DelegationPolicy;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{debug, error, warn};

use tokio_util::sync::CancellationToken;

// 2026-07-18 (W3a-3): Turn watchdog timeout. Reads `NORTHHING_TURN_WATCHDOG_SECS`
// env var; defaults to 600s. This is a safety net for the interactive desktop
// agent — normal turns complete in seconds to minutes, so 600s is generous
// enough to avoid false positives while still catching stuck turns (e.g. a
// turn task blocked in an uninterruptible await).
fn turn_watchdog_timeout() -> Duration {
    std::env::var("NORTHHING_TURN_WATCHDOG_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(600))
}

impl ConversationCoordinator {
    pub(super) async fn finalize_turn(&self, ctx: &mut TurnContext) -> NortHingResult<()> {
        let session_id = ctx.session_id.clone();
        let original_user_input = ctx
            .original_user_input
            .clone()
            .unwrap_or_else(|| ctx.user_input.clone());
        let user_message_metadata = ctx.user_message_metadata.clone();
        let session_workspace = ctx.session_workspace.clone();
        let workspace_services = ctx.workspace_services.clone();
        let effective_user_input = ctx.effective_user_input.clone();
        let effective_agent_type = ctx.effective_agent_type.clone();
        let turn_index = ctx.turn_index;
        let mut turn_id = ctx.final_turn_id.clone();
        let session = ctx.session.clone().expect("prepare_turn must set ctx.session first");
        let submission_policy = ctx.submission_policy.clone();
        let suppress_session_title_generation = ctx.suppress_session_title_generation;
        let active_counter = Arc::new(AtomicUsize::new(0));
        let active_counter = self
            .active_turns_per_session
            .entry(session_id.clone())
            .or_insert_with(|| Arc::new(AtomicUsize::new(0)))
            .clone();
        active_counter.fetch_add(1, Ordering::SeqCst);
        struct ActiveTurnRegistration {
            counter: Arc<AtomicUsize>,
            armed: bool,
        }
        impl ActiveTurnRegistration {
            fn disarm(&mut self) {
                self.armed = false;
            }
        }
        impl Drop for ActiveTurnRegistration {
            fn drop(&mut self) {
                if self.armed {
                    self.counter.fetch_sub(1, Ordering::SeqCst);
                }
            }
        }
        let mut active_registration = ActiveTurnRegistration {
            counter: active_counter.clone(),
            armed: true,
        };
        let cancellation_token = CancellationToken::new();
        self.execution_engine
            .register_cancel_token(&turn_id, cancellation_token);
        self.emit_event(AgenticEvent::DialogTurnStarted {
            session_id: session_id.clone(),
            turn_id: turn_id.clone(),
            turn_index,
            user_input: effective_user_input.clone(),
            original_user_input: if original_user_input != effective_user_input {
                Some(original_user_input.clone())
            } else {
                None
            },
            user_message_metadata: user_message_metadata.clone(),
        })
        .await;
        let messages = match self.session_manager.get_context_messages(&session_id).await {
            Ok(messages) => messages,
            Err(error) => {
                self.execution_engine.cleanup_cancel_token(&turn_id).await;
                return Err(error);
            }
        };
        let mut context_vars = std::collections::HashMap::new();
        context_vars.insert(
            "max_context_tokens".to_string(),
            session.config.max_context_tokens.to_string(),
        );
        context_vars.insert("enable_tools".to_string(), session.config.enable_tools.to_string());
        context_vars.insert("original_user_input".to_string(), original_user_input.clone());
        if let Some(model_id) = &session.config.model_id {
            context_vars.insert("model_name".to_string(), model_id.clone());
        }
        if let Some(snapshot_id) = &session.snapshot_session_id {
            context_vars.insert("snapshot_session_id".to_string(), snapshot_id.clone());
        }
        context_vars.insert("turn_index".to_string(), turn_index.to_string());
        if let Some(run_manifest) = user_message_metadata.as_ref().and_then(|metadata| {
            metadata
                .get("deepReviewRunManifest")
                .or_else(|| metadata.get("deep_review_run_manifest"))
        }) {
            context_vars.insert("deep_review_run_manifest".to_string(), run_manifest.to_string());
        }
        if user_message_metadata
            .as_ref()
            .and_then(|metadata| metadata.get("acp_transport"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            context_vars.insert("acp_transport".to_string(), "true".to_string());
        }
        if needs_computer_links_for_source(submission_policy.trigger_source) {
            context_vars.insert(
                crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY.to_string(),
                "true".to_string(),
            );
        }
        let session_workspace_path = session_workspace.as_ref().map(|workspace| workspace.root_path_string());
        let session_storage_path = session_workspace
            .as_ref()
            .map(|workspace| workspace.session_storage_path().to_path_buf());
        let runtime_tool_restrictions =
            if is_miniapp_headless_agent_run(user_message_metadata.as_ref(), session.created_by.as_deref()) {
                miniapp_headless_agent_tool_restrictions()
            } else {
                ToolRuntimeRestrictions::default()
            };
        let execution_context = ExecutionContext {
            session_id: session_id.clone(),
            dialog_turn_id: turn_id.clone(),
            turn_index,
            agent_type: effective_agent_type.clone(),
            workspace: session_workspace,
            context: context_vars,
            subagent_parent_info: None,
            delegation_policy: DelegationPolicy::top_level(),
            skip_tool_confirmation: submission_policy.skip_tool_confirmation,
            runtime_tool_restrictions,
            workspace_services,
            round_injection: self.round_injection_source.get().cloned(),
            recover_partial_on_cancel: false,
        };
        if turn_index == 0 && !suppress_session_title_generation {
            let sm = self.session_manager.clone();
            let eq = self.event_queue.clone();
            let sid = session_id.clone();
            let msg = original_user_input;
            let expected_title = self
                .session_manager
                .get_session(&session_id)
                .map(|session| session.session_name)
                .unwrap_or_default();
            tokio::spawn(async move {
                let allow_ai = is_ai_session_title_generation_enabled().await;
                let resolved = sm.resolve_session_title(&msg, Some(20), allow_ai).await;
                match sm
                    .update_session_title_if_current(&sid, &expected_title, &resolved.title)
                    .await
                {
                    Ok(true) => {
                        let _ = eq
                            .enqueue(
                                AgenticEvent::SessionTitleGenerated {
                                    session_id: sid,
                                    title: resolved.title,
                                    method: resolved.method.as_str().to_string(),
                                },
                                Some(EventPriority::Normal),
                            )
                            .await;
                    }
                    Ok(false) => {
                        debug!("Skipped auto session title update because title changed");
                    }
                    Err(error) => {
                        debug!("Auto session title generation failed to apply: {error}");
                    }
                }
            });
        }
        let session_manager = self.session_manager.clone();
        let execution_engine = self.execution_engine.clone();
        let event_queue = self.event_queue.clone();
        let session_id_clone = session_id.clone();
        let turn_id_clone = turn_id.clone();
        let user_input_for_workspace = effective_user_input.clone();
        let session_storage_path_for_finalize = session_storage_path.clone();
        let effective_agent_type_clone = effective_agent_type.clone();
        let user_message_metadata_clone = user_message_metadata;
        let scheduler_notify_tx = self.scheduler_notify_tx.get().cloned();
        tokio::spawn(async move {
            struct SessionExecutionGuard {
                session_manager: Arc<SessionManager>,
                session_id: String,
                turn_id: String,
                active_counter: Arc<AtomicUsize>,
            }
            impl SessionExecutionGuard {
                fn new(
                    session_manager: Arc<SessionManager>,
                    session_id: String,
                    turn_id: String,
                    active_counter: Arc<AtomicUsize>,
                ) -> Self {
                    Self {
                        session_manager,
                        session_id,
                        turn_id,
                        active_counter,
                    }
                }
            }
            impl Drop for SessionExecutionGuard {
                fn drop(&mut self) {
                    self.active_counter.fetch_sub(1, Ordering::SeqCst);
                    self.session_manager
                        .reset_session_state_if_processing(&self.session_id, &self.turn_id);
                }
            }
            let _guard = SessionExecutionGuard::new(
                session_manager.clone(),
                session_id_clone.clone(),
                turn_id_clone.clone(),
                active_counter,
            );
            match session_manager
                .update_session_state_for_turn_if_processing(
                    &session_id_clone,
                    &turn_id_clone,
                    SessionState::Processing {
                        current_turn_id: turn_id_clone.clone(),
                        phase: ProcessingPhase::Thinking,
                    },
                )
                .await
            {
                Ok(true) => {}
                Ok(false) => {
                    debug!(
                        "Skipped refreshing Processing state for stale or cancelled turn: session_id={}, turn_id={}",
                        session_id_clone, turn_id_clone
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to set session state to Processing: session_id={}, turn_id={}, error={}",
                        session_id_clone, turn_id_clone, e
                    );
                }
            }
            let workspace_turn_status = match execution_engine
                .execute_dialog_turn(effective_agent_type_clone.clone(), messages, execution_context)
                .await
            {
                Ok(execution_result) => Some(
                    Self::persist_completed_dialog_turn(
                        session_manager.as_ref(),
                        scheduler_notify_tx.as_ref(),
                        &session_id_clone,
                        &turn_id_clone,
                        &execution_result,
                    )
                    .await
                    .0,
                ),
                Err(e) => {
                    if matches!(&e, NortHingError::Cancelled(_)) {
                        Some(
                            Self::persist_cancelled_dialog_turn(
                                event_queue.as_ref(),
                                session_manager.as_ref(),
                                scheduler_notify_tx.as_ref(),
                                &session_id_clone,
                                &turn_id_clone,
                            )
                            .await,
                        )
                    } else {
                        Some(
                            Self::persist_failed_dialog_turn(
                                event_queue.as_ref(),
                                session_manager.as_ref(),
                                scheduler_notify_tx.as_ref(),
                                &session_id_clone,
                                &turn_id_clone,
                                &e,
                            )
                            .await,
                        )
                    }
                }
            };
            Self::finalize_persisted_turn_in_workspace_if_needed(
                session_manager.as_ref(),
                &session_id_clone,
                &turn_id_clone,
                turn_index,
                &effective_agent_type_clone,
                &user_input_for_workspace,
                session_workspace_path.as_deref(),
                session_storage_path_for_finalize.as_deref(),
                workspace_turn_status,
                user_message_metadata_clone,
            )
            .await;
        });
        // 2026-07-18 (W3a-3): Turn watchdog. After the timeout, if this turn
        // is still the active processing turn for the session, trigger
        // cancellation via the coordinator. The cancel path includes the
        // convergence fallback (persist_cancelled_dialog_turn) so the UI
        // converges to Idle even if the turn task is stuck in an
        // uninterruptible await. State check is by session_manager
        // (Processing + current_turn_id match) to avoid cancelling a newer
        // turn that replaced this one.
        let session_manager_for_watchdog = self.session_manager.clone();
        let session_id_for_watchdog = session_id.clone();
        let turn_id_for_watchdog = turn_id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(turn_watchdog_timeout()).await;
            let still_active = session_manager_for_watchdog
                .get_session(&session_id_for_watchdog)
                .map(|session| {
                    matches!(
                        &session.state,
                        SessionState::Processing { current_turn_id, .. }
                            if current_turn_id == &turn_id_for_watchdog
                    )
                })
                .unwrap_or(false);
            if still_active {
                warn!(
                    "Turn watchdog timeout: session_id={}, turn_id={}, timeout_secs={}",
                    session_id_for_watchdog,
                    turn_id_for_watchdog,
                    turn_watchdog_timeout().as_secs()
                );
                if let Some(coordinator) = global_coordinator() {
                    if let Err(error) = coordinator
                        .cancel_dialog_turn(&session_id_for_watchdog, &turn_id_for_watchdog)
                        .await
                    {
                        warn!(
                            "Watchdog cancel failed: session_id={}, turn_id={}, error={}",
                            session_id_for_watchdog, turn_id_for_watchdog, error
                        );
                    }
                }
            }
        });
        active_registration.disarm();
        Ok(())
    }

    pub(super) async fn cleanup_turn(&self, _ctx: &mut TurnContext) -> NortHingResult<()> {
        Ok(())
    }
}
