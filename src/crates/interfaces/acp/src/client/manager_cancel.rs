// R19 split: ACP session cancellation entry points.
// File: src/crates/interfaces/acp/src/client/manager_cancel.rs
// Origin: manager.rs (2519 lines god-object, Kimi P1 critical)
// Sibling files:
//             manager_config.rs
//             manager_install.rs
//             manager_connection.rs
//             manager_transport.rs
//             manager_session.rs
//             manager_prompt.rs
//             manager_permission.rs
//             manager_process.rs
//             manager_process_lifecycle.rs
//             manager_session_helpers.rs
//             manager_errors.rs
//
// All method bodies are moved verbatim from main. No behavior change.

use super::manager_errors::protocol_error;
use super::manager_session_helpers_identity::{build_session_key, session_client_connection_id};
use super::AcpClientService;
use agent_client_protocol::schema::{
    AgentCapabilities, CancelNotification, ClientCapabilities, CloseSessionRequest, Implementation, InitializeRequest,
    LoadSessionRequest, LoadSessionResponse, NewSessionRequest, NewSessionResponse, PermissionOption,
    PermissionOptionKind, ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, ResumeSessionRequest, ResumeSessionResponse, SelectedPermissionOutcome,
    SessionConfigOption, SessionConfigOptionValue, SessionModelState, SetSessionConfigOptionRequest,
    SetSessionModelRequest, StopReason,
};
use northhing_core::util::errors::{NortHingError, NortHingResult};
use std::path::{Path, PathBuf};
use std::sync::Arc;

impl AcpClientService {
    pub async fn cancel_agent_session(
        self: &Arc<Self>,
        client_id: &str,
        workspace_path: Option<String>,
        northhing_session_id: String,
    ) -> NortHingResult<()> {
        let connection_id = session_client_connection_id(client_id, &northhing_session_id);
        let client = self
            .clients
            .get(&connection_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| NortHingError::service(format!("ACP client is not running: {}", client_id)))?;

        let cwd = workspace_path
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .ok_or_else(|| NortHingError::validation("Workspace path is required".to_string()))?;
        let session_key = build_session_key(&northhing_session_id, client_id, &cwd);
        let handle = client.cancel_handles.get(&session_key).ok_or_else(|| {
            NortHingError::NotFound(format!(
                "ACP session is not active for client '{}' in workspace '{}'",
                client_id,
                cwd.display()
            ))
        })?;

        handle
            .connection
            .send_notification(CancelNotification::new(handle.session_id.clone()))
            .map_err(protocol_error)?;
        Ok(())
    }

    pub async fn cancel_northhing_session(self: &Arc<Self>, northhing_session_id: &str) -> NortHingResult<bool> {
        let session_key_prefix = format!("{}:", northhing_session_id);
        for client in self.clients.iter().map(|entry| entry.value().clone()) {
            let handle = client
                .cancel_handles
                .iter()
                .find(|entry| entry.key().starts_with(&session_key_prefix))
                .map(|entry| entry.value().clone());

            if let Some(handle) = handle {
                handle
                    .connection
                    .send_notification(CancelNotification::new(handle.session_id.clone()))
                    .map_err(protocol_error)?;
                return Ok(true);
            }
        }

        Ok(false)
    }
}
