//! Port forwarding manager and entry types (split out from `manager.rs`).
//!
//! Split out from `manager.rs` in Round 13 (facade + 3 sub-handlers pattern).
//! `PortForwardManager` holds an optional `SSHConnectionManager` (the facade)
//! so it can be wired into the active SSH session once the underlying
//! forwarding implementation lands.

use crate::remote_ssh::manager::SSHConnectionManager;
use std::collections::HashMap;
use std::sync::Arc;

/// Port forwarding entry
#[derive(Debug, Clone)]
pub struct PortForward {
    pub id: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    pub direction: PortForwardDirection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PortForwardDirection {
    Local,   // -L: forward local port to remote
    Remote,  // -R: forward remote port to local
    Dynamic, // -D: dynamic SOCKS proxy
}

/// Port forwarding manager
pub struct PortForwardManager {
    forwards: Arc<tokio::sync::RwLock<HashMap<String, PortForward>>>,
    ssh_manager: Arc<tokio::sync::RwLock<Option<SSHConnectionManager>>>,
}

impl PortForwardManager {
    pub fn new() -> Self {
        Self {
            forwards: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            ssh_manager: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    pub fn with_ssh_manager(ssh_manager: SSHConnectionManager) -> Self {
        Self {
            forwards: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            ssh_manager: Arc::new(tokio::sync::RwLock::new(Some(ssh_manager))),
        }
    }

    pub async fn set_ssh_manager(&self, manager: SSHConnectionManager) {
        let mut guard = self.ssh_manager.write().await;
        *guard = Some(manager);
    }

    /// Start local port forwarding (-L)
    ///
    /// Current status: **registration-only stub**.
    /// The forward entry is persisted in `self.forwards`, the log records the
    /// intent, but no TCP listener is opened and no SSH channel is established.
    ///
    /// Full implementation requires (tracked separately):
    /// - A Tokio TCP listener bound to `localhost:local_port`
    /// - An SSH channel per forwarded connection (via `russh`)
    /// - Proper teardown in `stop_forward`
    pub async fn start_local_forward(
        &self,
        _connection_id: &str,
        local_port: u16,
        remote_host: String,
        remote_port: u16,
    ) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();

        let forward = PortForward {
            id: id.clone(),
            local_port,
            remote_host: remote_host.clone(),
            remote_port,
            direction: PortForwardDirection::Local,
        };

        // Store forward entry
        let mut guard = self.forwards.write().await;
        guard.insert(id.clone(), forward);

        tracing::warn!(
            "Local port forward registration stored (connection not active): localhost:{} -> {}:{}",
            local_port,
            remote_host,
            remote_port
        );
        tracing::warn!("Port forwarding is not fully implemented - connections will not be forwarded");

        Ok(id)
    }

    /// Start remote port forwarding (-R)
    ///
    /// Current status: **registration-only stub**.
    /// The forward entry is persisted in `self.forwards`, but no SSH
    /// reverse channel is opened and no remote port is bound.
    ///
    /// Full implementation requires:
    /// - An SSH "reverse" channel (`russh::Channel::open_reverse`)
    /// - Binding to the remote port (or requesting the server allocate one)
    /// - Forwarding incoming remote connections back through the channel
    pub async fn start_remote_forward(
        &self,
        _connection_id: &str,
        remote_port: u16,
        local_host: String,
        local_port: u16,
    ) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();

        let forward = PortForward {
            id: id.clone(),
            local_port: remote_port,
            remote_host: local_host.clone(),
            remote_port: local_port,
            direction: PortForwardDirection::Remote,
        };

        // Remote port forwarding: registration-only stub.
        // The entry is persisted so `stop_forward` can clean it up when
        // the feature is eventually activated.

        let mut guard = self.forwards.write().await;
        guard.insert(id.clone(), forward);

        tracing::warn!(
            "Remote port forward registration stored (connection not active): *:{} -> {}:{}",
            remote_port,
            local_host,
            local_port
        );
        tracing::warn!("Remote port forwarding is not fully implemented - data will not be forwarded");

        Ok(id)
    }

    /// Stop a port forward
    pub async fn stop_forward(&self, forward_id: &str) -> anyhow::Result<()> {
        let mut guard = self.forwards.write().await;
        if let Some(forward) = guard.remove(forward_id) {
            tracing::info!(
                "Stopped port forward: {} ({}:{} -> {}:{})",
                forward.id,
                match forward.direction {
                    PortForwardDirection::Local => "local",
                    PortForwardDirection::Remote => "remote",
                    PortForwardDirection::Dynamic => "dynamic",
                },
                forward.local_port,
                forward.remote_host,
                forward.remote_port
            );
        }
        Ok(())
    }

    /// Stop all port forwards
    pub async fn stop_all(&self) {
        let mut guard = self.forwards.write().await;
        let count = guard.len();
        guard.drain();
        tracing::info!("All {} port forwards stopped", count);
    }

    /// List all active forwards
    pub async fn list_forwards(&self) -> Vec<PortForward> {
        let guard = self.forwards.read().await;
        guard.values().cloned().collect()
    }

    /// Check if a port is already forwarded
    pub async fn is_port_forwarded(&self, port: u16) -> bool {
        let guard = self.forwards.read().await;
        guard.values().any(|f| f.local_port == port)
    }
}

impl Default for PortForwardManager {
    fn default() -> Self {
        Self::new()
    }
}
