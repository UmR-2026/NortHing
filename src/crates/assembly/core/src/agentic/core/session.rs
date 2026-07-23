use super::state::SessionState;
pub use northhing_core_types::SessionKind;
pub use northhing_services_core::session::SessionStatus;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

// ============ Session ============

/// Session: contains multiple dialog turns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub session_name: String,
    /// Current/default mode selection for the session.
    ///
    /// This is the mode the next dialog turn should run with by default. It is
    /// not required to match either the last surviving history turn or the last
    /// message submission accepted by the scheduler.
    pub agent_type: String,
    /// Cached mode of the last surviving user dialog turn in history.
    ///
    /// Reminder builders use this value for `previous_agent_type` so
    /// first-entry vs ongoing mode prompts follow the surviving transcript
    /// after rollbacks or turn truncation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_user_dialog_agent_type: Option<String>,
    /// Mode of the most recent user submission accepted by the scheduler.
    ///
    /// Unlike `last_user_dialog_agent_type`, this value is not rewound by
    /// history rollback. It tracks session-level prompt-cache compatibility for
    /// the next accepted submission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_submitted_agent_type: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "created_by",
        alias = "createdBy"
    )]
    pub created_by: Option<String>,
    #[serde(default, alias = "session_kind", alias = "sessionKind")]
    pub kind: SessionKind,

    /// Associated resources
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "sandbox_session_id",
        alias = "sandboxSessionId"
    )]
    pub snapshot_session_id: Option<String>,

    /// Dialog turn ID list
    pub dialog_turn_ids: Vec<String>,

    /// Session state
    pub state: SessionState,

    /// Configuration
    pub config: SessionConfig,

    /// Context compression related
    pub compression_state: CompressionState,

    /// Parent-session relationship (Phase D.2).
    ///
    /// Lightweight projection of the persistence-layer
    /// `SessionRelationship` (in `services_core::session::types`) — the
    /// in-memory `Session` only carries the parent session id, not the
    /// full set of relationship fields (request id, tool call id, etc.)
    /// because those fields are only meaningful for persisted sessions
    /// being read back from disk.
    ///
    /// `None` is the correct value for newly created sessions and for any
    /// in-memory session whose relationship hasn't been loaded from disk.
    /// Callers that need the full relationship should re-read the
    /// persisted `SessionMetadata` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationship: Option<InMemoryRelationship>,

    /// Lifecycle
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub last_activity_at: SystemTime,
}

/// Lightweight in-memory projection of the persistence-layer
/// `SessionRelationship`. See `Session::relationship` for the rationale.
///
/// Phase I.4 (2026-06-20): extended with `parent_request_id` and
/// `parent_tool_call_id` to match the persistence-layer field names
/// (`parent_request_id`, `parent_tool_call_id`, etc.).
///
/// Phase I.4-ext (2026-06-20): added `parent_dialog_turn_id` and
/// `parent_turn_index` so the in-memory surface carries the full
/// set of fields the desktop sidebar tree renderer may need.
/// Old serialized data without the new fields loads cleanly
/// thanks to `#[serde(default)]` on every Option.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InMemoryRelationship {
    /// Parent session ID. `None` if this session is a root.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "parent_session_id")]
    pub parent_session_id: Option<String>,
    /// Originating user request id (matches
    /// `SessionRelationship::parent_request_id`). `None` for sessions
    /// not spawned by a request.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "parent_request_id")]
    pub parent_request_id: Option<String>,
    /// Dialog turn id that originated the subagent dispatch (matches
    /// `SessionRelationship::parent_dialog_turn_id`).
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "parent_dialog_turn_id")]
    pub parent_dialog_turn_id: Option<String>,
    /// Turn index within the parent session that dispatched this
    /// subagent (matches `SessionRelationship::parent_turn_index`).
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "parent_turn_index")]
    pub parent_turn_index: Option<usize>,
    /// Tool call id that originated the subagent dispatch (matches
    /// `SessionRelationship::parent_tool_call_id`). `None` for sessions
    /// not spawned by a tool call.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "parent_tool_call_id")]
    pub parent_tool_call_id: Option<String>,
}

/// Context compression state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompressionState {
    /// Time of last compression
    pub last_compression_at: Option<SystemTime>,
    /// Compression trigger count
    pub compression_count: usize,
}

impl CompressionState {
    pub fn increment_compression_count(&mut self) {
        self.last_compression_at = Some(SystemTime::now());
        self.compression_count += 1;
    }
}

impl Session {
    pub fn new(session_name: String, agent_type: String, config: SessionConfig) -> Self {
        let now = SystemTime::now();
        Self {
            session_id: Uuid::new_v4().to_string(),
            session_name,
            agent_type,
            last_user_dialog_agent_type: None,
            last_submitted_agent_type: None,
            created_by: None,
            kind: SessionKind::Standard,
            snapshot_session_id: None,
            dialog_turn_ids: vec![],
            state: SessionState::Idle,
            config,
            compression_state: CompressionState::default(),
            relationship: None,
            created_at: now,
            updated_at: now,
            last_activity_at: now,
        }
    }

    pub fn new_with_id(session_id: String, session_name: String, agent_type: String, config: SessionConfig) -> Self {
        let now = SystemTime::now();
        Self {
            session_id,
            session_name,
            agent_type,
            last_user_dialog_agent_type: None,
            last_submitted_agent_type: None,
            created_by: None,
            kind: SessionKind::Standard,
            snapshot_session_id: None,
            dialog_turn_ids: vec![],
            state: SessionState::Idle,
            config,
            compression_state: CompressionState::default(),
            relationship: None,
            created_at: now,
            updated_at: now,
            last_activity_at: now,
        }
    }
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub max_context_tokens: usize,
    pub auto_compact: bool,
    pub enable_tools: bool,
    pub safe_mode: bool,
    pub max_turns: usize,
    pub enable_context_compression: bool,
    /// Compression threshold (token usage rate), compression triggered when exceeded
    pub compression_threshold: f32,
    /// Workspace path bound to this session. Used to run AI in the correct workspace
    /// without changing the desktop's foreground workspace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    /// Stable workspace id for resolving workspace-scoped metadata such as related directories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    /// SSH workspace: required for remote tool I/O (file/shell). When set, `workspace_path` is
    /// interpreted as the path on that host; when unset, the workspace is always local regardless
    /// of string shape (avoids inferring remote from path alone). Also disambiguates the same
    /// `workspace_path` on different hosts (e.g. two `/` roots).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_connection_id: Option<String>,
    /// SSH config `host` for locating `~/.northhing/remote_ssh/{host}/.../sessions` when disconnected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_ssh_host: Option<String>,
    /// Model config ID used by this session (for token usage tracking)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 128128,
            auto_compact: true,
            enable_tools: true,
            safe_mode: true,
            max_turns: 200,
            enable_context_compression: true,
            compression_threshold: 0.8, // 80%
            workspace_path: None,
            workspace_id: None,
            remote_connection_id: None,
            remote_ssh_host: None,
            model_id: None,
        }
    }
}

/// Session summary (for list display)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub session_name: String,
    /// Current/default mode selection for the session.
    pub agent_type: String,
    /// Mode of the last surviving user dialog turn in the session history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_user_dialog_agent_type: Option<String>,
    /// Mode of the most recent user submission accepted by the scheduler.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_submitted_agent_type: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "created_by",
        alias = "createdBy"
    )]
    pub created_by: Option<String>,
    #[serde(default, alias = "session_kind", alias = "sessionKind")]
    pub kind: SessionKind,
    pub turn_count: usize,
    pub created_at: SystemTime,
    pub last_activity_at: SystemTime,
    pub state: SessionState,
    /// Lifecycle status projected from persisted metadata.
    /// The persistence path carries the real stored status; the in-memory
    /// path defaults to `Active` because `SessionState` has no archived notion.
    #[serde(default)]
    pub status: SessionStatus,
    /// Parent session ID for subagent sessions. `None` for root sessions.
    ///
    /// Phase C.1 (2026-06-19): added to enable the sidebar tree view. Existing
    /// sessions on disk have no parent recorded; the persistence layer
    /// projects `None` from the metadata's optional `relationship.parent_session_id`,
    /// so legacy data is treated as root by construction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════
// Phase I.4/I.5 tests (2026-06-20)
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod phase_i_tests {
    use super::*;

    /// `InMemoryRelationship` round-trips through JSON with all fields
    /// populated. This pins the `serde` rename aliases (parent_session_id,
    /// parent_request_id, parent_tool_call_id, parent_dialog_turn_id,
    /// parent_turn_index) so a future rename breaks loudly instead of
    /// silently dropping data on disk-load.
    #[test]
    fn in_memory_relationship_round_trips_full() {
        let rel = InMemoryRelationship {
            parent_session_id: Some("parent-1".into()),
            parent_request_id: Some("req-42".into()),
            parent_dialog_turn_id: Some("turn-7".into()),
            parent_turn_index: Some(3),
            parent_tool_call_id: Some("tool-call-7".into()),
        };
        let json = serde_json::to_value(&rel).expect("serialize");
        assert_eq!(json["parent_session_id"], "parent-1");
        assert_eq!(json["parent_request_id"], "req-42");
        assert_eq!(json["parent_dialog_turn_id"], "turn-7");
        assert_eq!(json["parent_turn_index"], 3);
        assert_eq!(json["parent_tool_call_id"], "tool-call-7");

        // And back.
        let s = serde_json::to_string(&rel).unwrap();
        let parsed: InMemoryRelationship = serde_json::from_str(&s).expect("parse");
        assert_eq!(parsed.parent_session_id, Some("parent-1".into()));
        assert_eq!(parsed.parent_request_id, Some("req-42".into()));
        assert_eq!(parsed.parent_dialog_turn_id, Some("turn-7".into()));
        assert_eq!(parsed.parent_turn_index, Some(3));
        assert_eq!(parsed.parent_tool_call_id, Some("tool-call-7".into()));
    }

    /// Back-compat: old serialized data without the I.4 fields still
    /// loads cleanly. `#[serde(default)]` on each Option gives us this
    /// for free; the test pins the behavior so a future "tighten the
    /// serde schema" PR doesn't silently break loading legacy data.
    #[test]
    fn in_memory_relationship_legacy_compat() {
        let legacy = r#"{"parent_session_id":"old"}"#;
        let parsed: InMemoryRelationship = serde_json::from_str(legacy).expect("legacy parse");
        assert_eq!(parsed.parent_session_id, Some("old".into()));
        assert_eq!(parsed.parent_request_id, None);
        assert_eq!(parsed.parent_dialog_turn_id, None);
        assert_eq!(parsed.parent_turn_index, None);
        assert_eq!(parsed.parent_tool_call_id, None);
    }

    /// Empty relationship deserializes to all-None.
    #[test]
    fn in_memory_relationship_empty() {
        let parsed: InMemoryRelationship = serde_json::from_str("{}").expect("empty parse");
        assert!(parsed.parent_session_id.is_none());
        assert!(parsed.parent_request_id.is_none());
        assert!(parsed.parent_dialog_turn_id.is_none());
        assert!(parsed.parent_turn_index.is_none());
        assert!(parsed.parent_tool_call_id.is_none());
    }
}
