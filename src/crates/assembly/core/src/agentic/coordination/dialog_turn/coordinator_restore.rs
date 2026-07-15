//! Sub-domain: restore.
//! Spec §2.1 — facade methods extracted from dialog_turn/mod.rs (R44a refactor).
//! Contains 12 thin wrappers around `*_impl` restore helpers in the `restore` sibling.

use super::super::coordinator::*;

use crate::agentic::core::Session;
use crate::agentic::session::session_manager::SessionViewRestoreTiming;
use crate::service::session::DialogTurnData;
use crate::util::errors::NortHingResult;
use std::path::Path;

impl ConversationCoordinator {
    /// Restore session
    pub async fn restore_session(&self, workspace_path: &Path, session_id: &str) -> NortHingResult<Session> {
        self.restore_session_impl(workspace_path, session_id).await
    }

    pub async fn restore_internal_session(&self, workspace_path: &Path, session_id: &str) -> NortHingResult<Session> {
        self.restore_internal_session_impl(workspace_path, session_id).await
    }

    /// Restore session and return the persisted turns read during restore.
    pub async fn restore_session_with_turns(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>)> {
        self.restore_session_with_turns_impl(workspace_path, session_id).await
    }

    pub async fn restore_internal_session_with_turns(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>)> {
        self.restore_internal_session_with_turns_impl(workspace_path, session_id)
            .await
    }

    /// Restore only the UI-visible persisted session view.
    pub async fn restore_session_view(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>)> {
        self.restore_session_view_impl(workspace_path, session_id).await
    }

    pub async fn restore_session_view_timed(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, SessionViewRestoreTiming)> {
        self.restore_session_view_timed_impl(workspace_path, session_id).await
    }

    pub async fn restore_session_view_tail(
        &self,
        workspace_path: &Path,
        session_id: &str,
        tail_turn_count: usize,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, usize)> {
        self.restore_session_view_tail_impl(workspace_path, session_id, tail_turn_count)
            .await
    }

    pub async fn restore_session_view_tail_timed(
        &self,
        workspace_path: &Path,
        session_id: &str,
        tail_turn_count: usize,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, usize, SessionViewRestoreTiming)> {
        self.restore_session_view_tail_timed_impl(workspace_path, session_id, tail_turn_count)
            .await
    }

    pub async fn restore_internal_session_view(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>)> {
        self.restore_internal_session_view_impl(workspace_path, session_id)
            .await
    }

    pub async fn restore_internal_session_view_timed(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, SessionViewRestoreTiming)> {
        self.restore_internal_session_view_timed_impl(workspace_path, session_id)
            .await
    }

    pub async fn restore_internal_session_view_tail(
        &self,
        workspace_path: &Path,
        session_id: &str,
        tail_turn_count: usize,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, usize)> {
        self.restore_internal_session_view_tail_impl(workspace_path, session_id, tail_turn_count)
            .await
    }

    pub async fn restore_internal_session_view_tail_timed(
        &self,
        workspace_path: &Path,
        session_id: &str,
        tail_turn_count: usize,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, usize, SessionViewRestoreTiming)> {
        self.restore_internal_session_view_tail_timed_impl(workspace_path, session_id, tail_turn_count)
            .await
    }
}
