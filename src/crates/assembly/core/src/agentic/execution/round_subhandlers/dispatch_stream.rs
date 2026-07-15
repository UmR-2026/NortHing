//! `RoundExecutor::dispatch_stream` sub-handler.
//!
//! Runs the stream attempt loop with retry policy. Returns `DispatchOutcome`
//! on success or propagates `Err`. Retries on transient network errors up to
//! `state.max_attempts` times with exponential backoff.

use super::super::round_executor::RoundExecutor;
use super::super::stream_processor::{StreamProcessOptions, StreamProcessor};
use super::round_state::{DispatchOutcome, RoundState};
use crate::util::elapsed_ms_u64;
use crate::util::errors::{NortHingError, NortHingResult};
use std::time::Instant;
use tracing::{debug, error, warn};

impl RoundExecutor {
    /// Run the stream attempt loop with retry policy. Returns DispatchOutcome on
    /// success or propagates Err.
    pub(crate) async fn dispatch_stream(&self, state: &mut RoundState) -> NortHingResult<DispatchOutcome> {
        let outcome = loop {
            // Check cancellation before opening a model stream. This catches
            // early cancellation registered before the first round starts.
            if state.cancel_token.is_cancelled() {
                debug!(
                    "Cancel token detected before AI request, stopping execution: session_id={}",
                    state.context.session_id
                );
                return Err(NortHingError::Cancelled("Execution cancelled".to_string()));
            }

            let request_started_at = Instant::now();
            debug!(
                "Sending request: model={}, messages={}, tools={}, attempt={}/{}",
                state.context.model_name,
                state.ai_messages.len(),
                state.tool_definitions.as_ref().map(|t| t.len()).unwrap_or(0),
                state.attempt_index + 1,
                state.max_attempts
            );
            // Use dynamically obtained client for call
            let send_future = state.ai_client.send_message_stream(
                state.ai_messages.clone(),
                state.tool_definitions.clone(),
                state.trace_config.clone(),
            );
            let send_result = tokio::select! {
                _ = state.cancel_token.cancelled() => {
                    return Err(NortHingError::Cancelled("Execution cancelled".to_string()));
                }
                result = send_future => result,
            };
            let (stream_response, send_to_stream_ms) = match send_result {
                Ok(response) => {
                    let send_to_stream_ms = elapsed_ms_u64(request_started_at);
                    debug!(
                        "AI stream opened: session_id={}, state.round_id={}, attempt={}/{}, send_to_stream_ms={}",
                        state.context.session_id,
                        state.round_id,
                        state.attempt_index + 1,
                        state.max_attempts,
                        send_to_stream_ms
                    );
                    (response, send_to_stream_ms)
                }
                Err(e) => {
                    error!("AI request failed: {}", e);
                    let err_msg = e.to_string();
                    if Self::is_transient_network_error(&err_msg) && state.attempt_index < state.max_attempts - 1 {
                        let delay_ms = Self::retry_delay_ms(state.attempt_index);
                        warn!(
                            "Retrying AI request after connection failure: session_id={}, state.round_id={}, attempt={}/{}, delay_ms={}, error={}",
                            state.context.session_id,
                            state.round_id,
                            state.attempt_index + 1,
                            state.max_attempts,
                            delay_ms,
                            err_msg
                        );
                        Self::sleep_with_cancellation(delay_ms, &state.cancel_token).await?;
                        state.attempt_index += 1;
                        continue;
                    }
                    if Self::is_transient_network_error(&err_msg) {
                        return Err(NortHingError::AIClient(format!(
                            "Stream retry budget exhausted after {} attempts: {}",
                            state.max_attempts, err_msg
                        )));
                    }
                    // Non-transient errors (429 budget exhausted, state.context
                    // overflow, auth, etc.) are returned directly. The error
                    // message is classified downstream via
                    // `NortHingError::error_category()` into `ErrorCategory` for
                    // frontend recovery actions (wait_and_retry, switch_model,
                    // etc.).
                    let error = NortHingError::AIClient(err_msg);
                    warn!(
                        "AI request terminal failure: session_id={}, state.round_id={}, category={:?}, error={}",
                        state.context.session_id,
                        state.round_id,
                        error.error_category(),
                        error
                    );
                    return Err(error);
                }
            };

            // Destructure StreamResponse: get stream and raw SSE data receiver
            let ai_stream = stream_response.stream;
            let raw_sse_rx = stream_response.raw_sse_rx;
            let trace_handle = stream_response.trace_handle;

            // Check cancellation token before calling stream processing.
            if state.cancel_token.is_cancelled() {
                Self::complete_model_exchange_trace(
                    state.trace_config.as_ref(),
                    trace_handle.as_ref(),
                    Self::error_trace_response("cancelled", "Execution cancelled".to_string()),
                )
                .await;
                debug!(
                    "Cancel token detected after AI stream opened, stopping execution: session_id={}",
                    state.context.session_id
                );
                return Err(NortHingError::Cancelled("Execution cancelled".to_string()));
            }

            debug!(
                "Starting AI stream processing: session={}, round={}, thread={:?}, attempt={}/{}",
                state.context.session_id,
                state.round_id,
                std::thread::current().id(),
                state.attempt_index + 1,
                state.max_attempts
            );

            let stream_started_at = Instant::now();
            match self
                .stream_processor
                .process_stream_with_options(
                    ai_stream,
                    StreamProcessor::derive_watchdog_timeout(state.ai_client.stream_idle_timeout()),
                    raw_sse_rx, // Pass raw SSE data receiver (for error diagnosis)
                    state.context.session_id.clone(),
                    state.context.dialog_turn_id.clone(),
                    state.round_id.clone(),
                    &state.cancel_token,
                    StreamProcessOptions {
                        recover_partial_on_cancel: state.context.recover_partial_on_cancel,
                    },
                )
                .await
            {
                Ok(result) => {
                    let stream_processing_ms = elapsed_ms_u64(stream_started_at);
                    if Self::has_interrupted_invalid_tool_calls(&result) {
                        let err_msg = result
                            .partial_recovery_reason
                            .clone()
                            .unwrap_or_else(|| "Interrupted while streaming tool arguments".to_string());

                        if !Self::has_user_visible_assistant_text(&result.full_text)
                            && state.attempt_index < state.max_attempts - 1
                            && Self::is_transient_network_error(&err_msg)
                        {
                            Self::complete_model_exchange_trace(
                                state.trace_config.as_ref(),
                                trace_handle.as_ref(),
                                Self::trace_response_from_stream_result("partial", &result),
                            )
                            .await;
                            let delay_ms = Self::retry_delay_ms(state.attempt_index);
                            warn!(
                                "Retrying stream because tool arguments were interrupted before valid JSON completed: session_id={}, state.round_id={}, attempt={}/{}, delay_ms={}, invalid_tool_calls={}, error={}",
                                state.context.session_id,
                                state.round_id,
                                state.attempt_index + 1,
                                state.max_attempts,
                                delay_ms,
                                result
                                    .tool_calls
                                    .iter()
                                    .filter(|tool_call| !tool_call.is_valid())
                                    .count(),
                                err_msg
                            );
                            Self::sleep_with_cancellation(delay_ms, &state.cancel_token).await?;
                            state.attempt_index += 1;
                            continue;
                        }

                        if Self::has_user_visible_assistant_text(&result.full_text) {
                            warn!(
                                "Dropping invalid partial tool calls from interrupted stream; preserving already-streamed assistant text: session_id={}, state.round_id={}, invalid_tool_calls={}, error={}",
                                state.context.session_id,
                                state.round_id,
                                result
                                    .tool_calls
                                    .iter()
                                    .filter(|tool_call| !tool_call.is_valid())
                                    .count(),
                                err_msg
                            );
                            self.emit_failed_partial_tool_calls(
                                &state.context,
                                &state.round_id,
                                &result.tool_calls,
                                &err_msg,
                            )
                            .await;
                            let mut recovered = result;
                            recovered.tool_calls.retain(|tool_call| tool_call.is_valid());
                            break DispatchOutcome {
                                stream_result: recovered,
                                send_to_stream_ms,
                                stream_processing_ms,
                                trace_handle,
                            };
                        }

                        self.emit_failed_partial_tool_calls(
                            &state.context,
                            &state.round_id,
                            &result.tool_calls,
                            &err_msg,
                        )
                        .await;
                        Self::complete_model_exchange_trace(
                            state.trace_config.as_ref(),
                            trace_handle.as_ref(),
                            Self::error_trace_response_from_stream_result("error", err_msg.clone(), &result),
                        )
                        .await;
                        return Err(NortHingError::AIClient(format!(
                            "Stream retry budget exhausted after {} attempts: {}",
                            state.max_attempts, err_msg
                        )));
                    }

                    let no_effective_output = !result.has_effective_output;
                    let is_partial_recovery = result.partial_recovery_reason.is_some();
                    let partial_recovery_reason = result.partial_recovery_reason.as_deref().unwrap_or("");

                    if is_partial_recovery
                        && !Self::has_user_visible_assistant_text(&result.full_text)
                        && !result.tool_calls.is_empty()
                        && Self::is_transient_network_error(partial_recovery_reason)
                        && state.attempt_index < state.max_attempts - 1
                    {
                        Self::complete_model_exchange_trace(
                            state.trace_config.as_ref(),
                            trace_handle.as_ref(),
                            Self::trace_response_from_stream_result("partial", &result),
                        )
                        .await;
                        let delay_ms = Self::retry_delay_ms(state.attempt_index);
                        warn!(
                            "Retrying stream because tool calls arrived on an interrupted network stream without assistant text: session_id={}, state.round_id={}, attempt={}/{}, delay_ms={}, tool_calls={}, reason={}",
                            state.context.session_id,
                            state.round_id,
                            state.attempt_index + 1,
                            state.max_attempts,
                            delay_ms,
                            result.tool_calls.len(),
                            partial_recovery_reason
                        );
                        Self::sleep_with_cancellation(delay_ms, &state.cancel_token).await?;
                        state.attempt_index += 1;
                        continue;
                    }

                    if Self::is_invalid_tool_only_without_text(&result) {
                        let err_msg = "Provider returned only invalid tool arguments".to_string();
                        if state.attempt_index < state.max_attempts - 1 {
                            Self::complete_model_exchange_trace(
                                state.trace_config.as_ref(),
                                trace_handle.as_ref(),
                                Self::error_trace_response_from_stream_result("error", err_msg.clone(), &result),
                            )
                            .await;
                            let delay_ms = Self::retry_delay_ms(state.attempt_index);
                            warn!(
                                "Retrying stream because provider returned only invalid tool arguments: session_id={}, state.round_id={}, attempt={}/{}, delay_ms={}, tool_calls={}",
                                state.context.session_id,
                                state.round_id,
                                state.attempt_index + 1,
                                state.max_attempts,
                                delay_ms,
                                result.tool_calls.len()
                            );
                            Self::sleep_with_cancellation(delay_ms, &state.cancel_token).await?;
                            state.attempt_index += 1;
                            continue;
                        }

                        self.emit_failed_partial_tool_calls(
                            &state.context,
                            &state.round_id,
                            &result.tool_calls,
                            &err_msg,
                        )
                        .await;
                        Self::complete_model_exchange_trace(
                            state.trace_config.as_ref(),
                            trace_handle.as_ref(),
                            Self::error_trace_response_from_stream_result("error", err_msg.clone(), &result),
                        )
                        .await;
                        return Err(NortHingError::AIClient(format!(
                            "Stream retry budget exhausted after {} attempts: {}",
                            state.max_attempts, err_msg
                        )));
                    }

                    if no_effective_output && state.attempt_index < state.max_attempts - 1 {
                        Self::complete_model_exchange_trace(
                            state.trace_config.as_ref(),
                            trace_handle.as_ref(),
                            Self::error_trace_response("error", "No effective output received".to_string()),
                        )
                        .await;
                        let delay_ms = Self::retry_delay_ms(state.attempt_index);
                        warn!(
                            "Retrying stream because no effective output was received: session_id={}, state.round_id={}, attempt={}/{}, delay_ms={}",
                            state.context.session_id,
                            state.round_id,
                            state.attempt_index + 1,
                            state.max_attempts,
                            delay_ms
                        );
                        Self::sleep_with_cancellation(delay_ms, &state.cancel_token).await?;
                        state.attempt_index += 1;
                        continue;
                    }

                    if is_partial_recovery {
                        warn!(
                            "Accepting stream partial recovery without retry: session_id={}, state.round_id={}, attempt={}/{}, reason={}",
                            state.context.session_id,
                            state.round_id,
                            state.attempt_index + 1,
                            state.max_attempts,
                            result
                                .partial_recovery_reason
                                .as_deref()
                                .unwrap_or("unknown")
                        );
                    }

                    break DispatchOutcome {
                        stream_result: result,
                        send_to_stream_ms,
                        stream_processing_ms,
                        trace_handle,
                    };
                }
                Err(stream_err) => {
                    let err_msg = stream_err.error.to_string();
                    let can_retry = !stream_err.has_effective_output
                        && state.attempt_index < state.max_attempts - 1
                        && Self::is_transient_network_error(&err_msg);
                    Self::complete_model_exchange_trace(
                        state.trace_config.as_ref(),
                        trace_handle.as_ref(),
                        Self::error_trace_response("error", err_msg.clone()),
                    )
                    .await;
                    if can_retry {
                        let delay_ms = Self::retry_delay_ms(state.attempt_index);
                        warn!(
                            "Retrying stream after transient error with no effective output: session_id={}, state.round_id={}, attempt={}/{}, delay_ms={}, error={}",
                            state.context.session_id,
                            state.round_id,
                            state.attempt_index + 1,
                            state.max_attempts,
                            delay_ms,
                            err_msg
                        );
                        Self::sleep_with_cancellation(delay_ms, &state.cancel_token).await?;
                        state.attempt_index += 1;
                        continue;
                    }
                    if Self::is_transient_network_error(&err_msg) {
                        return Err(NortHingError::AIClient(format!(
                            "Stream retry budget exhausted after {} attempts: {}",
                            state.max_attempts, err_msg
                        )));
                    }
                    return Err(stream_err.error);
                }
            }
        };
        Ok(outcome)
    }
}
