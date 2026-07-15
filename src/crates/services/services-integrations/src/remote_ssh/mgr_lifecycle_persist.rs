//! Transparent SSH reconnect: drift detect, lock serialization, vault refresh.
//!
//! Owns `ensure_alive_or_reconnect` and the four phase helpers that rebuild a
//! dead session without disturbing the connection map:
//!
//! - Phase 1 `check_alive_and_drift` reads the latest saved profile, snapshots
//!   the live `ActiveConnection` triplet, and pre-flips the alive flag to
//!   `false` when the saved profile has drifted from the live config (so the
//!   reconnect path is taken even though the connection is technically still
//!   up).
//! - Phase 2 `recheck_under_lock` re-checks under the reconnect lock; another
//!   task may have already restored the session.
//! - Phase 3 `prepare_reconnect_config` picks the latest saved config (or
//!   falls back to the live config), logs the reconnect intent, and refreshes
//!   the password from the encrypted vault when the in-memory copy is empty.
//! - Phase 4 `perform_reconnect` re-establishes the session (delegating to
//!   [`crate::remote_ssh::manager::SSHConnectionManager::establish_session`])
//!   and either replaces the existing `ActiveConnection` entry or inserts a
//!   new one.
//!
//! Uses a per-connection mutex to prevent reconnect stampedes when many
//! concurrent SFTP/exec calls hit a dead session at the same time.

use crate::remote_ssh::manager::{ActiveConnection, SSHConnectionManager};
use crate::remote_ssh::types::{SSHAuthMethod, SSHConnectionConfig};
use anyhow::anyhow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

impl SSHConnectionManager {
    /// Ensure the connection is alive; if it was torn down (network blip,
    /// server-side timeout), transparently reconnect using the saved config
    /// and (for password auth) the encrypted password vault.
    ///
    /// Also detects config drift (e.g. the user changed the port after the
    /// connection was established) and forces a reconnect with the updated
    /// parameters so that historical sessions never use a stale port.
    ///
    /// Uses a per-connection mutex to prevent reconnect stampedes when many
    /// concurrent SFTP/exec calls hit a dead session at the same time.
    /// Idempotent: returns Ok(()) immediately when the session is already alive
    /// **and** its config matches the latest saved profile.
    pub(super) async fn ensure_alive_or_reconnect(&self, connection_id: &str) -> anyhow::Result<()> {
        let (alive_flag, reconnect_lock, saved_config, active_config) =
            self.check_alive_and_drift(connection_id).await?;

        // Serialize concurrent reconnect attempts for the same connection.
        let _guard = reconnect_lock.lock().await;
        // Re-check under lock; another task may have already restored the session.
        if !Self::recheck_under_lock(&alive_flag, saved_config.as_ref(), connection_id, &self.connections).await {
            return Ok(());
        }

        let config = self
            .prepare_reconnect_config(connection_id, saved_config, active_config)
            .await?;
        self.perform_reconnect(connection_id, config).await
    }

    /// Phase 1 of `ensure_alive_or_reconnect`: read saved config + the live
    /// `ActiveConnection` triplet, and pre-flip the alive flag to `false` when
    /// the saved profile has drifted from the live config (so the reconnect
    /// path below is taken even though the connection is technically still up).
    async fn check_alive_and_drift(
        &self,
        connection_id: &str,
    ) -> anyhow::Result<(
        Arc<AtomicBool>,
        Arc<tokio::sync::Mutex<()>>,
        Option<SSHConnectionConfig>,
        Option<SSHConnectionConfig>,
    )> {
        // Always read the latest saved config — this is the source of truth
        // after the user edits a connection (e.g. changes the port).
        let saved_config = self.load_connection_config_from_saved(connection_id).await?;

        let (alive_flag, reconnect_lock, active_config) = {
            let guard = self.connections.read().await;
            if let Some(conn) = guard.get(connection_id) {
                (
                    conn.alive.clone(),
                    conn.reconnect_lock.clone(),
                    Some(conn.config.clone()),
                )
            } else {
                (
                    Arc::new(AtomicBool::new(false)),
                    Arc::new(tokio::sync::Mutex::new(())),
                    None,
                )
            }
        };

        // If the connection is alive, check for config drift before returning.
        if alive_flag.load(Ordering::SeqCst) {
            if let Some(ref saved) = saved_config {
                if let Some(ref active) = active_config {
                    if !saved.connection_params_equal(active) {
                        tracing::warn!(
                            "SSH config for {} has drifted (e.g. port {} -> {}), forcing reconnect",
                            connection_id,
                            active.port,
                            saved.port
                        );
                        // Mark as dead so the reconnect path below is taken.
                        alive_flag.store(false, Ordering::SeqCst);
                    }
                }
            }
        }

        Ok((alive_flag, reconnect_lock, saved_config, active_config))
    }

    /// Phase 2 of `ensure_alive_or_reconnect`: with the reconnect lock held,
    /// re-check whether the connection is alive and matches the saved config.
    /// Returns `true` if the reconnect path should proceed.
    async fn recheck_under_lock(
        alive_flag: &Arc<AtomicBool>,
        saved_config: Option<&SSHConnectionConfig>,
        connection_id: &str,
        connections: &Arc<tokio::sync::RwLock<std::collections::HashMap<String, ActiveConnection>>>,
    ) -> bool {
        if !alive_flag.load(Ordering::SeqCst) {
            return true;
        }
        let Some(saved) = saved_config else {
            return false;
        };
        let guard = connections.read().await;
        match guard.get(connection_id) {
            Some(conn) => !saved.connection_params_equal(&conn.config),
            None => false,
        }
    }

    /// Phase 3 of `ensure_alive_or_reconnect`: pick the latest saved config (or
    /// fall back to the live config), log the reconnect intent, and refresh the
    /// password from the encrypted vault if the in-memory copy is empty.
    async fn prepare_reconnect_config(
        &self,
        connection_id: &str,
        saved_config: Option<SSHConnectionConfig>,
        active_config: Option<SSHConnectionConfig>,
    ) -> anyhow::Result<SSHConnectionConfig> {
        // Prefer the latest saved config for reconnection; fall back to the
        // active config only when no saved profile exists (should be rare).
        let mut config = match saved_config {
            Some(c) => c,
            None => active_config.ok_or_else(|| {
                anyhow!(
                    "Connection {} not found and no saved SSH profile is available",
                    connection_id
                )
            })?,
        };

        let is_existing_connection = {
            let guard = self.connections.read().await;
            guard.contains_key(connection_id)
        };
        if is_existing_connection {
            tracing::warn!(
                "SSH session {} is dead; attempting transparent reconnect",
                connection_id
            );
        } else {
            tracing::info!(
                "SSH session {} is not active; attempting to connect using saved SSH profile",
                connection_id
            );
        }

        // Refresh the password from the encrypted vault if password auth was
        // configured but the in-memory copy is empty (defensive — covers cases
        // where callers cleared it intentionally).
        if let SSHAuthMethod::Password { ref password } = config.auth {
            if password.is_empty() {
                match self.password_vault.load(connection_id).await {
                    Ok(Some(pwd)) => {
                        config.auth = SSHAuthMethod::Password { password: pwd };
                    }
                    Ok(None) => {
                        return Err(anyhow!(
                            "SSH session {} is dead and no stored password is available for reconnect",
                            connection_id
                        ));
                    }
                    Err(e) => {
                        return Err(anyhow!("Failed to load stored SSH password: {}", e));
                    }
                }
            }
        }

        Ok(config)
    }

    /// Phase 4 of `ensure_alive_or_reconnect`: re-establish the session and
    /// either replace the existing `ActiveConnection` entry or insert a new one.
    async fn perform_reconnect(&self, connection_id: &str, config: SSHConnectionConfig) -> anyhow::Result<()> {
        let (handle, alive, server_info) = self.establish_session(&config, 30).await?;

        // Replace the handle, update the config to the latest saved version,
        // and clear the cached SFTP session so subsequent operations open a
        // fresh channel on the new transport.
        {
            let mut guard = self.connections.write().await;
            if let Some(conn) = guard.get_mut(connection_id) {
                conn.handle = Arc::new(handle);
                conn.config = config;
                conn.alive = alive;
                if let Some(si) = server_info.as_ref() {
                    conn.server_info = Some(si.clone());
                }
                let mut sftp_guard = conn.sftp_session.write().await;
                *sftp_guard = None;
            } else {
                guard.insert(
                    connection_id.to_string(),
                    ActiveConnection {
                        handle: Arc::new(handle),
                        config,
                        server_info,
                        sftp_session: Arc::new(tokio::sync::RwLock::new(None)),
                        server_key: None,
                        alive,
                        reconnect_lock: Arc::new(tokio::sync::Mutex::new(())),
                    },
                );
            }
        }

        tracing::info!("SSH session {} reconnected successfully", connection_id);
        Ok(())
    }
}
