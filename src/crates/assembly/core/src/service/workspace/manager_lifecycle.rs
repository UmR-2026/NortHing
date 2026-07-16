//! R27b sibling manager_lifecycle — `impl WorkspaceManager` lifecycle methods (new/rekey/migrate/open/close/set_active/set_current).
//!
//! Mavis take-over (impl-block god-impl sub-domain split R27b). impl+struct
//! kept in same sibling for private field access.

#[cfg(feature = "service-integrations")]
use crate::service::git::GitService;
use crate::service::remote_ssh::workspace_state::{
    canonicalize_local_workspace_root, local_workspace_roots_equal, local_workspace_stable_storage_id,
    normalize_local_workspace_root_for_stable_id, normalize_remote_workspace_path, LOCAL_WORKSPACE_SSH_HOST,
};
use crate::util::errors::*;
use tracing::{info, warn};

use std::collections::HashMap;
use std::path::PathBuf;


use super::workspace_info_impl::*;
use super::*;

impl WorkspaceManager {
    /// Creates a new workspace manager.
    pub fn new(config: WorkspaceManagerConfig) -> Self {
        Self {
            workspaces: HashMap::new(),
            opened_workspace_ids: Vec::new(),
            current_workspace_id: None,
            recent_workspaces: Vec::new(),
            recent_assistant_workspaces: Vec::new(),
            max_recent_workspaces: config.max_recent_workspaces,
        }
    }

    /// Reassigns a workspace id (e.g. migrating from UUID to `local_*` stable id).
    pub fn rekey_workspace_id(&mut self, old_id: &str, new_id: String) -> NortHingResult<()> {
        if old_id == new_id.as_str() {
            return Ok(());
        }
        let Some(mut workspace) = self.workspaces.remove(old_id) else {
            return Err(NortHingError::service(format!(
                "rekey_workspace_id: workspace not found: {}",
                old_id
            )));
        };
        if self.workspaces.contains_key(&new_id) {
            self.workspaces.insert(old_id.to_string(), workspace);
            return Err(NortHingError::service(format!(
                "rekey_workspace_id: target id already exists: {}",
                new_id
            )));
        }
        workspace.id = new_id.clone();
        if workspace.workspace_kind != WorkspaceKind::Remote {
            if let Ok((pb, _)) = canonicalize_local_workspace_root(&workspace.root_path) {
                workspace.root_path = pb;
            }
            workspace
                .metadata
                .insert("sshHost".to_string(), serde_json::json!(LOCAL_WORKSPACE_SSH_HOST));
        }
        self.workspaces.insert(new_id.clone(), workspace);

        for id in &mut self.opened_workspace_ids {
            if id.as_str() == old_id {
                *id = new_id.clone();
            }
        }
        if let Some(ref mut cur) = self.current_workspace_id {
            if cur.as_str() == old_id {
                *cur = new_id.clone();
            }
        }
        for rid in &mut self.recent_workspaces {
            if rid.as_str() == old_id {
                *rid = new_id.clone();
            }
        }
        for rid in &mut self.recent_assistant_workspaces {
            if rid.as_str() == old_id {
                *rid = new_id.clone();
            }
        }
        Ok(())
    }

    /// Migrates persisted local/assistant workspaces from legacy UUID ids to `local_*` stable ids.
    /// Returns a map from **old** id to **new** id for callers that still hold persisted workspace ids.
    pub fn migrate_local_workspace_ids_to_stable_storage(&mut self) -> HashMap<String, String> {
        let mut id_remap: HashMap<String, String> = HashMap::new();
        let old_ids: Vec<String> = self.workspaces.keys().cloned().collect();
        for old_id in old_ids {
            let Some(ws) = self.workspaces.get(&old_id).cloned() else {
                continue;
            };
            if ws.workspace_kind == WorkspaceKind::Remote {
                continue;
            }
            if old_id.starts_with("local_") {
                continue;
            }
            let Ok(norm) = normalize_local_workspace_root_for_stable_id(&ws.root_path) else {
                continue;
            };
            let new_id = local_workspace_stable_storage_id(&norm);
            if new_id == old_id {
                continue;
            }
            if self.workspaces.contains_key(&new_id) {
                info!(
                    "Dropping duplicate local workspace record (legacy id {}) in favor of stable id {}",
                    old_id, new_id
                );
                self.workspaces.remove(&old_id);
                self.opened_workspace_ids.retain(|x| x != &old_id);
                self.recent_workspaces.retain(|x| x != &old_id);
                self.recent_assistant_workspaces.retain(|x| x != &old_id);
                if self.current_workspace_id.as_deref() == Some(old_id.as_str()) {
                    self.current_workspace_id = Some(new_id.clone());
                }
                id_remap.insert(old_id, new_id);
                continue;
            }
            match self.rekey_workspace_id(&old_id, new_id.clone()) {
                Ok(()) => {
                    id_remap.insert(old_id, new_id);
                }
                Err(e) => {
                    warn!(
                        "migrate_local_workspace_ids_to_stable_storage: failed to rekey {}: {}",
                        old_id, e
                    );
                }
            }
        }
        id_remap
    }

    /// Opens a workspace.
    pub async fn open_workspace(&mut self, path: PathBuf) -> NortHingResult<WorkspaceInfo> {
        self.open_workspace_with_options(path, WorkspaceOpenOptions::default())
            .await
    }

    /// Opens a workspace with custom options.
    pub async fn open_workspace_with_options(
        &mut self,
        path: PathBuf,
        options: WorkspaceOpenOptions,
    ) -> NortHingResult<WorkspaceInfo> {
        self.upsert_workspace_with_options(path, options, true).await
    }

    /// Registers or refreshes workspace activity without changing opened UI state.
    pub async fn track_workspace_with_options(
        &mut self,
        path: PathBuf,
        options: WorkspaceOpenOptions,
    ) -> NortHingResult<WorkspaceInfo> {
        self.upsert_workspace_with_options(path, options, false).await
    }

    pub(super) async fn upsert_workspace_with_options(
        &mut self,
        path: PathBuf,
        options: WorkspaceOpenOptions,
        keep_opened: bool,
    ) -> NortHingResult<WorkspaceInfo> {
        let is_remote = options.workspace_kind == WorkspaceKind::Remote;

        if !is_remote {
            if !path.exists() {
                return Err(NortHingError::service(format!(
                    "Workspace path does not exist: {:?}",
                    path
                )));
            }

            if !path.is_dir() {
                return Err(NortHingError::service(format!(
                    "Workspace path is not a directory: {:?}",
                    path
                )));
            }
        }

        let existing_workspace_id = if is_remote {
            let desired = options
                .remote_connection_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let stable = options
                .stable_workspace_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let host_opt = options
                .remote_ssh_host
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let path_norm = normalize_remote_workspace_path(&path.to_string_lossy());

            let by_stable = stable.and_then(|sid| self.workspaces.get(sid)).and_then(|w| {
                if w.workspace_kind == WorkspaceKind::Remote
                    && normalize_remote_workspace_path(&w.root_path.to_string_lossy()) == path_norm
                {
                    Some(w.id.clone())
                } else {
                    None
                }
            });

            if let Some(id) = by_stable {
                Some(id)
            } else {
                self.workspaces
                    .values()
                    .find(|w| {
                        if w.workspace_kind != WorkspaceKind::Remote {
                            return false;
                        }
                        if normalize_remote_workspace_path(&w.root_path.to_string_lossy()) != path_norm {
                            return false;
                        }
                        let existing = w.remote_ssh_connection_id();
                        let conn_ok = match desired {
                            Some(d) => existing == Some(d),
                            None => existing.is_none(),
                        };
                        if !conn_ok {
                            return false;
                        }
                        if let Some(h) = host_opt {
                            match w
                                .metadata
                                .get("sshHost")
                                .and_then(|v| v.as_str())
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                            {
                                None => true,
                                Some(wh) => wh == h,
                            }
                        } else {
                            true
                        }
                    })
                    .map(|w| w.id.clone())
            }
        } else {
            let canon_norm = match normalize_local_workspace_root_for_stable_id(&path) {
                Ok(n) => n,
                Err(e) => return Err(NortHingError::service(e)),
            };
            let stable_local_id = local_workspace_stable_storage_id(&canon_norm);

            if self.workspaces.contains_key(&stable_local_id) {
                Some(stable_local_id)
            } else {
                let legacy_id = self
                    .workspaces
                    .iter()
                    .find(|(wid, w)| {
                        w.workspace_kind != WorkspaceKind::Remote
                            && wid.as_str() != stable_local_id.as_str()
                            && local_workspace_roots_equal(&w.root_path, &path)
                    })
                    .map(|(wid, _)| wid.clone());

                if let Some(legacy) = legacy_id {
                    match self.rekey_workspace_id(&legacy, stable_local_id.clone()) {
                        Ok(()) => Some(stable_local_id),
                        Err(e) => {
                            warn!(
                                "Could not rekey local workspace {} -> {}: {}",
                                legacy, stable_local_id, e
                            );
                            Some(legacy)
                        }
                    }
                } else {
                    None
                }
            }
        };

        if let Some(workspace_id) = existing_workspace_id {
            if let Some(workspace) = self.workspaces.get_mut(&workspace_id) {
                workspace.workspace_kind = options.workspace_kind.clone();
                workspace.assistant_id = if options.workspace_kind == WorkspaceKind::Assistant {
                    options.assistant_id.clone()
                } else {
                    None
                };
                if let Some(display_name) = &options.display_name {
                    workspace.name = display_name.clone();
                }
                if options.workspace_kind == WorkspaceKind::Remote {
                    if let Some(ssh_host) = options.remote_ssh_host.as_ref().filter(|s| !s.trim().is_empty()) {
                        workspace.metadata.insert(
                            "sshHost".to_string(),
                            serde_json::Value::String(ssh_host.trim().to_string()),
                        );
                    }
                    if let Some(conn_id) = options.remote_connection_id.as_ref().filter(|s| !s.trim().is_empty()) {
                        workspace.metadata.insert(
                            "connectionId".to_string(),
                            serde_json::Value::String(conn_id.trim().to_string()),
                        );
                    }
                }
                workspace.load_identity().await;
                workspace.load_worktree().await;
            }
            if keep_opened {
                self.ensure_workspace_open(&workspace_id);
            }
            if options.auto_set_current {
                self.set_current_workspace_with_recent_policy(workspace_id.clone(), options.add_to_recent)?;
            } else {
                self.touch_workspace_access(&workspace_id, options.add_to_recent);
            }
            return self.workspaces.get(&workspace_id).cloned().ok_or_else(|| {
                NortHingError::service(format!("Workspace '{}' disappeared after selecting it", workspace_id))
            });
        }

        let workspace = WorkspaceInfo::new(path, options.clone()).await?;
        let workspace_id = workspace.id.clone();

        self.workspaces.insert(workspace_id.clone(), workspace.clone());
        if keep_opened {
            self.ensure_workspace_open(&workspace_id);
        }
        if options.auto_set_current {
            self.set_current_workspace_with_recent_policy(workspace_id.clone(), options.add_to_recent)?;
        } else {
            self.touch_workspace_access(&workspace_id, options.add_to_recent);
        }

        Ok(workspace)
    }

    /// Closes the current workspace.
    pub fn close_current_workspace(&mut self) -> NortHingResult<()> {
        let current_workspace_id = self.current_workspace_id.clone();
        match current_workspace_id {
            Some(workspace_id) => self.close_workspace(&workspace_id),
            None => Ok(()),
        }
    }

    /// Closes the specified workspace.
    pub fn close_workspace(&mut self, workspace_id: &str) -> NortHingResult<()> {
        if !self.workspaces.contains_key(workspace_id) {
            return Err(NortHingError::service(format!("Workspace not found: {}", workspace_id)));
        }
        let closed_workspace_kind = self
            .workspaces
            .get(workspace_id)
            .map(|workspace| workspace.workspace_kind.clone())
            .unwrap_or_default();

        self.opened_workspace_ids.retain(|id| id != workspace_id);

        if let Some(workspace) = self.workspaces.get_mut(workspace_id) {
            workspace.status = WorkspaceStatus::Inactive;
        }

        if self.current_workspace_id.as_deref() == Some(workspace_id) {
            self.current_workspace_id = None;

            if let Some(next_workspace_id) = self.find_next_workspace_id_after_close(&closed_workspace_kind) {
                self.set_current_workspace(next_workspace_id)?;
            }
        }

        Ok(())
    }

    /// Sets the active workspace among already opened workspaces.
    pub fn set_active_workspace(&mut self, workspace_id: &str) -> NortHingResult<()> {
        if !self.opened_workspace_ids.iter().any(|id| id == workspace_id) {
            return Err(NortHingError::service(format!(
                "Workspace is not opened: {}",
                workspace_id
            )));
        }

        self.set_current_workspace(workspace_id.to_string())
    }

    /// Sets the current workspace.
    pub fn set_current_workspace(&mut self, workspace_id: String) -> NortHingResult<()> {
        self.set_current_workspace_with_recent_policy(workspace_id, true)
    }
}
