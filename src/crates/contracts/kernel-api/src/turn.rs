//! Kernel turn API and turn DTOs.

use crate::error::{KernelError, KernelResult};
use crate::session::{SessionId, MessageDto, MessageRoleDto, MessageContentDto};

pub type TurnId = String;

// ── Turn DTOs ─────────────────────────────────────────────────────────────────

/// Turn input DTO for submitting a dialog turn.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TurnInputDto {
    pub session_id: SessionId,
    pub text: String,
    pub mode: String,
    pub policy: SubmissionPolicyDto,
    pub source: TriggerSourceDto,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubmissionPolicyDto {
    pub allow_subagent: bool,
    pub max_turns: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSourceDto {
    User,
    Subagent,
    Scheduled,
    System,
}

/// Dialog submit outcome DTO (enumerated from core at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DialogSubmitOutcomeDto {
    pub turn_id: TurnId,
    pub accepted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// FROZEN — TurnStateKind (C2 new).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnStateKind {
    Started,
    Completed,
    Failed,
    Cancelled,
}

/// Turn state DTO (F1.5 new: includes duration_ms).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TurnStateDto {
    pub state: TurnStateKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

// ── KernelTurnApi ─────────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait KernelTurnApi: Send + Sync {
    /// Submit a turn (dialog message).
    /// Source: #3 #5 #8 #9 #10
    async fn submit_turn(&self, input: TurnInputDto) -> Result<DialogSubmitOutcomeDto, KernelError>;

    /// Stop the currently executing turn.
    /// Source: turn management implicit (cancel path)
    async fn stop_turn(&self, turn_id: &TurnId) -> Result<(), KernelError>;

    /// Query turn state (F1.5 new: includes duration_ms).
    /// Source: F1.5 TurnState.duration_ms
    async fn get_turn_state(&self, turn_id: &TurnId) -> Result<TurnStateDto, KernelError>;
}
