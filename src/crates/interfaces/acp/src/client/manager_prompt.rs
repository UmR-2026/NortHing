// R19 split: ACP prompt + prompt-stream entry points.
// File: src/crates/interfaces/acp/src/client/manager_prompt.rs
// Origin: manager.rs (2519 lines god-object, Kimi P1 critical)
// Sibling files:
//             manager_config.rs
//             manager_install.rs
//             manager_connection.rs
//             manager_transport.rs
//             manager_session.rs
//             manager_cancel.rs
//             manager_permission.rs
//             manager_process.rs
//             manager_process_lifecycle.rs
//             manager_session_helpers.rs
//             manager_errors.rs
//
// All method bodies are moved verbatim from main. No behavior change.

use super::manager_errors::protocol_error;
use super::manager_session_helpers_session_response::{drain_pending_turn_updates, read_turn_to_string};
use super::manager_session_helpers_session_state::{
    discard_pending_session_updates_if_needed, update_session_from_events,
};
use super::stream::{
    acp_dispatch_to_stream_events_with_tracker, AcpClientStreamEvent, AcpStreamRoundTracker, AcpToolCallTracker,
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
use northhing_core::util::errors::{NortHingError, NortHingResult};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

impl AcpClientService {
    pub async fn prompt_agent(
        self: &Arc<Self>,
        client_id: &str,
        prompt: String,
        workspace_path: Option<String>,
        remote_connection_id: Option<String>,
        northhing_session_id: String,
        session_storage_path: Option<PathBuf>,
        timeout_seconds: Option<u64>,
    ) -> NortHingResult<String> {
        let resolved = self
            .resolve_or_create_client_session(
                client_id,
                workspace_path,
                remote_connection_id.as_deref(),
                &northhing_session_id,
            )
            .await?;

        let run = async {
            let mut session = resolved.session.lock().await;
            self.ensure_remote_session(
                &resolved.client,
                &resolved.session_key,
                &resolved.cwd,
                &northhing_session_id,
                session_storage_path.as_deref(),
                &mut session,
            )
            .await?;

            discard_pending_session_updates_if_needed(&mut session).await;
            let active = session
                .active
                .as_mut()
                .ok_or_else(|| NortHingError::service("ACP session was not initialized"))?;
            active.send_prompt(prompt).map_err(protocol_error)?;
            read_turn_to_string(&mut session).await
        };

        if let Some(seconds) = timeout_seconds.filter(|seconds| *seconds > 0) {
            tokio::time::timeout(Duration::from_secs(seconds), run)
                .await
                .map_err(|_| NortHingError::tool(format!("ACP client timed out after {}s", seconds)))?
        } else {
            run.await
        }
    }

    pub async fn prompt_agent_stream<F>(
        self: &Arc<Self>,
        client_id: &str,
        prompt: String,
        workspace_path: Option<String>,
        remote_connection_id: Option<String>,
        northhing_session_id: String,
        session_storage_path: Option<PathBuf>,
        timeout_seconds: Option<u64>,
        mut on_event: F,
    ) -> NortHingResult<()>
    where
        F: FnMut(AcpClientStreamEvent) -> NortHingResult<()> + Send,
    {
        let resolved = self
            .resolve_or_create_client_session(
                client_id,
                workspace_path,
                remote_connection_id.as_deref(),
                &northhing_session_id,
            )
            .await?;

        let run = async {
            let mut session = resolved.session.lock().await;
            self.ensure_remote_session(
                &resolved.client,
                &resolved.session_key,
                &resolved.cwd,
                &northhing_session_id,
                session_storage_path.as_deref(),
                &mut session,
            )
            .await?;

            discard_pending_session_updates_if_needed(&mut session).await;
            {
                let active = session
                    .active
                    .as_mut()
                    .ok_or_else(|| NortHingError::service("ACP session was not initialized"))?;
                active.send_prompt(prompt).map_err(protocol_error)?;
            }
            let mut round_tracker = AcpStreamRoundTracker::new();
            let mut tool_call_tracker = AcpToolCallTracker::new();

            loop {
                let message = {
                    let active = session
                        .active
                        .as_mut()
                        .ok_or_else(|| NortHingError::service("ACP session was not initialized"))?;
                    active.read_update().await.map_err(protocol_error)?
                };

                match message {
                    SessionMessage::SessionMessage(dispatch) => {
                        let events =
                            acp_dispatch_to_stream_events_with_tracker(dispatch, &mut tool_call_tracker).await?;
                        update_session_from_events(&mut session, &events);
                        for event in events {
                            for event in round_tracker.apply(event) {
                                on_event(event)?;
                            }
                        }
                    }
                    SessionMessage::StopReason(stop_reason) => {
                        drain_pending_turn_updates(
                            &mut session,
                            &mut tool_call_tracker,
                            &mut round_tracker,
                            &mut on_event,
                        )
                        .await?;
                        let event = if matches!(stop_reason, StopReason::Cancelled) {
                            AcpClientStreamEvent::Cancelled
                        } else {
                            AcpClientStreamEvent::Completed
                        };
                        on_event(event)?;
                        break;
                    }
                    _ => {}
                }
            }
            Ok(())
        };

        if let Some(seconds) = timeout_seconds.filter(|seconds| *seconds > 0) {
            tokio::time::timeout(Duration::from_secs(seconds), run)
                .await
                .map_err(|_| NortHingError::tool(format!("ACP client timed out after {}s", seconds)))?
        } else {
            run.await
        }
    }
}
