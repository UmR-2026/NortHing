//! SFTP read/write/mkdir/rename operations.
//!
//! Owns the cached `SftpSession` per active connection plus the public entry
//! points that the remote-folder picker calls into. The cached session is
//! transparently refreshed on transient failures (see `sftp_read_dir`'s
//! retry-once path) so a network blip does not permanently break the picker.
//!
//! Split from `manager.rs` in Round 13b.

use crate::remote_ssh::manager::SSHConnectionManager;
use crate::remote_ssh::manager_handler::SSHHandler;
use anyhow::anyhow;
use russh::client::Handle;
use russh_sftp::client::fs::ReadDir;
use russh_sftp::client::SftpSession;
use std::sync::atomic::Ordering;
use std::sync::Arc;

impl SSHConnectionManager {
    /// Expand leading `~` using the remote user's home from [`ServerInfo`] (SFTP paths are not shell-expanded).
    pub async fn resolve_sftp_path(&self, connection_id: &str, path: &str) -> anyhow::Result<String> {
        let path = path.trim();
        if path.is_empty() {
            return Err(anyhow!("Empty remote path"));
        }
        if path == "~" || path.starts_with("~/") {
            let guard = self.connections.read().await;
            let home = guard
                .get(connection_id)
                .and_then(|c| c.server_info.as_ref())
                .map(|s| s.home_dir.trim())
                .filter(|h| !h.is_empty());
            let home = match home {
                Some(h) => h.to_string(),
                None => {
                    return Err(anyhow!(
                        "Cannot use '~' in remote path: home directory is not available for this connection"
                    ));
                }
            };
            if path == "~" || path == "~/" {
                return Ok(home);
            }
            let rest = path[2..].trim_start_matches('/');
            if rest.is_empty() {
                return Ok(home);
            }
            Ok(format!("{}/{}", home.trim_end_matches('/'), rest))
        } else {
            Ok(path.to_string())
        }
    }

    /// Get or create SFTP session for a connection.
    ///
    /// Detects dead transports up-front via [`Self::ensure_alive_or_reconnect`]
    /// so a transient SSH disconnect (e.g. NAT timeout while the user is idly
    /// browsing the remote folder picker) is recovered transparently instead
    /// of cascading into a stale cached SFTP handle that fails forever.
    pub async fn get_sftp(&self, connection_id: &str) -> anyhow::Result<Arc<SftpSession>> {
        self.ensure_alive_or_reconnect(connection_id).await?;

        // First check if we have an existing SFTP session
        {
            let guard = self.connections.read().await;
            if let Some(conn) = guard.get(connection_id) {
                let sftp_guard = conn.sftp_session.read().await;
                if let Some(ref sftp) = *sftp_guard {
                    return Ok(sftp.clone());
                }
            }
        }

        // Get handle (clone the Arc)
        let handle: Arc<Handle<SSHHandler>> = {
            let guard = self.connections.read().await;
            let conn = guard
                .get(connection_id)
                .ok_or_else(|| anyhow!("Connection {} not found", connection_id))?;
            conn.handle.clone()
        };

        // Open a channel and request SFTP subsystem
        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| anyhow!("Failed to open channel for SFTP: {}", e))?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| anyhow!("Failed to request SFTP subsystem: {}", e))?;

        let sftp = SftpSession::new(channel.into_stream())
            .await
            .map_err(|e| anyhow!("Failed to create SFTP session: {}", e))?;

        let sftp = Arc::new(sftp);

        // Store the SFTP session
        {
            let mut guard = self.connections.write().await;
            if let Some(conn) = guard.get_mut(connection_id) {
                let mut sftp_guard = conn.sftp_session.write().await;
                *sftp_guard = Some(sftp.clone());
            }
        }

        Ok(sftp)
    }

    /// Read a file via SFTP
    pub async fn sftp_read(&self, connection_id: &str, path: &str) -> anyhow::Result<Vec<u8>> {
        let path = self.resolve_sftp_path(connection_id, path).await?;
        let sftp = self.get_sftp(connection_id).await?;
        let mut file = sftp
            .open(&path)
            .await
            .map_err(|e| anyhow!("Failed to open remote file '{}': {}", path, e))?;

        let mut buffer = Vec::new();
        use tokio::io::AsyncReadExt;
        file.read_to_end(&mut buffer)
            .await
            .map_err(|e| anyhow!("Failed to read remote file '{}': {}", path, e))?;

        Ok(buffer)
    }

    /// Write a file via SFTP
    pub async fn sftp_write(&self, connection_id: &str, path: &str, content: &[u8]) -> anyhow::Result<()> {
        let path = self.resolve_sftp_path(connection_id, path).await?;
        let sftp = self.get_sftp(connection_id).await?;
        let mut file = sftp
            .create(&path)
            .await
            .map_err(|e| anyhow!("Failed to create remote file '{}': {}", path, e))?;

        use tokio::io::AsyncWriteExt;
        file.write_all(content)
            .await
            .map_err(|e| anyhow!("Failed to write remote file '{}': {}", path, e))?;

        file.flush()
            .await
            .map_err(|e| anyhow!("Failed to flush remote file '{}': {}", path, e))?;

        Ok(())
    }

    /// Read directory via SFTP.
    ///
    /// Retries once after dropping the cached SFTP session and forcing a
    /// reconnect attempt, so a stale SFTP channel left over from a prior
    /// network blip does not permanently break the remote folder picker.
    pub async fn sftp_read_dir(&self, connection_id: &str, path: &str) -> anyhow::Result<ReadDir> {
        let resolved = self.resolve_sftp_path(connection_id, path).await?;
        let sftp = self.get_sftp(connection_id).await?;
        match sftp.read_dir(&resolved).await {
            Ok(entries) => Ok(entries),
            Err(first_err) => {
                tracing::warn!(
                    "SFTP read_dir '{}' failed (will retry once after refreshing session): {}",
                    resolved,
                    first_err
                );
                self.invalidate_sftp_session(connection_id).await;
                // Force the alive flag to false so ensure_alive_or_reconnect rebuilds
                // the underlying SSH transport too — the previous failure may indicate
                // the channel was torn down even though the keepalive callback has not
                // fired yet.
                self.mark_dead(connection_id).await;
                let sftp = self.get_sftp(connection_id).await?;
                sftp.read_dir(&resolved)
                    .await
                    .map_err(|e| anyhow!("Failed to read directory '{}': {}", resolved, e))
            }
        }
    }

    /// Drop the cached SFTP session for a connection so the next call opens a
    /// fresh channel. Safe to call when no session is cached.
    pub(super) async fn invalidate_sftp_session(&self, connection_id: &str) {
        let guard = self.connections.read().await;
        if let Some(conn) = guard.get(connection_id) {
            let mut sftp_guard = conn.sftp_session.write().await;
            *sftp_guard = None;
        }
    }

    /// Force the liveness flag to false. Triggers a transparent reconnect on
    /// the next call to [`Self::ensure_alive_or_reconnect`].
    pub(super) async fn mark_dead(&self, connection_id: &str) {
        let guard = self.connections.read().await;
        if let Some(conn) = guard.get(connection_id) {
            conn.alive.store(false, Ordering::SeqCst);
        }
    }

    /// Create directory via SFTP
    pub async fn sftp_mkdir(&self, connection_id: &str, path: &str) -> anyhow::Result<()> {
        let path = self.resolve_sftp_path(connection_id, path).await?;
        let sftp = self.get_sftp(connection_id).await?;
        sftp.create_dir(&path)
            .await
            .map_err(|e| anyhow!("Failed to create directory '{}': {}", path, e))?;
        Ok(())
    }

    /// Create directory and all parents via SFTP
    pub async fn sftp_mkdir_all(&self, connection_id: &str, path: &str) -> anyhow::Result<()> {
        let path = self.resolve_sftp_path(connection_id, path).await?;
        let sftp = self.get_sftp(connection_id).await?;

        // Check if path exists
        match sftp.as_ref().try_exists(&path).await {
            Ok(true) => return Ok(()), // Already exists
            Ok(false) => {}
            Err(_) => {}
        }

        for dir in sftp_mkdir_all_prefixes(&path) {
            if let Ok(true) = sftp.as_ref().try_exists(&dir).await {
                continue;
            }

            if let Err(error) = sftp.as_ref().create_dir(&dir).await {
                match sftp.as_ref().try_exists(&dir).await {
                    Ok(true) => continue,
                    Ok(false) | Err(_) => {
                        return Err(anyhow!("Failed to create directory '{}': {}", dir, error));
                    }
                }
            }
        }

        Ok(())
    }

    /// Remove file via SFTP
    pub async fn sftp_remove(&self, connection_id: &str, path: &str) -> anyhow::Result<()> {
        let path = self.resolve_sftp_path(connection_id, path).await?;
        let sftp = self.get_sftp(connection_id).await?;
        sftp.remove_file(&path)
            .await
            .map_err(|e| anyhow!("Failed to remove file '{}': {}", path, e))?;
        Ok(())
    }

    /// Remove directory via SFTP
    pub async fn sftp_rmdir(&self, connection_id: &str, path: &str) -> anyhow::Result<()> {
        let path = self.resolve_sftp_path(connection_id, path).await?;
        let sftp = self.get_sftp(connection_id).await?;
        sftp.remove_dir(&path)
            .await
            .map_err(|e| anyhow!("Failed to remove directory '{}': {}", path, e))?;
        Ok(())
    }

    /// Rename/move via SFTP
    pub async fn sftp_rename(&self, connection_id: &str, old_path: &str, new_path: &str) -> anyhow::Result<()> {
        let old_path = self.resolve_sftp_path(connection_id, old_path).await?;
        let new_path = self.resolve_sftp_path(connection_id, new_path).await?;
        let sftp = self.get_sftp(connection_id).await?;
        sftp.rename(&old_path, &new_path)
            .await
            .map_err(|e| anyhow!("Failed to rename '{}' to '{}': {}", old_path, new_path, e))?;
        Ok(())
    }

    /// Check if path exists via SFTP
    pub async fn sftp_exists(&self, connection_id: &str, path: &str) -> anyhow::Result<bool> {
        let path = self.resolve_sftp_path(connection_id, path).await?;
        let sftp = self.get_sftp(connection_id).await?;
        sftp.as_ref()
            .try_exists(&path)
            .await
            .map_err(|e| anyhow!("Failed to check if '{}' exists: {}", path, e))
    }

    /// Get file metadata via SFTP
    pub async fn sftp_stat(&self, connection_id: &str, path: &str) -> anyhow::Result<russh_sftp::client::fs::Metadata> {
        let path = self.resolve_sftp_path(connection_id, path).await?;
        let sftp = self.get_sftp(connection_id).await?;
        sftp.as_ref()
            .metadata(&path)
            .await
            .map_err(|e| anyhow!("Failed to stat '{}': {}", path, e))
    }
}

/// Expand `path` into the list of parent directories that need to be created
/// for `mkdir_all`. Splits on `/`, drops empty components (collapsing
/// redundant separators), and preserves leading `/` for absolute POSIX paths.
pub(super) fn sftp_mkdir_all_prefixes(path: &str) -> Vec<String> {
    let is_absolute = path.starts_with('/');
    let mut current = String::new();
    let mut prefixes = Vec::new();

    for component in path.split('/').filter(|component| !component.is_empty()) {
        if current.is_empty() {
            if is_absolute {
                current.push('/');
            }
            current.push_str(component);
        } else {
            current.push('/');
            current.push_str(component);
        }
        prefixes.push(current.clone());
    }

    prefixes
}
