// R20c split: ACP client config loading + client listing.
// File: src/crates/interfaces/acp/src/client/manager_config_loading.rs
// Origin: manager_config.rs (292 canonical lines, QClaw R20a P2 D-deviation
//        +21% over QClaw 242 tolerance)
// Mavis fix: R20c split manager_config.rs into 2 files (this +
//        manager_config_requirements.rs) to close the 242 line cap.
//        Sub-domain A + C: client listing (1 pub method) +
//        config reading (3 pub helpers — must be pub because sibling
//        files call them via inherent dispatch:
//          - probe_client_requirements (requirements.rs) -> load_configs
//          - probe_remote_client_requirements (requirements.rs) -> load_config_file
//          - initialize_all (start.rs) -> load_configs).
//        Spec deviation R20c-D1: visibility table listed these as
//        "plain async fn" but they have sibling consumers. Kept pub
//        per R19 visibility lesson.
// R20c sibling: manager_config_requirements.rs (sub-domain B + D)
// R20c sibling: manager_connection_start.rs + manager_connection_stop.rs
// R19 sibling files:
//             manager_install.rs
//             manager_session.rs
//             manager_prompt.rs
//             manager_cancel.rs
//             manager_permission.rs
//             manager_process.rs
//             manager_process_lifecycle.rs
//             manager_session_helpers_identity.rs
//             manager_session_helpers_session_response.rs
//             manager_session_helpers_session_state.rs
//             manager_errors.rs
//
// All method bodies are moved verbatim from main. No behavior change.

use super::config::{AcpClientConfig, AcpClientConfigFile, AcpClientInfo, AcpClientStatus};
use super::manager::CONFIG_PATH;
use super::manager_session_helpers_identity::{aggregate_client_status, parse_config_value};
use super::tool::AcpAgentTool;
use super::AcpClientService;
use northhing_core::util::errors::NortHingResult;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

impl AcpClientService {
    pub async fn list_clients(self: &Arc<Self>) -> NortHingResult<Vec<AcpClientInfo>> {
        let configs = self.load_configs().await?;
        let mut infos = Vec::with_capacity(configs.len());
        for (id, config) in configs {
            let clients = self
                .clients
                .iter()
                .filter(|entry| entry.value().client_id == id)
                .map(|entry| entry.value().clone())
                .collect::<Vec<_>>();
            let mut statuses = Vec::with_capacity(clients.len());
            let mut session_count = 0usize;
            for client in &clients {
                statuses.push(*client.status.read().await);
                session_count += client.sessions.len();
            }
            let status = aggregate_client_status(&statuses);
            infos.push(AcpClientInfo {
                tool_name: AcpAgentTool::tool_name_for(&id),
                name: config.name.clone().unwrap_or_else(|| id.clone()),
                command: config.command.clone(),
                args: config.args.clone(),
                enabled: config.enabled,
                readonly: config.readonly,
                permission_mode: config.permission_mode,
                id,
                status,
                session_count,
            });
        }
        infos.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(infos)
    }

    pub async fn load_configs(&self) -> NortHingResult<HashMap<String, AcpClientConfig>> {
        Ok(self.load_config_file().await?.acp_clients)
    }

    pub async fn load_config_file(&self) -> NortHingResult<AcpClientConfigFile> {
        parse_config_value(self.load_config_value().await?)
    }

    pub async fn load_config_value(&self) -> NortHingResult<serde_json::Value> {
        Ok(self
            .config_service
            .config::<serde_json::Value>(Some(CONFIG_PATH))
            .await
            .unwrap_or_else(|_| json!({ "acpClients": {} })))
    }
}
