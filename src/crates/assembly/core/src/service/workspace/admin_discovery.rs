//! Workspace admin discovery sub-domain (R36 split from admin.rs).
//!
//! Owns `discover_assistant_workspaces` — scans the assistant workspace
//! root directory and returns descriptors for all valid assistant workspaces.

use super::service_types::AssistantWorkspaceDescriptor;
use super::WorkspaceService;
use crate::util::errors::*;
use tokio::fs;

impl WorkspaceService {
    /// Discovers all assistant workspaces on disk.
    ///
    /// First migrates any legacy assistant workspaces, then scans the
    /// assistant workspace root for `workspace-<id>` directories and
    /// returns descriptors sorted with the default workspace first.
    pub(super) async fn discover_assistant_workspaces(&self) -> NortHingResult<Vec<AssistantWorkspaceDescriptor>> {
        self.migrate_legacy_assistant_workspaces().await?;

        let assistant_root = self.path_manager.assistant_workspace_base_dir(None);
        fs::create_dir_all(&assistant_root).await.map_err(|e| {
            NortHingError::service(format!(
                "Failed to create assistant workspace root '{}': {}",
                assistant_root.display(),
                e
            ))
        })?;

        let default_workspace = self.path_manager.default_assistant_workspace_dir(None);
        fs::create_dir_all(&default_workspace).await.map_err(|e| {
            NortHingError::service(format!(
                "Failed to create default assistant workspace '{}': {}",
                default_workspace.display(),
                e
            ))
        })?;

        let mut descriptors = vec![AssistantWorkspaceDescriptor {
            path: default_workspace,
            assistant_id: None,
            display_name: Self::assistant_display_name(None),
        }];

        let mut entries = fs::read_dir(&assistant_root).await.map_err(|e| {
            NortHingError::service(format!(
                "Failed to read assistant workspace root '{}': {}",
                assistant_root.display(),
                e
            ))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            NortHingError::service(format!(
                "Failed to iterate assistant workspace root '{}': {}",
                assistant_root.display(),
                e
            ))
        })? {
            let file_type = entry.file_type().await.map_err(|e| {
                NortHingError::service(format!(
                    "Failed to inspect assistant workspace entry '{}': {}",
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

            descriptors.push(AssistantWorkspaceDescriptor {
                path: entry.path(),
                assistant_id: Some(assistant_id.to_string()),
                display_name: Self::assistant_display_name(Some(assistant_id)),
            });
        }

        descriptors.sort_by(
            |left, right| match (left.assistant_id.is_some(), right.assistant_id.is_some()) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => left.path.cmp(&right.path),
            },
        );

        Ok(descriptors)
    }
}
