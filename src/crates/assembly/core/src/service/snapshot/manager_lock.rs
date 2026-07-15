//! Snapshot manager — file lock + conflict path.
//!
//! Houses the per-file lock acquire / release / status helpers, the
//! conflict detector, and the Git isolation check that gates tool
//! execution. Capture / invalidate live in `manager_capture` /
//! `manager_invalidate`; read-only queries live in `manager_query`.
//!
//! This is an R46c split sibling of `manager.rs`.

use super::manager::SnapshotManager;

impl SnapshotManager {
    /// Tries to acquire a file lock.
    pub async fn try_acquire_file_lock(
        &self,
        session_id: &str,
        file_path: &str,
        tool_name: &str,
    ) -> crate::service::snapshot::types::SnapshotResult<bool> {
        let snapshot_service = self.snapshot_service.read().await;
        let file_path = std::path::Path::new(file_path);
        snapshot_service
            .try_acquire_file_lock(session_id, file_path, tool_name)
            .await
    }

    /// Releases a file lock.
    pub async fn release_file_lock(
        &self,
        session_id: &str,
        file_path: &str,
    ) -> crate::service::snapshot::types::SnapshotResult<()> {
        let snapshot_service = self.snapshot_service.read().await;
        let file_path = std::path::Path::new(file_path);
        snapshot_service.release_file_lock(session_id, file_path).await
    }

    /// Returns file lock status.
    pub async fn get_file_lock_status(
        &self,
        file_path: &str,
    ) -> crate::service::snapshot::types::SnapshotResult<serde_json::Value> {
        let snapshot_service = self.snapshot_service.read().await;
        let file_path = std::path::Path::new(file_path);

        let lock_status = snapshot_service.get_file_lock_status(file_path).await?;
        Ok(serde_json::json!({
            "locked": lock_status.is_some(),
            "lock_info": lock_status
        }))
    }

    /// Detects file conflicts.
    pub async fn detect_file_conflict(
        &self,
        session_id: &str,
        file_path: &str,
        tool_name: &str,
    ) -> crate::service::snapshot::types::SnapshotResult<serde_json::Value> {
        let snapshot_service = self.snapshot_service.read().await;
        let file_path = std::path::Path::new(file_path);

        let conflict = snapshot_service
            .detect_file_conflict(session_id, file_path, tool_name)
            .await?;

        Ok(serde_json::json!({
            "has_conflict": conflict.is_some(),
            "conflict_info": conflict
        }))
    }

    /// Checks Git isolation status.
    pub async fn check_git_isolation(&self) -> crate::service::snapshot::types::SnapshotResult<bool> {
        let snapshot_service = self.snapshot_service.read().await;
        snapshot_service.check_git_isolation().await
    }
}
