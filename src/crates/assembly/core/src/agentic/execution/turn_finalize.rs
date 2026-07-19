//! Round 8 split sibling: turn_finalize
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
    pub(super) async fn finalize_turn_impl(
        &self,
        context: &ExecutionContext,
        state: &mut ExecutionTurnState,
    ) -> NortHingResult<Option<(Message, Option<crate::util::types::ai::GeminiUsage>)>> {
        let effective_finish_reason = match state.finalization_reason {
            Some(r) => r,
            None => return Ok(None),
        };

        let finalize_reminder = match effective_finish_reason {
            "queued_user_message"
                if state
                    .messages
                    .iter()
                    .rev()
                    .find(|message| message.role == MessageRole::Assistant)
                    .is_some_and(Self::assistant_has_tool_calls)
                    && Self::has_tool_result_after_last_assistant(&state.messages) =>
            {
                Some(Self::FINALIZE_AFTER_TOOL_USE_REMINDER)
            }
            "repeated_tool_failures" => Some(Self::FINALIZE_AFTER_REPEATED_TOOL_FAILURES_REMINDER),
            "max_rounds" => Some(Self::FINALIZE_AFTER_MAX_ROUNDS_REMINDER),
            _ => None,
        };

        let Some(finalize_reminder) = finalize_reminder else {
            return Ok(None);
        };

        info!(
            "Finalizing dialog turn: session_id={}, turn_id={}, reason={}",
            context.session_id, context.dialog_turn_id, effective_finish_reason
        );

        let prepended_reminders = state.prepended_reminders.ordered_reminders();
        let final_round_result = self
            .run_finalize_round(
                state.ai_client.clone(),
                context,
                state.agent_type.clone(),
                state.completed_rounds,
                &state.execution_context_vars,
                state.primary_supports_image_understanding,
                &prepended_reminders,
                &state.messages,
                finalize_reminder,
                state.context_window,
            )
            .await?;

        let accepted = final_round_result.had_assistant_text
            && !Self::assistant_has_tool_calls(&final_round_result.assistant_message);
        let mut chosen_assistant_message: Option<Message> = None;
        let mut chosen_usage: Option<crate::util::types::ai::GeminiUsage> = final_round_result.usage.clone();

        if accepted {
            chosen_assistant_message = Some(final_round_result.assistant_message.clone());
        } else {
            // P1-10: First finalize round still returned tool calls
            // (rare; tools were not provided, but model hallucinated).
            // One last attempt with a stricter text-only reminder.
            warn!(
                "Finalize round still returned tool calls; retrying with text-only reminder: session_id={}, turn_id={}",
                context.session_id, context.dialog_turn_id
            );
            let retry_result = self
                .run_finalize_round(
                    state.ai_client.clone(),
                    context,
                    state.agent_type.clone(),
                    state.completed_rounds,
                    &state.execution_context_vars,
                    state.primary_supports_image_understanding,
                    &prepended_reminders,
                    &state.messages,
                    Self::FORCE_TEXT_ONLY_REMINDER,
                    state.context_window,
                )
                .await?;
            if !retry_result.had_assistant_text || Self::assistant_has_tool_calls(&retry_result.assistant_message) {
                warn!(
                    "Text-only retry did not return usable assistant text; keeping prior messages: session_id={}, turn_id={}",
                    context.session_id, context.dialog_turn_id
                );
            } else {
                // accepted = true; // Not needed; we set it below via chosen_assistant_message
                chosen_usage = retry_result.usage.clone();
                chosen_assistant_message = Some(retry_result.assistant_message);
            }
        }

        if let Some(msg) = chosen_assistant_message {
            state.completed_rounds += 1;
            if let Some(usage) = chosen_usage.clone() {
                state.last_usage = Some(usage);
            }
            state.messages.push(msg.clone());
            if let Err(e) = self.session_manager.add_message(&context.session_id, msg.clone()).await {
                warn!("Failed to update final assistant message in memory: {}", e);
            }
            Ok(Some((msg, chosen_usage)))
        } else {
            Ok(None)
        }
    }

    pub(super) fn build_result_impl(
        &self,
        state: &ExecutionTurnState,
        start_time: std::time::Instant,
        initial_count: usize,
    ) -> ExecutionResult {
        let duration_ms = elapsed_ms_u64(start_time);
        let effective_finish_reason = state.finalization_reason.unwrap_or("complete");

        let success = !matches!(
            effective_finish_reason,
            "finalize_failed" | "empty_round" | "max_rounds"
        );

        let finish_reason = match effective_finish_reason {
            "cancelled" => FinishReason::Cancelled,
            "tool_calls" => FinishReason::ToolCalls,
            "complete" => FinishReason::Complete,
            _ => FinishReason::Error,
        };

        let safe_initial_count = initial_count.min(state.messages.len());
        let new_messages = state.messages[safe_initial_count..].to_vec();

        if safe_initial_count != initial_count {
            warn!(
                "initial_count ({}) exceeds messages length ({}), adjusted to {}",
                initial_count,
                state.messages.len(),
                safe_initial_count
            );
        }

        ExecutionResult {
            final_message: state
                .messages
                .iter()
                .rev()
                .find(|message| message.role == MessageRole::Assistant)
                .cloned()
                .unwrap_or_else(|| Message::assistant(String::new())),
            total_rounds: state.completed_rounds,
            total_tools: state.total_tools,
            duration_ms,
            success,
            new_messages,
            finish_reason,
            partial_recovery_reason: state.last_partial_recovery_reason.clone(),
        }
    }
}
