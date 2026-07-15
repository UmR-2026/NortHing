//! R27b sibling manager_accessors — `impl WorkspaceManager` accessors/cleanup/recent/statistics + `WorkspaceManagerStatistics` struct.
//!
//! Mavis take-over (impl-block god-impl sub-domain split R27b). impl+struct
//! kept in same sibling for private field access.

#[cfg(feature = "service-integrations")]
use crate::service::git::GitService;
use crate::service::remote_ssh::workspace_state::{
    canonicalize_local_workspace_root, local_workspace_roots_equal, local_workspace_stable_storage_id,
    normalize_local_workspace_root_for_stable_id, normalize_remote_workspace_path, LOCAL_WORKSPACE_SSH_HOST,
};
use crate::util::{errors::*, FrontMatterMarkdown};
use tracing::{info, warn};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

pub use northhing_runtime_ports::RelatedPath;

use super::manager_lifecycle::*;
use super::types::*;
use super::workspace_info_impl::*;
use super::*;

impl WorkspaceManager {
    pub(super) fn set_current_workspace_with_recent_policy(
        &mut self,
        workspace_id: String,
        add_to_recent: bool,
    ) -> NortHingResult<()> {
        if !self.workspaces.contains_key(&workspace_id) {
            return Err(NortHingError::service(format!("Workspace not found: {}", workspace_id)));
        }

        self.ensure_workspace_open(&workspace_id);

        if let Some(previous_workspace_id) = &self.current_workspace_id {
            if previous_workspace_id != &workspace_id {
                if let Some(previous_workspace) = self.workspaces.get_mut(previous_workspace_id) {
                    previous_workspace.status = WorkspaceStatus::Inactive;
                }
            }
        }

        if let Some(workspace) = self.workspaces.get_mut(&workspace_id) {
            workspace.status = WorkspaceStatus::Active;
            workspace.touch();
        }

        self.current_workspace_id = Some(workspace_id.clone());

        if add_to_recent {
            self.update_recent_workspaces(workspace_id);
        }

        Ok(())
    }

    /// Gets the current workspace.
    pub fn current_workspace(&self) -> Option<&WorkspaceInfo> {
        if let Some(workspace_id) = &self.current_workspace_id {
            self.workspaces.get(workspace_id)
        } else {
            None
        }
    }

    /// Gets a workspace by id.
    pub fn get_workspace(&self, workspace_id: &str) -> Option<&WorkspaceInfo> {
        self.workspaces.get(workspace_id)
    }

    /// Gets all opened workspaces.
    pub fn opened_workspace_infos(&self) -> Vec<&WorkspaceInfo> {
        self.opened_workspace_ids
            .iter()
            .filter_map(|id| self.workspaces.get(id))
            .collect()
    }

    /// Lists all workspaces.
    pub fn list_workspaces(&self) -> Vec<WorkspaceSummary> {
        self.workspaces.values().map(|w| w.summary()).collect()
    }

    /// Returns recently accessed workspace records.
    pub fn recent_workspace_infos(&self) -> Vec<&WorkspaceInfo> {
        self.recent_workspaces
            .iter()
            .filter_map(|id| self.workspaces.get(id))
            .collect()
    }

    /// Returns recently accessed assistant workspace records.
    pub fn recent_assistant_workspace_infos(&self) -> Vec<&WorkspaceInfo> {
        self.recent_assistant_workspaces
            .iter()
            .filter_map(|id| self.workspaces.get(id))
            .collect()
    }

    /// Searches workspaces.
    pub fn search_workspaces(&self, query: &str) -> Vec<WorkspaceSummary> {
        let query_lower = query.to_lowercase();

        self.workspaces
            .values()
            .filter(|workspace| {
                workspace.name.to_lowercase().contains(&query_lower)
                    || workspace
                        .root_path
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&query_lower)
                    || workspace
                        .languages
                        .iter()
                        .any(|lang| lang.to_lowercase().contains(&query_lower))
                    || workspace
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .map(|w| w.summary())
            .collect()
    }

    /// Removes a workspace.
    pub fn remove_workspace(&mut self, workspace_id: &str) -> NortHingResult<()> {
        if self.workspaces.remove(workspace_id).is_some() {
            if self.current_workspace_id.as_ref() == Some(&workspace_id.to_string()) {
                self.current_workspace_id = None;
            }

            self.opened_workspace_ids.retain(|id| id != workspace_id);
            self.recent_workspaces.retain(|id| id != workspace_id);
            self.recent_assistant_workspaces.retain(|id| id != workspace_id);

            Ok(())
        } else {
            Err(NortHingError::service(format!("Workspace not found: {}", workspace_id)))
        }
    }

    /// Cleans up invalid workspaces.
    pub async fn cleanup_invalid_workspaces(&mut self) -> NortHingResult<usize> {
        let mut invalid_workspaces = Vec::new();

        for (workspace_id, workspace) in &self.workspaces {
            if !workspace.is_valid().await {
                invalid_workspaces.push(workspace_id.clone());
            }
        }

        let count = invalid_workspaces.len();
        for workspace_id in invalid_workspaces {
            self.remove_workspace(&workspace_id)?;
        }

        Ok(count)
    }

    /// Updates the recent-workspaces list.
    pub(super) fn update_recent_workspaces(&mut self, workspace_id: String) {
        self.recent_workspaces.retain(|id| id != &workspace_id);
        self.recent_assistant_workspaces.retain(|id| id != &workspace_id);

        let is_assistant = self
            .workspaces
            .get(&workspace_id)
            .map(|workspace| workspace.workspace_kind == WorkspaceKind::Assistant)
            .unwrap_or(false);
        let target_list = if is_assistant {
            &mut self.recent_assistant_workspaces
        } else {
            &mut self.recent_workspaces
        };
        target_list.insert(0, workspace_id);

        if target_list.len() > self.max_recent_workspaces {
            target_list.truncate(self.max_recent_workspaces);
        }
    }

    pub(super) fn touch_workspace_access(&mut self, workspace_id: &str, add_to_recent: bool) {
        if let Some(workspace) = self.workspaces.get_mut(workspace_id) {
            workspace.touch();
            if self.current_workspace_id.as_deref() != Some(workspace_id) {
                workspace.status = WorkspaceStatus::Inactive;
            }
        }

        if add_to_recent {
            self.update_recent_workspaces(workspace_id.to_string());
        }
    }

    pub(super) fn find_next_workspace_id_after_close(&self, preferred_kind: &WorkspaceKind) -> Option<String> {
        let same_kind = self
            .opened_workspace_ids
            .iter()
            .find(|id| {
                self.workspaces
                    .get(id.as_str())
                    .map(|workspace| &workspace.workspace_kind == preferred_kind)
                    .unwrap_or(false)
            })
            .cloned();

        if same_kind.is_some() {
            return same_kind;
        }

        // Closing the last remote workspace (e.g. SSH password session could not auto-reconnect)
        // must not activate an unrelated local project; leave current unset until the user picks
        // a workspace or reconnects.
        if *preferred_kind == WorkspaceKind::Remote {
            return None;
        }

        self.opened_workspace_ids.first().cloned()
    }

    /// Ensures a workspace stays in the opened list.
    pub(super) fn ensure_workspace_open(&mut self, workspace_id: &str) {
        self.opened_workspace_ids.retain(|id| id != workspace_id);
        self.opened_workspace_ids.insert(0, workspace_id.to_string());
    }

    /// Returns manager statistics.
    pub fn statistics(&self) -> WorkspaceManagerStatistics {
        let mut stats = WorkspaceManagerStatistics {
            total_workspaces: self.workspaces.len(),
            ..WorkspaceManagerStatistics::default()
        };

        for workspace in self.workspaces.values() {
            match workspace.status {
                WorkspaceStatus::Active => stats.active_workspaces += 1,
                WorkspaceStatus::Inactive => stats.inactive_workspaces += 1,
                WorkspaceStatus::Archived => stats.archived_workspaces += 1,
                _ => {}
            }

            *stats
                .workspaces_by_type
                .entry(workspace.workspace_type.clone())
                .or_insert(0) += 1;

            if let Some(statistics) = &workspace.statistics {
                stats.total_files += statistics.total_files;
                stats.total_size_bytes += statistics.total_size_bytes;
            }
        }

        stats
    }

    /// Returns the number of workspaces.
    pub fn workspace_count(&self) -> usize {
        self.workspaces.len()
    }

    /// Returns an immutable reference to the workspace map (for export).
    pub fn workspaces(&self) -> &HashMap<String, WorkspaceInfo> {
        &self.workspaces
    }

    /// Returns a mutable reference to the workspace map (for import).
    pub fn workspaces_mut(&mut self) -> &mut HashMap<String, WorkspaceInfo> {
        &mut self.workspaces
    }

    /// Returns the opened workspace ids.
    pub fn opened_workspace_ids(&self) -> &Vec<String> {
        &self.opened_workspace_ids
    }

    /// Sets the opened workspace ids.
    pub fn set_opened_workspace_ids(&mut self, opened_workspace_ids: Vec<String>) {
        self.opened_workspace_ids = opened_workspace_ids
            .into_iter()
            .filter(|id| self.workspaces.contains_key(id))
            .collect();
    }

    /// Removes a workspace id from recent lists only (does not unregister the workspace).
    pub fn remove_from_recent_workspaces_only(&mut self, workspace_id: &str) -> bool {
        let mut changed = false;
        let before = self.recent_workspaces.len();
        self.recent_workspaces.retain(|id| id != workspace_id);
        if self.recent_workspaces.len() != before {
            changed = true;
        }
        let before_a = self.recent_assistant_workspaces.len();
        self.recent_assistant_workspaces.retain(|id| id != workspace_id);
        if self.recent_assistant_workspaces.len() != before_a {
            changed = true;
        }
        changed
    }

    /// Returns a reference to the recent-workspaces list.
    pub fn recent_workspaces(&self) -> &Vec<String> {
        &self.recent_workspaces
    }

    /// Sets the recent-workspaces list.
    pub fn set_recent_workspaces(&mut self, recent: Vec<String>) {
        self.recent_workspaces = recent
            .into_iter()
            .filter(|id| {
                self.workspaces
                    .get(id)
                    .map(|workspace| workspace.workspace_kind == WorkspaceKind::Normal)
                    .unwrap_or(false)
            })
            .collect();
    }

    /// Returns a reference to the recent assistant-workspaces list.
    pub fn recent_assistant_workspaces(&self) -> &Vec<String> {
        &self.recent_assistant_workspaces
    }

    /// Sets the recent assistant-workspaces list.
    pub fn set_recent_assistant_workspaces(&mut self, recent: Vec<String>) {
        self.recent_assistant_workspaces = recent
            .into_iter()
            .filter(|id| {
                self.workspaces
                    .get(id)
                    .map(|workspace| workspace.workspace_kind == WorkspaceKind::Assistant)
                    .unwrap_or(false)
            })
            .collect();
    }
}

/// Workspace manager statistics.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceManagerStatistics {
    pub total_workspaces: usize,
    pub active_workspaces: usize,
    pub inactive_workspaces: usize,
    pub archived_workspaces: usize,
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub workspaces_by_type: HashMap<WorkspaceType, usize>,
}
