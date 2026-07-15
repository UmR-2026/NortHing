//! Round 8 split sibling: multimodal
//!
//! Auto-extracted from execution_engine.rs as part of the Round 8 sub-domain split.
//! Methods are declared `pub(super)` so the facade (`execution_engine.rs`) can call them.

use super::compression::CompressionRuntimeScaffold;
use super::execution_engine::ContextCompactionOutcome;
use super::execution_engine::ExecutionEngine;
use super::health_snapshot::ContextHealthSnapshot;

use super::model_exchange_trace::{prepare_model_exchange_trace_for_workspace, ModelExchangeTraceOperation};
use super::round_executor::RoundExecutor;
use super::types::{ExecutionContext, ExecutionResult, ExecutionTurnState, RoundContext, RoundResult, RoundTickResult};
use crate::agentic::agents::{
    agent_registry, build_prompt_context_for_workspace, PartitionedLoader, PrependedPromptReminders, PromptBuilder,
    PromptBuilderContext, RuntimeContextNeeds, ToolListingSections, USE_PARTITIONED_LOADER,
};
use crate::agentic::context_profile::{ContextProfilePolicy, ModelCapabilityProfile};
use crate::agentic::core::{
    render_system_reminder, InternalReminderKind, Message, MessageContent, MessageHelper, MessageRole,
    MessageSemanticKind, RequestReasoningTokenPolicy, Session,
};
use crate::agentic::events::{AgenticEvent, EventPriority, EventQueue};
use crate::agentic::execution::types::FinishReason;
use crate::agentic::image_analysis::{
    build_multimodal_message_with_images, process_image_contexts_for_provider, ImageContextData, ImageLimits,
};
use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
use crate::agentic::round_preempt::RoundInjectionKind;
use crate::agentic::session::{CompressionMode, ContextCompressor, SessionManager};
use crate::agentic::skill_agent_snapshot::build_skill_agent_tool_listing_sections_from_snapshot;
use crate::agentic::tools::implementations::{SkillTool, TaskTool};
use crate::agentic::tools::product_runtime::{collect_product_unlocked_collapsed_tools, GetToolSpecTool};
use crate::agentic::tools::{resolve_tool_manifest, tool_context_runtime, ResolvedToolManifest};
use crate::agentic::WorkspaceBinding;
use crate::infrastructure::ai::get_global_ai_client_factory;
use crate::service::config::get_global_config_service;
use crate::service::config::types::{ModelCapability, ModelCategory};
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::token_counter::TokenCounter;
use crate::util::types::Message as AIMessage;
use crate::util::types::ToolDefinition;
use crate::util::{elapsed_ms_u64, truncate_at_char_boundary};
use northhing_ai_adapters::ModelExchangeTraceConfig;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

impl ExecutionEngine {
    pub(super) fn is_redacted_image_context(image: &ImageContextData) -> bool {
        let missing_path = image.image_path.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true);
        let missing_data_url = image.data_url.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true);
        let has_redaction_hint = image
            .metadata
            .as_ref()
            .and_then(|m| m.get("has_data_url"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        missing_path && missing_data_url && has_redaction_hint
    }

    pub(super) fn is_recoverable_historical_image_error(err: &NortHingError) -> bool {
        match err {
            NortHingError::Io(_) | NortHingError::Deserialization(_) => true,
            NortHingError::Validation(msg) => {
                msg.starts_with("Failed to decode image data")
                    || msg.starts_with("Unsupported or unrecognized image format")
                    || msg.starts_with("Invalid data URL format")
                    || msg.starts_with("Data URL format error")
            }
            _ => false,
        }
    }

    pub(super) fn can_fallback_to_text_only(
        images: &[ImageContextData],
        err: &NortHingError,
        is_current_turn_message: bool,
    ) -> bool {
        let is_redacted_payload_error = matches!(
            err,
            NortHingError::Validation(msg) if msg.starts_with("Image context missing image_path/data_url")
        ) && !images.is_empty()
            && images.iter().all(Self::is_redacted_image_context);

        if is_redacted_payload_error {
            return true;
        }

        if is_current_turn_message {
            return false;
        }

        Self::is_recoverable_historical_image_error(err)
    }

    pub(super) fn skip_message_for_model_send(msg: &Message) -> bool {
        matches!(
            msg.metadata.semantic_kind.as_ref(),
            Some(MessageSemanticKind::ComputerUseVerificationScreenshot)
                | Some(MessageSemanticKind::ComputerUsePostActionSnapshot)
        )
    }

    pub(super) fn message_bears_images(msg: &Message) -> bool {
        if Self::skip_message_for_model_send(msg) {
            return false;
        }
        match &msg.content {
            MessageContent::Multimodal { images, .. } => !images.is_empty(),
            MessageContent::ToolResult { image_attachments, .. } => {
                image_attachments.as_ref().is_some_and(|a| !a.is_empty())
            }
            _ => false,
        }
    }

    pub(super) fn image_bearing_indices_to_keep(messages: &[Message], max_image_messages: usize) -> HashSet<usize> {
        let with_images: Vec<usize> = messages
            .iter()
            .enumerate()
            .filter(|(_, m)| Self::message_bears_images(m))
            .map(|(i, _)| i)
            .collect();
        let n = with_images.len();
        if n <= max_image_messages {
            return with_images.into_iter().collect();
        }
        with_images[n - max_image_messages..].iter().copied().collect()
    }

    pub(super) fn render_multimodal_as_text(text: &str, images: &[ImageContextData]) -> String {
        let mut content = text.to_string();

        if images.is_empty() {
            return content;
        }

        content.push_str("\n\n[Attached image(s):\n");
        for image in images {
            let name = image
                .metadata
                .as_ref()
                .and_then(|m| m.get("name"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .or_else(|| image.image_path.as_ref().filter(|s| !s.is_empty()).cloned())
                .unwrap_or_else(|| image.id.clone());

            content.push_str(&format!("- {} ({}, image_id={})\n", name, image.mime_type, image.id));
        }
        content.push_str("]\n");

        content.push_str("Note: image inspection is not available for this session.\n");

        content
    }

    pub(super) fn assistant_has_tool_calls(message: &Message) -> bool {
        matches!(
            &message.content,
            MessageContent::Mixed { tool_calls, .. } if !tool_calls.is_empty()
        )
    }

    pub(super) fn has_tool_result_after_last_assistant(messages: &[Message]) -> bool {
        let Some(last_assistant_index) = messages
            .iter()
            .rposition(|message| message.role == MessageRole::Assistant)
        else {
            return false;
        };

        messages[last_assistant_index + 1..]
            .iter()
            .any(|message| matches!(message.content, MessageContent::ToolResult { .. }))
    }
}
