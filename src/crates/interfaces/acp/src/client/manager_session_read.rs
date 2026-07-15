// R20a split: ACP client session read-only accessors (options, commands).
// File: src/crates/interfaces/acp/src/client/manager_session_read.rs
// Origin: manager_session_lifecycle.rs (R20a split, closed Critical
// D-deviation: manager_session.rs 486 → 2 files in R20a, then this file
// further splits lifecycle.rs 291 → ~181 to close the 242 line cap).
// Calls into manager_session_resolve.rs via inherent dispatch:
//   self.resolve_or_create_client_session(...), self.ensure_remote_session(...)
// R20a sibling: manager_session_lifecycle.rs (release_northhing_session +
//             set_session_model after this split)
//             manager_session_resolve.rs (3 helpers: 1 private + 2 pub(super))
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
// All method bodies are moved verbatim from manager_session_lifecycle.rs.
// No behavior change.

use super::manager_session_helpers_session_state::drain_pending_session_metadata_updates;
use super::session_options::{session_options_from_state, AcpAvailableCommand, AcpSessionOptions};
use super::AcpClientService;
use northhing_core::util::errors::NortHingResult;
use std::path::PathBuf;
use std::sync::Arc;

impl AcpClientService {
    pub async fn get_session_options(
        self: &Arc<Self>,
        client_id: &str,
        workspace_path: Option<String>,
        remote_connection_id: Option<String>,
        session_storage_path: Option<PathBuf>,
        northhing_session_id: String,
    ) -> NortHingResult<AcpSessionOptions> {
        let resolved = self
            .resolve_or_create_client_session(
                client_id,
                workspace_path,
                remote_connection_id.as_deref(),
                &northhing_session_id,
            )
            .await?;

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
        drain_pending_session_metadata_updates(&mut session).await?;
        Ok(session_options_from_state(
            session.models.as_ref(),
            &session.config_options,
            session.context_usage.as_ref(),
        ))
    }

    pub async fn get_session_commands(
        self: &Arc<Self>,
        client_id: &str,
        workspace_path: Option<String>,
        remote_connection_id: Option<String>,
        session_storage_path: Option<PathBuf>,
        northhing_session_id: String,
    ) -> NortHingResult<Vec<AcpAvailableCommand>> {
        let resolved = self
            .resolve_or_create_client_session(
                client_id,
                workspace_path,
                remote_connection_id.as_deref(),
                &northhing_session_id,
            )
            .await?;

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
        drain_pending_session_metadata_updates(&mut session).await?;
        Ok(session.available_commands.clone())
    }
}
