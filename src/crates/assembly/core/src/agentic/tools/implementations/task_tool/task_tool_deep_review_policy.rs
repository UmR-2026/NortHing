//! Task tool — DeepReview policy + retry + queue helpers (Round 12b split: production code)
//!
//! Owns 20 deep_review_* production fns + `setup_deep_review_for_call` helper
//! extracted from `call_impl` Phase 2 (R7 turn_internal pattern).
//!
//! Tests live in the sibling `task_tool_deep_review_tests` (Round 12b split).
//!
//! Spec: `docs/handoffs/2026-06-29-round12b-task-tool-deep-review-secondary-split-spec.md` (e4261ff)

use super::task_tool_input::CallInputs;
use super::task_tool_subagent::DeepReviewContext;
use crate::agentic::agents::agent_registry;
use crate::agentic::coordination::global_coordinator;
use crate::agentic::deep_review::task_adapter::{
    self as deep_review_task_adapter, DeepReviewLaunchBatchInfo, DeepReviewProviderQueueWaitOutcome,
    DeepReviewQueueWaitOutcome, DeepReviewQueueWaitSkipReason,
};
use crate::agentic::deep_review_policy::{
    deep_review_active_reviewer_count, deep_review_effective_parallel_instances, deep_review_has_judge_been_launched,
    deep_review_turn_elapsed_seconds, load_default_deep_review_policy, record_deep_review_runtime_auto_retry,
    record_deep_review_runtime_auto_retry_suppressed, record_deep_review_runtime_manual_retry,
    record_deep_review_task_budget, DeepReviewActiveReviewerGuard, DeepReviewCapacityQueueReason,
    DeepReviewConcurrencyPolicy, DeepReviewExecutionPolicy, DeepReviewPolicyViolation, DeepReviewRunManifestGate,
    DeepReviewSubagentRole, DEEP_REVIEW_AGENT_TYPE,
};
use crate::agentic::events::DeepReviewQueueStatus;
use crate::agentic::tools::framework::{ToolResult, ToolUseContext};
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_runtime_ports::SubagentContextMode;
use serde_json::Value;
use std::time::Instant;
use tracing::warn;

// ----- 20 deep_review_* free functions (moved verbatim from original TaskTool::xxx) -----

pub(super) fn deep_review_launch_batch_for_task(
    subagent_type: &str,
    description: Option<&str>,
    run_manifest: Option<&Value>,
) -> Option<DeepReviewLaunchBatchInfo> {
    deep_review_task_adapter::deep_review_launch_batch_for_task(subagent_type, description, run_manifest)
}

pub(super) fn attach_deep_review_cache(run_manifest: &mut Value, cache_value: Option<Value>) {
    deep_review_task_adapter::attach_deep_review_cache(run_manifest, cache_value);
}

pub(super) fn deep_review_retry_guidance_max_retries(
    effective_policy: Option<&DeepReviewExecutionPolicy>,
    dialog_turn_id: &str,
) -> usize {
    deep_review_task_adapter::deep_review_retry_guidance_max_retries(effective_policy, dialog_turn_id)
}

pub(super) fn should_emit_deep_review_retry_guidance(
    is_partial_timeout: bool,
    is_retry: bool,
    deep_review_subagent_role: Option<DeepReviewSubagentRole>,
) -> bool {
    deep_review_task_adapter::should_emit_deep_review_retry_guidance(
        is_partial_timeout,
        is_retry,
        deep_review_subagent_role,
    )
}

pub(super) fn ensure_deep_review_retry_coverage(
    input: &Value,
    subagent_type: &str,
    run_manifest: Option<&Value>,
) -> Result<Vec<String>, DeepReviewPolicyViolation> {
    deep_review_task_adapter::ensure_deep_review_retry_coverage(input, subagent_type, run_manifest)
}

pub(super) fn auto_retry_suppression_reason(code: &str) -> &'static str {
    deep_review_task_adapter::auto_retry_suppression_reason(code)
}

pub(super) fn ensure_deep_review_auto_retry_allowed(
    conc_policy: &DeepReviewConcurrencyPolicy,
    dialog_turn_id: &str,
) -> Result<(), DeepReviewPolicyViolation> {
    deep_review_task_adapter::ensure_deep_review_auto_retry_allowed(
        conc_policy,
        deep_review_turn_elapsed_seconds(dialog_turn_id),
    )
}

pub(super) fn prompt_with_deep_review_retry_scope(prompt: &str, retry_scope_files: &[String]) -> String {
    deep_review_task_adapter::prompt_with_deep_review_retry_scope(prompt, retry_scope_files)
}

pub(super) fn deep_review_capacity_decision_for_provider_error(
    error: &NortHingError,
) -> crate::agentic::deep_review_policy::DeepReviewCapacityQueueDecision {
    deep_review_task_adapter::capacity_decision_for_provider_error(error)
}

pub(super) fn deep_review_capacity_skip_result_for_provider_queue_outcome(
    reason: DeepReviewCapacityQueueReason,
    dialog_turn_id: &str,
    subagent_type: &str,
    conc_policy: &DeepReviewConcurrencyPolicy,
    duration_ms: u128,
    queue_elapsed_ms: u64,
    terminal_skip_reason: Option<DeepReviewQueueWaitSkipReason>,
) -> (Value, String) {
    deep_review_task_adapter::capacity_skip_result_for_provider_queue_outcome(
        reason,
        dialog_turn_id,
        subagent_type,
        conc_policy,
        duration_ms,
        queue_elapsed_ms,
        terminal_skip_reason,
    )
}

pub(super) async fn wait_for_deep_review_provider_capacity_retry(
    session_id: &str,
    dialog_turn_id: &str,
    tool_id: &str,
    subagent_type: &str,
    conc_policy: &DeepReviewConcurrencyPolicy,
    reason: DeepReviewCapacityQueueReason,
    max_wait_seconds: u64,
    is_optional_reviewer: bool,
) -> DeepReviewProviderQueueWaitOutcome {
    deep_review_task_adapter::wait_for_provider_capacity_retry(
        session_id,
        dialog_turn_id,
        tool_id,
        subagent_type,
        conc_policy,
        reason,
        max_wait_seconds,
        is_optional_reviewer,
    )
    .await
}

pub(super) fn record_deep_review_provider_capacity_retry(dialog_turn_id: &str, reason: DeepReviewCapacityQueueReason) {
    deep_review_task_adapter::record_provider_capacity_retry(dialog_turn_id, reason);
}

pub(super) fn record_deep_review_provider_capacity_retry_success(
    dialog_turn_id: &str,
    reason: DeepReviewCapacityQueueReason,
) {
    deep_review_task_adapter::record_provider_capacity_retry_success(dialog_turn_id, reason);
}

pub(super) async fn emit_deep_review_queue_state(
    session_id: &str,
    dialog_turn_id: &str,
    tool_id: &str,
    subagent_type: &str,
    status: DeepReviewQueueStatus,
    reason: Option<DeepReviewCapacityQueueReason>,
    queued_reviewer_count: usize,
    active_reviewer_count: usize,
    optional_reviewer_count: Option<usize>,
    effective_parallel_instances: Option<usize>,
    queue_elapsed_ms: u64,
    max_queue_wait_seconds: u64,
) {
    deep_review_task_adapter::emit_queue_state(
        session_id,
        dialog_turn_id,
        tool_id,
        subagent_type,
        status,
        reason,
        queued_reviewer_count,
        active_reviewer_count,
        optional_reviewer_count,
        effective_parallel_instances,
        queue_elapsed_ms,
        max_queue_wait_seconds,
    )
    .await;
}

pub(super) fn try_begin_deep_review_reviewer_admission(
    dialog_turn_id: &str,
    effective_parallel_instances: usize,
    launch_batch_info: Option<&DeepReviewLaunchBatchInfo>,
) -> Result<Option<DeepReviewActiveReviewerGuard<'static>>, DeepReviewPolicyViolation> {
    deep_review_task_adapter::try_begin_reviewer_admission(
        dialog_turn_id,
        effective_parallel_instances,
        launch_batch_info,
    )
}

pub(super) async fn wait_for_deep_review_reviewer_admission(
    session_id: &str,
    dialog_turn_id: &str,
    tool_id: &str,
    subagent_type: &str,
    conc_policy: &DeepReviewConcurrencyPolicy,
    is_optional_reviewer: bool,
    launch_batch_info: Option<&DeepReviewLaunchBatchInfo>,
) -> NortHingResult<DeepReviewQueueWaitOutcome> {
    deep_review_task_adapter::wait_for_reviewer_admission(
        session_id,
        dialog_turn_id,
        tool_id,
        subagent_type,
        conc_policy,
        is_optional_reviewer,
        launch_batch_info,
    )
    .await
}

pub(super) fn deep_review_local_capacity_skip_tool_result(
    dialog_turn_id: &str,
    subagent_type: &str,
    conc_policy: &DeepReviewConcurrencyPolicy,
    capacity_reason: DeepReviewCapacityQueueReason,
    skip_reason: DeepReviewQueueWaitSkipReason,
    queue_elapsed_ms: u64,
    duration_ms: u128,
) -> ToolResult {
    let (data, assistant_message) = deep_review_task_adapter::capacity_skip_result_for_local_queue_outcome(
        dialog_turn_id,
        subagent_type,
        conc_policy,
        capacity_reason,
        skip_reason,
        queue_elapsed_ms,
        duration_ms,
    );
    ToolResult::Result {
        data,
        result_for_assistant: Some(assistant_message),
        image_attachments: None,
    }
}

pub(super) fn deep_review_cancelled_reviewer_tool_result(
    subagent_type: &str,
    reason: &str,
    duration_ms: u128,
) -> ToolResult {
    let (data, result_for_assistant) =
        deep_review_task_adapter::deep_review_cancelled_reviewer_result(subagent_type, reason, duration_ms);

    ToolResult::Result {
        data,
        result_for_assistant: Some(result_for_assistant),
        image_attachments: None,
    }
}

// ----- call_impl Phase 2: DeepReview setup helper -----

/// call_impl Phase 2: build the DeepReviewContext for a DeepReview parent task.
/// Returns the populated context + possibly-modified `timeout_seconds` (policy cap).
///
/// If the task is NOT a DeepReview parent, returns `None` and the caller skips
/// the DeepReview branch.
#[allow(clippy::too_many_arguments)]
pub(super) async fn setup_deep_review_for_call(
    inputs: &CallInputs,
    raw_input: &Value,
    context: &ToolUseContext,
    timeout_seconds_in: Option<u64>,
    _start_time: Instant,
) -> NortHingResult<Option<(DeepReviewContext, Option<u64>)>> {
    let coordinator =
        global_coordinator().ok_or_else(|| NortHingError::tool("coordinator not initialized".to_string()))?;

    let is_deep_review_parent = context
        .agent_type
        .as_deref()
        .map(str::trim)
        .is_some_and(|agent_type| agent_type == DEEP_REVIEW_AGENT_TYPE);

    if inputs.context_mode == SubagentContextMode::Fork && is_deep_review_parent {
        return Err(NortHingError::tool(
            "fork_context=true is not supported for DeepReview Task calls".to_string(),
        ));
    }

    if !is_deep_review_parent {
        return Ok(None);
    }

    let subagent_type = inputs
        .subagent_type
        .as_deref()
        .ok_or_else(|| NortHingError::tool("subagent_type is required for DeepReview Task calls".to_string()))?;
    let base_policy = load_default_deep_review_policy()
        .await
        .map_err(|error| NortHingError::tool(format!("Failed to load DeepReview execution policy: {}", error)))?;
    let mut run_manifest = context.custom_data.get("deep_review_run_manifest").cloned();
    if let Some(workspace) = context.workspace.as_ref() {
        let session_storage_path = workspace.session_storage_path();
        match coordinator
            .session_manager()
            .load_session_metadata(&session_storage_path, &inputs.session_id)
            .await
        {
            Ok(Some(metadata)) => {
                if run_manifest.is_none() {
                    run_manifest = metadata.deep_review_run_manifest;
                }
                if let Some(run_manifest) = run_manifest.as_mut() {
                    attach_deep_review_cache(run_manifest, metadata.deep_review_cache);
                }
            }
            Ok(None) => {}
            Err(error) => {
                warn!(
                    "Failed to load DeepReview session metadata for run-manifest policy: session_id={}, error={}",
                    inputs.session_id, error
                );
            }
        }
    }
    let policy = if let Some(manifest) = run_manifest.as_ref() {
        base_policy.with_run_manifest_execution_policy(manifest)
    } else {
        base_policy
    };

    let role = policy.classify_subagent(subagent_type).map_err(|violation| {
        NortHingError::tool(format!(
            "DeepReview Task policy violation: {}",
            violation.to_tool_error_message()
        ))
    })?;

    if inputs.requested_auto_retry && !inputs.is_retry {
        return Err(NortHingError::tool(
            "auto_retry requires retry=true for DeepReview Task calls".to_string(),
        ));
    }
    if let Some(gate) = run_manifest.as_ref().and_then(DeepReviewRunManifestGate::from_value) {
        gate.ensure_active(subagent_type).map_err(|violation| {
            NortHingError::tool(format!(
                "DeepReview Task policy violation: {}",
                violation.to_tool_error_message()
            ))
        })?;
    }
    let conc_policy = policy.concurrency_policy_from_manifest(run_manifest.as_ref().unwrap_or(&Value::Null));

    let mut ctx = DeepReviewContext {
        effective_policy: Some(policy.clone()),
        concurrency_policy: Some(conc_policy.clone()),
        subagent_role: Some(role),
        ..Default::default()
    };

    if inputs.is_retry && role == DeepReviewSubagentRole::Reviewer {
        ctx.retry_scope_files = Some(
            match ensure_deep_review_retry_coverage(raw_input, subagent_type, run_manifest.as_ref()) {
                Ok(retry_scope_files) => retry_scope_files,
                Err(violation) => {
                    if inputs.is_auto_retry {
                        record_deep_review_runtime_auto_retry_suppressed(
                            &inputs.dialog_turn_id,
                            auto_retry_suppression_reason(violation.code),
                        );
                    }
                    return Err(NortHingError::tool(format!(
                        "DeepReview Task policy violation: {}",
                        violation.to_tool_error_message()
                    )));
                }
            },
        );
        if inputs.is_auto_retry {
            ensure_deep_review_auto_retry_allowed(&conc_policy, &inputs.dialog_turn_id).map_err(|violation| {
                record_deep_review_runtime_auto_retry_suppressed(
                    &inputs.dialog_turn_id,
                    auto_retry_suppression_reason(violation.code),
                );
                NortHingError::tool(format!(
                    "DeepReview Task policy violation: {}",
                    violation.to_tool_error_message()
                ))
            })?;
        }
    }

    let is_readonly = agent_registry()
        .get_subagent_is_readonly(subagent_type)
        .unwrap_or(false);
    if !is_readonly {
        return Err(NortHingError::tool(format!(
            "DeepReview Task policy violation: {}",
            serde_json::json!({
                "code": "deep_review_subagent_not_readonly",
                "message": format!(
                    "DeepReview review-phase subagent '{}' must be read-only",
                    subagent_type
                )
            })
        )));
    }
    let is_review = agent_registry().get_subagent_is_review(subagent_type).unwrap_or(false);
    if !is_review {
        return Err(NortHingError::tool(format!(
            "DeepReview Task policy violation: {}",
            serde_json::json!({
                "code": "deep_review_subagent_not_review",
                "message": format!(
                    "DeepReview review-phase subagent '{}' must be marked for review",
                    subagent_type
                )
            })
        )));
    }

    let timeout_seconds = policy.effective_timeout_seconds(role, timeout_seconds_in);

    // Reviewer admission gate
    if role == DeepReviewSubagentRole::Reviewer && !inputs.is_retry {
        if let Some(cache_hit) = deep_review_task_adapter::deep_review_incremental_cache_hit_for_task(
            subagent_type,
            inputs.description.as_deref(),
            run_manifest.as_ref(),
        ) {
            let (_data, _cached_result) =
                deep_review_task_adapter::deep_review_incremental_cache_hit_result(subagent_type, &cache_hit);
            // Surface cache hit via ctx flag — facade decides whether to short-circuit.
            ctx.launch_batch_info = None;
        }
    }

    match role {
        DeepReviewSubagentRole::Reviewer => {
            ctx.reviewer_configured_max_parallel_instances = Some(conc_policy.max_parallel_instances);
            let effective_parallel_instances =
                deep_review_effective_parallel_instances(&inputs.dialog_turn_id, conc_policy.max_parallel_instances);
            let is_optional_reviewer = policy.extra_subagent_ids.iter().any(|id| id == subagent_type);
            ctx.is_optional_reviewer = is_optional_reviewer;
            ctx.launch_batch_info =
                deep_review_launch_batch_for_task(subagent_type, inputs.description.as_deref(), run_manifest.as_ref());
            match try_begin_deep_review_reviewer_admission(
                &inputs.dialog_turn_id,
                effective_parallel_instances,
                ctx.launch_batch_info.as_ref(),
            ) {
                Ok(Some(guard)) => {
                    ctx.active_guard = Some(guard);
                }
                Ok(None)
                | Err(DeepReviewPolicyViolation {
                    code: "deep_review_launch_batch_blocked",
                    ..
                }) => {
                    match wait_for_deep_review_reviewer_admission(
                        &inputs.session_id,
                        &inputs.dialog_turn_id,
                        &inputs.tool_call_id,
                        subagent_type,
                        &conc_policy,
                        is_optional_reviewer,
                        ctx.launch_batch_info.as_ref(),
                    )
                    .await?
                    {
                        DeepReviewQueueWaitOutcome::Ready { guard } => {
                            ctx.active_guard = Some(guard);
                        }
                        DeepReviewQueueWaitOutcome::Skipped { .. } => {
                            // Surfaced to facade via flag — facade returns skip result.
                            ctx.active_guard = None;
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
        }
        DeepReviewSubagentRole::Judge => {
            let active_reviewers = deep_review_active_reviewer_count(&inputs.dialog_turn_id);
            let judge_pending = deep_review_has_judge_been_launched(&inputs.dialog_turn_id);
            conc_policy
                .check_launch_allowed(active_reviewers, role, judge_pending)
                .map_err(|violation| {
                    NortHingError::tool(format!(
                        "DeepReview concurrency policy violation: {}",
                        violation.to_tool_error_message()
                    ))
                })?;
        }
    }

    record_deep_review_task_budget(&inputs.dialog_turn_id, &policy, role, subagent_type, inputs.is_retry).map_err(
        |violation| {
            if inputs.is_auto_retry {
                record_deep_review_runtime_auto_retry_suppressed(
                    &inputs.dialog_turn_id,
                    auto_retry_suppression_reason(violation.code),
                );
            }
            NortHingError::tool(format!(
                "DeepReview Task policy violation: {}",
                violation.to_tool_error_message()
            ))
        },
    )?;

    if inputs.is_retry && role == DeepReviewSubagentRole::Reviewer {
        if inputs.is_auto_retry {
            record_deep_review_runtime_auto_retry(&inputs.dialog_turn_id);
        } else {
            record_deep_review_runtime_manual_retry(&inputs.dialog_turn_id);
        }
    }

    // Build subagent_context_map for downstream phases
    let mut values = std::collections::HashMap::new();
    values.insert(
        "deep_review_subagent_role".to_string(),
        match role {
            DeepReviewSubagentRole::Reviewer => "reviewer",
            DeepReviewSubagentRole::Judge => "judge",
        }
        .to_string(),
    );
    if let Some(st) = inputs.subagent_type.as_ref() {
        values.insert("deep_review_subagent_type".to_string(), st.clone());
    }
    ctx.subagent_context_map = Some(values);

    Ok(Some((ctx, timeout_seconds)))
}
