//! Runtime error types for the port-backed [`crate::runtime::AgentRuntime`] facade.
//!
//! Split from `runtime.rs` (R39e) so error variants stay adjacent to the
//! port mapping they describe. The facade re-exports both enums so callers
//! continue to import them via `northhing_agent_runtime::runtime::*`.

use northhing_runtime_ports::PortError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeBuildError {
    #[error("agent submission port is required")]
    MissingSubmissionPort,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeError {
    #[error("agent dialog turn port is not registered")]
    MissingDialogTurnPort,
    #[error("agent lifecycle delivery port is not registered")]
    MissingLifecycleDeliveryPort,
    #[error("agent cancellation port is not registered")]
    MissingCancellationPort,
    #[error("agent session management port is not registered")]
    MissingSessionManagementPort,
    #[error("runtime event sink is not registered")]
    MissingEventSink,
    #[error(transparent)]
    Port(#[from] PortError),
}
