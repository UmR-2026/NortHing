use super::pipeline_logging::*;
use super::pipeline_post::*;
use super::pipeline_pre::*;
use super::pipeline_types::*;
use crate::agentic::core::ToolCall;
use crate::agentic::core::ToolExecutionState;
use crate::agentic::core::ToolResult as ModelToolResult;
use crate::agentic::events::types::ToolEventData;
use crate::agentic::tools::computer_use_host::ComputerUseHostRef;
use crate::agentic::tools::framework::ToolResult as FrameworkToolResult;
use crate::agentic::tools::pipeline::types::*;
use crate::agentic::tools::registry::ToolRegistry;
use crate::agentic::tools::tool_context_runtime::ToolUseContext;
use crate::agentic::tools::tool_result_storage;
use crate::util::elapsed_ms_u64;
use crate::util::errors::{NortHingError, NortHingResult};
use dashmap::DashMap;
use northhing_agent_runtime::tool_confirmation::{
    resolve_confirmation_failure, resolve_confirmation_wait_result, resolve_tool_confirmation_plan,
    ConfirmationFailureKind, ToolConfirmationPlan, ToolConfirmationRequestFacts, ToolConfirmationWaitResult,
};
use northhing_agent_tools::{
    build_invalid_tool_call_error_message, build_tool_call_truncation_recovery_notice,
    build_tool_execution_error_presentation, build_user_steering_interrupted_presentation,
    render_tool_result_for_assistant, truncate_raw_tool_arguments_preview, truncate_tool_arguments_preview,
    validate_tool_execution_admission, ToolExecutionAdmissionRejection, ToolExecutionAdmissionRequest,
    GET_TOOL_SPEC_TOOL_NAME, USER_STEERING_INTERRUPTED_MESSAGE,
};
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use tokio::sync::{oneshot, RwLock as TokioRwLock};
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;
use tool_runtime::pipeline::{
    retry_delay_ms, should_cancel_tool_state, should_retry_tool_attempt, summarize_dialog_turn_cancellation,
    ToolExecutionErrorClass, ToolRetryAttemptFacts,
};
use tracing::{debug, error, info, warn};

impl ToolPipeline {
    /// Execute single tool
    pub(crate) async fn execute_single_tool(&self, tool_id: String) -> NortHingResult<ToolExecutionResult> {
        let start_time = Instant::now();

        debug!("Starting tool execution: tool_id={}", tool_id);

        // Get task
        let task = self
            .state_manager
            .get_task(&tool_id)
            .ok_or_else(|| NortHingError::NotFound(format!("Tool task not found: {}", tool_id)))?;

        let tool_name = task.tool_call.tool_name.clone();
        let tool_args = task.tool_call.arguments.clone();
        let tool_is_error = task.tool_call.is_error;
        let recovered_from_truncation = task.tool_call.recovered_from_truncation;
        let queue_wait_ms = elapsed_ms_since(task.created_at);
        let mut confirmation_wait_ms = 0;

        debug!(
            "Tool task details: tool_name={}, tool_id={}, queue_wait_ms={}",
            tool_name, tool_id, queue_wait_ms
        );

        if recovered_from_truncation {
            warn!(
                "Tool '{}' arguments were recovered from a truncated stream (tool_id={}, session_id={}). Executing with patched arguments — content may be incomplete.",
                tool_name, tool_id, task.context.session_id
            );
        }

        if tool_name.is_empty() || tool_is_error {
            let raw_arguments_preview = task
                .tool_call
                .raw_arguments
                .as_deref()
                .map(truncate_raw_tool_arguments_preview);
            let error_msg = build_invalid_tool_call_error_message(
                &tool_name,
                tool_is_error,
                recovered_from_truncation,
                raw_arguments_preview,
            );

            self.state_manager
                .update_state(
                    &tool_id,
                    ToolExecutionState::Failed {
                        error: error_msg.clone(),
                        is_retryable: false,
                        duration_ms: None,
                        queue_wait_ms: None,
                        preflight_ms: None,
                        confirmation_wait_ms: None,
                        execution_ms: None,
                    },
                )
                .await;

            return Err(NortHingError::Validation(error_msg));
        }

        // Repetition alone is not execution failure: polling and status checks
        // may legitimately reuse identical arguments. The execution engine
        // evaluates repeated patterns only after observing actual tool results.
        if let Err(err) = validate_task_admission(&task) {
            let error_msg = err.to_string();
            warn!("Tool execution admission rejected: {}", error_msg);

            self.state_manager
                .update_state(
                    &tool_id,
                    ToolExecutionState::Failed {
                        error: error_msg.clone(),
                        is_retryable: false,
                        duration_ms: None,
                        queue_wait_ms: None,
                        preflight_ms: None,
                        confirmation_wait_ms: None,
                        execution_ms: None,
                    },
                )
                .await;

            return Err(err);
        }

        let tool = {
            let registry = self.tool_registry.read().await;
            registry.get_tool(&task.tool_call.tool_name).ok_or_else(|| {
                let error_msg = format!("Tool '{}' is not registered or enabled.", task.tool_call.tool_name,);
                error!("{}", error_msg);
                NortHingError::tool(error_msg)
            })?
        };

        let cancellation_token = CancellationToken::new();
        let tool_context = self.build_tool_use_context(&task, cancellation_token.clone());
        let validation = tool.validate_input(&tool_args, Some(&tool_context)).await;
        if !validation.result {
            let error_msg = validation
                .message
                .unwrap_or_else(|| format!("Invalid input for tool '{}'", tool_name));
            self.state_manager
                .update_state(
                    &tool_id,
                    ToolExecutionState::Failed {
                        error: error_msg.clone(),
                        is_retryable: false,
                        duration_ms: None,
                        queue_wait_ms: None,
                        preflight_ms: None,
                        confirmation_wait_ms: None,
                        execution_ms: None,
                    },
                )
                .await;
            return Err(NortHingError::Validation(error_msg));
        }
        if let Some(message) = validation.message.filter(|message| !message.trim().is_empty()) {
            warn!(
                "Tool input validation warning: tool_name={}, warning={}",
                tool_name, message
            );
        }

        // Register cancellation only after deterministic validation and registry lookup succeed.
        self.cancellation_tokens
            .insert(tool_id.clone(), cancellation_token.clone());

        debug!("Executing tool: tool_name={}", tool_name);

        let is_streaming = tool.supports_streaming();
        let preflight_ms = elapsed_ms_u64(start_time);

        let confirmation_plan = resolve_tool_confirmation_plan(ToolConfirmationRequestFacts {
            confirm_before_run: task.options.confirm_before_run,
            tool_needs_permission: tool.needs_permissions(Some(&tool_args)),
            confirmation_timeout_secs: task.options.confirmation_timeout_secs,
            now: SystemTime::now(),
        });

        if let ToolConfirmationPlan::Await {
            timeout_at,
            timeout_secs,
        } = confirmation_plan
        {
            info!("Tool requires confirmation: tool_name={}", tool_name);

            let (tx, rx) = oneshot::channel::<ConfirmationResponse>();

            self.confirmation_channels.insert(tool_id.clone(), tx);

            self.state_manager
                .update_state(
                    &tool_id,
                    ToolExecutionState::AwaitingConfirmation {
                        params: tool_args.clone(),
                        timeout_at,
                    },
                )
                .await;

            debug!("Waiting for confirmation: tool_name={}", tool_name);
            let confirmation_started_at = Instant::now();

            let confirmation_result = match timeout_secs {
                Some(timeout_secs) => {
                    debug!(
                        "Waiting for user confirmation with timeout: timeout_secs={}, tool_name={}",
                        timeout_secs, tool_name
                    );
                    // There is a timeout limit
                    timeout(Duration::from_secs(timeout_secs), rx).await.ok()
                }
                None => {
                    debug!("Waiting for user confirmation without timeout: tool_name={}", tool_name);
                    Some(rx.await)
                }
            };
            confirmation_wait_ms = elapsed_ms_u64(confirmation_started_at);

            let confirmation_wait_result = match confirmation_result {
                Some(Ok(ConfirmationResponse::Confirmed)) => {
                    debug!("Tool confirmed: tool_name={}", tool_name);
                    ToolConfirmationWaitResult::Confirmed
                }
                Some(Ok(ConfirmationResponse::Rejected(reason))) => ToolConfirmationWaitResult::Rejected(reason),
                Some(Err(_)) => ToolConfirmationWaitResult::ChannelClosed,
                None => ToolConfirmationWaitResult::TimedOut,
            };
            let confirmation_outcome = resolve_confirmation_wait_result(confirmation_wait_result, &tool_name);

            if let Some(failure) = resolve_confirmation_failure(confirmation_outcome) {
                if matches!(
                    failure.kind,
                    ConfirmationFailureKind::ChannelClosed | ConfirmationFailureKind::Timeout
                ) {
                    self.confirmation_channels.remove(&tool_id);
                }

                if matches!(failure.kind, ConfirmationFailureKind::Timeout) {
                    warn!("{}", failure.error_message);
                }

                self.state_manager
                    .update_state(
                        &tool_id,
                        ToolExecutionState::Cancelled {
                            reason: failure.state_reason,
                            duration_ms: Some(elapsed_ms_u64(start_time)),
                            queue_wait_ms: Some(queue_wait_ms),
                            preflight_ms: Some(preflight_ms),
                            confirmation_wait_ms: Some(elapsed_ms_u64(confirmation_started_at)),
                            execution_ms: None,
                        },
                    )
                    .await;

                match failure.kind {
                    ConfirmationFailureKind::Rejected => {
                        return Err(NortHingError::Validation(failure.error_message));
                    }
                    ConfirmationFailureKind::ChannelClosed => {
                        return Err(NortHingError::service(failure.error_message));
                    }
                    ConfirmationFailureKind::Timeout => {
                        return Err(NortHingError::Timeout(failure.error_message));
                    }
                }
            }

            self.confirmation_channels.remove(&tool_id);
        }

        let preflight_ms = elapsed_ms_u64(start_time).saturating_sub(confirmation_wait_ms);

        if cancellation_token.is_cancelled() {
            self.state_manager
                .update_state(
                    &tool_id,
                    ToolExecutionState::Cancelled {
                        reason: "Tool was cancelled before execution".to_string(),
                        duration_ms: Some(elapsed_ms_u64(start_time)),
                        queue_wait_ms: Some(queue_wait_ms),
                        preflight_ms: Some(preflight_ms),
                        confirmation_wait_ms: Some(confirmation_wait_ms),
                        execution_ms: None,
                    },
                )
                .await;
            self.cancellation_tokens.remove(&tool_id);
            return Err(NortHingError::Cancelled(
                "Tool was cancelled before execution".to_string(),
            ));
        }

        // Set initial state
        if is_streaming {
            self.state_manager
                .update_state(
                    &tool_id,
                    ToolExecutionState::Streaming {
                        started_at: std::time::SystemTime::now(),
                        chunks_received: 0,
                    },
                )
                .await;
        } else {
            self.state_manager
                .update_state(
                    &tool_id,
                    ToolExecutionState::Running {
                        started_at: std::time::SystemTime::now(),
                        progress: None,
                    },
                )
                .await;
        }

        let execution_started_at = Instant::now();
        let tool_context = self.build_tool_use_context(&task, cancellation_token.clone());
        let result = self.execute_with_retry(&task, cancellation_token.clone(), tool).await;
        let execution_ms = elapsed_ms_u64(execution_started_at);

        self.cancellation_tokens.remove(&tool_id);

        match result {
            Ok(tool_result) => {
                let duration_ms = elapsed_ms_u64(start_time);
                let mut tool_result =
                    tool_result_storage::maybe_persist_large_tool_result(tool_result, &tool_context).await;
                tool_result.duration_ms = Some(duration_ms);

                // The tool call succeeded with arguments that we patched
                // because the model's output was truncated mid-stream. Tell
                // the model so it can decide whether the partial call needs
                // to be continued or regenerated.
                if recovered_from_truncation {
                    let original = tool_result.result_for_assistant.unwrap_or_default();
                    let notice = build_tool_call_truncation_recovery_notice(&tool_name);
                    tool_result.result_for_assistant = Some(if original.is_empty() {
                        notice.trim_end().to_string()
                    } else {
                        format!("{notice}{original}")
                    });
                }

                self.state_manager
                    .update_state(
                        &tool_id,
                        ToolExecutionState::Completed {
                            result: convert_to_framework_result(&tool_result),
                            duration_ms,
                            queue_wait_ms: Some(queue_wait_ms),
                            preflight_ms: Some(preflight_ms),
                            confirmation_wait_ms: Some(confirmation_wait_ms),
                            execution_ms: Some(execution_ms),
                        },
                    )
                    .await;

                info!(
                    "Tool completed: tool_name={}, duration_ms={}, queue_wait_ms={}, preflight_ms={}, confirmation_wait_ms={}, execution_ms={}, streaming={}",
                    tool_name,
                    duration_ms,
                    queue_wait_ms,
                    preflight_ms,
                    confirmation_wait_ms,
                    execution_ms,
                    is_streaming
                );

                Ok(build_success_result(tool_id, tool_name, tool_result, duration_ms))
            }
            Err(e) => {
                // Cancellation is a first-class terminal state, not a failure.
                // Preserve Cancelled here so a late cancel cannot be overwritten
                // by the generic Failed branch below.
                if let NortHingError::Cancelled(reason) = &e {
                    self.state_manager
                        .update_state(
                            &tool_id,
                            ToolExecutionState::Cancelled {
                                reason: reason.clone(),
                                duration_ms: Some(elapsed_ms_u64(start_time)),
                                queue_wait_ms: Some(queue_wait_ms),
                                preflight_ms: Some(preflight_ms),
                                confirmation_wait_ms: Some(confirmation_wait_ms),
                                execution_ms: Some(execution_ms),
                            },
                        )
                        .await;

                    info!(
                        "Tool cancelled during execution: tool_name={}, reason={}, duration_ms={}, queue_wait_ms={}, preflight_ms={}, confirmation_wait_ms={}, execution_ms={}",
                        tool_name,
                        reason,
                        elapsed_ms_u64(start_time),
                        queue_wait_ms,
                        preflight_ms,
                        confirmation_wait_ms,
                        execution_ms
                    );

                    return Err(e);
                }

                let error_msg = e.to_string();
                let is_retryable = task.options.max_retries > 0;

                self.state_manager
                    .update_state(
                        &tool_id,
                        ToolExecutionState::Failed {
                            error: error_msg.clone(),
                            is_retryable,
                            duration_ms: Some(elapsed_ms_u64(start_time)),
                            queue_wait_ms: Some(queue_wait_ms),
                            preflight_ms: Some(preflight_ms),
                            confirmation_wait_ms: Some(confirmation_wait_ms),
                            execution_ms: Some(execution_ms),
                        },
                    )
                    .await;

                error!(
                    "Tool failed: tool_name={}, error={}, duration_ms={}, queue_wait_ms={}, preflight_ms={}, confirmation_wait_ms={}, execution_ms={}",
                    tool_name,
                    error_msg,
                    elapsed_ms_u64(start_time),
                    queue_wait_ms,
                    preflight_ms,
                    confirmation_wait_ms,
                    execution_ms
                );

                Err(e)
            }
        }
    }

    /// Execute with retry
    async fn execute_with_retry(
        &self,
        task: &ToolTask,
        cancellation_token: CancellationToken,
        tool: Arc<dyn crate::agentic::tools::framework::Tool>,
    ) -> NortHingResult<ModelToolResult> {
        let mut attempts = 0;
        let max_attempts = task.options.max_retries + 1;

        loop {
            // Check cancellation token
            if cancellation_token.is_cancelled() {
                return Err(NortHingError::Cancelled("Tool execution was cancelled".to_string()));
            }

            attempts += 1;

            let result = self
                .execute_tool_impl(task, cancellation_token.clone(), tool.clone())
                .await;

            match result {
                Ok(r) => return Ok(r),
                Err(e) => {
                    if !should_retry_tool_attempt(ToolRetryAttemptFacts {
                        attempts,
                        max_attempts,
                        error_class: classify_tool_retry_error(&e),
                    }) {
                        return Err(e);
                    }

                    debug!(
                        "Retrying tool execution: attempt={}/{}, error={}",
                        attempts, max_attempts, e
                    );

                    // Wait for a period of time and retry
                    tokio::time::sleep(Duration::from_millis(retry_delay_ms(attempts))).await;
                }
            }
        }
    }

    /// Actual execution of tool
    async fn execute_tool_impl(
        &self,
        task: &ToolTask,
        cancellation_token: CancellationToken,
        tool: Arc<dyn crate::agentic::tools::framework::Tool>,
    ) -> NortHingResult<ModelToolResult> {
        // Check cancellation token
        if cancellation_token.is_cancelled() {
            return Err(NortHingError::Cancelled("Tool execution was cancelled".to_string()));
        }

        let tool_context = self.build_tool_use_context(task, cancellation_token);

        let execution_future = tool.call(&task.tool_call.arguments, &tool_context);

        let pipeline_timeout_secs = if tool.manages_own_execution_timeout() {
            None
        } else {
            task.options.timeout_secs
        };

        let tool_results = match pipeline_timeout_secs {
            Some(timeout_secs) => {
                let timeout_duration = Duration::from_secs(timeout_secs);
                let result = timeout(timeout_duration, execution_future).await.map_err(|_| {
                    NortHingError::Timeout(format!("Tool execution timeout: {}", task.tool_call.tool_name))
                })?;
                result?
            }
            None => execution_future.await?,
        };

        if tool.supports_streaming() && tool_results.len() > 1 {
            self.handle_streaming_results(task, &tool_results).await?;
        }

        tool_results
            .into_iter()
            .last()
            .map(|r| convert_tool_result(r, &task.tool_call.tool_id, &task.tool_call.tool_name))
            .ok_or_else(|| NortHingError::Tool(format!("Tool did not return result: {}", task.tool_call.tool_name)))
    }

    /// Handle streaming results
    async fn handle_streaming_results(&self, task: &ToolTask, results: &[FrameworkToolResult]) -> NortHingResult<()> {
        let mut chunks_received = 0;

        for result in results {
            if let FrameworkToolResult::StreamChunk {
                data,
                chunk_index: _,
                is_final: _,
            } = result
            {
                chunks_received += 1;

                // Update state
                self.state_manager
                    .update_state(
                        &task.tool_call.tool_id,
                        ToolExecutionState::Streaming {
                            started_at: std::time::SystemTime::now(),
                            chunks_received,
                        },
                    )
                    .await;

                // Send StreamChunk event
                let _event_data = ToolEventData::StreamChunk {
                    tool_id: task.tool_call.tool_id.clone(),
                    tool_name: task.tool_call.tool_name.clone(),
                    data: data.clone(),
                };
            }
        }

        Ok(())
    }
}
