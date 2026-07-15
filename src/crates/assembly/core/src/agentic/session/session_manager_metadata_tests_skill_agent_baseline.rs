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
async fn rebuild_skill_agent_listing_baseline_to_latest_removes_listing_diff_reminders() {
    use crate::agentic::core::{InternalReminderKind, Message, MessageSemanticKind};
    use crate::agentic::skill_agent_snapshot::{SkillSnapshotEntry, TurnSkillAgentSnapshot};

    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence manager"));
    let manager = test_manager(persistence_manager.clone());
    let session = manager
        .create_session(
            "Listing baseline rebuild".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("session should create");

    {
        let mut active = manager
            .sessions
            .get_mut(&session.session_id)
            .expect("session should be active");
        active.dialog_turn_ids = vec!["turn-0".to_string(), "turn-1".to_string()];
    }

    manager.context_store.replace_context(
        &session.session_id,
        vec![
            Message::internal_reminder(
                InternalReminderKind::SkillListingDiff,
                "# Skill Listing Update\n\nChanged",
            )
            .with_turn_id("turn-1".to_string()),
            Message::internal_reminder(
                InternalReminderKind::AgentListingDiff,
                "# Agent Listing Update\n\nChanged",
            )
            .with_turn_id("turn-1".to_string()),
            Message::user("real question".to_string())
                .with_turn_id("turn-1".to_string())
                .with_semantic_kind(MessageSemanticKind::ActualUserInput),
        ],
    );

    manager
        .remember_turn_skill_agent_snapshot(
            &session.session_id,
            0,
            TurnSkillAgentSnapshot {
                skills: vec![SkillSnapshotEntry {
                    name: "old-skill".to_string(),
                    description: "old".to_string(),
                    location: "/old".to_string(),
                }],
                ..Default::default()
            },
        )
        .await;
    manager
        .remember_turn_skill_agent_snapshot(
            &session.session_id,
            1,
            TurnSkillAgentSnapshot {
                skills: vec![SkillSnapshotEntry {
                    name: "new-skill".to_string(),
                    description: "new".to_string(),
                    location: "/new".to_string(),
                }],
                ..Default::default()
            },
        )
        .await;

    assert!(
        manager
            .rebuild_skill_agent_listing_baseline_to_latest(&session.session_id)
            .await
    );

    let context_messages = manager.context_store.get_context_messages(&session.session_id);
    assert_eq!(context_messages.len(), 1);
    assert_eq!(
        context_messages[0].metadata.semantic_kind,
        Some(MessageSemanticKind::ActualUserInput)
    );

    let baseline = manager
        .turn_skill_agent_snapshot(&session.session_id, 0)
        .await
        .expect("baseline snapshot should exist");
    assert_eq!(baseline.skills[0].name, "new-skill");
    assert!(manager
        .turn_skill_agent_snapshot(&session.session_id, 1)
        .await
        .is_none());

    let metadata = persistence_manager
        .load_session_metadata(workspace.path(), &session.session_id)
        .await
        .expect("metadata lookup should succeed")
        .expect("metadata should exist");
    assert_eq!(
        SessionManager::listing_baseline_rebuild_turn_index_from_metadata(Some(&metadata)),
        Some(1)
    );
}

#[tokio::test]
async fn skill_agent_baseline_override_snapshot_persists_across_session_restore() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence"));
    let manager = test_manager(persistence_manager.clone());
    let session = manager
        .create_session(
            "Listing baseline".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("session should be created");
    let baseline = TurnSkillAgentSnapshot {
        skills: vec![SkillSnapshotEntry {
            name: "skill-a".to_string(),
            description: "desc-a".to_string(),
            location: "/skills/a".to_string(),
        }],
        ..Default::default()
    };

    manager
        .remember_skill_agent_baseline_override_snapshot(&session.session_id, baseline.clone())
        .await;

    let metadata = persistence_manager
        .load_session_metadata(workspace.path(), &session.session_id)
        .await
        .expect("metadata load should succeed")
        .expect("metadata should exist");
    assert_eq!(metadata.custom_metadata, None);
    assert_eq!(
        persistence_manager
            .load_skill_agent_baseline_override_snapshot(workspace.path(), &session.session_id,)
            .await
            .expect("override snapshot load should succeed"),
        Some(baseline.clone())
    );

    let restored_manager = test_manager(persistence_manager);
    restored_manager
        .restore_session(workspace.path(), &session.session_id)
        .await
        .expect("session should restore");

    assert_eq!(
        restored_manager
            .skill_agent_baseline_override_snapshot(&session.session_id)
            .await,
        Some(baseline)
    );
}

#[tokio::test]
async fn seed_forked_skill_agent_listing_baselines_splits_prompt_and_diff_baselines() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence"));
    let manager = test_manager(persistence_manager.clone());
    let parent = manager
        .create_session(
            "Parent".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("parent session should create");
    let child = manager
        .create_session(
            "Child".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("child session should create");
    let prompt_baseline = TurnSkillAgentSnapshot {
        skills: vec![SkillSnapshotEntry {
            name: "skill-parent-turn-0".to_string(),
            description: "desc-0".to_string(),
            location: "/skills/turn-0".to_string(),
        }],
        ..Default::default()
    };
    let latest_baseline = TurnSkillAgentSnapshot {
        skills: vec![SkillSnapshotEntry {
            name: "skill-parent-latest".to_string(),
            description: "desc-latest".to_string(),
            location: "/skills/latest".to_string(),
        }],
        ..Default::default()
    };

    manager
        .remember_turn_skill_agent_snapshot(&parent.session_id, 0, prompt_baseline.clone())
        .await;
    manager
        .remember_turn_skill_agent_snapshot(&parent.session_id, 2, latest_baseline.clone())
        .await;
    {
        let mut parent_session = manager
            .sessions
            .get_mut(&parent.session_id)
            .expect("parent session should remain in memory");
        parent_session.dialog_turn_ids = vec!["turn-0".to_string(), "turn-1".to_string(), "turn-2".to_string()];
    }

    manager
        .seed_forked_skill_agent_listing_baselines(&parent.session_id, &child.session_id)
        .await;

    assert_eq!(
        manager.skill_agent_baseline_override_snapshot(&child.session_id).await,
        Some(prompt_baseline.clone())
    );
    assert_eq!(
        manager.turn_skill_agent_snapshot(&child.session_id, 0).await,
        Some(latest_baseline.clone())
    );

    let restored_manager = test_manager(persistence_manager);
    restored_manager
        .restore_session(workspace.path(), &child.session_id)
        .await
        .expect("child session should restore");
    assert_eq!(
        restored_manager
            .skill_agent_baseline_override_snapshot(&child.session_id)
            .await,
        Some(prompt_baseline)
    );
    assert_eq!(
        restored_manager.turn_skill_agent_snapshot(&child.session_id, 0).await,
        Some(latest_baseline)
    );
}
