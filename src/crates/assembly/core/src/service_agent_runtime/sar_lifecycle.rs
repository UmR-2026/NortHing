use async_trait::async_trait;
use northhing_runtime_ports::{
    AgentSessionCreateRequest, AgentSessionManagementPort, AgentSubmissionPort, AgentTurnCancellationPort,
};
use northhing_services_integrations::remote_connect::{
    ChatMessage, RemoteModelCatalog, RemoteSessionMetadata as RemoteConnectSessionMetadata, RemoteSessionRuntimeHost,
};
use std::sync::Arc;

use crate::agentic::coordination::{global_coordinator, ConversationCoordinator};
use crate::service::workspace::global_workspace_service;

pub struct CoreRemoteSessionRuntimeHost {
    coordinator: Arc<ConversationCoordinator>,
    runtime: northhing_agent_runtime::runtime::AgentRuntime,
}

impl CoreRemoteSessionRuntimeHost {
    pub fn new() -> Result<Self, String> {
        let coordinator = global_coordinator().ok_or_else(|| "Desktop session system not ready".to_string())?;
        let runtime = super::sar_dispatch::CoreServiceAgentRuntime::agent_runtime(coordinator.clone())?;
        Ok(Self { coordinator, runtime })
    }
}

#[async_trait::async_trait]
impl RemoteSessionRuntimeHost for CoreRemoteSessionRuntimeHost {
    async fn list_session_metadata(
        &self,
        workspace_path: &std::path::Path,
    ) -> Result<Vec<RemoteConnectSessionMetadata>, String> {
        super::sar_types::load_remote_session_metadata_for_workspace(workspace_path).await
    }

    async fn resolve_default_assistant_workspace_path(&self) -> Result<String, String> {
        let workspace_service =
            global_workspace_service().ok_or_else(|| "Workspace service not available".to_string())?;
        let workspaces = workspace_service.get_assistant_workspaces().await;
        if let Some(default_workspace) = workspaces
            .into_iter()
            .find(|workspace| workspace.assistant_id.is_none())
        {
            return Ok(default_workspace.root_path.to_string_lossy().to_string());
        }

        workspace_service
            .create_assistant_workspace(None)
            .await
            .map(|workspace| workspace.root_path.to_string_lossy().to_string())
            .map_err(|error| format!("Failed to create assistant workspace: {}", error))
    }

    async fn create_session(&self, request: AgentSessionCreateRequest) -> Result<String, String> {
        self.runtime
            .create_session(request)
            .await
            .map(|session| session.session_id)
            .map_err(super::sar_dispatch::CoreServiceAgentRuntime::runtime_error_message)
    }

    async fn load_model_catalog(&self, session_id: Option<&str>) -> Result<RemoteModelCatalog, String> {
        super::sar_dispatch::CoreServiceAgentRuntime::load_remote_model_catalog(session_id).await
    }

    async fn update_session_model(&self, session_id: &str, model_id: &str) -> Result<String, String> {
        super::sar_dispatch::CoreServiceAgentRuntime::update_remote_session_model(
            self.coordinator.as_ref(),
            session_id,
            model_id,
        )
        .await
    }

    async fn ensure_session_loaded(&self, session_id: &str) -> Result<(), String> {
        if self.coordinator.session_manager().get_session(session_id).is_some() {
            return Ok(());
        }

        let Some(workspace_path) =
            super::sar_dispatch::CoreServiceAgentRuntime::resolve_session_workspace_path(session_id).await
        else {
            return Err(format!("Workspace path not available for session: {}", session_id));
        };
        self.coordinator
            .restore_session(&workspace_path, session_id)
            .await
            .map(|_| ())
            .map_err(|error| format!("Failed to restore session: {error}"))
    }

    async fn update_session_title(&self, session_id: &str, title: &str) -> Result<String, String> {
        self.coordinator
            .update_session_title(session_id, title)
            .await
            .map_err(|error| error.to_string())
    }

    async fn resolve_session_workspace_path(&self, session_id: &str) -> Option<std::path::PathBuf> {
        super::sar_dispatch::CoreServiceAgentRuntime::resolve_session_workspace_path(session_id).await
    }

    async fn load_remote_chat_messages(
        &self,
        workspace_path: &std::path::Path,
        session_id: &str,
    ) -> (Vec<ChatMessage>, bool) {
        super::sar_dispatch::CoreServiceAgentRuntime::load_remote_chat_messages(workspace_path, session_id).await
    }

    async fn delete_session(&self, workspace_path: &std::path::Path, session_id: &str) -> Result<(), String> {
        self.coordinator
            .delete_session(workspace_path, session_id)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn remove_tracker(&self, session_id: &str) {
        crate::service::remote_connect::remote_server::get_or_init_global_dispatcher().remove_tracker(session_id);
    }
}
