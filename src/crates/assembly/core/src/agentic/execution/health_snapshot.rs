//! Round 8 split sibling: health_snapshot
//!
//! Auto-extracted from execution_engine.rs as part of the Round 8 sub-domain split.
//! Methods are declared `pub(super)` so the facade (`execution_engine.rs`) can call them.

use super::compression::CompressionRuntimeScaffold;
use super::execution_engine::ContextCompactionOutcome;
use super::execution_engine::ExecutionEngine;

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

#[derive(Debug, Clone)]
pub(super) struct ContextHealthSnapshot {
    pub(super) token_usage_ratio: f32,
    pub(super) full_compression_count: usize,
    pub(super) compression_failure_count: u32,
    pub(super) repeated_tool_signature_count: usize,
    pub(super) consecutive_failed_commands: usize,
}

impl ContextHealthSnapshot {
    pub(super) fn from_runtime_observations(
        token_usage_ratio: f32,
        full_compression_count: usize,
        compression_failure_count: u32,
        recent_tool_signatures: &[String],
        messages: &[Message],
    ) -> Self {
        Self {
            token_usage_ratio,
            full_compression_count,
            compression_failure_count,
            repeated_tool_signature_count: Self::repeated_tool_signature_count(recent_tool_signatures),
            consecutive_failed_commands: Self::consecutive_failed_commands(messages),
        }
    }

    pub(super) fn token_usage_ratio(current_tokens: usize, context_window: usize) -> f32 {
        if context_window == 0 {
            return 0.0;
        }
        current_tokens as f32 / context_window as f32
    }

    pub(super) fn log(&self, session_id: &str, turn_id: &str, round_index: usize, stage: &str) {
        debug!(
            "Context health snapshot: session_id={}, turn_id={}, round_index={}, stage={}, token_usage={:.3}, full_compression_count={}, compression_failure_count={}, repeated_tool_signature_count={}, consecutive_failed_commands={}",
            session_id,
            turn_id,
            round_index,
            stage,
            self.token_usage_ratio,
            self.full_compression_count,
            self.compression_failure_count,
            self.repeated_tool_signature_count,
            self.consecutive_failed_commands
        );
    }

    pub(super) fn log_policy_thresholds(
        &self,
        session_id: &str,
        turn_id: &str,
        round_index: usize,
        policy: &ContextProfilePolicy,
    ) {
        if policy.has_repeated_tool_loop(self.repeated_tool_signature_count) {
            debug!(
                "Context profile repeated-tool threshold reached: session_id={}, turn_id={}, round_index={}, profile={:?}, repeated_tool_signature_count={}, threshold={}",
                session_id,
                turn_id,
                round_index,
                policy.profile,
                self.repeated_tool_signature_count,
                policy.repeated_tool_signature_threshold
            );
        }

        if policy.has_consecutive_command_failure_loop(self.consecutive_failed_commands) {
            warn!(
                "Context profile command-failure threshold reached: session_id={}, turn_id={}, round_index={}, profile={:?}, consecutive_failed_commands={}, threshold={}",
                session_id,
                turn_id,
                round_index,
                policy.profile,
                self.consecutive_failed_commands,
                policy.consecutive_failed_command_threshold
            );
        }
    }

    pub(super) fn repeated_tool_signature_count(recent_tool_signatures: &[String]) -> usize {
        let Some(last_signature) = recent_tool_signatures.last() else {
            return 0;
        };

        let repeated_count = recent_tool_signatures
            .iter()
            .rev()
            .take_while(|signature| *signature == last_signature)
            .count();

        if repeated_count >= 2 {
            repeated_count
        } else {
            0
        }
    }

    pub(super) fn consecutive_failed_commands(messages: &[Message]) -> usize {
        let mut failures = 0;
        for message in messages.iter().rev() {
            let Some(failed) = Self::command_result_failed(message) else {
                continue;
            };

            if failed {
                failures += 1;
            } else {
                break;
            }
        }
        failures
    }

    pub(super) fn command_result_failed(message: &Message) -> Option<bool> {
        let MessageContent::ToolResult {
            tool_name,
            result,
            is_error,
            ..
        } = &message.content
        else {
            return None;
        };

        if !matches!(tool_name.as_str(), "Bash" | "Git") {
            return None;
        }

        Some(Self::tool_result_failed(result, *is_error))
    }

    pub(super) fn tool_result_failed(result: &serde_json::Value, is_error: bool) -> bool {
        is_error
            || Self::bool_field(result, "timed_out") == Some(true)
            || Self::bool_field(result, "interrupted") == Some(true)
            || Self::bool_field(result, "success") == Some(false)
            || Self::numeric_field(result, "exit_code").is_some_and(|code| code != 0)
    }

    pub(super) fn bool_field(value: &serde_json::Value, key: &str) -> Option<bool> {
        value.get(key).and_then(|field| field.as_bool())
    }

    pub(super) fn numeric_field(value: &serde_json::Value, key: &str) -> Option<i64> {
        value.get(key).and_then(|field| field.as_i64())
    }
}
