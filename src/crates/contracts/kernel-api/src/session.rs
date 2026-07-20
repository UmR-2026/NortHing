//! Kernel session API and session DTOs.

use crate::error::{KernelError, KernelResult};
use northhing_core_types::SessionKind;

// ── Session IDs ────────────────────────────────────────────────────────────────

pub type SessionId = String;
pub type BranchId = String;

// ── Session DTOs ───────────────────────────────────────────────────────────────

/// Session configuration for creation.
/// Fields mirror core::SessionConfig (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionConfigDto {
    pub workspace_path: Option<String>,
    pub agent_type: String,
    pub model_name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionSummaryDto {
    pub id: SessionId,
    pub name: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionDto {
    pub id: SessionId,
    pub state: SessionStateDto,
    pub kind: SessionKindDto,
}

/// Session state (enumerated from core SessionState at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionStateDto {
    pub status: String,
}

/// Session kind DTO mirroring core SessionKind.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKindDto {
    Standard,
    Subagent,
    EphemeralChild,
}

impl From<SessionKind> for SessionKindDto {
    fn from(kind: SessionKind) -> Self {
        match kind {
            SessionKind::Standard => SessionKindDto::Standard,
            SessionKind::Subagent => SessionKindDto::Subagent,
            SessionKind::EphemeralChild => SessionKindDto::EphemeralChild,
        }
    }
}

/// Session metadata DTO based on services-core SessionMetadata.
/// Fields mirror northhing_services_core::session::session_metadata::SessionMetadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMetadataDto {
    pub session_id: String,
    pub session_name: String,
    pub agent_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_user_dialog_agent_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_submitted_agent_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    pub session_kind: SessionKindDto,
    pub model_name: String,
    pub created_at: u64,
    pub last_active_at: u64,
    pub turn_count: usize,
    pub message_count: usize,
    pub tool_call_count: usize,
    pub status: SessionStatusDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_session_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship: Option<SessionRelationshipDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todos: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deep_review_run_manifest: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deep_review_cache: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unread_completion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_user_attention: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionRelationshipDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_dialog_turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_turn_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_type: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatusDto {
    Active,
    Archived,
    Completed,
}

/// Session branch request DTO.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionBranchDto {
    pub parent_session_id: SessionId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Persistence handle DTO (encapsulates PersistenceManager handle, #28 folding).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistenceHandleDto {
    pub handle_id: String,
}

// ── Message DTOs ────────────────────────────────────────────────────────────────

/// Message role enum mirroring core MessageRole (User/Assistant/Tool/System).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRoleDto {
    User,
    Assistant,
    Tool,
    System,
}

/// Message content enum mirroring core MessageContent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageContentDto {
    Text(String),
    Multimodal { text: String, images: Vec<String> },
    ToolResult {
        tool_id: String,
        tool_name: String,
        result: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        result_for_assistant: Option<String>,
        is_error: bool,
    },
    Mixed {
        #[serde(skip_serializing_if = "Option::is_none")]
        reasoning_content: Option<String>,
        text: String,
        tool_calls: Vec<ToolCallStub>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCallStub {
    pub tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
    #[serde(default)]
    pub is_error: bool,
}

/// Message metadata DTO mirroring core MessageMetadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MessageMetadataDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_reminder_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression_payload: Option<serde_json::Value>,
}

/// Unified message DTO (abnormal item 10 solution: eliminates dual module paths).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MessageDto {
    pub id: String,
    pub role: MessageRoleDto,
    pub content: MessageContentDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MessageMetadataDto>,
}

// ── KernelSessionApi ───────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait KernelSessionApi: Send + Sync {
    /// Create a session (abnormal item 1 solution: SessionConfigDto instead of SessionConfig literal).
    /// Source: #11
    async fn create_session(&self, config: SessionConfigDto) -> Result<SessionId, KernelError>;

    /// List session summaries.
    /// Source: #12
    async fn list_sessions(&self) -> Result<Vec<SessionSummaryDto>, KernelError>;

    /// Get a single session detail (includes state/kind).
    /// Source: #13 #14
    async fn get_session(&self, id: &SessionId) -> Result<SessionDto, KernelError>;

    /// Delete a session.
    /// Source: #4 (coordinator session CRUD)
    async fn delete_session(&self, id: &SessionId) -> Result<(), KernelError>;

    /// Rename a session.
    /// Source: #4
    async fn rename_session(&self, id: &SessionId, name: &str) -> Result<(), KernelError>;

    /// Get session message list (abnormal item 10 solution: unified MessageDto).
    /// Source: #15 #16 #17 #18 #19
    async fn get_messages(&self, session_id: &SessionId) -> Result<Vec<MessageDto>, KernelError>;

    /// Get session persistence metadata.
    /// Source: #67
    async fn get_session_metadata(&self, id: &SessionId) -> Result<SessionMetadataDto, KernelError>;

    /// Create a session branch.
    /// Source: #29
    async fn create_branch(&self, request: SessionBranchDto) -> Result<BranchId, KernelError>;

    /// Get persistence handle (#28 PersistenceManager::new folding scheme).
    /// Source: #28
    async fn get_persistence_handle(&self) -> Result<PersistenceHandleDto, KernelError>;
}
