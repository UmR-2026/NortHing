//! Tool pipeline
//!
//! Manages the complete lifecycle of tools:
//! confirmation, execution, caching, retries, etc.

pub mod exec_dispatch;
pub mod exec_parallel;
pub mod exec_retry;
pub mod exec_serial;
pub mod pipeline_logging;
pub mod pipeline_post;
pub mod pipeline_pre;
pub mod pipeline_types;

pub use exec_dispatch::*;
pub use exec_parallel::*;
pub use exec_retry::*;
pub use exec_serial::*;
pub use pipeline_logging::*;
pub use pipeline_post::*;
pub use pipeline_pre::*;
pub use pipeline_types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic::core::{ToolCall, ToolExecutionState};
    use crate::agentic::events::{EventQueue, EventQueueConfig};
    use crate::agentic::tools::framework::{Tool, ToolResult as FrameworkToolResult};
    use crate::agentic::tools::implementations::task_tool::TaskTool;
    use crate::agentic::tools::pipeline::state_manager::ToolStateManager;
    use crate::agentic::tools::registry::ToolRegistry;
    use crate::agentic::tools::{ToolExecutionContext, ToolExecutionOptions, ToolRuntimeRestrictions, ToolTask};
    use crate::util::errors::NortHingError;
    use northhing_agent_tools::{
        build_tool_call_truncation_recovery_notice, validate_tool_execution_admission, ToolExecutionAdmissionRequest,
        GET_TOOL_SPEC_TOOL_NAME, USER_STEERING_INTERRUPTED_MESSAGE,
    };
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::{Arc, OnceLock};
    use tokio::sync::RwLock as TokioRwLock;
    use tokio_util::sync::CancellationToken;

    fn test_tool_pipeline() -> ToolPipeline {
        let registry = Arc::new(TokioRwLock::new(ToolRegistry::new()));
        let event_queue = Arc::new(EventQueue::new(EventQueueConfig::default()));
        let state_manager = Arc::new(ToolStateManager::new(event_queue));
        ToolPipeline::new(registry, state_manager, None, Arc::new(OnceLock::new()))
    }

    fn test_tool_call(tool_id: &str, tool_name: &str) -> ToolCall {
        ToolCall {
            tool_id: tool_id.to_string(),
            tool_name: tool_name.to_string(),
            arguments: json!({ "path": "src/main.rs" }),
            raw_arguments: None,
            is_error: false,
            recovered_from_truncation: false,
        }
    }

    fn test_tool_execution_context() -> ToolExecutionContext {
        ToolExecutionContext {
            session_id: "session_1".to_string(),
            dialog_turn_id: "turn_1".to_string(),
            round_id: "round_1".to_string(),
            agent_type: "agent".to_string(),
            workspace: None,
            context_vars: HashMap::new(),
            subagent_parent_info: None,
            delegation_policy: northhing_runtime_ports::DelegationPolicy::top_level(),
            collapsed_tools: Vec::new(),
            unlocked_collapsed_tools: Vec::new(),
            allowed_tools: Vec::new(),
            runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
            steering_interrupt: None,
            workspace_services: None,
        }
    }

    fn test_tool_task(tool_id: &str, tool_name: &str) -> ToolTask {
        ToolTask::new(
            test_tool_call(tool_id, tool_name),
            test_tool_execution_context(),
            ToolExecutionOptions::default(),
        )
    }

    fn assert_failed_task_contains(pipeline: &ToolPipeline, tool_id: &str, expected: &str) {
        let task = pipeline
            .state_manager
            .get_task(tool_id)
            .unwrap_or_else(|| panic!("{tool_id} task should be retained"));
        match task.state {
            ToolExecutionState::Failed { error, .. } => assert!(
                error.contains(expected),
                "failed task error should contain '{expected}', got '{error}'"
            ),
            state => panic!("expected failed task state, got {state:?}"),
        }
    }

    #[test]
    fn steering_interrupted_result_preserves_tool_call_identity() {
        let task = test_tool_task("tool_1", "Read");
        let result = build_user_steering_interrupted_result("tool_1", Some(task));

        assert_eq!(result.tool_id, "tool_1");
        assert_eq!(result.tool_name, "Read");
        assert!(result.result.is_error);
        assert_eq!(
            result.result.result["category"],
            serde_json::Value::String("user_steering_interrupted".to_string())
        );
        assert_eq!(
            result.result.result_for_assistant.as_deref(),
            Some(USER_STEERING_INTERRUPTED_MESSAGE)
        );
    }

    #[test]
    fn error_result_prefers_raw_arguments_preview_when_available() {
        let mut task = test_tool_task("tool_1", "Git");
        task.tool_call.arguments = json!({});
        task.tool_call.raw_arguments = Some("{\"operation\":\"log\"".to_string());

        let result = build_error_execution_result(
            "tool_1",
            Some(task),
            &NortHingError::Validation("Arguments are invalid JSON.".to_string()),
        );

        assert_eq!(
            result.result.result["provided_arguments"],
            serde_json::Value::String("{\"operation\":\"log\"".to_string())
        );
        assert!(result
            .result
            .result_for_assistant
            .as_deref()
            .unwrap_or_default()
            .contains("Provided arguments: {\"operation\":\"log\""));
    }

    #[tokio::test]
    async fn pipeline_admission_allowed_list_rejection_updates_failed_state_before_registry_lookup() {
        let pipeline = test_tool_pipeline();
        let mut context = test_tool_execution_context();
        context.allowed_tools = vec!["Read".to_string()];

        let results = pipeline
            .execute_tools(
                vec![test_tool_call("tool_1", "UnregisteredBlockedTool")],
                context,
                ToolExecutionOptions::default(),
            )
            .await
            .expect("admission rejection should be returned as an error result");

        assert_eq!(results.len(), 1);
        assert!(results[0].result.is_error);
        assert_failed_task_contains(
            &pipeline,
            "tool_1",
            "Tool 'UnregisteredBlockedTool' is not in the allowed list",
        );
        assert!(
            results[0]
                .result
                .result_for_assistant
                .as_deref()
                .unwrap_or_default()
                .contains("UnregisteredBlockedTool"),
            "error result should preserve rejected tool identity"
        );
    }

    #[tokio::test]
    async fn pipeline_admission_runtime_restriction_rejection_updates_failed_state() {
        let pipeline = test_tool_pipeline();
        let mut context = test_tool_execution_context();
        context
            .runtime_tool_restrictions
            .denied_tool_names
            .insert("Read".to_string());

        let results = pipeline
            .execute_tools(
                vec![test_tool_call("tool_1", "Read")],
                context,
                ToolExecutionOptions::default(),
            )
            .await
            .expect("admission rejection should be returned as an error result");

        assert_eq!(results.len(), 1);
        assert!(results[0].result.is_error);
        assert_failed_task_contains(&pipeline, "tool_1", "Tool 'Read' is denied by runtime restrictions");
    }

    #[tokio::test]
    async fn pipeline_admission_collapsed_tool_rejection_updates_failed_state_before_validation() {
        let pipeline = test_tool_pipeline();
        let mut context = test_tool_execution_context();
        context.collapsed_tools = vec!["WebFetch".to_string()];

        let results = pipeline
            .execute_tools(
                vec![test_tool_call("tool_1", "WebFetch")],
                context,
                ToolExecutionOptions::default(),
            )
            .await
            .expect("admission rejection should be returned as an error result");

        assert_eq!(results.len(), 1);
        assert!(results[0].result.is_error);
        assert_failed_task_contains(
            &pipeline,
            "tool_1",
            "Call GetToolSpec first with {\"tool_name\":\"WebFetch\"}",
        );
    }

    #[test]
    fn fallback_assistant_text_preserves_full_structured_result() {
        let result = convert_tool_result(
            FrameworkToolResult::Result {
                data: json!({
                    "success": false,
                    "exit_code": 1,
                    "working_directory": "/private/tmp",
                    "output": "ERR_PNPM_NO_PKG_MANIFEST"
                }),
                result_for_assistant: None,
                image_attachments: None,
            },
            "tool_1",
            "Bash",
        );

        let assistant_text = result.result_for_assistant.unwrap_or_default();
        assert!(assistant_text.contains("\"success\": false"));
        assert!(assistant_text.contains("\"exit_code\": 1"));
        assert!(assistant_text.contains("\"working_directory\": \"/private/tmp\""));
        assert!(!assistant_text.contains("completed with error"));
    }

    #[test]
    fn truncation_notice_for_interactive_tools_does_not_claim_file_write() {
        let notice = build_tool_call_truncation_recovery_notice("AskUserQuestion");

        assert!(notice.contains("AskUserQuestion call was truncated"));
        assert!(notice.contains("fresh complete AskUserQuestion call"));
        assert!(!notice.contains("file was written"));
        assert!(!notice.contains("issue ONE Edit call"));
    }

    #[test]
    fn truncation_notice_for_write_tools_keeps_write_continuation_guidance() {
        let notice = build_tool_call_truncation_recovery_notice("Write");

        assert!(notice.contains("file may have been written with partial content"));
        assert!(notice.contains("latest Read result"));
        assert!(notice.contains("issue ONE Edit call"));
    }

    #[test]
    fn pipeline_preserves_core_owned_tool_context_without_portable_runtime_leak() {
        let pipeline = test_tool_pipeline();
        let mut task = test_tool_task("tool_context_1", "WebFetch");
        task.context
            .context_vars
            .insert("turn_index".to_string(), "7".to_string());
        task.context
            .context_vars
            .insert("primary_model_provider".to_string(), "openai".to_string());
        task.context.context_vars.insert(
            "primary_model_supports_image_understanding".to_string(),
            "true".to_string(),
        );
        task.context
            .context_vars
            .insert("acp_transport".to_string(), "true".to_string());
        task.context.collapsed_tools = vec!["WebFetch".to_string()];
        task.context.unlocked_collapsed_tools = vec!["WebFetch".to_string()];
        task.context.runtime_tool_restrictions = ToolRuntimeRestrictions {
            allowed_tool_names: ["WebFetch"].into_iter().map(str::to_string).collect(),
            denied_tool_names: ["Bash"].into_iter().map(str::to_string).collect(),
            denied_tool_messages: Default::default(),
            path_policy: Default::default(),
        };

        let context = pipeline.build_tool_use_context(&task, CancellationToken::new());

        assert_eq!(context.tool_call_id.as_deref(), Some("tool_context_1"));
        assert_eq!(context.agent_type.as_deref(), Some("agent"));
        assert_eq!(context.session_id.as_deref(), Some("session_1"));
        assert_eq!(context.dialog_turn_id.as_deref(), Some("turn_1"));
        assert_eq!(context.unlocked_collapsed_tools, vec!["WebFetch"]);
        assert!(context.cancellation_token().is_some());
        assert!(context.runtime_tool_restrictions.is_tool_allowed("WebFetch"));
        assert!(!context.runtime_tool_restrictions.is_tool_allowed("Bash"));
        assert_eq!(context.custom_data["turn_index"], json!(7));
        assert_eq!(context.custom_data["primary_model_provider"], json!("openai"));
        assert_eq!(
            context.custom_data["primary_model_supports_image_understanding"],
            json!(true)
        );
        assert_eq!(context.custom_data["acp_transport"], json!(true));

        let facts = context.to_tool_context_facts();
        let value = serde_json::to_value(&facts).expect("serialize context facts");
        assert_eq!(value["toolCallId"], "tool_context_1");
        assert_eq!(value["sessionId"], "session_1");
        assert!(value.get("unlockedCollapsedTools").is_none());
        assert!(value.get("customData").is_none());
        assert!(value.get("cancellationToken").is_none());
        assert!(value.get("workspaceServices").is_none());
    }

    #[test]
    fn collapsed_tool_requires_tool_catalog_unlock() {
        let mut task = test_tool_task("tool_1", "WebFetch");
        task.context.collapsed_tools = vec!["WebFetch".to_string()];

        let err = validate_tool_execution_admission(ToolExecutionAdmissionRequest {
            tool_name: &task.tool_call.tool_name,
            allowed_tools: &task.context.allowed_tools,
            runtime_tool_restrictions: &task.context.runtime_tool_restrictions,
            collapsed_tools: &task.context.collapsed_tools,
            loaded_collapsed_tools: &task.context.unlocked_collapsed_tools,
            get_tool_spec_tool_name: GET_TOOL_SPEC_TOOL_NAME,
        })
        .expect_err("collapsed tool should require GetToolSpec unlock");

        assert!(err
            .to_string()
            .contains("Call GetToolSpec first with {\"tool_name\":\"WebFetch\"}"));
    }

    #[test]
    fn tool_catalog_rejects_reloading_already_unlocked_tool() {
        let mut task = test_tool_task("tool_1", "GetToolSpec");
        task.tool_call.arguments = json!({ "tool_name": "WebFetch" });
        task.context.unlocked_collapsed_tools = vec!["WebFetch".to_string()];

        let result = validate_tool_execution_admission(ToolExecutionAdmissionRequest {
            tool_name: &task.tool_call.tool_name,
            allowed_tools: &task.context.allowed_tools,
            runtime_tool_restrictions: &task.context.runtime_tool_restrictions,
            collapsed_tools: &task.context.collapsed_tools,
            loaded_collapsed_tools: &task.context.unlocked_collapsed_tools,
            get_tool_spec_tool_name: GET_TOOL_SPEC_TOOL_NAME,
        });

        assert!(
            result.is_ok(),
            "GetToolSpec duplicate-load validation moved into GetToolSpec itself"
        );
    }

    #[test]
    fn task_tool_manages_its_own_execution_timeout() {
        let task_tool = TaskTool::new();
        assert!(task_tool.manages_own_execution_timeout());
    }
}
