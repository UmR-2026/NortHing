//! Workspace admin migration sub-domain (R36 split from admin.rs).
//!
//! Owns `migrate_legacy_assistant_workspaces` — moves legacy assistant
//! workspace directories from the old layout to the new one.

use super::WorkspaceService;
use crate::util::errors::*;
use tokio::fs;
use tracing::info;

impl WorkspaceService {
    /// Migrates legacy assistant workspace directories to the new layout.
    ///
    /// Moves the default legacy workspace and all named (`workspace-<id>`)
    /// assistant workspaces from the legacy base dir to the current one.
    pub(super) async fn migrate_legacy_assistant_workspaces(&self) -> NortHingResult<()> {
        let assistant_root = self.path_manager.assistant_workspace_base_dir(None);
        fs::create_dir_all(&assistant_root).await.map_err(|e| {
            NortHingError::service(format!(
                "Failed to create assistant workspace root '{}': {}",
                assistant_root.display(),
                e
            ))
        })?;

        let legacy_root = self.path_manager.legacy_assistant_workspace_base_dir(None);
        let default_legacy_workspace = self.path_manager.legacy_default_assistant_workspace_dir(None);
        let default_workspace = self.path_manager.default_assistant_workspace_dir(None);

        if fs::try_exists(&default_legacy_workspace).await.map_err(|e| {
            NortHingError::service(format!(
                "Failed to inspect legacy assistant workspace '{}': {}",
                default_legacy_workspace.display(),
                e
            ))
        })? && !fs::try_exists(&default_workspace).await.map_err(|e| {
            NortHingError::service(format!(
                "Failed to inspect assistant workspace '{}': {}",
                default_workspace.display(),
                e
            ))
        })? {
            fs::rename(&default_legacy_workspace, &default_workspace)
                .await
                .map_err(|e| {
                    NortHingError::service(format!(
                        "Failed to migrate assistant workspace '{}' to '{}': {}",
                        default_legacy_workspace.display(),
                        default_workspace.display(),
                        e
                    ))
                })?;
            info!(
                "Migrated default assistant workspace: from={}, to={}",
                default_legacy_workspace.display(),
                default_workspace.display()
            );
        }

        let mut entries = fs::read_dir(&legacy_root).await.map_err(|e| {
            NortHingError::service(format!(
                "Failed to read legacy assistant workspace root '{}': {}",
                legacy_root.display(),
                e
            ))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            NortHingError::service(format!(
                "Failed to iterate legacy assistant workspace root '{}': {}",
                legacy_root.display(),
                e
            ))
        })? {
            let file_type = entry.file_type().await.map_err(|e| {
                NortHingError::service(format!(
                    "Failed to inspect legacy assistant workspace entry '{}': {}",
                    entry.path().display(),
                    e
                ))
            })?;
            if !file_type.is_dir() {
                continue;
            }

            let file_name = entry.file_name().to_string_lossy().to_string();
            let Some(assistant_id) = file_name.strip_prefix("workspace-") else {
                continue;
            };
            if assistant_id.trim().is_empty() {
                continue;
            }

            let target_path = self.path_manager.assistant_workspace_dir(assistant_id, None);
            if fs::try_exists(&target_path).await.map_err(|e| {
                NortHingError::service(format!(
                    "Failed to inspect assistant workspace '{}': {}",
                    target_path.display(),
                    e
                ))
            })? {
                continue;
            }

            fs::rename(entry.path(), &target_path).await.map_err(|e| {
                NortHingError::service(format!(
                    "Failed to migrate assistant workspace '{}' to '{}': {}",
                    file_name,
                    target_path.display(),
                    e
                ))
            })?;
            info!(
                "Migrated named assistant workspace: assistant_id={}, to={}",
                assistant_id,
                target_path.display()
            );
        }

        Ok(())
    }
}
