#![cfg(test)]
#![allow(unused_imports)]

use super::super::super::session_manager::{SessionManager, SessionManagerConfig};
use super::super::{test_manager, test_manager_with_config, TestWorkspace};
use super::*;
use crate::agentic::core::{InternalReminderKind, Message, MessageSemanticKind, Session, SessionConfig, SessionKind};
use crate::agentic::persistence::PersistenceManager;
use crate::agentic::session::{
    PromptCachePolicy, PromptCacheScope, SystemPromptCacheIdentity, UserContextCacheIdentity,
};
use crate::agentic::skill_agent_snapshot::{SkillSnapshotEntry, TurnSkillAgentSnapshot};
use crate::infrastructure::PathManager;
use crate::service::session::{
    DialogTurnData, ModelRoundData, SessionMetadata, SessionRelationship, SessionRelationshipKind, ToolCallData,
    ToolItemData, ToolResultData, UserMessageData,
};
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn collect_hidden_subagent_cascade_for_parent_turns_returns_post_order_matches() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence manager"));
    let manager = test_manager(persistence_manager.clone());

    let mut matched_root = SessionMetadata::new(
        "child-root".to_string(),
        "Subagent: root".to_string(),
        "Explore".to_string(),
        "model".to_string(),
    );
    matched_root.session_kind = SessionKind::Subagent;
    matched_root.relationship = Some(SessionRelationship {
        kind: Some(SessionRelationshipKind::Subagent),
        parent_session_id: Some("parent-session".to_string()),
        parent_request_id: None,
        parent_dialog_turn_id: Some("turn-2".to_string()),
        parent_turn_index: Some(2),
        parent_tool_call_id: Some("tool-1".to_string()),
        subagent_type: Some("Explore".to_string()),
    });
    persistence_manager
        .save_session_metadata(workspace.path(), &matched_root)
        .await
        .expect("matched root should save");

    let mut matched_grandchild = SessionMetadata::new(
        "grandchild".to_string(),
        "Subagent: grandchild".to_string(),
        "Explore".to_string(),
        "model".to_string(),
    );
    matched_grandchild.session_kind = SessionKind::Subagent;
    matched_grandchild.relationship = Some(SessionRelationship {
        kind: Some(SessionRelationshipKind::Subagent),
        parent_session_id: Some("child-root".to_string()),
        parent_request_id: None,
        parent_dialog_turn_id: Some("child-turn".to_string()),
        parent_turn_index: None,
        parent_tool_call_id: Some("tool-child".to_string()),
        subagent_type: Some("Explore".to_string()),
    });
    persistence_manager
        .save_session_metadata(workspace.path(), &matched_grandchild)
        .await
        .expect("grandchild should save");

    let mut unmatched_root = SessionMetadata::new(
        "child-other-turn".to_string(),
        "Subagent: other turn".to_string(),
        "Explore".to_string(),
        "model".to_string(),
    );
    unmatched_root.session_kind = SessionKind::Subagent;
    unmatched_root.relationship = Some(SessionRelationship {
        kind: Some(SessionRelationshipKind::Subagent),
        parent_session_id: Some("parent-session".to_string()),
        parent_request_id: None,
        parent_dialog_turn_id: Some("turn-1".to_string()),
        parent_turn_index: Some(1),
        parent_tool_call_id: Some("tool-2".to_string()),
        subagent_type: Some("Explore".to_string()),
    });
    persistence_manager
        .save_session_metadata(workspace.path(), &unmatched_root)
        .await
        .expect("unmatched root should save");

    let mut visible_review_child = SessionMetadata::new(
        "review-child".to_string(),
        "Review child".to_string(),
        "DeepReview".to_string(),
        "model".to_string(),
    );
    visible_review_child.relationship = Some(SessionRelationship {
        kind: Some(SessionRelationshipKind::DeepReview),
        parent_session_id: Some("parent-session".to_string()),
        parent_request_id: None,
        parent_dialog_turn_id: Some("turn-2".to_string()),
        parent_turn_index: Some(2),
        parent_tool_call_id: None,
        subagent_type: None,
    });
    persistence_manager
        .save_session_metadata(workspace.path(), &visible_review_child)
        .await
        .expect("visible review child should save");

    let matched_turn_ids = HashSet::from(["turn-2".to_string()]);
    let cascade = manager
        .collect_hidden_subagent_cascade_for_parent_turns(workspace.path(), "parent-session", &matched_turn_ids)
        .await
        .expect("cascade lookup should succeed");

    assert_eq!(cascade, vec!["grandchild".to_string(), "child-root".to_string()]);
}

#[tokio::test]
async fn latest_skill_agent_snapshot_scans_persistence_beyond_stale_cache_hit() {
    use crate::agentic::skill_agent_snapshot::{AgentSnapshotEntry, SkillSnapshotEntry, TurnSkillAgentSnapshot};

    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence manager"));
    let manager = test_manager(persistence_manager.clone());
    let session = manager
        .create_session(
            "Skill agent snapshot".to_string(),
            "agent".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("session should create");

    manager
        .remember_turn_skill_agent_snapshot(
            &session.session_id,
            0,
            TurnSkillAgentSnapshot {
                skills: vec![SkillSnapshotEntry {
                    name: "skill-a".to_string(),
                    description: "desc-a".to_string(),
                    location: "/a".to_string(),
                }],
                subagents: vec![AgentSnapshotEntry {
                    id: "agent-a".to_string(),
                    description: "desc-a".to_string(),
                    default_tools: vec!["Read".to_string()],
                }],
            },
        )
        .await;
    manager
        .remember_turn_skill_agent_snapshot(
            &session.session_id,
            1,
            TurnSkillAgentSnapshot {
                skills: vec![SkillSnapshotEntry {
                    name: "skill-a".to_string(),
                    description: "desc-a".to_string(),
                    location: "/a".to_string(),
                }],
                subagents: vec![AgentSnapshotEntry {
                    id: "agent-b".to_string(),
                    description: "desc-b".to_string(),
                    default_tools: vec!["Read".to_string(), "Grep".to_string()],
                }],
            },
        )
        .await;

    manager
        .turn_skill_agent_snapshot_store
        .delete_session(&session.session_id);
    manager
        .turn_skill_agent_snapshot_store
        .create_session(&session.session_id);
    manager.turn_skill_agent_snapshot_store.set_snapshot(
        &session.session_id,
        0,
        TurnSkillAgentSnapshot {
            skills: vec![SkillSnapshotEntry {
                name: "skill-a".to_string(),
                description: "desc-a".to_string(),
                location: "/a".to_string(),
            }],
            subagents: vec![AgentSnapshotEntry {
                id: "agent-a".to_string(),
                description: "desc-a".to_string(),
                default_tools: vec!["Read".to_string()],
            }],
        },
    );

    let latest = manager
        .latest_turn_skill_agent_snapshot_at_or_before(&session.session_id, 1)
        .await
        .expect("latest snapshot should exist");

    assert_eq!(latest.0, 1);
    assert_eq!(latest.1.subagents[0].id, "agent-b");
}

#[tokio::test]
async fn records_subagent_partial_timeout_in_evidence_ledger() {
    let persistence_manager = Arc::new(
        PersistenceManager::new(Arc::new(PathManager::new().expect("path manager"))).expect("persistence manager"),
    );
    let manager = test_manager(persistence_manager);

    let event = manager.record_subagent_partial_timeout(
        "session-a",
        "turn-a",
        "ReviewSecurity",
        "Found token logging before timeout.",
        Some("timeout"),
    );

    assert!(!event.event_id.is_empty());
    let events = manager.evidence_events_for_turn("session-a", "turn-a");
    assert_eq!(events, vec![event.clone()]);
    let summary = manager.evidence_summary_for_session("session-a", 10);
    assert_eq!(summary.partial_subagent_results.len(), 1);
    assert_eq!(summary.partial_subagent_results[0].event_id, event.event_id);
}
