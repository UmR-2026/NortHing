// Facade for the agentic::coordination runtime port implementations.
//
// Round R50c split the original 1739-line ports.rs into five
// sibling modules:
// - port_types:  shared types and constants
// - port_core:   core helpers + AgentTurnCancellationPort + global coordinator
// - port_agent:  AgentSubmissionPort + session/agent helpers
// - port_session: AgentSessionManagementPort + SessionTranscriptReader
// - port_remote: RemoteControlStatePort
//
// The two test modules (tests and subagent_boundary_e2e) live in the
// tests/ subdirectory, organized by domain:
// - tests/session_ports.rs: session lifecycle tests
// - tests/turn_ports.rs: turn scheduling + format tests
// - tests/subagent_ports.rs: subagent dispatch boundary tests
//
// Re-exports keep the crate::agentic::coordination::* public path
// unchanged for all external callers.

// Public re-exports — preserve the existing public API surface.
#[allow(unused_imports)]
pub use super::format::*;
#[allow(unused_imports)]
pub use super::port_types::*;
#[allow(unused_imports)]
pub use super::remote_ports::*;
#[allow(unused_imports)]
pub use super::session_ports::*;
#[allow(unused_imports)]
pub use super::subagent_ports::*;
#[allow(unused_imports)]
pub use super::turn_ports::*;

// Items the test modules below reference through super::* (or
// super::{specific, items, ...}). use super::coordinator::*;
// brings in every pub / pub(crate) / pub(super) item declared
// in coordinator.rs.
#[cfg(test)]
use super::coordinator::*;
#[cfg(test)]
use crate::agentic::tools::pipeline::SubagentParentInfo;
#[cfg(test)]
use crate::agentic::tools::ToolRuntimeRestrictions;
#[cfg(test)]
use northhing_runtime_ports::DelegationPolicy;
