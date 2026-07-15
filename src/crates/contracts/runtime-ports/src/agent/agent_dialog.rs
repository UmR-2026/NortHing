//! Dialog submission policy, queue routing, turn outcome, reply route, steering,
//! and round-injection types.

use serde::{Deserialize, Serialize};

use crate::port_core::PortResult;

// ── Submission source / trigger ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSubmissionSource {
    DesktopUi,
    DesktopApi,
    AgentSession,
    ScheduledJob,
    RemoteRelay,
    Bot,
    Cli,
}

pub type DialogTriggerSource = AgentSubmissionSource;

// ── Queue priority / submission policy ───────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogQueuePriority {
    Low = 0,
    Normal = 1,
    High = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogSubmissionPolicy {
    pub trigger_source: DialogTriggerSource,
    pub queue_priority: DialogQueuePriority,
    pub skip_tool_confirmation: bool,
}

impl DialogSubmissionPolicy {
    pub const fn new(
        trigger_source: DialogTriggerSource,
        queue_priority: DialogQueuePriority,
        skip_tool_confirmation: bool,
    ) -> Self {
        Self {
            trigger_source,
            queue_priority,
            skip_tool_confirmation,
        }
    }

    pub const fn for_source(trigger_source: DialogTriggerSource) -> Self {
        let (queue_priority, skip_tool_confirmation) = match trigger_source {
            DialogTriggerSource::AgentSession => (DialogQueuePriority::Low, true),
            DialogTriggerSource::ScheduledJob => (DialogQueuePriority::Low, true),
            DialogTriggerSource::DesktopUi | DialogTriggerSource::DesktopApi | DialogTriggerSource::Cli => {
                (DialogQueuePriority::Normal, false)
            }
            DialogTriggerSource::RemoteRelay | DialogTriggerSource::Bot => (DialogQueuePriority::Normal, true),
        };
        Self::new(trigger_source, queue_priority, skip_tool_confirmation)
    }

    pub const fn with_queue_priority(mut self, queue_priority: DialogQueuePriority) -> Self {
        self.queue_priority = queue_priority;
        self
    }

    pub const fn with_skip_tool_confirmation(mut self, skip_tool_confirmation: bool) -> Self {
        self.skip_tool_confirmation = skip_tool_confirmation;
        self
    }
}

// ── Queue outcome / state facts / action ─────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogSubmitOutcome {
    Started { session_id: String, turn_id: String },
    Queued { session_id: String, turn_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogSessionStateFact {
    Missing,
    Idle,
    Processing,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogSubmitQueueFacts {
    pub session_state: DialogSessionStateFact,
    pub queue_has_items: bool,
    pub policy: DialogSubmissionPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogSubmitQueueAction {
    StartImmediately,
    ClearQueueAndStartImmediately,
    EnqueueThenStartNext,
    EnqueueForActiveTurn,
}

pub const fn resolve_dialog_submit_queue_action(facts: DialogSubmitQueueFacts) -> DialogSubmitQueueAction {
    match facts.session_state {
        DialogSessionStateFact::Missing => DialogSubmitQueueAction::StartImmediately,
        DialogSessionStateFact::Error => DialogSubmitQueueAction::ClearQueueAndStartImmediately,
        DialogSessionStateFact::Idle => {
            if facts.queue_has_items {
                DialogSubmitQueueAction::EnqueueThenStartNext
            } else {
                DialogSubmitQueueAction::StartImmediately
            }
        }
        DialogSessionStateFact::Processing => DialogSubmitQueueAction::EnqueueForActiveTurn,
    }
}

pub fn should_suppress_agent_session_cancelled_reply(
    policy: &DialogSubmissionPolicy,
    reply_source_session_id: Option<&str>,
    requester_session_id: &str,
) -> bool {
    policy.trigger_source == DialogTriggerSource::AgentSession
        && reply_source_session_id.is_some_and(|source| source == requester_session_id)
}

// ── Turn outcome / reply route / steer ───────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogTurnOutcomeKind {
    Completed,
    Cancelled,
    Failed,
}

pub const fn should_skip_agent_session_reply(
    outcome_kind: DialogTurnOutcomeKind,
    suppressed_cancelled_reply: bool,
) -> bool {
    matches!(outcome_kind, DialogTurnOutcomeKind::Cancelled) && suppressed_cancelled_reply
}

/// Source session route used when an agent-session request should reply to the
/// requester after the target session finishes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionReplyRoute {
    pub source_session_id: String,
    pub source_workspace_path: String,
}

/// Outcome for steering a message into an already-running dialog turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogSteerOutcome {
    /// Steering was buffered for the running turn and will be consumed at the
    /// next model-round boundary.
    Buffered {
        session_id: String,
        turn_id: String,
        steering_id: String,
    },
}

// ── Round injection ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundInjectionKind {
    UserSteering,
    BackgroundResult,
    ThreadGoalObjectiveUpdated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoundInjectionTarget {
    /// Only inject into the exact targeted running turn.
    ExactTurn(String),
    /// Inject into whichever turn is currently running for the session.
    CurrentRunningTurn,
}

/// A message to inject into the currently running dialog turn at the next
/// model-round boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundInjection {
    pub id: String,
    pub kind: RoundInjectionKind,
    pub target: RoundInjectionTarget,
    pub content: String,
    pub display_content: String,
    pub created_at: std::time::SystemTime,
}

/// Observes round-boundary injections for a given running turn.
pub trait DialogRoundInjectionSource: Send + Sync {
    fn has_pending(&self, session_id: &str, turn_id: &str) -> bool;
    fn take_pending(&self, session_id: &str, turn_id: &str) -> Vec<RoundInjection>;
}

// ── Dialog turn port trait ───────────────────────────────────────────────────

use super::AgentDialogTurnRequest;

#[async_trait::async_trait]
pub trait AgentDialogTurnPort: Send + Sync {
    async fn submit_dialog_turn(&self, request: AgentDialogTurnRequest) -> PortResult<DialogSubmitOutcome>;
}
