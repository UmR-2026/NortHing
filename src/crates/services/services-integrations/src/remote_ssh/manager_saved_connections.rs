//! Saved SSH connection profile persistence + password-vault coupling.
//!
//! Owns CRUD against the on-disk `ssh_connections.json` plus the encrypted
//! password-vault entry that backs password-based reconnect. Also exposes the
//! helper that rebuilds an in-memory `SSHConnectionConfig` from a saved profile
//! (used by `manager_session_lifecycle::ensure_alive_or_reconnect`).
//!
//! Split from `manager.rs` in Round 13b.

use crate::remote_ssh::manager::SSHConnectionManager;
use crate::remote_ssh::types::{SSHAuthMethod, SSHConnectionConfig, SavedAuthType, SavedConnection};
use anyhow::{anyhow, Context};

impl SSHConnectionManager {
    /// Load saved connections from disk
    pub async fn load_saved_connections(&self) -> anyhow::Result<()> {
        tracing::info!(
            "load_saved_connections: config_path={:?}, exists={}",
            self.config_path,
            self.config_path.exists()
        );

        if !self.config_path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&self.config_path).await?;
        tracing::info!("load_saved_connections: content={}", content);
        let saved: Vec<SavedConnection> =
            serde_json::from_str(&content).context("Failed to parse saved SSH connections")?;

        let mut guard = self.saved_connections.write().await;
        *guard = saved;

        // Migrate old-format connection IDs that include the port
        // (e.g. "ssh-root@host:22") to the new stable format ("ssh-root@host").
        // This ensures historical sessions can still find the connection after
        // the user changes the port.
        let mut migrated_ids = Vec::new();
        for conn in guard.iter_mut() {
            if let Some(new_id) = Self::migrate_connection_id(&conn.id) {
                let old_id = conn.id.clone();
                tracing::info!("Migrating saved connection ID: {} -> {}", old_id, new_id);
                conn.id = new_id.clone();
                migrated_ids.push((old_id, new_id));
            }
        }
        if !migrated_ids.is_empty() {
            drop(guard);
            for (old_id, new_id) in &migrated_ids {
                if let Err(e) = self.password_vault.migrate_entry(old_id, new_id).await {
                    tracing::warn!(
                        "Failed to migrate SSH password vault entry from {} to {}: {}",
                        old_id,
                        new_id,
                        e
                    );
                }
            }
            // Persist the migrated IDs to disk.
            if let Err(e) = self.save_connections().await {
                tracing::warn!("Failed to persist migrated connection IDs: {}", e);
            }
        } else {
            drop(guard);
        }

        let removed = self.prune_saved_connections_without_credentials().await?;
        if !removed.is_empty() {
            tracing::warn!(
                "Removed {} saved SSH connection(s) with unavailable local credentials during load",
                removed.len()
            );
        }

        let guard = self.saved_connections.read().await;
        tracing::info!("load_saved_connections: loaded {} connections", guard.len());
        Ok(())
    }

    /// If `id` follows the old format `ssh-{user}@{host}:{port}`, return the
    /// new stable format `ssh-{user}@{host}`.  Otherwise return `None`.
    fn migrate_connection_id(id: &str) -> Option<String> {
        if !id.starts_with("ssh-") {
            return None;
        }
        let rest = &id[4..]; // "{user}@{host}:{port}"
        let at_pos = rest.find('@')?;
        let colon_pos = rest.rfind(':')?;
        if colon_pos <= at_pos {
            return None;
        }
        // Verify the suffix after the last colon is a valid port number.
        let port_str = &rest[colon_pos + 1..];
        if port_str.parse::<u16>().is_ok() {
            let stable = format!("ssh-{}", &rest[..colon_pos]);
            // Only return if the ID actually changes (i.e. the port was present).
            if stable != id {
                return Some(stable);
            }
        }
        None
    }

    /// Save connections to disk
    pub(super) async fn save_connections(&self) -> anyhow::Result<()> {
        tracing::info!("save_connections: saving to {:?}", self.config_path);
        let guard = self.saved_connections.read().await;
        let content = serde_json::to_string_pretty(&*guard)?;
        tracing::info!("save_connections: content={}", content);

        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&self.config_path, content).await?;
        tracing::info!(
            "save_connections: saved {} connections to {:?}",
            guard.len(),
            self.config_path
        );
        Ok(())
    }

    /// Get list of saved connections
    pub async fn get_saved_connections(&self) -> Vec<SavedConnection> {
        if let Err(e) = self.prune_saved_connections_without_credentials().await {
            tracing::warn!("Failed to prune unavailable saved SSH connections: {}", e);
        }
        self.saved_connections.read().await.clone()
    }

    /// Remove saved profiles that cannot reconnect without user input, plus their
    /// persisted remote-workspace restore records. Passwords from older clients
    /// may not have a vault entry after an upgrade; keeping those profiles causes
    /// startup restore loops and hides matching SSH config hosts in the dialog.
    pub async fn prune_saved_connections_without_credentials(&self) -> anyhow::Result<Vec<String>> {
        let saved_snapshot = self.saved_connections.read().await.clone();
        let mut removed_ids = Vec::new();
        for conn in saved_snapshot {
            if !matches!(conn.auth_type, SavedAuthType::Password) {
                continue;
            }
            match self.password_vault.load(&conn.id).await {
                Ok(Some(_)) => {}
                Ok(None) => removed_ids.push(conn.id),
                Err(e) => {
                    tracing::warn!(
                        "Treating saved SSH password profile as unavailable: id={}, error={}",
                        conn.id,
                        e
                    );
                    removed_ids.push(conn.id);
                }
            }
        }

        if removed_ids.is_empty() {
            return Ok(Vec::new());
        }

        let removed_ids = {
            let mut guard = self.saved_connections.write().await;
            guard.retain(|conn| !removed_ids.iter().any(|id| id == &conn.id));
            removed_ids
        };

        for id in &removed_ids {
            if let Err(e) = self.password_vault.remove(id).await {
                tracing::warn!("Failed to remove SSH password vault entry for {}: {}", id, e);
            }
        }
        self.remove_remote_workspaces_for_connections(&removed_ids).await?;
        self.save_connections().await?;
        Ok(removed_ids)
    }

    /// SSH `host` field from the saved profile with this `connection_id` (works when not connected).
    /// Used to resolve session mirror paths when workspace metadata omitted `sshHost`.
    pub async fn get_saved_host_for_connection_id(&self, connection_id: &str) -> Option<String> {
        let cid = connection_id.trim();
        if cid.is_empty() {
            return None;
        }
        let guard = self.saved_connections.read().await;
        guard
            .iter()
            .find(|c| c.id == cid)
            .map(|c| c.host.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Save a connection configuration
    pub async fn save_connection(&self, config: &SSHConnectionConfig) -> anyhow::Result<()> {
        match &config.auth {
            SSHAuthMethod::Password { password } => {
                if password.is_empty() && self.password_vault.load(&config.id).await?.is_none() {
                    anyhow::bail!("Cannot save password SSH connection without a password or stored vault entry");
                }
                if !password.is_empty() {
                    self.password_vault
                        .store(&config.id, password)
                        .await
                        .with_context(|| format!("store ssh password vault for {}", config.id))?;
                }
            }
            SSHAuthMethod::PrivateKey { .. } => {
                self.password_vault.remove(&config.id).await?;
            }
        }

        let mut guard = self.saved_connections.write().await;

        // Remove existing entry with same id OR same host+username (dedup).
        // Using host+username (without port) so that changing the port replaces
        // the old entry instead of creating a duplicate.
        guard.retain(|c| c.id != config.id && !(c.host == config.host && c.username == config.username));

        // Add new entry
        guard.push(SavedConnection {
            id: config.id.clone(),
            name: config.name.clone(),
            host: config.host.clone(),
            port: config.port,
            username: config.username.clone(),
            auth_type: match &config.auth {
                SSHAuthMethod::Password { .. } => SavedAuthType::Password,
                SSHAuthMethod::PrivateKey { key_path, .. } => SavedAuthType::PrivateKey {
                    key_path: key_path.clone(),
                },
            },
            default_workspace: config.default_workspace.clone(),
            last_connected: Some(chrono::Utc::now().timestamp() as u64),
        });

        drop(guard);

        self.save_connections().await
    }

    /// Decrypt stored password for password-based saved connections (auto-reconnect).
    pub async fn load_stored_password(&self, connection_id: &str) -> anyhow::Result<Option<String>> {
        self.password_vault.load(connection_id).await
    }

    /// Whether the vault has a stored password for this connection (skip auto-reconnect when false).
    pub async fn has_stored_password(&self, connection_id: &str) -> bool {
        match self.load_stored_password(connection_id).await {
            Ok(opt) => opt.is_some(),
            Err(e) => {
                tracing::warn!("has_stored_password failed for {}: {}", connection_id, e);
                false
            }
        }
    }

    /// Delete a saved connection
    pub async fn delete_saved_connection(&self, connection_id: &str) -> anyhow::Result<()> {
        let mut guard = self.saved_connections.write().await;
        guard.retain(|c| c.id != connection_id);
        drop(guard);
        self.password_vault.remove(connection_id).await?;
        self.remove_remote_workspaces_for_connections(&[connection_id.to_string()])
            .await?;
        self.save_connections().await
    }

    /// Drop persisted remote workspace restore records for the given connection IDs.
    pub(super) async fn remove_remote_workspaces_for_connections(
        &self,
        connection_ids: &[String],
    ) -> anyhow::Result<()> {
        if connection_ids.is_empty() {
            return Ok(());
        }
        let removed = {
            let mut guard = self.remote_workspaces.write().await;
            let before = guard.len();
            guard.retain(|w| !connection_ids.iter().any(|id| id == &w.connection_id));
            before - guard.len()
        };
        if removed > 0 {
            tracing::warn!(
                "Removed {} persisted remote workspace(s) for unavailable SSH connection(s)",
                removed
            );
            self.save_remote_workspaces().await?;
        }
        Ok(())
    }

    /// Rebuild an `SSHConnectionConfig` from a saved profile (used by `ensure_alive_or_reconnect`).
    pub(super) async fn load_connection_config_from_saved(
        &self,
        connection_id: &str,
    ) -> anyhow::Result<Option<SSHConnectionConfig>> {
        let saved = {
            let guard = self.saved_connections.read().await;
            guard.iter().find(|conn| conn.id == connection_id).cloned()
        };

        let Some(saved) = saved else {
            return Ok(None);
        };

        let auth = match saved.auth_type {
            SavedAuthType::Password => {
                let password = self.password_vault.load(connection_id).await?.ok_or_else(|| {
                    anyhow!(
                        "Saved SSH connection {} requires a password, but no stored vault entry is available",
                        connection_id
                    )
                })?;
                SSHAuthMethod::Password { password }
            }
            SavedAuthType::PrivateKey { key_path } => SSHAuthMethod::PrivateKey {
                key_path,
                passphrase: None,
            },
        };

        Ok(Some(SSHConnectionConfig {
            id: saved.id,
            name: saved.name,
            host: saved.host,
            port: saved.port,
            username: saved.username,
            auth,
            default_workspace: saved.default_workspace,
        }))
    }
}
