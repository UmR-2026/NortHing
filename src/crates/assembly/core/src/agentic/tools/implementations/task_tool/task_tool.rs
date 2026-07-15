//! Task tool — facade (Round 12 split)
//!
//! Owns:
//! - `TaskTool` struct + `impl Default` + slim `impl TaskTool` (tool_core + delegation)
//! - `impl Tool for TaskTool` (full Tool trait impl)
//! - `call_impl` god method — now a thin orchestrator that delegates to sibling helpers
//! - 6 facade-level tests
//!
//! call_impl phase split (R7 turn_internal pattern):
//! - Phase 1 input prep → `task_tool_input::prepare_call_inputs`
//! - Phase 2 DeepReview setup → `task_tool_deep_review::setup_deep_review_for_call`
//! - Phase 3 background dispatch → `task_tool_subagent::dispatch_background_subagent`
//! - Phase 4 main execution loop → `task_tool_subagent::execute_subagent_loop`
//! - Phase 5 completion result → `task_tool_agents::build_completion_result`
//!
//! Spec: `docs/handoffs/2026-06-29-round12-task-tool-split-spec.md` (f0f9bc0).

use super::task_tool_agents::get_agents_types_impl;
use super::task_tool_input::{prepare_call_inputs, validate_task_input, CallInputs};
use super::task_tool_subagent::{dispatch_background_subagent, execute_subagent_loop, ExecuteOutcome};
use crate::agentic::tools::framework::{
    Tool, ToolExposure, ToolRenderOptions, ToolResult, ToolUseContext, ValidationResult,
};
use crate::service::config::global::GlobalConfigManager;
use crate::service::config::types::AIConfig;
use crate::util::errors::{NortHingError, NortHingResult};
use async_trait::async_trait;
use northhing_runtime_ports::SubagentContextMode;
use serde_json::{json, Value};
use std::time::Instant;

pub struct TaskTool;

impl Default for TaskTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskTool {
    pub fn new() -> Self {
        Self
    }

    /// Block nested subagent delegation.
    fn ensure_delegation_allowed(context: &ToolUseContext) -> NortHingResult<()> {
        let delegation_policy = context.delegation_policy();
        if delegation_policy.allow_subagent_spawn {
            return Ok(());
        }

        Err(NortHingError::tool(
            "Recursive subagent delegation is blocked. Use direct tools instead.".to_string(),
        ))
    }

    async fn load_configured_tool_execution_timeout() -> Option<u64> {
        let service = GlobalConfigManager::service().await.ok()?;
        let ai_config: AIConfig = service.config(Some("ai")).await.ok()?;
        ai_config.tool_execution_timeout_secs.filter(|seconds| *seconds > 0)
    }

    fn resolve_subagent_timeout_seconds(
        requested_timeout_seconds: Option<u64>,
        configured_execution_timeout_secs: Option<u64>,
    ) -> Option<u64> {
        match (
            requested_timeout_seconds.filter(|seconds| *seconds > 0),
            configured_execution_timeout_secs.filter(|seconds| *seconds > 0),
        ) {
            (Some(requested), Some(configured)) => Some(requested.max(configured)),
            (Some(requested), None) => Some(requested),
            (None, Some(configured)) => Some(configured),
            (None, None) => None,
        }
    }

    /// Sibling-exposed helper so `task_tool_input::prepare_call_inputs` can call
    /// back into facade-owned `get_agents_types` without depending on the
    /// `impl Tool for TaskTool` block.
    pub(super) async fn get_agents_types_impl_pub(&self, context: Option<&ToolUseContext>) -> Vec<String> {
        get_agents_types_impl(self, context).await
    }

    /// Backwards-compatible facade wrapper. Some callers (e.g. `turn_lifecycle`)
    /// call `TaskTool::build_available_agents_context_section(...)` as a static
    /// method. Delegate to the sibling free fn.
    pub(crate) async fn build_available_agents_context_section(context: Option<&ToolUseContext>) -> Option<String> {
        super::task_tool_agents::build_available_agents_context_section(context).await
    }

    /// Render the prompt-template `description` text. Inlined from original
    /// `TaskTool::render_description`.
    fn render_description(&self) -> String {
        r#"Launch a new agent to handle complex, multi-step tasks autonomously.

The Task tool launches specialized agents (subprocesses) that autonomously handle complex tasks. Each agent type has specific capabilities and tools available to it.

The current agent listing includes an <available_agents> section when subagents are available. Use the exact `type` attribute from that section as `subagent_type` when `fork_context` is false or omitted.

Supported context behaviors:
- `fork_context=false` (default): start a new subagent context from scratch. You must provide `subagent_type`. In this mode, include the necessary background information in the prompt.
- `fork_context=true`: start an isolated child session that inherits your current context and tools. Do not provide `subagent_type`, `workspace_path`, or `model_id` in this mode. Here the prompt is a directive: what to do, not what the situation is. Be specific about scope: what's in, what's out, and what another agent is handling. Don't re-explain background.

Do not put `fork_context`, `subagent_type`, `description`, `workspace_path`, `model_id`, or `timeout_seconds` inside the prompt string.

When to use the Task tool:
- Delegate when a specialized subagent or separate context is likely to improve coverage, independence, or parallelism.
- Use direct tools instead for focused lookups, known paths, single symbols, or code that can be inspected in a few reads/searches.

Usage notes:
- Include a short description summarizing what the agent will do.
- Provide a clear prompt so the agent can work autonomously and return the information you need.
- When `fork_context` is false, if 'workspace_path' is omitted, the task inherits the current workspace by default.
- When `fork_context` is false, provide 'workspace_path' when the selected agent requires an explicit workspace.
- When `fork_context` is false, use 'model_id' when a caller needs a specific model or model slot for the subagent. Omit it to use the agent default.
- When `fork_context` is true, the child session reuses the parent session's agent type, workspace, and prompt cache while still running in isolation.
- Use 'timeout_seconds' when you need a hard deadline for the subagent. When omitted, the session execution timeout from settings is used. When provided, the effective timeout is the larger of the requested value and the session execution timeout. Set it to 0 with no configured session execution timeout to disable the timeout.
- For DeepReview only, set 'retry' to true when re-dispatching a reviewer after that same reviewer returned partial_timeout or an explicit transient capacity failure in the current turn. Retry calls must include retry_coverage with source_packet_id, source_status, covered_files, and a smaller retry_scope_files list. Do not set 'auto_retry' unless this is a backend-owned automatic retry admitted by Review Team settings; model-issued retry decisions should omit it or set it to false. Example retry_coverage: {{ "source_packet_id": "reviewer-123", "source_status": "partial_timeout", "covered_files": ["src/main.rs"], "retry_scope_files": ["src/parser.rs"] }}.
- Launch independent agents concurrently when that improves coverage or latency; send parallel Task calls in a single assistant message.
- When the agent is done, it will return a single message back to you.
- Treat subagent outputs as useful evidence, but verify details yourself before making edits or final claims that depend on exact code.
- Clearly tell the agent whether you expect it to write code or just to do research (search, file reads, web fetches, etc.), since it is not aware of the user's intent.
- If the agent description mentions proactive use, consider it when relevant and use your judgement.
- If the user explicitly asks to run agents in parallel, send the independent Task calls together in one message."#
            .to_string()
    }
}

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &str {
        "Task"
    }

    fn manages_own_execution_timeout(&self) -> bool {
        true
    }

    async fn description(&self) -> NortHingResult<String> {
        Ok(self.render_description())
    }

    async fn is_available_in_context(&self, _context: Option<&ToolUseContext>) -> bool {
        // Keep Task prompt-visible even when no fresh subagents are currently
        // available. Hiding it based on transient subagent availability makes
        // the tool manifest drift across turns and causes provider prefix/KV
        // cache misses. Task also still supports `fork_context=true` in that
        // state, so removing it from the manifest would be behaviorally wrong.
        true
    }

    fn default_exposure(&self) -> ToolExposure {
        // Task is a meta-tool with a 30+-field input schema (~800-1,200 tokens
        // in the manifest). Most turns do not use Task, so the full schema
        // is wasteful in the default manifest. Collapsed = stub-by-default;
        // the model calls `GetToolSpec(tool_name="Task")` on first use to
        // fetch the full schema (standard collapsed-tool workflow, validated
        // by `validate_collapsed_tool_usage`).
        //
        // Spec: docs/superpowers/specs/2026-06-23-collapse-task-tool-design.md
        ToolExposure::Collapsed
    }

    fn short_description(&self) -> String {
        "Delegate work to a subagent task and collect the result.".to_string()
    }

    async fn description_with_context(&self, _context: Option<&ToolUseContext>) -> NortHingResult<String> {
        Ok(self.render_description())
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "A short (3-5 word) description of the task"
                },
                "prompt": {
                    "type": "string",
                    "description": "The task for the agent to perform. Keep it scoped and concise. Do not include top-level Task arguments such as fork_context or subagent_type inside this string. The 180-line / 16KB guideline is a soft reliability threshold, not a hard cap. For large delegations, split into multiple Task calls with clear ownership, and pass file paths, symbols, constraints, and exact questions instead of pasting large file contents."
                },
                "fork_context": {
                    "type": "boolean",
                    "default": false,
                    "description": "Optional. Defaults to false. Set true to fork the parent session into an isolated child session that reuses the parent agent type, latest runtime context, and prompt cache. Leave false to launch a specialized fresh subagent chosen by subagent_type."
                },
                "subagent_type": {
                    "type": "string",
                    "description": "Required top-level agent type id when fork_context is false or omitted."
                },
                "workspace_path": {
                    "type": "string",
                    "description": "Only used when fork_context is false. The absolute path of the workspace for this task. If omitted, inherits the current workspace."
                },
                "model_id": {
                    "type": "string",
                    "description": "Only used when fork_context is false. Optional model ID or model slot alias for this subagent task. Omit it to use the agent default."
                },
                "timeout_seconds": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Optional timeout for this subagent task in seconds. When omitted, the session execution timeout from settings is used. When provided, the effective timeout is the larger of this value and the session execution timeout. Use 0 with no configured session execution timeout to disable the timeout."
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Optional. When true, start the subagent in the background and return immediately. The final result will be delivered back to the parent agent by steering if it is still running, or by starting a new turn if it is idle."
                },
                "retry": {
                    "type": "boolean",
                    "description": "DeepReview only: true when this Task call is a retry for the same reviewer role after partial_timeout or an explicit transient capacity failure in the current turn."
                },
                "auto_retry": {
                    "type": "boolean",
                    "description": "DeepReview only: true only for backend-owned bounded automatic retries. Requires Review Team auto retry opt-in and retry=true. User/model-issued retry actions must omit this field or set it to false."
                },
                "retry_coverage": {
                    "type": "object",
                    "description": "DeepReview retry only: structured coverage metadata proving the retry is bounded. Required when retry=true.",
                    "properties": {
                        "source_packet_id": {
                            "type": "string",
                            "description": "The original reviewer packet_id being retried."
                        },
                        "source_status": {
                            "type": "string",
                            "enum": ["partial_timeout", "capacity_skipped"],
                            "description": "The retryable source status."
                        },
                        "capacity_reason": {
                            "type": "string",
                            "description": "Required for capacity_skipped; must be a transient capacity reason such as local_concurrency_cap, launch_batch_blocked, provider_rate_limit, provider_concurrency_limit, retry_after, or temporary_overload."
                        },
                        "covered_files": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Files already covered by the source attempt."
                        },
                        "retry_scope_files": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Smaller file list to retry. Every entry must belong to the source packet and must not overlap covered_files."
                        }
                    },
                    "required": [
                        "source_packet_id",
                        "source_status",
                        "covered_files",
                        "retry_scope_files"
                    ]
                }
            },
            "required": [
                "description",
                "prompt"
            ],
            "additionalProperties": false
        })
    }

    fn is_readonly(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self, input: Option<&Value>) -> bool {
        // is_concurrency_safe must be synchronous; we cannot await validate_task_input
        // here. Reproduce the original sync check: if `fork_context` is true,
        // concurrency is unsafe (the subagent inherits mutable parent state).
        if let Some(value) = input {
            if let Some(fork_context) = value.get("fork_context").and_then(|v| v.as_bool()) {
                if fork_context {
                    return false;
                }
            }
        }
        let subagent_type = input.and_then(|v| v.get("subagent_type")).and_then(|v| v.as_str());
        match subagent_type {
            Some(id) => crate::agentic::agents::agent_registry()
                .get_subagent_is_readonly(id)
                .unwrap_or(false),
            None => false,
        }
    }

    fn needs_permissions(&self, _input: Option<&Value>) -> bool {
        false
    }

    async fn validate_input(&self, input: &Value, context: Option<&ToolUseContext>) -> ValidationResult {
        validate_task_input(input, context).await
    }

    fn render_tool_use_message(&self, input: &Value, options: &ToolRenderOptions) -> String {
        if let Some(description) = input.get("description").and_then(|v| v.as_str()) {
            if options.verbose {
                format!("Creating task: {}", description)
            } else {
                format!("Task: {}", description)
            }
        } else {
            "Creating task".to_string()
        }
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        let start_time = Instant::now();

        Self::ensure_delegation_allowed(context)?;

        // Phase 1: input prep.
        let inputs = prepare_call_inputs(self, input, context).await?;

        // Phase 2: DeepReview setup (returns Some only if is_deep_review_parent).
        let mut dr_ctx = super::task_tool_subagent::DeepReviewContext::default();
        let mut timeout_seconds = inputs.timeout_seconds;
        let dr_setup = super::task_tool_deep_review::setup_deep_review_for_call(
            &inputs,
            input,
            context,
            timeout_seconds,
            start_time,
        )
        .await?;
        if let Some((ctx, ts)) = dr_setup {
            dr_ctx = ctx;
            timeout_seconds = ts;
        } else {
            // Not DeepReview parent: resolve timeout via configured execution timeout.
            let configured_timeout = Self::load_configured_tool_execution_timeout().await;
            timeout_seconds = Self::resolve_subagent_timeout_seconds(timeout_seconds, configured_timeout);
        }

        // Cache hit short-circuit (deep_review incremental cache).
        if let Some(cached) = dr_ctx.cache_hit_result.take() {
            return Ok(vec![cached]);
        }

        let mut prepared_prompt = inputs.prompt.clone();
        if let Some(retry_scope_files) = dr_ctx.retry_scope_files.as_ref() {
            prepared_prompt =
                super::task_tool_deep_review::prompt_with_deep_review_retry_scope(&prepared_prompt, retry_scope_files);
        }

        // Phase 3: background dispatch.
        if inputs.run_in_background {
            let subagent_context_map = dr_ctx.subagent_context_map.clone().unwrap_or_default();
            return dispatch_background_subagent(
                &inputs,
                context,
                prepared_prompt,
                timeout_seconds,
                subagent_context_map,
            )
            .await;
        }

        // Phase 4: main execution loop with provider capacity retry handling.
        let outcome = execute_subagent_loop(
            &inputs,
            context,
            &mut dr_ctx,
            prepared_prompt,
            timeout_seconds,
            start_time,
        )
        .await?;

        // Phase 5: completion result or early-exit ToolResult.
        match outcome {
            ExecuteOutcome::Success(result) => {
                let deep_review_subagent_id = inputs.subagent_type.as_deref().unwrap_or("");
                let should_emit_retry_guidance = super::task_tool_deep_review::should_emit_deep_review_retry_guidance(
                    result.is_partial_timeout(),
                    inputs.is_retry,
                    dr_ctx.subagent_role,
                );
                let tool_result = super::task_tool_agents::build_completion_result(
                    &result.text,
                    result.is_partial_timeout(),
                    result.reason.as_deref(),
                    result.ledger_event_id(),
                    &inputs,
                    dr_ctx.effective_policy.as_ref(),
                    deep_review_subagent_id,
                    dr_ctx.subagent_role,
                    &inputs.delegate_target_label,
                    start_time.elapsed().as_millis(),
                    should_emit_retry_guidance,
                    inputs.is_retry,
                );
                Ok(vec![tool_result])
            }
            ExecuteOutcome::CancelledReviewer(r)
            | ExecuteOutcome::ProviderCapacitySkip(r)
            | ExecuteOutcome::LocalCapacitySkip(r) => Ok(vec![r]),
        }
    }
}

// `CallInputs` is re-exported for sibling visibility (unused here, but ensures
// the type stays reachable from `super::task_tool::*`).
#[allow(dead_code)]
type _CallInputsRef = CallInputs;

#[cfg(test)]
mod tests {
    use super::TaskTool;
    use crate::agentic::tools::framework::{Tool, ToolUseContext};
    use crate::agentic::tools::ToolRuntimeRestrictions;
    use northhing_runtime_ports::{DelegationPolicy, SubagentContextMode};
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn task_prompt_guidance_omits_subagent_name_examples() {
        let description = TaskTool::new().render_description();
        assert!(!description.contains("subagent_type=\"Explore\""));
        assert!(!description.contains("subagent_type=\"FileFinder\""));
        assert!(!description.contains("For Explore"));
        assert!(!description.contains("Explore/FileFinder"));
        assert!(!description.contains("file-discovery"));
        assert!(!description.contains("listed investigation"));

        let schema = TaskTool::new().input_schema();
        let subagent_description = schema["properties"]["subagent_type"]["description"]
            .as_str()
            .expect("subagent_type description should be a string");
        assert!(!subagent_description.contains("Explore"));
        assert!(!subagent_description.contains("FileFinder"));
        assert!(!subagent_description.contains("available_agents"));
    }

    #[test]
    fn task_schema_accepts_optional_model_id() {
        let schema = TaskTool::new().input_schema();

        assert_eq!(schema["properties"]["model_id"]["type"], "string");
        assert!(!schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("model_id")));
    }

    #[test]
    fn task_schema_supports_fork_context_flag() {
        let schema = TaskTool::new().input_schema();

        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["properties"]["fork_context"]["type"], "boolean");
        assert_eq!(schema["properties"]["fork_context"]["default"], false);
        assert!(schema["properties"]["subagent_type"]["description"]
            .as_str()
            .unwrap()
            .contains("fork_context is false or omitted"));
        assert!(schema["properties"]["prompt"]["description"]
            .as_str()
            .unwrap()
            .contains("Do not include top-level Task arguments"));
        assert!(schema.get("allOf").is_none());
    }

    #[tokio::test]
    async fn task_tool_stays_available_without_enabled_subagents() {
        assert!(
            TaskTool::new().is_available_in_context(None).await,
            "Task should remain prompt-visible even when no fresh subagents are currently available"
        );
    }

    #[test]
    fn resolve_subagent_timeout_uses_session_execution_timeout_as_floor() {
        assert_eq!(
            TaskTool::resolve_subagent_timeout_seconds(Some(300), Some(1200)),
            Some(1200)
        );
        assert_eq!(TaskTool::resolve_subagent_timeout_seconds(None, Some(1200)), Some(1200));
        assert_eq!(
            TaskTool::resolve_subagent_timeout_seconds(Some(1800), Some(1200)),
            Some(1800)
        );
        assert_eq!(TaskTool::resolve_subagent_timeout_seconds(Some(300), None), Some(300));
        assert_eq!(TaskTool::resolve_subagent_timeout_seconds(None, None), None);
    }

    #[tokio::test]
    async fn call_impl_rejects_nested_subagent_delegation() {
        let policy = DelegationPolicy::top_level().spawn_child();
        let context = ToolUseContext {
            tool_call_id: Some("tool-call-1".to_string()),
            agent_type: Some("agentic".to_string()),
            session_id: Some("session-1".to_string()),
            dialog_turn_id: Some("turn-1".to_string()),
            workspace: None,
            unlocked_collapsed_tools: Vec::new(),
            custom_data: HashMap::from([
                (
                    "delegation_allow_subagent_spawn".to_string(),
                    json!(policy.allow_subagent_spawn),
                ),
                ("delegation_nesting_depth".to_string(), json!(policy.nesting_depth)),
            ]),
            computer_use_host: None,
            runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
            runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
            actor_runtime: None,
        };

        let error = TaskTool::new()
            .call_impl(
                &json!({
                    "description": "delegate",
                    "prompt": "Inspect the repo",
                    "subagent_type": "Explore"
                }),
                &context,
            )
            .await
            .expect_err("nested subagent delegation should be rejected");

        assert!(error
            .to_string()
            .contains("Recursive subagent delegation is blocked. Use direct tools instead."));
    }

    /// Spec: docs/superpowers/specs/2026-06-23-collapse-task-tool-design.md
    #[test]
    fn task_tool_default_exposure_is_collapsed() {
        let tool = TaskTool::new();
        assert_eq!(
            tool.default_exposure(),
            crate::agentic::tools::framework::ToolExposure::Collapsed,
            "TaskTool should be Collapsed so the manifest saves ~1K tokens/turn. \
             If this fails after a deliberate revert, update this assertion + the spec."
        );
    }

    // Marker to silence unused-import for `SubagentContextMode` (re-exported via
    // sibling modules, kept for parity with original TaskTool).
    #[allow(dead_code)]
    fn _type_marker(_mode: SubagentContextMode) {}

    // Marker for unused const imports from task_tool_input.
    // (Removed: consts are only referenced via test setup; no marker needed.)
}
