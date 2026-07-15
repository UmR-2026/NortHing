//! Mutating workspace operations for [`super::WorkspaceService`].
//!
//! Includes open/close/create/switch/reorder/track flows plus the
//! related-paths normalizer used by [`super::service_types::WorkspaceInfoUpdates`].

use super::manager::{RelatedPath, WorkspaceInfo, WorkspaceKind};
use super::service::WorkspaceService;
use super::service_types::{
    BatchImportResult, BatchRemoveResult, WorkspaceCreateOptions, WorkspaceExport, WorkspaceIdentityChangedEvent,
    WorkspaceImportResult, WorkspaceInfoUpdates,
};
use crate::service::remote_ssh::workspace_state::{
    canonicalize_local_workspace_root, normalize_remote_workspace_path, remote_workspace_manager,
};
use crate::util::errors::*;
use std::collections::HashSet;
use std::path::PathBuf;

impl WorkspaceService {
    /// Opens a workspace.
    pub async fn open_workspace(&self, path: PathBuf) -> NortHingResult<WorkspaceInfo> {
        self.open_workspace_impl(path).await
    }

    /// Opens a workspace with explicit workspace metadata.
    pub async fn open_workspace_with_options(
        &self,
        path: PathBuf,
        options: WorkspaceCreateOptions,
    ) -> NortHingResult<WorkspaceInfo> {
        self.open_workspace_with_options_impl(path, options).await
    }

    /// Registers or refreshes workspace activity without marking it as opened in the UI.
    pub async fn track_workspace_activity(
        &self,
        path: PathBuf,
        options: WorkspaceCreateOptions,
    ) -> NortHingResult<WorkspaceInfo> {
        self.track_workspace_activity_impl(path, options).await
    }

    /// Quickly opens a workspace (using default options).
    pub async fn quick_open(&self, path: &str) -> NortHingResult<WorkspaceInfo> {
        self.quick_open_impl(path).await
    }

    /// Creates a workspace (for a new project).
    pub async fn create_workspace(
        &self,
        path: PathBuf,
        options: WorkspaceCreateOptions,
    ) -> NortHingResult<WorkspaceInfo> {
        self.create_workspace_impl(path, options).await
    }

    /// Creates and opens a new assistant workspace, then sets it as current.
    pub async fn create_assistant_workspace(&self, assistant_id: Option<String>) -> NortHingResult<WorkspaceInfo> {
        self.create_assistant_workspace_impl(assistant_id).await
    }

    /// Closes the current workspace.
    pub async fn close_current_workspace(&self) -> NortHingResult<()> {
        self.close_current_workspace_impl().await
    }

    /// Closes the specified workspace.
    pub async fn close_workspace(&self, workspace_id: &str) -> NortHingResult<()> {
        self.close_workspace_impl(workspace_id).await
    }

    /// Sets the active workspace from the opened workspace list.
    pub async fn set_active_workspace(&self, workspace_id: &str) -> NortHingResult<()> {
        self.set_active_workspace_impl(workspace_id).await
    }

    /// Reorders the opened workspaces without changing active or recent state.
    pub async fn reorder_opened_workspaces(&self, workspace_ids: Vec<String>) -> NortHingResult<()> {
        self.reorder_opened_workspaces_impl(workspace_ids).await
    }

    /// Switches to the specified workspace.
    pub async fn switch_to_workspace(&self, workspace_id: &str) -> NortHingResult<()> {
        self.switch_to_workspace_impl(workspace_id).await
    }

    /// Removes a workspace.
    pub async fn remove_workspace(&self, workspace_id: &str) -> NortHingResult<()> {
        self.remove_workspace_impl(workspace_id).await
    }

    /// Removes workspaces in batch.
    pub async fn batch_remove_workspaces(&self, workspace_ids: Vec<String>) -> NortHingResult<BatchRemoveResult> {
        self.batch_remove_workspaces_impl(workspace_ids).await
    }

    /// Rescans a workspace.
    pub async fn rescan_workspace(&self, workspace_id: &str) -> NortHingResult<WorkspaceInfo> {
        self.rescan_workspace_impl(workspace_id).await
    }

    /// Refreshes the parsed `IDENTITY.md` content for an assistant workspace.
    pub async fn refresh_workspace_identity(
        &self,
        workspace_id: &str,
    ) -> NortHingResult<Option<WorkspaceIdentityChangedEvent>> {
        self.refresh_workspace_identity_impl(workspace_id).await
    }

    /// Updates workspace information.
    pub async fn update_workspace_info(
        &self,
        workspace_id: &str,
        updates: WorkspaceInfoUpdates,
    ) -> NortHingResult<WorkspaceInfo> {
        self.update_workspace_info_impl(workspace_id, updates).await
    }

    pub(super) async fn normalize_related_paths_for_workspace(
        &self,
        workspace: &WorkspaceInfo,
        related_paths: Vec<RelatedPath>,
    ) -> NortHingResult<Vec<RelatedPath>> {
        let mut normalized = Vec::with_capacity(related_paths.len());
        let mut seen_paths = HashSet::new();

        match workspace.workspace_kind {
            WorkspaceKind::Remote => {
                let connection_id = workspace
                    .remote_ssh_connection_id()
                    .ok_or_else(|| {
                        NortHingError::service(format!(
                            "Remote workspace is missing connectionId metadata: {}",
                            workspace.id
                        ))
                    })?
                    .to_string();
                let remote_manager = remote_workspace_manager().ok_or_else(|| {
                    NortHingError::service(
                        "Remote workspace manager is unavailable for related path validation".to_string(),
                    )
                })?;
                let file_service = remote_manager.get_file_service().await.ok_or_else(|| {
                    NortHingError::service("Remote file service is unavailable for related path validation".to_string())
                })?;

                for related_path in related_paths {
                    let description = Self::normalize_related_path_description(related_path.description);
                    let path = normalize_remote_workspace_path(related_path.path.trim());
                    if path.is_empty() {
                        return Err(NortHingError::service(
                            "Related directory path cannot be empty".to_string(),
                        ));
                    }
                    if !seen_paths.insert(path.clone()) {
                        continue;
                    }

                    if !file_service.exists(&connection_id, &path).await.map_err(|error| {
                        NortHingError::service(format!(
                            "Failed to validate remote related directory '{}': {}",
                            path, error
                        ))
                    })? {
                        return Err(NortHingError::service(format!(
                            "Remote related directory does not exist: {}",
                            path
                        )));
                    }

                    if !file_service.is_dir(&connection_id, &path).await.map_err(|error| {
                        NortHingError::service(format!(
                            "Failed to inspect remote related directory '{}': {}",
                            path, error
                        ))
                    })? {
                        return Err(NortHingError::service(format!(
                            "Remote related path is not a directory: {}",
                            path
                        )));
                    }

                    normalized.push(RelatedPath { path, description });
                }
            }
            _ => {
                for related_path in related_paths {
                    let description = Self::normalize_related_path_description(related_path.description);
                    let raw_path = related_path.path.trim();
                    if raw_path.is_empty() {
                        return Err(NortHingError::service(
                            "Related directory path cannot be empty".to_string(),
                        ));
                    }

                    let path_buf = PathBuf::from(raw_path);
                    let (canonical_path, normalized_key) =
                        canonicalize_local_workspace_root(&path_buf).map_err(NortHingError::service)?;

                    let metadata = tokio::fs::metadata(&canonical_path).await.map_err(|error| {
                        NortHingError::service(format!(
                            "Failed to inspect related directory '{}': {}",
                            canonical_path.display(),
                            error
                        ))
                    })?;

                    if !metadata.is_dir() {
                        return Err(NortHingError::service(format!(
                            "Related path is not a directory: {}",
                            canonical_path.display()
                        )));
                    }

                    if !seen_paths.insert(normalized_key) {
                        continue;
                    }

                    normalized.push(RelatedPath {
                        path: canonical_path.to_string_lossy().to_string(),
                        description,
                    });
                }
            }
        }

        Ok(normalized)
    }

    /// Imports workspaces in batch.
    pub async fn batch_import_workspaces(&self, paths: Vec<String>) -> NortHingResult<BatchImportResult> {
        self.batch_import_workspaces_impl(paths).await
    }

    /// Cleans up invalid workspaces.
    pub async fn cleanup_invalid_workspaces(&self) -> NortHingResult<usize> {
        self.cleanup_invalid_workspaces_impl().await
    }

    /// Exports workspace configuration.
    pub async fn export_workspaces(&self) -> NortHingResult<WorkspaceExport> {
        self.export_workspaces_impl().await
    }

    /// Imports workspace configuration.
    pub async fn import_workspaces(
        &self,
        export: WorkspaceExport,
        overwrite: bool,
    ) -> NortHingResult<WorkspaceImportResult> {
        self.import_workspaces_impl(export, overwrite).await
    }

    /// Saves workspace data manually (public API).
    pub async fn manual_save(&self) -> NortHingResult<()> {
        self.manual_save_impl().await
    }

    /// Drops a workspace from recent lists only (workspace record and open state unchanged).
    pub async fn remove_workspace_from_recent(&self, workspace_id: &str) -> NortHingResult<()> {
        self.remove_workspace_from_recent_impl(workspace_id).await
    }

    /// Clears all persisted data.
    pub async fn clear_persistent_data(&self) -> NortHingResult<()> {
        self.clear_persistent_data_impl().await
    }

    /// Trims related-path descriptions to `None` when blank.
    /// Exposed at `pub(super)` for unit tests and shared with the invoke-flow normalizer.
    pub(super) fn normalize_related_path_description(description: Option<String>) -> Option<String> {
        description.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    }
}
