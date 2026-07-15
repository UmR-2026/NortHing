use crate::service::mcp::server::MCPServerConfig;
use crate::util::errors::NortHingResult;

pub(super) fn config_to_cursor_format(config: &MCPServerConfig) -> serde_json::Value {
    northhing_services_integrations::mcp::config::config_to_cursor_format(config)
}

pub(super) fn parse_cursor_format(
    config: &serde_json::Value,
) -> NortHingResult<Vec<MCPServerConfig>> {
    Ok(northhing_services_integrations::mcp::config::parse_cursor_format(config))
}
