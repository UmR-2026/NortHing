//! Session identity, relationship, and storage-layout wrapper types.
//!
//! This sibling owns the top-level session DTOs: [`SessionRelationshipKind`],
//! [`SessionRelationship`], [`SessionMetadata`] (with its mutators), session
//! status / list wrappers, and the on-disk wrapper structs
//! ([`StoredSessionMetadataFile`], [`StoredSessionIndexFile`]). Storage layout
//! rules and session metadata construction helpers live in `crate::session`
//! siblings (`layout`, `metadata`, `metadata_store`) — not here.
//!
//! All items are publicly re-exported through `crate::session::types` (the
//! facade) so external imports such as
//! `northhing_services_core::session::SessionMetadata` keep working.

use northhing_core_types::SessionKind;
use serde::{Deserialize, Serialize};

/// Bumped whenever the on-disk shape of `StoredSessionMetadataFile` /
/// `StoredSessionIndexFile` changes in an incompatible way.
pub const SESSION_STORAGE_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionRelationshipKind {
    Btw,
    Review,
    DeepReview,
    Miniapp,
    Subagent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionRelationship {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<SessionRelationshipKind>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "parent_session_id")]
    pub parent_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "parent_request_id")]
    pub parent_request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "parent_dialog_turn_id")]
    pub parent_dialog_turn_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "parent_turn_index")]
    pub parent_turn_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "parent_tool_call_id")]
    pub parent_tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "subagent_type")]
    pub subagent_type: Option<String>,
}

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMetadata {
    /// Session ID
    #[serde(alias = "session_id")]
    pub session_id: String,

    /// Session name (user-editable)
    #[serde(alias = "session_name")]
    pub session_name: String,

    /// Agent type
    #[serde(alias = "agent_type")]
    pub agent_type: String,
    /// Mode of the last surviving user dialog turn in the persisted history.
    ///
    /// This follows rollback and turn-truncation semantics and is used for
    /// first-entry vs ongoing mode reminders.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "last_user_dialog_agent_type"
    )]
    pub last_user_dialog_agent_type: Option<String>,
    /// Mode of the most recent user submission accepted by the scheduler.
    ///
    /// This is a session-level prompt-cache guard signal and intentionally does
    /// not rewind when history is rolled back.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "last_submitted_agent_type"
    )]
    pub last_submitted_agent_type: Option<String>,

    /// Creator identity for future permission checks
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "created_by")]
    pub created_by: Option<String>,
    #[serde(default, alias = "session_kind", alias = "sessionKind")]
    pub session_kind: SessionKind,

    /// Model name
    #[serde(alias = "model_name")]
    pub model_name: String,

    /// Created time (Unix timestamp ms)
    #[serde(alias = "created_at")]
    pub created_at: u64,

    /// Last active time (Unix timestamp ms)
    #[serde(alias = "last_active_at")]
    pub last_active_at: u64,

    /// Turn count
    #[serde(alias = "turn_count")]
    pub turn_count: usize,

    /// Total message count (user + AI)
    #[serde(alias = "message_count")]
    pub message_count: usize,

    /// Total tool call count
    #[serde(alias = "tool_call_count")]
    pub tool_call_count: usize,

    /// Session status
    pub status: SessionStatus,

    /// Terminal session ID (if any)
    #[serde(skip_serializing_if = "Option::is_none", alias = "terminal_session_id")]
    pub terminal_session_id: Option<String>,

    /// Snapshot session ID (if any)
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "sandbox_session_id",
        alias = "sandboxSessionId"
    )]
    pub snapshot_session_id: Option<String>,

    /// Tags (for categorization and search)
    #[serde(default)]
    pub tags: Vec<String>,

    /// Custom metadata
    #[serde(skip_serializing_if = "Option::is_none", alias = "custom_metadata")]
    pub custom_metadata: Option<serde_json::Value>,

    /// Structured child-session relationship metadata.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "relationship",
        alias = "session_relationship",
        alias = "sessionRelationship"
    )]
    pub relationship: Option<SessionRelationship>,

    /// Todo list (for persisting the session's todo state)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todos: Option<serde_json::Value>,

    /// Deep Review run manifest for this session, when the session was launched
    /// from Code Review Team.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "deep_review_run_manifest",
        alias = "deepReviewRunManifest"
    )]
    pub deep_review_run_manifest: Option<serde_json::Value>,

    /// Cached reviewer outputs from previous deep review runs in this session.
    /// Keyed by packet_id, value is the reviewer's output text.
    /// Used for incremental review: when the fingerprint matches, skip re-dispatching.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "deep_review_cache",
        alias = "deepReviewCache"
    )]
    pub deep_review_cache: Option<serde_json::Value>,

    /// Workspace path this session belongs to (normalized source workspace root, not mirror dir)
    #[serde(skip_serializing_if = "Option::is_none", alias = "workspace_path")]
    pub workspace_path: Option<String>,

    /// Unified hostname for workspace identity: `localhost` for local workspaces,
    /// SSH host for remote workspaces.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "workspace_hostname")]
    pub workspace_hostname: Option<String>,

    /// Unread completion status for the session.
    /// 'completed' → green dot, 'error' → red dot.
    /// Cleared after the user switches to the session and the content renders.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "unread_completion",
        alias = "unreadCompletion"
    )]
    pub unread_completion: Option<String>,

    /// High-priority attention status for the session.
    /// Set when the session requires user action while not the active session.
    /// 'ask_user' → pending AskUserQuestion waiting for answer.
    /// 'tool_confirm' → pending tool confirmations.
    /// Takes precedence over unread_completion in the UI.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "needs_user_attention",
        alias = "needsUserAttention"
    )]
    pub needs_user_attention: Option<String>,
}

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Archived,
    Completed,
}

/// Session list (metadata for all sessions)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionList {
    pub sessions: Vec<SessionMetadata>,
    #[serde(alias = "last_updated")]
    pub last_updated: u64,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSessionMetadataFile {
    pub schema_version: u32,
    #[serde(flatten)]
    pub metadata: SessionMetadata,
}

impl StoredSessionMetadataFile {
    pub fn new(metadata: SessionMetadata) -> Self {
        Self {
            schema_version: SESSION_STORAGE_SCHEMA_VERSION,
            metadata,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSessionIndexFile {
    pub schema_version: u32,
    pub updated_at: u64,
    #[serde(default)]
    pub metadata_file_count: usize,
    pub sessions: Vec<SessionMetadata>,
}

impl StoredSessionIndexFile {
    pub fn new(updated_at: u64, sessions: Vec<SessionMetadata>) -> Self {
        let metadata_file_count = sessions.len();
        Self::with_metadata_file_count(updated_at, sessions, metadata_file_count)
    }

    pub fn with_metadata_file_count(
        updated_at: u64,
        sessions: Vec<SessionMetadata>,
        metadata_file_count: usize,
    ) -> Self {
        Self {
            schema_version: SESSION_STORAGE_SCHEMA_VERSION,
            updated_at,
            metadata_file_count,
            sessions,
        }
    }
}

impl Default for SessionList {
    fn default() -> Self {
        Self {
            sessions: Vec::new(),
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            version: "1.0".to_string(),
        }
    }
}

impl SessionMetadata {
    /// Creates a new session metadata.
    pub fn new(session_id: String, session_name: String, agent_type: String, model_name: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            session_id,
            session_name,
            agent_type,
            last_user_dialog_agent_type: None,
            last_submitted_agent_type: None,
            created_by: None,
            session_kind: SessionKind::Standard,
            model_name,
            created_at: now,
            last_active_at: now,
            turn_count: 0,
            message_count: 0,
            tool_call_count: 0,
            status: SessionStatus::Active,
            terminal_session_id: None,
            snapshot_session_id: None,
            tags: Vec::new(),
            custom_metadata: None,
            relationship: None,
            todos: None,
            deep_review_run_manifest: None,
            deep_review_cache: None,
            workspace_path: None,
            workspace_hostname: None,
            unread_completion: None,
            needs_user_attention: None,
        }
    }

    /// Updates the last active time.
    pub fn touch(&mut self) {
        self.last_active_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }

    /// Increments the turn count.
    pub fn increment_turn(&mut self) {
        self.turn_count += 1;
    }

    /// Adds to the message count.
    pub fn add_messages(&mut self, count: usize) {
        self.message_count += count;
    }

    /// Adds to the tool call count.
    pub fn add_tool_calls(&mut self, count: usize) {
        self.tool_call_count += count;
    }

    pub fn is_subagent(&self) -> bool {
        matches!(self.session_kind, SessionKind::Subagent)
    }

    pub fn is_standard(&self) -> bool {
        matches!(self.session_kind, SessionKind::Standard)
    }

    pub fn is_internal_hidden(&self) -> bool {
        matches!(self.session_kind, SessionKind::Subagent | SessionKind::EphemeralChild)
    }

    pub fn is_legacy_leaked_subagent_candidate(&self) -> bool {
        let Some(created_by) = self.created_by.as_deref() else {
            return false;
        };
        if !created_by.starts_with("session-") {
            return false;
        }

        self.session_name.starts_with("Subagent: ")
    }

    pub fn should_hide_from_user_lists(&self) -> bool {
        self.is_internal_hidden() || self.is_legacy_leaked_subagent_candidate()
    }
}
