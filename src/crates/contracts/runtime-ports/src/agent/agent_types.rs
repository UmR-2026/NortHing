//! Compression contract, attachment, submission result, cancellation,
//! remote control, runtime event, dynamic tool, config read, session
//! transcript, delegation policy, and subagent context types.

use serde;
use serde::{Deserialize, Serialize};

use crate::port_core::PortResult;

// ── Compression contract ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionContract {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub touched_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verification_commands: Vec<CompressionContractItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocking_failures: Vec<CompressionContractItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subagent_statuses: Vec<CompressionContractItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionContractItem {
    pub target: String,
    pub status: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_kind: Option<String>,
}

impl CompressionContract {
    pub fn is_empty(&self) -> bool {
        self.touched_files.is_empty()
            && self.verification_commands.is_empty()
            && self.blocking_failures.is_empty()
            && self.subagent_statuses.is_empty()
    }

    pub fn render_for_model(&self) -> String {
        let mut lines =
            vec!["Compaction contract: preserve these factual fields when continuing the task.".to_string()];

        if !self.touched_files.is_empty() {
            lines.push("Touched files:".to_string());
            for file in &self.touched_files {
                lines.push(format!("- {}", file));
            }
        }

        render_contract_items(&mut lines, "Verification commands:", &self.verification_commands);
        render_contract_items(&mut lines, "Blocking failures:", &self.blocking_failures);
        render_contract_items(&mut lines, "Subagent statuses:", &self.subagent_statuses);

        lines.join("\n")
    }
}

fn render_contract_items(lines: &mut Vec<String>, title: &str, items: &[CompressionContractItem]) {
    if items.is_empty() {
        return;
    }

    lines.push(title.to_string());
    for item in items {
        let mut rendered = format!("- {} [{}]: {}", item.target, item.status, item.summary);
        if let Some(error_kind) = item.error_kind.as_ref() {
            rendered.push_str(&format!(" ({})", error_kind));
        }
        lines.push(rendered);
    }
}

// ── Related path / attachment ────────────────────────────────────────────────

/// User-managed related directory reference for request-context prompts.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RelatedPath {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInputAttachment {
    pub kind: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl AgentInputAttachment {
    pub fn remote_image(id: impl Into<String>, name: impl Into<String>, data_url: impl Into<String>) -> Self {
        let mut metadata = serde_json::Map::new();
        metadata.insert("name".to_string(), serde_json::Value::String(name.into()));
        metadata.insert("dataUrl".to_string(), serde_json::Value::String(data_url.into()));

        Self {
            kind: "remote_image".to_string(),
            id: id.into(),
            metadata,
        }
    }
}

// ── Submission result / port ─────────────────────────────────────────────────

use super::{AgentSessionCreateRequest, AgentSessionCreateResult, AgentSubmissionRequest};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSubmissionResult {
    pub turn_id: String,
    #[serde(default)]
    pub accepted: bool,
}

#[async_trait::async_trait]
pub trait AgentSubmissionPort: Send + Sync {
    async fn create_session(&self, request: AgentSessionCreateRequest) -> PortResult<AgentSessionCreateResult>;

    async fn submit_message(&self, request: AgentSubmissionRequest) -> PortResult<AgentSubmissionResult>;

    async fn resolve_session_agent_type(&self, session_id: &str) -> PortResult<Option<String>>;
}

// ── Session management port ──────────────────────────────────────────────────

use super::{AgentSessionDeleteRequest, AgentSessionListRequest, AgentSessionSummary, AgentSessionWorkspaceRequest};

#[async_trait::async_trait]
pub trait AgentSessionManagementPort: Send + Sync {
    async fn list_sessions(&self, request: AgentSessionListRequest) -> PortResult<Vec<AgentSessionSummary>>;

    async fn delete_session(&self, request: AgentSessionDeleteRequest) -> PortResult<()>;

    async fn resolve_session_workspace_path(&self, request: AgentSessionWorkspaceRequest)
        -> PortResult<Option<String>>;
}

// ── Lifecycle delivery port ──────────────────────────────────────────────────

use super::agent_thread_goal::AgentThreadGoalDeliveryRequest;
use super::AgentBackgroundResultRequest;

#[async_trait::async_trait]
pub trait AgentLifecycleDeliveryPort: Send + Sync {
    async fn deliver_background_result(&self, request: AgentBackgroundResultRequest) -> PortResult<()>;

    async fn deliver_thread_goal(&self, request: AgentThreadGoalDeliveryRequest) -> PortResult<()>;
}

// ── Turn cancellation ────────────────────────────────────────────────────────

use super::agent_dialog::AgentSubmissionSource;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTurnCancellationRequest {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<AgentSubmissionSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requester_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTurnCancellationResult {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub requested: bool,
}

#[async_trait::async_trait]
pub trait AgentTurnCancellationPort: Send + Sync {
    async fn cancel_turn(&self, request: AgentTurnCancellationRequest) -> PortResult<AgentTurnCancellationResult>;
}

// ── Remote control ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteControlSessionState {
    Idle,
    Processing,
    Error,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteControlStateRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteControlStateSnapshot {
    pub session_id: String,
    pub state: RemoteControlSessionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_turn_id: Option<String>,
    #[serde(default)]
    pub queue_depth: usize,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

#[async_trait::async_trait]
pub trait RemoteControlStatePort: Send + Sync {
    async fn read_remote_control_state(
        &self,
        request: RemoteControlStateRequest,
    ) -> PortResult<Option<RemoteControlStateSnapshot>>;
}

// ── Runtime event ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEventType {
    TurnStarted,
    TurnCompleted,
    TurnFailed,
    TurnCancelled,
    SessionStateChanged,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEventEnvelope {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<AgentSubmissionSource>,
    pub event_type: RuntimeEventType,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[async_trait::async_trait]
pub trait RuntimeEventSink: Send + Sync {
    async fn publish_runtime_event(&self, event: RuntimeEventEnvelope) -> PortResult<()>;
}

// ── Dynamic tool ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
}

#[async_trait::async_trait]
pub trait DynamicToolProvider: Send + Sync {
    async fn list_dynamic_tools(&self) -> PortResult<Vec<DynamicToolDescriptor>>;
}

pub trait ToolDecorator<Tool>: Send + Sync {
    fn decorate(&self, tool: Tool) -> Tool;
}

// ── Config read ──────────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait ConfigReadPort: Send + Sync {
    async fn get_config_value(&self, key: &str) -> PortResult<Option<serde_json::Value>>;
}

// ── Session transcript ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTranscriptRequest {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTranscript {
    pub session_id: String,
    #[serde(default)]
    pub messages: Vec<TranscriptMessage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub content: serde_json::Value,
}

#[async_trait::async_trait]
pub trait SessionTranscriptReader: Send + Sync {
    async fn read_session_transcript(&self, request: SessionTranscriptRequest) -> PortResult<SessionTranscript>;
}

// ── Delegation policy ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DelegationPolicy {
    pub allow_subagent_spawn: bool,
    pub nesting_depth: u8,
}

impl Default for DelegationPolicy {
    fn default() -> Self {
        Self::top_level()
    }
}

impl DelegationPolicy {
    pub fn top_level() -> Self {
        Self {
            allow_subagent_spawn: true,
            nesting_depth: 0,
        }
    }

    pub fn spawn_child(self) -> Self {
        Self {
            allow_subagent_spawn: false,
            nesting_depth: self.nesting_depth.saturating_add(1),
        }
    }
}

// ── Subagent context mode ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentContextMode {
    #[default]
    Fresh,
    Fork,
}

impl SubagentContextMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Fork => "fork",
        }
    }
}
