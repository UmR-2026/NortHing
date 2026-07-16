#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]
#![allow(unused_imports)]
//! Agent runtime owner contracts.
//!
//! This crate owns runtime decisions that can be built and tested without
//! depending on `northhing-core` concrete session or scheduler lifecycle.

pub mod agents;
pub mod checkpoint;
pub mod custom_subagent;
pub mod deep_research;
pub mod deep_review;
pub mod events;
pub mod post_call_hooks;
pub mod prompt;
pub mod prompt_cache;
pub mod runtime;
pub mod scheduled_job;
pub mod scheduler;
pub mod session_control;
pub mod thread_goal;
pub mod thread_goal_tools;
pub mod tool_confirmation;
pub mod user_questions;

// Re-export common stable types so cross-crate callers do not need deep
// module paths.  Each group preserves the original module ownership; only
// the public surface is flattened here.
pub use custom_subagent::CustomSubagentKind;
pub use deep_research::{renumber_research_report, ResearchCitationDisplayMapEntry};
pub use events::{session_state_label, FinishReason};
pub use prompt::{PrependedPromptReminders, ToolListingSections, UserContextPolicy, UserContextSection};
pub use runtime::{AgentRuntime, AgentRuntimeBuilder, RuntimeError};
pub use scheduled_job::ScheduledJobEnqueueFailureAction;
pub use scheduler::{
    resolve_dialog_steering_action, ActiveDialogTurn, AgentSessionReplyPlan, DialogSteeringAction, TurnOutcome,
    TurnOutcomeQueueAction, TurnOutcomeStatus,
};
pub use thread_goal::{
    thread_goal_event_payload, thread_goal_from_custom_metadata, ThreadGoalRuntimeError, ThreadGoalTokenUsageFacts,
};
