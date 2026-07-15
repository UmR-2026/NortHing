//! Workspace update sub-domain (R23c Mavis take-over).
//!
//! Owns the 9 workspace update/refresh/import/batch methods that previously
//! lived on `service.rs` L393-855. Extracted via Mavis take-over after
//! R23a/b/c/d 4 producer parallel sub-rounds timed out at the 30min cap.
//!
//! Visibility pattern (R20 `manager_*.rs` precedent + R22 E0592 lesson):
//! sibling method names use the `_impl` suffix so they do not collide with
//! the facade delegates that stay on `service.rs`. The facade keeps the
//! public cross-crate API; this file's `pub(super)` items are only callable
//! from inside the `workspace` module.

use super::manager::{
    RelatedPath, ScanOptions, WorkspaceIdentity, WorkspaceInfo, WorkspaceKind, WorkspaceManager,
    WorkspaceManagerStatistics, WorkspaceOpenOptions, WorkspaceStatus, WorkspaceSummary,
};
use super::service::{BatchImportResult, BatchRemoveResult, WorkspaceIdentityChangedEvent, WorkspaceInfoUpdates};
use super::WorkspaceService;
use crate::service::remote_ssh::normalize_remote_workspace_path;
use crate::service::remote_ssh::workspace_state::{
    canonicalize_local_workspace_root, local_workspace_roots_equal, remote_workspace_manager,
    remote_workspace_stable_id,
};
use crate::util::errors::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;
impl WorkspaceService {
    pub(super) async fn remove_workspace_impl(&self, workspace_id: &str) -> NortHingResult<()> {
        let result = {
            let mut manager = self.manager.write().await;
            manager.remove_workspace(workspace_id)
        };

        if result.is_ok() {
            if let Err(e) = self.save_workspace_data().await {
                warn!("Failed to save workspace data after removal: {}", e);
            }
        }

        result
    }

    /// Removes workspaces in batch.
    pub(super) async fn batch_remove_workspaces_impl(
        &self,
        workspace_ids: Vec<String>,
    ) -> NortHingResult<BatchRemoveResult> {
        let mut result = BatchRemoveResult {
            successful: Vec::new(),
            failed: Vec::new(),
            total_processed: workspace_ids.len(),
        };

        for workspace_id in workspace_ids {
            match self.remove_workspace(&workspace_id).await {
                Ok(_) => result.successful.push(workspace_id),
                Err(e) => result.failed.push((workspace_id, e.to_string())),
            }
        }

        Ok(result)
    }

    /// Rescans a workspace.
    pub(super) async fn rescan_workspace_impl(&self, workspace_id: &str) -> NortHingResult<WorkspaceInfo> {
        let workspace_path = {
            let manager = self.manager.read().await;
            if let Some(workspace) = manager.get_workspace(workspace_id) {
                workspace.root_path.clone()
            } else {
                return Err(NortHingError::service(format!("Workspace not found: {}", workspace_id)));
            }
        };

        let existing_workspace = {
            let manager = self.manager.read().await;
            manager.get_workspace(workspace_id).cloned()
        };
        let Some(existing_workspace) = existing_workspace else {
            return Err(NortHingError::service(format!("Workspace not found: {}", workspace_id)));
        };
        let new_workspace = WorkspaceInfo::new(
            workspace_path,
            WorkspaceOpenOptions {
                scan_options: ScanOptions::default(),
                auto_set_current: existing_workspace.status == WorkspaceStatus::Active,
                add_to_recent: false,
                workspace_kind: existing_workspace.workspace_kind.clone(),
                assistant_id: existing_workspace.assistant_id.clone(),
                display_name: Some(existing_workspace.name.clone()),
                remote_connection_id: existing_workspace.remote_ssh_connection_id().map(str::to_string),
                remote_ssh_host: existing_workspace
                    .metadata
                    .get("sshHost")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
                stable_workspace_id: None,
            },
        )
        .await?;
        let mut new_workspace = new_workspace;
        new_workspace.id = existing_workspace.id.clone();
        new_workspace.opened_at = existing_workspace.opened_at;
        new_workspace.description = existing_workspace.description.clone();
        new_workspace.tags = existing_workspace.tags.clone();
        new_workspace.metadata = existing_workspace.metadata.clone();

        {
            let mut manager = self.manager.write().await;
            manager
                .workspaces_mut()
                .insert(workspace_id.to_string(), new_workspace.clone());
        }

        if let Err(e) = self.save_workspace_data().await {
            warn!("Failed to save workspace data after rescan: {}", e);
        }

        Ok(new_workspace)
    }

    /// Refreshes the parsed `IDENTITY.md` content for an assistant workspace.
    pub(super) async fn refresh_workspace_identity_impl(
        &self,
        workspace_id: &str,
    ) -> NortHingResult<Option<WorkspaceIdentityChangedEvent>> {
        let workspace = {
            let manager = self.manager.read().await;
            manager.get_workspace(workspace_id).cloned()
        }
        .ok_or_else(|| NortHingError::service(format!("Workspace not found: {}", workspace_id)))?;

        if workspace.workspace_kind != WorkspaceKind::Assistant {
            return Ok(None);
        }

        let updated_identity = match WorkspaceIdentity::load_from_workspace_root(&workspace.root_path).await {
            Ok(identity) => identity,
            Err(error) => {
                warn!(
                    "Failed to refresh workspace identity: workspace_id={} path={} error={}",
                    workspace_id,
                    workspace.root_path.display(),
                    error
                );
                return Ok(None);
            }
        };

        let changed_fields =
            WorkspaceIdentity::collect_changed_fields(workspace.identity.as_ref(), updated_identity.as_ref());
        let fallback_name = Self::assistant_display_name(workspace.assistant_id.as_deref());
        let updated_name = updated_identity
            .as_ref()
            .and_then(|identity| identity.name.clone())
            .unwrap_or(fallback_name);

        if changed_fields.is_empty() && workspace.name == updated_name {
            return Ok(None);
        }

        {
            let mut manager = self.manager.write().await;
            let workspace = manager
                .workspaces_mut()
                .get_mut(workspace_id)
                .ok_or_else(|| NortHingError::service(format!("Workspace not found: {}", workspace_id)))?;

            workspace.identity = updated_identity.clone();
            workspace.name = updated_name.clone();
        }

        if let Err(e) = self.save_workspace_data().await {
            warn!(
                "Failed to save workspace data after identity refresh: workspace_id={} error={}",
                workspace_id, e
            );
        }

        Ok(Some(WorkspaceIdentityChangedEvent {
            workspace_id: workspace.id,
            workspace_path: workspace.root_path.to_string_lossy().to_string(),
            name: updated_name,
            identity: updated_identity,
            changed_fields,
        }))
    }

    /// Updates workspace information.
    pub(super) async fn update_workspace_info_impl(
        &self,
        workspace_id: &str,
        updates: WorkspaceInfoUpdates,
    ) -> NortHingResult<WorkspaceInfo> {
        let WorkspaceInfoUpdates {
            name,
            description,
            tags,
            related_paths,
        } = updates;

        let existing_workspace = {
            let manager = self.manager.read().await;
            manager
                .workspaces()
                .get(workspace_id)
                .cloned()
                .ok_or_else(|| NortHingError::service(format!("Workspace not found: {}", workspace_id)))?
        };

        let normalized_related_paths = match related_paths {
            Some(related_paths) => Some(
                self.normalize_related_paths_for_workspace(&existing_workspace, related_paths)
                    .await?,
            ),
            None => None,
        };

        let updated_workspace = {
            let mut manager = self.manager.write().await;
            let workspace = manager
                .workspaces_mut()
                .get_mut(workspace_id)
                .ok_or_else(|| NortHingError::service(format!("Workspace not found: {}", workspace_id)))?;

            if let Some(name) = name {
                workspace.name = name;
            }

            if let Some(description) = description {
                workspace.description = Some(description);
            }

            if let Some(tags) = tags {
                workspace.tags = tags;
            }

            if let Some(related_paths) = normalized_related_paths {
                workspace.related_paths = related_paths;
            }

            workspace.last_accessed = chrono::Utc::now();
            workspace.clone()
        };

        self.save_workspace_data().await?;

        Ok(updated_workspace)
    }

    /// Imports workspaces in batch.
    pub(super) async fn batch_import_workspaces_impl(&self, paths: Vec<String>) -> NortHingResult<BatchImportResult> {
        let mut result = BatchImportResult {
            successful: Vec::new(),
            failed: Vec::new(),
            total_processed: paths.len(),
            skipped: Vec::new(),
        };

        for path_str in paths {
            let path = PathBuf::from(&path_str);

            if !path.exists() {
                result.failed.push((path_str, "Path does not exist".to_string()));
                continue;
            }

            if !path.is_dir() {
                result.failed.push((path_str, "Path is not a directory".to_string()));
                continue;
            }

            {
                let manager = self.manager.read().await;
                if manager.workspaces().values().any(|w| {
                    if w.workspace_kind == WorkspaceKind::Remote {
                        w.root_path == path
                    } else {
                        local_workspace_roots_equal(&w.root_path, &path)
                    }
                }) {
                    result.skipped.push(path_str);
                    continue;
                }
            }

            match self.open_workspace(path).await {
                Ok(workspace) => {
                    result.successful.push(workspace.id);
                }
                Err(e) => {
                    result.failed.push((path_str, e.to_string()));
                }
            }
        }

        Ok(result)
    }

    /// Cleans up invalid workspaces.
    pub(super) async fn cleanup_invalid_workspaces_impl(&self) -> NortHingResult<usize> {
        let result = {
            let mut manager = self.manager.write().await;
            manager.cleanup_invalid_workspaces().await
        };

        if result.is_ok() {
            if let Err(e) = self.save_workspace_data().await {
                warn!("Failed to save workspace data after cleanup: {}", e);
            }
        }

        result
    }

    /// Returns statistics.
    pub(super) async fn get_statistics_impl(&self) -> WorkspaceManagerStatistics {
        let manager = self.manager.read().await;
        manager.statistics()
    }

    /// Returns the workspace count.
    pub(super) async fn get_workspace_count_impl(&self) -> usize {
        let manager = self.manager.read().await;
        manager.workspace_count()
    }
}
