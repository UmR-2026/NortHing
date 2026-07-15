//! Scheduler owner decide / resolve functions.
//!
//! Pure functions that take scheduler facts in and return either an
//! action, a plan, or a constructed [`RoundInjection`]. These functions
//! never touch mutable state directly; the caller is expected to feed
//! them the current facts and act on the returned value.
//!
//! Sub-domain layout:
//! - `scheduler.rs` (facade)  — module wiring, `pub use` re-exports, tests.
//! - `sched_types.rs`         — data types + inherent impls.
//! - `sched_state.rs`         — state stores + injection sources.
//! - `sched_filter.rs`        — pure decide / resolve functions (this file).

use super::sched_types::{
    ActiveDialogTurn, AgentSessionReplyAction, AgentSessionReplyPlan, BackgroundDeliveryAction,
    BackgroundDeliveryFacts, BackgroundInjectionKind, DialogSteeringAction, GoalContinuationAfterTurnAction,
    ThreadGoalDeliveryPlan, ThreadGoalDeliveryReminder, ThreadGoalDeliveryReminderKind, TurnOutcome,
    TurnOutcomeLifecyclePlan, TurnOutcomeStatus,
};
use crate::events::turn_outcome_kind;
use crate::thread_goal::{build_objective_updated_plan, build_thread_goal_continuation_plan};
use northhing_runtime_ports::{
    should_skip_agent_session_reply, DialogSessionStateFact, DialogSteerOutcome, DialogSubmissionPolicy,
    DialogTriggerSource, RoundInjection, RoundInjectionKind, RoundInjectionTarget, ThreadGoal,
};
use std::time::SystemTime;

/// Decide whether a background delivery should be injected into the
/// currently running turn or queued as a new agent-session follow-up.
pub const fn resolve_background_delivery_action(facts: BackgroundDeliveryFacts) -> BackgroundDeliveryAction {
    match facts.session_state {
        DialogSessionStateFact::Processing => BackgroundDeliveryAction::InjectIntoRunningTurn,
        DialogSessionStateFact::Missing | DialogSessionStateFact::Idle | DialogSessionStateFact::Error => {
            let policy = DialogSubmissionPolicy::for_source(DialogTriggerSource::AgentSession);
            BackgroundDeliveryAction::SubmitAgentSessionFollowUp {
                queue_priority: policy.queue_priority,
                skip_tool_confirmation: policy.skip_tool_confirmation,
            }
        }
    }
}

/// Build the [`RoundInjection`] the concrete scheduler should buffer for a
/// background delivery of the given kind.
pub fn resolve_background_delivery_injection(
    kind: BackgroundInjectionKind,
    injection_id: String,
    content: String,
    display_content: Option<String>,
    created_at: SystemTime,
) -> RoundInjection {
    let display_content = display_content.unwrap_or_else(|| content.clone());
    RoundInjection {
        id: injection_id,
        kind: match kind {
            BackgroundInjectionKind::ThreadGoalObjectiveUpdated => RoundInjectionKind::ThreadGoalObjectiveUpdated,
            BackgroundInjectionKind::BackgroundResult => RoundInjectionKind::BackgroundResult,
        },
        target: RoundInjectionTarget::CurrentRunningTurn,
        content,
        display_content,
        created_at,
    }
}

/// Build a delivery plan that resumes work toward the supplied thread
/// goal. The plan is consumed by the concrete scheduler to materialise
/// the follow-up turn and any prepended reminders.
pub fn build_thread_goal_resumed_delivery_plan(goal: &ThreadGoal) -> ThreadGoalDeliveryPlan {
    let plan = build_thread_goal_continuation_plan(goal);
    let injection_prompt = plan.prepended_reminders.first().cloned().unwrap_or_default();
    let display_message = plan.display_message;
    ThreadGoalDeliveryPlan {
        injection_prompt,
        injection_display: display_message.clone(),
        display_message: display_message.clone(),
        follow_up_user_input: "Resume working toward the active thread goal.".to_string(),
        follow_up_original_user_input: Some(display_message),
        user_message_metadata: plan.user_message_metadata,
        prepended_reminders: plan
            .prepended_reminders
            .into_iter()
            .map(|content| ThreadGoalDeliveryReminder {
                kind: ThreadGoalDeliveryReminderKind::GoalContinuation,
                content,
            })
            .collect(),
    }
}

/// Build a delivery plan that adjusts work to match an updated thread
/// goal. The plan is consumed by the concrete scheduler to materialise
/// the follow-up turn and any prepended reminders.
pub fn build_thread_goal_objective_updated_delivery_plan(goal: &ThreadGoal) -> ThreadGoalDeliveryPlan {
    let plan = build_objective_updated_plan(goal);
    let injection_prompt = plan.prepended_reminders.first().cloned().unwrap_or_default();
    let display_message = plan.display_message;
    ThreadGoalDeliveryPlan {
        injection_prompt,
        injection_display: display_message.clone(),
        display_message: display_message.clone(),
        follow_up_user_input: "Adjust work to match the updated thread goal.".to_string(),
        follow_up_original_user_input: Some(display_message),
        user_message_metadata: plan.user_message_metadata,
        prepended_reminders: plan
            .prepended_reminders
            .into_iter()
            .map(|content| ThreadGoalDeliveryReminder {
                kind: ThreadGoalDeliveryReminderKind::GoalObjectiveUpdated,
                content,
            })
            .collect(),
    }
}

/// Aggregate all post-turn decisions the concrete scheduler needs to act
/// on: the queue side-effect, whether to drain finished-turn injections,
/// and what the thread-goal continuation loop should do.
pub fn resolve_turn_outcome_lifecycle_plan(outcome: &TurnOutcome, has_active_turn: bool) -> TurnOutcomeLifecyclePlan {
    let status = outcome.status();
    let goal_continuation = if !has_active_turn {
        GoalContinuationAfterTurnAction::SkipNoActiveTurn
    } else {
        match status {
            TurnOutcomeStatus::Cancelled => GoalContinuationAfterTurnAction::AbortForCancelled,
            TurnOutcomeStatus::Completed => GoalContinuationAfterTurnAction::Evaluate { turn_completed: true },
            TurnOutcomeStatus::Failed => GoalContinuationAfterTurnAction::Evaluate { turn_completed: false },
        }
    };

    TurnOutcomeLifecyclePlan {
        status,
        queue_action: outcome.queue_action(),
        drain_finished_turn_injections: true,
        goal_continuation,
    }
}

/// Decide whether — and how — to forward an automated reply back to the
/// requester session for a finished agent-session turn.
pub fn resolve_agent_session_reply_action(
    responder_session_id: &str,
    active_turn: &ActiveDialogTurn,
    outcome: &TurnOutcome,
    suppressed_cancelled_reply: bool,
) -> AgentSessionReplyAction {
    if !active_turn.is_agent_session_request() {
        return AgentSessionReplyAction::NoReply;
    }

    if should_skip_agent_session_reply(turn_outcome_kind(outcome), suppressed_cancelled_reply) {
        return AgentSessionReplyAction::SkipSuppressedCancelledReply;
    }

    let Some(reply_route) = active_turn.reply_route() else {
        return AgentSessionReplyAction::NoReply;
    };

    let responder_workspace = active_turn.workspace_path().unwrap_or("<unknown workspace>");
    let status = outcome.status();
    AgentSessionReplyAction::Forward(AgentSessionReplyPlan {
        target_session_id: reply_route.source_session_id.clone(),
        target_workspace_path: reply_route.source_workspace_path.clone(),
        user_input: outcome.reply_text(),
        reminder_text: format!(
            "This message is an automated reply to a previous SessionMessage call, not a human user message.\n\
             From session: {responder_session_id}\n\
             From workspace: {responder_workspace}\n\
             Status: {status}"
        ),
    })
}

/// Decide whether an inbound steering request should be buffered for the
/// currently running turn or rejected because the turn is no longer
/// running.
pub fn resolve_dialog_steering_action(
    active_turn_id: Option<&str>,
    session_id: &str,
    turn_id: &str,
    content: String,
    display_content: Option<String>,
    steering_id: String,
    created_at: SystemTime,
) -> DialogSteeringAction {
    if active_turn_id != Some(turn_id) {
        return DialogSteeringAction::Reject {
            error: format!(
                "Dialog turn is no longer running and cannot be steered: session_id={session_id}, turn_id={turn_id}"
            ),
        };
    }

    let display = display_content.unwrap_or_else(|| content.clone());
    DialogSteeringAction::Buffer {
        injection: RoundInjection {
            id: steering_id.clone(),
            kind: RoundInjectionKind::UserSteering,
            target: RoundInjectionTarget::ExactTurn(turn_id.to_string()),
            content,
            display_content: display,
            created_at,
        },
        outcome: DialogSteerOutcome::Buffered {
            session_id: session_id.to_string(),
            turn_id: turn_id.to_string(),
            steering_id,
        },
    }
}
