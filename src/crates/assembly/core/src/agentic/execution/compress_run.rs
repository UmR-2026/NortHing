//! Round 48b split sibling: auto-compression + manual compaction
//!
//! Moved from execution/compression.rs.
//! Methods are `pub(super)` so the facade (`execution_engine.rs`) can call them.

use super::execution_engine::{ContextCompactionOutcome, ExecutionEngine};
use super::model_exchange_trace::{prepare_model_exchange_trace_for_workspace, ModelExchangeTraceOperation};
use super::types::ExecutionContext;
use crate::agentic::agents::PrependedPromptReminders;
use crate::agentic::core::Message;
use crate::agentic::events::{AgenticEvent, EventPriority};
use crate::agentic::session::CompressionMode;
use crate::agentic::WorkspaceBinding;
use crate::util::elapsed_ms_u64;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::types::ToolDefinition;
use std::sync::Arc;

impl ExecutionEngine {
    pub(super) async fn compress_messages_impl(
        &self,
        session_id: &str,
        dialog_turn_id: &str,
        runtime_messages: Vec<Message>,
        current_tokens: usize,
        context_window: usize,
        ai_client: Arc<crate::infrastructure::ai::AIClient>,
        tool_definitions: &Option<Vec<ToolDefinition>>,
        system_prompt_message: Message,
        prepended_prompt_reminders: &PrependedPromptReminders,
        primary_supports_image_understanding: bool,
        compression_contract_limit: usize,
        workspace: Option<&WorkspaceBinding>,
    ) -> NortHingResult<Option<(usize, Vec<Message>)>> {
        let mut session = self
            .session_manager
            .get_session(session_id)
            .ok_or_else(|| NortHingError::NotFound(format!("Session not found: {}", session_id)))?;

        // Record start time
        let start_time = std::time::Instant::now();

        let old_messages_len = runtime_messages.len();
        let turns = self
            .context_compressor
            .collect_turns_for_auto_compression(session_id, runtime_messages.clone())?;
        if turns.is_empty() {
            return Ok(None);
        }

        // Generate compression ID
        let compression_id = format!("compression_{}", uuid::Uuid::new_v4());

        // Emit compression started event
        self.emit_event(
            AgenticEvent::ContextCompressionStarted {
                session_id: session_id.to_string(),
                turn_id: dialog_turn_id.to_string(),
                compression_id: compression_id.clone(),
                trigger: "auto".to_string(),
                tokens_before: current_tokens,
                context_window,
                threshold: session.config.compression_threshold,
            },
            EventPriority::Normal,
        )
        .await;

        // Execute compression
        let compression_contract = self
            .session_manager
            .compression_contract_for_session(session_id, compression_contract_limit);
        let trace_config = prepare_model_exchange_trace_for_workspace(
            session_id,
            dialog_turn_id,
            workspace,
            ModelExchangeTraceOperation {
                kind: "context_compression",
                id: &compression_id,
                trigger: Some("auto"),
            },
            ai_client.as_ref(),
        )
        .await;
        let model_summary = match self
            .generate_compression_model_summary(
                ai_client,
                &runtime_messages,
                dialog_turn_id,
                workspace,
                tool_definitions,
                prepended_prompt_reminders,
                primary_supports_image_understanding,
                compression_contract.as_ref(),
                trace_config,
            )
            .await
        {
            Ok(summary) => summary,
            Err(err) => {
                tracing::warn!(
                    "Model-based compression failed, falling back to structured local compression: {}",
                    err
                );
                None
            }
        };
        match self.context_compressor.compress_turns_with_contract(
            session_id,
            context_window,
            turns,
            CompressionMode::Auto,
            compression_contract,
            model_summary,
        ) {
            Ok(compression_result) => {
                self.session_manager
                    .replace_context_messages(session_id, compression_result.messages.clone())
                    .await;
                if self
                    .session_manager
                    .rebuild_skill_agent_listing_baseline_to_latest(session_id)
                    .await
                {
                    tracing::debug!(
                        "Rebuilt skill-agent listing baseline after compression: session_id={}",
                        session_id
                    );
                }
                self.session_manager
                    .invalidate_prompt_cache(
                        session_id,
                        crate::agentic::session::PromptCacheScope::All,
                        "context_compression_applied",
                    )
                    .await;
                let mut new_messages = vec![system_prompt_message];
                new_messages.extend(compression_result.messages);
                // Update session compression state
                session.compression_state.increment_compression_count();

                // Update session state
                let _ = self
                    .session_manager
                    .update_compression_state(session_id, session.compression_state.clone())
                    .await;

                // Calculate duration
                let duration_ms = elapsed_ms_u64(start_time);

                // Recalculate tokens after compression
                let compressed_tokens =
                    Self::estimate_request_tokens_internal(&new_messages, tool_definitions.as_deref());
                let summary_source = if compression_result.has_model_summary {
                    "model"
                } else {
                    "local_fallback"
                };

                tracing::info!(
                    "Compression completed: session_id={}, turn_id={}, messages {} -> {}, tokens {} -> {}, compression_count={}, duration_ms={}, summary_source={}",
                    session_id,
                    dialog_turn_id,
                    old_messages_len,
                    new_messages.len(),
                    current_tokens,
                    compressed_tokens,
                    session.compression_state.compression_count,
                    duration_ms,
                    summary_source
                );

                // Emit compression completed event
                self.emit_event(
                    AgenticEvent::ContextCompressionCompleted {
                        session_id: session_id.to_string(),
                        turn_id: dialog_turn_id.to_string(),
                        compression_id: compression_id.clone(),
                        compression_count: session.compression_state.compression_count,
                        tokens_before: current_tokens,
                        tokens_after: compressed_tokens,
                        compression_ratio: (compressed_tokens as f64) / (current_tokens as f64),
                        duration_ms,
                        has_summary: compression_result.has_model_summary,
                        summary_source: summary_source.to_string(),
                    },
                    EventPriority::Normal,
                )
                .await;

                Ok(Some((compressed_tokens, new_messages)))
            }
            Err(e) => {
                // Emit compression failed event
                self.emit_event(
                    AgenticEvent::ContextCompressionFailed {
                        session_id: session_id.to_string(),
                        turn_id: dialog_turn_id.to_string(),
                        compression_id: compression_id.clone(),
                        error: e.to_string(),
                    },
                    EventPriority::High,
                )
                .await;

                Err(NortHingError::Session(e.to_string()))
            }
        }
    }

    pub(super) async fn compact_session_context_impl(
        &self,
        session_id: String,
        dialog_turn_id: String,
        context: ExecutionContext,
        messages: Vec<Message>,
        current_tokens: usize,
        trigger: &str,
    ) -> NortHingResult<ContextCompactionOutcome> {
        let mut session = self
            .session_manager
            .get_session(&session_id)
            .ok_or_else(|| NortHingError::NotFound(format!("Session not found: {}", session_id)))?;
        let start_time = std::time::Instant::now();
        let compression_id = format!("compression_{}", uuid::Uuid::new_v4());
        let scaffold = self.resolve_compression_runtime_scaffold(&session, &context).await?;
        let context_window = (scaffold.ai_client.config.context_window as usize).min(session.config.max_context_tokens);

        self.emit_event(
            AgenticEvent::ContextCompressionStarted {
                session_id: session_id.to_string(),
                turn_id: dialog_turn_id.to_string(),
                compression_id: compression_id.clone(),
                trigger: trigger.to_string(),
                tokens_before: current_tokens,
                context_window,
                threshold: session.config.compression_threshold,
            },
            EventPriority::Normal,
        )
        .await;

        let turns = self
            .context_compressor
            .collect_all_turns_for_manual_compaction(&session_id, messages.clone())?;

        if turns.is_empty() {
            let duration_ms = elapsed_ms_u64(start_time);
            let tokens_after = current_tokens;
            let compression_ratio = if current_tokens == 0 {
                1.0
            } else {
                (tokens_after as f64) / (current_tokens as f64)
            };

            self.emit_event(
                AgenticEvent::ContextCompressionCompleted {
                    session_id: session_id.to_string(),
                    turn_id: dialog_turn_id.to_string(),
                    compression_id: compression_id.clone(),
                    compression_count: session.compression_state.compression_count,
                    tokens_before: current_tokens,
                    tokens_after,
                    compression_ratio,
                    duration_ms,
                    has_summary: false,
                    summary_source: "none".to_string(),
                },
                EventPriority::Normal,
            )
            .await;

            return Ok(ContextCompactionOutcome {
                compression_id,
                compression_count: session.compression_state.compression_count,
                tokens_before: current_tokens,
                tokens_after,
                compression_ratio,
                duration_ms,
                has_summary: false,
                summary_source: "none".to_string(),
                applied: false,
            });
        }

        let mut runtime_messages = vec![scaffold.system_prompt_message.clone()];
        runtime_messages.extend(messages);
        let compression_contract = self
            .session_manager
            .compression_contract_for_session(&session_id, scaffold.compression_contract_limit);
        let trace_config = prepare_model_exchange_trace_for_workspace(
            &session_id,
            &dialog_turn_id,
            context.workspace.as_ref(),
            ModelExchangeTraceOperation {
                kind: "context_compression",
                id: &compression_id,
                trigger: Some(trigger),
            },
            scaffold.ai_client.as_ref(),
        )
        .await;
        let model_summary = match self
            .generate_compression_model_summary(
                scaffold.ai_client.clone(),
                &runtime_messages,
                &dialog_turn_id,
                context.workspace.as_ref(),
                &scaffold.tool_definitions,
                &scaffold.prepended_prompt_reminders,
                scaffold.primary_supports_image_understanding,
                compression_contract.as_ref(),
                trace_config,
            )
            .await
        {
            Ok(summary) => summary,
            Err(err) => {
                tracing::warn!(
                    "Model-based manual compaction failed, falling back to structured local compression: {}",
                    err
                );
                None
            }
        };
        match self.context_compressor.compress_turns_with_contract(
            &session_id,
            context_window,
            turns,
            CompressionMode::Manual,
            compression_contract,
            model_summary,
        ) {
            Ok(compression_result) => {
                let mut compressed_messages = compression_result.messages;
                self.session_manager
                    .replace_context_messages(&session_id, compressed_messages.clone())
                    .await;
                if self
                    .session_manager
                    .rebuild_skill_agent_listing_baseline_to_latest(&session_id)
                    .await
                {
                    tracing::debug!(
                        "Rebuilt skill-agent listing baseline after manual compaction: session_id={}",
                        session_id
                    );
                }
                self.session_manager
                    .invalidate_prompt_cache(
                        &session_id,
                        crate::agentic::session::PromptCacheScope::All,
                        "manual_context_compaction_applied",
                    )
                    .await;

                session.compression_state.increment_compression_count();
                let compression_count = session.compression_state.compression_count;
                let _ = self
                    .session_manager
                    .update_compression_state(&session_id, session.compression_state.clone())
                    .await;

                let duration_ms = elapsed_ms_u64(start_time);
                let tokens_after = compressed_messages
                    .iter_mut()
                    .map(|message| message.tokens())
                    .sum::<usize>();
                let compression_ratio = if current_tokens == 0 {
                    1.0
                } else {
                    (tokens_after as f64) / (current_tokens as f64)
                };

                self.emit_event(
                    AgenticEvent::ContextCompressionCompleted {
                        session_id: session_id.to_string(),
                        turn_id: dialog_turn_id.to_string(),
                        compression_id: compression_id.clone(),
                        compression_count,
                        tokens_before: current_tokens,
                        tokens_after,
                        compression_ratio,
                        duration_ms,
                        has_summary: compression_result.has_model_summary,
                        summary_source: if compression_result.has_model_summary {
                            "model".to_string()
                        } else {
                            "local_fallback".to_string()
                        },
                    },
                    EventPriority::Normal,
                )
                .await;

                Ok(ContextCompactionOutcome {
                    compression_id,
                    compression_count,
                    tokens_before: current_tokens,
                    tokens_after,
                    compression_ratio,
                    duration_ms,
                    has_summary: compression_result.has_model_summary,
                    summary_source: if compression_result.has_model_summary {
                        "model".to_string()
                    } else {
                        "local_fallback".to_string()
                    },
                    applied: true,
                })
            }
            Err(err) => {
                self.emit_event(
                    AgenticEvent::ContextCompressionFailed {
                        session_id: session_id.to_string(),
                        turn_id: dialog_turn_id.to_string(),
                        compression_id: compression_id.clone(),
                        error: err.to_string(),
                    },
                    EventPriority::High,
                )
                .await;

                Err(NortHingError::Session(err.to_string()))
            }
        }
    }
}
