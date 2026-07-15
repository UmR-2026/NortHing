//! Snapshot manager — capture path.
//!
//! Owns the write path that records a tool-driven file change into the
//! underlying `SnapshotService`. Rollback / accept / reject live in
//! `manager_invalidate`; read-only queries live in `manager_query`;
//! file locks and conflict checks live in `manager_lock`.
//!
//! This is an R46c split sibling of `manager.rs`; the same split pattern
//! was used in R42c for `snapshot_core.rs` (siblings declared at the
//! `snapshot/` parent level, all `impl SnapshotManager { ... }` blocks
//! see the facade via `super::manager::SnapshotManager`).

use std::path::PathBuf;

use crate::service::snapshot::types::OperationType;

use super::manager::SnapshotManager;

impl SnapshotManager {
    /// Records a file change.
    pub async fn record_file_change(
        &self,
        session_id: &str,
        turn_index: usize,
        file_path: PathBuf,
        operation_type: OperationType,
        tool_name: String,
    ) -> crate::service::snapshot::types::SnapshotResult<String> {
        let snapshot_service = self.snapshot_service.read().await;
        snapshot_service
            .record_file_change(session_id, turn_index, file_path, operation_type, tool_name)
            .await
    }
}
