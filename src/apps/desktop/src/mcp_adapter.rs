//! Adapter: kernel facade → `McpCatalogPort`.
//!
//! Phase F.3 (2026-06-19): bridges the kernel facade's MCP read surface
//! to the `northhing-runtime-ports::McpCatalogPort` consumer boundary.
//! The desktop Inspector (`create_ui`) consumes this adapter when
//! refreshing the `mcp_status` Slint property.
//!
//! ## Shape (K4a-T4)
//!
//! - Reads the server list via facade `list_mcp_servers`.
//! - Probes runtime status via concurrent per-id `get_mcp_status`
//!   (N+1 pattern; MCP count is small, latency is same-order).
//! - Maps the facade `MCPServerStatusKind` enum to the
//!   `McpServerStatusDto` declared in runtime-ports.
//!
//! The constructor accepts an `Arc<KernelFacade>` so callers can share
//! the facade handle with other desktop consumers.

use std::sync::Arc;

use northhing_core::kernel_facade::KernelFacade;
use northhing_kernel_api::settings::MCPServerStatusKind;
use northhing_runtime_ports::{
    format_mcp_status, format_mcp_status_err, McpCatalogError, McpCatalogReader, McpServerDto, McpServerStatusDto,
};

/// Adapter wrapping a kernel facade handle so the desktop can read the
/// MCP catalog through the runtime-ports boundary.
pub struct McpCatalogAdapter {
    facade: Arc<KernelFacade>,
}

impl std::fmt::Debug for McpCatalogAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpCatalogAdapter").finish_non_exhaustive()
    }
}

impl McpCatalogAdapter {
    /// Build an adapter over an existing facade handle. Caller retains
    /// ownership of `facade` (it's `Arc`-shared).
    pub fn new(facade: Arc<KernelFacade>) -> Self {
        Self { facade }
    }
}

/// Map facade `MCPServerStatusKind` to port DTO. The facade enum
/// already folds the 9 producer-side variants into 5; this is a 1:1
/// mapping that keeps the adapter as the single place where the
/// runtime-ports shape is constructed.
fn map_status(kind: &MCPServerStatusKind) -> McpServerStatusDto {
    match kind {
        MCPServerStatusKind::Connected => McpServerStatusDto::Connected,
        MCPServerStatusKind::Starting => McpServerStatusDto::Starting,
        MCPServerStatusKind::Disabled => McpServerStatusDto::Disabled,
        MCPServerStatusKind::Failed { message } => McpServerStatusDto::Failed {
            message: message.clone(),
        },
        MCPServerStatusKind::ProbeTimeout => McpServerStatusDto::ProbeTimeout,
    }
}

/// Resolve the catalog `enabled` flag from the facade config DTO
/// (K4a-T5 MINOR①). Reads the real config-level enabled instead of
/// reverse-inferring it from a runtime `Disabled` status; `None` keeps
/// the pre-existing default (enabled) for backward compatibility.
fn resolve_enabled(config: &northhing_kernel_api::settings::MCPServerDto) -> bool {
    config.enabled.unwrap_or(true)
}

#[async_trait::async_trait]
impl McpCatalogReader for McpCatalogAdapter {
    async fn list_servers(&self) -> Result<Vec<McpServerDto>, McpCatalogError> {
        use northhing_kernel_api::KernelSettingsApi;

        let configs = self
            .facade
            .list_mcp_servers()
            .await
            .map_err(|e| McpCatalogError::new(format!("list_mcp_servers: {e}")))?;

        let ids: Vec<String> = configs.iter().map(|c| c.id.clone()).collect();
        let statuses =
            futures::future::join_all(ids.iter().map(|id| self.facade.get_mcp_status(id)).collect::<Vec<_>>()).await;

        let mut servers = Vec::with_capacity(configs.len());
        for (config, status_result) in configs.iter().zip(statuses.into_iter()) {
            let status = match status_result {
                Ok(dto) => map_status(&dto.status),
                Err(_) => McpServerStatusDto::Failed {
                    message: "status probe failed".into(),
                },
            };
            let enabled = resolve_enabled(config);
            servers.push(McpServerDto {
                id: config.id.clone(),
                name: config.name.clone(),
                enabled,
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
    fn map_status_connected() {
        assert_eq!(
            map_status(&MCPServerStatusKind::Connected),
            McpServerStatusDto::Connected
        );
    }

    #[test]
    fn map_status_starting() {
        assert_eq!(map_status(&MCPServerStatusKind::Starting), McpServerStatusDto::Starting);
    }

    #[test]
    fn map_status_failed_carries_message() {
        let s = map_status(&MCPServerStatusKind::Failed {
            message: "needs authentication".into(),
        });
        match s {
            McpServerStatusDto::Failed { message } => {
                assert_eq!(message, "needs authentication");
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn map_status_disabled() {
        assert_eq!(map_status(&MCPServerStatusKind::Disabled), McpServerStatusDto::Disabled);
    }

    #[test]
    fn map_status_probe_timeout() {
        assert_eq!(
            map_status(&MCPServerStatusKind::ProbeTimeout),
            McpServerStatusDto::ProbeTimeout
        );
    }

    #[test]
    fn resolve_enabled_reads_config_field() {
        use northhing_kernel_api::settings::{ConfigLocationDto, MCPServerConfigDto};

        let mk = |enabled: Option<bool>| northhing_kernel_api::settings::MCPServerDto {
            id: "a".into(),
            name: "a".into(),
            config: MCPServerConfigDto {
                command: "cmd".into(),
                args: vec![],
                env: None,
            },
            location: ConfigLocationDto::User,
            enabled,
        };

        assert!(resolve_enabled(&mk(None)), "None defaults to enabled (backward compat)");
        assert!(resolve_enabled(&mk(Some(true))));
        assert!(!resolve_enabled(&mk(Some(false))), "config-level disabled is honored");
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
