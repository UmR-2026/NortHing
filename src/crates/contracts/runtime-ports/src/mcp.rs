//! MCP (Model Context Protocol) catalog port.
//!
//! Phase F.3 (2026-06-19): previously the desktop Inspector showed
//! `"MCP: not configured"` (see C.5). This port defines the boundary
//! shape so a desktop-side implementation can surface the live MCP
//! catalog. The producer side lives in
//! `northhing-core::service::mcp::MCPService`; the consumer side is
//! the desktop Inspector.
//!
//! ## Status
//!
//! **Port only.** No desktop implementation exists yet (it lands in
//! G.2). The producer-side types are not re-exported through this
//! crate; we declare a minimal DTO so the boundary stays one-way.
//!
//! ## Reference
//!
//! Pattern source: `src/apps/cli/src/management.rs::print_mcp_servers`
//! (the CLI side that already reads the catalog). Phase F.3 is the
//! first move toward pulling that read path into the desktop shell.

use serde::{Deserialize, Serialize};

/// Runtime status of an MCP server. Matches the producer-side
/// `MCPServerStatus` enum shape (in
/// `services-integrations::mcp::protocol`); we declare a minimal
/// subset here so the boundary stays free of cross-crate imports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum McpServerStatusDto {
    /// The server is configured and connected; transport is live.
    Connected,
    /// The server is configured and enabled, but the runtime has not
    /// yet established a connection (initialization in progress).
    Starting,
    /// The server is configured but the user disabled it.
    Disabled,
    /// The runtime tried to connect but failed; the message holds the
    /// last error for surfacing in the Inspector.
    Failed { message: String },
    /// The status probe timed out (matches the 30ms probe in the CLI).
    /// Treated as "starting" for display purposes.
    ProbeTimeout,
}

/// One MCP server entry as returned by the catalog port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerDto {
    /// Stable id (matches the config-side `id`).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Whether the user enabled this server in the config.
    pub enabled: bool,
    /// Live runtime status. When the catalog can't probe (no runtime
    /// attached), this falls back to `Disabled` for `enabled = false`
    /// servers and `Starting` for `enabled = true` servers.
    pub status: McpServerStatusDto,
}

/// Reader port for the MCP catalog.
///
/// **Name choice**: there is also a marker `McpCatalogPort` trait in
/// this crate (declared in `lib.rs`) that extends `RuntimeServicePort`
/// and is consumed by `runtime-services`'s builder. To avoid the
/// marker-vs-async collision, the rich async reader is named
/// `McpCatalogReader`. Adapters that need to register with both the
/// runtime-services builder AND the Inspector should impl both
/// `McpCatalogPort` (marker, capability()) and `McpCatalogReader`
/// (rich, list_servers()).
///
/// Implementations must be `Send + Sync` and non-blocking on the call
/// path (callers run from the UI thread).
#[async_trait::async_trait]
pub trait McpCatalogReader: Send + Sync {
    /// Return the current MCP server catalog. Empty list = nothing
    /// configured. Errors should be surfaced as `Err` rather than
    /// silently returning empty — the Inspector shows the message.
    async fn list_servers(&self) -> Result<Vec<McpServerDto>, McpCatalogError>;
}

/// Error returned by `McpCatalogReader::list_servers`. The Inspector
/// renders the message verbatim, so it should be human-readable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpCatalogError {
    pub message: String,
}

impl std::fmt::Display for McpCatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mcp catalog error: {}", self.message)
    }
}

impl std::error::Error for McpCatalogError {}

impl McpCatalogError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Build the Inspector status string from a list of servers. This is
/// the consumer-side rendering helper that matches the
/// `set_mcp_status` Slint property contract.
///
/// Format:
/// - 0 servers, no error → `"MCP: not configured"`
/// - 0 servers, error → `"MCP: <error.message>"`
/// - N>0 servers → `"MCP: <connected>/<total> connected"`
///   (or `"MCP: <connected>/<total> connected, <disabled> disabled"` when
///   any are disabled).
pub fn format_mcp_status(servers: &[McpServerDto]) -> String {
    if servers.is_empty() {
        return "MCP: not configured".to_string();
    }
    let total = servers.len();
    let connected = servers
        .iter()
        .filter(|s| matches!(s.status, McpServerStatusDto::Connected))
        .count();
    let disabled = servers.iter().filter(|s| !s.enabled).count();
    if disabled == 0 {
        format!("MCP: {connected}/{total} connected")
    } else {
        format!("MCP: {connected}/{total} connected, {disabled} disabled")
    }
}

/// Render an error path: when `list_servers` returns `Err`, the
/// Inspector surfaces the message rather than the default "not
/// configured" string.
pub fn format_mcp_status_err(err: &McpCatalogError) -> String {
    format!("MCP: {}", err.message)
}

#[cfg(test)]
mod tests {
    use super::{
        format_mcp_status, format_mcp_status_err, McpCatalogError, McpCatalogReader, McpServerDto, McpServerStatusDto,
    };

    fn sample(id: &str, enabled: bool, status: McpServerStatusDto) -> McpServerDto {
        McpServerDto {
            id: id.into(),
            name: id.into(),
            enabled,
            status,
        }
    }

    #[test]
    fn empty_catalog_renders_not_configured() {
        let s = format_mcp_status(&[]);
        assert_eq!(s, "MCP: not configured");
    }

    #[test]
    fn all_connected_renders_count() {
        let servers = vec![
            sample("a", true, McpServerStatusDto::Connected),
            sample("b", true, McpServerStatusDto::Connected),
        ];
        assert_eq!(format_mcp_status(&servers), "MCP: 2/2 connected");
    }

    #[test]
    fn mixed_with_disabled_renders_both() {
        let servers = vec![
            sample("a", true, McpServerStatusDto::Connected),
            sample("b", false, McpServerStatusDto::Disabled),
            sample("c", true, McpServerStatusDto::Starting),
        ];
        assert_eq!(format_mcp_status(&servers), "MCP: 1/3 connected, 1 disabled");
    }

    #[test]
    fn error_path_renders_message() {
        let err = McpCatalogError::new("connection refused");
        assert_eq!(format_mcp_status_err(&err), "MCP: connection refused");
    }

    #[test]
    fn dto_round_trips_through_camel_case_json() {
        let s = sample("a", true, McpServerStatusDto::Connected);
        let json = serde_json::to_value(&s).expect("serialize");
        assert_eq!(json["id"], "a");
        assert_eq!(json["enabled"], true);
        assert_eq!(json["status"]["kind"], "connected");

        let failed = McpServerDto {
            id: "x".into(),
            name: "x".into(),
            enabled: true,
            status: McpServerStatusDto::Failed { message: "boom".into() },
        };
        let json = serde_json::to_value(&failed).expect("serialize failed");
        assert_eq!(json["status"]["kind"], "failed");
        assert_eq!(json["status"]["message"], "boom");
    }

    #[tokio::test]
    async fn port_trait_is_implementable() {
        struct Stub;
        #[async_trait::async_trait]
        impl McpCatalogReader for Stub {
            async fn list_servers(&self) -> Result<Vec<McpServerDto>, McpCatalogError> {
                Ok(vec![sample("a", true, McpServerStatusDto::Starting)])
            }
        }
        let port: std::sync::Arc<dyn McpCatalogReader> = std::sync::Arc::new(Stub);
        let out = port.list_servers().await.expect("list ok");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "a");
    }
}
