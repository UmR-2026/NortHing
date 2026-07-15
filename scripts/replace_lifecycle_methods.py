"""Replace 13 method bodies in service.rs L207-525 with facade delegates.

Body preservation: read source from git HEAD first, then splice current file
because the script may run multiple times (avoid self-overwrite like R8 lesson).

Mapping (preserves original signatures + doc comments on the facade side,
sibling impl lives in lifecycle.rs with _impl suffix per R22 E0592 lesson).
"""
import re
import subprocess
import sys

PATH = "src/crates/assembly/core/src/service/workspace/service.rs"


def read_current():
    with open(PATH, "r", encoding="utf-8") as f:
        return f.read()


def write_current(content):
    with open(PATH, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)


# (method_name, full_block_text_to_replace, facade_delegate_block)
# The facade delegates preserve the original public signature verbatim.
# Associated functions stay associated (no `&self`); instance methods keep
# `&self`. The body is one line that delegates to sibling.
REPLACEMENTS = [
    # 1. new (L207-210) — associated function
    (
        "new",
        """    /// Creates a new workspace service.
    pub async fn new() -> NortHingResult<Self> {
        let config = WorkspaceManagerConfig::default();
        Self::with_config(config).await
    }
""",
        """    /// Creates a new workspace service.
    pub async fn new() -> NortHingResult<Self> {
        Self::new_impl().await
    }
""",
    ),
    # 2. with_config (L213-253) — associated function
    (
        "with_config",
        """    /// Creates a workspace service with a custom configuration.
    pub async fn with_config(config: WorkspaceManagerConfig) -> NortHingResult<Self> {
        let path_manager = try_get_path_manager_arc()?;
        let runtime_service = try_get_workspace_runtime_service_arc()?;

        path_manager.initialize_user_directories().await?;

        let persistence = Arc::new(
            PersistenceService::new_user_level(path_manager.clone())
                .await
                .map_err(|e| {
                    NortHingError::service(format!("Failed to create persistence service: {}", e))
                })?,
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
            warn!(
                "Failed to remap legacy assistant workspace records on startup: {}",
                e
            );
        }

        if let Err(e) = service.ensure_assistant_workspaces().await {
            warn!("Failed to ensure assistant workspaces on startup: {}", e);
        }

        Ok(service)
    }
""",
        """    /// Creates a workspace service with a custom configuration.
    pub async fn with_config(config: WorkspaceManagerConfig) -> NortHingResult<Self> {
        Self::with_config_impl(config).await
    }
""",
    ),
    # 3. open_workspace (L270-273) — instance method
    (
        "open_workspace",
        """    /// Opens a workspace.
    pub async fn open_workspace(&self, path: PathBuf) -> NortHingResult<WorkspaceInfo> {
        self.open_workspace_with_options(path, WorkspaceCreateOptions::default())
            .await
    }
""",
        """    /// Opens a workspace.
    pub async fn open_workspace(&self, path: PathBuf) -> NortHingResult<WorkspaceInfo> {
        self.open_workspace_impl(path).await
    }
""",
    ),
    # 4. open_workspace_with_options (L276-303) — instance method
    (
        "open_workspace_with_options",
        """    /// Opens a workspace with explicit workspace metadata.
    pub async fn open_workspace_with_options(
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
            self.ensure_workspace_gitignore_best_effort(workspace, "opened")
                .await;
            self.ensure_workspace_runtime_best_effort(workspace, "opened")
                .await;
        }

        if result.is_ok() {
            if let Err(e) = self.save_workspace_data().await {
                warn!("Failed to save workspace data after opening: {}", e);
            }
        }

        result
    }
""",
        """    /// Opens a workspace with explicit workspace metadata.
    pub async fn open_workspace_with_options(
        &self,
        path: PathBuf,
        options: WorkspaceCreateOptions,
    ) -> NortHingResult<WorkspaceInfo> {
        self.open_workspace_with_options_impl(path, options).await
    }
""",
    ),
    # 5. track_workspace_activity (L306-335) — instance method
    (
        "track_workspace_activity",
        """    /// Registers or refreshes workspace activity without marking it as opened in the UI.
    pub async fn track_workspace_activity(
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
            self.ensure_workspace_runtime_best_effort(workspace, "tracked")
                .await;
        }

        if result.is_ok() {
            if let Err(e) = self.save_workspace_data().await {
                warn!(
                    "Failed to save workspace data after tracking activity: {}",
                    e
                );
            }
        }

        result
    }
""",
        """    /// Registers or refreshes workspace activity without marking it as opened in the UI.
    pub async fn track_workspace_activity(
        &self,
        path: PathBuf,
        options: WorkspaceCreateOptions,
    ) -> NortHingResult<WorkspaceInfo> {
        self.track_workspace_activity_impl(path, options).await
    }
""",
    ),
    # 6. quick_open (L338-341) — instance method
    (
        "quick_open",
        """    /// Quickly opens a workspace (using default options).
    pub async fn quick_open(&self, path: &str) -> NortHingResult<WorkspaceInfo> {
        let path_buf = PathBuf::from(path);
        self.open_workspace(path_buf).await
    }
""",
        """    /// Quickly opens a workspace (using default options).
    pub async fn quick_open(&self, path: &str) -> NortHingResult<WorkspaceInfo> {
        self.quick_open_impl(path).await
    }
""",
    ),
    # 7. create_workspace (L344-375) — instance method
    (
        "create_workspace",
        """    /// Creates a workspace (for a new project).
    pub async fn create_workspace(
        &self,
        path: PathBuf,
        options: WorkspaceCreateOptions,
    ) -> NortHingResult<WorkspaceInfo> {
        if !path.exists() {
            tokio::fs::create_dir_all(&path).await.map_err(|e| {
                NortHingError::service(format!("Failed to create workspace directory: {}", e))
            })?;
        }

        let mut workspace = self
            .open_workspace_with_options(path, options.clone())
            .await?;

        if let Some(description) = options.description {
            workspace.description = Some(description);
        }

        workspace.tags = options.tags;

        {
            let mut manager = self.manager.write().await;
            manager
                .get_workspaces_mut()
                .insert(workspace.id.clone(), workspace.clone());
        }

        self.save_workspace_data().await?;

        Ok(workspace)
    }
""",
        """    /// Creates a workspace (for a new project).
    pub async fn create_workspace(
        &self,
        path: PathBuf,
        options: WorkspaceCreateOptions,
    ) -> NortHingResult<WorkspaceInfo> {
        self.create_workspace_impl(path, options).await
    }
""",
    ),
    # 8. create_assistant_workspace (L378-413) — instance method
    (
        "create_assistant_workspace",
        """    /// Creates and opens a new assistant workspace, then sets it as current.
    pub async fn create_assistant_workspace(
        &self,
        assistant_id: Option<String>,
    ) -> NortHingResult<WorkspaceInfo> {
        let assistant_id = match assistant_id {
            Some(id) if !id.trim().is_empty() => id.trim().to_string(),
            _ => self.generate_assistant_workspace_id().await?,
        };
        let display_name = Self::assistant_display_name(Some(&assistant_id));
        let path = self
            .path_manager
            .assistant_workspace_dir(&assistant_id, None);
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

        self.create_workspace(path, options).await
    }
""",
        """    /// Creates and opens a new assistant workspace, then sets it as current.
    pub async fn create_assistant_workspace(
        &self,
        assistant_id: Option<String>,
    ) -> NortHingResult<WorkspaceInfo> {
        self.create_assistant_workspace_impl(assistant_id).await
    }
""",
    ),
    # 9. close_current_workspace (L416-429) — instance method
    (
        "close_current_workspace",
        """    /// Closes the current workspace.
    pub async fn close_current_workspace(&self) -> NortHingResult<()> {
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
""",
        """    /// Closes the current workspace.
    pub async fn close_current_workspace(&self) -> NortHingResult<()> {
        self.close_current_workspace_impl().await
    }
""",
    ),
    # 10. close_workspace (L432-445) — instance method
    (
        "close_workspace",
        """    /// Closes the specified workspace.
    pub async fn close_workspace(&self, workspace_id: &str) -> NortHingResult<()> {
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
""",
        """    /// Closes the specified workspace.
    pub async fn close_workspace(&self, workspace_id: &str) -> NortHingResult<()> {
        self.close_workspace_impl(workspace_id).await
    }
""",
    ),
    # 11. set_active_workspace (L448-471) — instance method
    (
        "set_active_workspace",
        """    /// Sets the active workspace from the opened workspace list.
    pub async fn set_active_workspace(&self, workspace_id: &str) -> NortHingResult<()> {
        let result = {
            let mut manager = self.manager.write().await;
            manager.set_active_workspace(workspace_id)
        };

        if result.is_ok() {
            if let Err(e) = self.save_workspace_data().await {
                warn!(
                    "Failed to save workspace data after switching active workspace: {}",
                    e
                );
            }
        }

        if result.is_ok() {
            if let Some(workspace) = self.get_workspace(workspace_id).await {
                self.ensure_workspace_runtime_best_effort(&workspace, "activated")
                    .await;
            }
        }

        result
    }
""",
        """    /// Sets the active workspace from the opened workspace list.
    pub async fn set_active_workspace(&self, workspace_id: &str) -> NortHingResult<()> {
        self.set_active_workspace_impl(workspace_id).await
    }
""",
    ),
    # 12. reorder_opened_workspaces (L474-518) — instance method
    (
        "reorder_opened_workspaces",
        """    /// Reorders the opened workspaces without changing active or recent state.
    pub async fn reorder_opened_workspaces(
        &self,
        workspace_ids: Vec<String>,
    ) -> NortHingResult<()> {
        let current_ids = {
            let manager = self.manager.read().await;
            manager.get_opened_workspace_ids().clone()
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
                "Opened workspace order must contain exactly the currently opened workspace ids"
                    .to_string(),
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
""",
        """    /// Reorders the opened workspaces without changing active or recent state.
    pub async fn reorder_opened_workspaces(
        &self,
        workspace_ids: Vec<String>,
    ) -> NortHingResult<()> {
        self.reorder_opened_workspaces_impl(workspace_ids).await
    }
""",
    ),
    # 13. switch_to_workspace (L521-523) — instance method
    (
        "switch_to_workspace",
        """    /// Switches to the specified workspace.
    pub async fn switch_to_workspace(&self, workspace_id: &str) -> NortHingResult<()> {
        self.set_active_workspace(workspace_id).await
    }
""",
        """    /// Switches to the specified workspace.
    pub async fn switch_to_workspace(&self, workspace_id: &str) -> NortHingResult<()> {
        self.switch_to_workspace_impl(workspace_id).await
    }
""",
    ),
]


def main():
    content = read_current()
    applied = 0
    missed = []
    for name, old, new in REPLACEMENTS:
        if old in content:
            content = content.replace(old, new, 1)
            applied += 1
            print(f"OK   {name}")
        else:
            missed.append(name)
            print(f"MISS {name}")
    if missed:
        print(f"\nERROR: {len(missed)} methods not found in source:")
        for n in missed:
            print(f"  - {n}")
        sys.exit(1)
    write_current(content)
    print(f"\nApplied {applied} replacements.")


if __name__ == "__main__":
    main()