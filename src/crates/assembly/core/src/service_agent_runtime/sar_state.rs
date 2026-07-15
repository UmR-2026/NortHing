use async_trait::async_trait;
use northhing_runtime_ports::{
    AgentSessionManagementPort, AgentSubmissionPort, AgentTurnCancellationPort, AgentTurnCancellationRequest,
    RemoteControlStatePort, RemoteControlStateRequest, RemoteControlStateSnapshot,
};
use std::sync::Arc;
use tracing::info;

use crate::agentic::coordination::{global_coordinator, ConversationCoordinator};
use crate::agentic::tools::user_input_manager::user_input_manager;
use crate::service::remote_connect::remote_server::RemoteExecutionDispatcher;

pub struct CoreRemoteCancelRuntimeHost {
    coordinator: Arc<ConversationCoordinator>,
    runtime: northhing_agent_runtime::runtime::AgentRuntime,
}

impl CoreRemoteCancelRuntimeHost {
    pub fn new() -> Result<Self, String> {
        let coordinator = global_coordinator().ok_or_else(|| "Desktop session system not ready".to_string())?;
        let runtime = super::sar_dispatch::CoreServiceAgentRuntime::agent_runtime(coordinator.clone())?;
        Ok(Self { coordinator, runtime })
    }
}

#[async_trait::async_trait]
impl northhing_services_integrations::remote_connect::RemoteCancelRuntimeHost for CoreRemoteCancelRuntimeHost {
    async fn resolve_restore_workspace(&self, session_id: &str) -> Option<String> {
        self.coordinator
            .resolve_session_workspace_path(session_id)
            .await
            .map(|path| path.to_string_lossy().into_owned())
    }

    async fn remote_control_state(&self, session_id: &str) -> Result<Option<RemoteControlStateSnapshot>, String> {
        let state_port =
            super::sar_dispatch::CoreServiceAgentRuntime::remote_control_state_port(self.coordinator.as_ref());
        state_port
            .read_remote_control_state(RemoteControlStateRequest {
                session_id: session_id.to_string(),
            })
            .await
            .map_err(|error| error.message)
    }

    async fn restore_remote_session(&self, session_id: &str, workspace_path: &str) -> Result<(), String> {
        self.coordinator
            .restore_session(std::path::Path::new(workspace_path), session_id)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    async fn cancel_remote_turn(&self, session_id: &str, turn_id: &str) -> Result<(), String> {
        self.runtime
            .cancel_turn(northhing_runtime_ports::AgentTurnCancellationRequest {
                session_id: session_id.to_string(),
                turn_id: Some(turn_id.to_string()),
                source: Some(northhing_runtime_ports::AgentSubmissionSource::RemoteRelay),
                requester_session_id: None,
                reason: None,
                wait_timeout_ms: None,
            })
            .await
            .map(|_| ())
            .map_err(super::sar_dispatch::CoreServiceAgentRuntime::runtime_error_message)
    }
}

pub struct CoreRemoteInteractionRuntimeHost {
    coordinator: Option<Arc<ConversationCoordinator>>,
}

impl CoreRemoteInteractionRuntimeHost {
    pub fn new() -> Self {
        Self {
            coordinator: global_coordinator(),
        }
    }

    fn coordinator(&self) -> Result<&ConversationCoordinator, String> {
        self.coordinator
            .as_deref()
            .ok_or_else(|| "Desktop session system not ready".to_string())
    }
}

#[async_trait::async_trait]
impl northhing_services_integrations::remote_connect::RemoteInteractionRuntimeHost
    for CoreRemoteInteractionRuntimeHost
{
    async fn confirm_tool(&self, tool_id: &str, updated_input: Option<serde_json::Value>) -> Result<(), String> {
        self.coordinator()?
            .confirm_tool(tool_id, updated_input)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    async fn reject_tool(&self, tool_id: &str, reason: String) -> Result<(), String> {
        self.coordinator()?
            .reject_tool(tool_id, reason)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    async fn cancel_tool(&self, tool_id: &str, reason: String) -> Result<(), String> {
        self.coordinator()?
            .cancel_tool(tool_id, reason)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn answer_question(&self, tool_id: &str, answers: serde_json::Value) -> Result<(), String> {
        user_input_manager().send_answer(tool_id, answers)
    }
}
