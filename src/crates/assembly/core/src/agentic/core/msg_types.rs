use crate::agentic::image_analysis::ImageContextData;
use crate::util::types::ToolImageAttachment;
use serde::{Deserialize, Serialize};

// ============ Message ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: MessageContent,
    pub timestamp: std::time::SystemTime,
    pub metadata: MessageMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Text(String),
    Multimodal {
        text: String,
        images: Vec<ImageContextData>,
    },
    ToolResult {
        tool_id: String,
        tool_name: String,
        result: serde_json::Value,
        result_for_assistant: Option<String>,
        is_error: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        image_attachments: Option<Vec<ToolImageAttachment>>,
    },
    Mixed {
        /// Reasoning content (for interleaved thinking mode)
        reasoning_content: Option<String>,
        text: String,
        tool_calls: Vec<ToolCall>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub turn_id: Option<String>,
    pub round_id: Option<String>,
    pub tokens: Option<usize>,
    /// Anthropic extended thinking signature (for passing back in multi-turn conversations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_kind: Option<MessageSemanticKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_reminder_kind: Option<InternalReminderKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression_payload: Option<CompressionPayload>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageSemanticKind {
    ActualUserInput,
    InternalReminder,
    CompressionBoundaryMarker,
    CompressionSummary,
    /// Shown in chat after Computer use; omitted from model API requests (see `build_ai_messages_for_send`).
    ComputerUseVerificationScreenshot,
    /// Full-screen snapshot appended after mutating ComputerUse tool results within the same turn;
    /// **included** in the next model request so the agent sees the desktop without calling screenshot again.
    ComputerUsePostActionSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalReminderKind {
    Generic,
    SkillListingDiff,
    AgentListingDiff,
    AgentMode,
    SideQuestion,
    InitAgentsMd,
    ScheduledJob,
    ForkSubagent,
    GoalMode,
    GoalContinuation,
    GoalObjectiveUpdated,
    RemoteFileDelivery,
    SessionMessageRequest,
    SessionMessageReply,
    LoopRecovery,
    PeriodicLoopRecovery,
    UserSteering,
    BackgroundResult,
    InterruptedContinue,
    ThinkingOnlyRescue,
}

impl InternalReminderKind {
    pub fn should_drop_during_compaction(self) -> bool {
        matches!(
            self,
            Self::SkillListingDiff
                | Self::AgentListingDiff
                | Self::LoopRecovery
                | Self::PeriodicLoopRecovery
                | Self::InterruptedContinue
                | Self::ThinkingOnlyRescue
        )
    }

    pub fn is_listing_diff(self) -> bool {
        matches!(self, Self::SkillListingDiff | Self::AgentListingDiff)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompressionPayload {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<CompressionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CompressionEntry {
    Contract {
        contract: northhing_runtime_ports::CompressionContract,
    },
    ModelSummary {
        text: String,
    },
    Turn {
        #[serde(skip_serializing_if = "Option::is_none")]
        turn_id: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        messages: Vec<CompressedMessage>,
        #[serde(skip_serializing_if = "Option::is_none")]
        todo: Option<CompressedTodoSnapshot>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedMessage {
    pub role: CompressedMessageRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<CompressedToolCall>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressedMessageRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedToolCall {
    pub tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedTodoSnapshot {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub todos: Vec<CompressedTodoItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedTodoItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub content: String,
    pub status: String,
}

impl CompressionPayload {
    pub fn from_summary(text: String) -> Self {
        Self {
            entries: vec![CompressionEntry::ModelSummary { text }],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

// ============ Tool Calls and Results ============

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    /// Original provider-emitted argument JSON, preserved for replay stability when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_arguments: Option<String>,
    /// Record whether tool parameters are valid
    #[serde(default)]
    pub is_error: bool,
    /// True when the raw JSON arguments were truncated mid-stream and we
    /// successfully repaired them. Downstream consumers can flag this to the
    /// model so it understands the content may be incomplete.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub recovered_from_truncation: bool,
}

impl ToolCall {
    pub fn is_valid(&self) -> bool {
        !self.tool_id.is_empty() && !self.tool_name.is_empty() && !self.is_error
    }
}

impl From<northhing_agent_stream::ToolCall> for ToolCall {
    fn from(tool_call: northhing_agent_stream::ToolCall) -> Self {
        Self {
            tool_id: tool_call.tool_id,
            tool_name: tool_call.tool_name,
            arguments: tool_call.arguments,
            raw_arguments: tool_call.raw_arguments,
            is_error: tool_call.is_error,
            recovered_from_truncation: tool_call.recovered_from_truncation,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_id: String,
    pub tool_name: String,
    pub result: serde_json::Value,
    /// Result text specifically for passing to AI assistant (if None, then use result)
    pub result_for_assistant: Option<String>,
    pub is_error: bool,
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_attachments: Option<Vec<ToolImageAttachment>>,
}
