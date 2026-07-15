use super::*;
use tracing::warn;

impl SnapshotCore {
    pub async fn rollback_session(&mut self, session_id: &str) -> SnapshotResult<Vec<PathBuf>> {
        info!("Rolling back session: session_id={}", session_id);
        let Some(session) = self.sessions.get(session_id) else {
            return Ok(Vec::new());
        };

        let mut to_rollback: Vec<FileOperation> = session.all_operations_iter().cloned().collect();
        to_rollback.sort_by_key(|op| (op.turn_index, op.seq_in_turn));
        to_rollback.reverse();

        let restored = self.apply_rollback_ops(&to_rollback).await?;

        self.sessions.remove(session_id);
        self.delete_session_file(session_id).await?;
        self.rebuild_operation_index();
        Ok(restored)
    }

    /// Rollback to the start of `target_turn` (undo target_turn and later turns).
    pub async fn rollback_to_turn(&mut self, session_id: &str, target_turn: usize) -> SnapshotResult<Vec<PathBuf>> {
        info!(
            "Rolling back to turn: session_id={} turn_index={}",
            session_id, target_turn
        );
        let Some(session) = self.sessions.get(session_id) else {
            return Ok(Vec::new());
        };

        let mut to_rollback: Vec<FileOperation> = session
            .all_operations_iter()
            .filter(|op| op.turn_index >= target_turn)
            .cloned()
            .collect();
        to_rollback.sort_by_key(|op| (op.turn_index, op.seq_in_turn));
        to_rollback.reverse();

        let restored = self.apply_rollback_ops(&to_rollback).await?;

        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| SnapshotError::SessionNotFound(session_id.to_string()))?;
        session.turns.retain(|turn_index, _| *turn_index < target_turn);
        session.last_updated = SystemTime::now();
        self.persist_session(session_id).await?;
        self.rebuild_operation_index();

        Ok(restored)
    }

    pub async fn cleanup_session(&mut self, session_id: &str) -> SnapshotResult<()> {
        let snapshot_ids_to_delete: Vec<String> = if let Some(session) = self.sessions.get(session_id) {
            session
                .all_operations_iter()
                .flat_map(|op| {
                    let mut ids = Vec::new();
                    if let Some(ref id) = op.before_snapshot_id {
                        if !id.starts_with("empty_snapshot_") {
                            ids.push(id.clone());
                        }
                    }
                    if let Some(ref id) = op.after_snapshot_id {
                        if !id.starts_with("empty_snapshot_") {
                            ids.push(id.clone());
                        }
                    }
                    ids
                })
                .collect()
        } else {
            Vec::new()
        };

        for snapshot_id in &snapshot_ids_to_delete {
            if let Err(e) = self.snapshot_system.delete_snapshot(snapshot_id).await {
                warn!("Failed to delete snapshot: snapshot_id={} error={}", snapshot_id, e);
            }
        }

        if !snapshot_ids_to_delete.is_empty() {
            info!(
                "Cleaned up {} snapshot files: session_id={}",
                snapshot_ids_to_delete.len(),
                session_id
            );
        }

        self.sessions.remove(session_id);

        self.delete_session_file(session_id).await?;

        self.rebuild_operation_index();

        Ok(())
    }

    pub async fn cleanup_file_session(&mut self, session_id: &str, file_path: &Path) -> SnapshotResult<()> {
        let Some(session) = self.sessions.get_mut(session_id) else {
            return Ok(());
        };

        for turn in session.turns.values_mut() {
            turn.operations
                .retain(|op| !Self::operation_matches_file_path(op, file_path));
        }
        session.turns.retain(|_, t| !t.operations.is_empty());
        session.last_updated = SystemTime::now();
        self.persist_session(session_id).await?;
        self.rebuild_operation_index();
        Ok(())
    }

    pub async fn rollback_file_session(&mut self, session_id: &str, file_path: &Path) -> SnapshotResult<Vec<PathBuf>> {
        let Some(session) = self.sessions.get(session_id) else {
            return Ok(Vec::new());
        };

        let mut to_rollback: Vec<FileOperation> = session
            .all_operations_iter()
            .filter(|op| Self::operation_matches_file_path(op, file_path))
            .cloned()
            .collect();
        to_rollback.sort_by_key(|op| (op.turn_index, op.seq_in_turn));
        to_rollback.reverse();

        let restored = self.apply_rollback_ops(&to_rollback).await?;
        self.cleanup_file_session(session_id, file_path).await?;
        Ok(restored)
    }

    pub(crate) fn operation_matches_file_path(op: &FileOperation, file_path: &Path) -> bool {
        op.file_path == file_path
            || op.path_before.as_deref() == Some(file_path)
            || op.path_after.as_deref() == Some(file_path)
    }

    async fn apply_rollback_ops(&self, ops: &[FileOperation]) -> SnapshotResult<Vec<PathBuf>> {
        let mut restored_files: Vec<PathBuf> = Vec::new();

        for op in ops {
            let before_path = op.path_before.as_ref().unwrap_or(&op.file_path).to_path_buf();
            let after_path = op.path_after.as_ref().unwrap_or(&op.file_path).to_path_buf();

            if before_path != after_path && after_path.exists() {
                if let Err(e) = tokio::fs::remove_file(&after_path).await {
                    warn!("Failed to delete after_path: path={} error={}", after_path.display(), e);
                }
            }

            match op.before_snapshot_id.as_deref() {
                None => {
                    if after_path.exists() {
                        if let Err(e) = tokio::fs::remove_file(&after_path).await {
                            warn!("Failed to delete file: path={} error={}", after_path.display(), e);
                        } else {
                            restored_files.push(after_path.clone());
                        }
                    }
                }
                Some(snapshot_id) if snapshot_id.starts_with("empty_snapshot_") => {
                    if after_path.exists() {
                        let _ = tokio::fs::remove_file(&after_path).await;
                        restored_files.push(after_path.clone());
                    }
                }
                Some(snapshot_id) => {
                    self.snapshot_system.restore_file(snapshot_id, &before_path).await?;
                    restored_files.push(before_path.clone());
                }
            }
        }

        Ok(super::format::unique_paths(restored_files.into_iter()))
    }
}
