//! Kernel usage API and usage DTOs.

use crate::error::KernelError;
use crate::session::SessionId;

// ── Usage DTOs ────────────────────────────────────────────────────────────────

/// Usage request DTO (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UsageRequestDto {
    pub session_id: SessionId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_turn: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_turn: Option<usize>,
}

/// Usage report DTO (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UsageReportDto {
    pub session_id: SessionId,
    pub total_tokens: usize,
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub turn_count: usize,
    pub tool_call_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_estimate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_turn: Option<Vec<TurnUsageDto>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TurnUsageDto {
    pub turn_index: usize,
    pub tokens: usize,
    pub tool_calls: usize,
}

/// Token usage DTO (enumerated at implementation time).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenUsageDto {
    pub session_id: SessionId,
    pub total_tokens: usize,
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<usize>,
}

// ── KernelUsageApi ─────────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait KernelUsageApi: Send + Sync {
    /// Generate session usage report.
    /// Source: #68 #70 #71
    async fn generate_session_usage(&self, request: UsageRequestDto) -> Result<UsageReportDto, KernelError>;

    /// Render usage report as markdown.
    /// Source: #69
    async fn render_usage_markdown(&self, report: &UsageReportDto) -> String;

    /// Get token usage.
    /// Source: #72
    async fn get_token_usage(&self, session_id: &SessionId) -> Result<TokenUsageDto, KernelError>;
}
