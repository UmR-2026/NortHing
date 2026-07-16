//! Workspace accessors sub-domain (R23b Mavis take-over).
//!
//! Owns the 15 workspace accessor methods (get/list/search) that previously
//! lived on `service.rs` L304-485. Extracted via Mavis take-over after
//! R23a/b/c/d 4 producer parallel sub-rounds timed out at the 30min cap.
//!
//! Visibility pattern (R20 `manager_*.rs` precedent + R22 E0592 lesson):
//! sibling method names use the `_impl` suffix so they do not collide with
//! the facade delegates that stay on `service.rs`. The facade keeps the
//! public cross-crate API; this file's `pub(super)` items are only callable
//! from inside the `workspace` module.

use super::manager::{WorkspaceInfo, WorkspaceStatus, WorkspaceSummary, WorkspaceType};
use super::WorkspaceService;
use crate::service::remote_ssh::normalize_remote_workspace_path;
use crate::service::remote_ssh::workspace_state::local_workspace_roots_equal;
use crate::util::errors::*;
use std::path::{Path, PathBuf};

impl WorkspaceService {
    /// Returns the current workspace.
    pub(super) async fn get_current_workspace_impl(&self) -> Option<WorkspaceInfo> {
        let manager = self.manager.read().await;
        manager.current_workspace().cloned()
    }

    /// Best-effort synchronous read for contexts that cannot `await`.
    pub(super) fn try_get_current_workspace_path_impl(&self) -> Option<PathBuf> {
        self.manager
            .try_read()
            .ok()
            .and_then(|manager| manager.current_workspace().map(|workspace| workspace.root_path.clone()))
    }

    /// Returns workspace details.
    pub(super) async fn get_workspace_impl(&self, workspace_id: &str) -> Option<WorkspaceInfo> {
        let manager = self.manager.read().await;
        manager.get_workspace(workspace_id).cloned()
    }

    /// Returns workspace details by root path.
    pub(super) async fn get_workspace_by_path_impl(&self, path: &Path) -> Option<WorkspaceInfo> {
        let manager = self.manager.read().await;
        manager
            .workspaces()
            .values()
            .find(|workspace| {
                if workspace.workspace_kind == super::manager::WorkspaceKind::Remote {
                    workspace.root_path == path
                } else {
                    local_workspace_roots_equal(&workspace.root_path, path)
                }
            })
            .cloned()
    }

    /// Returns all currently opened workspaces.
    pub(super) async fn get_opened_workspaces_impl(&self) -> Vec<WorkspaceInfo> {
        let manager = self.manager.read().await;
        manager.opened_workspace_infos().into_iter().cloned().collect()
    }

    /// All tracked workspaces with full metadata (insights, maintenance, etc.).
    pub(super) async fn list_workspace_infos_impl(&self) -> Vec<WorkspaceInfo> {
        let manager = self.manager.read().await;
        manager.workspaces().values().cloned().collect()
    }

    /// `metadata["sshHost"]` for a remote workspace matching `connection_id` and normalized remote root.
    pub(super) async fn remote_ssh_host_for_remote_workspace_impl(
        &self,
        connection_id: &str,
        remote_workspace_path: &str,
    ) -> Option<String> {
        use super::manager::WorkspaceKind;
        let cid = connection_id.trim();
        if cid.is_empty() {
            return None;
        }
        let want = normalize_remote_workspace_path(remote_workspace_path);
        let manager = self.manager.read().await;
        for w in manager.workspaces().values() {
            if w.workspace_kind != WorkspaceKind::Remote {
                continue;
            }
            let wcid = w.remote_ssh_connection_id()?;
            if wcid != cid {
                continue;
            }
            let root = normalize_remote_workspace_path(&w.root_path.to_string_lossy());
            if root != want {
                continue;
            }
            let host = w
                .metadata
                .get("sshHost")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())?;
            return Some(host.to_string());
        }
        None
    }

    /// Returns all tracked assistant workspaces, including inactive ones.
    pub(super) async fn get_assistant_workspaces_impl(&self) -> Vec<WorkspaceInfo> {
        let manager = self.manager.read().await;
        manager
            .workspaces()
            .values()
            .filter(|workspace| workspace.workspace_kind == super::manager::WorkspaceKind::Assistant)
            .cloned()
            .collect()
    }

    /// Lists all workspaces.
    pub(super) async fn list_workspaces_impl(&self) -> Vec<WorkspaceSummary> {
        let manager = self.manager.read().await;
        manager.list_workspaces()
    }

    /// Lists workspaces by type.
    pub(super) async fn list_workspaces_by_type_impl(&self, workspace_type: WorkspaceType) -> Vec<WorkspaceSummary> {
        let manager = self.manager.read().await;
        manager
            .list_workspaces()
            .into_iter()
            .filter(|ws| ws.workspace_type == workspace_type)
            .collect()
    }

    /// Lists workspaces by status.
    pub(super) async fn list_workspaces_by_status_impl(&self, status: WorkspaceStatus) -> Vec<WorkspaceSummary> {
        let manager = self.manager.read().await;
        manager
            .list_workspaces()
            .into_iter()
            .filter(|ws| ws.status == status)
            .collect()
    }

    /// Returns recently accessed workspaces.
    pub(super) async fn get_recent_workspaces_impl(&self) -> Vec<WorkspaceInfo> {
        let manager = self.manager.read().await;
        let recent_ids = manager.recent_workspaces();
        let mut recent_workspaces = Vec::new();

        for workspace_id in recent_ids {
            if let Some(workspace) = manager.workspaces().get(workspace_id) {
                recent_workspaces.push(workspace.clone());
            }
        }

        recent_workspaces
    }

    /// Returns recently accessed assistant workspaces.
    pub(super) async fn get_recent_assistant_workspaces_impl(&self) -> Vec<WorkspaceInfo> {
        let manager = self.manager.read().await;
        let recent_ids = manager.recent_assistant_workspaces();
        let mut recent_workspaces = Vec::new();

        for workspace_id in recent_ids {
            if let Some(workspace) = manager.workspaces().get(workspace_id) {
                recent_workspaces.push(workspace.clone());
            }
        }

        recent_workspaces
    }

    /// Drops a workspace from recent lists only (workspace record and open state unchanged).
    pub(super) async fn remove_workspace_from_recent_impl(&self, workspace_id: &str) -> NortHingResult<()> {
        let changed = {
            let mut manager = self.manager.write().await;
            manager.remove_from_recent_workspaces_only(workspace_id)
        };
        if changed {
            self.save_workspace_data().await?;
        }
        Ok(())
    }

    /// Searches workspaces.
    pub(super) async fn search_workspaces_impl(&self, query: &str) -> Vec<WorkspaceSummary> {
        let manager = self.manager.read().await;
        manager.search_workspaces(query)
    }
}
