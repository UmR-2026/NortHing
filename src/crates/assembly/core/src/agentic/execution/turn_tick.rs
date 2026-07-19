//! Round 8 split sibling: turn_tick
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
    pub(super) async fn tick_impl(
        &self,
        context: &ExecutionContext,
        state: &mut ExecutionTurnState,
    ) -> NortHingResult<RoundTickResult> {
        // A2: Phase 2 — extracted loop body from execute_dialog_turn_impl.
        // Each call executes one model round (compression + LLM + tools + accumulate + decide).

        // W4-P: elapsed reference for the diagnostic probes below.
        let w4_start = std::time::Instant::now();

        const MAX_CONSECUTIVE_COMPRESSION_FAILURES: u32 = 3;
        const MAX_FAILED_TOOL_RECOVERY_ATTEMPTS: usize = 3;
        const MAX_PARTIAL_CONTINUATION_ATTEMPTS: usize = 3;

        // 1. Check max rounds limit
        if state.completed_rounds >= self.config.max_rounds {
            warn!(
                "Reached max rounds limit: {}, stopping execution",
                self.config.max_rounds
            );
            state.finalization_reason = Some("max_rounds");
            return Ok(RoundTickResult::Done);
        }

        // 2. Check and compress before sending AI request
        let (current_tokens, conversation_tokens, token_usage_ratio) = Self::estimate_auto_compression_pressure(
            &state.messages,
            state.tool_definitions.as_deref(),
            state.context_window,
        );
        debug!(
            "Round {} token usage before send: total={} / {}, conversation={} / {}, usage={:.1}%",
            state.round_index,
            current_tokens,
            state.context_window,
            conversation_tokens,
            state.context_window,
            token_usage_ratio * 100.0
        );

        let should_compress = state.enable_context_compression && token_usage_ratio >= state.compression_threshold;

        let circuit_breaker_open = state.consecutive_compression_failures >= MAX_CONSECUTIVE_COMPRESSION_FAILURES;

        if !should_compress {
            debug!(
                "No compression needed: session={}, token_usage={:.1}%, threshold={:.1}%",
                context.session_id,
                token_usage_ratio * 100.0,
                state.compression_threshold * 100.0
            );
        } else if circuit_breaker_open {
            warn!(
                "Compression circuit breaker open ({} consecutive failures), skipping full compression for round {}",
                state.consecutive_compression_failures, state.round_index
            );
        } else {
            info!(
                "Triggering context compression: session={}, token_usage={:.1}%, threshold={:.1}%",
                context.session_id,
                token_usage_ratio * 100.0,
                state.compression_threshold * 100.0
            );

            match self
                .compress_messages(
                    &context.session_id,
                    &context.dialog_turn_id,
                    state.messages.clone(),
                    current_tokens,
                    state.context_window,
                    state.ai_client.clone(),
                    &state.tool_definitions,
                    state.system_prompt_message.clone(),
                    &state.prepended_reminders,
                    state.primary_supports_image_understanding,
                    state.context_profile_policy.compression_contract_limit,
                    context.workspace.as_ref(),
                )
                .await
            {
                Ok(Some((compressed_tokens, compressed_messages))) => {
                    info!(
                        "Round {} compression completed: messages {} -> {}, tokens {} -> {}",
                        state.round_index,
                        state.messages.len(),
                        compressed_messages.len(),
                        current_tokens,
                        compressed_tokens,
                    );

                    state.messages = compressed_messages;
                    state.full_compression_count += 1;
                    state.consecutive_compression_failures = 0;
                }
                Ok(None) => {
                    debug!("No eligible multi-turn context available for compression");
                    state.consecutive_compression_failures = 0;
                }
                Err(e) => {
                    state.consecutive_compression_failures += 1;
                    state.compression_failure_count += 1;
                    error!(
                        "Round {} compression failed ({}/{}): {}, continuing with uncompressed context",
                        state.round_index,
                        state.consecutive_compression_failures,
                        MAX_CONSECUTIVE_COMPRESSION_FAILURES,
                        e
                    );
                }
            }
        }

        // L2: Emergency truncation
        let post_compress_tokens =
            Self::estimate_request_tokens_internal(&state.messages, state.tool_definitions.as_deref());
        if post_compress_tokens > state.context_window {
            warn!(
                "Round {} tokens ({}) still exceed context_window ({}), performing emergency truncation",
                state.round_index, post_compress_tokens, state.context_window
            );
            state.messages = Self::emergency_truncate_messages(
                state.messages.clone(),
                state.context_window,
                state.tool_definitions.as_deref(),
            );
            let after_truncate =
                Self::estimate_request_tokens_internal(&state.messages, state.tool_definitions.as_deref());
            info!(
                "Emergency truncation complete: tokens {} -> {}",
                post_compress_tokens, after_truncate
            );
        }

        let before_send_tokens =
            Self::estimate_request_tokens_internal(&state.messages, state.tool_definitions.as_deref());
        ContextHealthSnapshot::from_runtime_observations(
            ContextHealthSnapshot::token_usage_ratio(before_send_tokens, state.context_window),
            state.full_compression_count,
            state.compression_failure_count,
            &state.recent_tool_signatures,
            &state.messages,
        )
        .log(
            &context.session_id,
            &context.dialog_turn_id,
            state.round_index,
            "before_send",
        );

        // 3. Create round context
        let mut round_context_vars = state.execution_context_vars.clone();
        if context.skip_tool_confirmation {
            round_context_vars.insert("skip_tool_confirmation".to_string(), "true".to_string());
        }
        let unlocked_collapsed_tools =
            collect_product_unlocked_collapsed_tools(&state.messages, &state.collapsed_tools);

        let round_context = RoundContext {
            session_id: context.session_id.clone(),
            subagent_parent_info: context.subagent_parent_info.clone(),
            dialog_turn_id: context.dialog_turn_id.clone(),
            turn_index: context.turn_index,
            round_number: state.round_index,
            workspace: context.workspace.clone(),
            messages: state.messages.clone(),
            available_tools: state.available_tools.clone(),
            collapsed_tools: state.collapsed_tools.clone(),
            unlocked_collapsed_tools,
            model_name: state.ai_client.config.model.clone(),
            agent_type: state.agent_type.clone(),
            context_vars: round_context_vars,
            delegation_policy: context.delegation_policy,
            runtime_tool_restrictions: context.runtime_tool_restrictions.clone(),
            steering_interrupt: context.round_injection.as_ref().map(|source| {
                crate::agentic::round_preempt::DialogRoundInjectionInterrupt::new(
                    context.session_id.clone(),
                    context.dialog_turn_id.clone(),
                    Arc::clone(source),
                )
            }),
            cancellation_token: CancellationToken::new(),
            workspace_services: context.workspace_services.clone(),
            recover_partial_on_cancel: context.recover_partial_on_cancel,
        };

        // 4. Execute single model round
        debug!(
            "Starting model round: round_index={}, messages={}",
            state.round_index,
            state.messages.len()
        );

        let ai_messages = Self::build_ai_messages_for_send(
            &state.messages,
            &state.ai_client.config.format,
            context.workspace.as_ref().map(|workspace| workspace.root_path()),
            &context.dialog_turn_id,
            state.primary_supports_image_understanding,
            &state.prepended_reminders.ordered_reminders(),
        )
        .await?;

        info!(
            "W4-P: before execute_round elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
        let round_result = self
            .round_executor
            .execute_round(
                state.ai_client.clone(),
                round_context,
                ai_messages,
                state.tool_definitions.clone(),
                Some(state.context_window),
            )
            .await?;
        info!(
            "W4-P: after execute_round elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );

        debug!(
            "Model round completed: round_index={}, has_more_rounds={}, tool_calls={}",
            state.round_index,
            round_result.has_more_rounds,
            round_result.tool_calls.len()
        );
        state.completed_rounds += 1;

        // 5. Accumulate state
        if let Some(ref usage) = round_result.usage {
            state.last_usage = Some(usage.clone());
        }

        state.messages.push(round_result.assistant_message.clone());
        if let Err(e) = self
            .session_manager
            .add_message(&context.session_id, round_result.assistant_message.clone())
            .await
        {
            warn!("Failed to update assistant message in memory: {}", e);
        }

        for tool_result_msg in round_result.tool_result_messages.iter() {
            state.messages.push(tool_result_msg.clone());
            if let Err(e) = self
                .session_manager
                .add_message(&context.session_id, tool_result_msg.clone())
                .await
            {
                warn!("Failed to update tool result message in memory: {}", e);
            }
        }

        state.total_tools += round_result.tool_calls.len();

        if round_result.partial_recovery_reason.is_some() {
            state.last_partial_recovery_reason = round_result.partial_recovery_reason.clone();
        }

        if let Some(round_signature) = Self::tool_call_signature(&round_result.tool_calls) {
            state.recent_tool_signatures.push(round_signature.clone());
            if Self::failed_tool_round_signature(&round_result.tool_calls, &round_result.tool_result_messages).is_some()
            {
                state.recent_failed_tool_signatures.push(round_signature);
            } else {
                state.recent_failed_tool_signatures.clear();
                state.failed_tool_recovery_attempts = 0;
            }
        } else {
            state.recent_tool_signatures.clear();
            state.recent_failed_tool_signatures.clear();
            state.failed_tool_recovery_attempts = 0;
        }

        let after_round_tokens =
            Self::estimate_request_tokens_internal(&state.messages, state.tool_definitions.as_deref());
        let after_round_health = ContextHealthSnapshot::from_runtime_observations(
            ContextHealthSnapshot::token_usage_ratio(after_round_tokens, state.context_window),
            state.full_compression_count,
            state.compression_failure_count,
            &state.recent_tool_signatures,
            &state.messages,
        );
        after_round_health.log(
            &context.session_id,
            &context.dialog_turn_id,
            state.round_index,
            "after_round",
        );
        after_round_health.log_policy_thresholds(
            &context.session_id,
            &context.dialog_turn_id,
            state.round_index,
            &state.context_profile_policy,
        );

        // 6. Loop detection
        let max_consec = state
            .context_profile_policy
            .effective_loop_threshold(self.config.max_consecutive_same_tool);
        if state.recent_failed_tool_signatures.len() >= max_consec {
            let tail = &state.recent_failed_tool_signatures[state.recent_failed_tool_signatures.len() - max_consec..];
            if tail.windows(2).all(|w| w[0] == w[1]) {
                if state.failed_tool_recovery_attempts < MAX_FAILED_TOOL_RECOVERY_ATTEMPTS {
                    state.failed_tool_recovery_attempts += 1;
                    warn!(
                        "Repeated tool failure detected: {} consecutive rounds with identical tool signatures, injecting recovery prompt #{}",
                        max_consec, state.failed_tool_recovery_attempts
                    );
                    let reminder = format!(
                        "<system_reminder>Repeated tool failure detected: the same tool call with identical arguments has failed {} times in a row. \
                        The current approach is not making progress. You MUST now change your strategy: \
                        (1) if the tool keeps failing, try a completely different approach or tool; \
                        (2) if you are stuck, step back and reason about the root cause before acting; \
                        (3) if the task is genuinely impossible with the available tools, provide a clear explanation to the user. \
                        Do NOT repeat the same tool call again.</system_reminder>",
                        max_consec
                    );
                    let user_msg = Message::internal_reminder(InternalReminderKind::LoopRecovery, reminder)
                        .with_turn_id(context.dialog_turn_id.clone());
                    state.messages.push(user_msg.clone());
                    if let Err(e) = self.session_manager.add_message(&context.session_id, user_msg).await {
                        warn!("Failed to persist failed-tool recovery reminder: {}", e);
                    }
                    state.recent_failed_tool_signatures.clear();
                } else {
                    warn!(
                        "Repeated tool failure detected: {} consecutive rounds with identical tool signatures, max recovery attempts ({}) exhausted, finalizing without tools",
                        max_consec, MAX_FAILED_TOOL_RECOVERY_ATTEMPTS
                    );
                    state.finalization_reason = Some("repeated_tool_failures");
                    return Ok(RoundTickResult::Done);
                }
            }
        }

        if Self::is_periodic_tool_signature_loop(&state.recent_failed_tool_signatures, max_consec) {
            let window_size = max_consec.max(1).saturating_mul(2);
            if state.failed_tool_recovery_attempts < MAX_FAILED_TOOL_RECOVERY_ATTEMPTS {
                state.failed_tool_recovery_attempts += 1;
                warn!(
                    "Repeated tool failure detected: last {} failed rounds form a periodic tool-call pattern (<= {} distinct signatures, each repeated), injecting recovery prompt #{}",
                    window_size, max_consec, state.failed_tool_recovery_attempts
                );
                let reminder = format!(
                    "<system_reminder>Repeated tool failure detected: your last {} failed tool calls form a repeating pattern with no new progress. \
                    You are cycling between failing actions without advancing the task. You MUST now change your strategy: \
                    (1) try a completely different approach or tool; \
                    (2) step back and reason about the root cause before acting; \
                    (3) if the task is genuinely impossible with the available tools, provide a clear explanation to the user. \
                    Do NOT repeat the same pattern of tool calls.</system_reminder>",
                    window_size
                );
                let user_msg = Message::internal_reminder(InternalReminderKind::PeriodicLoopRecovery, reminder)
                    .with_turn_id(context.dialog_turn_id.clone());
                state.messages.push(user_msg.clone());
                if let Err(e) = self.session_manager.add_message(&context.session_id, user_msg).await {
                    warn!("Failed to persist periodic loop recovery reminder: {}", e);
                }
                state.recent_failed_tool_signatures.clear();
            } else {
                warn!(
                    "Repeated tool failure detected: last {} failed rounds form a periodic tool-call pattern, max recovery attempts ({}) exhausted, finalizing without tools",
                    window_size, MAX_FAILED_TOOL_RECOVERY_ATTEMPTS
                );
                state.finalization_reason = Some("repeated_tool_failures");
                return Ok(RoundTickResult::Done);
            }
        }

        // 7. Round injection (Codex-style mid-turn steering)
        let mut injection_applied = false;
        if let Some(source) = context.round_injection.as_ref() {
            let pending = source.take_pending(&context.session_id, &context.dialog_turn_id);
            if !pending.is_empty() {
                info!(
                    "Injecting {} round message(s) at round boundary: session_id={}, dialog_turn_id={}, round_index={}",
                    pending.len(),
                    context.session_id,
                    context.dialog_turn_id,
                    state.round_index
                );
                for injection in pending {
                    let wrapped = match injection.kind {
                        RoundInjectionKind::UserSteering => format!(
                            "<system_reminder>\nThe user sent a new message while this turn was running. You have just finished the previous atomic action; handle this new user message now as the current direction, while preserving the existing conversation and task context. Do not ignore it or wait for a separate future turn.\n\nNew user message:\n{}\n</system_reminder>",
                            injection.content
                        ),
                        RoundInjectionKind::BackgroundResult => format!(
                            "<system_reminder>\nA background task has finished and returned new information while this turn was running. Incorporate it into your current work immediately when relevant. Do not wait for a separate future turn.\n\nBackground result:\n{}\n</system_reminder>",
                            injection.content
                        ),
                        RoundInjectionKind::ThreadGoalObjectiveUpdated => injection.content.clone(),
                    };
                    let reminder_kind = match injection.kind {
                        RoundInjectionKind::UserSteering => InternalReminderKind::UserSteering,
                        RoundInjectionKind::BackgroundResult => InternalReminderKind::BackgroundResult,
                        RoundInjectionKind::ThreadGoalObjectiveUpdated => InternalReminderKind::GoalObjectiveUpdated,
                    };
                    let user_msg =
                        Message::internal_reminder(reminder_kind, wrapped).with_turn_id(context.dialog_turn_id.clone());
                    state.messages.push(user_msg.clone());
                    if let Err(e) = self.session_manager.add_message(&context.session_id, user_msg).await {
                        warn!("Failed to persist user steering message in memory: {}", e);
                    }

                    self.emit_event(
                        AgenticEvent::UserSteeringInjected {
                            session_id: context.session_id.clone(),
                            turn_id: context.dialog_turn_id.clone(),
                            round_index: state.round_index,
                            steering_id: injection.id,
                            content: injection.content,
                            display_content: injection.display_content,
                        },
                        EventPriority::Normal,
                    )
                    .await;
                    injection_applied = true;
                }
            }
        }

        // 8. Decide whether to end the turn
        if injection_applied {
            // Continue to next round so the model can respond to the steering
            state.round_index += 1;
            return Ok(RoundTickResult::Continue);
        } else if !round_result.has_more_rounds {
            if round_result.had_assistant_text {
                if let Some(ref reason) = round_result.partial_recovery_reason {
                    if Self::should_continue_after_partial_response(reason) {
                        state.partial_continuation_attempts += 1;
                        if state.partial_continuation_attempts <= MAX_PARTIAL_CONTINUATION_ATTEMPTS {
                            let reminder = format!(
                                "<system_reminder>Your previous assistant response was interrupted mid-stream ({reason}). Continue writing from exactly where you stopped. Do not repeat content that was already delivered; pick up seamlessly and complete the answer.</system_reminder>"
                            );
                            let user_msg =
                                Message::internal_reminder(InternalReminderKind::InterruptedContinue, reminder.clone())
                                    .with_turn_id(context.dialog_turn_id.clone());
                            state.messages.push(user_msg.clone());
                            if let Err(e) = self.session_manager.add_message(&context.session_id, user_msg).await {
                                warn!("Failed to persist partial continuation reminder: {}", e);
                            }
                            warn!(
                                "Partial stream recovery with assistant text; injecting continuation reminder #{}/{}: turn={}, round={}, reason={}",
                                state.partial_continuation_attempts,
                                MAX_PARTIAL_CONTINUATION_ATTEMPTS,
                                context.dialog_turn_id,
                                state.round_index,
                                reason
                            );
                            state.round_index += 1;
                            return Ok(RoundTickResult::Continue);
                        } else {
                            warn!(
                                "Partial stream continuation attempts exhausted; accepting truncated answer: turn={}, round={}, reason={}",
                                context.dialog_turn_id, state.round_index, reason
                            );
                            state.finalization_reason = Some("partial_truncated");
                            return Ok(RoundTickResult::Done);
                        }
                    } else {
                        debug!(
                            "Model round {} ended with partial answer after cancellation, reason: {:?}",
                            state.round_index, round_result.finish_reason
                        );
                        return Ok(RoundTickResult::Done);
                    }
                } else {
                    debug!(
                        "Model round {} ended with final answer, reason: {:?}",
                        state.round_index, round_result.finish_reason
                    );
                    return Ok(RoundTickResult::Done);
                }
            } else if round_result.had_thinking_content {
                state.thinking_only_rescue_attempts += 1;
                let reminder = "<system_reminder>The previous round produced internal reasoning only — no tool call and no user-visible response. You MUST now either: (1) call the single tool that best advances the user's task, or (2) write your final answer to the user. Do not produce another round of reasoning without taking action.</system_reminder>".to_string();
                let user_msg = Message::internal_reminder(InternalReminderKind::ThinkingOnlyRescue, reminder.clone())
                    .with_turn_id(context.dialog_turn_id.clone());
                state.messages.push(user_msg.clone());
                if let Err(e) = self.session_manager.add_message(&context.session_id, user_msg).await {
                    warn!("Failed to persist thinking-only rescue reminder: {}", e);
                }
                warn!(
                    "Thinking-only round detected; injecting rescue reminder #{}: turn={}, round={}",
                    state.thinking_only_rescue_attempts, context.dialog_turn_id, state.round_index
                );
                state.round_index += 1;
                return Ok(RoundTickResult::Continue);
            } else {
                warn!(
                    "Empty round (no text/thinking/tool_call); ending turn: turn={}, round={}",
                    context.dialog_turn_id, state.round_index
                );
                state.finalization_reason = Some("empty_round");
                return Ok(RoundTickResult::Done);
            }
        }

        // 9. Check cancellation
        if self.round_executor.is_dialog_turn_cancelled(&context.dialog_turn_id) {
            debug!(
                "Dialog turn cancelled, stopping execution: dialog_turn_id={}",
                context.dialog_turn_id
            );
            self.emit_event(
                AgenticEvent::DialogTurnCancelled {
                    session_id: context.session_id.clone(),
                    turn_id: context.dialog_turn_id.clone(),
                },
                EventPriority::High,
            )
            .await;
            return Ok(RoundTickResult::Cancelled);
        }

        // 10. Continue to next round
        state.round_index += 1;

        debug!(
            "Model round {} completed, continuing to round {}",
            state.round_index - 1,
            state.round_index
        );

        Ok(RoundTickResult::Continue)
    }
}
