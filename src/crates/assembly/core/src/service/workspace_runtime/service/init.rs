use super::super::types::{WorkspaceRuntimeContext, WorkspaceRuntimeEnsureResult, WorkspaceRuntimeTarget};
use super::state::runtime_lock_for;
use super::state::{RuntimeMigrationSpec, WorkspaceRuntimeService};
#[cfg(feature = "product-full")]
use crate::agentic::WorkspaceBinding;
use crate::infrastructure::{path_manager_arc, PathManager};
use crate::service::remote_ssh::workspace_state::normalize_remote_workspace_path;
use crate::util::errors::NortHingResult;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use tracing::debug;

impl WorkspaceRuntimeService {
    pub fn new(path_manager: Arc<PathManager>) -> Self {
        Self {
            path_manager,
            verified_runtime_roots: Mutex::new(HashSet::new()),
        }
    }

    pub fn path_manager(&self) -> &Arc<PathManager> {
        &self.path_manager
    }

    pub fn context_for_target(&self, target: WorkspaceRuntimeTarget) -> WorkspaceRuntimeContext {
        match target {
            WorkspaceRuntimeTarget::LocalWorkspace { workspace_root } => {
                self.context_for_local_workspace(&workspace_root)
            }
            WorkspaceRuntimeTarget::RemoteWorkspaceMirror { ssh_host, remote_root } => {
                self.context_for_remote_workspace(&ssh_host, &remote_root)
            }
        }
    }

    pub fn context_for_local_workspace(&self, workspace_path: &Path) -> WorkspaceRuntimeContext {
        WorkspaceRuntimeContext::new(
            WorkspaceRuntimeTarget::LocalWorkspace {
                workspace_root: workspace_path.to_path_buf(),
            },
            self.path_manager.project_runtime_root(workspace_path),
        )
    }

    pub fn context_for_remote_workspace(&self, ssh_host: &str, remote_root: &str) -> WorkspaceRuntimeContext {
        let normalized_remote_root = normalize_remote_workspace_path(remote_root);
        WorkspaceRuntimeContext::new(
            WorkspaceRuntimeTarget::RemoteWorkspaceMirror {
                ssh_host: ssh_host.to_string(),
                remote_root: normalized_remote_root.clone(),
            },
            self.remote_workspace_runtime_root(ssh_host, &normalized_remote_root),
        )
    }

    pub async fn ensure_workspace_runtime(
        &self,
        target: WorkspaceRuntimeTarget,
    ) -> NortHingResult<WorkspaceRuntimeEnsureResult> {
        let context = self.context_for_target(target);
        let migration_specs = self.migration_specs_for_context(&context);
        self.ensure_runtime_context(context, migration_specs).await
    }

    pub async fn ensure_local_workspace_runtime(
        &self,
        workspace_path: &Path,
    ) -> NortHingResult<WorkspaceRuntimeEnsureResult> {
        self.ensure_workspace_runtime(WorkspaceRuntimeTarget::LocalWorkspace {
            workspace_root: workspace_path.to_path_buf(),
        })
        .await
    }

    pub async fn ensure_remote_workspace_runtime(
        &self,
        ssh_host: &str,
        remote_root: &str,
    ) -> NortHingResult<WorkspaceRuntimeEnsureResult> {
        self.ensure_workspace_runtime(WorkspaceRuntimeTarget::RemoteWorkspaceMirror {
            ssh_host: ssh_host.to_string(),
            remote_root: remote_root.to_string(),
        })
        .await
    }

    #[cfg(feature = "product-full")]
    pub async fn ensure_runtime_for_workspace_binding(
        &self,
        workspace: &WorkspaceBinding,
    ) -> NortHingResult<WorkspaceRuntimeEnsureResult> {
        if workspace.is_remote() {
            self.ensure_remote_workspace_runtime(
                &workspace.session_identity.hostname,
                workspace.session_identity.logical_workspace_path(),
            )
            .await
        } else {
            self.ensure_local_workspace_runtime(workspace.root_path()).await
        }
    }

    pub(crate) async fn ensure_runtime_context(
        &self,
        context: WorkspaceRuntimeContext,
        migration_specs: Vec<RuntimeMigrationSpec>,
    ) -> NortHingResult<WorkspaceRuntimeEnsureResult> {
        if self.is_runtime_verified(&context.runtime_root) {
            return Ok(super::format::cached_ensure_result(context));
        }

        let runtime_lock = runtime_lock_for(&context.runtime_root);
        let _guard = runtime_lock.lock().await;

        if self.is_runtime_verified(&context.runtime_root) {
            return Ok(super::format::cached_ensure_result(context));
        }

        let migrated_entries = self.apply_migration_specs(&migration_specs).await?;
        self.cleanup_legacy_artifacts_for_context(&context).await?;

        let mut created_directories = Vec::new();
        for dir in context.required_directories() {
            if !dir.exists() {
                self.path_manager.ensure_dir(dir).await?;
                created_directories.push(dir.to_path_buf());
            }
        }

        if !context.layout_state_file.exists() || !created_directories.is_empty() || !migrated_entries.is_empty() {
            self.persist_layout_state(&context, &migrated_entries).await?;
        }

        self.mark_runtime_verified(&context.runtime_root);

        if !created_directories.is_empty() || !migrated_entries.is_empty() {
            debug!(
                "Workspace runtime ensured: root={} created_dirs={} migrated_entries={}",
                context.runtime_root.display(),
                created_directories.len(),
                migrated_entries.len()
            );
        }

        Ok(WorkspaceRuntimeEnsureResult {
            context,
            created_directories,
            migrated_entries,
        })
    }
}

static GLOBAL_WORKSPACE_RUNTIME_SERVICE: OnceLock<Arc<WorkspaceRuntimeService>> = OnceLock::new();

fn init_global_workspace_runtime_service() -> Arc<WorkspaceRuntimeService> {
    Arc::new(WorkspaceRuntimeService::new(path_manager_arc()))
}

pub fn workspace_runtime_service_arc() -> Arc<WorkspaceRuntimeService> {
    GLOBAL_WORKSPACE_RUNTIME_SERVICE
        .get_or_init(init_global_workspace_runtime_service)
        .clone()
}

pub fn try_get_workspace_runtime_service_arc() -> NortHingResult<Arc<WorkspaceRuntimeService>> {
    Ok(workspace_runtime_service_arc())
}
#[cfg(test)]
mod tests {
    use super::WorkspaceRuntimeService;
    use crate::infrastructure::PathManager;
    use crate::service::session::{SessionMetadata, StoredSessionIndexFile, StoredSessionMetadataFile};
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;

    #[tokio::test]
    async fn ensure_local_workspace_runtime_creates_complete_layout_without_project_dot_dir() {
        let test_root = std::env::temp_dir().join(format!("northhing-runtime-test-{}", Uuid::new_v4()));
        let workspace_root = test_root.join("workspace");
        fs::create_dir_all(&workspace_root).expect("workspace should exist");

        let path_manager = Arc::new(PathManager::with_user_root_for_tests(test_root.join("user")));
        let service = WorkspaceRuntimeService::new(path_manager.clone());

        let ensured = service
            .ensure_local_workspace_runtime(&workspace_root)
            .await
            .expect("runtime should be ensured");

        let context = ensured.context;
        assert!(context.runtime_root.exists());
        assert!(context.sessions_dir.exists());
        assert!(context.request_traces_dir.exists());
        assert!(context.snapshot_by_hash_dir.exists());
        assert!(context.snapshot_metadata_dir.exists());
        assert!(context.snapshot_baselines_dir.exists());
        assert!(context.snapshot_operations_dir.exists());
        assert!(context.locks_dir.exists());
        assert!(context.layout_state_file.exists());
        assert!(!path_manager.project_root(&workspace_root).join("context").exists());

        let _ = fs::remove_dir_all(&test_root);
    }

    #[tokio::test]
    async fn ensure_local_workspace_runtime_migrates_legacy_runtime_entries() {
        let test_root = std::env::temp_dir().join(format!("northhing-runtime-test-{}", Uuid::new_v4()));
        let workspace_root = test_root.join("workspace");
        let legacy_root = workspace_root.join(".northhing");
        fs::create_dir_all(legacy_root.join("sessions")).expect("legacy sessions should exist");
        fs::write(legacy_root.join("sessions").join("s1.json"), "{}").expect("legacy session file should be written");

        let path_manager = Arc::new(PathManager::with_user_root_for_tests(test_root.join("user")));
        let service = WorkspaceRuntimeService::new(path_manager.clone());

        let ensured = service
            .ensure_local_workspace_runtime(&workspace_root)
            .await
            .expect("runtime should be ensured");

        assert!(ensured.context.sessions_dir.join("s1.json").exists());
        assert!(!legacy_root.join("sessions").exists());
        assert_eq!(ensured.migrated_entries.len(), 1);

        let _ = fs::remove_dir_all(&test_root);
    }

    #[tokio::test]
    async fn ensure_remote_workspace_runtime_merges_legacy_sessions_only() {
        let test_root = std::env::temp_dir().join(format!("northhing-runtime-test-{}", Uuid::new_v4()));
        let path_manager = Arc::new(PathManager::with_user_root_for_tests(test_root.join("user")));
        let service = WorkspaceRuntimeService::new(path_manager);

        let context = service.context_for_remote_workspace("example-host", "/root/repo");
        let legacy_sessions_root = context
            .runtime_root
            .join("sessions")
            .join(".northhing")
            .join("sessions");

        fs::create_dir_all(&legacy_sessions_root).expect("legacy remote sessions should exist");
        fs::create_dir_all(context.sessions_dir.join("existing-session")).expect("new sessions root should exist");

        let mut newer_metadata = SessionMetadata::new(
            "existing-session".to_string(),
            "Existing Session".to_string(),
            "agent".to_string(),
            "model".to_string(),
        );
        newer_metadata.last_active_at = 200;
        write_session_metadata(&context.sessions_dir.join("existing-session"), &newer_metadata);

        let mut older_metadata = newer_metadata.clone();
        older_metadata.last_active_at = 100;
        write_session_metadata(&legacy_sessions_root.join("existing-session"), &older_metadata);
        fs::create_dir_all(legacy_sessions_root.join("legacy-session")).expect("legacy-only session dir should exist");
        let mut legacy_only_metadata = SessionMetadata::new(
            "legacy-session".to_string(),
            "Legacy Session".to_string(),
            "agent".to_string(),
            "model".to_string(),
        );
        legacy_only_metadata.last_active_at = 150;
        write_session_metadata(&legacy_sessions_root.join("legacy-session"), &legacy_only_metadata);
        fs::create_dir_all(legacy_sessions_root.join("hidden-session"))
            .expect("hidden legacy session dir should exist");
        let mut hidden_metadata = SessionMetadata::new(
            "hidden-session".to_string(),
            "Hidden Session".to_string(),
            "agent".to_string(),
            "model".to_string(),
        );
        hidden_metadata.session_kind = northhing_core_types::SessionKind::Subagent;
        hidden_metadata.last_active_at = 250;
        write_session_metadata(&legacy_sessions_root.join("hidden-session"), &hidden_metadata);
        write_session_index(
            &legacy_sessions_root.join("index.json"),
            vec![
                hidden_metadata.clone(),
                older_metadata.clone(),
                legacy_only_metadata.clone(),
            ],
        );
        write_session_index(&context.sessions_dir.join("index.json"), vec![newer_metadata.clone()]);

        let ensured = service
            .ensure_remote_workspace_runtime("example-host", "/root/repo")
            .await
            .expect("remote runtime should be ensured");

        assert!(context.sessions_dir.join("legacy-session").exists());
        assert!(context.sessions_dir.join("existing-session").exists());
        assert!(
            !legacy_sessions_root.exists(),
            "legacy sessions root should be removed after merge"
        );

        let merged_metadata: StoredSessionMetadataFile = serde_json::from_slice(
            &fs::read(context.sessions_dir.join("existing-session").join("metadata.json"))
                .expect("merged metadata should exist"),
        )
        .expect("merged metadata should deserialize");
        assert_eq!(merged_metadata.metadata.last_active_at, 200);

        let merged_index: StoredSessionIndexFile = serde_json::from_slice(
            &fs::read(context.sessions_dir.join("index.json")).expect("merged session index should exist"),
        )
        .expect("merged session index should deserialize");
        assert_eq!(merged_index.sessions.len(), 2);
        assert_eq!(merged_index.metadata_file_count, 3);
        assert!(merged_index
            .sessions
            .iter()
            .all(|metadata| metadata.session_id != "hidden-session"));
        assert!(ensured
            .migrated_entries
            .iter()
            .any(|record| record.strategy == "merge_sessions"));
        assert_eq!(ensured.migrated_entries.len(), 1);

        let _ = fs::remove_dir_all(&test_root);
    }

    #[tokio::test]
    async fn ensure_local_workspace_runtime_uses_verified_cache_on_repeat_calls() {
        let test_root = std::env::temp_dir().join(format!("northhing-runtime-test-{}", Uuid::new_v4()));
        let workspace_root = test_root.join("workspace");
        fs::create_dir_all(&workspace_root).expect("workspace should exist");

        let path_manager = Arc::new(PathManager::with_user_root_for_tests(test_root.join("user")));
        let service = WorkspaceRuntimeService::new(path_manager);

        let first = service
            .ensure_local_workspace_runtime(&workspace_root)
            .await
            .expect("first ensure should succeed");
        let first_modified = fs::metadata(&first.context.layout_state_file)
            .expect("layout state should exist")
            .modified()
            .expect("layout state should have modified time");

        tokio::time::sleep(Duration::from_millis(20)).await;

        let second = service
            .ensure_local_workspace_runtime(&workspace_root)
            .await
            .expect("second ensure should succeed");
        let second_modified = fs::metadata(&second.context.layout_state_file)
            .expect("layout state should still exist")
            .modified()
            .expect("layout state should have modified time");

        assert!(second.created_directories.is_empty());
        assert!(second.migrated_entries.is_empty());
        assert_eq!(first_modified, second_modified);

        let _ = fs::remove_dir_all(&test_root);
    }

    fn write_session_metadata(session_dir: &Path, metadata: &SessionMetadata) {
        fs::create_dir_all(session_dir).expect("session dir should exist");
        let stored = StoredSessionMetadataFile::new(metadata.clone());
        fs::write(
            session_dir.join("metadata.json"),
            serde_json::to_string_pretty(&stored).expect("metadata should serialize"),
        )
        .expect("metadata should write");
    }

    fn write_session_index(path: &Path, sessions: Vec<SessionMetadata>) {
        let index = StoredSessionIndexFile::new(0, sessions);
        fs::write(
            path,
            serde_json::to_string_pretty(&index).expect("index should serialize"),
        )
        .expect("index should write");
    }
}
