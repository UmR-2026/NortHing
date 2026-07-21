use super::super::coordinator::ConversationCoordinator;
use super::super::turn_outcome::TurnOutcome;
use super::scheduler_types::DialogScheduler;
use crate::agentic::core::{InternalReminderKind, Message, SessionState};
use crate::agentic::goal_mode::goal_continuation_submit_retry_delay_ms;
use crate::agentic::round_preempt::{DialogRoundInjectionSource, SessionRoundInjectionBuffer};
use crate::agentic::session::SessionManager;
use northhing_agent_runtime::scheduler::{
    resolve_agent_session_reply_action, resolve_turn_outcome_lifecycle_plan, ActiveDialogTurnStore,
    AgentSessionReplyAction, AgentSessionReplyPlan, DialogReplySuppressionSet, DialogTurnQueue,
    GoalContinuationAfterTurnAction, SessionAbortFlags, TurnOutcomeQueueAction, TurnOutcomeStatus,
};
use northhing_runtime_ports::{DialogSessionStateFact, MAX_THREAD_GOAL_AUTO_CONTINUATIONS};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

impl DialogScheduler {
    /// Create a new DialogScheduler and start its background outcome handler.
    ///
    /// The returned `Arc<DialogScheduler>` should be stored globally.
    /// Call `coordinator.set_scheduler_notifier(scheduler.outcome_sender())`
    /// immediately after to wire up the notification channel.
    pub fn new(coordinator: Arc<ConversationCoordinator>, session_manager: Arc<SessionManager>) -> Arc<Self> {
        let (outcome_tx, outcome_rx) = mpsc::channel(128);

        let scheduler = Arc::new(Self {
            coordinator,
            session_manager,
            queues: Arc::new(DialogTurnQueue::default()),
            active_turns: Arc::new(ActiveDialogTurnStore::default()),
            suppressed_cancelled_replies: Arc::new(DialogReplySuppressionSet::default()),
            goal_continuation_abort: Arc::new(SessionAbortFlags::default()),
            outcome_tx,
            round_injection_buffer: Arc::new(SessionRoundInjectionBuffer::default()),
        });

        let scheduler_for_handler = Arc::clone(&scheduler);
        tokio::spawn(async move {
            scheduler_for_handler.run_outcome_handler(outcome_rx).await;
        });

        scheduler
    }

    async fn run_outcome_handler(&self, mut outcome_rx: mpsc::Receiver<(String, TurnOutcome)>) {
        while let Some((session_id, outcome)) = outcome_rx.recv().await {
            let lifecycle_plan = resolve_turn_outcome_lifecycle_plan(&outcome, self.active_turns.contains(&session_id));

            // Only drop steering messages targeted at the *finished* turn. We
            // must NOT clear the entire session buffer here: a user might have
            // legitimately submitted steering against a brand-new follow-up
            // turn that the dispatcher will pick up immediately after this
            // outcome is processed (race window between turn finalize and the
            // next turn starting). Targeting by turn_id keeps those alive.
            if lifecycle_plan.drain_finished_turn_injections {
                let _drained = self
                    .round_injection_buffer
                    .drain_for_turn(&session_id, outcome.turn_id());
            }
            let suppressed_cancelled_reply = self.take_suppressed_cancelled_reply(&session_id, outcome.turn_id());

            let active_turn = self.active_turns.remove(&session_id);
            if let Some(active_turn) = active_turn.as_ref() {
                match resolve_agent_session_reply_action(&session_id, active_turn, &outcome, suppressed_cancelled_reply)
                {
                    AgentSessionReplyAction::NoReply => {}
                    AgentSessionReplyAction::SkipSuppressedCancelledReply => {
                        debug!(
                            "Skipping cancelled auto-reply because the source session explicitly cancelled its own SessionMessage request: session_id={}, turn_id={}",
                            session_id,
                            outcome.turn_id()
                        );
                    }
                    AgentSessionReplyAction::Forward(plan) => {
                        self.forward_agent_session_reply(&session_id, plan).await;
                    }
                }
            }

            let status = lifecycle_plan.status;
            let queue_action = lifecycle_plan.queue_action;
            if queue_action == TurnOutcomeQueueAction::ClearQueue {
                debug!("Turn {}, clearing queue: session_id={}", status, session_id);
                self.clear_queue(&session_id);
            }

            if let Some(active_turn) = active_turn.as_ref() {
                match lifecycle_plan.goal_continuation {
                    GoalContinuationAfterTurnAction::SkipNoActiveTurn => {}
                    GoalContinuationAfterTurnAction::AbortForCancelled => {
                        self.goal_continuation_abort.mark(&session_id);
                        debug!(
                            "Skipping thread goal continuation after user-cancelled turn: session_id={}, turn_id={}",
                            session_id,
                            outcome.turn_id()
                        );
                    }
                    GoalContinuationAfterTurnAction::Evaluate { turn_completed } => {
                        self.goal_continuation_abort.clear(&session_id);
                        match self
                            .coordinator
                            .prepare_goal_continuation_after_turn(
                                &session_id,
                                outcome.turn_id(),
                                active_turn.user_input(),
                                active_turn.user_message_metadata(),
                                turn_completed,
                            )
                            .await
                        {
                            Ok(Some(plan)) => {
                                let prepended: Vec<Message> = plan
                                    .prepended_reminders
                                    .into_iter()
                                    .map(|text| {
                                        Message::internal_reminder(InternalReminderKind::GoalContinuation, text)
                                    })
                                    .collect();
                                let mut last_error = None;
                                for attempt in 1..=MAX_THREAD_GOAL_AUTO_CONTINUATIONS {
                                    if self.goal_continuation_abort.contains(&session_id) {
                                        debug!(
                                            "Aborting goal continuation submit retries after user cancellation: session_id={}",
                                            session_id
                                        );
                                        break;
                                    }
                                    match self
                                        .submit_with_prepended_messages(
                                            session_id.clone(),
                                            "Continue working toward the active thread goal.".to_string(),
                                            Some(plan.display_message.clone()),
                                            None,
                                            active_turn.agent_type_owned(),
                                            active_turn.workspace_path_owned(),
                                            crate::agentic::coordination::DialogSubmissionPolicy::for_source(
                                                crate::agentic::coordination::DialogTriggerSource::AgentSession,
                                            ),
                                            None,
                                            Some(plan.user_message_metadata.clone()),
                                            prepended.clone(),
                                            None,
                                        )
                                        .await
                                    {
                                        Ok(_) => {
                                            last_error = None;
                                            break;
                                        }
                                        Err(error) => {
                                            last_error = Some(error);
                                            if self.goal_continuation_abort.contains(&session_id) {
                                                debug!(
                                                    "Aborting goal continuation submit retries after user cancellation: session_id={}",
                                                    session_id
                                                );
                                                break;
                                            }
                                            if attempt < MAX_THREAD_GOAL_AUTO_CONTINUATIONS {
                                                let delay_ms = goal_continuation_submit_retry_delay_ms(attempt);
                                                warn!(
                                                    "Goal continuation submit failed; retrying: session_id={}, attempt={}/{}, delay_ms={}, error={}",
                                                    session_id,
                                                    attempt,
                                                    MAX_THREAD_GOAL_AUTO_CONTINUATIONS,
                                                    delay_ms,
                                                    last_error.as_ref().unwrap()
                                                );
                                                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                                            }
                                        }
                                    }
                                }
                                if let Some(error) = last_error {
                                    if !self.goal_continuation_abort.contains(&session_id) {
                                        warn!(
                                            "Failed to submit goal continuation turn after retries: session_id={}, error={}",
                                            session_id, error
                                        );
                                    }
                                }
                            }
                            Ok(None) => {}
                            Err(error) => {
                                warn!(
                                    "Goal verification failed after turn stopped: session_id={}, status={}, error={}",
                                    session_id, status, error
                                );
                            }
                        }
                    }
                }
            }

            match queue_action {
                TurnOutcomeQueueAction::DispatchNext => {
                    if status == TurnOutcomeStatus::Cancelled {
                        debug!(
                            "Turn cancelled, dispatching next queued message if present: session_id={}",
                            session_id
                        );
                    }

                    if let Err(e) = self.dispatch_next_if_idle(&session_id).await {
                        warn!(
                            "Failed to dispatch next queued message after {}: session_id={}, error={}",
                            status, session_id, e
                        );
                    }
                }
                TurnOutcomeQueueAction::ClearQueue => {}
            }
        }
    }

    async fn dispatch_next_if_idle(&self, session_id: &str) -> Result<(), String> {
        let _ = self.try_start_next_queued(session_id).await?;
        Ok(())
    }

    pub(super) fn session_state_fact(state: Option<&SessionState>) -> DialogSessionStateFact {
        match state {
            None => DialogSessionStateFact::Missing,
            Some(SessionState::Idle) => DialogSessionStateFact::Idle,
            Some(SessionState::Processing { .. }) => DialogSessionStateFact::Processing,
            Some(SessionState::Error { .. }) => DialogSessionStateFact::Error,
        }
    }
}
