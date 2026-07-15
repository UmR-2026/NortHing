//! Round 8 split sibling: loop_detection
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
    pub(super) fn should_continue_after_partial_response(reason: &str) -> bool {
        let lower = reason.to_ascii_lowercase();
        !lower.contains("cancelled")
    }

    pub(super) fn is_periodic_tool_signature_loop(recent_signatures: &[String], threshold: usize) -> bool {
        let threshold = threshold.max(1);
        let window_size = threshold.saturating_mul(2);
        if window_size == 0 || recent_signatures.len() < window_size {
            return false;
        }

        let tail = &recent_signatures[recent_signatures.len() - window_size..];
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for sig in tail {
            *counts.entry(sig.as_str()).or_insert(0) += 1;
        }

        if counts.len() > threshold {
            return false;
        }

        counts.values().all(|&count| count >= 2)
    }

    pub(super) fn failed_tool_round_signature(
        tool_calls: &[crate::agentic::core::ToolCall],
        tool_result_messages: &[Message],
    ) -> Option<String> {
        if tool_result_messages.is_empty()
            || !tool_result_messages.iter().all(|message| {
                let MessageContent::ToolResult { result, is_error, .. } = &message.content else {
                    return false;
                };
                ContextHealthSnapshot::tool_result_failed(result, *is_error)
            })
        {
            return None;
        }

        Self::tool_call_signature(tool_calls)
    }

    pub(super) fn tool_call_signature(tool_calls: &[crate::agentic::core::ToolCall]) -> Option<String> {
        if tool_calls.is_empty() {
            return None;
        }

        let mut signatures: Vec<String> = tool_calls
            .iter()
            .map(|tool_call| {
                let arguments = tool_call.arguments.to_string();
                let arguments_summary = Self::tool_signature_args_summary(&arguments);
                format!("{}:{}", tool_call.tool_name, arguments_summary)
            })
            .collect();
        signatures.sort();
        Some(signatures.join("|"))
    }

    pub(super) fn tool_signature_args_summary(args_str: &str) -> String {
        if args_str.len() <= 128 {
            return args_str.to_string();
        }

        let args_hash = hex::encode(Sha256::digest(args_str.as_bytes()));
        format!(
            "{}..#{}:sha256={}",
            truncate_at_char_boundary(args_str, 64),
            args_str.len(),
            args_hash
        )
    }
}
