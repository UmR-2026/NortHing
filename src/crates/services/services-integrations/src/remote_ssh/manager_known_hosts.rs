//! Known-hosts storage and host-key verification support.
//!
//! Owns the [`KnownHostEntry`] DTO and the CRUD/persistence methods on
//! [`crate::remote_ssh::manager::SSHConnectionManager`] for the in-memory
//! known-hosts map plus its on-disk JSON file. The `known_hosts` field is read
//! by [`crate::remote_ssh::manager_handler::SSHHandler`] during the SSH handshake
//! to verify the server's host key.
//!
//! Split from `manager.rs` in Round 13b.

use crate::remote_ssh::manager::SSHConnectionManager;
use anyhow::{anyhow, Context};

/// Known hosts entry.
///
/// Re-exported via [`crate::remote_ssh`] (`pub use manager_known_hosts::KnownHostEntry`)
/// so callers outside this crate can read it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnownHostEntry {
    pub host: String,
    pub port: u16,
    pub key_type: String,
    pub fingerprint: String,
    pub public_key: String,
}

fn key_for(host: &str, port: u16) -> String {
    format!("{}:{}", host, port)
}

impl SSHConnectionManager {
    /// Load known hosts from disk
    pub async fn load_known_hosts(&self) -> anyhow::Result<()> {
        if !self.known_hosts_path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&self.known_hosts_path)
            .await
            .context("Failed to read known hosts file")?;

        let entries: Vec<KnownHostEntry> = if content.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&content).context("Failed to parse known hosts file")?
        };

        let mut guard = self.known_hosts.write().await;
        guard.clear();
        for entry in entries {
            guard.insert(key_for(&entry.host, entry.port), entry);
        }
        Ok(())
    }

    /// Persist the in-memory known-hosts map to disk.
    async fn save_known_hosts(&self) -> anyhow::Result<()> {
        let entries: Vec<KnownHostEntry> = {
            let guard = self.known_hosts.read().await;
            guard.values().cloned().collect()
        };

        if let Some(parent) = self.known_hosts_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(&entries)?;
        tokio::fs::write(&self.known_hosts_path, content)
            .await
            .context("Failed to write known hosts file")?;
        Ok(())
    }

    /// Add or replace the known host entry for `(host, port)`.
    pub async fn add_known_host(&self, host: &str, port: u16, entry: KnownHostEntry) -> anyhow::Result<()> {
        {
            let mut guard = self.known_hosts.write().await;
            guard.insert(key_for(host, port), entry);
        }
        self.save_known_hosts().await
    }

    /// Check whether a known host entry exists for `(host, port)`.
    pub async fn is_known_host(&self, host: &str, port: u16) -> bool {
        let guard = self.known_hosts.read().await;
        guard.contains_key(&key_for(host, port))
    }

    /// Get the known host entry for `(host, port)` if present.
    pub async fn get_known_host(&self, host: &str, port: u16) -> Option<KnownHostEntry> {
        let guard = self.known_hosts.read().await;
        guard.get(&key_for(host, port)).cloned()
    }

    /// Remove the known host entry for `(host, port)`. Returns an error if no such entry exists.
    pub async fn remove_known_host(&self, host: &str, port: u16) -> anyhow::Result<()> {
        let removed = {
            let mut guard = self.known_hosts.write().await;
            guard.remove(&key_for(host, port)).is_some()
        };
        if !removed {
            return Err(anyhow!("No known host entry exists for {}:{}", host, port));
        }
        self.save_known_hosts().await
    }

    /// List all known host entries.
    pub async fn list_known_hosts(&self) -> Vec<KnownHostEntry> {
        let guard = self.known_hosts.read().await;
        let mut entries: Vec<KnownHostEntry> = guard.values().cloned().collect();
        entries.sort_by(|a, b| a.host.cmp(&b.host).then(a.port.cmp(&b.port)));
        entries
    }
}
