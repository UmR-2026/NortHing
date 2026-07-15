//! Task tool — subagent dispatch + execution loop (Round 12 split)
//!
//! Owns:
//! - `background_subagent_started_assistant_message` (legacy helper)
//! - `dispatch_background_subagent` extracted from `call_impl` Phase 3 (R7 pattern)
//! - `execute_subagent_loop` extracted from `call_impl` Phase 4 (R7 pattern)
//! - 2 tests moved verbatim
//!
//! Spec: `docs/handoffs/2026-06-29-round12-task-tool-split-spec.md` (f0f9bc0).

use super::task_tool_input::CallInputs;
use crate::agentic::coordination::{global_coordinator, SubagentExecutionRequest, SubagentResult};
use crate::agentic::deep_review::task_adapter::{self as deep_review_task_adapter, DeepReviewQueueWaitOutcome};
use crate::agentic::deep_review_policy::{
    deep_review_active_reviewer_count, deep_review_effective_parallel_instances,
    record_deep_review_effective_concurrency_success, DeepReviewActiveReviewerGuard, DeepReviewConcurrencyPolicy,
    DeepReviewExecutionPolicy, DeepReviewPolicyViolation, DeepReviewSubagentRole,
};
use crate::agentic::events::DeepReviewQueueStatus;
use crate::agentic::tools::framework::{ToolResult, ToolUseContext};
use crate::agentic::tools::pipeline::SubagentParentInfo;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::timing::elapsed_ms_u64;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, warn};

/// Background subagent started message (legacy, unchanged).
pub(super) fn background_subagent_started_assistant_message(
    delegate_target_label: &str,
    background_task_id: &str,
) -> String {
    format!(
        "Background {} started successfully.\n<background_task status=\"started\" id=\"{}\">Its final result will be delivered back automatically to you when it is finished. Do not poll for status updates. If your current path is blocked on this result and there is no other useful local work to do, it is fine to end the current turn.</background_task>",
        delegate_target_label, background_task_id
    )
}

/// Mutable DeepReview execution state passed between call_impl phases.
pub(super) struct DeepReviewContext {
    pub effective_policy: Option<DeepReviewExecutionPolicy>,
    pub active_guard: Option<DeepReviewActiveReviewerGuard<'static>>,
    pub reviewer_configured_max_parallel_instances: Option<usize>,
    pub concurrency_policy: Option<DeepReviewConcurrencyPolicy>,
    pub is_optional_reviewer: bool,
    pub launch_batch_info: Option<crate::agentic::deep_review::task_adapter::DeepReviewLaunchBatchInfo>,
    pub retry_scope_files: Option<Vec<String>>,
    pub subagent_role: Option<DeepReviewSubagentRole>,
    pub subagent_context_map: Option<HashMap<String, String>>,
    /// If a deep_review incremental cache hit was found during setup, this holds
    /// the early-return `ToolResult` so the facade can short-circuit.
    pub cache_hit_result: Option<ToolResult>,
}

impl Default for DeepReviewContext {
    fn default() -> Self {
        Self {
            effective_policy: None,
            active_guard: None,
            reviewer_configured_max_parallel_instances: None,
            concurrency_policy: None,
            is_optional_reviewer: false,
            launch_batch_info: None,
            retry_scope_files: None,
            subagent_role: None,
            subagent_context_map: None,
            cache_hit_result: None,
        }
    }
}

/// Outcome of `execute_subagent_loop`. The success arm carries the coordinator
/// result; the other arms are early-exit `ToolResult`s that the facade
/// converts into `Vec<ToolResult>`.
pub(super) enum ExecuteOutcome {
    Success(SubagentResult),
    CancelledReviewer(ToolResult),
    ProviderCapacitySkip(ToolResult),
    LocalCapacitySkip(ToolResult),
}

/// call_impl Phase 3: background subagent dispatch.
pub(super) async fn dispatch_background_subagent(
    inputs: &CallInputs,
    context: &ToolUseContext,
    prepared_prompt: String,
    timeout_seconds: Option<u64>,
    subagent_context_map: HashMap<String, String>,
) -> NortHingResult<Vec<ToolResult>> {
    let coordinator =
        global_coordinator().ok_or_else(|| NortHingError::tool("coordinator not initialized".to_string()))?;
    let parent_info = SubagentParentInfo {
        tool_call_id: inputs.tool_call_id.clone(),
        session_id: inputs.session_id.clone(),
        dialog_turn_id: inputs.dialog_turn_id.clone(),
    };
    let background_result = coordinator
        .start_background_subagent(
            SubagentExecutionRequest {
                task_description: prepared_prompt,
                context_mode: inputs.context_mode,
                subagent_type: inputs.subagent_type.clone(),
                workspace_path: inputs.effective_workspace_path.clone(),
                model_id: inputs.model_id.clone(),
                subagent_parent_info: parent_info,
                context: subagent_context_map,
                delegation_policy: context.delegation_policy().spawn_child(),
            },
            timeout_seconds,
            context.actor_runtime(),
        )
        .await?;
    Ok(vec![ToolResult::Result {
        data: json!({
            "context_mode": inputs.context_mode.as_str(),
            "status": "started",
            "run_in_background": true,
            "background_task_id": background_result.background_task_id,
        }),
        result_for_assistant: Some(background_subagent_started_assistant_message(
            &inputs.delegate_target_label,
            &background_result.background_task_id,
        )),
        image_attachments: None,
    }])
}

/// call_impl Phase 4: main execution loop with provider capacity retry handling
/// for DeepReview reviewers. The body of this fn corresponds to original lines
/// 1246-1507.
pub(super) async fn execute_subagent_loop(
    inputs: &CallInputs,
    context: &ToolUseContext,
    dr_ctx: &mut DeepReviewContext,
    prepared_prompt: String,
    timeout_seconds: Option<u64>,
    start_time: Instant,
) -> NortHingResult<ExecuteOutcome> {
    let coordinator =
        global_coordinator().ok_or_else(|| NortHingError::tool("coordinator not initialized".to_string()))?;
    let mut provider_capacity_retry = deep_review_task_adapter::DeepReviewProviderCapacityRetryRuntime::default();
    let deep_review_subagent_id = inputs.subagent_type.as_deref().unwrap_or("");
    let result = loop {
        let parent_info = SubagentParentInfo {
            tool_call_id: inputs.tool_call_id.clone(),
            session_id: inputs.session_id.clone(),
            dialog_turn_id: inputs.dialog_turn_id.clone(),
        };
        let subagent_execution_started_at = Instant::now();
        debug!(
            "TaskTool awaiting subagent result: parent_session_id={}, dialog_turn_id={}, tool_call_id={}, context_mode={}, delegate_target={}, timeout_seconds={:?}, workspace_path={:?}, model_id={:?}",
            inputs.session_id,
            inputs.dialog_turn_id,
            inputs.tool_call_id,
            inputs.context_mode.as_str(),
            inputs.delegate_target_label,
            timeout_seconds,
            inputs.effective_workspace_path,
            inputs.model_id
        );
        let execution_result = coordinator
            .execute_subagent(
                SubagentExecutionRequest {
                    task_description: prepared_prompt.clone(),
                    context_mode: inputs.context_mode,
                    subagent_type: inputs.subagent_type.clone(),
                    workspace_path: inputs.effective_workspace_path.clone(),
                    model_id: inputs.model_id.clone(),
                    subagent_parent_info: parent_info,
                    context: dr_ctx.subagent_context_map.clone().unwrap_or_default(),
                    delegation_policy: context.delegation_policy().spawn_child(),
                },
                context.cancellation_token(),
                timeout_seconds,
                context.actor_runtime(),
            )
            .await;

        match execution_result {
            Ok(result) => {
                debug!(
                    "TaskTool subagent returned: parent_session_id={}, dialog_turn_id={}, tool_call_id={}, context_mode={}, delegate_target={}, status={:?}, text_len={}, duration_ms={}, ledger_event_id={:?}",
                    inputs.session_id,
                    inputs.dialog_turn_id,
                    inputs.tool_call_id,
                    inputs.context_mode.as_str(),
                    inputs.delegate_target_label,
                    result.status,
                    result.text.len(),
                    elapsed_ms_u64(subagent_execution_started_at),
                    result.ledger_event_id()
                );
                if let Some(reason) = provider_capacity_retry.last_retry_reason() {
                    super::task_tool_deep_review::record_deep_review_provider_capacity_retry_success(
                        &inputs.dialog_turn_id,
                        reason,
                    );
                }
                break result;
            }
            Err(error) => {
                warn!(
                    "TaskTool subagent failed: parent_session_id={}, dialog_turn_id={}, tool_call_id={}, context_mode={}, delegate_target={}, duration_ms={}, error={}",
                    inputs.session_id,
                    inputs.dialog_turn_id,
                    inputs.tool_call_id,
                    inputs.context_mode.as_str(),
                    inputs.delegate_target_label,
                    elapsed_ms_u64(subagent_execution_started_at),
                    error
                );
                if matches!(dr_ctx.subagent_role, Some(DeepReviewSubagentRole::Reviewer))
                    && matches!(error, NortHingError::Cancelled(_))
                    && !context
                        .cancellation_token()
                        .as_ref()
                        .is_some_and(|token| token.is_cancelled())
                {
                    let reason = match &error {
                        NortHingError::Cancelled(reason) => reason.as_str(),
                        _ => "",
                    };
                    return Ok(ExecuteOutcome::CancelledReviewer(
                        super::task_tool_deep_review::deep_review_cancelled_reviewer_tool_result(
                            deep_review_subagent_id,
                            reason,
                            start_time.elapsed().as_millis(),
                        ),
                    ));
                }
                if matches!(dr_ctx.subagent_role, Some(DeepReviewSubagentRole::Reviewer)) {
                    if let Some(conc_policy) = dr_ctx.concurrency_policy.as_ref() {
                        let decision =
                            super::task_tool_deep_review::deep_review_capacity_decision_for_provider_error(&error);
                        match provider_capacity_retry.decide_after_error(&decision, conc_policy) {
                            deep_review_task_adapter::DeepReviewProviderCapacityRetryDecision::NotQueueable => {}
                            deep_review_task_adapter::DeepReviewProviderCapacityRetryDecision::CapacitySkipped {
                                reason,
                                queue_elapsed_ms,
                            } => {
                                drop(dr_ctx.active_guard.take());
                                let (data, assistant_message) = super::task_tool_deep_review::deep_review_capacity_skip_result_for_provider_queue_outcome(
                                    reason,
                                    &inputs.dialog_turn_id,
                                    deep_review_subagent_id,
                                    conc_policy,
                                    start_time.elapsed().as_millis(),
                                    queue_elapsed_ms,
                                    None,
                                );
                                let effective_parallel_instances = data
                                    .get("effective_parallel_instances")
                                    .and_then(Value::as_u64)
                                    .and_then(|value| usize::try_from(value).ok());
                                super::task_tool_deep_review::emit_deep_review_queue_state(
                                    &inputs.session_id,
                                    &inputs.dialog_turn_id,
                                    &inputs.tool_call_id,
                                    deep_review_subagent_id,
                                    DeepReviewQueueStatus::CapacitySkipped,
                                    Some(reason),
                                    0,
                                    deep_review_active_reviewer_count(&inputs.dialog_turn_id),
                                    dr_ctx.is_optional_reviewer.then_some(1),
                                    effective_parallel_instances,
                                    queue_elapsed_ms,
                                    conc_policy.max_queue_wait_seconds,
                                )
                                .await;
                                return Ok(ExecuteOutcome::ProviderCapacitySkip(ToolResult::Result {
                                    data,
                                    result_for_assistant: Some(assistant_message),
                                    image_attachments: None,
                                }));
                            }
                            deep_review_task_adapter::DeepReviewProviderCapacityRetryDecision::WaitForCapacity {
                                reason,
                                max_wait_seconds,
                            } => {
                                drop(dr_ctx.active_guard.take());
                                match super::task_tool_deep_review::wait_for_deep_review_provider_capacity_retry(
                                    &inputs.session_id,
                                    &inputs.dialog_turn_id,
                                    &inputs.tool_call_id,
                                    deep_review_subagent_id,
                                    conc_policy,
                                    reason,
                                    max_wait_seconds,
                                    dr_ctx.is_optional_reviewer,
                                )
                                .await
                                {
                                    deep_review_task_adapter::DeepReviewProviderQueueWaitOutcome::ReadyToRetry {
                                        queue_elapsed_ms,
                                        early_capacity_probe,
                                    } => {
                                        provider_capacity_retry.record_ready_to_retry(
                                            reason,
                                            queue_elapsed_ms,
                                            early_capacity_probe,
                                        );
                                        let effective_parallel_instances = deep_review_effective_parallel_instances(
                                            &inputs.dialog_turn_id,
                                            conc_policy.max_parallel_instances,
                                        );
                                        match super::task_tool_deep_review::try_begin_deep_review_reviewer_admission(
                                            &inputs.dialog_turn_id,
                                            effective_parallel_instances,
                                            dr_ctx.launch_batch_info.as_ref(),
                                        ) {
                                            Ok(Some(guard)) => {
                                                dr_ctx.active_guard = Some(guard);
                                            }
                                            Ok(None)
                                            | Err(DeepReviewPolicyViolation {
                                                code: "deep_review_launch_batch_blocked",
                                                ..
                                            }) => {
                                                match super::task_tool_deep_review::wait_for_deep_review_reviewer_admission(
                                                    &inputs.session_id,
                                                    &inputs.dialog_turn_id,
                                                    &inputs.tool_call_id,
                                                    deep_review_subagent_id,
                                                    conc_policy,
                                                    dr_ctx.is_optional_reviewer,
                                                    dr_ctx.launch_batch_info.as_ref(),
                                                )
                                                .await?
                                                {
                                                    DeepReviewQueueWaitOutcome::Ready { guard } => {
                                                        dr_ctx.active_guard = Some(guard);
                                                    }
                                                    DeepReviewQueueWaitOutcome::Skipped {
                                                        queue_elapsed_ms,
                                                        skip_reason,
                                                        capacity_reason,
                                                    } => {
                                                        return Ok(ExecuteOutcome::LocalCapacitySkip(
                                                            super::task_tool_deep_review::deep_review_local_capacity_skip_tool_result(
                                                                &inputs.dialog_turn_id,
                                                                deep_review_subagent_id,
                                                                conc_policy,
                                                                capacity_reason,
                                                                skip_reason,
                                                                queue_elapsed_ms,
                                                                start_time.elapsed().as_millis(),
                                                            ),
                                                        ));
                                                    }
                                                }
                                            }
                                            Err(violation) => {
                                                return Err(NortHingError::tool(format!(
                                                    "DeepReview Task policy violation: {}",
                                                    violation.to_tool_error_message()
                                                )));
                                            }
                                        }
                                        super::task_tool_deep_review::record_deep_review_provider_capacity_retry(
                                            &inputs.dialog_turn_id,
                                            reason,
                                        );
                                        continue;
                                    }
                                    deep_review_task_adapter::DeepReviewProviderQueueWaitOutcome::Skipped {
                                        queue_elapsed_ms,
                                        skip_reason,
                                    } => {
                                        let total_provider_capacity_queue_elapsed_ms =
                                            provider_capacity_retry.record_queue_skipped(queue_elapsed_ms);
                                        let (data, assistant_message) = super::task_tool_deep_review::deep_review_capacity_skip_result_for_provider_queue_outcome(
                                            reason,
                                            &inputs.dialog_turn_id,
                                            deep_review_subagent_id,
                                            conc_policy,
                                            start_time.elapsed().as_millis(),
                                            total_provider_capacity_queue_elapsed_ms,
                                            Some(skip_reason),
                                        );
                                        return Ok(ExecuteOutcome::ProviderCapacitySkip(ToolResult::Result {
                                            data,
                                            result_for_assistant: Some(assistant_message),
                                            image_attachments: None,
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
                return Err(error);
            }
        }
    };
    if !result.is_partial_timeout() {
        if let Some(configured_max_parallel_instances) = dr_ctx.reviewer_configured_max_parallel_instances {
            record_deep_review_effective_concurrency_success(&inputs.dialog_turn_id, configured_max_parallel_instances);
        }
    }
    drop(dr_ctx.active_guard.take());
    Ok(ExecuteOutcome::Success(result))
}

#[cfg(test)]
mod tests {
    use super::background_subagent_started_assistant_message;

    #[test]
    fn background_subagent_start_acknowledgement_keeps_structured_task_marker() {
        let message = background_subagent_started_assistant_message("GeneralPurpose", "bg-subagent-123");

        assert!(message.starts_with("Background GeneralPurpose started successfully."));
        assert!(message.contains("<background_task status=\"started\" id=\"bg-subagent-123\">"));
        assert!(message.contains("Do not poll for status updates."));
        assert!(message.ends_with("</background_task>"));
        assert!(!message.contains("background_task_id="));
    }

    // call_impl_rejects_nested_subagent_delegation lives in facade tests
    // (it exercises the full call_impl entrypoint, which must remain in facade).
}
