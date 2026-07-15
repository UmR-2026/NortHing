//! Remote-connect request builders and shared DTOs (Round 11 split).
//!
//! Owns the `build_remote_*` prefix cluster plus the request/image/context DTOs
//! shared by `command_handlers` and `session_tracker`. Other siblings consume
//! these types through the `pub use` re-exports in `mod.rs`.
//!
//! Domain split (Round 11, sub-domain pattern):
//! - build_remote_session_create_request / submission / image_attachment /
//!   image_submission_request / image_contexts / execution_image_contexts
//! - build_remote_model_catalog + RemoteModelCatalog/Facts/Config + capabilities
//! - build_remote_chat_messages + RemoteChatHistory* DTOs + ChatMessage DTO
//! - normalize_remote_* + remote_session_restore_target

use serde::{Deserialize, Serialize};

use super::RemoteConnectSubmissionSource;
use northhing_runtime_ports::{AgentInputAttachment, AgentSessionCreateRequest, AgentSubmissionRequest};

/// Image sent from a remote client as a base64 data URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageAttachment {
    pub name: String,
    pub data_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatImageAttachment {
    pub name: String,
    pub data_url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<RemoteToolStatus>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ChatMessageItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ChatImageAttachment>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessageItem {
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<RemoteToolStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_subagent: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteChatHistoryTurn {
    pub turn_id: String,
    pub user_message_id: String,
    pub user_display_content: String,
    pub user_timestamp_ms: u64,
    pub user_images: Vec<ChatImageAttachment>,
    pub is_in_progress: bool,
    pub start_time_ms: u64,
    pub rounds: Vec<RemoteChatHistoryRound>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteChatHistoryRound {
    pub start_time_ms: u64,
    pub end_time_ms: Option<u64>,
    pub text_items: Vec<RemoteChatHistoryTextItem>,
    pub thinking_items: Vec<RemoteChatHistoryThinkingItem>,
    pub tool_items: Vec<RemoteChatHistoryToolItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteChatHistoryTextItem {
    pub content: String,
    pub order_index: Option<usize>,
    pub is_subagent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteChatHistoryThinkingItem {
    pub content: String,
    pub order_index: Option<usize>,
    pub is_subagent: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteChatHistoryToolItem {
    pub id: String,
    pub name: String,
    pub call: RemoteChatHistoryToolCall,
    pub has_result: bool,
    pub status: Option<String>,
    pub duration_ms: Option<u64>,
    pub start_ms: u64,
    pub order_index: Option<usize>,
    pub is_subagent: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteChatHistoryToolCall {
    pub id: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteToolStatus {
    pub id: String,
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RemoteDefaultModelsConfig {
    pub primary: Option<String>,
    pub fast: Option<String>,
    pub search: Option<String>,
    pub image_understanding: Option<String>,
    pub image_generation: Option<String>,
    pub speech_recognition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteModelConfig {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub model_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    pub enabled: bool,
    pub capabilities: Vec<String>,
    pub enable_thinking_process: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budget_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteModelCatalog {
    pub version: u64,
    pub models: Vec<RemoteModelConfig>,
    pub default_models: RemoteDefaultModelsConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_model_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteModelCapabilityFact {
    TextChat,
    ImageUnderstanding,
    ImageGeneration,
    Embedding,
    Search,
    CodeSpecialized,
    FunctionCalling,
    SpeechRecognition,
}

impl RemoteModelCapabilityFact {
    const fn wire_value(self) -> &'static str {
        match self {
            RemoteModelCapabilityFact::TextChat => "text_chat",
            RemoteModelCapabilityFact::ImageUnderstanding => "image_understanding",
            RemoteModelCapabilityFact::ImageGeneration => "image_generation",
            RemoteModelCapabilityFact::Embedding => "embedding",
            RemoteModelCapabilityFact::Search => "search",
            RemoteModelCapabilityFact::CodeSpecialized => "code_specialized",
            RemoteModelCapabilityFact::FunctionCalling => "function_calling",
            RemoteModelCapabilityFact::SpeechRecognition => "speech_recognition",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteReasoningModeFact {
    Default,
    Enabled,
    Disabled,
    Adaptive,
}

impl RemoteReasoningModeFact {
    const fn wire_value(self) -> &'static str {
        match self {
            RemoteReasoningModeFact::Default => "default",
            RemoteReasoningModeFact::Enabled => "enabled",
            RemoteReasoningModeFact::Disabled => "disabled",
            RemoteReasoningModeFact::Adaptive => "adaptive",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteModelFacts {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub model_name: String,
    pub context_window: Option<u32>,
    pub enabled: bool,
    pub capabilities: Vec<RemoteModelCapabilityFact>,
    pub enable_thinking_process: bool,
    pub reasoning_mode: Option<RemoteReasoningModeFact>,
    pub reasoning_effort: Option<String>,
    pub thinking_budget_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteModelCatalogFacts {
    pub last_modified_ms: i64,
    pub models: Vec<RemoteModelFacts>,
    pub default_models: RemoteDefaultModelsConfig,
    pub session_model_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteModelCatalogPollDelta {
    pub changed: bool,
    pub catalog: Option<RemoteModelCatalog>,
}

/// Portable image context produced from legacy remote image payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteImageContext {
    pub id: String,
    pub image_path: Option<String>,
    pub data_url: Option<String>,
    pub mime_type: String,
    pub metadata: Option<serde_json::Value>,
}

pub trait RemoteImageContextAdapter {
    fn from_remote_image_context(context: RemoteImageContext) -> Self;
}

pub fn build_remote_session_create_request(
    session_name: impl Into<String>,
    agent_type: impl Into<String>,
    workspace_path: Option<impl Into<String>>,
    source: RemoteConnectSubmissionSource,
) -> AgentSessionCreateRequest {
    let mut metadata = serde_json::Map::new();
    metadata.insert(
        "source".to_string(),
        serde_json::Value::String(source.metadata_source().to_string()),
    );

    AgentSessionCreateRequest {
        session_name: session_name.into(),
        agent_type: agent_type.into(),
        workspace_path: workspace_path.map(Into::into),
        metadata,
    }
}

pub fn build_remote_submission_request(
    session_id: impl Into<String>,
    message: impl Into<String>,
    turn_id: Option<String>,
    source: RemoteConnectSubmissionSource,
) -> AgentSubmissionRequest {
    AgentSubmissionRequest {
        session_id: session_id.into(),
        message: message.into(),
        turn_id,
        source: Some(source.agent_submission_source()),
        attachments: Vec::new(),
        metadata: serde_json::Map::new(),
    }
}

pub fn build_remote_image_attachment(index: usize, attachment: &ImageAttachment) -> AgentInputAttachment {
    AgentInputAttachment::remote_image(
        format!("remote-image-{}", index + 1),
        attachment.name.clone(),
        attachment.data_url.clone(),
    )
}

pub fn build_remote_image_submission_request(
    session_id: impl Into<String>,
    message: impl Into<String>,
    turn_id: Option<String>,
    source: RemoteConnectSubmissionSource,
    images: &[ImageAttachment],
) -> AgentSubmissionRequest {
    AgentSubmissionRequest {
        session_id: session_id.into(),
        message: message.into(),
        turn_id,
        source: Some(source.agent_submission_source()),
        attachments: images
            .iter()
            .enumerate()
            .map(|(index, image)| build_remote_image_attachment(index, image))
            .collect(),
        metadata: serde_json::Map::new(),
    }
}

pub fn build_remote_image_contexts(images: Option<&[ImageAttachment]>) -> Vec<RemoteImageContext> {
    let Some(images) = images.filter(|images| !images.is_empty()) else {
        return Vec::new();
    };

    images
        .iter()
        .map(|image| {
            let mime_type = image
                .data_url
                .split_once(',')
                .and_then(|(header, _)| header.strip_prefix("data:").and_then(|rest| rest.split(';').next()))
                .unwrap_or("image/png")
                .to_string();

            RemoteImageContext {
                id: format!("remote_img_{}", uuid::Uuid::new_v4()),
                image_path: None,
                data_url: Some(image.data_url.clone()),
                mime_type,
                metadata: Some(serde_json::json!({
                    "name": image.name,
                    "source": "remote"
                })),
            }
        })
        .collect()
}

pub fn resolve_remote_execution_image_contexts<T>(
    legacy_images: Option<&[ImageAttachment]>,
    image_contexts: Option<Vec<T>>,
    legacy_contexts: impl FnOnce(Option<&[ImageAttachment]>) -> Vec<T>,
) -> Vec<T> {
    image_contexts.unwrap_or_else(|| legacy_contexts(legacy_images))
}

pub fn remote_session_restore_target(session_exists: bool, binding_workspace: Option<&str>) -> Option<&str> {
    if session_exists {
        None
    } else {
        binding_workspace
    }
}

pub fn build_remote_model_catalog(facts: RemoteModelCatalogFacts) -> RemoteModelCatalog {
    RemoteModelCatalog {
        version: facts.last_modified_ms.max(0) as u64,
        models: facts
            .models
            .into_iter()
            .map(|model| RemoteModelConfig {
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
                    .map(|capability| capability.wire_value().to_string())
                    .collect(),
                enable_thinking_process: model.enable_thinking_process,
                reasoning_mode: model
                    .reasoning_mode
                    .map(|reasoning_mode| reasoning_mode.wire_value().to_string()),
                reasoning_effort: model.reasoning_effort,
                thinking_budget_tokens: model.thinking_budget_tokens,
            })
            .collect(),
        default_models: facts.default_models,
        session_model_id: facts.session_model_id,
    }
}

pub fn normalize_remote_session_model_id(model_id: Option<&str>) -> Option<String> {
    match model_id {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() || trimmed == "default" {
                Some("auto".to_string())
            } else {
                Some(trimmed.to_string())
            }
        }
        None => Some("auto".to_string()),
    }
}

pub fn remote_model_selection_needs_config(requested_model_id: &str) -> bool {
    let requested_model_id = requested_model_id.trim();
    !requested_model_id.is_empty() && !matches!(requested_model_id, "auto" | "default" | "primary" | "fast")
}

pub fn normalize_remote_model_selection(
    requested_model_id: &str,
    resolve_model_reference: impl FnOnce(&str) -> Option<String>,
) -> Result<String, String> {
    let requested_model_id = requested_model_id.trim();
    if requested_model_id.is_empty() {
        return Err("model_id is required".to_string());
    }

    if matches!(requested_model_id, "auto" | "default" | "primary" | "fast") {
        return Ok(if requested_model_id == "default" {
            "auto".to_string()
        } else {
            requested_model_id.to_string()
        });
    }

    resolve_model_reference(requested_model_id).ok_or_else(|| format!("Unknown model selection: {requested_model_id}"))
}

pub fn build_remote_chat_messages(turns: Vec<RemoteChatHistoryTurn>) -> Vec<ChatMessage> {
    let mut result = Vec::new();

    for turn in turns {
        result.push(ChatMessage {
            id: turn.user_message_id,
            role: "user".to_string(),
            content: turn.user_display_content,
            timestamp: (turn.user_timestamp_ms / 1000).to_string(),
            metadata: None,
            tools: None,
            thinking: None,
            items: None,
            images: if turn.user_images.is_empty() {
                None
            } else {
                Some(turn.user_images)
            },
        });

        if turn.is_in_progress {
            continue;
        }

        struct OrderedEntry {
            order_index: Option<usize>,
            sequence: usize,
            round_idx: usize,
            item: ChatMessageItem,
        }

        let mut ordered = Vec::new();
        let mut tools_flat = Vec::new();
        let mut thinking_parts = Vec::new();
        let mut text_parts = Vec::new();
        let mut sequence = 0usize;
        let assistant_ts = turn
            .rounds
            .last()
            .map(|round| round.end_time_ms.unwrap_or(round.start_time_ms))
            .unwrap_or(turn.start_time_ms);

        for (round_idx, round) in turn.rounds.into_iter().enumerate() {
            for item in round.thinking_items {
                if item.is_subagent || item.content.is_empty() {
                    continue;
                }
                thinking_parts.push(item.content.clone());
                ordered.push(OrderedEntry {
                    order_index: item.order_index,
                    sequence,
                    round_idx,
                    item: ChatMessageItem {
                        item_type: "thinking".to_string(),
                        content: Some(item.content.clone()),
                        tool: None,
                        is_subagent: None,
                    },
                });
                sequence += 1;
            }

            for item in round.text_items {
                if item.is_subagent || item.content.is_empty() {
                    continue;
                }
                text_parts.push(item.content.clone());
                ordered.push(OrderedEntry {
                    order_index: item.order_index,
                    sequence,
                    round_idx,
                    item: ChatMessageItem {
                        item_type: "text".to_string(),
                        content: Some(item.content.clone()),
                        tool: None,
                        is_subagent: None,
                    },
                });
                sequence += 1;
            }

            for item in round.tool_items {
                if item.is_subagent {
                    continue;
                }
                let status = item
                    .status
                    .as_deref()
                    .unwrap_or(if item.has_result { "completed" } else { "running" });
                let tool_status = RemoteToolStatus {
                    id: item.id,
                    name: item.name.clone(),
                    status: status.to_string(),
                    duration_ms: item.duration_ms,
                    start_ms: Some(item.start_ms),
                    input_preview: make_slim_tool_params(&item.call.input),
                    tool_input: if item.name == "AskUserQuestion" || item.name == "Task" || item.name == "TodoWrite" {
                        Some(item.call.input.clone())
                    } else {
                        None
                    },
                };
                tools_flat.push(tool_status.clone());
                ordered.push(OrderedEntry {
                    order_index: item.order_index,
                    sequence,
                    round_idx,
                    item: ChatMessageItem {
                        item_type: "tool".to_string(),
                        content: None,
                        tool: Some(tool_status),
                        is_subagent: None,
                    },
                });
                sequence += 1;
            }
        }

        ordered.sort_by(|a, b| {
            let round_cmp = a.round_idx.cmp(&b.round_idx);
            if round_cmp != std::cmp::Ordering::Equal {
                return round_cmp;
            }
            match (a.order_index, b.order_index) {
                (Some(a_idx), Some(b_idx)) => a_idx.cmp(&b_idx),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.sequence.cmp(&b.sequence),
            }
        });

        let items: Vec<ChatMessageItem> = ordered.into_iter().map(|entry| entry.item).collect();

        result.push(ChatMessage {
            id: format!("{}_assistant", turn.turn_id),
            role: "assistant".to_string(),
            content: text_parts.join("\n\n"),
            timestamp: (assistant_ts / 1000).to_string(),
            metadata: None,
            tools: if tools_flat.is_empty() { None } else { Some(tools_flat) },
            thinking: if thinking_parts.is_empty() {
                None
            } else {
                Some(thinking_parts.join("\n\n"))
            },
            items: if items.is_empty() { None } else { Some(items) },
            images: None,
        });
    }

    result
}

/// Build a slim version of tool params for remote preview payloads.
///
/// Large string values such as file contents and diffs are omitted, while
/// short structured fields stay available for remote clients that need to
/// render tool details.
pub fn make_slim_tool_params(params: &serde_json::Value) -> Option<String> {
    match params {
        serde_json::Value::Object(obj) => {
            let slim: serde_json::Map<String, serde_json::Value> = obj
                .iter()
                .filter_map(|(key, value)| match value {
                    serde_json::Value::String(text) if text.len() > 200 => None,
                    _ => Some((key.clone(), value.clone())),
                })
                .collect();
            if slim.is_empty() {
                return None;
            }
            serde_json::to_string(&serde_json::Value::Object(slim)).ok()
        }
        serde_json::Value::String(text) => Some(text.chars().take(200).collect()),
        _ => None,
    }
}
