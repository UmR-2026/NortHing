// R19 split: ACP client adapter predownload + CLI install entry points.
// File: src/crates/interfaces/acp/src/client/manager_install.rs
// Origin: manager.rs (2519 lines god-object, Kimi P1 critical)
// Sibling files:
//             manager_config.rs
//             manager_connection.rs
//             manager_transport.rs
//             manager_session.rs
//             manager_prompt.rs
//             manager_cancel.rs
//             manager_permission.rs
//             manager_process.rs
//             manager_process_lifecycle.rs
//             manager_session_helpers.rs
//             manager_errors.rs
//
// All method bodies are moved verbatim from main. No behavior change.

use super::manager_process::resolve_config_for_client;
use super::requirements::{
    acp_requirement_spec, apply_command_environment, install_npm_cli_package, install_remote_npm_cli_package,
    predownload_npm_adapter, probe_executable, probe_npm_adapter, probe_remote_executable, probe_remote_npx_adapter,
    resolve_configured_command,
};
use super::AcpClientService;
use northhing_core::service::remote_ssh::workspace_state::remote_workspace_manager;
use northhing_core::util::errors::{NortHingError, NortHingResult};
use std::sync::Arc;

impl AcpClientService {
    pub async fn predownload_client_adapter(self: &Arc<Self>, client_id: &str) -> NortHingResult<()> {
        let configs = self.load_configs().await?;
        let spec = acp_requirement_spec(client_id, configs.get(client_id));
        let adapter = spec.adapter.ok_or_else(|| {
            NortHingError::config(format!(
                "ACP client '{}' does not use a downloadable adapter",
                client_id
            ))
        })?;

        predownload_npm_adapter(adapter.package, adapter.bin).await
    }

    pub async fn install_client_cli(
        self: &Arc<Self>,
        client_id: &str,
        remote_connection_id: Option<&str>,
    ) -> NortHingResult<()> {
        let remote_connection_id = remote_connection_id.map(str::trim).filter(|value| !value.is_empty());
        let config_file = self.load_config_file().await?;
        let config = resolve_config_for_client(&config_file, client_id, remote_connection_id);
        let spec = acp_requirement_spec(client_id, config.as_ref());
        let package = spec.install_package.ok_or_else(|| {
            NortHingError::config(format!(
                "ACP client '{}' does not have a known CLI installer",
                client_id
            ))
        })?;

        if let Some(remote_connection_id) = remote_connection_id {
            let remote_manager = remote_workspace_manager()
                .ok_or_else(|| NortHingError::service("Remote workspace manager is not initialized".to_string()))?;
            let ssh_manager = remote_manager
                .get_ssh_manager()
                .await
                .ok_or_else(|| NortHingError::service("SSH manager is not available for remote ACP".to_string()))?;
            install_remote_npm_cli_package(&ssh_manager, remote_connection_id, package).await
        } else {
            install_npm_cli_package(package).await
        }
    }
}
