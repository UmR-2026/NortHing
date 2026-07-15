use northhing_agent_runtime::runtime::{AgentRuntime, AgentRuntimeBuilder, RuntimeError};
use northhing_runtime_ports::{
    AgentDialogTurnPort, AgentDialogTurnRequest, AgentInputAttachment, AgentLifecycleDeliveryPort,
    AgentSessionCreateRequest, AgentSessionManagementPort, AgentSubmissionPort, AgentSubmissionSource,
    AgentTurnCancellationPort, AgentTurnCancellationRequest, RemoteControlStatePort, RemoteControlStateRequest,
    RemoteControlStateSnapshot, RuntimeServiceCapability, RuntimeServicePort,
};
use std::sync::Arc;
use tracing::info;

use crate::agentic::coordination::{global_coordinator, global_scheduler, ConversationCoordinator, DialogScheduler};
use crate::agentic::image_analysis::ImageContextData;
use crate::service::config::types::AIConfig;
use northhing_services_integrations::remote_connect::RemoteImageContextAdapter;
pub struct CoreServiceAgentRuntime;

impl CoreServiceAgentRuntime {
    pub(crate) async fn resolve_session_workspace_path(session_id: &str) -> Option<std::path::PathBuf> {
        let coordinator = global_coordinator()?;
        coordinator.resolve_session_workspace_path(session_id).await
    }

    pub(crate) async fn resolve_remote_file_workspace_root(session_id: Option<&str>) -> Option<std::path::PathBuf> {
        if let Some(session_id) = session_id {
            if let Some(workspace_path) = Self::resolve_session_workspace_path(session_id).await {
                return Some(workspace_path);
            }
        }

        super::sar_types::current_workspace_path()
    }

    pub(crate) fn remote_dialog_host(
        dispatcher: &crate::service::remote_connect::remote_server::RemoteExecutionDispatcher,
    ) -> Result<super::sar_handler::CoreRemoteDialogRuntimeHost<'_>, String> {
        super::sar_handler::CoreRemoteDialogRuntimeHost::new(dispatcher)
    }

    pub(crate) fn remote_cancel_host() -> Result<super::sar_state::CoreRemoteCancelRuntimeHost, String> {
        super::sar_state::CoreRemoteCancelRuntimeHost::new()
    }

    pub(crate) fn remote_workspace_file_host() -> super::sar_handler::CoreRemoteWorkspaceFileRuntimeHost {
        super::sar_handler::CoreRemoteWorkspaceFileRuntimeHost::new()
    }

    pub(crate) fn remote_workspace_host() -> super::sar_handler::CoreRemoteWorkspaceRuntimeHost {
        super::sar_handler::CoreRemoteWorkspaceRuntimeHost::new()
    }

    pub(crate) fn remote_initial_sync_host() -> super::sar_handler::CoreRemoteWorkspaceRuntimeHost {
        super::sar_handler::CoreRemoteWorkspaceRuntimeHost::new()
    }

    pub(crate) fn remote_session_host() -> Result<super::sar_lifecycle::CoreRemoteSessionRuntimeHost, String> {
        super::sar_lifecycle::CoreRemoteSessionRuntimeHost::new()
    }

    pub(crate) fn remote_poll_host(
        dispatcher: &crate::service::remote_connect::remote_server::RemoteExecutionDispatcher,
    ) -> super::sar_handler::CoreRemotePollRuntimeHost<'_> {
        super::sar_handler::CoreRemotePollRuntimeHost::new(dispatcher)
    }

    pub(crate) fn remote_interaction_host() -> super::sar_state::CoreRemoteInteractionRuntimeHost {
        super::sar_state::CoreRemoteInteractionRuntimeHost::new()
    }

    pub(crate) fn remote_image_context(
        context: northhing_services_integrations::remote_connect::RemoteImageContext,
    ) -> crate::agentic::image_analysis::ImageContextData {
        crate::agentic::image_analysis::ImageContextData::from_remote_image_context(context)
    }

    pub(crate) async fn load_remote_chat_messages(
        workspace_path: &std::path::Path,
        session_id: &str,
    ) -> (Vec<northhing_services_integrations::remote_connect::ChatMessage>, bool) {
        let Ok(pm) = crate::infrastructure::PathManager::new() else {
            return (vec![], false);
        };
        let pm = std::sync::Arc::new(pm);
        let Ok(store) = crate::agentic::persistence::PersistenceManager::new(pm) else {
            return (vec![], false);
        };
        let Ok(turns) = store.load_session_turns(workspace_path, session_id).await else {
            return (vec![], false);
        };
        (super::sar_types::remote_chat_messages_from_turns(&turns), false)
    }

    pub(crate) async fn load_remote_model_catalog(
        session_id: Option<&str>,
    ) -> Result<northhing_services_integrations::remote_connect::RemoteModelCatalog, String> {
        let config_service = crate::service::config::get_global_config_service()
            .await
            .map_err(|e| format!("Config service not available: {e}"))?;
        let global_config: crate::service::config::types::GlobalConfig = config_service
            .config(None)
            .await
            .map_err(|e| format!("Failed to load global config: {e}"))?;
        let ai_config: AIConfig = global_config.ai;

        let models: Vec<northhing_services_integrations::remote_connect::RemoteModelFacts> = ai_config
            .models
            .into_iter()
            .map(|model| {
                let reasoning_mode = model.effective_reasoning_mode();

                northhing_services_integrations::remote_connect::RemoteModelFacts {
                    id: model.id,
                    name: model.name,
                    provider: model.provider,
                    base_url: model.base_url,
                    model_name: model.model_name,
                    context_window: model.context_window,
                    enabled: model.enabled,
                    capabilities: model
                        .capabilities
                        .into_iter()
                        .map(super::sar_types::remote_model_capability_fact)
                        .collect(),
                    enable_thinking_process: model.enable_thinking_process,
                    reasoning_mode: Some(super::sar_types::remote_reasoning_mode_fact(reasoning_mode)),
                    reasoning_effort: model.reasoning_effort,
                    thinking_budget_tokens: model.thinking_budget_tokens,
                }
            })
            .collect();

        let session_model_id = if let Some(session_id) = session_id {
            super::sar_types::resolve_session_model_id(session_id).await
        } else {
            None
        };
        Ok(
            northhing_services_integrations::remote_connect::build_remote_model_catalog(
                northhing_services_integrations::remote_connect::RemoteModelCatalogFacts {
                    last_modified_ms: global_config.last_modified.timestamp_millis(),
                    models,
                    default_models: northhing_services_integrations::remote_connect::RemoteDefaultModelsConfig {
                        primary: ai_config.default_models.primary,
                        fast: ai_config.default_models.fast,
                        search: ai_config.default_models.search,
                        image_understanding: ai_config.default_models.image_understanding,
                        image_generation: ai_config.default_models.image_generation,
                        speech_recognition: ai_config.default_models.speech_recognition,
                    },
                    session_model_id,
                },
            ),
        )
    }

    pub(crate) async fn update_remote_session_model(
        coordinator: &ConversationCoordinator,
        session_id: &str,
        model_id: &str,
    ) -> Result<String, String> {
        let ai_config = if super::sar_types::remote_model_selection_needs_config(model_id) {
            let config_service = crate::service::config::get_global_config_service()
                .await
                .map_err(|_| "Config service not available".to_string())?;
            Some(
                config_service
                    .config::<AIConfig>(Some("ai"))
                    .await
                    .map_err(|e| format!("Failed to load AI config: {e}"))?,
            )
        } else {
            None
        };
        let normalized_model_id = super::sar_types::normalize_remote_model_selection(model_id, ai_config.as_ref())?;

        if coordinator.session_manager().get_session(session_id).is_none() {
            let Some(workspace_path) = Self::resolve_session_workspace_path(session_id).await else {
                return Err(format!("Workspace path not available for session: {session_id}"));
            };
            coordinator
                .restore_session(&workspace_path, session_id)
                .await
                .map_err(|e| format!("Failed to restore session: {e}"))?;
        }

        coordinator
            .session_manager()
            .update_session_model_id(session_id, &normalized_model_id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(normalized_model_id)
    }

    pub(crate) fn remote_control_state_port(
        coordinator: &ConversationCoordinator,
    ) -> &(dyn RemoteControlStatePort + '_) {
        coordinator
    }

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
