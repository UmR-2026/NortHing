//! Workspace admin sub-domain (R23d Mavis take-over).
//!
//! Owns 8 workspace admin method + 8 helpers that previously lived on
//! `service.rs` L589-1365. Extracted via Mavis take-over after
//! R23a/b/c/d 4 producer parallel sub-rounds timed out at the 30-min cap.
//!
//! 8 admin method (facade delegates stay in `service.rs`):
//!   - health_check_impl, export_workspaces_impl, import_workspaces_impl,
//!     get_quick_summary_impl, manual_save_impl,
//!     is_assistant_workspace_path_impl, clear_persistent_data_impl,
//!     get_manager_impl
//!
//! 8 internal helpers (already `pub(super)`, stay accessible cross-sibling):
//!   - save_workspace_data, load_workspace_history_only,
//!     to_manager_open_options, assistant_display_name,
//!     generate_assistant_workspace_id,
//!     remap_legacy_assistant_workspace_records,
//!     normalize_workspace_options_for_path, ensure_assistant_workspaces

use super::manager::{
    WorkspaceInfo, WorkspaceKind, WorkspaceManager, WorkspaceOpenOptions,
};
use super::service::{
    WorkspaceCreateOptions, WorkspaceExport, WorkspaceHealthStatus,
    WorkspaceImportResult, WorkspaceQuickSummary,
};
use super::service_types::{AssistantWorkspaceDescriptor, WorkspacePersistenceData};
use super::WorkspaceService;
use crate::infrastructure::storage::StorageOptions;
use crate::service::remote_ssh::normalize_remote_workspace_path;
use crate::service::remote_ssh::workspace_state::remote_workspace_stable_id;
use crate::util::errors::*;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{info, warn};

impl WorkspaceService {
    /// Runs a health check.
    pub(super) async fn health_check_impl(&self) -> NortHingResult<WorkspaceHealthStatus> {
        let stats = self.statistics().await;

        let mut warnings = Vec::new();
        let mut issues = Vec::new();

        if stats.total_workspaces == 0 {
            warnings.push("No workspaces found".to_string());
        }

        if stats.active_workspaces == 0 {
            warnings.push("No active workspaces".to_string());
        }

        if stats.inactive_workspaces > stats.active_workspaces * 3 {
            issues.push("Too many inactive workspaces, consider cleanup".to_string());
        }

        let current_workspace_valid = match self.current_workspace().await {
            Some(current) => current.is_valid().await,
            None => true,
        };

        if !current_workspace_valid {
            issues.push("Current workspace path is invalid".to_string());
        }

        let healthy = issues.is_empty() && current_workspace_valid;

        Ok(WorkspaceHealthStatus {
            healthy,
            total_workspaces: stats.total_workspaces,
            active_workspaces: stats.active_workspaces,
            current_workspace_valid,
            total_files: stats.total_files,
            total_size_mb: stats.total_size_bytes / (1024 * 1024),
            warnings,
            issues: issues.clone(),
            message: if healthy {
                "Workspace system is healthy".to_string()
            } else {
                format!("{} issues detected", issues.len())
            },
        })
    }

    /// Exports workspace configuration.
    pub(super) async fn export_workspaces_impl(&self) -> NortHingResult<WorkspaceExport> {
        let manager = self.manager.read().await;
        let workspaces: Vec<WorkspaceInfo> = manager.workspaces().values().cloned().collect();
        let current_workspace_id = manager.current_workspace().map(|w| w.id.clone());
        let _recent_workspaces = manager.recent_workspaces().clone();

        Ok(WorkspaceExport {
            workspaces,
            current_workspace_id,
            recent_workspaces: manager.recent_workspace_infos().iter().map(|w| w.id.clone()).collect(),
            recent_assistant_workspaces: manager
                .recent_assistant_workspace_infos()
                .iter()
                .map(|w| w.id.clone())
                .collect(),
            export_timestamp: chrono::Utc::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    /// Imports workspace configuration.
    pub(super) async fn import_workspaces_impl(
        &self,
        export: WorkspaceExport,
        overwrite: bool,
    ) -> NortHingResult<WorkspaceImportResult> {
        let mut result = WorkspaceImportResult {
            imported_workspaces: 0,
            skipped_workspaces: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        let mut manager = self.manager.write().await;

        for workspace in export.workspaces {
            if !workspace.is_valid().await {
                result
                    .warnings
                    .push(format!("Workspace path no longer valid: {:?}", workspace.root_path));
                continue;
            }

            if !overwrite && manager.workspaces().contains_key(&workspace.id) {
                result.skipped_workspaces += 1;
                continue;
            }

            manager.workspaces_mut().insert(workspace.id.clone(), workspace);
            result.imported_workspaces += 1;
        }

        manager.set_recent_workspaces(export.recent_workspaces.clone());
        manager.set_recent_assistant_workspaces(export.recent_assistant_workspaces.clone());

        if let Some(current_id) = export.current_workspace_id {
            if manager.workspaces().contains_key(&current_id) {
                if let Err(e) = manager.set_current_workspace(current_id) {
                    result
                        .warnings
                        .push(format!("Failed to restore current workspace: {}", e));
                }
            } else {
                result
                    .warnings
                    .push("Current workspace not found in import".to_string());
            }
        }

        drop(manager);

        Ok(result)
    }

    /// Returns a quick summary.
    pub(super) async fn get_quick_summary_impl(&self) -> WorkspaceQuickSummary {
        let stats = self.statistics().await;
        let current_workspace = self.current_workspace().await;
        let recent_workspaces = self.recent_workspaces().await;
        let recent_assistant_workspaces = self.recent_assistant_workspaces().await;

        WorkspaceQuickSummary {
            total_workspaces: stats.total_workspaces,
            active_workspaces: stats.active_workspaces,
            current_workspace: current_workspace.map(|w| w.summary()),
            recent_workspaces: recent_workspaces.into_iter().take(5).map(|w| w.summary()).collect(),
            recent_assistant_workspaces: recent_assistant_workspaces
                .into_iter()
                .take(5)
                .map(|w| w.summary())
                .collect(),
            workspace_types: stats.workspaces_by_type,
        }
    }

    /// Saves workspace data locally.
    pub(super) async fn save_workspace_data(&self) -> NortHingResult<()> {
        let manager = self.manager.read().await;

        let workspace_data = WorkspacePersistenceData {
            workspaces: manager.workspaces().clone(),
            opened_workspace_ids: manager.opened_workspace_ids().clone(),
            current_workspace_id: manager.current_workspace().map(|w| w.id.clone()),
            recent_workspaces: manager.recent_workspaces().clone(),
            recent_assistant_workspaces: manager.recent_assistant_workspaces().clone(),
            saved_at: chrono::Utc::now(),
        };

        self.persistence
            .save_json("workspace_data", &workspace_data, StorageOptions::default())
            .await
            .map_err(|e| NortHingError::service(format!("Failed to save workspace data: {}", e)))?;

        Ok(())
    }

    /// Loads workspace data from local storage.
    #[allow(dead_code)]
    async fn load_workspace_data(&self) -> NortHingResult<()> {
        let workspace_data: Option<WorkspacePersistenceData> = self
            .persistence
            .load_json("workspace_data")
            .await
            .map_err(|e| NortHingError::service(format!("Failed to load workspace data: {}", e)))?;

        if let Some(data) = workspace_data {
            let mut manager = self.manager.write().await;

            *manager.workspaces_mut() = data.workspaces;
            manager.set_opened_workspace_ids(data.opened_workspace_ids);
            manager.set_recent_workspaces(data.recent_workspaces);
            manager.set_recent_assistant_workspaces(data.recent_assistant_workspaces);
            let id_remap = manager.migrate_local_workspace_ids_to_stable_storage();

            if let Some(raw_current) = data.current_workspace_id {
                let current_id = id_remap.get(&raw_current).cloned().unwrap_or(raw_current);
                if let Some(workspace) = manager.workspaces().get(&current_id) {
                    if workspace.is_valid().await {
                        if let Err(e) = manager.set_current_workspace(current_id) {
                            warn!("Failed to restore current workspace: {}", e);
                        }
                    } else {
                        warn!("Current workspace path no longer valid, skipping restore");
                    }
                }
            }

            info!("Loaded {} workspaces from local storage", manager.workspaces().len());
        } else {
            info!("No saved workspace data found, starting fresh");
        }

        Ok(())
    }

    /// Loads workspace history only without restoring the current workspace (used on startup).
    pub(super) async fn load_workspace_history_only(&self) -> NortHingResult<()> {
        let workspace_data: Option<WorkspacePersistenceData> = self
            .persistence
            .load_json("workspace_data")
            .await
            .map_err(|e| NortHingError::service(format!("Failed to load workspace data: {}", e)))?;

        let mut workspaces_to_restore = Vec::new();
        let mut should_persist_cleaned_history = false;

        if let Some(data) = workspace_data {
            let mut manager = self.manager.write().await;

            let mut workspaces = data.workspaces;
            let original_workspace_count = workspaces.len();
            // Filter out legacy remote workspaces that don't have the required metadata (sshHost and connectionId)
            workspaces.retain(|_id, ws| {
                if ws.workspace_kind == WorkspaceKind::Remote {
                    // Check if this remote workspace has the required metadata
                    let has_ssh_host = ws
                        .metadata
                        .get("sshHost")
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| !s.trim().is_empty());
                    let has_connection_id = ws
                        .metadata
                        .get("connectionId")
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| !s.trim().is_empty());
                    if !has_ssh_host || !has_connection_id {
                        // Skip this legacy remote workspace
                        info!(
                            "Skipping legacy remote workspace without required metadata: id={}, root_path={}",
                            _id,
                            ws.root_path.display()
                        );
                        return false;
                    }
                }
                true
            });
            if workspaces.len() != original_workspace_count {
                should_persist_cleaned_history = true;
            }

            *manager.workspaces_mut() = workspaces;
            // Also filter opened/recent lists to remove references to removed legacy workspaces
            let filtered_opened_ids: Vec<String> = data
                .opened_workspace_ids
                .clone()
                .into_iter()
                .filter(|id| manager.workspaces().contains_key(id))
                .collect();
            if filtered_opened_ids != data.opened_workspace_ids {
                should_persist_cleaned_history = true;
            }
            manager.set_opened_workspace_ids(filtered_opened_ids);

            let filtered_recent: Vec<String> = data
                .recent_workspaces
                .clone()
                .into_iter()
                .filter(|id| manager.workspaces().contains_key(id))
                .collect();
            if filtered_recent != data.recent_workspaces {
                should_persist_cleaned_history = true;
            }
            manager.set_recent_workspaces(filtered_recent);

            let filtered_recent_assistant: Vec<String> = data
                .recent_assistant_workspaces
                .clone()
                .into_iter()
                .filter(|id| manager.workspaces().contains_key(id))
                .collect();
            if filtered_recent_assistant != data.recent_assistant_workspaces {
                should_persist_cleaned_history = true;
            }
            manager.set_recent_assistant_workspaces(filtered_recent_assistant);

            let id_remap = manager.migrate_local_workspace_ids_to_stable_storage();
            if !id_remap.is_empty() {
                should_persist_cleaned_history = true;
            }

            let raw_current = data
                .current_workspace_id
                .or_else(|| data.opened_workspace_ids.first().cloned());

            if let Some(raw) = raw_current {
                let current_id = id_remap.get(&raw).cloned().unwrap_or(raw);
                if manager.workspaces().contains_key(&current_id) {
                    if let Err(e) = manager.set_current_workspace(current_id) {
                        warn!("Failed to restore current workspace on startup: {}", e);
                    }
                }
            }

            workspaces_to_restore = Self::collect_startup_restored_workspaces(&manager);
        }

        if should_persist_cleaned_history {
            self.save_workspace_data().await?;
        }

        self.prepare_startup_restored_workspaces(workspaces_to_restore).await;

        Ok(())
    }

    pub(super) fn to_manager_open_options(options: &WorkspaceCreateOptions) -> WorkspaceOpenOptions {
        WorkspaceOpenOptions {
            scan_options: options.scan_options.clone(),
            auto_set_current: options.auto_set_current,
            add_to_recent: options.add_to_recent,
            workspace_kind: options.workspace_kind.clone(),
            assistant_id: options.assistant_id.clone(),
            display_name: options.display_name.clone(),
            remote_connection_id: options.remote_connection_id.clone(),
            remote_ssh_host: options.remote_ssh_host.clone(),
            stable_workspace_id: options.stable_workspace_id.clone(),
        }
    }

    pub(super) fn assistant_display_name(assistant_id: Option<&str>) -> String {
        match assistant_id {
            Some(id) if !id.trim().is_empty() => format!("Claw {}", id.trim()),
            _ => "Claw".to_string(),
        }
    }

    pub(super) async fn generate_assistant_workspace_id(&self) -> NortHingResult<String> {
        for _ in 0..32 {
            let assistant_id = uuid::Uuid::new_v4()
                .simple()
                .to_string()
                .chars()
                .take(8)
                .collect::<String>();
            let path = self.path_manager.assistant_workspace_dir(&assistant_id, None);

            if fs::try_exists(&path).await.map_err(|e| {
                NortHingError::service(format!(
                    "Failed to check assistant workspace path '{}': {}",
                    path.display(),
                    e
                ))
            })? {
                continue;
            }

            if self.get_workspace_by_path(&path).await.is_none() {
                return Ok(assistant_id);
            }
        }

        Err(NortHingError::service(
            "Failed to allocate a unique assistant workspace id".to_string(),
        ))
    }

    fn assistant_descriptor_from_path(&self, path: &Path) -> Option<AssistantWorkspaceDescriptor> {
        let default_workspace = self.path_manager.default_assistant_workspace_dir(None);
        if path == default_workspace {
            return Some(AssistantWorkspaceDescriptor {
                path: path.to_path_buf(),
                assistant_id: None,
                display_name: Self::assistant_display_name(None),
            });
        }

        let assistant_root = self.path_manager.assistant_workspace_base_dir(None);
        if path.parent()? != assistant_root {
            return None;
        }

        let file_name = path.file_name()?.to_string_lossy();
        let assistant_id = file_name.strip_prefix("workspace-")?;
        if assistant_id.trim().is_empty() {
            return None;
        }

        Some(AssistantWorkspaceDescriptor {
            path: path.to_path_buf(),
            assistant_id: Some(assistant_id.to_string()),
            display_name: Self::assistant_display_name(Some(assistant_id)),
        })
    }

    fn legacy_assistant_descriptor_from_path(&self, path: &Path) -> Option<AssistantWorkspaceDescriptor> {
        let default_workspace = self.path_manager.legacy_default_assistant_workspace_dir(None);
        if path == default_workspace {
            return Some(AssistantWorkspaceDescriptor {
                path: path.to_path_buf(),
                assistant_id: None,
                display_name: Self::assistant_display_name(None),
            });
        }

        let assistant_root = self.path_manager.legacy_assistant_workspace_base_dir(None);
        if path.parent()? != assistant_root {
            return None;
        }

        let file_name = path.file_name()?.to_string_lossy();
        let assistant_id = file_name.strip_prefix("workspace-")?;
        if assistant_id.trim().is_empty() {
            return None;
        }

        Some(AssistantWorkspaceDescriptor {
            path: path.to_path_buf(),
            assistant_id: Some(assistant_id.to_string()),
            display_name: Self::assistant_display_name(Some(assistant_id)),
        })
    }

    pub(super) async fn remap_legacy_assistant_workspace_records(&self) -> NortHingResult<()> {
        let mut changed = false;
        let mut manager = self.manager.write().await;

        for workspace in manager.workspaces_mut().values_mut() {
            let Some(descriptor) = self.legacy_assistant_descriptor_from_path(&workspace.root_path) else {
                continue;
            };
            let new_path = self
                .path_manager
                .resolve_assistant_workspace_dir(descriptor.assistant_id.as_deref(), None);

            if workspace.root_path != new_path {
                info!(
                    "Remap legacy assistant workspace record: workspace_id={}, from={}, to={}",
                    workspace.id,
                    workspace.root_path.display(),
                    new_path.display()
                );
                workspace.root_path = new_path;
                changed = true;
            }

            if workspace.workspace_kind != WorkspaceKind::Assistant {
                workspace.workspace_kind = WorkspaceKind::Assistant;
                changed = true;
            }

            if workspace.assistant_id != descriptor.assistant_id {
                workspace.assistant_id = descriptor.assistant_id.clone();
                changed = true;
            }
        }

        drop(manager);

        if changed {
            self.save_workspace_data().await?;
        }

        Ok(())
    }

    pub(super) fn normalize_workspace_options_for_path(
        &self,
        path: &Path,
        mut options: WorkspaceCreateOptions,
    ) -> WorkspaceCreateOptions {
        if options.workspace_kind == WorkspaceKind::Remote {
            if options.stable_workspace_id.is_none() {
                if let Some(ssh_host) = options
                    .remote_ssh_host
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    options.stable_workspace_id = Some(remote_workspace_stable_id(
                        ssh_host,
                        &normalize_remote_workspace_path(&path.to_string_lossy()),
                    ));
                }
            }
            return options;
        }

        if options.workspace_kind == WorkspaceKind::Assistant {
            if options.display_name.is_none() {
                options.display_name = Some(Self::assistant_display_name(options.assistant_id.as_deref()));
            }
            return options;
        }

        if let Some(descriptor) = self.assistant_descriptor_from_path(path) {
            options.workspace_kind = WorkspaceKind::Assistant;
            if options.assistant_id.is_none() {
                options.assistant_id = descriptor.assistant_id;
            }
            if options.display_name.is_none() {
                options.display_name = Some(descriptor.display_name);
            }
        }

        options
    }

    pub(super) async fn ensure_assistant_workspaces(&self) -> NortHingResult<()> {
        let descriptors = self.discover_assistant_workspaces().await?;
        let mut has_current_workspace = self.current_workspace().await.is_some();
        let has_opened_remote = {
            let manager = self.manager.read().await;
            manager
                .opened_workspace_infos()
                .iter()
                .any(|w| w.workspace_kind == WorkspaceKind::Remote)
        };

        for descriptor in descriptors {
            // If a remote workspace tab exists but nothing is current yet (e.g. pending SSH
            // reconnect), do not auto-activate the default assistant workspace — that would look
            // like a spurious new local workspace.
            let should_activate = !has_current_workspace && !has_opened_remote && descriptor.assistant_id.is_none();
            let options = WorkspaceCreateOptions {
                auto_set_current: should_activate,
                add_to_recent: false,
                workspace_kind: WorkspaceKind::Assistant,
                assistant_id: descriptor.assistant_id.clone(),
                display_name: Some(descriptor.display_name.clone()),
                ..Default::default()
            };

            self.open_workspace_with_options(descriptor.path, options).await?;
            has_current_workspace = true;
        }

        Ok(())
    }

    /// Saves workspace data manually (public API).
    pub(super) async fn manual_save_impl(&self) -> NortHingResult<()> {
        self.save_workspace_data().await
    }

    /// Returns whether a path is a managed assistant workspace.
    pub(super) fn is_assistant_workspace_path_impl(&self, path: &Path) -> bool {
        self.assistant_descriptor_from_path(path).is_some()
    }

    /// Clears all persisted data.
    pub(super) async fn clear_persistent_data_impl(&self) -> NortHingResult<()> {
        self.persistence
            .delete("workspace_data")
            .await
            .map_err(|e| NortHingError::service(format!("Failed to clear workspace data: {}", e)))?;

        Ok(())
    }

    /// Returns the underlying `WorkspaceManager` handle.
    /// Used to share workspace state with other services (e.g. Agent).
    pub(super) fn get_manager_impl(&self) -> Arc<RwLock<WorkspaceManager>> {
        self.manager.clone()
    }
}
