//! Remote-workspace persistence.
//!
//! Owns the per-connection workspace restore records (one per `(connection_id,
//! remote_path)` pair). Each entry remembers the last-used remote directory for
//! a given SSH profile so the remote folder picker can reopen at the same path
//! across sessions.
//!
//! Split from `manager.rs` in Round 13b.

use crate::remote_ssh::manager::SSHConnectionManager;
use crate::remote_ssh::types::RemoteWorkspace;
use anyhow::Context;

impl SSHConnectionManager {
    /// Load remote workspaces from disk
    pub async fn load_remote_workspace(&self) -> anyhow::Result<()> {
        if !self.remote_workspace_path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&self.remote_workspace_path).await?;
        // Try array format first, fall back to single-object for backward compat
        let mut workspaces: Vec<RemoteWorkspace> = serde_json::from_str(&content)
            .or_else(|_| {
                // Legacy: single workspace object
                serde_json::from_str::<RemoteWorkspace>(&content).map(|ws| vec![ws])
            })
            .context("Failed to parse remote workspace(s)")?;

        let before = workspaces.len();
        workspaces.retain(|w| !w.connection_id.is_empty() && !w.remote_path.is_empty());
        if workspaces.len() < before {
            tracing::warn!(
                "Dropped {} persisted remote workspace(s) with empty connectionId or remotePath",
                before - workspaces.len()
            );
        }

        let mut guard = self.remote_workspaces.write().await;
        *guard = workspaces;

        Ok(())
    }

    /// Save remote workspaces to disk
    pub(super) async fn save_remote_workspaces(&self) -> anyhow::Result<()> {
        let guard = self.remote_workspaces.read().await;

        if let Some(parent) = self.remote_workspace_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(&*guard)?;
        tokio::fs::write(&self.remote_workspace_path, content).await?;
        Ok(())
    }

    /// Add/update a persisted remote workspace (key = `connection_id` + `remote_path`).
    pub async fn set_remote_workspace(&self, mut workspace: RemoteWorkspace) -> anyhow::Result<()> {
        workspace.remote_path = crate::remote_ssh::normalize_remote_workspace_path(&workspace.remote_path);
        {
            let mut guard = self.remote_workspaces.write().await;
            let rp = workspace.remote_path.clone();
            let cid = workspace.connection_id.clone();
            guard.retain(|w| {
                !(w.connection_id == cid && crate::remote_ssh::normalize_remote_workspace_path(&w.remote_path) == rp)
            });
            guard.push(workspace);
        }
        self.save_remote_workspaces().await
    }

    /// Get all persisted remote workspaces
    pub async fn get_remote_workspaces(&self) -> Vec<RemoteWorkspace> {
        self.remote_workspaces.read().await.clone()
    }

    /// Drop persisted remote workspace restore entries whose saved SSH profile is gone.
    pub async fn prune_remote_workspaces_without_saved_connections(&self) -> anyhow::Result<Vec<RemoteWorkspace>> {
        let saved_ids: Vec<String> = self
            .saved_connections
            .read()
            .await
            .iter()
            .map(|c| c.id.clone())
            .collect();

        let removed = {
            let mut guard = self.remote_workspaces.write().await;
            let mut removed = Vec::new();
            guard.retain(|w| {
                let keep = saved_ids.iter().any(|id| id == &w.connection_id);
                if !keep {
                    removed.push(w.clone());
                }
                keep
            });
            removed
        };

        if !removed.is_empty() {
            tracing::warn!(
                "Removed {} persisted remote workspace(s) without saved SSH connection",
                removed.len()
            );
            self.save_remote_workspaces().await?;
        }

        Ok(removed)
    }

    /// Get first persisted remote workspace (legacy compat)
    pub async fn get_remote_workspace(&self) -> Option<RemoteWorkspace> {
        self.remote_workspaces.read().await.first().cloned()
    }

    /// Remove a specific remote workspace by **connection** + **remote path** (not path alone).
    pub async fn remove_remote_workspace(&self, connection_id: &str, remote_path: &str) -> anyhow::Result<()> {
        let rp = crate::remote_ssh::normalize_remote_workspace_path(remote_path);
        {
            let mut guard = self.remote_workspaces.write().await;
            guard.retain(|w| {
                !(w.connection_id == connection_id
                    && crate::remote_ssh::normalize_remote_workspace_path(&w.remote_path) == rp)
            });
        }
        self.save_remote_workspaces().await
    }

    /// Clear all remote workspaces
    pub async fn clear_remote_workspace(&self) -> anyhow::Result<()> {
        {
            let mut guard = self.remote_workspaces.write().await;
            guard.clear();
        }
        if self.remote_workspace_path.exists() {
            tokio::fs::remove_file(&self.remote_workspace_path).await?;
        }
        Ok(())
    }
}
