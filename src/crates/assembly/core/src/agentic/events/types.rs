//! Unified event model
//!
//! Uses northhing-events layer event definitions, extending core-specific functionality here

use crate::agentic::core::SessionState;
use northhing_agent_runtime::events::session_state_label;
use northhing_runtime_ports::DialogSessionStateFact;

// ============ Re-export events layer types ============
pub use northhing_events::agentic::ErrorCategory;
pub use northhing_events::{
    AgenticEvent as BaseAgenticEvent, AgenticEventEnvelope as EventEnvelope, AgenticEventPriority as EventPriority,
    DeepReviewQueueReason, DeepReviewQueueState, DeepReviewQueueStatus, SubagentParentInfo, ToolEventData,
};

// ============ Core layer AgenticEvent extension ============

/// Core layer AgenticEvent
///
/// Used internally in core, contains full type information (SessionState)
/// When sent to transport layer, it is converted to BaseAgenticEvent (using serde_json::Value)
pub type AgenticEvent = BaseAgenticEvent;

// ============ Helper conversion functions ============

/// Convert SessionState to String (for transmission)
pub fn session_state_to_string(state: &SessionState) -> String {
    let fact = match state {
        SessionState::Idle => DialogSessionStateFact::Idle,
        SessionState::Processing { .. } => DialogSessionStateFact::Processing,
        SessionState::Error { .. } => DialogSessionStateFact::Error,
    };
    session_state_label(fact).to_string()
}
