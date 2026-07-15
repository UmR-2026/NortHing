//! `StreamProcessor`: drives a provider-neutral stream into a `StreamResult`.
//!
//! Wires together `StreamContext` (per-stream accumulator), the watchdog-aware
//! `next_stream_item` adapter, and an injected `StreamEventSink` to emit
//! `AgenticEvent`s. Handles text/thinking/tool chunks, partial recovery on
//! cancellation or token-limit truncation, and graceful shutdown that drains
//! pending tool calls.

use crate::sse_log_collector::SseLogCollector;
use crate::stream_context::{next_stream_item, StreamContext, TimedStreamItem};
use crate::tool_call_accumulator::{ToolCallBoundary, ToolCallStreamKey};
use crate::types::SseLogConfig;
use crate::types::{
    is_token_limit_finish_reason, StreamEventSink, StreamProcessError, StreamProcessOptions, StreamProcessorError,
    StreamResult, ToolCall,
};
use crate::unified::{UnifiedResponse, UnifiedTokenUsage, UnifiedToolCall};
use northhing_events::{AgenticEvent, AgenticEventPriority as EventPriority, ToolEventData};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, trace};

/// Stream processor
pub struct StreamProcessor {
    event_sink: Arc<dyn StreamEventSink>,
}

impl StreamProcessor {
    const WATCHDOG_GRACE_SECS: u64 = 2;

    pub fn new<E>(event_sink: Arc<E>) -> Self
    where
        E: StreamEventSink + 'static,
    {
        Self { event_sink }
    }

    pub fn derive_watchdog_timeout(stream_idle_timeout: Option<std::time::Duration>) -> Option<std::time::Duration> {
        stream_idle_timeout.map(|timeout| {
            timeout
                .checked_add(std::time::Duration::from_secs(Self::WATCHDOG_GRACE_SECS))
                .unwrap_or(std::time::Duration::MAX)
        })
    }

    fn merge_json_value(target: &mut Value, overlay: Value) {
        match (target, overlay) {
            (Value::Object(target_map), Value::Object(overlay_map)) => {
                for (key, value) in overlay_map {
                    let entry = target_map.entry(key).or_insert(Value::Null);
                    Self::merge_json_value(entry, value);
                }
            }
            (target_slot, overlay_value) => {
                *target_slot = overlay_value;
            }
        }
    }

    // ==================== Helper Methods ====================

    /// Send thinking end event (if needed)
    async fn send_thinking_end_if_needed(&self, ctx: &mut StreamContext) {
        if ctx.thinking_chunks_count > 0 && !ctx.thinking_completed_sent {
            ctx.thinking_completed_sent = true;
            debug!("Thinking process ended, sending ThinkingChunk end event");
            let _ = self
                .event_sink
                .enqueue(
                    AgenticEvent::ThinkingChunk {
                        session_id: ctx.session_id.clone(),
                        turn_id: ctx.dialog_turn_id.clone(),
                        round_id: ctx.round_id.clone(),
                        content: String::new(),
                        is_end: true,
                    },
                    Some(EventPriority::Normal),
                )
                .await;
        }
    }

    /// Check cancellation and execute graceful shutdown, returns Some(Err) if processing needs to be interrupted
    async fn check_cancellation(
        &self,
        ctx: &mut StreamContext,
        cancellation_token: &tokio_util::sync::CancellationToken,
        location: &str,
    ) -> Option<Result<StreamResult, StreamProcessError>> {
        if cancellation_token.is_cancelled() {
            debug!("Cancellation detected at {}: location={}", location, location);
            self.graceful_shutdown_from_ctx(ctx, "User cancelled stream processing".to_string())
                .await;
            Some(Err(StreamProcessError::new(
                StreamProcessorError::Cancelled("Stream processing cancelled".to_string()),
                ctx.has_effective_output,
            )))
        } else {
            None
        }
    }

    /// Execute graceful shutdown from context
    async fn graceful_shutdown_from_ctx(&self, ctx: &mut StreamContext, reason: String) {
        ctx.force_finish_pending_tool_calls();
        self.graceful_shutdown(
            ctx.session_id.clone(),
            ctx.dialog_turn_id.clone(),
            ctx.round_id.clone(),
            ctx.tool_calls.clone(),
            reason,
        )
        .await;
    }

    /// Graceful shutdown: cleanup all unfinished tool states and notify frontend
    async fn graceful_shutdown(
        &self,
        session_id: String,
        turn_id: String,
        round_id: String,
        tool_calls: Vec<ToolCall>,
        reason: String,
    ) {
        debug!(
            "Starting graceful shutdown: session_id={}, reason={}",
            session_id, reason
        );

        let is_user_cancellation = reason.contains("cancelled") || reason.contains("cancelled");
        let tool_call_count = tool_calls.len();

        // 1. Cleanup all tool calls
        for tool_call in tool_calls {
            trace!("Cleaning up tool: {} ({})", tool_call.tool_name, tool_call.tool_id);

            let tool_event = if is_user_cancellation {
                ToolEventData::Cancelled {
                    tool_id: tool_call.tool_id,
                    tool_name: tool_call.tool_name,
                    reason: reason.clone(),
                    duration_ms: None,
                    queue_wait_ms: None,
                    preflight_ms: None,
                    confirmation_wait_ms: None,
                    execution_ms: None,
                }
            } else {
                ToolEventData::Failed {
                    tool_id: tool_call.tool_id,
                    tool_name: tool_call.tool_name,
                    error: reason.clone(),
                    duration_ms: None,
                    queue_wait_ms: None,
                    preflight_ms: None,
                    confirmation_wait_ms: None,
                    execution_ms: None,
                }
            };

            let _ = self
                .event_sink
                .enqueue(
                    AgenticEvent::ToolEvent {
                        session_id: session_id.clone(),
                        turn_id: turn_id.clone(),
                        round_id: round_id.clone(),
                        tool_event,
                    },
                    Some(EventPriority::High),
                )
                .await;
        }

        // 2. Send dialog turn status update (if tools were cleaned up)
        if tool_call_count > 0 {
            let event = if is_user_cancellation {
                AgenticEvent::DialogTurnCancelled {
                    session_id: session_id.clone(),
                    turn_id: turn_id.clone(),
                }
            } else {
                AgenticEvent::DialogTurnFailed {
                    session_id: session_id.clone(),
                    turn_id: turn_id.clone(),
                    error: reason,
                    error_category: None,
                    error_detail: None,
                }
            };
            let _ = self.event_sink.enqueue(event, Some(EventPriority::Critical)).await;
        }

        debug!("Graceful shutdown completed: cleaned up {} tools", tool_call_count);
    }

    /// Handle usage statistics
    fn handle_usage(&self, ctx: &mut StreamContext, response_usage: &UnifiedTokenUsage) {
        ctx.usage = Some(response_usage.clone());
        debug!(
            "Received token usage stats: input={}, output={}, total={}",
            response_usage.prompt_token_count, response_usage.candidates_token_count, response_usage.total_token_count
        );
    }

    /// Handle tool call chunk
    async fn handle_tool_call_chunk(&self, ctx: &mut StreamContext, tool_call: UnifiedToolCall) {
        let UnifiedToolCall {
            tool_call_index,
            id,
            name,
            arguments,
            arguments_is_snapshot,
        } = tool_call;
        let outcome = ctx.pending_tool_calls.apply_delta(
            ToolCallStreamKey::from(tool_call_index),
            id,
            name,
            arguments,
            arguments_is_snapshot,
        );

        if let Some(finalized) = outcome.finalized_previous {
            ctx.record_finalized_tool_call(&finalized);
        }

        if let Some(early_detected) = outcome.early_detected {
            ctx.has_effective_output = true;
            ctx.mark_first_visible_output();
            debug!("Tool detected: {}", early_detected.tool_name);
            let _ = self
                .event_sink
                .enqueue(
                    AgenticEvent::ToolEvent {
                        session_id: ctx.session_id.clone(),
                        turn_id: ctx.dialog_turn_id.clone(),
                        round_id: ctx.round_id.clone(),
                        tool_event: ToolEventData::EarlyDetected {
                            tool_id: early_detected.tool_id,
                            tool_name: early_detected.tool_name,
                        },
                    },
                    None,
                )
                .await;
        }

        if let Some(params_partial) = outcome.params_partial {
            ctx.has_effective_output = true;
            ctx.mark_first_visible_output();
            let _ = self
                .event_sink
                .enqueue(
                    AgenticEvent::ToolEvent {
                        session_id: ctx.session_id.clone(),
                        turn_id: ctx.dialog_turn_id.clone(),
                        round_id: ctx.round_id.clone(),
                        tool_event: ToolEventData::ParamsPartial {
                            tool_id: params_partial.tool_id,
                            tool_name: params_partial.tool_name,
                            params: params_partial.params_chunk,
                        },
                    },
                    None,
                )
                .await;
        }
    }

    /// Handle text chunk
    async fn handle_text_chunk(&self, ctx: &mut StreamContext, text: String) {
        if !text.trim().is_empty() {
            ctx.has_effective_output = true;
            ctx.mark_first_visible_output();
        }
        ctx.full_text.push_str(&text);
        ctx.text_chunks_count += 1;

        // Send streaming text event
        let _ = self
            .event_sink
            .enqueue(
                AgenticEvent::TextChunk {
                    session_id: ctx.session_id.clone(),
                    turn_id: ctx.dialog_turn_id.clone(),
                    round_id: ctx.round_id.clone(),
                    text,
                },
                None,
            )
            .await;
    }

    /// Handle thinking chunk
    async fn handle_thinking_chunk(&self, ctx: &mut StreamContext, thinking_content: String) {
        // Thinking-only output does NOT count as "effective" for retry purposes:
        // if the stream fails after producing only thinking (no text/tool calls),
        // it is safe to retry because the model will re-think from scratch.
        ctx.full_thinking.push_str(&thinking_content);
        ctx.mark_first_visible_output();
        ctx.thinking_chunks_count += 1;

        // Send thinking chunk event
        let _ = self
            .event_sink
            .enqueue(
                AgenticEvent::ThinkingChunk {
                    session_id: ctx.session_id.clone(),
                    turn_id: ctx.dialog_turn_id.clone(),
                    round_id: ctx.round_id.clone(),
                    content: thinking_content,
                    is_end: false,
                },
                None,
            )
            .await;
    }

    /// Print stream processing end log
    fn log_stream_result(&self, ctx: &StreamContext) {
        debug!(
            "Stream loop ended: text_chunks={}, thinking_chunks={}, tool_calls({}), first_chunk_ms={:?}, first_visible_output_ms={:?}: {}",
            ctx.text_chunks_count,
            ctx.thinking_chunks_count,
            ctx.tool_calls.len(),
            ctx.first_chunk_ms,
            ctx.first_visible_output_ms,
            ctx.tool_calls
                .iter()
                .map(|tc| tc.tool_name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );

        if tracing::level_enabled!(tracing::Level::DEBUG) {
            if !ctx.full_thinking.is_empty() {
                debug!(target: "ai::stream_processor", "Full thinking content: \n{}", ctx.full_thinking);
            }
            if !ctx.full_text.is_empty() {
                debug!(target: "ai::stream_processor", "Full text content: \n{}", ctx.full_text);
            }
            if !ctx.tool_calls.is_empty() {
                let log_str: String = ctx
                    .tool_calls
                    .iter()
                    .map(|tc| {
                        format!(
                            "Tool name: {}, arguments: {}\n",
                            tc.tool_name,
                            serde_json::to_string(&tc.arguments).unwrap_or_else(|_| "Serialization failed".to_string())
                        )
                    })
                    .collect();
                debug!(target: "ai::stream_processor", "Tool call details: \n{}", log_str);
            }
        }

        trace!(
            "Returning StreamResult: thinking_len={}, text_len={}, tool_calls={}, has_usage={}, has_effective_output={}",
            ctx.full_thinking.len(),
            ctx.full_text.len(),
            ctx.tool_calls.len(),
            ctx.usage.is_some(),
            ctx.has_effective_output
        );
    }

    // ==================== Main Processing Methods ====================

    /// Process AI streaming response
    ///
    /// # Arguments
    /// * `stream` - Parsed response stream
    /// * `raw_sse_rx` - Optional raw SSE data receiver (for collecting raw data during error diagnosis)
    /// * `session_id` - Session ID
    /// * `dialog_turn_id` - Dialog turn ID
    /// * `round_id` - Model round ID
    /// * `cancellation_token` - Cancellation token
    #[allow(clippy::too_many_arguments)]
    pub async fn process_stream(
        &self,
        stream: futures::stream::BoxStream<'static, Result<UnifiedResponse, anyhow::Error>>,
        watchdog_timeout: Option<std::time::Duration>,
        raw_sse_rx: Option<mpsc::UnboundedReceiver<String>>,
        session_id: String,
        dialog_turn_id: String,
        round_id: String,
        cancellation_token: &tokio_util::sync::CancellationToken,
    ) -> Result<StreamResult, StreamProcessError> {
        self.process_stream_with_options(
            stream,
            watchdog_timeout,
            raw_sse_rx,
            session_id,
            dialog_turn_id,
            round_id,
            cancellation_token,
            StreamProcessOptions::default(),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn process_stream_with_options(
        &self,
        mut stream: futures::stream::BoxStream<'static, Result<UnifiedResponse, anyhow::Error>>,
        watchdog_timeout: Option<std::time::Duration>,
        raw_sse_rx: Option<mpsc::UnboundedReceiver<String>>,
        session_id: String,
        dialog_turn_id: String,
        round_id: String,
        cancellation_token: &tokio_util::sync::CancellationToken,
        options: StreamProcessOptions,
    ) -> Result<StreamResult, StreamProcessError> {
        let mut ctx = StreamContext::new(session_id, dialog_turn_id, round_id, options);
        // Start SSE log collector (if raw_sse_rx is provided)
        let sse_collector = if let Some(mut rx) = raw_sse_rx {
            let collector = Arc::new(tokio::sync::Mutex::new(SseLogCollector::new(
                SseLogConfig::default(), // No limit for now
            )));
            let collector_clone = collector.clone();

            // Start background task to collect SSE data
            tokio::spawn(async move {
                while let Some(data) = rx.recv().await {
                    collector_clone.lock().await.push(data);
                }
            });

            Some(collector)
        } else {
            None
        };

        // Define a helper closure to flush SSE logs on error
        let flush_sse_on_error = |collector: &Option<Arc<tokio::sync::Mutex<SseLogCollector>>>, error_context: &str| {
            let collector = collector.clone();
            let error_context = error_context.to_string();
            async move {
                if let Some(c) = collector {
                    // Wait a short time for background task to finish collecting data
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    c.lock().await.flush_on_error(&error_context);
                }
            }
        };

        loop {
            tokio::select! {
                // Check cancellation token
                _ = cancellation_token.cancelled() => {
                    debug!("Cancel token detected, stopping stream processing: session_id={}", ctx.session_id);
                    if options.recover_partial_on_cancel && ctx.can_recover_as_partial_result() {
                        self.send_thinking_end_if_needed(&mut ctx).await;
                        ctx.force_finish_pending_tool_calls();
                        ctx.partial_recovery_reason =
                            Some("Stream processing cancelled after partial output".to_string());
                        self.log_stream_result(&ctx);
                        break;
                    }
                    self.graceful_shutdown_from_ctx(&mut ctx, "User cancelled stream processing".to_string()).await;
                    return Err(StreamProcessError::new(
                        StreamProcessorError::Cancelled("Stream processing cancelled".to_string()),
                        ctx.has_effective_output,
                    ));
                }

                // Watch the adapter -> processor stream only when the upstream stream idle timeout is configured.
                next_result = next_stream_item(&mut stream, watchdog_timeout) => {
                    let response = match next_result {
                        TimedStreamItem::Item(Ok(response)) => response,
                        TimedStreamItem::End => {
                            debug!("Stream ended normally (no more data)");
                            break;
                        }
                        TimedStreamItem::Item(Err(e)) => {
                            let error_msg = format!("Stream processing error: {}", e);
                            error!("{}", error_msg);
                            let non_recoverable_stream_error =
                                error_msg.contains("SSE Parsing Error");
                            if !non_recoverable_stream_error && ctx.can_recover_as_partial_result()
                            {
                                flush_sse_on_error(&sse_collector, &error_msg).await;
                                self.send_thinking_end_if_needed(&mut ctx).await;
                                ctx.force_finish_pending_tool_calls();
                                ctx.partial_recovery_reason = Some(error_msg.clone());
                                self.log_stream_result(&ctx);
                                break;
                            }
                            // log SSE for network errors
                            flush_sse_on_error(&sse_collector, &error_msg).await;
                            self.graceful_shutdown_from_ctx(&mut ctx, error_msg.clone()).await;
                            return Err(StreamProcessError::new(
                                StreamProcessorError::AiClient(error_msg),
                                ctx.has_effective_output,
                            ));
                        }
                        TimedStreamItem::TimedOut => {
                            let timeout_secs =
                                watchdog_timeout.map(|timeout| timeout.as_secs()).unwrap_or(0);
                            let error_msg = format!(
                                "Stream processor watchdog timeout (no data received for {} seconds)",
                                timeout_secs
                            );
                            error!(
                                "Stream processor watchdog timeout ({} seconds), forcing termination",
                                timeout_secs
                            );
                            // log SSE for timeout errors
                            flush_sse_on_error(&sse_collector, &error_msg).await;
                            if ctx.can_recover_as_partial_result() {
                                self.send_thinking_end_if_needed(&mut ctx).await;
                                ctx.force_finish_pending_tool_calls();
                                ctx.partial_recovery_reason = Some(error_msg.clone());
                                self.log_stream_result(&ctx);
                                break;
                            }
                            self.graceful_shutdown_from_ctx(&mut ctx, error_msg.clone()).await;
                            return Err(StreamProcessError::new(
                                StreamProcessorError::AiClient(error_msg),
                                ctx.has_effective_output,
                            ));
                        }
                    };

                    let UnifiedResponse {
                        text,
                        reasoning_content,
                        thinking_signature,
                        tool_call,
                        usage,
                        finish_reason,
                        provider_metadata,
                    } = response;
                    ctx.mark_first_stream_chunk();

                    // Handle thinking_signature
                    if let Some(signature) = thinking_signature {
                        if !signature.is_empty() {
                            ctx.reasoning_content_present = true;
                            ctx.thinking_signature = Some(signature);
                            trace!("Received thinking_signature");
                        }
                    }

                    // Handle different types of response content
                    // Normalize empty strings to None
                    //  (some models send empty text alongside reasoning content)
                    let text = text.filter(|t| !t.is_empty());

                    if let Some(thinking_content) = reasoning_content {
                        ctx.reasoning_content_present = true;
                        if !thinking_content.is_empty() {
                            self.handle_thinking_chunk(&mut ctx, thinking_content).await;
                            if let Some(err) = self.check_cancellation(&mut ctx, cancellation_token, "processing thinking chunk").await {
                                return err;
                            }
                        }
                    }

                    if let Some(text) = text {
                        self.send_thinking_end_if_needed(&mut ctx).await;
                        self.handle_text_chunk(&mut ctx, text).await;
                        if let Some(err) = self.check_cancellation(&mut ctx, cancellation_token, "processing text chunk").await {
                            return err;
                        }
                    }

                    if let Some(tool_call) = tool_call {
                        self.send_thinking_end_if_needed(&mut ctx).await;
                        self.handle_tool_call_chunk(&mut ctx, tool_call).await;
                        if let Some(err) = self.check_cancellation(&mut ctx, cancellation_token, "processing tool call").await {
                            return err;
                        }
                    }

                    if let Some(ref response_usage) = usage {
                        self.handle_usage(&mut ctx, response_usage);
                    }

                    if let Some(provider_metadata) = provider_metadata {
                        match ctx.provider_metadata.as_mut() {
                            Some(existing) => Self::merge_json_value(existing, provider_metadata),
                            None => ctx.provider_metadata = Some(provider_metadata),
                        }
                    }

                    if let Some(reason) = finish_reason {
                        let _ = ctx.finalize_all_pending_tool_calls(ToolCallBoundary::FinishReason);
                        if is_token_limit_finish_reason(&reason) {
                            ctx.token_limit_finish_reason = Some(reason);
                        }
                    }
                }
            }
        }

        // Ensure thinking end marker is sent
        self.send_thinking_end_if_needed(&mut ctx).await;

        let _ = ctx.finalize_all_pending_tool_calls(ToolCallBoundary::StreamEnd);

        // A token-limit finish_reason means the provider ended the stream
        // gracefully but the answer is silently truncated. Surface it as a
        // partial recovery so downstream execution can continue the answer in
        // a follow-up round instead of accepting cut-off output as final.
        // Tool-call rounds are excluded: they already continue via the normal
        // round loop, and truncated tool arguments have their own repair path.
        if ctx.partial_recovery_reason.is_none() && ctx.tool_calls.is_empty() && !ctx.full_text.is_empty() {
            if let Some(reason) = ctx.token_limit_finish_reason.take() {
                ctx.partial_recovery_reason = Some(format!(
                    "response truncated by model output token limit (finish_reason={})",
                    reason
                ));
            }
        }

        // Invalid tool payloads that survive to finalization still need detailed SSE logs for diagnosis.
        if ctx.tool_calls.iter().any(|tc| !tc.is_valid()) {
            flush_sse_on_error(&sse_collector, "Has invalid tool calls").await;
        }

        self.log_stream_result(&ctx);

        Ok(ctx.into_result())
    }
}
