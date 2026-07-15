use super::super::coordinator::{ConversationCoordinator, DialogTriggerSource};
use super::super::turn_outcome::TurnOutcome;
use crate::agentic::core::{InternalReminderKind, Message, SessionState};
use crate::agentic::goal_mode::{
    goal_continuation_submit_retry_delay_ms, goal_internal_context_message, goal_objective_updated_message,
};
use crate::agentic::image_analysis::ImageContextData;
use crate::agentic::init_agents_md::build_init_agents_md_user_input;
use crate::agentic::round_preempt::{DialogRoundInjectionSource, SessionRoundInjectionBuffer};
use crate::agentic::session::SessionManager;
use northhing_runtime_ports::{ThreadGoal, MAX_THREAD_GOAL_AUTO_CONTINUATIONS};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc;
use uuid::Uuid;

use northhing_agent_runtime::scheduler::{
    ActiveDialogTurnStore, DialogReplySuppressionSet, DialogTurnQueue, SessionAbortFlags,
};
pub use northhing_runtime_ports::{
    AgentSessionReplyRoute, DialogQueuePriority, DialogSteerOutcome, DialogSubmissionPolicy, DialogSubmitOutcome,
};

/// A message waiting to be dispatched to the coordinator
#[derive(Debug, Clone)]
pub struct QueuedTurn {
    pub user_input: String,
    pub original_user_input: Option<String>,
    pub prepended_messages: Vec<Message>,
    pub turn_id: Option<String>,
    pub agent_type: String,
    pub workspace_path: Option<String>,
    pub policy: DialogSubmissionPolicy,
    pub reply_route: Option<AgentSessionReplyRoute>,
    pub user_message_metadata: Option<serde_json::Value>,
    pub image_contexts: Option<Vec<ImageContextData>>,
    #[allow(dead_code)]
    pub enqueued_at: SystemTime,
}

/// Message queue manager for dialog turns.
///
/// All user-facing callers (frontend Tauri commands, remote server, bot router)
/// should submit messages through this scheduler instead of calling
/// ConversationCoordinator directly.
pub struct DialogScheduler {
    pub(super) coordinator: Arc<ConversationCoordinator>,
    pub(super) session_manager: Arc<SessionManager>,
    pub(super) queues: Arc<DialogTurnQueue<QueuedTurn>>,
    pub(super) active_turns: Arc<ActiveDialogTurnStore>,
    pub(super) suppressed_cancelled_replies: Arc<DialogReplySuppressionSet>,
    pub(super) goal_continuation_abort: Arc<SessionAbortFlags>,
    pub(super) outcome_tx: mpsc::Sender<(String, TurnOutcome)>,
    pub(super) round_injection_buffer: Arc<SessionRoundInjectionBuffer>,
}
