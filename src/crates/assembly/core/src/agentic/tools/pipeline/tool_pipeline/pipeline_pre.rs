use super::pipeline_logging::*;
use super::pipeline_post::build_user_steering_interrupted_result;
use super::pipeline_types::*;
use crate::agentic::core::ToolExecutionState;
use crate::agentic::tools::pipeline::state_manager::{tool_task_state_kind, ToolStateManager};
use crate::agentic::tools::pipeline::types::*;
use crate::agentic::tools::tool_context_runtime;
use crate::agentic::tools::tool_context_runtime::ToolUseContext;
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_agent_tools::USER_STEERING_INTERRUPTED_MESSAGE;
use tokio_util::sync::CancellationToken;
use tool_runtime::pipeline::{should_cancel_tool_state, summarize_dialog_turn_cancellation};
use tracing::{debug, info};

pub fn map_tool_execution_admission_rejection(
    error: northhing_agent_tools::ToolExecutionAdmissionRejection,
) -> NortHingError {
    match error {
        northhing_agent_tools::ToolExecutionAdmissionRejection::RuntimeRestriction(error) => error.into(),
        northhing_agent_tools::ToolExecutionAdmissionRejection::AllowedList(error) => {
            NortHingError::Validation(error.to_string())
        }
        northhing_agent_tools::ToolExecutionAdmissionRejection::Collapsed(error) => {
            NortHingError::Validation(error.to_string())
        }
    }
}

pub fn validate_task_admission(task: &ToolTask) -> Result<(), NortHingError> {
    northhing_agent_tools::validate_tool_execution_admission(northhing_agent_tools::ToolExecutionAdmissionRequest {
        tool_name: &task.tool_call.tool_name,
        allowed_tools: &task.context.allowed_tools,
        runtime_tool_restrictions: &task.context.runtime_tool_restrictions,
        collapsed_tools: &task.context.collapsed_tools,
        loaded_collapsed_tools: &task.context.unlocked_collapsed_tools,
        get_tool_spec_tool_name: northhing_agent_tools::GET_TOOL_SPEC_TOOL_NAME,
    })
    .map_err(map_tool_execution_admission_rejection)
}

impl ToolPipeline {
    pub(crate) fn should_interrupt_for_steering(&self, context: &ToolExecutionContext) -> bool {
        context
            .steering_interrupt
            .as_ref()
            .map(|interrupt| interrupt.should_interrupt())
            .unwrap_or(false)
    }

    pub(crate) async fn build_steering_interrupted_results(
        &self,
        task_ids: impl IntoIterator<Item = String>,
    ) -> Vec<ToolExecutionResult> {
        let mut results = Vec::new();
        for task_id in task_ids {
            let task = self.state_manager.get_task(&task_id);
            self.state_manager
                .update_state(
                    &task_id,
                    ToolExecutionState::Cancelled {
                        reason: northhing_agent_tools::USER_STEERING_INTERRUPTED_MESSAGE.to_string(),
                        duration_ms: None,
                        queue_wait_ms: None,
                        preflight_ms: None,
                        confirmation_wait_ms: None,
                        execution_ms: None,
                    },
                )
                .await;
            results.push(build_user_steering_interrupted_result(&task_id, task));
        }
        results
    }

    pub(crate) fn build_tool_use_context(
        &self,
        task: &ToolTask,
        cancellation_token: CancellationToken,
    ) -> ToolUseContext {
        tool_context_runtime::build_tool_use_context_for_task(
            task,
            self.computer_use_host.clone(),
            cancellation_token,
            self.actor_runtime.get().cloned(),
        )
    }

    /// Cancel tool execution
    pub async fn cancel_tool(&self, tool_id: &str, reason: String) -> NortHingResult<()> {
        let Some(task) = self.state_manager.get_task(tool_id) else {
            debug!("Ignoring cancel request for unknown tool: tool_id={}", tool_id);
            return Ok(());
        };

        if tool_task_state_kind(&task.state).is_terminal() {
            debug!(
                "Ignoring duplicate cancel request for tool in terminal state: tool_id={}, state={:?}",
                tool_id, task.state
            );
            return Ok(());
        }

        // 1. Trigger cancellation token
        if let Some((_, token)) = self.cancellation_tokens.remove(tool_id) {
            token.cancel();
            debug!("Cancellation token triggered: tool_id={}", tool_id);
        } else {
            debug!(
                "Cancellation token not found (tool may have completed): tool_id={}",
                tool_id
            );
        }

        // 2. Clean up confirmation channel (if waiting for confirmation)
        if let Some((_, _tx)) = self.confirmation_channels.remove(tool_id) {
            // Channel will be automatically closed, causing await rx to return Err
            debug!("Cleared confirmation channel: tool_id={}", tool_id);
        }

        // 3. Update state to cancelled
        self.state_manager
            .update_state(
                tool_id,
                ToolExecutionState::Cancelled {
                    reason: reason.clone(),
                    duration_ms: None,
                    queue_wait_ms: None,
                    preflight_ms: None,
                    confirmation_wait_ms: None,
                    execution_ms: None,
                },
            )
            .await;

        info!("Tool execution cancelled: tool_id={}, reason={}", tool_id, reason);
        Ok(())
    }

    /// Cancel all tools for a dialog turn
    pub async fn cancel_dialog_turn_tools(&self, dialog_turn_id: &str) -> NortHingResult<()> {
        info!(
            "Cancelling all tools for dialog turn: dialog_turn_id={}",
            dialog_turn_id
        );

        let tasks = self.state_manager.get_dialog_turn_tasks(dialog_turn_id);
        debug!("Found {} tool tasks for dialog turn", tasks.len());

        let summary = summarize_dialog_turn_cancellation(tasks.iter().map(|task| tool_task_state_kind(&task.state)));

        for task in tasks {
            if should_cancel_tool_state(tool_task_state_kind(&task.state)) {
                debug!(
                    "Cancelling tool: tool_id={}, state={:?}",
                    task.tool_call.tool_id, task.state
                );
                self.cancel_tool(&task.tool_call.tool_id, "Dialog turn cancelled".to_string())
                    .await?;
            } else {
                debug!(
                    "Skipping tool (state not cancellable): tool_id={}, state={:?}",
                    task.tool_call.tool_id, task.state
                );
            }
        }

        info!(
            "Tool cancellation completed: cancelled={}, skipped={}",
            summary.cancelled, summary.skipped
        );
        Ok(())
    }

    /// Confirm tool execution
    pub async fn confirm_tool(&self, tool_id: &str, updated_input: Option<serde_json::Value>) -> NortHingResult<()> {
        let task = self
            .state_manager
            .get_task(tool_id)
            .ok_or_else(|| NortHingError::NotFound(format!("Tool task not found: {}", tool_id)))?;

        // Check if the state is waiting for confirmation
        if !matches!(task.state, ToolExecutionState::AwaitingConfirmation { .. }) {
            return Err(NortHingError::Validation(format!(
                "Tool is not in awaiting confirmation state: {:?}",
                task.state
            )));
        }

        // If the user modified the parameters, update the task parameters first
        if let Some(new_args) = updated_input {
            debug!("User updated tool arguments: tool_id={}", tool_id);
            self.state_manager.update_task_arguments(tool_id, new_args);
        }

        // Get sender from map and send confirmation response
        if let Some((_, tx)) = self.confirmation_channels.remove(tool_id) {
            let _ = tx.send(ConfirmationResponse::Confirmed);
            info!("User confirmed tool execution: tool_id={}", tool_id);
            Ok(())
        } else {
            Err(NortHingError::NotFound(format!(
                "Confirmation channel not found: {}",
                tool_id
            )))
        }
    }

    /// Reject tool execution
    pub async fn reject_tool(&self, tool_id: &str, reason: String) -> NortHingResult<()> {
        let task = self
            .state_manager
            .get_task(tool_id)
            .ok_or_else(|| NortHingError::NotFound(format!("Tool task not found: {}", tool_id)))?;

        // Check if the state is waiting for confirmation
        if !matches!(task.state, ToolExecutionState::AwaitingConfirmation { .. }) {
            return Err(NortHingError::Validation(format!(
                "Tool is not in awaiting confirmation state: {:?}",
                task.state
            )));
        }

        // Get sender from map and send rejection response
        if let Some((_, tx)) = self.confirmation_channels.remove(tool_id) {
            let _ = tx.send(ConfirmationResponse::Rejected(reason.clone()));
            info!("User rejected tool execution: tool_id={}, reason={}", tool_id, reason);
            Ok(())
        } else {
            // If the channel does not exist, mark it as cancelled directly
            self.state_manager
                .update_state(
                    tool_id,
                    ToolExecutionState::Cancelled {
                        reason: format!("User rejected: {}", reason),
                        duration_ms: None,
                        queue_wait_ms: None,
                        preflight_ms: None,
                        confirmation_wait_ms: None,
                        execution_ms: None,
                    },
                )
                .await;

            Ok(())
        }
    }
}
