//! SSH Connection Manager using russh
//!
//! This module owns the [`SSHConnectionManager`] struct and the active-connection
//! state, plus the connect/disconnect lifecycle methods. Sub-domain methods live in
//! sibling files (`manager_known_hosts`, `manager_remote_workspace`,
//! `manager_ssh_config`, `manager_saved_connections`, `manager_sftp`,
//! `manager_session_lifecycle`, `manager_command_dispatch`, `manager_tests`).
//!
//! Split history:
//! - Round 13 split the original 2810-line manager.rs into 1 facade + 3 sub-handlers
//!   (manager_handler, manager_session, manager_port_forward) to extract Russh
//!   callback + PTY/PortForward ownership.
//! - Round 13b continues the split, extracting per-sub-domain methods out of the
//!   facade so the facade is reduced from 2303 to ~150 lines and each sibling
//!   stays under the 800-line QClaw cap.
//!
//! Visibility discipline: struct fields are `pub(super)` rather than `pub` so they
//! remain encapsulated to the `remote_ssh` module ??sibling files can read/write
//! them, but nothing outside this crate can touch the raw maps.

use super::manager_handler::SSHHandler;
use crate::remote_ssh::manager_known_hosts::KnownHostEntry;
use crate::remote_ssh::password_vault::SSHPasswordVault;
use crate::remote_ssh::types::{RemoteWorkspace, SSHConnectionConfig, SavedConnection};
use anyhow::Context;
use russh::client::Handle;
use russh_keys::key::PublicKey;
use russh_sftp::client::SftpSession;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::Duration;

/// Poll interval for remote-exec wait loops. Used by `manager_session_lifecycle::execute_command_internal`.
pub(super) const SSH_COMMAND_WAIT_POLL_INTERVAL: Duration = Duration::from_millis(100);
/// Drain grace after interrupt/timeout before forcibly closing the exec channel.
/// Used by `manager_session_lifecycle::execute_command_internal`.
pub(super) const SSH_COMMAND_INTERRUPT_DRAIN_GRACE: Duration = Duration::from_millis(500);

/// Truncate `s` to at most `max_bytes` without splitting a UTF-8 code point.
/// Used by `manager_session_lifecycle::execute_command_internal` for command-preview logging.
pub(super) fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }

    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Active SSH connection state held in [`SSHConnectionManager::connections`].
///
/// Sibling files access this struct's fields when constructing or replacing
/// an entry on connect / reconnect, so it is `pub(super)` (visible to the
/// `remote_ssh` parent module and its descendants ??i.e., every sibling).
pub(super) struct ActiveConnection {
    pub(super) handle: Arc<Handle<SSHHandler>>,
    pub(super) config: SSHConnectionConfig,
    pub(super) server_info: Option<crate::remote_ssh::types::ServerInfo>,
    pub(super) sftp_session: Arc<tokio::sync::RwLock<Option<Arc<SftpSession>>>>,
    // reason: server_key is reserved for the upcoming host-key pinning verification; today's connection uses known_hosts-style trust
    pub(super) server_key: Option<PublicKey>,
    /// Liveness flag; flipped to false from `SSHHandler::disconnected`.
    /// Allows `is_connected` and SFTP/exec entry points to detect a dead session
    /// without waiting for the next failed I/O.
    pub(super) alive: Arc<AtomicBool>,
    /// Per-connection lock to serialize transparent reconnect attempts and
    /// avoid stampedes when multiple SFTP/exec calls hit a dead session at once.
    pub(super) reconnect_lock: Arc<tokio::sync::Mutex<()>>,
}

/// SSH Connection Manager.
///
/// Re-exports `KnownHostEntry` and `SSHConnectionManager` via [`crate::remote_ssh`]
/// (the `remote_ssh` module's `pub use` block) for callers outside this crate.
/// Sub-domain methods are split across sibling files:
/// - `manager_known_hosts` ??host-key verification store.
/// - `manager_remote_workspace` ??remote-workspace persistence.
/// - `manager_ssh_config` ??`~/.ssh/config` parsing.
/// - `manager_saved_connections` ??saved profile persistence + vault coupling.
/// - `manager_sftp` ??SFTP read/write/mkdir/rename operations.
/// - `manager_session_lifecycle` ??handshake + auth + execute + reconnect.
/// - `manager_command_dispatch` ??public command/PTY/exec entry points.
/// - `manager_tests` ??`#[cfg(test)]` unit tests.
#[derive(Clone)]
pub struct SSHConnectionManager {
    pub(super) connections: Arc<tokio::sync::RwLock<HashMap<String, ActiveConnection>>>,
    pub(super) saved_connections: Arc<tokio::sync::RwLock<Vec<SavedConnection>>>,
    pub(super) config_path: std::path::PathBuf,
    /// Known hosts storage
    pub(super) known_hosts: Arc<tokio::sync::RwLock<HashMap<String, KnownHostEntry>>>,
    pub(super) known_hosts_path: std::path::PathBuf,
    /// Remote workspace persistence (multiple workspaces)
    pub(super) remote_workspaces: Arc<tokio::sync::RwLock<Vec<RemoteWorkspace>>>,
    pub(super) remote_workspace_path: std::path::PathBuf,
    pub(super) password_vault: std::sync::Arc<SSHPasswordVault>,
}

impl SSHConnectionManager {
    /// Create a new SSH connection manager
    pub fn new(data_dir: std::path::PathBuf) -> Self {
        let config_path = data_dir.join("ssh_connections.json");
        let known_hosts_path = data_dir.join("known_hosts");
        let remote_workspace_path = data_dir.join("remote_workspace.json");
        let password_vault = std::sync::Arc::new(SSHPasswordVault::new(data_dir));
        Self {
            connections: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            saved_connections: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            config_path,
            known_hosts: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            known_hosts_path,
            remote_workspaces: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            remote_workspace_path,
            password_vault,
        }
    }

    /// Connect to a remote SSH server
    ///
    /// # Arguments
    /// * `config` - SSH connection configuration
    /// * `timeout_secs` - Connection timeout in seconds (default: 30)
    pub async fn connect(
        &self,
        config: SSHConnectionConfig,
    ) -> anyhow::Result<crate::remote_ssh::types::SSHConnectionResult> {
        self.connect_with_timeout(config, 30).await
    }

    /// Connect with custom timeout
    pub async fn connect_with_timeout(
        &self,
        config: SSHConnectionConfig,
        timeout_secs: u64,
    ) -> anyhow::Result<crate::remote_ssh::types::SSHConnectionResult> {
        let (handle, alive, server_info) = self
            .establish_session(&config, timeout_secs)
            .await
            .context("establish_session failed during connect_with_timeout")?;

        let connection_id = config.id.clone();

        let mut guard = self.connections.write().await;
        guard.insert(
            connection_id.clone(),
            ActiveConnection {
                handle: Arc::new(handle),
                config,
                server_info: server_info.clone(),
                sftp_session: Arc::new(tokio::sync::RwLock::new(None)),
                server_key: None,
                alive,
                reconnect_lock: Arc::new(tokio::sync::Mutex::new(())),
            },
        );

        Ok(crate::remote_ssh::types::SSHConnectionResult {
            success: true,
            connection_id: Some(connection_id),
            error: None,
            server_info,
        })
    }

    /// Disconnect from a server
    pub async fn disconnect(&self, connection_id: &str) -> anyhow::Result<()> {
        let mut guard = self.connections.write().await;
        guard.remove(connection_id);
        Ok(())
    }

    /// Disconnect all connections
    pub async fn disconnect_all(&self) {
        let mut guard = self.connections.write().await;
        guard.clear();
    }

    /// Check if connected.
    ///
    /// Returns true only when there is an entry in the connections map AND its
    /// liveness flag is still set. A previously-connected session that the
    /// server (or network) tore down is considered NOT connected even though
    /// the entry has not yet been pruned, so the UI cannot mistakenly believe
    /// the session is healthy.
    pub async fn is_connected(&self, connection_id: &str) -> bool {
        let guard = self.connections.read().await;
        guard
            .get(connection_id)
            .map(|c| c.alive.load(Ordering::SeqCst))
            .unwrap_or(false)
    }
}
