//! Turn IO sub-handlers (Round 10b split)
//!
//! Owns single-turn I/O and the related delete operations
//! (save/load/delete dialog turn, delete turns after a given index, etc).
//!
//! This file owns the turn-io-related methods of `PersistenceManager`
//! via the Rust multi-impl pattern: each sibling file declares its own
//! `impl PersistenceManager` block, and Rust links them automatically.
//! Visibility for shared helpers is promoted to `pub(super)` so other
//! siblings can call them.

use super::manager::PersistenceManager;
use crate::service::session::{DialogTurnData, SESSION_STORAGE_SCHEMA_VERSION};
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_services_core::session::{
    compute_turn_checksum, read_turn_checksum_sidecar, refresh_session_metadata_from_turns,
    try_refresh_session_metadata_for_saved_turn, verify_turn_checksum, write_turn_checksum_sidecar,
    TurnChecksumError,
};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};
use tokio::fs;
use tracing::{debug, warn};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct StoredDialogTurnFile {
    pub(crate) schema_version: u32,
    #[serde(flatten)]
    pub(crate) turn: DialogTurnData,
}

impl PersistenceManager {
    pub async fn save_dialog_turn(&self, workspace_path: &Path, turn: &DialogTurnData) -> NortHingResult<()> {
        let save_started_at = Instant::now();
        self.ensure_runtime_for_write(workspace_path).await?;
        let metadata_update_lock = self
            .get_session_metadata_update_lock(workspace_path, &turn.session_id)
            .await;
        let _metadata_update_guard = metadata_update_lock.lock().await;
        let mut metadata = self
            .load_session_metadata(workspace_path, &turn.session_id)
            .await?
            .ok_or_else(|| NortHingError::NotFound(format!("Session metadata not found: {}", turn.session_id)))?;
        self.ensure_turns_dir(workspace_path, &turn.session_id).await?;

        let previous_turn = match self
            .load_dialog_turn(workspace_path, &turn.session_id, turn.turn_index)
            .await
        {
            Ok(turn) => turn,
            Err(error) => {
                warn!(
                    "Failed to load existing dialog turn before save; falling back to full metadata refresh: session_id={} turn_index={} error={}",
                    turn.session_id, turn.turn_index, error
                );
                None
            }
        };
        let previous_turn_load_failed = previous_turn.is_none()
            && self
                .turn_path(workspace_path, &turn.session_id, turn.turn_index)
                .exists();

        let file = StoredDialogTurnFile {
            schema_version: SESSION_STORAGE_SCHEMA_VERSION,
            turn: turn.clone(),
        };
        let turn_path = self.turn_path(workspace_path, &turn.session_id, turn.turn_index);
        let write_started_at = Instant::now();
        self.write_json_atomic(&turn_path, &file).await?;
        // B-3: write per-turn SHA-256 checksum sidecar (atomic, write-time
        // defense against silent corruption; verified on load).
        let checksum = compute_turn_checksum(turn);
        write_turn_checksum_sidecar(&turn_path, &checksum)
            .await
            .map_err(|e| NortHingError::Validation(format!("turn checksum sidecar write: {}", e)))?;
        let write_duration = write_started_at.elapsed();

        let last_active_at = turn
            .end_time
            .unwrap_or_else(|| Self::system_time_to_unix_ms(SystemTime::now()));
        let mut metadata_refresh_mode = "incremental";
        let workspace_path_text = workspace_path.to_string_lossy();
        if previous_turn_load_failed
            || !try_refresh_session_metadata_for_saved_turn(
                &mut metadata,
                workspace_path_text.as_ref(),
                previous_turn.as_ref(),
                turn,
                last_active_at,
            )
        {
            metadata_refresh_mode = "full_scan";
            let turns = self.load_session_turns(workspace_path, &turn.session_id).await?;
            refresh_session_metadata_from_turns(&mut metadata, workspace_path_text.as_ref(), &turns, last_active_at);
        }

        let metadata_started_at = Instant::now();
        self.save_session_metadata(workspace_path, &metadata).await?;
        let metadata_duration = metadata_started_at.elapsed();
        let total_duration = save_started_at.elapsed();
        if total_duration >= Duration::from_millis(80) || metadata_refresh_mode == "full_scan" {
            debug!(
                "Saved dialog turn: session_id={} turn_index={} metadata_refresh={} write_duration_ms={} metadata_duration_ms={} total_duration_ms={}",
                turn.session_id,
                turn.turn_index,
                metadata_refresh_mode,
                write_duration.as_millis(),
                metadata_duration.as_millis(),
                total_duration.as_millis()
            );
        }

        Ok(())
    }

    pub async fn load_dialog_turn(
        &self,
        workspace_path: &Path,
        session_id: &str,
        turn_index: usize,
    ) -> NortHingResult<Option<DialogTurnData>> {
        let turn_path = self.turn_path(workspace_path, session_id, turn_index);
        let file = match self
            .read_json_optional::<StoredDialogTurnFile>(&turn_path)
            .await?
        {
            Some(f) => f,
            None => return Ok(None),
        };
        // B-3: verify per-turn checksum sidecar (lazy on read).
        // Pre-checksum turns return None from read_turn_checksum_sidecar
        // and are accepted as back-compat (no error).
        let stored_checksum = read_turn_checksum_sidecar(&turn_path)
            .await
            .map_err(|e| NortHingError::Validation(format!("turn checksum sidecar read: {}", e)))?;
        if let Some(stored) = stored_checksum {
            verify_turn_checksum(&file.turn, &stored).map_err(|e| match e {
                TurnChecksumError::Mismatch { turn_id, expected, got } => NortHingError::Validation(format!(
                    "turn checksum mismatch: turn_id={} expected={:?} got={:?}",
                    turn_id, expected, got
                )),
                other => NortHingError::Validation(format!("turn checksum error: {}", other)),
            })?;
        } else {
            // No sidecar (pre-checksum turn). Accept for back-compat;
            // this is the lazy backfill path: future saves will populate
            // the sidecar.
            debug!(
                "No checksum sidecar for turn (pre-checksum turn, accepted as back-compat): session_id={} turn_index={}",
                session_id, turn_index
            );
        }
        Ok(Some(file.turn))
    }

    pub async fn delete_dialog_turns_from(
        &self,
        workspace_path: &Path,
        session_id: &str,
        turn_index: usize,
    ) -> NortHingResult<()> {
        if !self.turns_dir(workspace_path, session_id).exists() {
            return Ok(());
        }

        self.session_layout(workspace_path)
            .delete_indexed_turn_paths_from(session_id, turn_index)
            .await
            .map_err(|e| NortHingError::io(format!("Failed to delete dialog turn files: {}", e)))?;

        if let Some(mut metadata) = self.load_session_metadata(workspace_path, session_id).await? {
            let turns = self.load_session_turns(workspace_path, session_id).await?;
            let workspace_path_text = workspace_path.to_string_lossy();
            refresh_session_metadata_from_turns(
                &mut metadata,
                workspace_path_text.as_ref(),
                &turns,
                Self::system_time_to_unix_ms(SystemTime::now()),
            );
            self.save_session_metadata(workspace_path, &metadata).await?;
        }

        Ok(())
    }

    pub async fn delete_turns_after(
        &self,
        workspace_path: &Path,
        session_id: &str,
        turn_index: usize,
    ) -> NortHingResult<usize> {
        let turns = self.load_session_turns(workspace_path, session_id).await?;
        let mut deleted = 0usize;

        for turn in turns.into_iter().filter(|value| value.turn_index > turn_index) {
            let path = self.turn_path(workspace_path, session_id, turn.turn_index);
            if path.exists() {
                fs::remove_file(&path)
                    .await
                    .map_err(|e| NortHingError::io(format!("Failed to delete turn file: {}", e)))?;
                deleted += 1;
            }
        }

        if let Some(mut metadata) = self.load_session_metadata(workspace_path, session_id).await? {
            let remaining_turns = self.load_session_turns(workspace_path, session_id).await?;
            let workspace_path_text = workspace_path.to_string_lossy();
            refresh_session_metadata_from_turns(
                &mut metadata,
                workspace_path_text.as_ref(),
                &remaining_turns,
                Self::system_time_to_unix_ms(SystemTime::now()),
            );
            self.save_session_metadata(workspace_path, &metadata).await?;
        }

        Ok(deleted)
    }

    pub async fn delete_turns_from(
        &self,
        workspace_path: &Path,
        session_id: &str,
        turn_index: usize,
    ) -> NortHingResult<usize> {
        let turns = self.load_session_turns(workspace_path, session_id).await?;
        let mut deleted = 0usize;

        for turn in turns.into_iter().filter(|value| value.turn_index >= turn_index) {
            let path = self.turn_path(workspace_path, session_id, turn.turn_index);
            if path.exists() {
                fs::remove_file(&path)
                    .await
                    .map_err(|e| NortHingError::io(format!("Failed to delete turn file: {}", e)))?;
                deleted += 1;
            }
        }

        if let Some(mut metadata) = self.load_session_metadata(workspace_path, session_id).await? {
            let remaining_turns = self.load_session_turns(workspace_path, session_id).await?;
            let workspace_path_text = workspace_path.to_string_lossy();
            refresh_session_metadata_from_turns(
                &mut metadata,
                workspace_path_text.as_ref(),
                &remaining_turns,
                Self::system_time_to_unix_ms(SystemTime::now()),
            );
            self.save_session_metadata(workspace_path, &metadata).await?;
        }

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::{PersistenceManager, StoredDialogTurnFile};
    use crate::agentic::core::{Session, SessionConfig};
    use crate::infrastructure::PathManager;
    use crate::service::session::{DialogTurnData, ModelRoundData, SessionMetadata, TextItemData, UserMessageData};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use uuid::Uuid;

    struct TestWorkspace {
        path: PathBuf,
    }

    impl TestWorkspace {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("northhing-session-transcript-test-{}", Uuid::new_v4()));
            std::fs::create_dir_all(&path).expect("test workspace should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn path_manager(&self) -> Arc<PathManager> {
            Arc::new(PathManager::with_user_root_for_tests(self.path.join("user-root")))
        }
    }

    impl Drop for TestWorkspace {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn user_message(content: &str) -> UserMessageData {
        UserMessageData {
            id: format!("user-{}", content),
            content: content.to_string(),
            timestamp: 0,
            metadata: None,
        }
    }

    fn text_item(id: &str, content: &str) -> TextItemData {
        TextItemData {
            id: id.to_string(),
            content: content.to_string(),
            is_streaming: false,
            timestamp: 0,
            is_markdown: true,
            order_index: None,
            is_subagent_item: None,
            parent_task_tool_id: None,
            subagent_session_id: None,
            status: None,
        }
    }

    fn round_with_text(turn_id: &str, text_items: Vec<TextItemData>) -> ModelRoundData {
        ModelRoundData {
            id: format!("round-{}", turn_id),
            turn_id: turn_id.to_string(),
            round_index: 0,
            timestamp: 0,
            text_items,
            tool_items: Vec::new(),
            thinking_items: Vec::new(),
            start_time: 0,
            end_time: Some(0),
            duration_ms: Some(0),
            provider_id: None,
            model_id: None,
            model_alias: None,
            first_chunk_ms: None,
            first_visible_output_ms: None,
            stream_duration_ms: None,
            attempt_count: None,
            failure_category: None,
            token_details: None,
            status: "completed".to_string(),
        }
    }

    #[tokio::test]
    async fn save_dialog_turn_updates_metadata_without_scanning_unrelated_turn_files() {
        let workspace = TestWorkspace::new();
        let manager =
            PersistenceManager::new(Arc::new(PathManager::new().expect("path manager"))).expect("persistence manager");
        let session_id = Uuid::new_v4().to_string();
        let session = Session::new_with_id(
            session_id.clone(),
            "Incremental metadata".to_string(),
            "agent".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        );

        manager
            .save_session(workspace.path(), &session)
            .await
            .expect("session should save");

        let mut turn_0 = DialogTurnData::new("turn-0".to_string(), 0, session_id.clone(), user_message("first"));
        turn_0
            .model_rounds
            .push(round_with_text("turn-0", vec![text_item("text-0", "first response")]));
        turn_0.mark_completed();
        manager
            .save_dialog_turn(workspace.path(), &turn_0)
            .await
            .expect("first turn should save");

        let mut turn_1 = DialogTurnData::new("turn-1".to_string(), 1, session_id.clone(), user_message("second"));
        turn_1
            .model_rounds
            .push(round_with_text("turn-1", vec![text_item("text-1", "second response")]));
        turn_1.mark_completed();
        manager
            .save_dialog_turn(workspace.path(), &turn_1)
            .await
            .expect("second turn should save");

        std::fs::write(manager.turn_path(workspace.path(), &session_id, 0), "{ not valid json")
            .expect("old turn file should be replaceable for test");

        turn_1.model_rounds[0]
            .text_items
            .push(text_item("text-2", "additional response"));
        manager
            .save_dialog_turn(workspace.path(), &turn_1)
            .await
            .expect("saving current turn should not scan unrelated old turn files");

        let metadata = manager
            .load_session_metadata(workspace.path(), &session_id)
            .await
            .expect("metadata should load")
            .expect("metadata should exist");
        assert_eq!(metadata.turn_count, 2);
        assert_eq!(metadata.message_count, 5);
    }

    #[tokio::test]
    async fn concurrent_dialog_turn_saves_keep_metadata_counts_consistent() {
        let workspace = TestWorkspace::new();
        let manager =
            PersistenceManager::new(Arc::new(PathManager::new().expect("path manager"))).expect("persistence manager");
        let session_id = Uuid::new_v4().to_string();
        let session = Session::new_with_id(
            session_id.clone(),
            "Concurrent metadata".to_string(),
            "agent".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        );

        manager
            .save_session(workspace.path(), &session)
            .await
            .expect("session should save");

        let mut turn_0 = DialogTurnData::new("turn-0".to_string(), 0, session_id.clone(), user_message("first"));
        turn_0
            .model_rounds
            .push(round_with_text("turn-0", vec![text_item("text-0", "first response")]));
        turn_0.mark_completed();
        manager
            .save_dialog_turn(workspace.path(), &turn_0)
            .await
            .expect("first turn should save");

        let mut turn_1 = DialogTurnData::new("turn-1".to_string(), 1, session_id.clone(), user_message("second"));
        turn_1
            .model_rounds
            .push(round_with_text("turn-1", vec![text_item("text-1", "second response")]));
        turn_1.mark_completed();
        manager
            .save_dialog_turn(workspace.path(), &turn_1)
            .await
            .expect("second turn should save");

        let mut updated_turn_0 = turn_0.clone();
        updated_turn_0.model_rounds[0]
            .text_items
            .push(text_item("text-0b", "first follow-up"));

        let mut updated_turn_1 = turn_1.clone();
        updated_turn_1.model_rounds[0]
            .text_items
            .push(text_item("text-1b", "second follow-up"));
        updated_turn_1.model_rounds[0]
            .text_items
            .push(text_item("text-1c", "second final"));

        let (first_result, second_result) = tokio::join!(
            manager.save_dialog_turn(workspace.path(), &updated_turn_0),
            manager.save_dialog_turn(workspace.path(), &updated_turn_1)
        );
        first_result.expect("first concurrent save should succeed");
        second_result.expect("second concurrent save should succeed");

        let metadata = manager
            .load_session_metadata(workspace.path(), &session_id)
            .await
            .expect("metadata should load")
            .expect("metadata should exist");
        assert_eq!(metadata.turn_count, 2);
        assert_eq!(metadata.message_count, 7);
    }
}
