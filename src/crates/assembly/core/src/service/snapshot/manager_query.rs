//! Snapshot manager — read-only query path.
//!
//! Houses the read paths that surface session state, per-turn / per-file
//! listing, diff content, operation summaries, and aggregate statistics
//! to callers. The capture / invalidate paths live in `manager_capture` /
//! `manager_invalidate`; file locks and conflict checks live in
//! `manager_lock`.
//!
//! This is an R46c split sibling of `manager.rs`.

use std::path::PathBuf;

use super::manager::SnapshotManager;

impl SnapshotManager {
    /// Returns the list of files affected by a session.
    pub async fn get_session_files(
        &self,
        session_id: &str,
    ) -> crate::service::snapshot::types::SnapshotResult<Vec<PathBuf>> {
        let snapshot_service = self.snapshot_service.read().await;
        snapshot_service.get_session_files(session_id).await
    }

    /// Returns the list of turns for a session.
    pub async fn get_session_turns(
        &self,
        session_id: &str,
    ) -> crate::service::snapshot::types::SnapshotResult<Vec<usize>> {
        let snapshot_service = self.snapshot_service.read().await;
        snapshot_service.get_session_turns(session_id).await
    }

    /// Returns the list of files modified in a turn.
    pub async fn get_turn_files(
        &self,
        session_id: &str,
        turn_index: usize,
    ) -> crate::service::snapshot::types::SnapshotResult<Vec<PathBuf>> {
        let snapshot_service = self.snapshot_service.read().await;
        snapshot_service.get_turn_files(session_id, turn_index).await
    }

    /// Returns the diff content for a file.
    pub async fn get_file_diff(
        &self,
        session_id: &str,
        file_path: &str,
        anchor_operation_id: Option<&str>,
    ) -> crate::service::snapshot::types::SnapshotResult<serde_json::Value> {
        let snapshot_service = self.snapshot_service.read().await;
        let file_path = std::path::Path::new(file_path);
        let (original, modified, anchor_line) = snapshot_service
            .get_file_diff_with_anchor(session_id, file_path, anchor_operation_id)
            .await?;

        Ok(serde_json::json!({
            "file_path": file_path.to_string_lossy(),
            "original_content": original,
            "modified_content": modified,
            "anchor_line": anchor_line,
        }))
    }

    pub async fn get_session_file_diff_stats(
        &self,
        session_id: &str,
        file_path: &str,
    ) -> crate::service::snapshot::types::SnapshotResult<crate::service::snapshot::types::SessionFileDiffStats> {
        let snapshot_service = self.snapshot_service.read().await;
        let file_path = std::path::Path::new(file_path);
        snapshot_service
            .get_session_file_diff_stats(session_id, file_path)
            .await
    }

    pub async fn get_operation_summary(
        &self,
        session_id: &str,
        operation_id: &str,
    ) -> crate::service::snapshot::types::SnapshotResult<serde_json::Value> {
        let snapshot_service = self.snapshot_service.read().await;
        let op = snapshot_service.get_operation_summary(session_id, operation_id).await?;
        Ok(serde_json::json!({
            "operation_id": op.operation_id,
            "session_id": op.session_id,
            "turn_index": op.turn_index,
            "seq_in_turn": op.seq_in_turn,
            "file_path": op.file_path.to_string_lossy(),
            "operation_type": format!("{:?}", op.operation_type),
            "tool_name": op.tool_context.tool_name,
            "lines_added": op.diff_summary.lines_added,
            "lines_removed": op.diff_summary.lines_removed,
        }))
    }

    pub async fn get_session(
        &self,
        session_id: &str,
    ) -> crate::service::snapshot::types::SnapshotResult<crate::service::snapshot::types::SessionInfo> {
        let snapshot_service = self.snapshot_service.read().await;
        snapshot_service.get_session(session_id).await
    }

    /// Returns session statistics.
    pub async fn get_session_stats(
        &self,
        session_id: &str,
    ) -> crate::service::snapshot::types::SnapshotResult<serde_json::Value> {
        let snapshot_service = self.snapshot_service.read().await;
        let stats = snapshot_service.get_session_stats(session_id).await?;

        serde_json::to_value(stats).map_err(|e| {
            crate::service::snapshot::types::SnapshotError::ConfigError(format!(
                "Failed to serialize statistics: {}",
                e
            ))
        })
    }

    /// Returns system statistics.
    pub async fn get_system_stats(&self) -> crate::service::snapshot::types::SnapshotResult<serde_json::Value> {
        let snapshot_service = self.snapshot_service.read().await;
        let stats = snapshot_service.get_system_stats().await?;

        serde_json::to_value(stats).map_err(|e| {
            crate::service::snapshot::types::SnapshotError::ConfigError(format!(
                "Failed to serialize system statistics: {}",
                e
            ))
        })
    }

    pub async fn list_sessions(&self) -> crate::service::snapshot::types::SnapshotResult<Vec<String>> {
        let snapshot_service = self.snapshot_service.read().await;
        snapshot_service.list_sessions().await
    }

    /// Returns the change history for a file.
    pub async fn get_file_change_history(
        &self,
        file_path: &std::path::Path,
    ) -> crate::service::snapshot::types::SnapshotResult<Vec<crate::service::snapshot::snapshot_core::FileChangeEntry>>
    {
        let snapshot_service = self.snapshot_service.read().await;
        snapshot_service.get_file_change_history(file_path).await
    }

    /// Returns the list of all modified files.
    pub async fn all_modified_files(&self) -> crate::service::snapshot::types::SnapshotResult<Vec<PathBuf>> {
        let snapshot_service = self.snapshot_service.read().await;
        snapshot_service.all_modified_files().await
    }
}
