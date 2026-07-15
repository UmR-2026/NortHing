//! Task tool — input validation + call_impl input-prep phase (Round 12 split)
//!
//! Owns 5 input-validation fns + 2 tests + `prepare_call_inputs` helper extracted from
//! `call_impl` god method (R7 turn_internal pattern).
//!
//! Spec: `docs/handoffs/2026-06-29-round12-task-tool-split-spec.md` (f0f9bc0).

use super::TaskTool;
use crate::agentic::tools::framework::{ToolUseContext, ValidationResult};
use crate::agentic::tools::InputValidator;
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_runtime_ports::SubagentContextMode;
use serde_json::{json, Value};

/// Soft reliability thresholds for large Task prompts. Shared with facade via `pub(super)`
/// so the input validator + Tool trait `validate_input` impl can reference them.
pub(super) const LARGE_TASK_PROMPT_SOFT_LINE_LIMIT: usize = 180;
pub(super) const LARGE_TASK_PROMPT_SOFT_BYTE_LIMIT: usize = 16 * 1024;

/// Input validation helpers — moved verbatim from original `impl TaskTool` block.

pub(super) fn context_mode_from_input(input: &Value) -> NortHingResult<SubagentContextMode> {
    match input.get("fork_context") {
        None => Ok(SubagentContextMode::Fresh),
        Some(value) => {
            let fork_context = value
                .as_bool()
                .ok_or_else(|| NortHingError::tool("fork_context must be a boolean".to_string()))?;
            Ok(if fork_context {
                SubagentContextMode::Fork
            } else {
                SubagentContextMode::Fresh
            })
        }
    }
}

pub(super) fn invalid_input(message: impl Into<String>) -> ValidationResult {
    ValidationResult {
        result: false,
        message: Some(message.into()),
        error_code: None,
        meta: None,
    }
}

pub(super) fn is_deep_review_auto_retry(input: &Value) -> bool {
    input.get("auto_retry").and_then(Value::as_bool).unwrap_or(false)
}

pub(super) async fn validate_task_input(input: &Value, _context: Option<&ToolUseContext>) -> ValidationResult {
    let validation = InputValidator::new(input)
        .validate_required("description")
        .validate_required("prompt")
        .finish();
    if !validation.result {
        return validation;
    }

    let context_mode = match context_mode_from_input(input) {
        Ok(mode) => mode,
        Err(error) => return invalid_input(error.to_string()),
    };

    match context_mode {
        SubagentContextMode::Fresh => {
            if input.get("subagent_type").is_none() {
                return invalid_input("subagent_type is required when fork_context is false or omitted");
            }
        }
        SubagentContextMode::Fork => {
            for field in [
                "subagent_type",
                "workspace_path",
                "model_id",
                "retry",
                "auto_retry",
                "retry_coverage",
            ] {
                if input.get(field).is_some() {
                    return invalid_input(format!("{field} is not allowed when fork_context is true"));
                }
            }
        }
    }

    if let Some(prompt) = input.get("prompt").and_then(|value| value.as_str()) {
        let line_count = prompt.lines().count();
        let byte_count = prompt.len();
        if line_count > LARGE_TASK_PROMPT_SOFT_LINE_LIMIT || byte_count > LARGE_TASK_PROMPT_SOFT_BYTE_LIMIT {
            return ValidationResult {
                result: true,
                message: Some(format!(
                    "Large Task prompt: {} lines, {} bytes. This is allowed when necessary, but prefer staged delegation: split large work into multiple Task calls with clear ownership, and pass file paths, symbols, constraints, and exact questions instead of large pasted context.",
                    line_count, byte_count
                )),
                error_code: None,
                meta: Some(json!({
                    "large_task_prompt": true,
                    "line_count": line_count,
                    "byte_count": byte_count,
                    "soft_line_limit": LARGE_TASK_PROMPT_SOFT_LINE_LIMIT,
                    "soft_byte_limit": LARGE_TASK_PROMPT_SOFT_BYTE_LIMIT
                })),
            };
        }
    }

    validation
}

/// Prepared inputs for `call_impl` orchestrator. Built by `prepare_call_inputs`, then
/// passed to deep_review setup → subagent dispatch/execution → completion.
pub(super) struct CallInputs {
    pub description: Option<String>,
    pub prompt: String,
    pub context_mode: SubagentContextMode,
    pub subagent_type: Option<String>,
    pub requested_workspace_path: Option<String>,
    pub model_id: Option<String>,
    pub timeout_seconds: Option<u64>,
    pub run_in_background: bool,
    pub is_retry: bool,
    pub requested_auto_retry: bool,
    pub is_auto_retry: bool,
    pub current_workspace_path: Option<String>,
    pub effective_workspace_path: Option<String>,
    pub delegate_target_label: String,
    pub session_id: String,
    pub tool_call_id: String,
    pub dialog_turn_id: String,
}

/// call_impl Phase 1: input extraction + validation + workspace resolution.
/// Extracted from call_impl lines 701-887 of original task_tool.rs.
pub(super) async fn prepare_call_inputs(
    self_: &TaskTool,
    input: &Value,
    context: &ToolUseContext,
) -> NortHingResult<CallInputs> {
    // description is only used for frontend display
    let description = input.get("description").and_then(Value::as_str).map(str::to_string);

    let mut prompt = input
        .get("prompt")
        .and_then(|v| v.as_str())
        .ok_or_else(|| NortHingError::tool("Required parameters: prompt and description. Missing prompt".to_string()))?
        .to_string();
    let context_mode = context_mode_from_input(input)?;

    let subagent_type = match context_mode {
        SubagentContextMode::Fresh => {
            let subagent_type = input
                .get("subagent_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    NortHingError::tool("subagent_type is required when fork_context is false or omitted".to_string())
                })?
                .to_string();
            let all_agent_types = self_.get_agents_types_impl_pub(Some(context)).await;
            if !all_agent_types.contains(&subagent_type) {
                return Err(NortHingError::tool(format!(
                    "subagent_type {} is not valid, must be one of: {}",
                    subagent_type,
                    all_agent_types.join(", ")
                )));
            }
            Some(subagent_type)
        }
        SubagentContextMode::Fork => None,
    };
    let delegate_target_label = match subagent_type.as_deref() {
        Some(subagent_type) => format!("subagent '{}'", subagent_type),
        None => "forked subagent".to_string(),
    };

    let requested_workspace_path = input
        .get("workspace_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let model_id = match input.get("model_id") {
        Some(value) => {
            let value = value
                .as_str()
                .ok_or_else(|| NortHingError::tool("model_id must be a string".to_string()))?;
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_string())
        }
        None => None,
    };
    let mut timeout_seconds = match input.get("timeout_seconds") {
        Some(value) => {
            let parsed = value
                .as_u64()
                .ok_or_else(|| NortHingError::tool("timeout_seconds must be a non-negative integer".to_string()))?;
            (parsed > 0).then_some(parsed)
        }
        None => None,
    };
    let run_in_background = input.get("run_in_background").and_then(Value::as_bool).unwrap_or(false);
    let is_retry = input.get("retry").and_then(Value::as_bool).unwrap_or(false);
    let requested_auto_retry = is_deep_review_auto_retry(input);
    let is_auto_retry = is_retry && requested_auto_retry;
    let current_workspace_path = context.workspace_root().map(|path| path.to_string_lossy().into_owned());
    if context_mode == SubagentContextMode::Fork {
        if requested_workspace_path.is_some() {
            return Err(NortHingError::tool(
                "workspace_path is not allowed when fork_context is true".to_string(),
            ));
        }
        if model_id.is_some() {
            return Err(NortHingError::tool(
                "model_id is not allowed when fork_context is true".to_string(),
            ));
        }
        if is_retry || requested_auto_retry || input.get("retry_coverage").is_some() {
            return Err(NortHingError::tool(
                "DeepReview retry fields are not allowed when fork_context is true".to_string(),
            ));
        }
    }
    let effective_workspace_path = if let Some(subagent_type) = subagent_type.as_deref() {
        if subagent_type == "Explore" || subagent_type == "FileFinder" {
            let workspace_path = requested_workspace_path
                .as_deref()
                .or(current_workspace_path.as_deref())
                .ok_or_else(|| {
                    NortHingError::tool("workspace_path is required for Explore/FileFinder agent".to_string())
                })?;

            if workspace_path.is_empty() {
                return Err(NortHingError::tool(
                    "workspace_path cannot be empty for Explore/FileFinder agent".to_string(),
                ));
            }

            // For remote workspaces, skip local filesystem validation - the path
            // exists on the remote server, not locally.
            if !context.is_remote() {
                let path = std::path::Path::new(&workspace_path);
                if !path.exists() {
                    return Err(NortHingError::tool(format!(
                        "workspace_path '{}' does not exist",
                        workspace_path
                    )));
                }
                if !path.is_dir() {
                    return Err(NortHingError::tool(format!(
                        "workspace_path '{}' is not a directory",
                        workspace_path
                    )));
                }
            }

            prompt.push_str(&format!("\n\nThe workspace you need to explore: {workspace_path}"));
        }

        Some(
            requested_workspace_path
                .clone()
                .or(current_workspace_path.clone())
                .ok_or_else(|| {
                    NortHingError::tool(
                        "workspace_path is required when the current workspace is unavailable".to_string(),
                    )
                })?,
        )
    } else {
        None
    };

    let session_id = if let Some(session_id) = &context.session_id {
        session_id.clone()
    } else {
        return Err(NortHingError::tool("session_id is required in context".to_string()));
    };

    // Get parent tool ID (tool_call_id)
    let tool_call_id = if let Some(tool_id) = &context.tool_call_id {
        tool_id.clone()
    } else {
        return Err(NortHingError::tool("tool_call_id is required in context".to_string()));
    };

    // Get parent dialog turn ID (dialog_turn_id)
    let dialog_turn_id = if let Some(turn_id) = &context.dialog_turn_id {
        turn_id.clone()
    } else {
        return Err(NortHingError::tool("dialog_turn_id is required in context".to_string()));
    };

    Ok(CallInputs {
        description,
        prompt,
        context_mode,
        subagent_type,
        requested_workspace_path,
        model_id,
        timeout_seconds,
        run_in_background,
        is_retry,
        requested_auto_retry,
        is_auto_retry,
        current_workspace_path,
        effective_workspace_path,
        delegate_target_label,
        session_id,
        tool_call_id,
        dialog_turn_id,
    })
}

// Re-export facade-only constants to `super::TaskTool` via `use super::*`
// (not strictly necessary; sibling constants accessed via `super::task_tool_input::CONST`)

#[cfg(test)]
mod tests {
    use super::validate_task_input;
    use crate::agentic::tools::framework::ToolUseContext;
    use serde_json::json;

    #[tokio::test]
    async fn validate_input_requires_subagent_type_when_not_forking() {
        let validation = validate_task_input(
            &json!({
                "description": "delegate",
                "prompt": "Inspect the repo"
            }),
            None,
        )
        .await;

        assert!(!validation.result);
        assert!(validation
            .message
            .as_deref()
            .is_some_and(|message| message.contains("subagent_type is required")));
    }

    #[tokio::test]
    async fn validate_input_rejects_fork_context_conflicting_fields() {
        let validation = validate_task_input(
            &json!({
                "description": "delegate",
                "prompt": "Continue with inherited context",
                "fork_context": true,
                "subagent_type": "Explore"
            }),
            None,
        )
        .await;

        assert!(!validation.result);
        assert!(validation
            .message
            .as_deref()
            .is_some_and(|message| message.contains("subagent_type is not allowed")));
    }

    // Marker to silence unused import warnings when ToolUseContext is referenced
    // only by other tests in the workspace (kept for completeness).
    #[allow(dead_code)]
    fn _type_marker(_ctx: &ToolUseContext) {}
}
