//! Kernel events API and event DTOs.

use crate::error::KernelError;
use crate::turn::TurnStateKind;

// ── Event Subscription ID ─────────────────────────────────────────────────────

pub type SubscriptionId = String;

// ── FROZEN ToolCallDto (Schema §5.1) ──────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCallDto {
    pub session_id: String,
    pub turn_id: String,
    pub call_id: String,
    pub phase: ToolCallPhase,
    pub name: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// FROZEN ToolCallPhase (Schema §5.1).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallPhase {
    Started,
    Completed,
}

// ── Banner Level ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BannerLevel {
    Info,
    Warning,
    Error,
}

// ── KernelEventDto ─────────────────────────────────────────────────────────────

/// FROZEN KernelEventDto enum (Schema §5).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum KernelEventDto {
    TextChunk { session_id: String, text: String },
    TurnState {
        session_id: String,
        turn_id: String,
        state: TurnStateKind,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
    },
    ToolCall(ToolCallDto),
    Banner { level: BannerLevel, message: String },
    Error { message: String },
}

/// Backend event DTO for host→kernel broadcast (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackendEventDto {
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

// ── KernelEventsApi ────────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait KernelEventsApi: Send + Sync {
    /// Subscribe to kernel events (TextChunk/TurnState/ToolCall/Banner/Error).
    /// Source: #20 #21 #22
    async fn subscribe_events(&self, callback: Box<dyn Fn(KernelEventDto) + Send>) -> SubscriptionId;

    /// Unsubscribe from events.
    /// Source: #20
    async fn unsubscribe_events(&self, id: SubscriptionId) -> Result<(), KernelError>;

    /// Broadcast an event to the backend (host→kernel).
    /// Source: #83 #84
    async fn emit_backend_event(&self, event: BackendEventDto) -> Result<(), KernelError>;
}
