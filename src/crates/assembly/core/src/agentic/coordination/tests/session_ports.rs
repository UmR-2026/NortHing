//! Session lifecycle port tests.
//!
//! Tests for `AgentSessionManagementPort`, session creation metadata,
//! remote workspace identity propagation, and hidden-between-session
//! baseline inheritance.

use crate::agentic::coordination::tests::test_coordinator;
use crate::agentic::core::SessionConfig;
use crate::agentic::session::prompt_cache::PromptCachePolicy;
use crate::agentic::skill_agent_snapshot::SkillSnapshotEntry;
use crate::agentic::tools::registry::ToolRegistry;
use crate::agentic::TurnSkillAgentSnapshot;
use crate::agentic::{Message, SessionKind};
use crate::infrastructure::app_paths::PathManager;
use crate::service::remote_ssh::workspace_state::init_remote_workspace_manager;
use northhing_runtime_ports::AgentSessionCreateRequest;
use std::time::Duration;

#[tokio::test]
async fn agent_submission_create_session_preserves_creator_metadata() {
    let (coordinator, session_manager) = test_coordinator();
    let workspace_path =
        std::env::temp_dir().join(format!("northhing-agent-session-port-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir should exist");
    let mut metadata = serde_json::Map::new();
    metadata.insert(
        "createdBy".to_string(),
        serde_json::Value::String("session-parent".to_string()),
    );

    let result = northhing_runtime_ports::AgentSubmissionPort::create_session(
        &*coordinator,
        AgentSessionCreateRequest {
            session_name: "Worker".to_string(),
            agent_type: "agentic".to_string(),
            workspace_path: Some(workspace_path.to_string_lossy().into_owned()),
            metadata,
        },
    )
    .await
    .expect("port-backed session creation should succeed");
    let created = session_manager
        .get_session(&result.session_id)
        .expect("created session should be persisted");

    assert_eq!(result.session_name, "Worker");
    assert_eq!(result.session_name, created.session_name);
    assert_eq!(created.created_by.as_deref(), Some("session-parent"));

    let _ = std::fs::remove_dir_all(workspace_path);
}

#[tokio::test]
async fn subagent_session_config_preserves_registered_remote_workspace_identity() {
    let manager = init_remote_workspace_manager();
    manager
        .register_remote_workspace(
            "/remote/subagent-test".to_string(),
            "conn-subagent-test".to_string(),
            "Remote Test".to_string(),
            "remote-host".to_string(),
        )
        .await;
    manager
        .set_active_connection_hint(Some("conn-subagent-test".to_string()))
        .await;

    let config =
        crate::agentic::coordination::coordinator::ConversationCoordinator::build_session_config_for_workspace(
            "/remote/subagent-test/project".to_string(),
            Some("model-fast".to_string()),
        )
        .await;

    assert_eq!(config.workspace_path.as_deref(), Some("/remote/subagent-test/project"));
    assert_eq!(config.remote_connection_id.as_deref(), Some("conn-subagent-test"));
    assert_eq!(config.remote_ssh_host.as_deref(), Some("remote-host"));
    assert_eq!(config.model_id.as_deref(), Some("model-fast"));
}

#[tokio::test]
async fn hidden_btw_session_seeds_forked_listing_baselines() {
    let (coordinator, session_manager) = test_coordinator();
    let workspace_path = std::env::temp_dir().join(format!("northhing-btw-baseline-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir should exist");
    let parent_session = session_manager
        .create_session(
            "Parent".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace_path.to_string_lossy().into_owned()),
                ..Default::default()
            },
        )
        .await
        .expect("parent session should be created");
    session_manager
        .replace_context_messages(
            &parent_session.session_id,
            vec![Message::user("parent context".to_string())],
        )
        .await;

    let system_prompt_identity =
        crate::agentic::session::prompt_cache::SystemPromptCacheIdentity::new("template:agentic_mode");
    let user_context_identity =
        crate::agentic::session::prompt_cache::UserContextCacheIdentity::new("workspace_context");
    session_manager
        .remember_system_prompt(
            &parent_session.session_id,
            system_prompt_identity.clone(),
            "cached system prompt".to_string(),
        )
        .await;
    session_manager
        .remember_user_context(
            &parent_session.session_id,
            user_context_identity.clone(),
            "cached user context".to_string(),
        )
        .await;

    let baseline_snapshot = TurnSkillAgentSnapshot {
        skills: vec![SkillSnapshotEntry {
            name: "interactive-debug".to_string(),
            description: "debug helper".to_string(),
            location: "C:/Users/wsp/.codex/skills/interactive-debug".to_string(),
        }],
        subagents: Vec::new(),
    };
    session_manager
        .remember_turn_skill_agent_snapshot(&parent_session.session_id, 0, baseline_snapshot.clone())
        .await;

    let child_session = coordinator
        .ensure_hidden_btw_session(&parent_session.session_id, "btw-child", None)
        .await
        .expect("btw child session should be created");

    assert_eq!(child_session.kind, SessionKind::EphemeralChild);
    assert_eq!(
        session_manager
            .cached_system_prompt(&child_session.session_id, &system_prompt_identity)
            .await,
        Some("cached system prompt".to_string())
    );
    assert_eq!(
        session_manager
            .cached_user_context(&child_session.session_id, &user_context_identity)
            .await,
        Some("cached user context".to_string())
    );
    assert_eq!(
        session_manager
            .skill_agent_baseline_override_snapshot(&child_session.session_id)
            .await,
        Some(baseline_snapshot.clone())
    );
    assert_eq!(
        session_manager
            .turn_skill_agent_snapshot(&child_session.session_id, 0)
            .await,
        Some(baseline_snapshot)
    );
}
