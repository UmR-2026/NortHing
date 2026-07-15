//! Sub-domain: restore.
//! Spec §2.1 — facade keeps all public API; this sibling has no private helpers.
//!
//! Revived from R6 placeholder (originally created empty in Round 6) to host
//! the 12 restore_* methods that previously lived in `dialog_turn/mod.rs`
//! L1426-1568. The facade in mod.rs stays `pub async fn` for cross-crate
//! consumers; sibling methods use `pub(super) async fn ..._impl` per R20
//! manager_*.rs precedent. The `_impl` suffix is needed because Rust does
//! NOT allow same-named inherent methods in two `impl` blocks for the same
//! type within one crate (E0592 duplicate definitions).
//!
//! Sibling uses `use super::super::coordinator::*` for the struct definition,
//! matching the session.rs/turn.rs/workspace.rs pattern.

use super::super::coordinator::*;

use crate::agentic::core::Session;
use crate::service::session::DialogTurnData;
use crate::util::errors::NortHingResult;
use std::path::Path;

impl ConversationCoordinator {
    /// Restore session
    pub(super) async fn restore_session_impl(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<Session> {
        self.session_manager.restore_session(workspace_path, session_id).await
    }

    pub(super) async fn restore_internal_session_impl(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<Session> {
        self.session_manager
            .restore_internal_session(workspace_path, session_id)
            .await
    }

    /// Restore session and return the persisted turns read during restore.
    pub(super) async fn restore_session_with_turns_impl(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>)> {
        self.session_manager
            .restore_session_with_turns(workspace_path, session_id)
            .await
    }

    pub(super) async fn restore_internal_session_with_turns_impl(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>)> {
        self.session_manager
            .restore_internal_session_with_turns(workspace_path, session_id)
            .await
    }

    /// Restore only the UI-visible persisted session view.
    pub(super) async fn restore_session_view_impl(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>)> {
        self.session_manager
            .restore_session_view(workspace_path, session_id)
            .await
    }

    pub(super) async fn restore_session_view_timed_impl(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(
        Session,
        Vec<DialogTurnData>,
        crate::agentic::session::session_manager::SessionViewRestoreTiming,
    )> {
        self.session_manager
            .restore_session_view_timed(workspace_path, session_id)
            .await
    }

    pub(super) async fn restore_session_view_tail_impl(
        &self,
        workspace_path: &Path,
        session_id: &str,
        tail_turn_count: usize,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, usize)> {
        self.session_manager
            .restore_session_view_tail(workspace_path, session_id, tail_turn_count)
            .await
    }

    pub(super) async fn restore_session_view_tail_timed_impl(
        &self,
        workspace_path: &Path,
        session_id: &str,
        tail_turn_count: usize,
    ) -> NortHingResult<(
        Session,
        Vec<DialogTurnData>,
        usize,
        crate::agentic::session::session_manager::SessionViewRestoreTiming,
    )> {
        self.session_manager
            .restore_session_view_tail_timed(workspace_path, session_id, tail_turn_count)
            .await
    }

    pub(super) async fn restore_internal_session_view_impl(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>)> {
        self.session_manager
            .restore_internal_session_view(workspace_path, session_id)
            .await
    }

    pub(super) async fn restore_internal_session_view_timed_impl(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(
        Session,
        Vec<DialogTurnData>,
        crate::agentic::session::session_manager::SessionViewRestoreTiming,
    )> {
        self.session_manager
            .restore_internal_session_view_timed(workspace_path, session_id)
            .await
    }

    pub(super) async fn restore_internal_session_view_tail_impl(
        &self,
        workspace_path: &Path,
        session_id: &str,
        tail_turn_count: usize,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, usize)> {
        self.session_manager
            .restore_internal_session_view_tail(workspace_path, session_id, tail_turn_count)
            .await
    }

    pub(super) async fn restore_internal_session_view_tail_timed_impl(
        &self,
        workspace_path: &Path,
        session_id: &str,
        tail_turn_count: usize,
    ) -> NortHingResult<(
        Session,
        Vec<DialogTurnData>,
        usize,
        crate::agentic::session::session_manager::SessionViewRestoreTiming,
    )> {
        self.session_manager
            .restore_internal_session_view_tail_timed(workspace_path, session_id, tail_turn_count)
            .await
    }
}
