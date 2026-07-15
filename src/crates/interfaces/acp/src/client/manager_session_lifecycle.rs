// R20a split: ACP client session lifecycle entry points (release, set model).
// File: src/crates/interfaces/acp/src/client/manager_session_lifecycle.rs
// Origin: manager_session.rs (486 lines, Kimi R19 Critical D-deviation +101%
//        over QClaw 242)
// Mavis fix: R20a split lifecycle.rs 291 → 2 files (this + manager_session_read.rs)
//        to close the 242 line cap (21% over). 2 read-only accessors moved out.
// R20a sibling: manager_session_resolve.rs (3 helpers: 1 private + 2 pub(super))
//             manager_session_read.rs (2 pub read accessors — get_session_options,
//                                  get_session_commands)
// R19 sibling files:
//             manager.rs
//             manager_config.rs
//             manager_install.rs
//             manager_connection.rs
//             manager_transport.rs
//             manager_prompt.rs
//             manager_cancel.rs
//             manager_permission.rs
//             manager_process.rs
//             manager_process_lifecycle.rs
//             manager_session_helpers.rs
//             manager_errors.rs
//
// All method bodies are moved verbatim from main. No behavior change.

use super::manager::SetAcpSessionModelRequest;
use super::manager_errors::protocol_error;
use super::manager_process::close_or_cancel_remote_session;
use super::session_options::{model_config_id, session_options_from_state, AcpSessionOptions};
use super::AcpClientService;
use agent_client_protocol::schema::{SessionConfigOptionValue, SetSessionConfigOptionRequest, SetSessionModelRequest};
use northhing_core::util::errors::{NortHingError, NortHingResult};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::warn;

impl AcpClientService {
    pub async fn release_northhing_session(self: &Arc<Self>, northhing_session_id: &str) -> bool {
        let session_key_prefix = format!("{}:", northhing_session_id);
        let clients = self
            .clients
            .iter()
            .map(|entry| entry.value().clone())
            .collect::<Vec<_>>();
        let mut released = false;
        let mut idle_client_ids = Vec::new();

        for client in clients {
            let session_keys = client
                .sessions
                .iter()
                .filter(|entry| entry.key().starts_with(&session_key_prefix))
                .map(|entry| entry.key().clone())
                .collect::<Vec<_>>();
            if session_keys.is_empty() {
                continue;
            }

            released = true;
            let supports_close = client
                .agent_capabilities
                .read()
                .await
                .as_ref()
                .and_then(|capabilities| capabilities.session_capabilities.close.as_ref())
                .is_some();

            for session_key in session_keys {
                let active_session_id = if let Some((_, session)) = client.sessions.remove(&session_key) {
                    let mut session = session.lock().await;
                    let session_id = session.active.as_ref().map(|active| active.session_id().to_string());
                    session.active = None;
                    session_id
                } else {
                    None
                };
                let cancel_handle = client.cancel_handles.remove(&session_key).map(|(_, handle)| handle);
                let remote_session_id = cancel_handle
                    .as_ref()
                    .map(|handle| handle.session_id.clone())
                    .or(active_session_id);

                let Some(remote_session_id) = remote_session_id else {
                    continue;
                };

                self.session_permission_modes.remove(&remote_session_id);
                let connection = cancel_handle.as_ref().map(|handle| handle.connection.clone());
                close_or_cancel_remote_session(&client, connection, &remote_session_id, supports_close).await;
            }

            if client.id != client.client_id && client.sessions.is_empty() && client.cancel_handles.is_empty() {
                idle_client_ids.push(client.id.clone());
            }
        }

        for connection_id in idle_client_ids {
            if let Err(error) = self.stop_connection(&connection_id).await {
                warn!(
                    "Failed to stop idle ACP client after session release: id={} error={}",
                    connection_id, error
                );
            }
        }

        released
    }

    pub async fn set_session_model(
        self: &Arc<Self>,
        request: SetAcpSessionModelRequest,
        session_storage_path: Option<PathBuf>,
    ) -> NortHingResult<AcpSessionOptions> {
        let resolved = self
            .resolve_or_create_client_session(
                &request.client_id,
                request.workspace_path,
                request.remote_connection_id.as_deref(),
                &request.session_id,
            )
            .await?;

        let mut session = resolved.session.lock().await;
        self.ensure_remote_session(
            &resolved.client,
            &resolved.session_key,
            &resolved.cwd,
            &request.session_id,
            session_storage_path.as_deref(),
            &mut session,
        )
        .await?;
        let active = session
            .active
            .as_ref()
            .ok_or_else(|| NortHingError::service("ACP session was not initialized"))?;
        let remote_session_id = active.session_id().to_string();
        let connection = active.connection();

        let mut set_model_error = None;
        if session.models.is_some() {
            match connection
                .send_request(SetSessionModelRequest::new(
                    remote_session_id.clone(),
                    request.model_id.clone(),
                ))
                .block_task()
                .await
                .map_err(protocol_error)
            {
                Ok(_) => {
                    if let Some(models) = session.models.as_mut() {
                        models.current_model_id = request.model_id.clone().into();
                    }
                    if let Some(session_storage_path) = session_storage_path.as_deref() {
                        self.session_persistence
                            .update_model_id(session_storage_path, &request.session_id, &request.model_id)
                            .await?;
                    }
                    return Ok(session_options_from_state(
                        session.models.as_ref(),
                        &session.config_options,
                        session.context_usage.as_ref(),
                    ));
                }
                Err(error) => {
                    set_model_error = Some(error);
                }
            }
        }

        if let Some(config_id) = model_config_id(&session.config_options) {
            let response = connection
                .send_request(SetSessionConfigOptionRequest::new(
                    remote_session_id,
                    config_id,
                    SessionConfigOptionValue::value_id(request.model_id.clone()),
                ))
                .block_task()
                .await
                .map_err(protocol_error)?;
            session.config_options = response.config_options;
            if let Some(session_storage_path) = session_storage_path.as_deref() {
                self.session_persistence
                    .update_model_id(session_storage_path, &request.session_id, &request.model_id)
                    .await?;
            }
            return Ok(session_options_from_state(
                session.models.as_ref(),
                &session.config_options,
                session.context_usage.as_ref(),
            ));
        }

        if let Some(error) = set_model_error {
            return Err(error);
        }
        Err(NortHingError::NotFound(
            "ACP session does not expose selectable models".to_string(),
        ))
    }
}
