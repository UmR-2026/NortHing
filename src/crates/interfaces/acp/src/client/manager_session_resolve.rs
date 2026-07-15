// R20a split: ACP client session resolution + remote-session bootstrap helpers.
// File: src/crates/interfaces/acp/src/client/manager_session_resolve.rs
// Origin: manager_session.rs (486 lines, Kimi R19 Critical D-deviation +101% over QClaw 242)
// R20a sibling: manager_session_lifecycle.rs (2 pub lifecycle entry methods
//             after Mavis split moved 2 read accessors to manager_session_read.rs)
//             manager_session_read.rs (2 pub read accessors)
// These helpers are called from those siblings via inherent dispatch.
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
//
// Visibility note (R19 lesson applied):
// - resolve_client_session is private (async fn, no pub): only called from
//   resolve_or_create_client_session within this file.
// - resolve_or_create_client_session and ensure_remote_session are pub(super):
//   called from siblings manager_session_lifecycle.rs +
//   manager_session_read.rs via inherent dispatch
//   (self.resolve_or_create_client_session(...), self.ensure_remote_session(...)).
//   pub(super) is the minimum visibility that lets sibling modules reach them.

use super::manager::{AcpClientConnection, AcpRemoteSession, ResolvedClientSession};
use super::manager_errors::{is_startup_timeout_error, protocol_error};
use super::manager_session_helpers_identity::{build_session_key, session_client_connection_id};
use super::manager_session_helpers_session_response::{
    new_session_response_from_load, new_session_response_from_resume,
};
use super::remote_session::{preferred_resume_strategies, AcpRemoteSessionStrategy};
use super::AcpClientService;
use agent_client_protocol::schema::{LoadSessionRequest, NewSessionRequest, ResumeSessionRequest};
use northhing_core::util::errors::{NortHingError, NortHingResult};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::warn;

impl AcpClientService {
    async fn resolve_client_session(
        self: &Arc<Self>,
        client_id: &str,
        workspace_path: Option<String>,
        remote_connection_id: Option<&str>,
        northhing_session_id: &str,
    ) -> NortHingResult<(Arc<AcpClientConnection>, PathBuf, String)> {
        let connection_id = session_client_connection_id(client_id, northhing_session_id);
        self.start_client_connection(
            &connection_id,
            client_id,
            workspace_path.as_deref(),
            remote_connection_id,
        )
        .await?;
        let client = self
            .clients
            .get(&connection_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| NortHingError::service(format!("ACP client is not running: {}", client_id)))?;

        let cwd = workspace_path
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .ok_or_else(|| NortHingError::validation("Workspace path is required".to_string()))?;
        let session_key = build_session_key(northhing_session_id, client_id, &cwd);
        Ok((client, cwd, session_key))
    }

    pub(super) async fn resolve_or_create_client_session(
        self: &Arc<Self>,
        client_id: &str,
        workspace_path: Option<String>,
        remote_connection_id: Option<&str>,
        northhing_session_id: &str,
    ) -> NortHingResult<ResolvedClientSession> {
        let (client, cwd, session_key) = self
            .resolve_client_session(client_id, workspace_path, remote_connection_id, northhing_session_id)
            .await?;
        let session = client
            .sessions
            .entry(session_key.clone())
            .or_insert_with(|| Arc::new(Mutex::new(AcpRemoteSession::new())))
            .clone();
        Ok(ResolvedClientSession {
            client,
            cwd,
            session_key,
            session,
        })
    }

    pub(super) async fn ensure_remote_session(
        self: &Arc<Self>,
        client: &Arc<AcpClientConnection>,
        session_key: &str,
        cwd: &Path,
        northhing_session_id: &str,
        session_storage_path: Option<&Path>,
        session: &mut AcpRemoteSession,
    ) -> NortHingResult<()> {
        if session.active.is_some() {
            return Ok(());
        }

        let cx = client.connection().await?;
        let persisted_remote_session_id = if let Some(session_storage_path) = session_storage_path {
            self.session_persistence
                .load_remote_session_id(session_storage_path, northhing_session_id)
                .await?
        } else {
            None
        };
        let capabilities = client.agent_capabilities.read().await.clone();
        let mut last_resume_error: Option<String> = None;

        for strategy in preferred_resume_strategies(capabilities.as_ref(), persisted_remote_session_id.as_deref()) {
            let response = match strategy {
                AcpRemoteSessionStrategy::Load => {
                    let Some(remote_session_id) = persisted_remote_session_id.as_deref() else {
                        continue;
                    };
                    match self
                        .run_startup_step(
                            client,
                            strategy.startup_phase_name(),
                            cx.send_request(LoadSessionRequest::new(remote_session_id.to_string(), cwd))
                                .block_task(),
                        )
                        .await
                        .map_err(protocol_error)
                    {
                        Ok(response) => new_session_response_from_load(remote_session_id, response),
                        Err(error) => {
                            if is_startup_timeout_error(&error) {
                                return Err(error);
                            }
                            warn!(
                                "Failed to load ACP remote session, falling back: \
                                 client_id={}, remote_session_id={}, error={}",
                                client.id, remote_session_id, error
                            );
                            last_resume_error = Some(error.to_string());
                            continue;
                        }
                    }
                }
                AcpRemoteSessionStrategy::Resume => {
                    let Some(remote_session_id) = persisted_remote_session_id.as_deref() else {
                        continue;
                    };
                    match self
                        .run_startup_step(
                            client,
                            strategy.startup_phase_name(),
                            cx.send_request(ResumeSessionRequest::new(remote_session_id.to_string(), cwd))
                                .block_task(),
                        )
                        .await
                        .map_err(protocol_error)
                    {
                        Ok(response) => new_session_response_from_resume(remote_session_id, response),
                        Err(error) => {
                            if is_startup_timeout_error(&error) {
                                return Err(error);
                            }
                            warn!(
                                "Failed to resume ACP remote session, falling back: \
                                 client_id={}, remote_session_id={}, error={}",
                                client.id, remote_session_id, error
                            );
                            last_resume_error = Some(error.to_string());
                            continue;
                        }
                    }
                }
                AcpRemoteSessionStrategy::New => self
                    .run_startup_step(
                        client,
                        strategy.startup_phase_name(),
                        cx.send_request(NewSessionRequest::new(cwd)).block_task(),
                    )
                    .await
                    .map_err(protocol_error)?,
            };

            self.attach_remote_session(
                client,
                session_key,
                northhing_session_id,
                session_storage_path,
                session,
                response,
                strategy,
                last_resume_error.clone(),
            )
            .await?;
            return Ok(());
        }

        Err(NortHingError::service(
            "Failed to initialize ACP remote session".to_string(),
        ))
    }
}
