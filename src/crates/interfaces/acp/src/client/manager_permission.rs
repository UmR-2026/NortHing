// R19 split: ACP permission request submission, handling, and mode lookup.
// File: src/crates/interfaces/acp/src/client/manager_permission.rs
// Origin: manager.rs (2519 lines god-object, Kimi P1 critical)
// Sibling files:
//             manager_config.rs
//             manager_install.rs
//             manager_connection.rs
//             manager_transport.rs
//             manager_session.rs
//             manager_prompt.rs
//             manager_cancel.rs
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
    AcpClientPermissionResponse, PendingPermission, SubmitAcpPermissionResponseRequest, PERMISSION_TIMEOUT,
};
use super::manager_errors::{protocol_error, select_permission_by_kind, select_permission_option_id};
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
use northhing_core::infrastructure::events::{emit_global_event, BackendEvent};
use northhing_core::util::errors::{NortHingError, NortHingResult};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex, RwLock};
use tracing::{debug, info, warn};

impl AcpClientService {
    pub async fn submit_permission_response(
        &self,
        request: SubmitAcpPermissionResponseRequest,
    ) -> NortHingResult<AcpClientPermissionResponse> {
        let Some((_, pending)) = self.pending_permissions.remove(&request.permission_id) else {
            return Err(NortHingError::NotFound(format!(
                "ACP permission request not found: {}",
                request.permission_id
            )));
        };

        let option_id = request
            .option_id
            .unwrap_or_else(|| select_permission_option_id(&pending.options, request.approve));
        let response = RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
            SelectedPermissionOutcome::new(option_id),
        ));
        let _ = pending.sender.send(response);
        Ok(AcpClientPermissionResponse {
            permission_id: request.permission_id,
            resolved: true,
        })
    }

    pub async fn handle_permission_request(
        self: Arc<Self>,
        request: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse, Error> {
        let session_id = request.session_id.to_string();
        let permission_mode = self.permission_mode_for_session(&session_id);
        match permission_mode {
            AcpClientPermissionMode::AllowOnce => {
                return Ok(select_permission_by_kind(
                    &request,
                    PermissionOptionKind::AllowOnce,
                    true,
                ));
            }
            AcpClientPermissionMode::RejectOnce => {
                return Ok(select_permission_by_kind(
                    &request,
                    PermissionOptionKind::RejectOnce,
                    false,
                ));
            }
            AcpClientPermissionMode::Ask => {}
        }

        let permission_id = format!("acp_permission_{}", uuid::Uuid::new_v4());
        let (tx, rx) = oneshot::channel();
        self.pending_permissions.insert(
            permission_id.clone(),
            PendingPermission {
                sender: tx,
                options: request.options.clone(),
            },
        );

        let payload = json!({
            "permissionId": permission_id,
            "sessionId": session_id,
            "toolCall": request.tool_call,
            "options": request.options,
        });

        if let Err(error) = emit_global_event(BackendEvent::Custom {
            event_name: "backend-event-acppermissionrequest".to_string(),
            payload,
        })
        .await
        {
            warn!("Failed to emit ACP permission request: {}", error);
        }

        match tokio::time::timeout(PERMISSION_TIMEOUT, rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Ok(RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled)),
            Err(_) => {
                self.pending_permissions.remove(&permission_id);
                Ok(RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled))
            }
        }
    }

    pub fn permission_mode_for_session(&self, session_id: &str) -> AcpClientPermissionMode {
        self.session_permission_modes
            .get(session_id)
            .map(|entry| *entry.value())
            .unwrap_or(AcpClientPermissionMode::Ask)
    }
}
