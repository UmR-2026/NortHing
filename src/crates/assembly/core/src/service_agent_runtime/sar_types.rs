use northhing_runtime_ports::AgentInputAttachment;
use northhing_services_integrations::remote_connect::{
    build_remote_chat_messages, build_remote_model_catalog,
    normalize_remote_model_selection as normalize_remote_model_selection_contract,
    normalize_remote_session_model_id as normalize_remote_session_model_id_contract,
    remote_dialog_submit_outcome_from_scheduler,
    remote_model_selection_needs_config as remote_model_selection_needs_config_contract, ChatImageAttachment,
    ChatMessage, RemoteAssistantWorkspaceFacts, RemoteCancelRuntimeHost, RemoteChatHistoryRound,
    RemoteChatHistoryTextItem, RemoteChatHistoryThinkingItem, RemoteChatHistoryToolCall, RemoteChatHistoryToolItem,
    RemoteChatHistoryTurn, RemoteConnectSubmissionSource, RemoteDefaultModelsConfig, RemoteDialogQueuePriority,
    RemoteDialogResolvedSubmission, RemoteDialogRuntimeHost, RemoteDialogSchedulerOutcomeFact,
    RemoteDialogSubmissionPolicy, RemoteDialogSubmitOutcome, RemoteImageContext, RemoteImageContextAdapter,
    RemoteInitialSyncRuntimeHost, RemoteInteractionRuntimeHost, RemoteModelCapabilityFact, RemoteModelCatalog,
    RemoteModelCatalogFacts, RemoteModelFacts, RemotePollRuntimeHost, RemoteReasoningModeFact,
    RemoteRecentWorkspaceFacts, RemoteSessionMetadata, RemoteSessionRuntimeHost, RemoteSessionStateTracker,
    RemoteSessionTrackerHost, RemoteTerminalPrewarmRequest, RemoteWorkspaceFacts, RemoteWorkspaceFileRuntimeHost,
    RemoteWorkspaceKind as RemoteConnectWorkspaceKind, RemoteWorkspaceRuntimeHost, RemoteWorkspaceUpdate,
};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::agentic::image_analysis::ImageContextData;
use crate::service::config::types::{AIConfig, GlobalConfig, ModelCapability, ReasoningMode};
use crate::service::session::{DialogTurnData, TurnStatus};

/// Max thumbnail size per remote chat image sent to mobile (100 KB).
const MOBILE_IMAGE_MAX_BYTES: usize = 100 * 1024;

pub(crate) fn current_workspace_path() -> Option<std::path::PathBuf> {
    crate::service::workspace::global_workspace_service().and_then(|service| service.try_get_current_workspace_path())
}

pub(crate) fn remote_workspace_kind(kind: crate::service::workspace::WorkspaceKind) -> RemoteConnectWorkspaceKind {
    match kind {
        crate::service::workspace::WorkspaceKind::Normal => RemoteConnectWorkspaceKind::Normal,
        crate::service::workspace::WorkspaceKind::Assistant => RemoteConnectWorkspaceKind::Assistant,
        crate::service::workspace::WorkspaceKind::Remote => RemoteConnectWorkspaceKind::Remote,
    }
}

fn git_branch_for_workspace_path(path: &std::path::Path) -> Option<String> {
    let path_str = path.to_string_lossy();
    northhing_services_integrations::git::execute_git_command_sync(&path_str, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "HEAD")
}

pub(crate) async fn current_remote_workspace_facts() -> Option<RemoteWorkspaceFacts> {
    let workspace_service = crate::service::workspace::global_workspace_service()?;
    workspace_service.current_workspace().await.map(|workspace| {
        let root_path = workspace.root_path.clone();
        RemoteWorkspaceFacts {
            path: root_path.to_string_lossy().to_string(),
            name: workspace.name,
            git_branch: git_branch_for_workspace_path(&root_path),
            kind: remote_workspace_kind(workspace.workspace_kind),
            assistant_id: workspace.assistant_id,
        }
    })
}

pub(crate) async fn open_workspace_with_snapshot(
    path: &str,
    snapshot_log_context: &str,
) -> Result<RemoteWorkspaceUpdate, String> {
    let workspace_service = crate::service::workspace::global_workspace_service()
        .ok_or_else(|| "Workspace service not available".to_string())?;
    let path_buf = std::path::PathBuf::from(path);
    let info = workspace_service
        .open_workspace(path_buf)
        .await
        .map_err(|error| error.to_string())?;
    if let Err(error) =
        crate::service::snapshot::initialize_snapshot_manager_for_workspace(info.root_path.clone(), None).await
    {
        error!("Failed to initialize snapshot after {snapshot_log_context}: {error}");
    }
    Ok(RemoteWorkspaceUpdate {
        path: info.root_path.to_string_lossy().to_string(),
        name: info.name,
    })
}

pub(crate) async fn load_remote_session_metadata_for_workspace(
    workspace_path: &std::path::Path,
) -> Result<Vec<RemoteSessionMetadata>, String> {
    let workspace_path_display = workspace_path.to_string_lossy().to_string();
    let path_manager =
        crate::infrastructure::PathManager::new().map_err(|_| "Failed to initialize path manager".to_string())?;
    let path_manager = std::sync::Arc::new(path_manager);
    let store = crate::agentic::persistence::PersistenceManager::new(path_manager).map_err(|error| {
        debug!("PersistenceManager init failed for {workspace_path_display}: {error}");
        format!("Failed to initialize session storage: {error}")
    })?;
    let metadata = store.list_session_metadata(workspace_path).await.map_err(|error| {
        debug!("Session list read failed for {workspace_path_display}: {error}");
        format!("Failed to list sessions for workspace: {error}")
    })?;

    Ok(metadata
        .into_iter()
        .map(|session| RemoteSessionMetadata {
            session_id: session.session_id,
            name: session.session_name,
            agent_type: session.agent_type,
            created_at_ms: session.created_at,
            last_active_at_ms: session.last_active_at,
            turn_count: session.turn_count,
        })
        .collect())
}

pub(crate) fn normalize_remote_session_model_id(model_id: Option<String>) -> Option<String> {
    normalize_remote_session_model_id_contract(model_id.as_deref())
}

pub(crate) fn normalize_remote_model_selection(
    requested_model_id: &str,
    ai_config: Option<&AIConfig>,
) -> Result<String, String> {
    if remote_model_selection_needs_config(requested_model_id) && ai_config.is_none() {
        return Err("Config service not available".to_string());
    }

    normalize_remote_model_selection_contract(requested_model_id, |model_id| {
        ai_config.and_then(|config| config.resolve_model_reference(model_id))
    })
}

pub(crate) fn remote_model_selection_needs_config(requested_model_id: &str) -> bool {
    remote_model_selection_needs_config_contract(requested_model_id)
}

pub(crate) fn remote_model_capability_fact(capability: ModelCapability) -> RemoteModelCapabilityFact {
    match capability {
        ModelCapability::TextChat => RemoteModelCapabilityFact::TextChat,
        ModelCapability::ImageUnderstanding => RemoteModelCapabilityFact::ImageUnderstanding,
        ModelCapability::ImageGeneration => RemoteModelCapabilityFact::ImageGeneration,
        ModelCapability::Embedding => RemoteModelCapabilityFact::Embedding,
        ModelCapability::Search => RemoteModelCapabilityFact::Search,
        ModelCapability::CodeSpecialized => RemoteModelCapabilityFact::CodeSpecialized,
        ModelCapability::FunctionCalling => RemoteModelCapabilityFact::FunctionCalling,
        ModelCapability::SpeechRecognition => RemoteModelCapabilityFact::SpeechRecognition,
    }
}

pub(crate) fn remote_reasoning_mode_fact(reasoning_mode: ReasoningMode) -> RemoteReasoningModeFact {
    match reasoning_mode {
        ReasoningMode::Default => RemoteReasoningModeFact::Default,
        ReasoningMode::Enabled => RemoteReasoningModeFact::Enabled,
        ReasoningMode::Disabled => RemoteReasoningModeFact::Disabled,
        ReasoningMode::Adaptive => RemoteReasoningModeFact::Adaptive,
    }
}

/// Compress a base64 data-URL image to a small thumbnail for mobile display.
/// Falls back to the original if decoding/compression fails or the image is
/// already within `max_bytes`.
fn compress_remote_chat_data_url_for_mobile(data_url: &str, max_bytes: usize) -> String {
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use image::imageops::FilterType;

    const MAX_THUMBNAIL_DIM: u32 = 400;

    let Some(comma_pos) = data_url.find(',') else {
        return data_url.to_string();
    };
    let b64_data = &data_url[comma_pos + 1..];

    if b64_data.len() * 3 / 4 <= max_bytes {
        return data_url.to_string();
    }

    let Ok(raw_bytes) = BASE64.decode(b64_data) else {
        return data_url.to_string();
    };

    let Ok(img) = image::load_from_memory(&raw_bytes) else {
        return data_url.to_string();
    };

    let resized = if img.width() > MAX_THUMBNAIL_DIM || img.height() > MAX_THUMBNAIL_DIM {
        img.resize(MAX_THUMBNAIL_DIM, MAX_THUMBNAIL_DIM, FilterType::Triangle)
    } else {
        img
    };

    fn encode_jpeg(img: &image::DynamicImage, quality: u8) -> Option<Vec<u8>> {
        let mut buf = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
        img.write_with_encoder(encoder).ok()?;
        Some(buf)
    }

    for quality in [75u8, 60, 45, 30] {
        if let Some(buf) = encode_jpeg(&resized, quality) {
            if buf.len() <= max_bytes || quality == 30 {
                let b64 = BASE64.encode(&buf);
                return format!("data:image/jpeg;base64,{b64}");
            }
        }
    }

    data_url.to_string()
}

/// Convert persisted turns into mobile ChatMessages.
/// This is the same data source the desktop frontend uses.
pub(crate) fn remote_chat_messages_from_turns(turns: &[DialogTurnData]) -> Vec<ChatMessage> {
    let projected_turns = turns
        .iter()
        .filter(|turn| turn.kind.is_model_visible())
        .map(remote_chat_history_turn_from_core_turn)
        .collect::<Vec<_>>();
    build_remote_chat_messages(projected_turns)
}

pub(crate) fn remote_chat_history_turn_from_core_turn(turn: &DialogTurnData) -> RemoteChatHistoryTurn {
    let user_images = turn
        .user_message
        .metadata
        .as_ref()
        .and_then(|m| m.get("images"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let name = v.get("name")?.as_str()?.to_string();
                    let raw_url = v.get("data_url")?.as_str()?;
                    let data_url = compress_remote_chat_data_url_for_mobile(raw_url, MOBILE_IMAGE_MAX_BYTES);
                    Some(ChatImageAttachment { name, data_url })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Prefer original_text from metadata (pre-enhancement) for display.
    let user_display_content = turn
        .user_message
        .metadata
        .as_ref()
        .and_then(|m| m.get("original_text"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| strip_remote_user_input_tags(&turn.user_message.content));

    let rounds = turn
        .model_rounds
        .iter()
        .map(|round| RemoteChatHistoryRound {
            start_time_ms: round.start_time,
            end_time_ms: round.end_time,
            text_items: round
                .text_items
                .iter()
                .map(|item| RemoteChatHistoryTextItem {
                    content: item.content.clone(),
                    order_index: item.order_index,
                    is_subagent: item.is_subagent_item.unwrap_or(false),
                })
                .collect(),
            thinking_items: round
                .thinking_items
                .iter()
                .map(|item| RemoteChatHistoryThinkingItem {
                    content: item.content.clone(),
                    order_index: item.order_index,
                    is_subagent: item.is_subagent_item.unwrap_or(false),
                })
                .collect(),
            tool_items: round
                .tool_items
                .iter()
                .map(|item| RemoteChatHistoryToolItem {
                    id: item.id.clone(),
                    name: item.tool_name.clone(),
                    call: RemoteChatHistoryToolCall {
                        id: item.tool_call.id.clone(),
                        input: item.tool_call.input.clone(),
                    },
                    has_result: item.tool_result.is_some(),
                    status: item.status.clone(),
                    duration_ms: item.duration_ms,
                    start_ms: item.start_time,
                    order_index: item.order_index,
                    is_subagent: item.is_subagent_item.unwrap_or(false),
                })
                .collect(),
        })
        .collect();

    RemoteChatHistoryTurn {
        turn_id: turn.turn_id.clone(),
        user_message_id: turn.user_message.id.clone(),
        user_display_content,
        user_timestamp_ms: turn.user_message.timestamp,
        user_images,
        is_in_progress: turn.status == TurnStatus::InProgress,
        start_time_ms: turn.start_time,
        rounds,
    }
}

pub(crate) fn strip_remote_user_input_tags(content: &str) -> String {
    let s = crate::agentic::core::strip_prompt_markup(content);
    if s.starts_with("User uploaded") {
        if let Some(pos) = s.find("User's question:\n") {
            return s[pos + "User's question:\n".len()..].trim().to_string();
        }
    }
    s
}

pub(crate) async fn resolve_session_model_id(session_id: &str) -> Option<String> {
    let coordinator = crate::agentic::coordination::global_coordinator()?;
    let session_manager = coordinator.session_manager();

    if let Some(session) = session_manager.get_session(session_id) {
        return normalize_remote_session_model_id(session.config.model_id.clone());
    }

    let workspace_path =
        super::sar_dispatch::CoreServiceAgentRuntime::resolve_session_workspace_path(session_id).await?;
    coordinator
        .restore_session(&workspace_path, session_id)
        .await
        .ok()
        .and_then(|session| normalize_remote_session_model_id(session.config.model_id.clone()))
}

pub(crate) fn core_dialog_submission_policy(
    policy: RemoteDialogSubmissionPolicy,
) -> crate::agentic::coordination::DialogSubmissionPolicy {
    let trigger_source = match policy.source {
        RemoteConnectSubmissionSource::Relay => crate::agentic::coordination::DialogTriggerSource::RemoteRelay,
        RemoteConnectSubmissionSource::Bot => crate::agentic::coordination::DialogTriggerSource::Bot,
    };
    let queue_priority = match policy.queue_priority {
        RemoteDialogQueuePriority::Low => crate::agentic::coordination::DialogQueuePriority::Low,
        RemoteDialogQueuePriority::Normal => crate::agentic::coordination::DialogQueuePriority::Normal,
        RemoteDialogQueuePriority::High => crate::agentic::coordination::DialogQueuePriority::High,
    };

    crate::agentic::coordination::DialogSubmissionPolicy::new(
        trigger_source,
        queue_priority,
        policy.skip_tool_confirmation,
    )
}

pub(crate) fn remote_dialog_scheduler_outcome_fact(
    outcome: crate::agentic::coordination::DialogSubmitOutcome,
) -> RemoteDialogSchedulerOutcomeFact {
    match outcome {
        crate::agentic::coordination::DialogSubmitOutcome::Started { session_id, turn_id } => {
            RemoteDialogSchedulerOutcomeFact::Started { session_id, turn_id }
        }
        crate::agentic::coordination::DialogSubmitOutcome::Queued { session_id, turn_id } => {
            RemoteDialogSchedulerOutcomeFact::Queued { session_id, turn_id }
        }
    }
}

pub(crate) fn agent_input_attachment_from_image_context(context: ImageContextData) -> AgentInputAttachment {
    let mut metadata = serde_json::Map::new();
    if let Some(image_path) = context.image_path {
        metadata.insert("imagePath".to_string(), serde_json::Value::String(image_path));
    }
    if let Some(data_url) = context.data_url {
        metadata.insert("dataUrl".to_string(), serde_json::Value::String(data_url));
    }
    metadata.insert("mimeType".to_string(), serde_json::Value::String(context.mime_type));
    if let Some(context_metadata) = context.metadata {
        metadata.insert("metadata".to_string(), context_metadata);
    }

    AgentInputAttachment {
        kind: "remote_image".to_string(),
        id: context.id,
        metadata,
    }
}

impl RemoteImageContextAdapter for ImageContextData {
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
