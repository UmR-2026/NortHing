//! inspector module — see mod.rs for the wiring entry point.

/// Phase G.2: build the Inspector `mcp-status` string from the live
/// `McpCatalogReader`. Falls back to the existing `"MCP: not configured"`
/// placeholder on any failure (config service missing, global MCPService
/// not registered, list_servers error).
///
/// P0-D (2026-06-25): prefer the global MCPService registered by
/// `initialize_core_services` instead of constructing a fresh service
/// every refresh (which would re-run `initialize_all` each call and
/// hammer the catalog).
///
/// The implementation mirrors the CLI's `print_mcp_servers` flow
/// (`src/apps/cli/src/management.rs:112`) but goes through the
/// runtime-ports boundary so the desktop-side read path doesn't
/// depend on the concrete `MCPService` shape.
pub(super) async fn build_mcp_status_string() -> String {
    use crate::mcp_adapter::{render_status, McpCatalogAdapter};
    use northhing_runtime_ports::McpCatalogReader;

    // P0-D: use the global MCPService registered during startup. Fall
    // back to constructing one only if the global isn't registered yet
    // (e.g. during very early startup before initialize_core_services
    // finishes).
    let mcp_service = if let Some(global) = northhing_core::service::mcp::global_mcp_service() {
        global
    } else {
        let config_service = match northhing_core::service::config::get_global_config_service().await {
            Ok(svc) => svc,
            Err(e) => {
                tracing::warn!("P0-D: failed to fetch global config service for MCP fallback: {e}");
                return "MCP: not configured".to_string();
            }
        };
        match northhing_core::service::mcp::MCPService::new(config_service) {
            Ok(svc) => std::sync::Arc::new(svc),
            Err(e) => {
                tracing::warn!("P0-D: failed to construct MCPService fallback: {e}");
                return "MCP: not configured".to_string();
            }
        }
    };

    let adapter = McpCatalogAdapter::new(mcp_service);
    let result = adapter.list_servers().await;
    render_status(&result)
}
