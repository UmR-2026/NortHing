pub use northhing_events::{AgenticEvent, ToolEventData};
pub use northhing_runtime_ports::{AgentSubmissionSource, RemoteControlSessionState, RemoteControlStateSnapshot};
pub use northhing_services_integrations::remote_connect::{
    build_remote_chat_messages, build_remote_image_attachment, build_remote_image_contexts,
    build_remote_image_submission_request, build_remote_model_catalog, build_remote_session_create_request,
    build_remote_submission_request, cancel_remote_task, handle_remote_command, handle_remote_workspace_file_command,
    make_slim_tool_params, normalize_remote_model_selection, normalize_remote_session_model_id,
    read_remote_workspace_file, read_remote_workspace_file_chunk, read_remote_workspace_file_info,
    remote_answer_question_response, remote_assistant_list_response, remote_assistant_updated_response,
    remote_dialog_submit_outcome_from_scheduler, remote_dialog_submit_response, remote_file_chunk_response,
    remote_file_content_response, remote_file_display_name, remote_file_info_response, remote_initial_sync_response,
    remote_interaction_accepted_response, remote_messages_response, remote_model_catalog_poll_delta,
    remote_model_selection_needs_config, remote_no_change_poll_response, remote_persisted_poll_response,
    remote_recent_workspaces_response, remote_session_created_response, remote_session_deleted_response,
    remote_session_info, remote_session_list_response, remote_session_model_updated_response,
    remote_session_restore_target, remote_snapshot_poll_response, remote_task_cancel_response,
    remote_workspace_info_response, remote_workspace_updated_response, resolve_remote_agent_type,
    resolve_remote_cancel_decision, resolve_remote_execution_image_contexts, resolve_remote_file_chunk_range,
    resolve_remote_workspace_path, should_send_remote_model_catalog, submit_remote_dialog, ActiveTurnSnapshot,
    ChatImageAttachment, ChatMessage, ChatMessageItem, DeviceIdentity, ImageAttachment, KeyPair, PairingProtocol,
    PairingState, QrGenerator, QrPayload, RelayMessage, RemoteAssistantWorkspaceFacts, RemoteCancelDecision,
    RemoteCancelRuntimeHost, RemoteCancelTaskRequest, RemoteChatHistoryRound, RemoteChatHistoryTextItem,
    RemoteChatHistoryThinkingItem, RemoteChatHistoryToolCall, RemoteChatHistoryToolItem, RemoteChatHistoryTurn,
    RemoteCommand, RemoteCommandRuntimeHost, RemoteConnectSubmissionSource, RemoteDefaultModelsConfig,
    RemoteDialogQueuePriority, RemoteDialogResolvedSubmission, RemoteDialogRuntimeHost,
    RemoteDialogSchedulerOutcomeFact, RemoteDialogSubmissionPolicy, RemoteDialogSubmissionRequest,
    RemoteDialogSubmitOutcome, RemoteImageContext, RemoteImageContextAdapter, RemoteModelCapabilityFact,
    RemoteModelCatalog, RemoteModelCatalogFacts, RemoteModelConfig, RemoteModelFacts, RemoteReasoningModeFact,
    RemoteRecentWorkspaceFacts, RemoteResponse, RemoteSessionMetadata, RemoteSessionStateTracker,
    RemoteSessionTrackerHost, RemoteSessionTrackerRegistry, RemoteTerminalPrewarmRequest, RemoteToolStatus,
    RemoteWorkspaceFacts, RemoteWorkspaceFileChunk, RemoteWorkspaceFileContent, RemoteWorkspaceFileInfo,
    RemoteWorkspaceFileRuntimeHost, RemoteWorkspaceKind, RemoteWorkspaceUpdate, TrackerEvent,
    REMOTE_FILE_MAX_CHUNK_BYTES, REMOTE_FILE_MAX_READ_BYTES,
};
pub use std::path::PathBuf;
pub use std::sync::{Arc, Mutex};
pub use serde_json::json;
pub use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq)]
pub struct TestImageContext {
    pub id: String,
    pub image_path: Option<String>,
    pub data_url: Option<String>,
    pub mime_type: String,
    pub metadata: Option<serde_json::Value>,
}

impl RemoteImageContextAdapter for TestImageContext {
    fn from_remote_image_context(context: RemoteImageContext) -> Self {
        Self {
            id: context.id,
            image_path: context.image_path,
            data_url: context.data_url,
            mime_type: context.mime_type,
            metadata: context.metadata,
        }
    }
}

#[test]
fn remote_connect_image_context_adapter_owns_portable_conversion_shape() {
    let context = RemoteImageContext {
        id: "ctx-1".to_string(),
        image_path: Some("D:/workspace/project/screenshot.png".to_string()),
        data_url: Some("data:image/png;base64,abc".to_string()),
        mime_type: "image/png".to_string(),
        metadata: Some(serde_json::json!({ "source": "remote" })),
    };

    let adapted = TestImageContext::from_remote_image_context(context);

    assert_eq!(adapted.id, "ctx-1");
    assert_eq!(
        adapted.image_path.as_deref(),
        Some("D:/workspace/project/screenshot.png")
    );
    assert_eq!(adapted.data_url.as_deref(), Some("data:image/png;base64,abc"));
    assert_eq!(adapted.mime_type, "image/png");
    assert_eq!(adapted.metadata.as_ref().unwrap()["source"], "remote");
}

pub fn remote_history_contract_turn(is_in_progress: bool) -> RemoteChatHistoryTurn {
    RemoteChatHistoryTurn {
        turn_id: "turn-1".to_string(),
        user_message_id: "user-1".to_string(),
        user_display_content: "original question".to_string(),
        user_timestamp_ms: 1_000,
        user_images: vec![ChatImageAttachment {
            name: "screenshot.png".to_string(),
            data_url: "data:image/png;base64,abcd".to_string(),
        }],
        is_in_progress,
        start_time_ms: 1_000,
        rounds: vec![RemoteChatHistoryRound {
            start_time_ms: 1_100,
            end_time_ms: Some(1_200),
            text_items: vec![
                RemoteChatHistoryTextItem {
                    content: "hidden text".to_string(),
                    order_index: Some(1),
                    is_subagent: true,
                },
                RemoteChatHistoryTextItem {
                    content: "visible text".to_string(),
                    order_index: Some(1),
                    is_subagent: false,
                },
            ],
            thinking_items: vec![RemoteChatHistoryThinkingItem {
                content: "visible thought".to_string(),
                order_index: Some(0),
                is_subagent: false,
            }],
            tool_items: vec![RemoteChatHistoryToolItem {
                id: "tool-1".to_string(),
                name: "AskUserQuestion".to_string(),
                call: RemoteChatHistoryToolCall {
                    id: "call-1".to_string(),
                    input: serde_json::json!({ "question": "confirm?" }),
                },
                has_result: false,
                status: Some("running".to_string()),
                duration_ms: Some(25),
                start_ms: 1_130,
                order_index: Some(2),
                is_subagent: false,
            }],
        }],
    }
}

pub struct RecordingDialogHost {
    pub session_exists: bool,
    pub binding_workspace: Option<String>,
    pub generated_turn_id: String,
    pub restore_error: bool,
    pub submit_outcome: RemoteDialogSubmitOutcome,
    pub events: Mutex<Vec<String>>,
    pub submitted: Mutex<Option<RemoteDialogResolvedSubmission<String>>>,
}

impl RecordingDialogHost {
    pub fn new(session_exists: bool, binding_workspace: Option<&str>) -> Self {
        Self {
            session_exists,
            binding_workspace: binding_workspace.map(ToOwned::to_owned),
            generated_turn_id: "turn-generated".to_string(),
            restore_error: false,
            submit_outcome: RemoteDialogSubmitOutcome::Started {
                session_id: "session-1".to_string(),
                turn_id: "turn-generated".to_string(),
            },
            events: Mutex::new(Vec::new()),
            submitted: Mutex::new(None),
        }
    }

    pub fn with_restore_error(mut self) -> Self {
        self.restore_error = true;
        self
    }

    pub fn with_submit_outcome(mut self, submit_outcome: RemoteDialogSubmitOutcome) -> Self {
        self.submit_outcome = submit_outcome;
        self
    }

    pub fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }

    pub fn submitted(&self) -> RemoteDialogResolvedSubmission<String> {
        self.submitted.lock().unwrap().clone().expect("dialog submitted")
    }
}

#[async_trait::async_trait]
impl RemoteDialogRuntimeHost for RecordingDialogHost {
    type ImageContext = String;

    fn ensure_tracker(&self, session_id: &str) {
        self.events.lock().unwrap().push(format!("ensure_tracker:{session_id}"));
    }

    async fn resolve_binding_workspace(&self, session_id: &str) -> Option<String> {
        self.events
            .lock()
            .unwrap()
            .push(format!("resolve_workspace:{session_id}"));
        self.binding_workspace.clone()
    }

    async fn remote_session_exists(&self, session_id: &str) -> Result<bool, String> {
        self.events.lock().unwrap().push(format!("session_exists:{session_id}"));
        Ok(self.session_exists)
    }

    async fn restore_remote_session(&self, session_id: &str, workspace_path: &str) -> Result<(), String> {
        self.events
            .lock()
            .unwrap()
            .push(format!("restore:{session_id}:{workspace_path}"));
        if self.restore_error {
            Err("restore failed".to_string())
        } else {
            Ok(())
        }
    }

    fn prewarm_remote_terminal(&self, request: RemoteTerminalPrewarmRequest) {
        self.events.lock().unwrap().push(format!(
            "prewarm:{}:{}",
            request.session_id,
            request.binding_workspace.as_deref().unwrap_or("<none>")
        ));
    }

    fn generate_turn_id(&self) -> String {
        self.events.lock().unwrap().push("generate_turn".to_string());
        self.generated_turn_id.clone()
    }

    async fn submit_dialog(
        &self,
        submission: RemoteDialogResolvedSubmission<Self::ImageContext>,
    ) -> Result<RemoteDialogSubmitOutcome, String> {
        self.events
            .lock()
            .unwrap()
            .push(format!("submit:{}", submission.session_id));
        *self.submitted.lock().unwrap() = Some(submission);
        Ok(self.submit_outcome.clone())
    }
}

pub struct RecordingCancelHost {
    pub initial_state: Mutex<Option<RemoteControlStateSnapshot>>,
    pub restored_state: Mutex<Option<RemoteControlStateSnapshot>>,
    pub state_reads: Mutex<usize>,
    pub restore_workspace: Option<String>,
    pub restore_error: bool,
    pub cancel_error: Option<String>,
    pub events: Mutex<Vec<String>>,
}

impl RecordingCancelHost {
    pub fn new(
        initial_state: Option<RemoteControlStateSnapshot>,
        restored_state: Option<RemoteControlStateSnapshot>,
        restore_workspace: Option<&str>,
    ) -> Self {
        Self {
            initial_state: Mutex::new(initial_state),
            restored_state: Mutex::new(restored_state),
            state_reads: Mutex::new(0),
            restore_workspace: restore_workspace.map(ToOwned::to_owned),
            restore_error: false,
            cancel_error: None,
            events: Mutex::new(Vec::new()),
        }
    }

    pub fn with_restore_error(mut self) -> Self {
        self.restore_error = true;
        self
    }

    pub fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }
}

pub fn remote_state(
    session_id: &str,
    state: RemoteControlSessionState,
    active_turn_id: Option<&str>,
) -> RemoteControlStateSnapshot {
    RemoteControlStateSnapshot {
        session_id: session_id.to_string(),
        state,
        active_turn_id: active_turn_id.map(ToOwned::to_owned),
        queue_depth: 0,
        metadata: serde_json::Map::new(),
    }
}

#[async_trait::async_trait]
impl RemoteCancelRuntimeHost for RecordingCancelHost {
    async fn resolve_restore_workspace(&self, session_id: &str) -> Option<String> {
        self.events
            .lock()
            .unwrap()
            .push(format!("resolve_workspace:{session_id}"));
        self.restore_workspace.clone()
    }

    async fn remote_control_state(&self, session_id: &str) -> Result<Option<RemoteControlStateSnapshot>, String> {
        self.events.lock().unwrap().push(format!("read_state:{session_id}"));
        let mut reads = self.state_reads.lock().unwrap();
        let read_index = *reads;
        *reads += 1;
        drop(reads);

        if read_index == 0 {
            return Ok(self.initial_state.lock().unwrap().clone());
        }
        Ok(self.restored_state.lock().unwrap().clone())
    }

    async fn restore_remote_session(&self, session_id: &str, workspace_path: &str) -> Result<(), String> {
        self.events
            .lock()
            .unwrap()
            .push(format!("restore:{session_id}:{workspace_path}"));
        if self.restore_error {
            Err("restore failed".to_string())
        } else {
            Ok(())
        }
    }

    async fn cancel_remote_turn(&self, session_id: &str, turn_id: &str) -> Result<(), String> {
        self.events
            .lock()
            .unwrap()
            .push(format!("cancel:{session_id}:{turn_id}"));
        if let Some(error) = &self.cancel_error {
            Err(error.clone())
        } else {
            Ok(())
        }
    }
}


#[derive(Default)]
pub struct RecordingCommandHost {
    pub events: Mutex<Vec<String>>,
    pub submitted_dialog: Mutex<Option<RemoteDialogSubmissionRequest<String>>>,
    pub cancel_request: Mutex<Option<RemoteCancelTaskRequest>>,
    pub explicit_context_ids: Mutex<Vec<String>>,
    pub legacy_image_names: Mutex<Vec<String>>,
}

impl RecordingCommandHost {
    pub fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }

    pub fn submitted_dialog(&self) -> RemoteDialogSubmissionRequest<String> {
        self.submitted_dialog.lock().unwrap().clone().expect("dialog submitted")
    }

    pub fn cancel_request(&self) -> RemoteCancelTaskRequest {
        self.cancel_request.lock().unwrap().clone().expect("cancel requested")
    }
}

#[async_trait::async_trait]
impl RemoteCommandRuntimeHost for RecordingCommandHost {
    type ImageContext = String;

    async fn handle_workspace_command(&self, _command: &RemoteCommand) -> RemoteResponse {
        self.events.lock().unwrap().push("workspace".to_string());
        RemoteResponse::WorkspaceInfo {
            has_workspace: false,
            path: None,
            project_name: None,
            git_branch: None,
            workspace_kind: None,
            assistant_id: None,
        }
    }

    async fn handle_session_command(&self, _command: &RemoteCommand) -> RemoteResponse {
        self.events.lock().unwrap().push("session".to_string());
        RemoteResponse::SessionCreated {
            session_id: "session-created".to_string(),
        }
    }

    async fn handle_poll_command(&self, _command: &RemoteCommand) -> RemoteResponse {
        self.events.lock().unwrap().push("poll".to_string());
        RemoteResponse::SessionPoll {
            version: 0,
            changed: false,
            session_state: None,
            title: None,
            new_messages: None,
            total_msg_count: None,
            active_turn: None,
            model_catalog: Box::new(None),
        }
    }

    async fn handle_workspace_file_command(&self, _command: &RemoteCommand) -> RemoteResponse {
        self.events.lock().unwrap().push("file".to_string());
        RemoteResponse::FileInfo {
            name: "file.txt".to_string(),
            size: 1,
            mime_type: "text/plain".to_string(),
        }
    }

    async fn handle_interaction_command(&self, _command: &RemoteCommand) -> RemoteResponse {
        self.events.lock().unwrap().push("interaction".to_string());
        RemoteResponse::InteractionAccepted {
            action: "confirm_tool".to_string(),
            target_id: "tool-1".to_string(),
        }
    }

    async fn submit_dialog(
        &self,
        request: RemoteDialogSubmissionRequest<Self::ImageContext>,
    ) -> Result<RemoteDialogSubmitOutcome, String> {
        self.events.lock().unwrap().push("submit".to_string());
        *self.submitted_dialog.lock().unwrap() = Some(request.clone());
        Ok(RemoteDialogSubmitOutcome::Started {
            session_id: request.session_id,
            turn_id: "turn-command".to_string(),
        })
    }

    async fn cancel_task(&self, request: RemoteCancelTaskRequest) -> Result<(), String> {
        self.events.lock().unwrap().push("cancel".to_string());
        *self.cancel_request.lock().unwrap() = Some(request);
        Ok(())
    }

    fn legacy_image_contexts(&self, images: Option<&[ImageAttachment]>) -> Vec<Self::ImageContext> {
        let names = images
            .unwrap_or_default()
            .iter()
            .map(|image| image.name.clone())
            .collect::<Vec<_>>();
        *self.legacy_image_names.lock().unwrap() = names.clone();
        names.into_iter().map(|name| format!("legacy:{name}")).collect()
    }

    fn explicit_image_contexts(&self, contexts: Vec<RemoteImageContext>) -> Vec<Self::ImageContext> {
        let ids = contexts.into_iter().map(|context| context.id).collect::<Vec<_>>();
        *self.explicit_context_ids.lock().unwrap() = ids.clone();
        ids.into_iter().map(|id| format!("explicit:{id}")).collect()
    }
}

pub fn make_temp_remote_workspace() -> (PathBuf, PathBuf, PathBuf) {
    let base = std::env::temp_dir().join(format!("northhing-remote-connect-contract-{}", uuid::Uuid::new_v4()));
    let workspace = base.join("workspace");
    let artifacts = workspace.join("artifacts");
    std::fs::create_dir_all(&artifacts).expect("create remote workspace");
    let report = artifacts.join("report.md");
    std::fs::write(&report, b"hello remote file").expect("write remote file");
    (base, workspace, report)
}

#[derive(Default)]
pub struct RecordingFileHost {
    pub workspace_root: PathBuf,
    pub seen_sessions: Mutex<Vec<Option<String>>>,
}

#[async_trait::async_trait]
impl RemoteWorkspaceFileRuntimeHost for RecordingFileHost {
    async fn resolve_remote_file_workspace_root(&self, session_id: Option<&str>) -> Option<PathBuf> {
        self.seen_sessions
            .lock()
            .unwrap()
            .push(session_id.map(ToOwned::to_owned));
        Some(self.workspace_root.clone())
    }
}

pub fn sample_remote_model_catalog(version: u64) -> RemoteModelCatalog {
    RemoteModelCatalog {
        version,
        models: vec![RemoteModelConfig {
            id: "model-1".to_string(),
            name: "Model One".to_string(),
            provider: "openai".to_string(),
            base_url: "https://api.example.com".to_string(),
            model_name: "gpt-test".to_string(),
            context_window: Some(128_000),
            enabled: true,
            capabilities: vec!["text_chat".to_string()],
            enable_thinking_process: false,
            reasoning_mode: Some("default".to_string()),
            reasoning_effort: None,
            thinking_budget_tokens: None,
        }],
        default_models: RemoteDefaultModelsConfig {
            primary: Some("model-1".to_string()),
            ..RemoteDefaultModelsConfig::default()
        },
        session_model_id: Some("model-1".to_string()),
    }
}

#[derive(Default)]
pub struct RecordingTrackerHost {
    pub subscribed: Mutex<Vec<String>>,
    pub unsubscribed: Mutex<Vec<String>>,
    pub active_turn_id: Mutex<Option<String>>,
}

impl RecordingTrackerHost {
    pub fn with_active_turn(turn_id: impl Into<String>) -> Self {
        Self {
            active_turn_id: Mutex::new(Some(turn_id.into())),
            ..Self::default()
        }
    }
}

impl RemoteSessionTrackerHost for RecordingTrackerHost {
    fn subscribe_tracker(&self, session_id: &str, _tracker: Arc<RemoteSessionStateTracker>) {
        self.subscribed.lock().unwrap().push(session_id.to_string());
    }

    fn unsubscribe_tracker(&self, session_id: &str) {
        self.unsubscribed.lock().unwrap().push(session_id.to_string());
    }

    fn active_turn_id(&self, _session_id: &str) -> Option<String> {
        self.active_turn_id.lock().unwrap().clone()
    }
}

pub use northhing_services_integrations::mcp::auth::{MCPRemoteOAuthSessionSnapshot, MCPRemoteOAuthStatus};
pub use northhing_services_integrations::mcp::config::ConfigLocation;
pub use northhing_services_integrations::mcp::config::{
    config_to_cursor_format, format_mcp_json_config_value, get_mcp_remote_authorization_source,
    get_mcp_remote_authorization_value, has_mcp_remote_authorization, has_mcp_remote_oauth, has_mcp_remote_xaa,
    merge_mcp_server_config_sources, normalize_mcp_authorization_value, parse_cursor_format,
    remove_mcp_authorization_keys, validate_mcp_json_config, MCPConfigService, MCPConfigStore,
};
pub use northhing_services_integrations::mcp::protocol::{
    create_initialize_request, create_ping_request, create_tools_call_request, create_tools_list_request,
    default_protocol_version, MCPCapability, MCPError, MCPPrompt, MCPPromptArgument, MCPPromptContent,
    MCPPromptMessage, MCPPromptMessageContent, MCPPromptMessageContentBlock, MCPRequest, MCPResource,
    MCPResourceContent, MCPTool, MCPToolAnnotations, MCPToolResult, MCPToolResultContent,
};
pub use northhing_services_integrations::mcp::server::{
    compute_mcp_backoff_delay, detect_mcp_list_changed_kind, is_mcp_auth_error_message, merge_mcp_remote_headers,
    MCPCatalogCache, MCPConnectionPool, MCPListChangedKind, MCPRuntimeErrorKind, MCPRuntimeResult, MCPServerConfig,
    MCPServerProcess, MCPServerStatus, MCPServerTransport, MCPServerType,
};
pub use northhing_services_integrations::mcp::{
    build_mcp_tool_descriptor, build_mcp_tool_name, normalize_name_for_mcp, render_mcp_tool_result_for_assistant,
    MCPContextEnhancer, MCPContextEnhancerConfig, MCPDynamicToolProvider, MCPToolCatalogClient,
    McpDynamicToolDescriptor, McpToolInfo, PromptAdapter, ResourceAdapter, MCP_TOOL_DELIMITER, MCP_TOOL_PREFIX,
};
pub use std::collections::HashMap;
pub use std::time::Duration;

pub fn make_mcp_config(
    id: &str,
    location: ConfigLocation,
    server_type: MCPServerType,
    command: Option<&str>,
    url: Option<&str>,
) -> MCPServerConfig {
    MCPServerConfig {
        id: id.to_string(),
        name: id.to_string(),
        server_type,
        transport: None,
        command: command.map(str::to_string),
        args: Vec::new(),
        env: HashMap::new(),
        headers: HashMap::new(),
        url: url.map(str::to_string),
        auto_start: true,
        enabled: true,
        location,
        capabilities: Vec::new(),
        settings: Default::default(),
        oauth: None,
        xaa: None,
    }
}

pub fn make_resource(name: &str, description: Option<&str>, uri: &str) -> MCPResource {
    MCPResource {
        uri: uri.to_string(),
        name: name.to_string(),
        title: None,
        description: description.map(str::to_string),
        mime_type: Some("text/plain".to_string()),
        icons: None,
        size: Some(12),
        annotations: None,
        metadata: None,
    }
}

#[derive(Default)]
pub struct InMemoryMCPConfigStore {
    pub values: tokio::sync::Mutex<HashMap<String, serde_json::Value>>,
}

#[async_trait::async_trait]
impl MCPConfigStore for InMemoryMCPConfigStore {
    async fn get_config_value(&self, key: &str) -> MCPRuntimeResult<Option<serde_json::Value>> {
        Ok(self.values.lock().await.get(key).cloned())
    }

    async fn set_config_value(&self, key: &str, value: serde_json::Value) -> MCPRuntimeResult<()> {
        self.values.lock().await.insert(key.to_string(), value);
        Ok(())
    }
}

pub struct FailingMCPConfigStore;

#[async_trait::async_trait]
impl MCPConfigStore for FailingMCPConfigStore {
    async fn get_config_value(&self, key: &str) -> MCPRuntimeResult<Option<serde_json::Value>> {
        Err(northhing_services_integrations::mcp::MCPRuntimeError::configuration(
            format!("backend unavailable for {key}"),
        ))
    }

    async fn set_config_value(&self, key: &str, _value: serde_json::Value) -> MCPRuntimeResult<()> {
        Err(northhing_services_integrations::mcp::MCPRuntimeError::configuration(
            format!("backend unavailable for {key}"),
        ))
    }
}

pub struct FakeMCPToolCatalogClient {
    pub tools: Vec<MCPTool>,
}

#[async_trait::async_trait]
impl MCPToolCatalogClient for FakeMCPToolCatalogClient {
    async fn list_mcp_tools(&self) -> MCPRuntimeResult<Vec<MCPTool>> {
        Ok(self.tools.clone())
    }
}
