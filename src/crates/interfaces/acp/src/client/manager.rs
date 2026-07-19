// R19 split: facade for northhing-acp ACP client service.
// File: src/crates/interfaces/acp/src/client/manager.rs
// Origin: manager.rs (2519 lines god-object, Kimi P1 critical)
//
// Thin facade keeping only:
//   - Imports needed by the 4 small entry methods below
//   - 7 internal constants
//   - 2 type aliases (AcpOutgoingStream, AcpIncomingStream)
//   - 3 public structs (SubmitAcpPermissionResponseRequest,
//     AcpClientPermissionResponse, SetAcpSessionModelRequest) - kept here
//     for mod.rs re-exports stability
//   - 6 private type definitions (PendingPermission, AcpClientConnection,
//     AcpRemoteSession, ResolvedClientSession, StartClientConfig,
//     AcpCancelHandle) - used by all siblings via inherent-method dispatch
//   - impl AcpRemoteSession::new
//   - AcpClientService struct + new()
//   - 4 small entry methods (create_flow_session_record,
//     delete_flow_session_record, load_json_config, save_json_config)
//
// All 22 pub method bodies and 17 private methods moved to:
//   - manager_config.rs (8 methods)
//   - manager_install.rs (2 methods)
//   - manager_connection.rs (6 methods)
//   - manager_transport.rs (6 methods)
//   - manager_session.rs (7 methods)
//   - manager_prompt.rs (2 methods)
//   - manager_cancel.rs (2 methods)
//   - manager_permission.rs (3 methods)
//   - manager_process.rs (impl AcpClientConnection + 5 free fns + 2 tests)
//   - manager_process_lifecycle.rs (3 free fns: wait/configure/terminate)
//   - manager_session_helpers.rs (16 free fns)
//   - manager_errors.rs (6 free fns + 3 tests)
//
// Method signatures unchanged. Cross-crate callers continue to call
// service.method() via inherent-method dispatch.
//
// Total: 1 facade + 11 sub-siblings = 12 files (spec said 11; +1 for
// manager_process_lifecycle.rs to keep process.rs strictly ≤242 lines).

use serde;
use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
use futures::io::{AsyncRead as FuturesAsyncRead, AsyncWrite as FuturesAsyncWrite};
use northhing_core::agentic::tools::registry::global_tool_registry;
use northhing_core::infrastructure::events::{emit_global_event, BackendEvent};
use northhing_core::infrastructure::PathManager;
use northhing_core::service::config::ConfigService;
use northhing_core::service::remote_ssh::workspace_state::remote_workspace_manager;
use northhing_core::util::errors::{NortHingError, NortHingResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::{debug, info, warn};

use super::builtin_clients::{builtin_client_ids, default_config_for_builtin_client};
use super::config::{
    AcpClientConfig, AcpClientConfigFile, AcpClientInfo, AcpClientPermissionMode, AcpClientRequirementProbe,
    AcpClientStatus, RemoteAcpClientRequirementSnapshot,
};
use super::remote_capability_store::RemoteAcpCapabilityStore;
use super::remote_session::{preferred_resume_strategies, AcpRemoteSessionStrategy};
use super::remote_shell::{remote_user_shell_command, render_remote_env_assignments, shell_escape};
use super::requirements::{
    acp_requirement_spec, apply_command_environment, install_npm_cli_package, install_remote_npm_cli_package,
    predownload_npm_adapter, probe_executable, probe_npm_adapter, probe_remote_executable, probe_remote_npx_adapter,
    resolve_configured_command,
};
use super::session_options::{
    model_config_id, session_options_from_state, AcpAvailableCommand, AcpSessionContextUsage, AcpSessionOptions,
};
use super::session_persistence::AcpSessionPersistence;
pub use super::session_persistence::CreateAcpFlowSessionRecordResponse;
use super::stream::{
    acp_dispatch_to_stream_events_with_tracker, AcpClientStreamEvent, AcpStreamRoundTracker, AcpToolCallTracker,
};
use super::tool::AcpAgentTool;

use super::manager_session_helpers_identity::parse_config_value;

pub(super) const CONFIG_PATH: &str = "acp_clients";
pub(super) const CLIENT_STARTUP_TIMEOUT_SECS: u64 = 60;
pub(super) const CLIENT_STARTUP_TIMEOUT: Duration = Duration::from_secs(CLIENT_STARTUP_TIMEOUT_SECS);
pub(super) const PERMISSION_TIMEOUT: Duration = Duration::from_secs(600);
pub(super) const SESSION_CLOSE_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const LOAD_REPLAY_DRAIN_QUIET_WINDOW: Duration = Duration::from_millis(250);
pub(super) const LOAD_REPLAY_DRAIN_MAX_DURATION: Duration = Duration::from_secs(2);
pub(super) const SESSION_METADATA_DRAIN_QUIET_WINDOW: Duration = Duration::from_millis(250);
pub(super) const SESSION_METADATA_DRAIN_MAX_DURATION: Duration = Duration::from_secs(2);
pub(super) const TURN_COMPLETION_DRAIN_QUIET_WINDOW: Duration = Duration::from_millis(250);
pub(super) const TURN_COMPLETION_DRAIN_MAX_DURATION: Duration = Duration::from_secs(2);

pub(super) type AcpOutgoingStream = Pin<Box<dyn FuturesAsyncWrite + Send>>;
pub(super) type AcpIncomingStream = Pin<Box<dyn FuturesAsyncRead + Send>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitAcpPermissionResponseRequest {
    pub permission_id: String,
    pub approve: bool,
    #[serde(default)]
    pub option_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpClientPermissionResponse {
    pub permission_id: String,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAcpSessionModelRequest {
    pub client_id: String,
    pub session_id: String,
    #[serde(default)]
    pub workspace_path: Option<String>,
    #[serde(default)]
    pub remote_connection_id: Option<String>,
    #[serde(default)]
    pub remote_ssh_host: Option<String>,
    pub model_id: String,
}

pub struct AcpClientService {
    pub(super) config_service: Arc<ConfigService>,
    pub(super) session_persistence: AcpSessionPersistence,
    pub(super) remote_capability_store: RemoteAcpCapabilityStore,
    pub(super) clients: DashMap<String, Arc<AcpClientConnection>>,
    pub(super) pending_permissions: DashMap<String, PendingPermission>,
    pub(super) session_permission_modes: DashMap<String, AcpClientPermissionMode>,
}

pub(super) struct PendingPermission {
    pub(super) sender: oneshot::Sender<RequestPermissionResponse>,
    pub(super) options: Vec<PermissionOption>,
}

pub(super) struct AcpClientConnection {
    pub(super) id: String,
    pub(super) client_id: String,
    pub(super) config: AcpClientConfig,
    pub(super) status: RwLock<AcpClientStatus>,
    pub(super) connection: RwLock<Option<ConnectionTo<Agent>>>,
    pub(super) agent_capabilities: RwLock<Option<AgentCapabilities>>,
    pub(super) sessions: DashMap<String, Arc<Mutex<AcpRemoteSession>>>,
    pub(super) cancel_handles: DashMap<String, AcpCancelHandle>,
    pub(super) shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
    pub(super) child: Mutex<Option<Child>>,
}

pub(super) struct AcpRemoteSession {
    pub(super) active: Option<ActiveSession<'static, Agent>>,
    pub(super) models: Option<SessionModelState>,
    pub(super) config_options: Vec<SessionConfigOption>,
    pub(super) context_usage: Option<AcpSessionContextUsage>,
    pub(super) available_commands: Vec<AcpAvailableCommand>,
    pub(super) discard_pending_updates_before_next_prompt: bool,
}

pub(super) struct ResolvedClientSession {
    pub(super) client: Arc<AcpClientConnection>,
    pub(super) cwd: PathBuf,
    pub(super) session_key: String,
    pub(super) session: Arc<Mutex<AcpRemoteSession>>,
}

pub(super) struct StartClientConfig {
    pub(super) remote_connection_id: Option<String>,
    pub(super) config: AcpClientConfig,
}

#[derive(Clone)]
pub(super) struct AcpCancelHandle {
    pub(super) session_id: String,
    pub(super) connection: ConnectionTo<Agent>,
}

impl AcpRemoteSession {
    pub fn new() -> Self {
        Self {
            active: None,
            models: None,
            config_options: Vec::new(),
            context_usage: None,
            available_commands: Vec::new(),
            discard_pending_updates_before_next_prompt: false,
        }
    }
}

impl AcpClientService {
    pub fn new(config_service: Arc<ConfigService>, path_manager: Arc<PathManager>) -> NortHingResult<Arc<Self>> {
        Ok(Arc::new(Self {
            config_service,
            session_persistence: AcpSessionPersistence::new(path_manager.clone())?,
            remote_capability_store: RemoteAcpCapabilityStore::new(
                path_manager.user_data_dir().join("ssh_acp_capabilities.json"),
            ),
            clients: DashMap::new(),
            pending_permissions: DashMap::new(),
            session_permission_modes: DashMap::new(),
        }))
    }

    pub async fn create_flow_session_record(
        &self,
        session_storage_path: &Path,
        workspace_path: &str,
        client_id: &str,
        session_name: Option<String>,
    ) -> NortHingResult<CreateAcpFlowSessionRecordResponse> {
        self.session_persistence
            .create_flow_session_record(session_storage_path, workspace_path, client_id, session_name)
            .await
    }

    pub async fn delete_flow_session_record(
        &self,
        session_storage_path: &Path,
        northhing_session_id: &str,
    ) -> NortHingResult<()> {
        self.session_persistence
            .delete_flow_session_record(session_storage_path, northhing_session_id)
            .await
    }

    pub async fn load_json_config(&self) -> NortHingResult<String> {
        let config = parse_config_value(self.load_config_value().await?)?;
        serde_json::to_string_pretty(&config)
            .map_err(|error| NortHingError::config(format!("Failed to render ACP config: {}", error)))
    }

    pub async fn save_json_config(self: &Arc<Self>, json_config: &str) -> NortHingResult<()> {
        let value: serde_json::Value = serde_json::from_str(json_config)
            .map_err(|error| NortHingError::config(format!("Invalid ACP client JSON config: {}", error)))?;
        let config = parse_config_value(value)?;
        let canonical_value = serde_json::to_value(config)
            .map_err(|error| NortHingError::config(format!("Failed to render ACP config: {}", error)))?;
        self.config_service.set_config(CONFIG_PATH, canonical_value).await?;
        self.remote_capability_store.clear().await?;
        self.initialize_all().await
    }
}
