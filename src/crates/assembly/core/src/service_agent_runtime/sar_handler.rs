use async_trait::async_trait;
use northhing_runtime_ports::{
    AgentDialogTurnRequest, AgentInputAttachment, AgentLifecycleDeliveryPort, AgentSessionManagementPort,
    AgentSubmissionPort, AgentTurnCancellationPort, AgentTurnCancellationRequest, RemoteControlStatePort,
    RemoteControlStateRequest, RemoteControlStateSnapshot, RuntimeServiceCapability, RuntimeServicePort,
};
use northhing_services_integrations::remote_connect::{
    remote_dialog_submit_outcome_from_scheduler, ChatMessage, RemoteAssistantWorkspaceFacts, RemoteCancelRuntimeHost,
    RemoteChatHistoryRound, RemoteChatHistoryTextItem, RemoteChatHistoryThinkingItem, RemoteChatHistoryToolCall,
    RemoteChatHistoryToolItem, RemoteChatHistoryTurn, RemoteConnectSubmissionSource, RemoteDefaultModelsConfig,
    RemoteDialogQueuePriority, RemoteDialogResolvedSubmission, RemoteDialogRuntimeHost,
    RemoteDialogSchedulerOutcomeFact, RemoteDialogSubmissionPolicy, RemoteDialogSubmitOutcome, RemoteImageContext,
    RemoteInitialSyncRuntimeHost, RemoteInteractionRuntimeHost, RemoteModelCatalog, RemoteModelCatalogFacts,
    RemoteModelFacts, RemotePollRuntimeHost, RemoteReasoningModeFact, RemoteRecentWorkspaceFacts,
    RemoteSessionMetadata as RemoteConnectSessionMetadata, RemoteSessionStateTracker, RemoteSessionTrackerHost,
    RemoteTerminalPrewarmRequest, RemoteWorkspaceFacts, RemoteWorkspaceFileRuntimeHost,
    RemoteWorkspaceKind as RemoteConnectWorkspaceKind, RemoteWorkspaceRuntimeHost, RemoteWorkspaceUpdate,
};
use std::sync::Arc;
use tracing::{debug, info};

use crate::agentic::coordination::{global_coordinator, global_scheduler, ConversationCoordinator, DialogScheduler};
use crate::agentic::events::{AgenticEvent, EventSubscriber};
use crate::agentic::image_analysis::ImageContextData;
use crate::service::remote_connect::remote_server::RemoteExecutionDispatcher;
use crate::service::workspace::global_workspace_service;

pub struct CoreRemoteSessionTrackerHost;

#[async_trait::async_trait]
impl EventSubscriber for Arc<RemoteSessionStateTracker> {
    async fn on_event(&self, event: &AgenticEvent) -> crate::util::errors::NortHingResult<()> {
        self.handle_agentic_event(event);
        Ok(())
    }
}

impl RemoteSessionTrackerHost for CoreRemoteSessionTrackerHost {
    fn subscribe_tracker(&self, session_id: &str, tracker: Arc<RemoteSessionStateTracker>) {
        if let Some(coordinator) = global_coordinator() {
            let sub_id = format!("remote_tracker_{}", session_id);
            coordinator.subscribe_internal(sub_id, tracker);
            info!("Registered state tracker for session {session_id}");
        }
    }

    fn unsubscribe_tracker(&self, session_id: &str) {
        if let Some(coordinator) = global_coordinator() {
            let sub_id = format!("remote_tracker_{}", session_id);
            coordinator.unsubscribe_internal(&sub_id);
        }
    }

    fn active_turn_id(&self, session_id: &str) -> Option<String> {
        let coordinator = global_coordinator()?;
        let session_mgr = coordinator.session_manager();
        let session = session_mgr.get_session(session_id)?;
        match &session.state {
            crate::agentic::core::SessionState::Processing { current_turn_id, .. } => {
                info!(
                    "Seeded tracker with existing active turn {} for session {}",
                    current_turn_id, session_id
                );
                Some(current_turn_id.clone())
            }
            _ => None,
        }
    }
}

pub struct CoreRemoteDialogRuntimeHost<'a> {
    dispatcher: &'a RemoteExecutionDispatcher,
    coordinator: Arc<ConversationCoordinator>,
    runtime: northhing_agent_runtime::runtime::AgentRuntime,
}

impl<'a> CoreRemoteDialogRuntimeHost<'a> {
    pub fn new(dispatcher: &'a RemoteExecutionDispatcher) -> Result<Self, String> {
        let coordinator = global_coordinator().ok_or_else(|| "Desktop session system not ready".to_string())?;
        let scheduler = global_scheduler().ok_or_else(|| "Dialog scheduler is not initialized".to_string())?;
        let runtime = super::sar_dispatch::CoreServiceAgentRuntime::agent_runtime_with_dialog_turns(
            coordinator.clone(),
            scheduler,
        )?;

        Ok(Self {
            dispatcher,
            coordinator,
            runtime,
        })
    }
}

pub struct CoreRemoteWorkspaceFileRuntimeHost;

impl CoreRemoteWorkspaceFileRuntimeHost {
    pub fn new() -> Self {
        Self
    }
}

impl RuntimeServicePort for CoreRemoteWorkspaceFileRuntimeHost {
    fn capability(&self) -> RuntimeServiceCapability {
        RuntimeServiceCapability::RemoteProjection
    }
}

#[async_trait::async_trait]
impl RemoteWorkspaceFileRuntimeHost for CoreRemoteWorkspaceFileRuntimeHost {
    async fn resolve_remote_file_workspace_root(&self, session_id: Option<&str>) -> Option<std::path::PathBuf> {
        super::sar_dispatch::CoreServiceAgentRuntime::resolve_remote_file_workspace_root(session_id).await
    }
}

pub struct CoreRemoteWorkspaceRuntimeHost;

impl CoreRemoteWorkspaceRuntimeHost {
    pub fn new() -> Self {
        Self
    }
}

impl RuntimeServicePort for CoreRemoteWorkspaceRuntimeHost {
    fn capability(&self) -> RuntimeServiceCapability {
        RuntimeServiceCapability::RemoteWorkspace
    }
}

#[async_trait::async_trait]
impl RemoteWorkspaceRuntimeHost for CoreRemoteWorkspaceRuntimeHost {
    async fn current_workspace(&self) -> Option<RemoteWorkspaceFacts> {
        super::sar_types::current_remote_workspace_facts().await
    }

    async fn recent_workspaces(&self) -> Vec<RemoteRecentWorkspaceFacts> {
        let Some(workspace_service) = global_workspace_service() else {
            return Vec::new();
        };
        workspace_service
            .recent_workspaces()
            .await
            .into_iter()
            .map(|workspace| RemoteRecentWorkspaceFacts {
                path: workspace.root_path.to_string_lossy().to_string(),
                name: workspace.name,
                last_opened: workspace.last_accessed.to_rfc3339(),
                kind: super::sar_types::remote_workspace_kind(workspace.workspace_kind),
            })
            .collect()
    }

    async fn open_workspace(&self, path: &str) -> Result<RemoteWorkspaceUpdate, String> {
        super::sar_types::open_workspace_with_snapshot(path, "remote workspace set").await
    }

    async fn assistant_workspaces(&self) -> Vec<RemoteAssistantWorkspaceFacts> {
        let Some(workspace_service) = global_workspace_service() else {
            return Vec::new();
        };
        workspace_service
            .get_assistant_workspaces()
            .await
            .into_iter()
            .map(|workspace| RemoteAssistantWorkspaceFacts {
                path: workspace.root_path.to_string_lossy().to_string(),
                name: workspace.name,
                assistant_id: workspace.assistant_id,
            })
            .collect()
    }

    async fn open_assistant_workspace(&self, path: &str) -> Result<RemoteWorkspaceUpdate, String> {
        super::sar_types::open_workspace_with_snapshot(path, "remote assistant set").await
    }
}

#[async_trait::async_trait]
impl RemoteInitialSyncRuntimeHost for CoreRemoteWorkspaceRuntimeHost {
    async fn current_workspace(&self) -> Option<RemoteWorkspaceFacts> {
        super::sar_types::current_remote_workspace_facts().await
    }

    async fn list_session_metadata(
        &self,
        workspace_path: &std::path::Path,
    ) -> Result<Vec<RemoteConnectSessionMetadata>, String> {
        super::sar_types::load_remote_session_metadata_for_workspace(workspace_path).await
    }
}

pub struct CoreRemotePollRuntimeHost<'a> {
    dispatcher: &'a RemoteExecutionDispatcher,
}

impl<'a> CoreRemotePollRuntimeHost<'a> {
    pub fn new(dispatcher: &'a RemoteExecutionDispatcher) -> Self {
        Self { dispatcher }
    }
}

#[async_trait::async_trait]
impl RemotePollRuntimeHost for CoreRemotePollRuntimeHost<'_> {
    fn ensure_tracker(&self, session_id: &str) -> Arc<RemoteSessionStateTracker> {
        self.dispatcher.ensure_tracker(session_id)
    }

    async fn load_model_catalog(&self, session_id: &str) -> Option<RemoteModelCatalog> {
        super::sar_dispatch::CoreServiceAgentRuntime::load_remote_model_catalog(Some(session_id))
            .await
            .ok()
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
}

#[async_trait::async_trait]
impl RemoteDialogRuntimeHost for CoreRemoteDialogRuntimeHost<'_> {
    type ImageContext = ImageContextData;

    fn ensure_tracker(&self, session_id: &str) {
        self.dispatcher.ensure_tracker(session_id);
    }

    async fn resolve_binding_workspace(&self, session_id: &str) -> Option<String> {
        self.coordinator
            .resolve_session_workspace_path(session_id)
            .await
            .map(|path| path.to_string_lossy().into_owned())
    }

    async fn remote_session_exists(&self, session_id: &str) -> Result<bool, String> {
        Ok(self.coordinator.session_manager().get_session(session_id).is_some())
    }

    async fn restore_remote_session(&self, session_id: &str, workspace_path: &str) -> Result<(), String> {
        self.coordinator
            .restore_session(std::path::Path::new(workspace_path), session_id)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn prewarm_remote_terminal(&self, request: RemoteTerminalPrewarmRequest) {
        use terminal_core::session::SessionSource;
        use terminal_core::{TerminalApi, TerminalBindingOptions};

        let sid = request.session_id;
        let binding_workspace_for_terminal = request.binding_workspace;
        tokio::spawn(async move {
            let Ok(api) = TerminalApi::from_singleton() else {
                return;
            };
            let binding = api.session_manager().binding();
            if binding.get(&sid).is_some() {
                return;
            }
            let workspace = binding_workspace_for_terminal;
            let name = format!("Chat-{}", &sid[..8.min(sid.len())]);
            match binding
                .get_or_create(
                    &sid,
                    TerminalBindingOptions {
                        working_directory: workspace,
                        session_id: Some(sid.clone()),
                        session_name: Some(name),
                        env: Some(crate::agentic::tools::implementations::bash_tool::BashTool::noninteractive_env()),
                        source: Some(SessionSource::Agent),
                        ..Default::default()
                    },
                )
                .await
            {
                Ok(_) => info!("Terminal pre-warmed for remote session {sid}"),
                Err(e) => debug!("Terminal pre-warm skipped for {sid}: {e}"),
            }
        });
    }

    fn generate_turn_id(&self) -> String {
        format!("turn_{}", chrono::Utc::now().timestamp_millis())
    }

    async fn submit_dialog(
        &self,
        submission: RemoteDialogResolvedSubmission<Self::ImageContext>,
    ) -> Result<RemoteDialogSubmitOutcome, String> {
        let policy = super::sar_types::core_dialog_submission_policy(submission.policy);
        let attachments = submission
            .image_contexts
            .into_iter()
            .map(super::sar_types::agent_input_attachment_from_image_context)
            .collect();

        self.runtime
            .submit_dialog_turn(AgentDialogTurnRequest {
                session_id: submission.session_id,
                message: submission.content,
                original_message: None,
                turn_id: Some(submission.turn_id),
                agent_type: submission.resolved_agent_type,
                workspace_path: submission.binding_workspace,
                policy,
                reply_route: None,
                prepended_reminders: Vec::new(),
                attachments,
                metadata: serde_json::Map::new(),
            })
            .await
            .map(super::sar_types::remote_dialog_scheduler_outcome_fact)
            .map(remote_dialog_submit_outcome_from_scheduler)
            .map_err(super::sar_dispatch::CoreServiceAgentRuntime::runtime_error_message)
    }
}
