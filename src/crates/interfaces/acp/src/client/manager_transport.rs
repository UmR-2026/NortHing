// R19 split: ACP client transport setup (local + remote) and remote session attachment.
// File: src/crates/interfaces/acp/src/client/manager_transport.rs
// Origin: manager.rs (2519 lines god-object, Kimi P1 critical)
// Sibling files:
//             manager_config.rs
//             manager_install.rs
//             manager_connection.rs
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

use super::config::{
    AcpClientConfig, AcpClientConfigFile, AcpClientInfo, AcpClientPermissionMode, AcpClientRequirementProbe,
    AcpClientStatus, RemoteAcpClientRequirementSnapshot,
};
use super::manager::{
    AcpCancelHandle, AcpClientConnection, AcpIncomingStream, AcpOutgoingStream, AcpRemoteSession, StartClientConfig,
    CLIENT_STARTUP_TIMEOUT, CLIENT_STARTUP_TIMEOUT_SECS,
};
use super::manager_errors::{protocol_error, startup_timeout_error, startup_timeout_error_message};
use super::manager_process::{ensure_remote_client_supported, render_remote_client_command, resolve_config_for_client};
use super::manager_process_lifecycle::{configure_process_group, terminate_child_process_tree};
use super::remote_session::{preferred_resume_strategies, AcpRemoteSessionStrategy};
use super::requirements::{
    acp_requirement_spec, apply_command_environment, install_npm_cli_package, install_remote_npm_cli_package,
    predownload_npm_adapter, probe_executable, probe_npm_adapter, probe_remote_executable, probe_remote_npx_adapter,
    resolve_configured_command,
};
use super::AcpClientService;
use agent_client_protocol::schema::{
    AgentCapabilities, CancelNotification, ClientCapabilities, CloseSessionRequest, Implementation, InitializeRequest,
    LoadSessionRequest, LoadSessionResponse, NewSessionRequest, NewSessionResponse, PermissionOption,
    PermissionOptionKind, ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, ResumeSessionRequest, ResumeSessionResponse, SelectedPermissionOutcome,
    SessionConfigOption, SessionConfigOptionValue, SessionModelState, SetSessionConfigOptionRequest,
    SetSessionModelRequest, StopReason,
};
use agent_client_protocol::{ActiveSession, Agent, ByteStreams, Client, ConnectionTo, Error, SessionMessage};
use northhing_core::service::remote_ssh::workspace_state::remote_workspace_manager;
use northhing_core::util::errors::{NortHingError, NortHingResult};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::{debug, info, warn};

impl AcpClientService {
    pub(super) async fn run_startup_step<T, F>(
        self: &Arc<Self>,
        client: &Arc<AcpClientConnection>,
        phase: &'static str,
        future: F,
    ) -> Result<T, Error>
    where
        F: Future<Output = Result<T, Error>>,
    {
        match tokio::time::timeout(CLIENT_STARTUP_TIMEOUT, future).await {
            Ok(result) => result,
            Err(_) => {
                warn!(
                    "ACP client startup timed out: id={} connection_id={} phase={} timeout_secs={}",
                    client.client_id, client.id, phase, CLIENT_STARTUP_TIMEOUT_SECS
                );
                self.cleanup_failed_startup(&client.id).await;
                Err(agent_client_protocol::util::internal_error(
                    startup_timeout_error_message(&client.client_id, phase),
                ))
            }
        }
    }

    pub(super) async fn attach_remote_session(
        &self,
        client: &Arc<AcpClientConnection>,
        session_key: &str,
        northhing_session_id: &str,
        session_storage_path: Option<&Path>,
        session: &mut AcpRemoteSession,
        response: NewSessionResponse,
        strategy: AcpRemoteSessionStrategy,
        last_resume_error: Option<String>,
    ) -> NortHingResult<()> {
        let cx = client.connection().await?;
        let models = response.models.clone();
        let config_options = response.config_options.clone().unwrap_or_default();
        let active = cx.attach_session(response, Vec::new()).map_err(protocol_error)?;
        let remote_session_id = active.session_id().to_string();
        client.cancel_handles.insert(
            session_key.to_string(),
            AcpCancelHandle {
                session_id: remote_session_id.clone(),
                connection: active.connection(),
            },
        );
        self.session_permission_modes
            .insert(remote_session_id.clone(), client.config.permission_mode);
        if let Some(session_storage_path) = session_storage_path {
            self.session_persistence
                .update_remote_session_state(
                    session_storage_path,
                    northhing_session_id,
                    &remote_session_id,
                    strategy.as_str(),
                    last_resume_error,
                )
                .await?;
        }
        session.models = models;
        session.config_options = config_options;
        session.discard_pending_updates_before_next_prompt = matches!(strategy, AcpRemoteSessionStrategy::Load);
        session.active = Some(active);
        Ok(())
    }

    pub async fn start_local_transport(
        &self,
        client_id: &str,
        connection_id: &str,
        config: &AcpClientConfig,
    ) -> NortHingResult<(ByteStreams<AcpOutgoingStream, AcpIncomingStream>, Child)> {
        let program = resolve_configured_command(&config.command, &config.env);
        let mut command = northhing_core::util::process_manager::create_tokio_command(&program);
        command
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        apply_command_environment(&mut command, Some(&config.env));
        configure_process_group(&mut command);

        let mut child = command.spawn().map_err(|error| {
            NortHingError::service(format!("Failed to spawn ACP client '{}': {}", client_id, error))
        })?;

        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                terminate_child_process_tree(connection_id, child).await;
                return Err(NortHingError::service(format!(
                    "ACP client '{}' stdout is unavailable",
                    client_id
                )));
            }
        };
        let stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                terminate_child_process_tree(connection_id, child).await;
                return Err(NortHingError::service(format!(
                    "ACP client '{}' stdin is unavailable",
                    client_id
                )));
            }
        };

        Ok((
            ByteStreams::new(Box::pin(stdin.compat_write()), Box::pin(stdout.compat())),
            child,
        ))
    }

    pub async fn open_transport_for_connection(
        &self,
        client_id: &str,
        connection_id: &str,
        config: &AcpClientConfig,
        workspace_path: Option<&str>,
        remote_connection_id: Option<&str>,
    ) -> NortHingResult<(ByteStreams<AcpOutgoingStream, AcpIncomingStream>, Option<Child>)> {
        match remote_connection_id {
            Some(remote_connection_id) => self
                .start_remote_transport(client_id, config, workspace_path, remote_connection_id)
                .await
                .map(|transport| (transport, None)),
            None => self
                .start_local_transport(client_id, connection_id, config)
                .await
                .map(|(transport, child)| (transport, Some(child))),
        }
    }

    pub async fn start_remote_transport(
        &self,
        client_id: &str,
        config: &AcpClientConfig,
        workspace_path: Option<&str>,
        remote_connection_id: &str,
    ) -> NortHingResult<ByteStreams<AcpOutgoingStream, AcpIncomingStream>> {
        let command = render_remote_client_command(config, workspace_path)?;
        let remote_manager = remote_workspace_manager()
            .ok_or_else(|| NortHingError::service("Remote workspace manager is not initialized".to_string()))?;
        let ssh_manager = remote_manager
            .get_ssh_manager()
            .await
            .ok_or_else(|| NortHingError::service("SSH manager is not available for remote ACP".to_string()))?;
        let channel = ssh_manager
            .open_exec_channel(remote_connection_id, &command)
            .await
            .map_err(|error| {
                NortHingError::service(format!("Failed to start remote ACP client '{}': {}", client_id, error))
            })?;
        let stream = channel.into_stream();
        let (reader, writer) = tokio::io::split(stream);
        Ok(ByteStreams::new(
            Box::pin(writer.compat_write()),
            Box::pin(reader.compat()),
        ))
    }

    pub(super) async fn resolve_start_client_config(
        &self,
        client_id: &str,
        workspace_path: Option<&str>,
        remote_connection_id: Option<&str>,
    ) -> NortHingResult<StartClientConfig> {
        let remote_connection_id = remote_connection_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let is_remote = remote_connection_id.is_some();
        let config_file = self.load_config_file().await?;
        let config = resolve_config_for_client(&config_file, client_id, remote_connection_id.as_deref())
            .ok_or_else(|| NortHingError::NotFound(format!("ACP client not found: {}", client_id)))?;

        if config.command.trim().is_empty() {
            return Err(NortHingError::config(format!(
                "ACP client command is empty: {}",
                client_id
            )));
        }
        if !config.enabled {
            return Err(NortHingError::config(format!("ACP client is disabled: {}", client_id)));
        }

        if is_remote {
            ensure_remote_client_supported(client_id, workspace_path)?;
        }

        Ok(StartClientConfig {
            remote_connection_id,
            config,
        })
    }
}
