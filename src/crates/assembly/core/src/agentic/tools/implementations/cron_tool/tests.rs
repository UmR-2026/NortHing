use super::*;
use crate::agentic::tools::framework::ToolUseContext;
use serde_json::json;
use std::collections::HashMap;

fn empty_context() -> ToolUseContext {
    ToolUseContext {
        tool_call_id: None,
        agent_type: None,
        session_id: None,
        dialog_turn_id: None,
        workspace: None,
        unlocked_collapsed_tools: Vec::new(),
        custom_data: HashMap::new(),
        computer_use_host: None,
        runtime_tool_restrictions: Default::default(),
        runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
        actor_runtime: None,
    }
}

#[tokio::test]
async fn validate_list_allows_missing_workspace_when_session_id_present() {
    let tool = CronTool::new();

    let validation = tool
        .validate_input(
            &json!({
                "action": "list",
                "session_id": "worker_1",
            }),
            Some(&empty_context()),
        )
        .await;

    assert!(validation.result, "{:?}", validation.message);
}

#[tokio::test]
async fn validate_add_allows_missing_workspace_when_session_id_present() {
    let tool = CronTool::new();

    let validation = tool
        .validate_input(
            &json!({
                "action": "add",
                "session_id": "worker_1",
                "job": {
                    "payload": "hello",
                    "schedule": {
                        "kind": "every",
                        "every": 60
                    }
                }
            }),
            Some(&empty_context()),
        )
        .await;

    assert!(validation.result, "{:?}", validation.message);
}

#[tokio::test]
async fn validate_rejects_legacy_workspace_field() {
    let tool = CronTool::new();

    let validation = tool
        .validate_input(
            &json!({
                "action": "list",
                "session_id": "worker_1",
                "workspace": "E:/Projects/Opennorthhing/northhing",
            }),
            Some(&empty_context()),
        )
        .await;

    assert!(!validation.result);
    assert!(validation
        .message
        .as_deref()
        .unwrap_or_default()
        .contains("unknown field"));
}
