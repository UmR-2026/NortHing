//! Workspace lifecycle sub-domain (R23a split).
//!
//! Owns the 13 workspace lifecycle methods (construct / open / track /
//! create / close / switch) that previously lived on `service.rs` L207-525.
//!
//! Visibility pattern (R20 `manager_*.rs` precedent + R22 E0592 lesson):
//! sibling method names use the `_impl` suffix so they do not collide with
//! the facade delegates that stay on `service.rs`. The facade keeps the
//! public cross-crate API; this file's `pub(super)` items are only callable
//! from inside the `workspace` module.

use super::manager::{WorkspaceInfo, WorkspaceKind, WorkspaceManager, WorkspaceManagerConfig};
use super::{WorkspaceCreateOptions, WorkspaceService};
use crate::infrastructure::storage::PersistenceService;
use crate::infrastructure::{try_get_path_manager_arc, PathManager};
use crate::service::bootstrap::initialize_workspace_persona_files;
use crate::service::workspace_runtime::{try_get_workspace_runtime_service_arc, WorkspaceRuntimeService};
use crate::util::errors::*;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::warn;

impl WorkspaceService {
    /// Creates a new workspace service.
    pub(super) async fn new_impl() -> NortHingResult<Self> {
        let config = WorkspaceManagerConfig::default();
        Self::with_config_impl(config).await
    }

    /// Creates a workspace service with a custom configuration.
    pub(super) async fn with_config_impl(config: WorkspaceManagerConfig) -> NortHingResult<Self> {
        let path_manager = try_get_path_manager_arc()?;
        let runtime_service = try_get_workspace_runtime_service_arc()?;

        path_manager.initialize_user_directories().await?;

        let persistence = Arc::new(
            PersistenceService::new_user_level(path_manager.clone())
                .await
                .map_err(|e| NortHingError::service(format!("Failed to create persistence service: {}", e)))?,
        );

        let manager = WorkspaceManager::new(config.clone());

        let service = Self {
            manager: Arc::new(RwLock::new(manager)),
            config,
            persistence,
            path_manager,
            runtime_service,
        };

        if let Err(e) = service.load_workspace_history_only().await {
            warn!("Failed to load workspace history on startup: {}", e);
        }

        if let Err(e) = service.remap_legacy_assistant_workspace_records().await {
            warn!("Failed to remap legacy assistant workspace records on startup: {}", e);
        }

        if let Err(e) = service.ensure_assistant_workspaces().await {
            warn!("Failed to ensure assistant workspaces on startup: {}", e);
        }

        Ok(service)
    }

    /// Opens a workspace.
    pub(super) async fn open_workspace_impl(&self, path: PathBuf) -> NortHingResult<WorkspaceInfo> {
        self.open_workspace_with_options_impl(path, WorkspaceCreateOptions::default())
            .await
    }

    /// Opens a workspace with explicit workspace metadata.
    pub(super) async fn open_workspace_with_options_impl(
        &self,
        path: PathBuf,
        options: WorkspaceCreateOptions,
    ) -> NortHingResult<WorkspaceInfo> {
        let options = self.normalize_workspace_options_for_path(&path, options);
        let result = {
            let mut manager = self.manager.write().await;
            manager
                .open_workspace_with_options(path, Self::to_manager_open_options(&options))
                .await
        };

        if let Ok(workspace) = result.as_ref() {
            self.ensure_workspace_gitignore_best_effort(workspace, "opened").await;
            self.ensure_workspace_runtime_best_effort(workspace, "opened").await;
        }

        if result.is_ok() {
            if let Err(e) = self.save_workspace_data().await {
                warn!("Failed to save workspace data after opening: {}", e);
            }
        }

        result
    }

    /// Registers or refreshes workspace activity without marking it as opened in the UI.
    pub(super) async fn track_workspace_activity_impl(
        &self,
        path: PathBuf,
        options: WorkspaceCreateOptions,
    ) -> NortHingResult<WorkspaceInfo> {
        let mut options = self.normalize_workspace_options_for_path(&path, options);
        options.auto_set_current = false;
        let result = {
            let mut manager = self.manager.write().await;
            manager
                .track_workspace_with_options(path, Self::to_manager_open_options(&options))
                .await
        };

        if let Ok(workspace) = result.as_ref() {
            self.ensure_workspace_runtime_best_effort(workspace, "tracked").await;
        }

        if result.is_ok() {
            if let Err(e) = self.save_workspace_data().await {
                warn!("Failed to save workspace data after tracking activity: {}", e);
            }
        }

        result
    }

    /// Quickly opens a workspace (using default options).
    pub(super) async fn quick_open_impl(&self, path: &str) -> NortHingResult<WorkspaceInfo> {
        let path_buf = PathBuf::from(path);
        self.open_workspace_impl(path_buf).await
    }

    /// Creates a workspace (for a new project).
    pub(super) async fn create_workspace_impl(
        &self,
        path: PathBuf,
        options: WorkspaceCreateOptions,
    ) -> NortHingResult<WorkspaceInfo> {
        if !path.exists() {
            tokio::fs::create_dir_all(&path)
                .await
                .map_err(|e| NortHingError::service(format!("Failed to create workspace directory: {}", e)))?;
        }

        let mut workspace = self.open_workspace_with_options_impl(path, options.clone()).await?;

        if let Some(description) = options.description {
            workspace.description = Some(description);
        }

        workspace.tags = options.tags;

        {
            let mut manager = self.manager.write().await;
            manager.workspaces_mut().insert(workspace.id.clone(), workspace.clone());
        }

        self.save_workspace_data().await?;

        Ok(workspace)
    }

    /// Creates and opens a new assistant workspace, then sets it as current.
    pub(super) async fn create_assistant_workspace_impl(
        &self,
        assistant_id: Option<String>,
    ) -> NortHingResult<WorkspaceInfo> {
        let assistant_id = match assistant_id {
            Some(id) if !id.trim().is_empty() => id.trim().to_string(),
            _ => self.generate_assistant_workspace_id().await?,
        };
        let display_name = Self::assistant_display_name(Some(&assistant_id));
        let path = self.path_manager.assistant_workspace_dir(&assistant_id, None);
        let options = WorkspaceCreateOptions {
            auto_set_current: true,
            add_to_recent: false,
            workspace_kind: WorkspaceKind::Assistant,
            assistant_id: Some(assistant_id),
            display_name: Some(display_name),
            ..Default::default()
        };

        if !path.exists() {
            fs::create_dir_all(&path).await.map_err(|e| {
                NortHingError::service(format!(
                    "Failed to create assistant workspace directory '{}': {}",
                    path.display(),
                    e
                ))
            })?;
        }

        // New assistant dirs get persona files at creation; coordinator also fills missing files when opening.
        initialize_workspace_persona_files(&path).await?;

        self.create_workspace_impl(path, options).await
    }

    /// Closes the current workspace.
    pub(super) async fn close_current_workspace_impl(&self) -> NortHingResult<()> {
        let result = {
            let mut manager = self.manager.write().await;
            manager.close_current_workspace()
        };

        if result.is_ok() {
            if let Err(e) = self.save_workspace_data().await {
                warn!("Failed to save workspace data after closing: {}", e);
            }
        }

        result
    }

    /// Closes the specified workspace.
    pub(super) async fn close_workspace_impl(&self, workspace_id: &str) -> NortHingResult<()> {
        let result = {
            let mut manager = self.manager.write().await;
            manager.close_workspace(workspace_id)
        };

        if result.is_ok() {
            if let Err(e) = self.save_workspace_data().await {
                warn!("Failed to save workspace data after closing: {}", e);
            }
        }

        result
    }

    /// Sets the active workspace from the opened workspace list.
    pub(super) async fn set_active_workspace_impl(&self, workspace_id: &str) -> NortHingResult<()> {
        let result = {
            let mut manager = self.manager.write().await;
            manager.set_active_workspace(workspace_id)
        };

        if result.is_ok() {
            if let Err(e) = self.save_workspace_data().await {
                warn!("Failed to save workspace data after switching active workspace: {}", e);
            }
        }

        if result.is_ok() {
            if let Some(workspace) = self.get_workspace(workspace_id).await {
                self.ensure_workspace_runtime_best_effort(&workspace, "activated").await;
            }
        }

        result
    }

    /// Reorders the opened workspaces without changing active or recent state.
    pub(super) async fn reorder_opened_workspaces_impl(&self, workspace_ids: Vec<String>) -> NortHingResult<()> {
        let current_ids = {
            let manager = self.manager.read().await;
            manager.opened_workspace_ids().clone()
        };

        if workspace_ids.len() != current_ids.len() {
            return Err(NortHingError::service(format!(
                "Opened workspace count mismatch: expected {}, got {}",
                current_ids.len(),
                workspace_ids.len()
            )));
        }

        let requested_ids = workspace_ids.iter().cloned().collect::<HashSet<_>>();
        if requested_ids.len() != workspace_ids.len() {
            return Err(NortHingError::service(
                "Opened workspace order contains duplicate ids".to_string(),
            ));
        }

        let current_id_set = current_ids.iter().cloned().collect::<HashSet<_>>();
        if requested_ids != current_id_set {
            return Err(NortHingError::service(
                "Opened workspace order must contain exactly the currently opened workspace ids".to_string(),
            ));
        }

        {
            let mut manager = self.manager.write().await;
            manager.set_opened_workspace_ids(workspace_ids.clone());
        }

        if let Err(error) = self.save_workspace_data().await {
            let mut manager = self.manager.write().await;
            manager.set_opened_workspace_ids(current_ids);
            return Err(error);
        }

        Ok(())
    }

    /// Switches to the specified workspace.
    pub(super) async fn switch_to_workspace_impl(&self, workspace_id: &str) -> NortHingResult<()> {
        self.set_active_workspace_impl(workspace_id).await
    }
}
