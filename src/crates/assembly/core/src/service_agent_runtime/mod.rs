//! Core-owned bindings for service and agent runtime ports.
//!
//! Owner crates keep portable contracts and orchestration policy. This module
//! centralizes the concrete core adapters that still own scheduler execution,
//! session restore, terminal pre-warm, and runtime-port implementations until a
//! reviewed port/provider migration proves equivalence.

pub use sar_dispatch::CoreServiceAgentRuntime;

#[path = "sar_dispatch.rs"]
mod sar_dispatch;

#[cfg(test)]
mod tests {
    use super::sar_dispatch::CoreServiceAgentRuntime;

    #[test]
    fn core_service_agent_runtime_owner_keeps_coordinator_port_contracts() {
        fn assert_runtime_ports<T>()
        where
            T: northhing_runtime_ports::AgentSubmissionPort
                + northhing_runtime_ports::AgentSessionManagementPort
                + northhing_runtime_ports::AgentTurnCancellationPort
                + northhing_runtime_ports::RemoteControlStatePort
                + northhing_runtime_ports::SessionTranscriptReader,
        {
        }

        assert_runtime_ports::<crate::agentic::coordination::ConversationCoordinator>();
    }

    #[test]
    fn core_service_agent_runtime_owner_keeps_scheduler_lifecycle_port_contracts() {
        fn assert_scheduler_ports<T>()
        where
            T: northhing_runtime_ports::AgentDialogTurnPort
                + northhing_runtime_ports::AgentLifecycleDeliveryPort
                + northhing_runtime_ports::AgentTurnCancellationPort,
        {
        }

        assert_scheduler_ports::<crate::agentic::coordination::DialogScheduler>();
    }

    #[test]
    fn core_service_agent_runtime_owner_exposes_agent_runtime_and_remote_control_port() {
        fn assert_agent_runtime(
            coordinator: std::sync::Arc<crate::agentic::coordination::ConversationCoordinator>,
        ) -> Result<northhing_agent_runtime::runtime::AgentRuntime, String> {
            CoreServiceAgentRuntime::agent_runtime(coordinator)
        }

        fn assert_agent_runtime_with_dialog_turns(
            coordinator: std::sync::Arc<crate::agentic::coordination::ConversationCoordinator>,
            scheduler: std::sync::Arc<crate::agentic::coordination::DialogScheduler>,
        ) -> Result<northhing_agent_runtime::runtime::AgentRuntime, String> {
            CoreServiceAgentRuntime::agent_runtime_with_dialog_turns(coordinator, scheduler)
        }

        fn assert_agent_runtime_with_lifecycle_delivery(
            coordinator: std::sync::Arc<crate::agentic::coordination::ConversationCoordinator>,
            scheduler: std::sync::Arc<crate::agentic::coordination::DialogScheduler>,
        ) -> Result<northhing_agent_runtime::runtime::AgentRuntime, String> {
            CoreServiceAgentRuntime::agent_runtime_with_lifecycle_delivery(coordinator, scheduler)
        }

        fn assert_agent_runtime_with_scheduler_ports(
            coordinator: std::sync::Arc<crate::agentic::coordination::ConversationCoordinator>,
            scheduler: std::sync::Arc<crate::agentic::coordination::DialogScheduler>,
        ) -> Result<northhing_agent_runtime::runtime::AgentRuntime, String> {
            CoreServiceAgentRuntime::agent_runtime_with_scheduler_ports(coordinator, scheduler)
        }

        let _ = assert_agent_runtime;
        let _ = assert_agent_runtime_with_dialog_turns;
        let _ = assert_agent_runtime_with_lifecycle_delivery;
        let _ = assert_agent_runtime_with_scheduler_ports;
    }
}
