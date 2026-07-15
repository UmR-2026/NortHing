// R19 split: ACP client connection impl + config resolution + close-or-cancel session.
// File: src/crates/interfaces/acp/src/client/manager_process.rs
// Origin: manager.rs (2519 lines god-object, Kimi P1 critical)
// Sibling files:
//             manager_config.rs
//             manager_install.rs
//             manager_connection.rs
//             manager_transport.rs
//             manager_session.rs
//             manager_prompt.rs
//             manager_cancel.rs
//             manager_permission.rs
//             manager_process_lifecycle.rs
//             manager_session_helpers.rs
//             manager_errors.rs
//
// All method bodies are moved verbatim from main. No behavior change.

use super::builtin_clients::{builtin_client_ids, default_config_for_builtin_client};
use super::config::{
    AcpClientConfig, AcpClientConfigFile, AcpClientInfo, AcpClientPermissionMode, AcpClientRequirementProbe,
    AcpClientStatus, RemoteAcpClientRequirementSnapshot,
};
use super::manager::AcpClientConnection;
use super::manager::StartClientConfig;
use super::manager::SESSION_CLOSE_TIMEOUT;
use super::manager_errors::{protocol_error, startup_timeout_error};
use super::remote_shell::{remote_user_shell_command, render_remote_env_assignments, shell_escape};
use agent_client_protocol::schema::{
    AgentCapabilities, CancelNotification, ClientCapabilities, CloseSessionRequest, Implementation, InitializeRequest,
    LoadSessionRequest, LoadSessionResponse, NewSessionRequest, NewSessionResponse, PermissionOption,
    PermissionOptionKind, ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, ResumeSessionRequest, ResumeSessionResponse, SelectedPermissionOutcome,
    SessionConfigOption, SessionConfigOptionValue, SessionModelState, SetSessionConfigOptionRequest,
    SetSessionModelRequest, StopReason,
};
use agent_client_protocol::{ActiveSession, Agent, ByteStreams, Client, ConnectionTo, Error, SessionMessage};
use dashmap::DashMap;
use northhing_core::util::errors::{NortHingError, NortHingResult};
use tokio::sync::{oneshot, Mutex, RwLock};
use tracing::{debug, info, warn};

pub fn resolve_config_for_client(
    config_file: &AcpClientConfigFile,
    client_id: &str,
    remote_connection_id: Option<&str>,
) -> Option<AcpClientConfig> {
    config_file
        .acp_clients
        .get(client_id)
        .cloned()
        .or_else(|| remote_connection_id.and_then(|_| default_config_for_builtin_client(client_id)))
}

pub fn ensure_remote_client_supported(_client_id: &str, workspace_path: Option<&str>) -> NortHingResult<()> {
    if workspace_path
        .map(str::trim)
        .is_none_or(|workspace_path| workspace_path.is_empty())
    {
        return Err(NortHingError::validation(
            "Workspace path is required for remote ACP sessions".to_string(),
        ));
    }

    Ok(())
}

pub fn render_remote_client_command(config: &AcpClientConfig, workspace_path: Option<&str>) -> NortHingResult<String> {
    let command = config.command.trim();
    if command.is_empty() {
        return Err(NortHingError::config("ACP client command is empty".to_string()));
    }

    let mut command_parts = Vec::new();
    command_parts.push(shell_escape(command));
    command_parts.extend(config.args.iter().map(|arg| shell_escape(arg)));

    let mut parts = Vec::new();
    parts.push("exec".to_string());
    let env_assignments = render_remote_env_assignments(&config.env);
    if !env_assignments.is_empty() {
        parts.push("env".to_string());
        parts.extend(env_assignments);
    }
    parts.extend(command_parts);

    let command = parts.join(" ");
    let workspace_path = workspace_path.map(str::trim).unwrap_or_default();
    let body = if workspace_path.is_empty() {
        command
    } else {
        format!("cd {} && {}", shell_escape(workspace_path), command)
    };
    Ok(remote_user_shell_command(&body))
}

pub fn current_unix_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

impl AcpClientConnection {
    pub fn new(id: String, client_id: String, config: AcpClientConfig) -> Self {
        Self {
            id,
            client_id,
            config,
            status: RwLock::new(AcpClientStatus::Configured),
            connection: RwLock::new(None),
            agent_capabilities: RwLock::new(None),
            sessions: DashMap::new(),
            cancel_handles: DashMap::new(),
            shutdown_tx: Mutex::new(None),
            child: Mutex::new(None),
        }
    }

    pub async fn connection(&self) -> NortHingResult<ConnectionTo<Agent>> {
        self.connection
            .read()
            .await
            .clone()
            .ok_or_else(|| NortHingError::service(format!("ACP client is not connected: {}", self.id)))
    }
}

pub async fn close_or_cancel_remote_session(
    client: &AcpClientConnection,
    connection: Option<ConnectionTo<Agent>>,
    remote_session_id: &str,
    supports_close: bool,
) {
    let connection = match connection {
        Some(connection) => connection,
        None => match client.connection().await {
            Ok(connection) => connection,
            Err(error) => {
                warn!(
                    "Failed to release ACP session because client is disconnected: client_id={} remote_session_id={} error={}",
                    client.id, remote_session_id, error
                );
                return;
            }
        },
    };

    if supports_close {
        let close = connection
            .send_request(CloseSessionRequest::new(remote_session_id.to_string()))
            .block_task();
        match tokio::time::timeout(SESSION_CLOSE_TIMEOUT, close).await {
            Ok(Ok(_)) => {
                debug!(
                    "ACP remote session closed: client_id={} remote_session_id={}",
                    client.id, remote_session_id
                );
            }
            Ok(Err(error)) => {
                warn!(
                    "Failed to close ACP remote session: client_id={} remote_session_id={} error={}",
                    client.id, remote_session_id, error
                );
            }
            Err(_) => {
                warn!(
                    "Timed out closing ACP remote session: client_id={} remote_session_id={} timeout_ms={}",
                    client.id,
                    remote_session_id,
                    SESSION_CLOSE_TIMEOUT.as_millis()
                );
            }
        }
    } else if let Err(error) = connection
        .send_notification(CancelNotification::new(remote_session_id.to_string()))
        .map_err(protocol_error)
    {
        warn!(
            "Failed to cancel ACP remote session during release: client_id={} remote_session_id={} error={}",
            client.id, remote_session_id, error
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn renders_remote_client_command_from_config() {
        let config = AcpClientConfig {
            name: Some("Custom".to_string()),
            command: "custom-acp".to_string(),
            args: vec!["--stdio".to_string(), "with space".to_string()],
            env: HashMap::from([
                ("PATH".to_string(), "/remote/bin:/usr/bin".to_string()),
                ("INVALID-NAME".to_string(), "ignored".to_string()),
            ]),
            enabled: true,
            readonly: false,
            permission_mode: AcpClientPermissionMode::Ask,
        };

        let command = render_remote_client_command(&config, Some("/srv/my repo")).expect("command");
        assert!(command.starts_with("bash -lc "));
        assert!(command.contains(".nvm/nvm.sh"));
        assert!(command.contains(
            "cd '\\''/srv/my repo'\\'' && exec env PATH=/remote/bin:/usr/bin custom-acp --stdio '\\''with space'\\''"
        ));
    }

    #[test]
    fn resolves_remote_client_config_from_global_config() {
        let config_file = AcpClientConfigFile {
            acp_clients: HashMap::from([(
                "codex".to_string(),
                AcpClientConfig {
                    name: Some("Codex".to_string()),
                    command: "npx".to_string(),
                    args: vec!["--yes".to_string(), "@zed-industries/codex-acp@latest".to_string()],
                    env: HashMap::from([("BASE".to_string(), "1".to_string())]),
                    enabled: true,
                    readonly: false,
                    permission_mode: AcpClientPermissionMode::Ask,
                },
            )]),
        };

        let resolved = resolve_config_for_client(&config_file, "codex", Some("huawei-server")).expect("config");

        assert_eq!(resolved.command, "npx");
        assert_eq!(resolved.args, vec!["--yes", "@zed-industries/codex-acp@latest"]);
        assert_eq!(resolved.env.get("BASE").map(String::as_str), Some("1"));
        assert!(resolved.enabled);
    }
}
