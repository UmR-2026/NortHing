//! MCP server configuration types.

use crate::util::errors::NortHingError;

pub use northhing_services_integrations::mcp::server::{
    MCPServerConfig, MCPServerConfigValidationError, MCPServerOAuthConfig, MCPServerTransport, MCPServerXaaConfig,
};

impl From<MCPServerConfigValidationError> for NortHingError {
    fn from(error: MCPServerConfigValidationError) -> Self {
        Self::Configuration(error.to_string())
    }
}
