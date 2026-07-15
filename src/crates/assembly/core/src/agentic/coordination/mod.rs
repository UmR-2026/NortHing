//! Conversation coordinator module
//!
//! Public API entry point. The 7232-line original was split by responsibility
//! region (Round 3a of the debt-reduction plan) into:
//! - coordinator.rs: struct + accessors + subagent types + simple helpers
//! - dialog_turn.rs: dialog turn lifecycle + thread goal + compaction
//! - subagent_orchestrator.rs: subagent orchestration methods
//! - ports.rs: 5 trait impls (AgentSubmissionPort, AgentSessionManagementPort,
//!             AgentTurnCancellationPort, RemoteControlStatePort,
//!             SessionTranscriptReader)
//!
//! Re-exports keep the `crate::agentic::coordination::*` public path
//! unchanged for all 37+ external callers.

// Submodule declarations
mod a1_path;
mod coordinator;
mod dialog_turn;
mod format;
mod handoff; // B-2: SubAgentHandoff trait + per-turn counter
mod port_types;
mod ports;
mod remote_ports;
mod scheduler; // pre-existing
mod session_ports;
mod state_manager;
mod subagent_orchestrator;
mod subagent_ports;
mod turn_outcome; // pre-existing
mod turn_ports;

// Re-exports for the public API
pub use self::coordinator::*;
pub use self::dialog_turn::*;
pub(crate) use self::handoff::{CoordinatorHiddenSubagentHandoff, HandoffError, SubAgentHandoff, TurnHandoffCounter};
pub use self::ports::*;
pub use self::subagent_orchestrator::*;

// Re-export pre-existing sibling modules' public items that callers
// reference through `crate::agentic::coordination::*`.
pub use self::coordinator::DialogTriggerSource;
pub use self::scheduler::{
    abort_thread_goal_continuation_for_session, clear_thread_goal_continuation_abort, global_scheduler,
    DialogQueuePriority, DialogScheduler, DialogSubmissionPolicy, DialogSubmitOutcome,
};
pub use self::turn_outcome::TurnOutcome;

#[cfg(test)]
mod tests;
