//! R49b split sibling: view restore family (session restore for UI view)
//!
//! Contains the restore_session_view* thin wrappers plus the monolithic
//! restore_session_view_internal helper that loads session+turns for display
//! without inserting into the in-memory coordinator state.

use super::session_manager::SessionManager;

use crate::agentic::core::Session;
use crate::agentic::session::session_store_port::CoreSessionStorePort;
use crate::service::session::DialogTurnData;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::timing::elapsed_ms_u64;
pub use northhing_runtime_ports::SessionViewRestoreTiming;
use northhing_runtime_ports::{SessionStoragePathRequest, SessionStorePort, SessionViewRestoreRequest};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::debug;

impl SessionManager {
    /// Restore the persisted session header and turns needed by the UI view
    /// without loading runtime context snapshots or inserting the session into
    /// the in-memory coordinator state.
    pub(crate) async fn restore_session_view(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>)> {
        self.restore_session_view_timed(workspace_path, session_id)
            .await
            .map(|(session, turns, _)| (session, turns))
    }

    pub(crate) async fn restore_session_view_timed(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, SessionViewRestoreTiming)> {
        self.restore_session_view_internal(workspace_path, session_id, false, None)
            .await
            .map(|(session, turns, _, timing)| (session, turns, timing))
    }

    pub(crate) async fn restore_internal_session_view(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>)> {
        self.restore_internal_session_view_timed(workspace_path, session_id)
            .await
            .map(|(session, turns, _)| (session, turns))
    }

    pub(crate) async fn restore_internal_session_view_timed(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, SessionViewRestoreTiming)> {
        self.restore_session_view_internal(workspace_path, session_id, true, None)
            .await
            .map(|(session, turns, _, timing)| (session, turns, timing))
    }

    pub(crate) async fn restore_session_view_tail(
        &self,
        workspace_path: &Path,
        session_id: &str,
        tail_turn_count: usize,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, usize)> {
        self.restore_session_view_tail_timed(workspace_path, session_id, tail_turn_count)
            .await
            .map(|(session, turns, total_turn_count, _)| (session, turns, total_turn_count))
    }

    pub(crate) async fn restore_session_view_tail_timed(
        &self,
        workspace_path: &Path,
        session_id: &str,
        tail_turn_count: usize,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, usize, SessionViewRestoreTiming)> {
        self.restore_session_view_internal(workspace_path, session_id, false, Some(tail_turn_count))
            .await
    }

    pub(crate) async fn restore_internal_session_view_tail(
        &self,
        workspace_path: &Path,
        session_id: &str,
        tail_turn_count: usize,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, usize)> {
        self.restore_internal_session_view_tail_timed(workspace_path, session_id, tail_turn_count)
            .await
            .map(|(session, turns, total_turn_count, _)| (session, turns, total_turn_count))
    }

    pub(crate) async fn restore_internal_session_view_tail_timed(
        &self,
        workspace_path: &Path,
        session_id: &str,
        tail_turn_count: usize,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, usize, SessionViewRestoreTiming)> {
        self.restore_session_view_internal(workspace_path, session_id, true, Some(tail_turn_count))
            .await
    }

    pub(crate) async fn restore_session_view_internal(
        &self,
        workspace_path: &Path,
        session_id: &str,
        include_internal: bool,
        tail_turn_count: Option<usize>,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, usize, SessionViewRestoreTiming)> {
        let restore_request = SessionViewRestoreRequest {
            workspace_path: workspace_path.to_path_buf(),
            session_id: session_id.to_string(),
            include_internal,
            tail_turn_count,
        };
        let restore_started_at = Instant::now();
        let storage_path_started_at = Instant::now();
        let session_storage_path = CoreSessionStorePort
            .resolve_session_storage_path(SessionStoragePathRequest {
                workspace_path: restore_request.workspace_path.clone(),
                remote_connection_id: None,
                remote_ssh_host: None,
            })
            .await
            .map(|resolution| resolution.effective_storage_path().clone())
            .unwrap_or_else(|_| restore_request.workspace_path.clone());
        let resolve_storage_path_duration_ms = elapsed_ms_u64(storage_path_started_at);
        debug!(
            "Session view restore phase completed: session_id={}, phase=resolve_storage_path, duration_ms={}",
            restore_request.session_id, resolve_storage_path_duration_ms
        );

        let metadata_started_at = Instant::now();
        if self
            .persistence_manager
            .load_session_metadata(&session_storage_path, session_id)
            .await?
            .is_some_and(|metadata| !restore_request.include_internal && metadata.should_hide_from_user_lists())
        {
            return Err(NortHingError::NotFound(format!(
                "Session not found: {}",
                restore_request.session_id
            )));
        }
        let visibility_metadata_duration_ms = elapsed_ms_u64(metadata_started_at);
        debug!(
            "Session view restore phase completed: session_id={}, phase=load_metadata, duration_ms={}",
            restore_request.session_id, visibility_metadata_duration_ms
        );

        let session_started_at = Instant::now();
        let (mut session, persisted_turns, total_turn_count, turn_load) = if let Some(tail_turn_count) =
            restore_request.tail_turn_count
        {
            self.persistence_manager
                .load_session_with_tail_turns_timed(&session_storage_path, &restore_request.session_id, tail_turn_count)
                .await?
        } else {
            let (session, turns, timing) = self
                .persistence_manager
                .load_session_with_turns_timed(&session_storage_path, &restore_request.session_id)
                .await?;
            let total_turn_count = turns.len();
            (session, turns, total_turn_count, timing)
        };
        let load_session_with_turns_duration_ms = elapsed_ms_u64(session_started_at);
        debug!(
            "Session view restore phase completed: session_id={}, phase=load_session_with_turns, turn_count={}, total_turn_count={}, tail_turn_count={:?}, duration_ms={}",
            session_id,
            persisted_turns.len(),
            total_turn_count,
            restore_request.tail_turn_count,
            load_session_with_turns_duration_ms
        );

        if !matches!(session.state, crate::agentic::core::SessionState::Idle) {
            let old_state = session.state.clone();
            session.state = crate::agentic::core::SessionState::Idle;
            debug!(
                "Resetting session state during view restore: session_id={}, state={:?} -> Idle",
                session_id, old_state
            );
        }

        let normalize_started_at = Instant::now();
        let persisted_turn_ids: Vec<String> = persisted_turns.iter().map(|turn| turn.turn_id.clone()).collect();
        if session.dialog_turn_ids != persisted_turn_ids {
            debug!(
                "Session view restore normalized turn ids: session_id={}, session_turn_count={}, persisted_turn_count={}",
                session_id,
                session.dialog_turn_ids.len(),
                persisted_turn_ids.len()
            );
            session.dialog_turn_ids = persisted_turn_ids;
        }
        let normalize_turn_ids_duration_ms = elapsed_ms_u64(normalize_started_at);

        let total_duration_ms = elapsed_ms_u64(restore_started_at);
        debug!(
            "Session view restored: session_id={}, session_name={}, turn_count={}, total_duration_ms={}",
            session_id,
            session.session_name,
            persisted_turns.len(),
            total_duration_ms
        );

        let timing = SessionViewRestoreTiming {
            resolve_storage_path_duration_ms,
            visibility_metadata_duration_ms,
            load_session_with_turns_duration_ms,
            normalize_turn_ids_duration_ms,
            total_duration_ms,
            turn_load,
        };

        Ok((session, persisted_turns, total_turn_count, timing))
    }
}
