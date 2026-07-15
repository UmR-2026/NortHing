//! Snapshot manager — invalidate / resolve path.
//!
//! Houses the rollback flows (session-wide, turn-anchored, file-scoped)
//! and the accept / reject resolutions that close out a session's
//! modifications. Capture lives in `manager_capture`; read-only queries
//! live in `manager_query`; file locks and conflict checks live in
//! `manager_lock`.
//!
//! This is an R46c split sibling of `manager.rs`.

use std::path::PathBuf;

use super::manager::SnapshotManager;

impl SnapshotManager {
    /// Rolls back a session.
    pub async fn rollback_session(
        &self,
        session_id: &str,
    ) -> crate::service::snapshot::types::SnapshotResult<Vec<PathBuf>> {
        let snapshot_service = self.snapshot_service.read().await;
        snapshot_service.rollback_session(session_id).await
    }

    /// Rolls back to a specific turn.
    pub async fn rollback_to_turn(
        &self,
        session_id: &str,
        turn_index: usize,
    ) -> crate::service::snapshot::types::SnapshotResult<Vec<PathBuf>> {
        let snapshot_service = self.snapshot_service.read().await;
        snapshot_service.rollback_to_turn(session_id, turn_index).await
    }

    /// Accepts all changes in a session.
    pub async fn accept_session(&self, session_id: &str) -> crate::service::snapshot::types::SnapshotResult<()> {
        let snapshot_service = self.snapshot_service.read().await;
        snapshot_service.accept_session(session_id).await
    }

    /// Accepts changes for a single file.
    pub async fn accept_file(
        &self,
        session_id: &str,
        file_path: &str,
    ) -> crate::service::snapshot::types::SnapshotResult<()> {
        let snapshot_service = self.snapshot_service.read().await;
        let file_path = std::path::Path::new(file_path);
        snapshot_service.accept_file(session_id, file_path).await
    }

    /// Rejects changes for a single file by restoring its pre-session state.
    pub async fn reject_file(
        &self,
        session_id: &str,
        file_path: &str,
    ) -> crate::service::snapshot::types::SnapshotResult<Vec<PathBuf>> {
        let snapshot_service = self.snapshot_service.read().await;
        let file_path = std::path::Path::new(file_path);
        snapshot_service.reject_file(session_id, file_path).await
    }
}
