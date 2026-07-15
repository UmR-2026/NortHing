//! `SessionMessage` tool — unit tests.
//!
//! These exercise the public surface (`SessionMessageTool::new()` plus
//! the `Tool` trait surface kept in `tool.rs`). All test inputs are
//! constructed with throwaway temporary workspaces so the suite stays
//! hermetic and self-cleaning via the `TestTempDir` drop guard.

use northhing_test_support::TestTempDir;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::agentic::tools::framework::{Tool, ToolUseContext};

use super::tool::SessionMessageTool;

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

fn session_context(session_id: &str) -> ToolUseContext {
    ToolUseContext {
        session_id: Some(session_id.to_string()),
        ..empty_context()
    }
}

#[tokio::test]
async fn validate_existing_session_rejects_agent_type_override() {
    let tool = SessionMessageTool::new();
    let workspace = TestTempDir::new("northhing-session-message-tool-test");

    let validation = tool
        .validate_input(
            &json!({
                "workspace": workspace.path().to_string_lossy().to_string(),
                "session_id": "worker_1",
                "message": "hello",
                "agent_type": "Plan",
            }),
            Some(&session_context("source_1")),
        )
        .await;

    assert!(!validation.result);
    assert_eq!(
        validation.message.as_deref(),
        Some("agent_type override is not allowed when session_id is provided")
    );
}

#[tokio::test]
async fn validate_new_session_requires_session_name() {
    let tool = SessionMessageTool::new();
    let workspace = TestTempDir::new("northhing-session-message-tool-test");

    let validation = tool
        .validate_input(
            &json!({
                "workspace": workspace.path().to_string_lossy().to_string(),
                "message": "hello",
                "agent_type": "agentic",
            }),
            Some(&session_context("source_1")),
        )
        .await;

    assert!(!validation.result);
    assert_eq!(
        validation.message.as_deref(),
        Some("session_name is required when session_id is omitted")
    );
}

#[tokio::test]
async fn validate_new_session_requires_agent_type() {
    let tool = SessionMessageTool::new();
    let workspace = TestTempDir::new("northhing-session-message-tool-test");

    let validation = tool
        .validate_input(
            &json!({
                "workspace": workspace.path().to_string_lossy().to_string(),
                "message": "hello",
                "session_name": "Worker Session",
            }),
            Some(&session_context("source_1")),
        )
        .await;

    assert!(!validation.result);
    assert_eq!(
        validation.message.as_deref(),
        Some("agent_type is required when session_id is omitted")
    );
}

#[tokio::test]
async fn validate_new_session_accepts_create_and_send_shape() {
    let tool = SessionMessageTool::new();
    let workspace = TestTempDir::new("northhing-session-message-tool-test");

    let validation = tool
        .validate_input(
            &json!({
                "workspace": workspace.path().to_string_lossy().to_string(),
                "message": "hello",
                "session_name": "Worker Session",
                "agent_type": "agentic",
            }),
            Some(&session_context("source_1")),
        )
        .await;

    assert!(validation.result, "{:?}", validation.message);
}

#[tokio::test]
async fn validate_existing_session_allows_missing_workspace() {
    let tool = SessionMessageTool::new();

    let validation = tool
        .validate_input(
            &json!({
                "session_id": "worker_1",
                "message": "hello",
            }),
            Some(&session_context("source_1")),
        )
        .await;

    assert!(validation.result, "{:?}", validation.message);
}

#[tokio::test]
async fn validate_new_session_requires_workspace() {
    let tool = SessionMessageTool::new();

    let validation = tool
        .validate_input(
            &json!({
                "message": "hello",
                "session_name": "Worker Session",
                "agent_type": "agentic",
            }),
            Some(&session_context("source_1")),
        )
        .await;

    assert!(!validation.result);
    assert_eq!(
        validation.message.as_deref(),
        Some("workspace is required when session_id is omitted")
    );
}
