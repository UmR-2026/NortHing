//! Adapter: `MCPService` → `McpCatalogPort`.
//!
//! Phase F.3 (2026-06-19): bridges the producer-side
//! `northhing-core::service::mcp::MCPService` to the
//! `northhing-runtime-ports::McpCatalogPort` consumer boundary. The
//! desktop Inspector (`create_ui`) consumes this adapter when refreshing
//! the `mcp_status` Slint property.
//!
//! ## Shape
//!
//! - Reads the user-side config via `mcp_service.config_service().load_all_configs()`.
//! - Probes runtime status via
//!   `mcp_service.server_manager().get_server_status(&id)` with a 30ms
//!   timeout — same as the CLI's `print_mcp_servers`.
//! - Maps the producer-side `MCPServerStatus` enum to the
//!   `McpServerStatusDto` declared in runtime-ports.
//!
//! The constructor accepts an `Arc<MCPService>` so callers can share
//! the service with other desktop consumers (Phase 3 may want the
//! `mcp.dynamic_tool.list` query too).

use std::sync::Arc;
use std::time::Duration;

use northhing_core::service::mcp::{MCPServerConfig, MCPServerStatus, MCPService};
use northhing_runtime_ports::{
    format_mcp_status, format_mcp_status_err, McpCatalogError, McpCatalogReader, McpServerDto, McpServerStatusDto,
};

// Note: the marker `McpCatalogPort` (extends `RuntimeServicePort`) is
// referenced via fully-qualified path in the impl below to avoid an
// unused-import warning.

/// Default probe timeout — matches the CLI's `print_mcp_servers`
/// behavior (`tokio::time::timeout(Duration::from_millis(30), ...)`).
const PROBE_TIMEOUT: Duration = Duration::from_millis(30);

/// Adapter wrapping an `MCPService` so the desktop can read the MCP
/// catalog through the runtime-ports boundary.
pub struct McpCatalogAdapter {
    service: Arc<MCPService>,
}

impl std::fmt::Debug for McpCatalogAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpCatalogAdapter").finish_non_exhaustive()
    }
}

impl McpCatalogAdapter {
    /// Build an adapter over an existing `MCPService`. Caller retains
    /// ownership of `service` (it's `Arc`-shared).
    pub fn new(service: Arc<MCPService>) -> Self {
        Self { service }
    }

    /// Convenience: probe a single server's status, mapping the
    /// producer-side enum onto the port DTO. The `enabled` flag
    /// wins: if the user disabled the server, status is `Disabled`
    /// regardless of runtime state.
    async fn probe_status(&self, config: &MCPServerConfig) -> McpServerStatusDto {
        if !config.enabled {
            return McpServerStatusDto::Disabled;
        }
        match tokio::time::timeout(
            PROBE_TIMEOUT,
            self.service.server_manager().get_server_status(&config.id),
        )
        .await
        {
            Ok(Ok(status)) => map_status(status),
            Ok(Err(_err)) => McpServerStatusDto::Failed {
                message: "status probe failed".into(),
            },
            Err(_elapsed) => McpServerStatusDto::ProbeTimeout,
        }
    }
}

/// Map producer-side `MCPServerStatus` to port DTO. The producer enum
/// has 9 variants; we fold `Uninitialized` / `Starting` / `Reconnecting`
/// into `Starting`, treat `Healthy` as `Connected`, surface `Failed`
/// with the message placeholder (the producer side doesn't carry a
/// message on the enum), and treat the rest as the closest consumer
/// match.
fn map_status(status: MCPServerStatus) -> McpServerStatusDto {
    match status {
        MCPServerStatus::Connected | MCPServerStatus::Healthy => McpServerStatusDto::Connected,
        MCPServerStatus::Starting | MCPServerStatus::Uninitialized | MCPServerStatus::Reconnecting => {
            McpServerStatusDto::Starting
        }
        MCPServerStatus::NeedsAuth => McpServerStatusDto::Failed {
            message: "needs authentication".into(),
        },
        MCPServerStatus::Failed => McpServerStatusDto::Failed {
            message: "runtime reported failure".into(),
        },
        MCPServerStatus::Stopping | MCPServerStatus::Stopped => McpServerStatusDto::Disabled,
    }
}

#[async_trait::async_trait]
impl McpCatalogReader for McpCatalogAdapter {
    async fn list_servers(&self) -> Result<Vec<McpServerDto>, McpCatalogError> {
        let configs = self
            .service
            .config_service()
            .load_all_configs()
            .await
            .map_err(|e| McpCatalogError::new(format!("load_all_configs: {e}")))?;

        let mut servers = Vec::with_capacity(configs.len());
        for config in &configs {
            let status = self.probe_status(config).await;
            servers.push(McpServerDto {
                id: config.id.clone(),
                name: config.name.clone(),
                enabled: config.enabled,
                status,
            });
        }
        Ok(servers)
    }
}

// Marker impl: `runtime-services::RuntimeServicesBuilder::with_optional_mcp_catalog`
// expects `Option<Arc<dyn McpCatalogPort>>` where `McpCatalogPort:
// RuntimeServicePort`. By implementing the marker here, the same
// `McpCatalogAdapter` can be registered with the runtime services
// builder AND consumed by the Inspector as a `McpCatalogReader`.
impl northhing_runtime_ports::RuntimeServicePort for McpCatalogAdapter {
    fn capability(&self) -> northhing_runtime_ports::RuntimeServiceCapability {
        northhing_runtime_ports::RuntimeServiceCapability::McpCatalog
    }
}

/// Compute the Inspector status string from a result returned by
/// [`McpCatalogReader::list_servers`]. The Inspector calls this from a
/// `set_mcp_status` Slint callback (Phase G.2).
pub fn render_status(result: &Result<Vec<McpServerDto>, McpCatalogError>) -> String {
    match result {
        Ok(servers) => format_mcp_status(servers),
        Err(err) => format_mcp_status_err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_status_folds_uninitialized_into_starting() {
        assert_eq!(map_status(MCPServerStatus::Uninitialized), McpServerStatusDto::Starting);
    }

    #[test]
    fn map_status_treats_healthy_as_connected() {
        assert_eq!(map_status(MCPServerStatus::Healthy), McpServerStatusDto::Connected);
    }

    #[test]
    fn map_status_failed_carries_message() {
        let s = map_status(MCPServerStatus::Failed);
        match s {
            McpServerStatusDto::Failed { message } => {
                assert_eq!(message, "runtime reported failure");
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn map_status_stopping_is_disabled() {
        assert_eq!(map_status(MCPServerStatus::Stopping), McpServerStatusDto::Disabled);
    }

    #[test]
    fn render_status_uses_format_helpers() {
        let ok = Ok(vec![McpServerDto {
            id: "a".into(),
            name: "a".into(),
            enabled: true,
            status: McpServerStatusDto::Connected,
        }]);
        assert_eq!(render_status(&ok), "MCP: 1/1 connected");

        let err = Err(McpCatalogError::new("offline"));
        assert_eq!(render_status(&err), "MCP: offline");
    }
}
