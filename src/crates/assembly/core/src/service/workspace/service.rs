//! Workspace service - advanced workspace management API
//!
//! Provides comprehensive workspace management functionality.
//!
//! This file is the facade for the workspace service. It owns:
//!
//! - the [`WorkspaceService`] struct definition
//! - the global workspace service singleton
//! - the unit-test module (compiled only under `#[cfg(test)]`)
//!
//! The actual API surface is split across sibling files by sub-domain:
//!
//! - [`super::service_types`]  — public DTOs and persistence payloads
//! - [`super::service_init`]   — construction + startup helpers
//! - [`super::service_invoke`] — mutating workspace operations
//! - [`super::service_state`]  — read-only workspace queries

pub use super::service_types::{
    BatchImportResult, BatchRemoveResult, WorkspaceCreateOptions, WorkspaceExport, WorkspaceHealthStatus,
    WorkspaceIdentityChangedEvent, WorkspaceImportResult, WorkspaceInfoUpdates, WorkspaceQuickSummary,
};

use super::manager::WorkspaceManager;
use crate::infrastructure::storage::PersistenceService;
use crate::infrastructure::PathManager;
use crate::service::workspace_runtime::WorkspaceRuntimeService;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Workspace service.
pub struct WorkspaceService {
    pub(super) manager: Arc<RwLock<WorkspaceManager>>,
    // reason: config field is held for the upcoming config-driven behavior in workspace service; today's service reads config at construction
    #[allow(dead_code)]
    pub(super) config: super::manager::WorkspaceManagerConfig,
    pub(super) persistence: Arc<PersistenceService>,
    pub(super) path_manager: Arc<PathManager>,
    pub(super) runtime_service: Arc<WorkspaceRuntimeService>,
}

// ── Global workspace service singleton ──────────────────────────────

static GLOBAL_WORKSPACE_SERVICE: std::sync::OnceLock<Arc<WorkspaceService>> = std::sync::OnceLock::new();

pub fn set_global_workspace_service(service: Arc<WorkspaceService>) {
    match GLOBAL_WORKSPACE_SERVICE.set(service) {
        Ok(_) => info!("Global workspace service set"),
        Err(_) => info!("Global workspace service already exists, skipping set"),
    }
}

pub fn global_workspace_service() -> Option<Arc<WorkspaceService>> {
    GLOBAL_WORKSPACE_SERVICE.get().cloned()
}

#[cfg(all(test, feature = "product-full"))]
mod tests {
    use super::super::service_types::WorkspacePersistenceData;
    use super::*;
    use crate::agentic::persistence::PersistenceManager;
    use crate::infrastructure::storage::{PersistenceService, StorageOptions};
    use crate::service::remote_ssh::workspace_state::remote_workspace_stable_id;
    use crate::service::session::SessionMetadata;
    use crate::service::workspace::manager::{
        WorkspaceInfo, WorkspaceKind, WorkspaceManagerConfig, WorkspaceOpenOptions,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;
    use uuid::Uuid;

    struct TestEnvironment {
        root: PathBuf,
        path_manager: Arc<PathManager>,
    }

    impl TestEnvironment {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("northhing-workspace-service-test-{}", Uuid::new_v4()));
            std::fs::create_dir_all(&root).expect("test root should be created");

            let path_manager = Arc::new(PathManager::with_user_root_for_tests(root.join("user-root")));

            Self { root, path_manager }
        }

        fn create_workspace_dir(&self, name: &str) -> PathBuf {
            let path = self.root.join(name);
            std::fs::create_dir_all(&path).expect("workspace directory should be created");
            path
        }
    }

    impl Drop for TestEnvironment {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    async fn build_test_workspace_service(path_manager: Arc<PathManager>) -> WorkspaceService {
        path_manager
            .initialize_user_directories()
            .await
            .expect("user directories should initialize");

        let config = WorkspaceManagerConfig::default();
        let persistence = Arc::new(
            PersistenceService::new_user_level(path_manager.clone())
                .await
                .expect("persistence should initialize"),
        );
        let runtime_service = Arc::new(WorkspaceRuntimeService::new(path_manager.clone()));

        WorkspaceService {
            manager: Arc::new(RwLock::new(WorkspaceManager::new(config.clone()))),
            config,
            persistence,
            path_manager,
            runtime_service,
        }
    }

    #[tokio::test]
    async fn ensure_workspace_gitignore_best_effort_skips_remote_workspaces() {
        let env = TestEnvironment::new();
        let service = build_test_workspace_service(env.path_manager.clone()).await;
        let remote_workspace_root = env.create_workspace_dir("remote-workspace-shadow");
        std::fs::write(remote_workspace_root.join(".gitignore"), "target/\n").expect("gitignore should be seeded");

        let remote_workspace = WorkspaceInfo::new(
            remote_workspace_root.clone(),
            WorkspaceOpenOptions {
                workspace_kind: WorkspaceKind::Remote,
                remote_ssh_host: Some("example-host".to_string()),
                remote_connection_id: Some("conn-1".to_string()),
                stable_workspace_id: Some("remote-test".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("remote workspace should initialize");

        service
            .ensure_workspace_gitignore_best_effort(&remote_workspace, "test")
            .await;

        let gitignore =
            std::fs::read_to_string(remote_workspace_root.join(".gitignore")).expect("gitignore should be readable");
        assert_eq!(gitignore, "target/\n");
    }

    #[tokio::test]
    async fn load_workspace_history_only_ensures_all_opened_local_workspaces() {
        let env = TestEnvironment::new();
        let service = build_test_workspace_service(env.path_manager.clone()).await;
        let persistence_manager =
            PersistenceManager::new(env.path_manager.clone()).expect("persistence manager should initialize");

        let first_workspace_root = env.create_workspace_dir("workspace-one");
        let second_workspace_root = env.create_workspace_dir("workspace-two");

        let first_workspace = WorkspaceInfo::new(
            first_workspace_root.clone(),
            WorkspaceOpenOptions {
                auto_set_current: false,
                ..Default::default()
            },
        )
        .await
        .expect("first workspace should initialize");
        let second_workspace = WorkspaceInfo::new(
            second_workspace_root.clone(),
            WorkspaceOpenOptions {
                auto_set_current: false,
                ..Default::default()
            },
        )
        .await
        .expect("second workspace should initialize");

        let legacy_session = SessionMetadata::new(
            Uuid::new_v4().to_string(),
            "Legacy Session".to_string(),
            "agent".to_string(),
            "model".to_string(),
        );
        persistence_manager
            .save_session_metadata(&second_workspace_root, &legacy_session)
            .await
            .expect("legacy session metadata should save");

        let second_runtime = persistence_manager
            .runtime_service()
            .context_for_local_workspace(&second_workspace_root);
        let legacy_sessions_root = second_workspace_root.join(".northhing").join("sessions");
        std::fs::create_dir_all(&legacy_sessions_root).expect("legacy sessions root should be created");
        std::fs::rename(
            second_runtime.sessions_dir.join(&legacy_session.session_id),
            legacy_sessions_root.join(&legacy_session.session_id),
        )
        .expect("session directory should move to legacy path");
        let _ = std::fs::remove_dir_all(&second_runtime.runtime_root);

        let first_runtime = service
            .runtime_service
            .context_for_local_workspace(&first_workspace_root);
        assert!(
            !first_runtime.runtime_root.exists(),
            "startup should begin without a runtime root for the first workspace"
        );
        assert!(
            !second_runtime.runtime_root.exists(),
            "startup should begin without a runtime root for the second workspace"
        );

        let workspace_data = WorkspacePersistenceData {
            workspaces: HashMap::from([
                (first_workspace.id.clone(), first_workspace.clone()),
                (second_workspace.id.clone(), second_workspace.clone()),
            ]),
            opened_workspace_ids: vec![first_workspace.id.clone(), second_workspace.id.clone()],
            current_workspace_id: Some(first_workspace.id.clone()),
            recent_workspaces: vec![first_workspace.id.clone(), second_workspace.id.clone()],
            recent_assistant_workspaces: Vec::new(),
            saved_at: chrono::Utc::now(),
        };

        service
            .persistence
            .save_json("workspace_data", &workspace_data, StorageOptions::default())
            .await
            .expect("workspace data should save");

        service
            .load_workspace_history_only()
            .await
            .expect("workspace history should restore");

        let restored_current = service
            .current_workspace()
            .await
            .expect("current workspace should be restored");
        assert_eq!(restored_current.id, first_workspace.id);
        assert!(
            first_runtime.runtime_root.exists(),
            "active workspace runtime should be ensured on startup"
        );
        assert!(
            second_runtime.sessions_dir.join(&legacy_session.session_id).exists(),
            "non-active opened workspace sessions should migrate into the shared runtime root"
        );

        let restored_sessions = persistence_manager
            .list_session_metadata(&second_workspace_root)
            .await
            .expect("restored workspace sessions should list successfully");
        assert_eq!(restored_sessions.len(), 1);
        assert_eq!(restored_sessions[0].session_id, legacy_session.session_id);
        assert!(
            !legacy_sessions_root.join(&legacy_session.session_id).exists(),
            "legacy session directory should be removed after startup migration"
        );
    }

    #[tokio::test]
    async fn track_workspace_activity_registers_without_opening_workspace() {
        let env = TestEnvironment::new();
        let service = build_test_workspace_service(env.path_manager.clone()).await;
        let workspace_root = env.create_workspace_dir("tracked-workspace");

        let tracked = service
            .track_workspace_activity(workspace_root.clone(), WorkspaceCreateOptions::default())
            .await
            .expect("workspace tracking should succeed");

        let tracked_by_path = service
            .get_workspace_by_path(&workspace_root)
            .await
            .expect("tracked workspace should be queryable by path");
        assert_eq!(tracked_by_path.id, tracked.id);

        let recent = service.recent_workspaces().await;
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, tracked.id);

        assert!(
            service.get_opened_workspaces().await.is_empty(),
            "tracked workspace activity should not add the workspace to the opened UI list"
        );
        assert!(
            service.current_workspace().await.is_none(),
            "tracked workspace activity should not change the current workspace"
        );
    }

    #[tokio::test]
    async fn track_workspace_activity_assigns_stable_remote_workspace_id() {
        let env = TestEnvironment::new();
        let service = build_test_workspace_service(env.path_manager.clone()).await;
        let remote_workspace_root = PathBuf::from("/srv/northhing/project");

        let tracked = service
            .track_workspace_activity(
                remote_workspace_root.clone(),
                WorkspaceCreateOptions {
                    workspace_kind: WorkspaceKind::Remote,
                    remote_connection_id: Some("conn-1".to_string()),
                    remote_ssh_host: Some("example-host".to_string()),
                    ..Default::default()
                },
            )
            .await
            .expect("remote workspace tracking should succeed");

        assert_eq!(
            tracked.id,
            remote_workspace_stable_id("example-host", "/srv/northhing/project")
        );
        assert_eq!(tracked.root_path, remote_workspace_root);
        assert!(service.get_opened_workspaces().await.is_empty());
    }

    #[test]
    fn normalize_related_path_description_treats_blank_as_none() {
        assert_eq!(WorkspaceService::normalize_related_path_description(None), None);
        assert_eq!(
            WorkspaceService::normalize_related_path_description(Some("".to_string())),
            None
        );
        assert_eq!(
            WorkspaceService::normalize_related_path_description(Some("   ".to_string())),
            None
        );
        assert_eq!(
            WorkspaceService::normalize_related_path_description(Some(
                " Legacy TypeScript implementation ".to_string()
            )),
            Some("Legacy TypeScript implementation".to_string())
        );
    }
}
