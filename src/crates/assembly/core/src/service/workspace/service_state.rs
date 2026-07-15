//! Read-only workspace queries for [`super::WorkspaceService`].
//!
//! Includes current/active workspace lookup, listing, searching, statistics,
//! health checks, export/import, and the assistant-workspace classifier.

use super::manager::{WorkspaceInfo, WorkspaceStatus, WorkspaceSummary, WorkspaceType};
use super::service::WorkspaceService;
use super::service_types::{WorkspaceHealthStatus, WorkspaceQuickSummary};
use crate::util::errors::*;
use std::path::{Path, PathBuf};

impl WorkspaceService {
    /// Returns the current workspace.
    pub async fn current_workspace(&self) -> Option<WorkspaceInfo> {
        self.get_current_workspace_impl().await
    }

    /// Best-effort synchronous read for contexts that cannot `await`.
    pub fn try_get_current_workspace_path(&self) -> Option<PathBuf> {
        self.try_get_current_workspace_path_impl()
    }

    /// Returns workspace details.
    pub async fn get_workspace(&self, workspace_id: &str) -> Option<WorkspaceInfo> {
        self.get_workspace_impl(workspace_id).await
    }

    /// Returns workspace details by root path.
    pub async fn get_workspace_by_path(&self, path: &Path) -> Option<WorkspaceInfo> {
        self.get_workspace_by_path_impl(path).await
    }

    /// Returns all currently opened workspaces.
    pub async fn get_opened_workspaces(&self) -> Vec<WorkspaceInfo> {
        self.get_opened_workspaces_impl().await
    }

    /// All tracked workspaces with full metadata (insights, maintenance, etc.).
    pub async fn list_workspace_infos(&self) -> Vec<WorkspaceInfo> {
        self.list_workspace_infos_impl().await
    }

    /// `metadata["sshHost"]` for a remote workspace matching `connection_id` and normalized remote root.
    ///
    /// Used when session APIs receive `remote_connection_id` but the client omitted `remote_ssh_host`:
    /// session files live under `~/.northhing/remote_ssh/{sshHost}/...`, not the legacy per-connection tree.
    /// This reads only persisted workspace records (no filesystem guessing, no DNS).
    pub async fn remote_ssh_host_for_remote_workspace(
        &self,
        connection_id: &str,
        remote_workspace_path: &str,
    ) -> Option<String> {
        self.remote_ssh_host_for_remote_workspace_impl(connection_id, remote_workspace_path)
            .await
    }

    /// Returns all tracked assistant workspaces, including inactive ones.
    pub async fn get_assistant_workspaces(&self) -> Vec<WorkspaceInfo> {
        self.get_assistant_workspaces_impl().await
    }

    /// Lists all workspaces.
    pub async fn list_workspaces(&self) -> Vec<WorkspaceSummary> {
        self.list_workspaces_impl().await
    }

    /// Lists workspaces by type.
    pub async fn list_workspaces_by_type(&self, workspace_type: WorkspaceType) -> Vec<WorkspaceSummary> {
        self.list_workspaces_by_type_impl(workspace_type).await
    }

    /// Lists workspaces by status.
    pub async fn list_workspaces_by_status(&self, status: WorkspaceStatus) -> Vec<WorkspaceSummary> {
        self.list_workspaces_by_status_impl(status).await
    }

    /// Returns recently accessed workspaces.
    pub async fn recent_workspaces(&self) -> Vec<WorkspaceInfo> {
        self.get_recent_workspaces_impl().await
    }

    /// Returns recently accessed assistant workspaces.
    pub async fn recent_assistant_workspaces(&self) -> Vec<WorkspaceInfo> {
        self.get_recent_assistant_workspaces_impl().await
    }

    /// Searches workspaces.
    pub async fn search_workspaces(&self, query: &str) -> Vec<WorkspaceSummary> {
        self.search_workspaces_impl(query).await
    }

    /// Returns statistics.
    pub async fn statistics(&self) -> super::manager::WorkspaceManagerStatistics {
        self.get_statistics_impl().await
    }

    /// Returns the workspace count.
    pub async fn workspace_count(&self) -> usize {
        self.get_workspace_count_impl().await
    }

    /// Runs a health check.
    pub async fn health_check(&self) -> NortHingResult<WorkspaceHealthStatus> {
        self.health_check_impl().await
    }

    /// Returns a quick summary.
    pub async fn get_quick_summary(&self) -> WorkspaceQuickSummary {
        self.get_quick_summary_impl().await
    }

    /// Returns whether a path is a managed assistant workspace.
    pub fn is_assistant_workspace_path(&self, path: &Path) -> bool {
        self.is_assistant_workspace_path_impl(path)
    }
}
