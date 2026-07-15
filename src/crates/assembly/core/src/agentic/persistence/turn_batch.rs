//! Turn batch sub-handlers (Round 10b split).
//!
//! Owns the path-listing + concurrent-read helpers shared by both the
//! "session loader" and "turns loader" halves, plus the 3 unit tests.
//!
//! This file is the entry point of the `turn_batch` module — sibling
//! files (`session_loader.rs` and `turns_loader.rs`) declare their own
//! `impl PersistenceManager` blocks, and Rust links them automatically.
//! Visibility for shared helpers is `pub(super)` so siblings can call
//! them.
//!
//! R73-2 split: this entry was 694 lines (god file) with a single
//! `impl PersistenceManager` block. Split into 1 entry + 2 sibling
//! sub-modules by responsibility:
//! - `session_loader` — load Session + Vec<DialogTurnData> together
//!   (4 methods, ~255 lines)
//! - `turns_loader`   — load Vec<DialogTurnData> only (3 methods, ~125 lines)
//! - This entry       — shared helpers (list_indexed_turn_paths +
//!   read_turn_paths) + ReadTurnPathsResult struct + 3 unit tests (~270 lines)

use super::manager::PersistenceManager;
use super::turn_io::StoredDialogTurnFile;
use crate::agentic::core::{Session, SessionConfig};
use crate::infrastructure::PathManager;
use crate::service::session::{
    DialogTurnData, ModelRoundData, SessionMetadata, TextItemData, UserMessageData, SESSION_STORAGE_SCHEMA_VERSION,
};
use crate::util::errors::{NortHingError, NortHingResult};
use futures::{stream, StreamExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

mod session_loader;
mod turns_loader;

const SESSION_TURN_READ_CONCURRENCY: usize = 4;

pub(super) struct ReadTurnPathsResult {
    pub(crate) turns: Vec<DialogTurnData>,
    pub(crate) missing_turn_file_count: usize,
    pub(crate) max_turn_read_duration_ms: u64,
}

impl PersistenceManager {
    pub(super) async fn list_indexed_turn_paths(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<Vec<(usize, PathBuf)>> {
        self.session_layout(workspace_path)
            .list_indexed_turn_paths(session_id)
            .await
            .map_err(|e| NortHingError::io(format!("Failed to list dialog turn files: {}", e)))
    }

    pub(super) async fn read_turn_paths(
        &self,
        indexed_paths: Vec<(usize, PathBuf)>,
    ) -> NortHingResult<ReadTurnPathsResult> {
        let mut turns = Vec::with_capacity(indexed_paths.len());
        let mut missing_turn_file_count = 0usize;
        let mut max_turn_read_duration_ms = 0u64;
        let reads = stream::iter(indexed_paths.into_iter().map(|(_, path)| {
            let manager = self;
            async move {
                let started_at = Instant::now();
                let result = manager.read_json_optional::<StoredDialogTurnFile>(&path).await;
                (result, started_at.elapsed().as_millis() as u64)
            }
        }))
        .buffered(SESSION_TURN_READ_CONCURRENCY)
        .collect::<Vec<_>>()
        .await;

        for (result, duration_ms) in reads {
            max_turn_read_duration_ms = max_turn_read_duration_ms.max(duration_ms);
            if let Some(file) = result? {
                turns.push(file.turn);
            } else {
                missing_turn_file_count += 1;
            }
        }

        Ok(ReadTurnPathsResult {
            turns,
            missing_turn_file_count,
            max_turn_read_duration_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::turn_io::StoredDialogTurnFile;
    use super::PersistenceManager;
    use crate::agentic::core::{Session, SessionConfig};
    use crate::infrastructure::PathManager;
    use crate::service::session::{
        DialogTurnData, ModelRoundData, SessionMetadata, TextItemData, UserMessageData, SESSION_STORAGE_SCHEMA_VERSION,
    };
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

    #[tokio::test]
    async fn load_session_tail_turns_returns_latest_turns_in_chronological_order() {
        let workspace = TestWorkspace::new();
        let manager = PersistenceManager::new(workspace.path_manager()).expect("persistence manager");
        let session_id = Uuid::new_v4().to_string();
        let metadata = SessionMetadata::new(
            session_id.clone(),
            "Tail turns test".to_string(),
            "agent".to_string(),
            "model".to_string(),
        );
        manager
            .save_session_metadata(workspace.path(), &metadata)
            .await
            .expect("metadata should save");

        for index in 0..5 {
            let user_message = UserMessageData {
                id: format!("user-{index}"),
                content: format!("prompt {index}"),
                timestamp: index as u64,
                metadata: None,
            };
            let mut turn = DialogTurnData::new(format!("turn-{index}"), index, session_id.clone(), user_message);
            turn.mark_completed();
            manager
                .save_dialog_turn(workspace.path(), &turn)
                .await
                .expect("turn should save");
        }

        let tail = manager
            .load_session_tail_turns(workspace.path(), &session_id, 2)
            .await
            .expect("tail turns should load");

        let turn_indices = tail.iter().map(|turn| turn.turn_index).collect::<Vec<_>>();
        let prompts = tail
            .iter()
            .map(|turn| turn.user_message.content.as_str())
            .collect::<Vec<_>>();

        assert_eq!(turn_indices, vec![3, 4]);
        assert_eq!(prompts, vec!["prompt 3", "prompt 4"]);

        let (_session, view_tail, total_turn_count) = manager
            .load_session_with_tail_turns(workspace.path(), &session_id, 2)
            .await
            .expect("tail view should load");
        let view_turn_indices = view_tail.iter().map(|turn| turn.turn_index).collect::<Vec<_>>();

        assert_eq!(view_turn_indices, vec![3, 4]);
        assert_eq!(total_turn_count, 5);
    }

    #[tokio::test]
    async fn load_session_tail_turns_uses_metadata_turn_count_as_normal_path_boundary() {
        let workspace = TestWorkspace::new();
        let manager = PersistenceManager::new(workspace.path_manager()).expect("persistence manager");
        let session_id = Uuid::new_v4().to_string();
        let metadata = SessionMetadata::new(
            session_id.clone(),
            "Tail turns boundary test".to_string(),
            "agent".to_string(),
            "model".to_string(),
        );
        manager
            .save_session_metadata(workspace.path(), &metadata)
            .await
            .expect("metadata should save");

        for index in 0..5 {
            let user_message = UserMessageData {
                id: format!("user-{index}"),
                content: format!("prompt {index}"),
                timestamp: index as u64,
                metadata: None,
            };
            let mut turn = DialogTurnData::new(format!("turn-{index}"), index, session_id.clone(), user_message);
            turn.mark_completed();
            manager
                .save_dialog_turn(workspace.path(), &turn)
                .await
                .expect("turn should save");
        }

        let orphan_user_message = UserMessageData {
            id: "user-99".to_string(),
            content: "orphan prompt".to_string(),
            timestamp: 99,
            metadata: None,
        };
        let mut orphan_turn = DialogTurnData::new("turn-99".to_string(), 99, session_id.clone(), orphan_user_message);
        orphan_turn.mark_completed();
        let orphan_file = StoredDialogTurnFile {
            schema_version: SESSION_STORAGE_SCHEMA_VERSION,
            turn: orphan_turn,
        };
        let orphan_json = serde_json::to_string_pretty(&orphan_file).expect("orphan turn should serialize");
        std::fs::write(manager.turn_path(workspace.path(), &session_id, 99), orphan_json)
            .expect("orphan turn should be written");

        let tail = manager
            .load_session_tail_turns(workspace.path(), &session_id, 2)
            .await
            .expect("tail turns should load");

        let turn_indices = tail.iter().map(|turn| turn.turn_index).collect::<Vec<_>>();
        let prompts = tail
            .iter()
            .map(|turn| turn.user_message.content.as_str())
            .collect::<Vec<_>>();

        assert_eq!(turn_indices, vec![3, 4]);
        assert_eq!(prompts, vec!["prompt 3", "prompt 4"]);

        let (_session, view_tail, total_turn_count) = manager
            .load_session_with_tail_turns(workspace.path(), &session_id, 2)
            .await
            .expect("tail view should load");
        let view_turn_indices = view_tail.iter().map(|turn| turn.turn_index).collect::<Vec<_>>();

        assert_eq!(view_turn_indices, vec![3, 4]);
        assert_eq!(total_turn_count, 5);
    }

    #[tokio::test]
    async fn load_session_with_turns_returns_session_and_persisted_turns() {
        let workspace = TestWorkspace::new();
        let manager =
            PersistenceManager::new(Arc::new(PathManager::new().expect("path manager"))).expect("persistence manager");
        let session_id = Uuid::new_v4().to_string();
        let session = Session::new_with_id(
            session_id.clone(),
            "Load once".to_string(),
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

        let user_message = UserMessageData {
            id: "user-1".to_string(),
            content: "hello once".to_string(),
            timestamp: 0,
            metadata: None,
        };
        let mut turn = DialogTurnData::new("turn-1".to_string(), 0, session_id.clone(), user_message);
        turn.mark_completed();
        manager
            .save_dialog_turn(workspace.path(), &turn)
            .await
            .expect("turn should save");

        let (loaded_session, loaded_turns) = manager
            .load_session_with_turns(workspace.path(), &session_id)
            .await
            .expect("session and turns should load together");

        assert_eq!(loaded_session.dialog_turn_ids, vec!["turn-1".to_string()]);
        assert_eq!(loaded_turns.len(), 1);
        assert_eq!(loaded_turns[0].turn_id, "turn-1");
    }
}
