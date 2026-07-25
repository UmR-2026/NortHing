//! inspector module — see mod.rs for the wiring entry point.

/// Phase G.2: build the Inspector `mcp-status` string from the live
/// kernel facade. Falls back to the existing `"MCP: not configured"`
/// placeholder on any failure (facade not initialized, list error).
///
/// K4a-T4: uses facade `list_mcp_servers` + concurrent per-id
/// `get_mcp_status` (N+1 pattern; MCP count is small, latency is
/// same-order as the old single-call path).
pub(super) async fn build_mcp_status_string() -> String {
    use crate::mcp_adapter::{render_status, McpCatalogAdapter};
    use northhing_core::kernel_facade::kernel_facade;
    use northhing_runtime_ports::McpCatalogReader;

    let facade = kernel_facade();
    let adapter = McpCatalogAdapter::new(facade);
    let result = adapter.list_servers().await;
    render_status(&result)
}
