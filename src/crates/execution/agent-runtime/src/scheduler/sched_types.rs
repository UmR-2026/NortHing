//! Scheduler owner data types.
//!
//! Pure data structures, enums, and their inherent impls that the
//! scheduler owner uses to describe the per-session dialog state, turn
//! outcomes, delivery plans, and steering actions. This file owns the
//! "shape" of the scheduler API surface; behaviour lives in the sibling
//! modules.
//!
//! Sub-domain layout:
//! - `scheduler.rs` (facade)  — module wiring, `pub use` re-exports, tests.
//! - `sched_types.rs`         — data types + inherent impls (this file).
//! - `sched_state.rs`         — per-session state stores and round-injection
//!                              sources (DashMap-backed, mutable).
//! - `sched_filter.rs`        — pure decide / resolve functions that take
//!                              facts in and return an action / plan out.
//!
//! All public items are re-exported from the facade so existing
//! `northhing_agent_runtime::scheduler::Item` import paths keep working.

use northhing_runtime_ports::{
    should_suppress_agent_session_cancelled_reply, AgentSessionReplyRoute, DialogQueuePriority, DialogSessionStateFact,
    DialogSteerOutcome, DialogSubmissionPolicy, DialogTriggerSource, RoundInjection,
};
use std::fmt;

/// Default cap on per-session dialog-turn queue depth.
pub const DEFAULT_MAX_DIALOG_QUEUE_DEPTH: usize = 20;

/// In-flight dialog turn bound to a session. Carries the submission policy,
/// reply routing, and user input metadata that downstream filter / resolve
/// functions inspect.
#[derive(Debug, Clone)]
pub struct ActiveDialogTurn {
    turn_id: String,
    workspace_path: Option<String>,
    agent_type: String,
    user_input: String,
    user_message_metadata: Option<serde_json::Value>,
    policy: DialogSubmissionPolicy,
    reply_route: Option<AgentSessionReplyRoute>,
}

impl ActiveDialogTurn {
    pub fn new(
        turn_id: String,
        workspace_path: Option<String>,
        agent_type: String,
        user_input: String,
        user_message_metadata: Option<serde_json::Value>,
        policy: DialogSubmissionPolicy,
        reply_route: Option<AgentSessionReplyRoute>,
    ) -> Self {
        Self {
            turn_id,
            workspace_path,
            agent_type,
            user_input,
            user_message_metadata,
            policy,
            reply_route,
        }
    }

    pub fn turn_id(&self) -> &str {
        &self.turn_id
    }

    pub fn workspace_path(&self) -> Option<&str> {
        self.workspace_path.as_deref()
    }

    pub fn workspace_path_owned(&self) -> Option<String> {
        self.workspace_path.clone()
    }

    pub fn agent_type(&self) -> &str {
        &self.agent_type
    }

    pub fn agent_type_owned(&self) -> String {
        self.agent_type.clone()
    }

    pub fn user_input(&self) -> &str {
        &self.user_input
    }

    pub fn user_message_metadata(&self) -> Option<&serde_json::Value> {
        self.user_message_metadata.as_ref()
    }

    pub fn reply_route(&self) -> Option<&AgentSessionReplyRoute> {
        self.reply_route.as_ref()
    }

    pub fn is_agent_session_request(&self) -> bool {
        self.policy.trigger_source == DialogTriggerSource::AgentSession && self.reply_route.is_some()
    }

    pub fn should_suppress_cancelled_reply_for_requester(&self, requester_session_id: &str) -> bool {
        should_suppress_agent_session_cancelled_reply(
            &self.policy,
            self.reply_route
                .as_ref()
                .map(|reply_route| reply_route.source_session_id.as_str()),
            requester_session_id,
        )
    }
}

/// Error returned when a [`DialogTurnQueue`] rejects an enqueue because the
/// per-session depth cap was hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogTurnQueueError {
    Full { session_id: String, max_depth: usize },
}

impl fmt::Display for DialogTurnQueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full { session_id, max_depth } => write!(
                f,
                "Message queue full for session {session_id} (max {max_depth} messages)"
            ),
        }
    }
}

impl std::error::Error for DialogTurnQueueError {}

/// Plan for forwarding an automated reply into the requesting session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionReplyPlan {
    pub target_session_id: String,
    pub target_workspace_path: String,
    pub user_input: String,
    pub reminder_text: String,
}

/// Owner decision for an in-flight agent-session reply request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSessionReplyAction {
    NoReply,
    SkipSuppressedCancelledReply,
    Forward(AgentSessionReplyPlan),
}

/// Outcome of attempting to steer a running dialog turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogSteeringAction {
    Reject {
        error: String,
    },
    Buffer {
        injection: RoundInjection,
        outcome: DialogSteerOutcome,
    },
}

/// Input facts for [`crate::scheduler::resolve_background_delivery_action`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundDeliveryFacts {
    pub session_state: DialogSessionStateFact,
}

/// Decision returned by [`crate::scheduler::resolve_background_delivery_action`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundDeliveryAction {
    InjectIntoRunningTurn,
    SubmitAgentSessionFollowUp {
        queue_priority: DialogQueuePriority,
        skip_tool_confirmation: bool,
    },
}

/// Distinguishes the two background injection sources currently routed
/// through the scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundInjectionKind {
    ThreadGoalObjectiveUpdated,
    BackgroundResult,
}

/// Tag carried by every [`ThreadGoalDeliveryReminder`] so the receiver can
/// pick a presentation path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadGoalDeliveryReminderKind {
    GoalContinuation,
    GoalObjectiveUpdated,
}

/// One user-visible reminder that should be prepended to a thread-goal
/// delivery follow-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadGoalDeliveryReminder {
    pub kind: ThreadGoalDeliveryReminderKind,
    pub content: String,
}

/// Fully-shaped plan that the concrete scheduler should hand to the dialog
/// runner when resuming or updating a thread goal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadGoalDeliveryPlan {
    pub injection_prompt: String,
    pub injection_display: String,
    pub display_message: String,
    pub follow_up_user_input: String,
    pub follow_up_original_user_input: Option<String>,
    pub user_message_metadata: serde_json::Value,
    pub prepended_reminders: Vec<ThreadGoalDeliveryReminder>,
}

impl BackgroundDeliveryAction {
    /// Submission policy the runner should use when this action materializes
    /// as a new dialog turn. `None` for actions that piggy-back on the
    /// currently running turn.
    pub const fn follow_up_submission_policy(self) -> Option<DialogSubmissionPolicy> {
        match self {
            Self::InjectIntoRunningTurn => None,
            Self::SubmitAgentSessionFollowUp {
                queue_priority,
                skip_tool_confirmation,
            } => Some(DialogSubmissionPolicy::new(
                DialogTriggerSource::AgentSession,
                queue_priority,
                skip_tool_confirmation,
            )),
        }
    }
}

/// Outcome of a completed dialog turn, used to notify the concrete scheduler.
#[derive(Debug, Clone)]
pub enum TurnOutcome {
    /// Turn completed normally.
    Completed { turn_id: String, final_response: String },
    /// Turn was cancelled by user.
    Cancelled { turn_id: String },
    /// Turn failed with an error.
    Failed { turn_id: String, error: String },
}

/// Queue-side action the concrete scheduler should take after a turn
/// outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOutcomeQueueAction {
    DispatchNext,
    ClearQueue,
}

/// Stable tag for a [`TurnOutcome`] used in events and logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOutcomeStatus {
    Completed,
    Cancelled,
    Failed,
}

impl TurnOutcomeStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}

impl fmt::Display for TurnOutcomeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TurnOutcome {
    pub fn turn_id(&self) -> &str {
        match self {
            Self::Completed { turn_id, .. } | Self::Cancelled { turn_id } | Self::Failed { turn_id, .. } => turn_id,
        }
    }

    pub fn status(&self) -> TurnOutcomeStatus {
        match self {
            Self::Completed { .. } => TurnOutcomeStatus::Completed,
            Self::Cancelled { .. } => TurnOutcomeStatus::Cancelled,
            Self::Failed { .. } => TurnOutcomeStatus::Failed,
        }
    }

    pub fn status_str(&self) -> &'static str {
        self.status().as_str()
    }

    pub fn reply_text(&self) -> String {
        match self {
            Self::Completed { final_response, .. } => {
                if final_response.trim().is_empty() {
                    "(no final text response)".to_string()
                } else {
                    final_response.clone()
                }
            }
            Self::Cancelled { .. } => {
                "The target session cancelled this request before producing a final answer.".to_string()
            }
            Self::Failed { error, .. } => {
                format!("The target session failed to complete this request.\nError: {error}")
            }
        }
    }

    pub fn queue_action(&self) -> TurnOutcomeQueueAction {
        match self {
            Self::Completed { .. } | Self::Cancelled { .. } => TurnOutcomeQueueAction::DispatchNext,
            Self::Failed { .. } => TurnOutcomeQueueAction::ClearQueue,
        }
    }
}

/// Decision about what the thread-goal continuation loop should do after a
/// turn finishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalContinuationAfterTurnAction {
    SkipNoActiveTurn,
    AbortForCancelled,
    Evaluate { turn_completed: bool },
}

/// Aggregated post-turn decisions surfaced to the concrete scheduler so it
/// can update queue state, drain injections, and continue the goal loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnOutcomeLifecyclePlan {
    pub status: TurnOutcomeStatus,
    pub queue_action: TurnOutcomeQueueAction,
    pub drain_finished_turn_injections: bool,
    pub goal_continuation: GoalContinuationAfterTurnAction,
}

impl TurnOutcomeLifecyclePlan {
    pub const fn dispatch_next(self) -> bool {
        matches!(self.queue_action, TurnOutcomeQueueAction::DispatchNext)
    }

    pub const fn clear_queue(self) -> bool {
        matches!(self.queue_action, TurnOutcomeQueueAction::ClearQueue)
    }
}
