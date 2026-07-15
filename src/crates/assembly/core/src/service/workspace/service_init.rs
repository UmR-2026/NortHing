//! Construction + startup helpers for [`super::WorkspaceService`].
//!
//! Owns the construction delegations (`new`, `with_config`, `path_manager`,
//! `persistence`, `runtime_service`, `get_manager`) and the helpers used when
//! restoring workspaces on startup (`collect_startup_restored_workspaces`,
//! `prepare_startup_restored_workspaces`, `ensure_workspace_gitignore_best_effort`,
//! `ensure_workspace_runtime_best_effort`).

use super::manager::{WorkspaceInfo, WorkspaceKind, WorkspaceManagerConfig};
use super::service::WorkspaceService;
use crate::service::bootstrap::ensure_workspace_gitignore_ignores_northhing;
use crate::util::errors::*;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

impl WorkspaceService {
    pub(super) fn collect_startup_restored_workspaces(
        manager: &super::manager::WorkspaceManager,
    ) -> Vec<WorkspaceInfo> {
        let mut targets = Vec::new();
        let mut seen_workspace_ids = HashSet::new();

        if let Some(workspace) = manager.current_workspace() {
            Self::push_startup_restored_workspace(&mut targets, &mut seen_workspace_ids, workspace);
        }

        for workspace in manager.opened_workspace_infos() {
            Self::push_startup_restored_workspace(&mut targets, &mut seen_workspace_ids, workspace);
        }

        targets
    }

    pub(super) fn push_startup_restored_workspace(
        targets: &mut Vec<WorkspaceInfo>,
        seen_workspace_ids: &mut HashSet<String>,
        workspace: &WorkspaceInfo,
    ) {
        if seen_workspace_ids.insert(workspace.id.clone()) {
            targets.push(workspace.clone());
        }
    }

    pub(super) async fn prepare_startup_restored_workspaces(&self, workspaces: Vec<WorkspaceInfo>) {
        for workspace in workspaces {
            self.ensure_workspace_gitignore_best_effort(&workspace, "restored")
                .await;
            self.ensure_workspace_runtime_best_effort(&workspace, "restored").await;
        }
    }

    pub(super) async fn ensure_workspace_gitignore_best_effort(&self, workspace: &WorkspaceInfo, trigger: &str) {
        if workspace.workspace_kind == WorkspaceKind::Remote || !workspace.root_path.exists() {
            return;
        }

        if let Err(e) = ensure_workspace_gitignore_ignores_northhing(&workspace.root_path).await {
            warn!(
                "Failed to ensure workspace .gitignore ignores .northhing: workspace_path={} trigger={} error={}",
                workspace.root_path.display(),
                trigger,
                e
            );
        }
    }

    pub(super) async fn ensure_workspace_runtime_best_effort(&self, workspace: &WorkspaceInfo, trigger: &str) {
        let result = match workspace.workspace_kind {
            WorkspaceKind::Remote => {
                let Some(ssh_host) = workspace
                    .metadata
                    .get("sshHost")
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                else {
                    warn!(
                        "Skipping remote runtime ensure due to missing sshHost: workspace_id={} trigger={}",
                        workspace.id, trigger
                    );
                    return;
                };

                self.runtime_service
                    .ensure_remote_workspace_runtime(ssh_host, &workspace.root_path.to_string_lossy())
                    .await
            }
            _ => {
                if !workspace.root_path.exists() {
                    return;
                }

                self.runtime_service
                    .ensure_local_workspace_runtime(&workspace.root_path)
                    .await
            }
        };

        if let Err(e) = result {
            warn!(
                "Failed to initialize workspace runtime: workspace_path={} trigger={} error={}",
                workspace.root_path.display(),
                trigger,
                e
            );
        }
    }

    /// Creates a new workspace service.
    pub async fn new() -> NortHingResult<Self> {
        Self::new_impl().await
    }

    /// Creates a workspace service with a custom configuration.
    pub async fn with_config(config: WorkspaceManagerConfig) -> NortHingResult<Self> {
        Self::with_config_impl(config).await
    }

    /// Returns the path manager.
    pub fn path_manager(&self) -> &Arc<crate::infrastructure::PathManager> {
        &self.path_manager
    }

    /// Returns the persistence service.
    pub fn persistence(&self) -> &Arc<crate::infrastructure::storage::PersistenceService> {
        &self.persistence
    }

    pub fn runtime_service(&self) -> &Arc<crate::service::workspace_runtime::WorkspaceRuntimeService> {
        &self.runtime_service
    }

    /// Returns the underlying `WorkspaceManager` handle.
    /// Used to share workspace state with other services (e.g. Agent).
    pub fn manager(&self) -> Arc<RwLock<super::manager::WorkspaceManager>> {
        self.get_manager_impl()
    }
}
