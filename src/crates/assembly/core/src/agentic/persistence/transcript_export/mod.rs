//! Transcript Export sub-handlers (Round 10b split)
//!
//! Owns `export_session_transcript` and the render/format helpers used to
//! build the markdown body. The 191-line god method from R10a has been
//! split into `export_session_transcript` (orchestrator) +
//! `prepare_export_options` + `build_export_sections` + `render_transcript_body`
//! + `write_export_files`.
//!
//! This file owns the transcript-export-related methods of `PersistenceManager`
//! via the Rust multi-impl pattern: each sibling file declares its own
//! `impl PersistenceManager` block, and Rust links them automatically.
//! Visibility for shared helpers is promoted to `pub(super)` so other
//! siblings can call them.

pub mod te_build;
pub mod te_format;
pub mod te_types;
pub mod te_write;

#[cfg(test)]
mod tests {
    use crate::agentic::persistence::manager::PersistenceManager;
    use crate::infrastructure::PathManager;
    use crate::service::session::{DialogTurnData, SessionMetadata, SessionTranscriptExportOptions, UserMessageData};
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
    async fn export_session_transcript_handles_first_selected_turn_without_panicking() {
        let workspace = TestWorkspace::new();
        let manager = PersistenceManager::new(workspace.path_manager()).expect("persistence manager");
        let session_id = Uuid::new_v4().to_string();

        let metadata = SessionMetadata::new(
            session_id.clone(),
            "Transcript test".to_string(),
            "agent".to_string(),
            "model".to_string(),
        );
        manager
            .save_session_metadata(workspace.path(), &metadata)
            .await
            .expect("metadata should save");

        let user_message = UserMessageData {
            id: "user-1".to_string(),
            content: "hello transcript".to_string(),
            timestamp: 0,
            metadata: None,
        };
        let mut turn = DialogTurnData::new("turn-1".to_string(), 0, session_id.clone(), user_message);
        turn.mark_completed();
        manager
            .save_dialog_turn(workspace.path(), &turn)
            .await
            .expect("turn should save");

        let export = manager
            .export_session_transcript(
                workspace.path(),
                &session_id,
                &SessionTranscriptExportOptions::default(),
            )
            .await
            .expect("transcript export should succeed");

        assert_eq!(export.turn_count, 1);
        assert_eq!(export.index.len(), 1);

        let transcript = std::fs::read_to_string(&export.transcript_path).expect("transcript file should be readable");
        assert!(transcript.contains("## Turn 0"));
        assert!(transcript.contains("hello transcript"));
    }
}
