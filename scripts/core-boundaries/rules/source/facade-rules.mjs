// Boundary rules for source ownership, facades, and required owner content.

export const facadeOnlyFiles = [
  {
    path: 'src/crates/assembly/core/src/infrastructure/filesystem/mod.rs',
    importPrefix: 'northhing_services_core::filesystem',
    reason: 'core filesystem infrastructure facade must only re-export the services-core owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/filesystem/listing.rs',
    importPrefix: 'northhing_services_core::filesystem',
    reason: 'core filesystem listing facade must only re-export the services-core owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/filesystem/types.rs',
    importPrefix: 'northhing_services_core::filesystem',
    reason: 'core filesystem DTO facade must only re-export the services-core owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/git/git_service.rs',
    importPrefix: 'northhing_services_integrations::git',
    reason: 'core git service facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/git/git_types.rs',
    importPrefix: 'northhing_services_integrations::git',
    reason: 'core git types facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/git/git_utils.rs',
    importPrefix: 'northhing_services_integrations::git',
    reason: 'core git utils facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/git/graph.rs',
    importPrefix: 'northhing_services_integrations::git',
    reason: 'core git graph facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/remote_ssh/types.rs',
    importPrefix: 'northhing_services_integrations::remote_ssh',
    reason: 'core remote SSH types facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/remote_ssh/manager.rs',
    importPrefix: 'northhing_services_integrations::remote_ssh::manager',
    reason: 'core remote SSH manager facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/remote_ssh/remote_fs.rs',
    importPrefix: 'northhing_services_integrations::remote_ssh::remote_fs',
    reason: 'core remote SSH filesystem facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/remote_ssh/remote_terminal.rs',
    importPrefix: 'northhing_services_integrations::remote_ssh::remote_terminal',
    reason: 'core remote SSH terminal facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/mcp/tool_info.rs',
    importPrefix: 'northhing_services_integrations::mcp',
    reason: 'core MCP tool info facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/mcp/tool_name.rs',
    importPrefix: 'northhing_services_integrations::mcp',
    reason: 'core MCP tool name facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/mcp/protocol/types.rs',
    importPrefix: 'northhing_services_integrations::mcp',
    reason: 'core MCP protocol types facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/mcp/protocol/transport.rs',
    importPrefix: 'northhing_services_integrations::mcp::protocol',
    reason: 'core MCP stdio transport facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/mcp/protocol/transport_remote.rs',
    importPrefix: 'northhing_services_integrations::mcp::protocol',
    reason: 'core MCP remote transport facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/mcp/server/connection.rs',
    importPrefix: 'northhing_services_integrations::mcp::server',
    reason: 'core MCP connection facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/mcp/config/location.rs',
    importPrefix: 'northhing_services_integrations::mcp',
    reason: 'core MCP config location facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/mcp/adapter/resource.rs',
    importPrefix: 'northhing_services_integrations::mcp',
    reason: 'core MCP resource adapter facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/mcp/adapter/prompt.rs',
    importPrefix: 'northhing_services_integrations::mcp',
    reason: 'core MCP prompt adapter facade must only re-export the integrations owner crate',
  },
  {
    path: 'src/crates/assembly/core/src/service/announcement/types.rs',
    importPrefix: 'northhing_services_integrations::announcement',
    reason: 'core announcement types facade must only re-export the integrations owner crate',
  },
];
