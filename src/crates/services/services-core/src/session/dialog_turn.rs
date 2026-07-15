//! Persisted dialog-turn DTOs: [`DialogTurnData`] (with its constructors /
//! mutators), [`DialogTurnTokenUsageData`], [`DialogTurnKind`] (with
//! `is_model_visible`), and [`TurnStatus`].
//!
//! DialogTurnData references content types that live in the
//! [`super::model_round`] sibling ([`UserMessageData`], [`ModelRoundData`]);
//! those are imported through `super::model_round::*` so the god-split keeps
//! the original `crate::session::types::*` re-export surface intact.

use serde::{Deserialize, Serialize};

use super::model_round::{ModelRoundData, UserMessageData};

/// Full dialog turn data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogTurnData {
    /// Turn ID
    #[serde(alias = "turn_id")]
    pub turn_id: String,

    /// Turn index (starting from 0)
    #[serde(alias = "turn_index")]
    pub turn_index: usize,

    /// Session ID
    #[serde(alias = "session_id")]
    pub session_id: String,

    /// Timestamp
    pub timestamp: u64,

    /// Turn kind
    #[serde(default, alias = "turn_kind")]
    pub kind: DialogTurnKind,

    /// Agent type used for this turn when it represents a user dialog.
    /// Maintenance/local utility turns leave this empty so they do not affect
    /// mode-transition reminder semantics.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "agent_type")]
    pub agent_type: Option<String>,

    /// User message
    #[serde(alias = "user_message")]
    pub user_message: UserMessageData,

    /// Model interaction rounds
    #[serde(alias = "model_rounds")]
    pub model_rounds: Vec<ModelRoundData>,

    /// Turn start time
    #[serde(alias = "start_time")]
    pub start_time: u64,

    /// Turn end time
    #[serde(skip_serializing_if = "Option::is_none", alias = "end_time")]
    pub end_time: Option<u64>,

    /// Turn duration (milliseconds)
    #[serde(skip_serializing_if = "Option::is_none", alias = "duration_ms")]
    pub duration_ms: Option<u64>,

    /// Provider-reported token usage for this dialog turn, when available.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "token_usage")]
    pub token_usage: Option<DialogTurnTokenUsageData>,

    /// Turn status
    pub status: TurnStatus,
}

/// Provider-reported token usage attached to a dialog turn.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DialogTurnTokenUsageData {
    /// Input/prompt tokens for the model request.
    #[serde(alias = "input_tokens")]
    pub input_tokens: u64,

    /// Output/completion tokens, when the provider reports them.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "output_tokens")]
    pub output_tokens: Option<u64>,

    /// Total tokens reported by the provider for this request.
    #[serde(alias = "total_tokens")]
    pub total_tokens: u64,

    /// Frontend event timestamp in milliseconds since epoch.
    pub timestamp: u64,
}

/// Persisted dialog turn kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum DialogTurnKind {
    #[default]
    UserDialog,
    ManualCompaction,
    LocalCommand,
}

impl DialogTurnKind {
    pub fn is_model_visible(self) -> bool {
        matches!(self, Self::UserDialog)
    }
}

/// Turn status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TurnStatus {
    InProgress,
    Completed,
    Error,
    Cancelled,
}

impl DialogTurnData {
    /// Creates a new dialog turn.
    pub fn new(turn_id: String, turn_index: usize, session_id: String, user_message: UserMessageData) -> Self {
        Self::new_with_kind(
            DialogTurnKind::UserDialog,
            turn_id,
            turn_index,
            session_id,
            None,
            user_message,
        )
    }

    /// Creates a new dialog turn with an explicit persisted kind.
    pub fn new_with_kind(
        kind: DialogTurnKind,
        turn_id: String,
        turn_index: usize,
        session_id: String,
        agent_type: Option<String>,
        user_message: UserMessageData,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            turn_id,
            turn_index,
            session_id,
            timestamp: now,
            kind,
            agent_type,
            user_message,
            model_rounds: Vec::new(),
            start_time: now,
            end_time: None,
            duration_ms: None,
            token_usage: None,
            status: TurnStatus::InProgress,
        }
    }

    /// Marks this turn as completed.
    pub fn mark_completed(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.end_time = Some(now);
        self.duration_ms = Some(now.saturating_sub(self.start_time));
        self.status = TurnStatus::Completed;
    }

    /// Counts total tool calls.
    pub fn count_tool_calls(&self) -> usize {
        self.model_rounds.iter().map(|round| round.tool_items.len()).sum()
    }
}
