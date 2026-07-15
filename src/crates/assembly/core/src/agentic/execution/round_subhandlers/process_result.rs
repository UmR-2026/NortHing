//! `RoundExecutor::process_result` sub-handler.
//!
//! Post-loop finalize + tool execution + build `RoundResult`:
//! 1. Complete the model exchange trace.
//! 2. Emit token usage update.
//! 3. Emit `ModelRoundCompleted` event.
//! 4. If no tool calls: return with `FinishReason::Complete`.
//! 5. If tool calls present: build `ToolExecutionContext`, compute
//!    confirmation/timeout from global config, run `tool_pipeline`,
//!    apply round-level tool result budget.
//! 6. Build assistant message + tool result messages and return.

use super::super::round_executor::RoundExecutor;
use super::super::types::{FinishReason, RoundResult};
use super::round_state::{DispatchOutcome, RoundState};
use crate::agentic::core::Message;
use crate::agentic::events::{AgenticEvent, EventPriority};
use crate::agentic::tools::pipeline::{ToolExecutionContext, ToolExecutionOptions};
use crate::agentic::tools::registry::global_tool_registry;
use crate::agentic::tools::tool_context_runtime;
use crate::agentic::tools::tool_result_storage;
use crate::agentic::MessageContent;
use crate::service::config::GlobalConfigManager;
use crate::util::elapsed_ms_u64;
use crate::util::errors::{NortHingError, NortHingResult};
use std::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

impl RoundExecutor {
    /// Post-loop finalize + tool execution + build RoundResult.
    pub(crate) async fn process_result(
        &self,
        state: &mut RoundState,
        outcome: DispatchOutcome,
    ) -> NortHingResult<RoundResult> {
        Self::complete_model_exchange_trace(
            state.trace_config.as_ref(),
            outcome.trace_handle.as_ref(),
            Self::final_trace_response(&outcome.stream_result),
        )
        .await;

        // Model returned successfully (output to AI log file)
        if let Some(ref reason) = outcome.stream_result.partial_recovery_reason {
            warn!(
                "Stream recovered with partial output: session_id={}, state.round_id={}, reason={}, text_len={}, tool_calls={}",
                state.context.session_id,
                state.round_id,
                reason,
                outcome.stream_result.full_text.len(),
                outcome.stream_result.tool_calls.len()
            );
        }

        let tool_names: Vec<&str> = outcome
            .stream_result
            .tool_calls
            .iter()
            .map(|tc| tc.tool_name.as_str())
            .collect();
        debug!(
            target: "ai::model_response",
            "Model response received: text_length={}, tool_calls={}, token_usage={:?}, outcome.send_to_stream_ms={}, outcome.stream_processing_ms={}, first_chunk_ms={:?}, first_visible_output_ms={:?}",
            outcome.stream_result.full_text.len(),
            if tool_names.is_empty() { "none".to_string() } else { tool_names.join(", ") },
            outcome.stream_result.usage.as_ref().map(|u| format!("input={}, output={}, total={}", u.prompt_token_count, u.candidates_token_count, u.total_token_count)).unwrap_or_else(|| "none".to_string()),
            outcome.send_to_stream_ms,
            outcome.stream_processing_ms,
            outcome.stream_result.first_chunk_ms,
            outcome.stream_result.first_visible_output_ms
        );

        // If stream response contains usage info, record it before the
        // post-stream cancellation gate. A user can press stop after the
        // provider returned usage but before this round settles; dropping that
        // usage makes cancelled turns look unaccounted even though the provider
        // already supplied authoritative counts.
        if let Some(ref usage) = outcome.stream_result.usage {
            self.emit_token_usage_update(&state.context, usage, state.context_window, state.is_subagent)
                .await;
        }

        // Check cancellation token again after stream processing completes.
        if state.cancel_token.is_cancelled() {
            debug!(
                "Cancel token detected after stream processing, stopping execution: session_id={}",
                state.context.session_id
            );
            return Err(NortHingError::Cancelled("Execution cancelled".to_string()));
        }

        // Emit model round completed event
        debug!(
            "Preparing to send ModelRoundCompleted event: round={}, has_tools={}",
            state.round_id,
            !outcome.stream_result.tool_calls.is_empty()
        );

        self.emit_event(
            AgenticEvent::ModelRoundCompleted {
                session_id: state.context.session_id.clone(),
                turn_id: state.context.dialog_turn_id.clone(),
                round_id: state.round_id.clone(),
                has_tool_calls: !outcome.stream_result.tool_calls.is_empty(),
                duration_ms: Some(elapsed_ms_u64(state.round_started_at)),
                provider_id: None,
                model_id: Some(state.context.model_name.clone()),
                model_alias: Some(state.context.model_name.clone()),
                first_chunk_ms: outcome.stream_result.first_chunk_ms,
                first_visible_output_ms: outcome.stream_result.first_visible_output_ms,
                stream_duration_ms: Some(outcome.stream_processing_ms),
                attempt_count: Some((state.attempt_index + 1) as u32),
                failure_category: None,
                token_details: outcome
                    .stream_result
                    .usage
                    .as_ref()
                    .and_then(super::super::round_executor::rexec_types::token_details_from_usage),
            },
            EventPriority::High,
        )
        .await;

        debug!("ModelRoundCompleted event sent");

        // If no tool calls, this round ends
        if outcome.stream_result.tool_calls.is_empty() {
            debug!("No tool calls, round completed: round={}", state.round_id);

            // Create assistant message (includes thinking content, supports interleaved thinking mode)
            let reasoning = if outcome.stream_result.full_thinking.is_empty() {
                if outcome.stream_result.reasoning_content_present {
                    Some(String::new())
                } else {
                    None
                }
            } else {
                Some(outcome.stream_result.full_thinking.clone())
            };
            let assistant_message =
                Message::assistant_with_reasoning(reasoning, outcome.stream_result.full_text.clone(), vec![])
                    .with_turn_id(state.context.dialog_turn_id.clone())
                    .with_round_id(state.round_id.clone())
                    .with_thinking_signature(outcome.stream_result.thinking_signature.clone());

            debug!("Returning RoundResult: has_more_rounds=false");
            debug!(
                "Model round timing summary: session_id={}, turn_id={}, state.round_id={}, tool_calls=0, outcome.send_to_stream_ms={}, outcome.stream_processing_ms={}, first_chunk_ms={:?}, first_visible_output_ms={:?}, tool_phase_ms=0, round_total_ms={}, has_more_rounds=false",
                state.context.session_id,
                state.context.dialog_turn_id,
                state.round_id,
                outcome.send_to_stream_ms,
                outcome.stream_processing_ms,
                outcome.stream_result.first_chunk_ms,
                outcome.stream_result.first_visible_output_ms,
                elapsed_ms_u64(state.round_started_at)
            );

            // Note: Do not cleanup cancellation token here, as this is only the end of a single model round
            // Cancellation token will be cleaned up by ExecutionEngine when the entire dialog turn ends

            return Ok(RoundResult {
                assistant_message,
                tool_calls: vec![],
                tool_result_messages: vec![],
                has_more_rounds: false,
                finish_reason: FinishReason::Complete,
                usage: outcome.stream_result.usage.clone(),
                provider_metadata: outcome.stream_result.provider_metadata.clone(),
                partial_recovery_reason: outcome.stream_result.partial_recovery_reason.clone(),
                had_assistant_text: Self::has_user_visible_assistant_text(&outcome.stream_result.full_text),
                had_thinking_content: !outcome.stream_result.full_thinking.is_empty(),
            });
        }

        // Check cancellation token before executing tools
        if state.cancel_token.is_cancelled() {
            debug!(
                "Cancel token detected before tool execution, stopping execution: session_id={}",
                state.context.session_id
            );
            return Err(NortHingError::Cancelled("Execution cancelled".to_string()));
        }

        let tool_calls = outcome.stream_result.tool_calls.clone();

        // Execute tool calls
        debug!("Preparing to execute tool calls: count={}", tool_calls.len());

        let tool_phase_started_at = Instant::now();
        let tool_results = if let Some(tool_pipeline) = &self.tool_pipeline {
            // Create tool execution context
            let allowed_tools = state.context.available_tools.clone();
            let tool_context = ToolExecutionContext {
                session_id: state.context.session_id.clone(),
                dialog_turn_id: state.context.dialog_turn_id.clone(),
                round_id: state.round_id.clone(),
                agent_type: state.context.agent_type.clone(),
                workspace: state.context.workspace.clone(),
                context_vars: state.context.context_vars.clone(),
                subagent_parent_info: state.subagent_parent_info.clone(),
                delegation_policy: state.context.delegation_policy,
                collapsed_tools: state.context.collapsed_tools.clone(),
                unlocked_collapsed_tools: state.context.unlocked_collapsed_tools.clone(),
                allowed_tools,
                runtime_tool_restrictions: state.context.runtime_tool_restrictions.clone(),
                steering_interrupt: state.context.steering_interrupt.clone(),
                workspace_services: state.context.workspace_services.clone(),
            };

            // Read tool execution related configuration from global config
            let (needs_confirmation, tool_execution_timeout, tool_confirmation_timeout) = {
                let config_service = GlobalConfigManager::service().await.ok();

                // Timeout and skip confirmation settings
                let (exec_timeout, confirm_timeout, skip_confirmation) = if let Some(ref service) = config_service {
                    let ai_config: crate::service::config::types::AIConfig =
                        service.config(Some("ai")).await.unwrap_or_default();

                    // R1 Phase 3: prefer ShellSecurityConfig (mode-aware)
                    // over legacy skip_tool_confirmation boolean.
                    // agent_type comes from state.context; fall back to "agentic".
                    //
                    // AND semantics: skip confirmation only when BOTH
                    // shell_security AND legacy flag agree. This allows
                    // new ShellSecurityConfig.mode_overrides to take effect
                    // even if legacy skip_tool_confirmation defaults to true.
                    //
                    // Migration path:
                    // - Old configs (skip=true, no shell_security) → skip=true
                    //   (unchanged behavior, shell_security default = Permissive)
                    // - New config with mode_override Strict → skip=false
                    //   (mode override wins via AND)
                    let agent_type = if state.context.agent_type.is_empty() {
                        "agentic"
                    } else {
                        &state.context.agent_type
                    };
                    let shell_security_skip = ai_config.shell_security.should_skip_confirmation(agent_type);

                    // AND: both must agree to skip. Default to skip only when
                    // both default configs are set (preserves prior behavior).
                    let combined_skip = shell_security_skip && ai_config.skip_tool_confirmation;

                    if combined_skip {
                        debug!(
                            "R1: skipping tool confirmation (agent_type={}, shell_security={}, legacy_skip={})",
                            agent_type, shell_security_skip, ai_config.skip_tool_confirmation
                        );
                    }

                    (
                        ai_config.tool_execution_timeout_secs,
                        ai_config.tool_confirmation_timeout_secs,
                        combined_skip,
                    )
                } else {
                    (None, None, false) // Default: no timeout, requires confirmation
                };

                let skip_from_context = state
                    .context
                    .context_vars
                    .get("skip_tool_confirmation")
                    .map(|v| v == "true")
                    .unwrap_or(false);

                let needs_confirm = if skip_confirmation || skip_from_context {
                    false
                } else {
                    // Otherwise judge based on tool's needs_permissions()
                    let registry = global_tool_registry();
                    let tool_registry = registry.read().await;
                    let mut requires_permission = false;

                    for tool_call in &outcome.stream_result.tool_calls {
                        if let Some(tool) = tool_registry.get_tool(&tool_call.tool_name) {
                            if tool.needs_permissions(Some(&tool_call.arguments)) {
                                requires_permission = true;
                                break;
                            }
                        }
                    }

                    requires_permission
                };

                (needs_confirm, exec_timeout, confirm_timeout)
            };

            // Create tool execution options (use configured timeout values)
            let tool_options = ToolExecutionOptions {
                confirm_before_run: needs_confirmation,
                timeout_secs: tool_execution_timeout,
                confirmation_timeout_secs: tool_confirmation_timeout,
                ..ToolExecutionOptions::default()
            };

            let storage_context = tool_context_runtime::build_tool_use_context_for_execution_context(
                &tool_context,
                Some(format!("round-budget-{}", state.round_id)),
                self.computer_use_host(),
                CancellationToken::new(),
                None, // K.2.3 follow-up: round executor doesn't spawn long-running skills
            );

            // Execute tools — convert pipeline-level Err into per-tool error results
            // so the model always receives a tool_result for every tool_call.
            let execution_results = match tool_pipeline
                .execute_tools(tool_calls.clone(), tool_context, tool_options)
                .await
            {
                Ok(results) => results,
                Err(e) => {
                    error!(
                        "Tool pipeline execution failed, generating error results for all {} tool calls: {}",
                        tool_calls.len(),
                        e
                    );
                    tool_calls
                        .iter()
                        .map(|tc| crate::agentic::tools::pipeline::ToolExecutionResult {
                            tool_id: tc.tool_id.clone(),
                            tool_name: tc.tool_name.clone(),
                            result: crate::agentic::core::ToolResult {
                                tool_id: tc.tool_id.clone(),
                                tool_name: tc.tool_name.clone(),
                                result: serde_json::json!({
                                    "error": e.to_string(),
                                    "message": format!("Tool pipeline execution failed: {}", e)
                                }),
                                result_for_assistant: Some(format!("Tool execution failed: {}", e)),
                                is_error: true,
                                duration_ms: None,
                                image_attachments: None,
                            },
                            execution_time_ms: 0,
                        })
                        .collect()
                }
            };

            // Convert to ToolResult, then enforce the aggregate budget for this model round.
            let tool_results = execution_results.into_iter().map(|r| r.result).collect();
            tool_result_storage::apply_round_tool_result_budget(tool_results, &storage_context).await
        } else {
            vec![]
        };
        let tool_phase_ms = elapsed_ms_u64(tool_phase_started_at);

        // Create assistant message (includes tool calls and thinking content, supports interleaved thinking mode)
        let reasoning = if outcome.stream_result.full_thinking.is_empty() {
            if outcome.stream_result.reasoning_content_present {
                Some(String::new())
            } else {
                None
            }
        } else {
            Some(outcome.stream_result.full_thinking.clone())
        };
        let assistant_message =
            Message::assistant_with_reasoning(reasoning, outcome.stream_result.full_text.clone(), tool_calls.clone())
                .with_turn_id(state.context.dialog_turn_id.clone())
                .with_round_id(state.round_id.clone())
                .with_thinking_signature(outcome.stream_result.thinking_signature.clone());

        debug!(
            "Tool execution completed, creating message: assistant_msg_len={}, tool_results={}",
            match &assistant_message.content {
                MessageContent::Text(t) => t.len(),
                MessageContent::Mixed { text, .. } => text.len(),
                _ => 0,
            },
            tool_results.len()
        );

        // Create tool result messages (also need to set turn_id and state.round_id)
        let dialog_turn_id = state.context.dialog_turn_id.clone();
        let round_id_clone = state.round_id.clone();
        let tool_result_messages: Vec<Message> = tool_results
            .iter()
            .map(|result| {
                Message::tool_result(result.clone())
                    .with_turn_id(dialog_turn_id.clone())
                    .with_round_id(round_id_clone.clone())
            })
            .collect();

        let has_more_rounds = !tool_result_messages.is_empty();

        debug!(
            "Returning RoundResult: has_more_rounds={}, tool_result_messages={}",
            has_more_rounds,
            tool_result_messages.len()
        );
        debug!(
            "Model round timing summary: session_id={}, turn_id={}, state.round_id={}, tool_calls={}, tool_results={}, outcome.send_to_stream_ms={}, outcome.stream_processing_ms={}, first_chunk_ms={:?}, first_visible_output_ms={:?}, tool_phase_ms={}, round_total_ms={}, has_more_rounds={}",
            state.context.session_id,
            state.context.dialog_turn_id,
            state.round_id,
            outcome.stream_result.tool_calls.len(),
            tool_result_messages.len(),
            outcome.send_to_stream_ms,
            outcome.stream_processing_ms,
            outcome.stream_result.first_chunk_ms,
            outcome.stream_result.first_visible_output_ms,
            tool_phase_ms,
            elapsed_ms_u64(state.round_started_at),
            has_more_rounds
        );

        // Note: Do not cleanup cancellation token here, as there may be subsequent model rounds
        // Cancellation token will be cleaned up by ExecutionEngine when the entire dialog turn ends

        Ok(RoundResult {
            assistant_message,
            tool_calls,
            tool_result_messages,
            has_more_rounds,
            finish_reason: if has_more_rounds {
                FinishReason::ToolCalls
            } else {
                FinishReason::Complete
            },
            usage: outcome.stream_result.usage.clone(),
            provider_metadata: outcome.stream_result.provider_metadata.clone(),
            partial_recovery_reason: outcome.stream_result.partial_recovery_reason.clone(),
            had_assistant_text: Self::has_user_visible_assistant_text(&outcome.stream_result.full_text),
            had_thinking_content: !outcome.stream_result.full_thinking.is_empty(),
        })
    }
}
