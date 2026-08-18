use northhing_agent_runtime::runtime::{AgentRuntime, AgentRuntimeBuilder, RuntimeError};
use northhing_runtime_ports::{
    AgentDialogTurnPort, AgentLifecycleDeliveryPort, AgentSessionManagementPort, AgentSubmissionPort,
    AgentTurnCancellationPort,
};
use std::sync::Arc;

use crate::agentic::coordination::{global_coordinator, global_scheduler, ConversationCoordinator, DialogScheduler};

pub struct CoreServiceAgentRuntime;

impl CoreServiceAgentRuntime {
    pub(crate) fn agent_runtime(coordinator: Arc<ConversationCoordinator>) -> Result<AgentRuntime, String> {
        let submission: Arc<dyn AgentSubmissionPort> = coordinator.clone();
        let session_management: Arc<dyn AgentSessionManagementPort> = coordinator.clone();
        let cancellation: Arc<dyn AgentTurnCancellationPort> = coordinator;
        AgentRuntimeBuilder::new()
            .with_submission_port(submission)
            .with_session_management_port(session_management)
            .with_cancellation_port(cancellation)
            .build()
            .map_err(|error| error.to_string())
    }

    pub(crate) fn agent_runtime_with_dialog_turns(
        coordinator: Arc<ConversationCoordinator>,
        scheduler: Arc<DialogScheduler>,
    ) -> Result<AgentRuntime, String> {
        let submission: Arc<dyn AgentSubmissionPort> = coordinator.clone();
        let session_management: Arc<dyn AgentSessionManagementPort> = coordinator.clone();
        let cancellation: Arc<dyn AgentTurnCancellationPort> = coordinator;
        let dialog_turn: Arc<dyn AgentDialogTurnPort> = scheduler.clone();
        let lifecycle_delivery: Arc<dyn AgentLifecycleDeliveryPort> = scheduler;
        AgentRuntimeBuilder::new()
            .with_submission_port(submission)
            .with_session_management_port(session_management)
            .with_cancellation_port(cancellation)
            .with_dialog_turn_port(dialog_turn)
            .with_lifecycle_delivery_port(lifecycle_delivery)
            .build()
            .map_err(|error| error.to_string())
    }

    pub(crate) fn agent_runtime_with_lifecycle_delivery(
        coordinator: Arc<ConversationCoordinator>,
        scheduler: Arc<DialogScheduler>,
    ) -> Result<AgentRuntime, String> {
        let submission: Arc<dyn AgentSubmissionPort> = coordinator.clone();
        let session_management: Arc<dyn AgentSessionManagementPort> = coordinator.clone();
        let cancellation: Arc<dyn AgentTurnCancellationPort> = coordinator;
        let lifecycle_delivery: Arc<dyn AgentLifecycleDeliveryPort> = scheduler;
        AgentRuntimeBuilder::new()
            .with_submission_port(submission)
            .with_session_management_port(session_management)
            .with_cancellation_port(cancellation)
            .with_lifecycle_delivery_port(lifecycle_delivery)
            .build()
            .map_err(|error| error.to_string())
    }

    pub(crate) fn agent_runtime_with_scheduler_ports(
        coordinator: Arc<ConversationCoordinator>,
        scheduler: Arc<DialogScheduler>,
    ) -> Result<AgentRuntime, String> {
        let submission: Arc<dyn AgentSubmissionPort> = coordinator.clone();
        let session_management: Arc<dyn AgentSessionManagementPort> = coordinator;
        let cancellation: Arc<dyn AgentTurnCancellationPort> = scheduler.clone();
        let dialog_turn: Arc<dyn AgentDialogTurnPort> = scheduler.clone();
        let lifecycle_delivery: Arc<dyn AgentLifecycleDeliveryPort> = scheduler;
        AgentRuntimeBuilder::new()
            .with_submission_port(submission)
            .with_session_management_port(session_management)
            .with_cancellation_port(cancellation)
            .with_dialog_turn_port(dialog_turn)
            .with_lifecycle_delivery_port(lifecycle_delivery)
            .build()
            .map_err(|error| error.to_string())
    }

    pub(crate) fn global_agent_runtime_with_lifecycle_delivery() -> Result<AgentRuntime, String> {
        let coordinator = global_coordinator().ok_or_else(|| "Desktop session system not ready".to_string())?;
        let scheduler = global_scheduler().ok_or_else(|| "Dialog scheduler is not initialized".to_string())?;
        Self::agent_runtime_with_lifecycle_delivery(coordinator, scheduler)
    }

    pub(crate) fn runtime_error_message(error: RuntimeError) -> String {
        match error {
            RuntimeError::Port(error) => error.message,
            other => other.to_string(),
        }
    }
}
