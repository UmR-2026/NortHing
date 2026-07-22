//! KernelUsageApi implementation.

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::usage::{TokenUsageDto, TurnUsageDto, UsageReportDto, UsageRequestDto};

use super::{SessionId, TurnId};

#[async_trait]
impl northhing_kernel_api::KernelUsageApi for super::KernelFacade {
    async fn generate_session_usage(
        &self,
        _request: UsageRequestDto,
    ) -> Result<UsageReportDto, KernelError> {
        Err(KernelError::Internal("not yet wired: generate_session_usage".to_string()))
    }

    async fn render_usage_markdown(&self, report: &UsageReportDto) -> String {
        // NOTE: Cannot forward to `render_usage_report_markdown` because UsageReportDto
        // is not type-isomorphic with SessionUsageReport — requires a DTO→SessionUsageReport
        // adapter (P2 trait extension territory). Hand-written format retained as fallback.
        format!(
            "## Usage Report\n\nSession: {}\n\nTotal tokens: {}\nPrompt tokens: {}\nCompletion tokens: {}\nTurn count: {}\nTool call count: {}",
            report.session_id,
            report.total_tokens,
            report.prompt_tokens,
            report.completion_tokens,
            report.turn_count,
            report.tool_call_count
        )
    }

    async fn get_token_usage(&self, _session_id: &SessionId) -> Result<TokenUsageDto, KernelError> {
        // NEEDS_CONTEXT: requires TokenUsageService access and PersistenceManager.
        Err(KernelError::Internal("not yet wired: get_token_usage".to_string()))
    }
}
