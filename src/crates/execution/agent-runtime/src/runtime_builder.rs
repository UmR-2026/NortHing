//! Builder for the port-backed [`crate::runtime::AgentRuntime`].
//!
//! Split from `runtime.rs` (R39e). The builder hands its assembled parts to
//! a `pub(super)` constructor on `AgentRuntime` so the facade remains the
//! single owner of the runtime's private field layout.

use std::sync::Arc;

use northhing_runtime_ports::{
    AgentDialogTurnPort, AgentLifecycleDeliveryPort, AgentSessionManagementPort, AgentSubmissionPort,
    AgentTurnCancellationPort,
};
use northhing_runtime_services::RuntimeServices;

use super::runtime_error::RuntimeBuildError;
use super::runtime_event_stream::AgentEventStream;

pub use super::AgentRuntime;

#[derive(Default, Clone)]
pub struct AgentRuntimeBuilder {
    submission: Option<Arc<dyn AgentSubmissionPort>>,
    session_management: Option<Arc<dyn AgentSessionManagementPort>>,
    dialog_turn: Option<Arc<dyn AgentDialogTurnPort>>,
    lifecycle_delivery: Option<Arc<dyn AgentLifecycleDeliveryPort>>,
    cancellation: Option<Arc<dyn AgentTurnCancellationPort>>,
    services: Option<RuntimeServices>,
    event_stream: Option<AgentEventStream>,
}

impl AgentRuntimeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_submission_port(mut self, port: Arc<dyn AgentSubmissionPort>) -> Self {
        self.submission = Some(port);
        self
    }

    pub fn with_session_management_port(mut self, port: Arc<dyn AgentSessionManagementPort>) -> Self {
        self.session_management = Some(port);
        self
    }

    pub fn with_dialog_turn_port(mut self, port: Arc<dyn AgentDialogTurnPort>) -> Self {
        self.dialog_turn = Some(port);
        self
    }

    pub fn with_lifecycle_delivery_port(mut self, port: Arc<dyn AgentLifecycleDeliveryPort>) -> Self {
        self.lifecycle_delivery = Some(port);
        self
    }

    pub fn with_cancellation_port(mut self, port: Arc<dyn AgentTurnCancellationPort>) -> Self {
        self.cancellation = Some(port);
        self
    }

    pub fn with_services(mut self, services: RuntimeServices) -> Self {
        self.services = Some(services);
        self
    }

    pub fn with_event_stream(mut self, events: AgentEventStream) -> Self {
        self.event_stream = Some(events);
        self
    }

    pub fn build(self) -> Result<AgentRuntime, RuntimeBuildError> {
        Ok(super::build_agent_runtime(
            self.submission.ok_or(RuntimeBuildError::MissingSubmissionPort)?,
            self.session_management,
            self.dialog_turn,
            self.lifecycle_delivery,
            self.cancellation,
            self.services,
            self.event_stream,
        ))
    }
}
