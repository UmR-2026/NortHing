//! Round 48b split sibling: model-based compression summary generation
//!
//! Moved from execution/compression.rs.
//! Methods are `pub(super)` so sibling modules can call them via `self`.

use super::execution_engine::ExecutionEngine;
use crate::agentic::agents::PrependedPromptReminders;
use crate::agentic::session::ContextCompressor;
use crate::agentic::WorkspaceBinding;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::types::ToolDefinition;
use northhing_ai_adapters::ModelExchangeTraceConfig;
use std::sync::Arc;

impl ExecutionEngine {
    pub(super) async fn generate_compression_model_summary(
        &self,
        ai_client: Arc<crate::infrastructure::ai::AIClient>,
        runtime_messages: &[crate::agentic::core::Message],
        dialog_turn_id: &str,
        workspace: Option<&WorkspaceBinding>,
        tool_definitions: &Option<Vec<ToolDefinition>>,
        prepended_prompt_reminders: &PrependedPromptReminders,
        primary_supports_image_understanding: bool,
        contract: Option<&crate::agentic::core::CompressionContract>,
        trace_config: Option<ModelExchangeTraceConfig>,
    ) -> NortHingResult<Option<String>> {
        let request_messages = self
            .build_compression_request_messages(
                runtime_messages,
                dialog_turn_id,
                workspace,
                &ai_client.config.format,
                primary_supports_image_understanding,
                prepended_prompt_reminders,
                contract,
            )
            .await?;

        let raw_summary = self
            .request_compression_summary_with_retry(
                ai_client,
                request_messages,
                tool_definitions.clone(),
                trace_config,
                2,
            )
            .await?;
        let summary = ContextCompressor::normalize_model_summary_output(&raw_summary).ok_or_else(|| {
            NortHingError::AIClient(
                "Model-based compression returned <analysis> without a usable <summary>".to_string(),
            )
        })?;
        Ok(Some(summary))
    }
}
