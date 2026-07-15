//! Round 8 split sibling: turn_main_loop
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
    pub(super) async fn execute_dialog_turn_impl(
        &self,
        agent_type: String,
        initial_messages: Vec<Message>,
        context: ExecutionContext,
        start_time: std::time::Instant,
        initial_count: usize,
    ) -> NortHingResult<ExecutionResult> {
        let dialog_turn_id = context.dialog_turn_id.clone();

        debug!(
            "Executing dialog turn implementation: dialog_turn_id={}",
            dialog_turn_id
        );

        // A2: Initialize turn state once, then drive rounds via tick().
        let mut state = self
            .init_turn(agent_type.clone(), initial_messages.clone(), &context)
            .await?;

        // Drive rounds until tick says Done, Cancelled, or Error.
        loop {
            match self.tick(&context, &mut state).await? {
                RoundTickResult::Continue => continue,
                RoundTickResult::Done => break,
                RoundTickResult::Cancelled => {
                    return Err(NortHingError::cancelled("Dialog cancelled"));
                }
                RoundTickResult::Error { error } => {
                    return Err(NortHingError::Agent(error));
                }
            }
        }

        // Finalize (e.g. after max_rounds or repeated_tool_failures)
        let _ = self.finalize_turn(&context, &mut state).await?;

        // Build and return result
        let result = self.build_result(&state, start_time, initial_count);

        // Post-processing hook: DeepResearch citation renumbering
        #[cfg(feature = "product-full")]
        {
            if result.success && agent_type == "DeepResearch" {
                if let Some(workspace) = context.workspace.as_ref() {
                    crate::agentic::agents::citation_renumber::run_for_session_workspace(
                        workspace.root_path(),
                        &context.session_id,
                    )
                    .await;
                }
            }
        }

        // Emit dialog turn completed event
        let duration_ms = elapsed_ms_u64(start_time);
        let effective_finish_reason = state.finalization_reason.unwrap_or("complete");
        let _ = self
            .event_queue
            .enqueue(
                AgenticEvent::DialogTurnCompleted {
                    session_id: context.session_id.clone(),
                    turn_id: context.dialog_turn_id.clone(),
                    total_rounds: state.completed_rounds,
                    total_tools: state.total_tools,
                    duration_ms,
                    partial_recovery_reason: state.last_partial_recovery_reason.clone(),
                    success: Some(result.success),
                    finish_reason: Some(effective_finish_reason.to_string()),
                },
                None,
            )
            .await;

        // Print dialog turn token statistics
        if let Some(ref usage) = state.last_usage {
            info!(
                "Dialog turn completed - Token stats: turn_id={}, rounds={}, tools={}, duration={}ms, prompt_tokens={}, completion_tokens={}, total_tokens={}",
                context.dialog_turn_id,
                state.completed_rounds,
                state.total_tools,
                duration_ms,
                usage.prompt_token_count,
                usage.candidates_token_count,
                usage.total_token_count
            );
        } else {
            warn!("Dialog turn completed but token stats not available");
        }

        Ok(result)
    }

    pub(super) async fn cancel_dialog_turn_impl(&self, dialog_turn_id: &str) -> NortHingResult<()> {
        debug!("Cancelling dialog turn: dialog_turn_id={}", dialog_turn_id);
        let result = self.round_executor.cancel_dialog_turn(dialog_turn_id).await;
        if result.is_ok() {
            debug!("Dialog turn cancelled successfully: dialog_turn_id={}", dialog_turn_id);
        } else {
            error!(
                "Failed to cancel dialog turn: dialog_turn_id={}, error={:?}",
                dialog_turn_id, result
            );
        }
        result
    }

    pub(super) fn has_active_turn_impl(&self, dialog_turn_id: &str) -> bool {
        self.round_executor.has_active_dialog_turn(dialog_turn_id)
    }

    pub(super) fn register_cancel_token_impl(&self, dialog_turn_id: &str, token: CancellationToken) {
        self.round_executor.register_cancel_token(dialog_turn_id, token)
    }

    pub(super) fn cancel_token_for_dialog_turn_impl(&self, dialog_turn_id: &str) -> Option<CancellationToken> {
        self.round_executor.cancel_token_for_dialog_turn(dialog_turn_id)
    }

    pub(super) async fn cleanup_cancel_token_impl(&self, dialog_turn_id: &str) {
        self.round_executor.cleanup_dialog_turn(dialog_turn_id).await
    }

    pub(super) async fn emit_event(&self, event: AgenticEvent, priority: EventPriority) {
        let _ = self.event_queue.enqueue(event, Some(priority)).await;
    }
}
