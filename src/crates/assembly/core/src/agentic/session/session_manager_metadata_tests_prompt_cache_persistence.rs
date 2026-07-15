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
async fn prompt_cache_persists_across_session_restore() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence"));
    let manager = test_manager(persistence_manager.clone());
    let workspace_path = workspace.path().to_string_lossy().to_string();
    let session = manager
        .create_session(
            "Prompt cache".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace_path),
                ..Default::default()
            },
        )
        .await
        .expect("session should be created");
    let identity = SystemPromptCacheIdentity::new("template:agentic_mode");
    let user_context_identity =
        UserContextCacheIdentity::new("workspace_context|workspace_instructions|workspace_memory_files|project_layout");

    manager
        .remember_system_prompt(
            &session.session_id,
            identity.clone(),
            "cached system prompt".to_string(),
        )
        .await;
    manager
        .remember_user_context(
            &session.session_id,
            user_context_identity.clone(),
            "cached user context".to_string(),
        )
        .await;

    let restored_manager = test_manager(persistence_manager);
    restored_manager
        .restore_session(workspace.path(), &session.session_id)
        .await
        .expect("session should restore");

    assert_eq!(
        restored_manager
            .cached_system_prompt(&session.session_id, &identity)
            .await,
        Some("cached system prompt".to_string())
    );
    assert_eq!(
        restored_manager
            .cached_user_context(&session.session_id, &user_context_identity)
            .await,
        Some("cached user context".to_string())
    );
}

#[tokio::test]
async fn prompt_cache_invalidation_removes_persisted_entries() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence"));
    let manager = test_manager(persistence_manager.clone());
    let workspace_path = workspace.path().to_string_lossy().to_string();
    let session = manager
        .create_session(
            "Prompt cache".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace_path),
                ..Default::default()
            },
        )
        .await
        .expect("session should be created");
    let identity = SystemPromptCacheIdentity::new("template:agentic_mode");
    let user_context_identity =
        UserContextCacheIdentity::new("workspace_context|workspace_instructions|workspace_memory_files|project_layout");

    manager
        .remember_system_prompt(
            &session.session_id,
            identity.clone(),
            "cached system prompt".to_string(),
        )
        .await;
    manager
        .remember_user_context(
            &session.session_id,
            user_context_identity.clone(),
            "cached user context".to_string(),
        )
        .await;

    manager
        .invalidate_prompt_cache(&session.session_id, PromptCacheScope::All, "test")
        .await;

    let restored_manager = test_manager(persistence_manager.clone());
    restored_manager
        .restore_session(workspace.path(), &session.session_id)
        .await
        .expect("session should restore");

    assert_eq!(
        restored_manager
            .cached_system_prompt(&session.session_id, &identity)
            .await,
        None
    );
    assert_eq!(
        restored_manager
            .cached_user_context(&session.session_id, &user_context_identity)
            .await,
        None
    );
    assert_eq!(
        persistence_manager
            .load_prompt_cache(workspace.path(), &session.session_id)
            .await
            .expect("prompt cache load should succeed"),
        None
    );
}

#[tokio::test]
async fn clone_prompt_cache_copies_runtime_and_persisted_entries() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence"));
    let manager = test_manager(persistence_manager.clone());
    let workspace_path = workspace.path().to_string_lossy().to_string();
    let source_session = manager
        .create_session(
            "Prompt cache source".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace_path.clone()),
                ..Default::default()
            },
        )
        .await
        .expect("source session should be created");
    let target_session = manager
        .create_session(
            "Prompt cache target".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace_path),
                ..Default::default()
            },
        )
        .await
        .expect("target session should be created");
    let identity = SystemPromptCacheIdentity::new("template:agentic_mode");
    let user_context_identity =
        UserContextCacheIdentity::new("workspace_context|workspace_instructions|workspace_memory_files|project_layout");

    manager
        .remember_system_prompt(
            &source_session.session_id,
            identity.clone(),
            "cached system prompt".to_string(),
        )
        .await;
    manager
        .remember_user_context(
            &source_session.session_id,
            user_context_identity.clone(),
            "cached user context".to_string(),
        )
        .await;

    assert!(
        manager
            .clone_prompt_cache(&source_session.session_id, &target_session.session_id)
            .await
    );
    assert_eq!(
        manager
            .cached_system_prompt(&target_session.session_id, &identity)
            .await,
        Some("cached system prompt".to_string())
    );
    assert_eq!(
        manager
            .cached_user_context(&target_session.session_id, &user_context_identity)
            .await,
        Some("cached user context".to_string())
    );
    assert_eq!(
        persistence_manager
            .load_prompt_cache(workspace.path(), &target_session.session_id)
            .await
            .expect("prompt cache load should succeed")
            .expect("cloned prompt cache should persist"),
        persistence_manager
            .load_prompt_cache(workspace.path(), &source_session.session_id)
            .await
            .expect("source prompt cache load should succeed")
            .expect("source prompt cache should exist")
    );
}

#[tokio::test]
async fn prompt_cache_persistence_ttl_only_affects_cold_start_restore() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence"));
    let manager = test_manager_with_config(
        persistence_manager.clone(),
        SessionManagerConfig {
            max_active_sessions: 100,
            session_idle_timeout: Duration::from_secs(3600),
            auto_save_interval: Duration::from_secs(300),
            enable_persistence: true,
            prompt_cache_policy: PromptCachePolicy {
                cache_ttl: None,
                persistence_ttl: Some(Duration::from_millis(0)),
            },
        },
    );
    let workspace_path = workspace.path().to_string_lossy().to_string();
    let session = manager
        .create_session(
            "Prompt cache".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace_path),
                ..Default::default()
            },
        )
        .await
        .expect("session should be created");
    let identity = SystemPromptCacheIdentity::new("template:agentic_mode");
    let user_context_identity =
        UserContextCacheIdentity::new("workspace_context|workspace_instructions|workspace_memory_files|project_layout");

    manager
        .remember_system_prompt(
            &session.session_id,
            identity.clone(),
            "cached system prompt".to_string(),
        )
        .await;
    manager
        .remember_user_context(
            &session.session_id,
            user_context_identity.clone(),
            "cached user context".to_string(),
        )
        .await;

    assert_eq!(
        manager.cached_system_prompt(&session.session_id, &identity).await,
        Some("cached system prompt".to_string())
    );
    assert_eq!(
        manager
            .cached_user_context(&session.session_id, &user_context_identity)
            .await,
        Some("cached user context".to_string())
    );

    let restored_manager = test_manager_with_config(
        persistence_manager.clone(),
        SessionManagerConfig {
            max_active_sessions: 100,
            session_idle_timeout: Duration::from_secs(3600),
            auto_save_interval: Duration::from_secs(300),
            enable_persistence: true,
            prompt_cache_policy: PromptCachePolicy {
                cache_ttl: None,
                persistence_ttl: Some(Duration::from_millis(0)),
            },
        },
    );
    restored_manager
        .restore_session(workspace.path(), &session.session_id)
        .await
        .expect("session should restore");

    assert_eq!(
        restored_manager
            .cached_system_prompt(&session.session_id, &identity)
            .await,
        None
    );
    assert_eq!(
        restored_manager
            .cached_user_context(&session.session_id, &user_context_identity)
            .await,
        None
    );
}
