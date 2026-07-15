//! Public command/PTY/exec entry points on [`SSHConnectionManager`].
//!
//! These are the methods that the rest of the codebase (remote folder picker,
//! command runner, PTY terminal, etc.) calls. The actual implementation lives
//! in `manager_session_lifecycle` (handshake/auth/exec/reconnect) and
//! `manager_sftp` (SFTP); this file is the thin entry-point layer that
//! resolves the connection, fetches the handle, and delegates.
//!
//! Split from `manager.rs` in Round 13b.

use crate::remote_ssh::manager::SSHConnectionManager;
use crate::remote_ssh::manager_session::PTYSession;
use crate::remote_ssh::types::{SSHCommandOptions, SSHCommandResult, SSHConnectionConfig, ServerInfo};
use anyhow::anyhow;
use russh::client::Msg;

impl SSHConnectionManager {
    /// Execute a command on the remote server
    pub async fn execute_command(&self, connection_id: &str, command: &str) -> anyhow::Result<(String, String, i32)> {
        let result = self
            .execute_command_with_options(connection_id, command, SSHCommandOptions::default())
            .await?;

        if result.timed_out {
            return Err(anyhow!("Command timed out"));
        }
        if result.interrupted {
            return Err(anyhow!("Command was cancelled"));
        }

        Ok((result.stdout, result.stderr, result.exit_code))
    }

    /// Execute a command on the remote server with structured timeout/cancellation handling.
    pub async fn execute_command_with_options(
        &self,
        connection_id: &str,
        command: &str,
        options: SSHCommandOptions,
    ) -> anyhow::Result<SSHCommandResult> {
        self.ensure_alive_or_reconnect(connection_id).await?;
        let handle = {
            let guard = self.connections.read().await;
            guard
                .get(connection_id)
                .ok_or_else(|| anyhow!("Connection {} not found", connection_id))?
                .handle
                .clone()
        };

        Self::execute_command_internal(&handle, command, options)
            .await
            .map_err(|e| anyhow!("Command execution failed: {}", e))
    }

    /// Open a long-lived non-PTY exec channel for streaming stdin/stdout protocols.
    pub async fn open_exec_channel(&self, connection_id: &str, command: &str) -> anyhow::Result<russh::Channel<Msg>> {
        self.ensure_alive_or_reconnect(connection_id).await?;
        let handle = {
            let guard = self.connections.read().await;
            guard
                .get(connection_id)
                .ok_or_else(|| anyhow!("Connection {} not found", connection_id))?
                .handle
                .clone()
        };

        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| anyhow!("Failed to open SSH exec channel: {}", e))?;
        channel
            .exec(true, command)
            .await
            .map_err(|e| anyhow!("Failed to start remote command: {}", e))?;
        Ok(channel)
    }

    /// Open a long-lived exec channel with a PTY attached.
    ///
    /// This gives the command TTY semantics without starting an interactive shell
    /// and typing the command into it, so command wrappers are not echoed into
    /// model-visible output.
    pub async fn open_pty_exec_channel(
        &self,
        connection_id: &str,
        command: &str,
        cols: u32,
        rows: u32,
    ) -> anyhow::Result<russh::Channel<Msg>> {
        self.ensure_alive_or_reconnect(connection_id).await?;
        let handle = {
            let guard = self.connections.read().await;
            guard
                .get(connection_id)
                .ok_or_else(|| anyhow!("Connection {} not found", connection_id))?
                .handle
                .clone()
        };

        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| anyhow!("Failed to open SSH PTY exec channel: {}", e))?;
        channel
            .request_pty(false, "xterm-256color", cols, rows, 0, 0, &[])
            .await
            .map_err(|e| anyhow!("Failed to request PTY for remote command: {}", e))?;
        channel
            .exec(true, command)
            .await
            .map_err(|e| anyhow!("Failed to start remote PTY command: {}", e))?;
        Ok(channel)
    }

    /// Get server info for a connection
    pub async fn get_server_info(&self, connection_id: &str) -> Option<ServerInfo> {
        let guard = self.connections.read().await;
        guard.get(connection_id).and_then(|c| c.server_info.clone())
    }

    /// If `home_dir` is missing, run [`Self::probe_remote_home_dir`] and persist it on the connection.
    pub async fn resolve_remote_home_if_missing(&self, connection_id: &str) -> Option<ServerInfo> {
        let need_probe = {
            let guard = self.connections.read().await;
            match guard.get(connection_id) {
                None => return None,
                Some(conn) => conn
                    .server_info
                    .as_ref()
                    .map(|s| s.home_dir.trim().is_empty())
                    .unwrap_or(true),
            }
        };
        if !need_probe {
            return self.get_server_info(connection_id).await;
        }
        let handle = {
            let guard = self.connections.read().await;
            guard.get(connection_id)?.handle.clone()
        };
        let Some(home) = Self::probe_remote_home_dir(&handle).await else {
            return self.get_server_info(connection_id).await;
        };
        {
            let mut guard = self.connections.write().await;
            if let Some(conn) = guard.get_mut(connection_id) {
                match conn.server_info.as_mut() {
                    Some(si) => si.home_dir = home.clone(),
                    None => {
                        conn.server_info = Some(ServerInfo {
                            os_type: "unknown".to_string(),
                            hostname: "unknown".to_string(),
                            home_dir: home,
                        });
                    }
                }
            }
        }
        self.get_server_info(connection_id).await
    }

    /// Get connection configuration
    pub async fn get_connection_config(&self, connection_id: &str) -> Option<SSHConnectionConfig> {
        let guard = self.connections.read().await;
        guard.get(connection_id).map(|c| c.config.clone())
    }

    /// Open a PTY session and start a shell
    pub async fn open_pty(&self, connection_id: &str, cols: u32, rows: u32) -> anyhow::Result<PTYSession> {
        let guard = self.connections.read().await;
        let conn = guard
            .get(connection_id)
            .ok_or_else(|| anyhow!("Connection {} not found", connection_id))?;

        // Open a session channel
        let channel = conn
            .handle
            .channel_open_session()
            .await
            .map_err(|e| anyhow!("Failed to open channel: {}", e))?;

        // Request PTY — `false` = don't wait for reply (reply handled in reader loop)
        channel
            .request_pty(false, "xterm-256color", cols, rows, 0, 0, &[])
            .await
            .map_err(|e| anyhow!("Failed to request PTY: {}", e))?;

        // Start shell — `false` = don't wait for reply
        channel
            .request_shell(false)
            .await
            .map_err(|e| anyhow!("Failed to start shell: {}", e))?;

        Ok(PTYSession::new(channel, connection_id.to_string()))
    }

    /// Get server key fingerprint for verification
    pub async fn get_server_key_fingerprint(&self, connection_id: &str) -> anyhow::Result<String> {
        let guard = self.connections.read().await;
        let conn = guard
            .get(connection_id)
            .ok_or_else(|| anyhow!("Connection {} not found", connection_id))?;

        // Return a fingerprint based on connection info
        // Note: Actual server key fingerprint requires access to the SSH transport layer
        // For security verification, the server key is verified during connection via SSHHandler
        let fingerprint = format!("{}:{}:{}", conn.config.host, conn.config.port, conn.config.username);
        Ok(fingerprint)
    }
}
