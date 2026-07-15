// R20c split: ACP client requirement probing + tool registration.
// File: src/crates/interfaces/acp/src/client/manager_config_requirements.rs
// Origin: manager_config.rs (292 canonical lines, QClaw R20a P2 D-deviation
//        +21% over QClaw 242 tolerance)
// Mavis fix: R20c sub-domain B (3 fns: requirement probing) + sub-domain D
//        (1 fn: tool registration). All 4 are pub fn / pub async fn
//        (sibling-inherent-dispatch consumers).
// R20c sibling: manager_config_loading.rs (sub-domain A + C)
// R20c sibling: manager_connection_start.rs + manager_connection_stop.rs
// R19 sibling files (consumers of B + D methods):
//             manager_session.rs (probe_remote_client_requirements for session init)
//             manager_install.rs (probe_client_requirements for install flow)
//             manager_prompt.rs (register_configured_tools for prompt-time tool setup)
//             ... (other siblings that call these via self.method())
//
// All method bodies are moved verbatim from main. No behavior change.

use super::builtin_clients::builtin_client_ids;
use super::config::{
    AcpClientConfig, AcpClientConfigFile, AcpClientRequirementProbe, RemoteAcpClientRequirementSnapshot,
};
use super::manager_process::{current_unix_timestamp_ms, resolve_config_for_client};
use super::requirements::{
    acp_requirement_spec, probe_executable, probe_npm_adapter, probe_remote_executable, probe_remote_npx_adapter,
};
use super::tool::AcpAgentTool;
use super::AcpClientService;
use northhing_core::agentic::tools::registry::global_tool_registry;
use northhing_core::service::remote_ssh::workspace_state::remote_workspace_manager;
use northhing_core::util::errors::{NortHingError, NortHingResult};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

impl AcpClientService {
    pub async fn probe_client_requirements(
        self: &Arc<Self>,
        remote_connection_id: Option<&str>,
        force_refresh: bool,
    ) -> NortHingResult<Vec<AcpClientRequirementProbe>> {
        if let Some(remote_connection_id) = remote_connection_id.map(str::trim).filter(|value| !value.is_empty()) {
            return if force_refresh {
                self.refresh_remote_client_requirements(remote_connection_id).await
            } else {
                Ok(self
                    .remote_capability_store
                    .get(remote_connection_id)
                    .await
                    .map(|snapshot| snapshot.probes)
                    .unwrap_or_default())
            };
        }

        let configs = self.load_configs().await?;
        let mut ids = configs.keys().cloned().collect::<Vec<_>>();
        for id in builtin_client_ids() {
            if !ids.iter().any(|candidate| candidate == id) {
                ids.push(id.to_string());
            }
        }
        ids.sort();

        let mut probes = Vec::with_capacity(ids.len());
        for id in ids {
            let spec = acp_requirement_spec(&id, configs.get(&id));
            let tool = probe_executable(spec.tool_command).await;
            let adapter = match spec.adapter {
                Some(adapter) => Some(probe_npm_adapter(adapter.package, adapter.bin).await),
                None => None,
            };
            let runnable = tool.installed && adapter.as_ref().map(|adapter| adapter.installed).unwrap_or(true);
            let mut notes = Vec::new();
            if !tool.installed {
                notes.push(format!("{} is not available on PATH", spec.tool_command));
            }
            if let Some(adapter) = adapter.as_ref() {
                if !adapter.installed {
                    notes.push(format!(
                        "{} is not installed in npm global or offline cache",
                        adapter.name
                    ));
                }
            }

            debug!(
                "ACP requirement probe: id={} tool_installed={} adapter_installed={} runnable={} notes={:?}",
                id,
                tool.installed,
                adapter.as_ref().map(|adapter| adapter.installed).unwrap_or(true),
                runnable,
                notes
            );

            probes.push(AcpClientRequirementProbe {
                id,
                tool,
                adapter,
                runnable,
                notes,
            });
        }

        Ok(probes)
    }

    pub async fn refresh_remote_client_requirements(
        &self,
        remote_connection_id: &str,
    ) -> NortHingResult<Vec<AcpClientRequirementProbe>> {
        let probes = self.probe_remote_client_requirements(remote_connection_id).await?;
        self.remote_capability_store
            .set(RemoteAcpClientRequirementSnapshot {
                connection_id: remote_connection_id.to_string(),
                last_probed_at: current_unix_timestamp_ms(),
                probes: probes.clone(),
            })
            .await?;
        Ok(probes)
    }

    pub async fn probe_remote_client_requirements(
        &self,
        remote_connection_id: &str,
    ) -> NortHingResult<Vec<AcpClientRequirementProbe>> {
        let remote_manager = remote_workspace_manager()
            .ok_or_else(|| NortHingError::service("Remote workspace manager is not initialized".to_string()))?;
        let ssh_manager = remote_manager
            .get_ssh_manager()
            .await
            .ok_or_else(|| NortHingError::service("SSH manager is not available for remote ACP".to_string()))?;

        let config_file = self.load_config_file().await?;
        let mut ids = config_file.acp_clients.keys().cloned().collect::<Vec<_>>();
        for id in builtin_client_ids() {
            if !ids.iter().any(|candidate| candidate == id) {
                ids.push(id.to_string());
            }
        }
        ids.sort();

        let mut probes = Vec::with_capacity(ids.len());
        for id in ids {
            let config = resolve_config_for_client(&config_file, &id, Some(remote_connection_id));
            let spec = acp_requirement_spec(&id, config.as_ref());
            let tool = probe_remote_executable(
                &ssh_manager,
                remote_connection_id,
                spec.tool_command,
                config.as_ref().map(|config| &config.env),
            )
            .await;
            let adapter = match spec.adapter {
                Some(adapter) => Some(
                    probe_remote_npx_adapter(
                        &ssh_manager,
                        remote_connection_id,
                        adapter.package,
                        config.as_ref().map(|config| &config.env),
                    )
                    .await,
                ),
                None => None,
            };
            let runnable = tool.installed && adapter.as_ref().map(|adapter| adapter.installed).unwrap_or(true);
            let mut notes = Vec::new();
            if !tool.installed {
                notes.push(format!("{} is not available on remote PATH", spec.tool_command));
            }
            if let Some(adapter) = adapter.as_ref() {
                if !adapter.installed {
                    notes.push("npx is not available on remote PATH".to_string());
                }
            }

            debug!(
                "Remote ACP requirement probe: id={} tool_installed={} adapter_installed={} runnable={} notes={:?}",
                id,
                tool.installed,
                adapter.as_ref().map(|adapter| adapter.installed).unwrap_or(true),
                runnable,
                notes
            );

            probes.push(AcpClientRequirementProbe {
                id,
                tool,
                adapter,
                runnable,
                notes,
            });
        }

        Ok(probes)
    }

    pub async fn register_configured_tools(self: &Arc<Self>, configs: &HashMap<String, AcpClientConfig>) {
        let registry = global_tool_registry();
        let mut registry = registry.write().await;
        registry.unregister_tools_by_prefix("acp__");

        let tools = configs
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(id, config)| {
                Arc::new(AcpAgentTool::new(id.clone(), config.clone(), self.clone()))
                    as Arc<dyn northhing_core::agentic::tools::framework::Tool>
            })
            .collect::<Vec<_>>();

        for tool in tools {
            debug!("Registering ACP client tool: name={}", tool.name());
            registry.register_tool(tool);
        }
    }
}
