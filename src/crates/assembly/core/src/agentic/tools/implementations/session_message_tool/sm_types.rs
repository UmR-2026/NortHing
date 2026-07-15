//! `SessionMessage` tool — input types and session-id shape validation.
//!
//! Owns the serde-deserializable input struct plus the agent-type enum
//! consumed by both the Tool trait implementation and the resolve/send
//! siblings. Keeping these data definitions in one place lets the other
//! siblings use `super::sm_types::*` without pulling in any executable
//! logic from the Tool impl.

use serde::Deserialize;

/// Allowed target agent types when `session_id` is omitted and the tool
/// is asked to create a new session.
///
/// Renamed/aliased serde tags match the values surfaced through the
/// JSON schema (`agentic` / `Plan` / `Cowork`) plus their PascalCase and
/// SCREAMING forms so external callers can pick either convention.
#[derive(Debug, Clone, Deserialize)]
pub(super) enum SessionMessageAgentType {
    #[serde(rename = "agentic", alias = "Agentic", alias = "AGENTIC")]
    Agentic,
    #[serde(rename = "Plan", alias = "plan", alias = "PLAN")]
    Plan,
    #[serde(rename = "Cowork", alias = "cowork", alias = "COWORK")]
    Cowork,
}

impl SessionMessageAgentType {
    /// Canonical wire form returned by the tool's success payload and used
    /// by the underlying `create_session` runtime call.
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Agentic => "agentic",
            Self::Plan => "Plan",
            Self::Cowork => "Cowork",
        }
    }
}

/// Deserialized tool input. All fields are optional except `message`
/// because the JSON schema declares `additionalProperties: false` and
/// validates the rest at the trait boundary.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct SessionMessageInput {
    pub(super) workspace: Option<String>,
    pub(super) session_id: Option<String>,
    pub(super) session_name: Option<String>,
    pub(super) message: String,
    pub(super) agent_type: Option<SessionMessageAgentType>,
}

/// Static shape validator for the `session_id` string. Used by both
/// `validate_input` (sync, for the existing-session branch) and kept
/// available to `resolve_*` siblings for any future defensive checks.
pub(super) fn validate_session_id(session_id: &str) -> Result<(), String> {
    if session_id.is_empty() {
        return Err("session_id cannot be empty".to_string());
    }
    if session_id == "." || session_id == ".." {
        return Err("session_id cannot be '.' or '..'".to_string());
    }
    if session_id.contains('/') || session_id.contains('\\') {
        return Err("session_id cannot contain path separators".to_string());
    }
    if !session_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err("session_id can only contain ASCII letters, numbers, '-' and '_'".to_string());
    }
    Ok(())
}
