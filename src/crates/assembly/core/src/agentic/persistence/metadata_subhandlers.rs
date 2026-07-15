//! Session Metadata sub-handlers (Round 10a split)
//!
//! List and persist session metadata files; manage the per-metadata update lock registry.
//!
//! This file owns the session metadata-related methods of `PersistenceManager`
//! via the Rust multi-impl pattern: each sibling file declares its own
//! `impl PersistenceManager` block, and Rust links them automatically.
//! Visibility for shared helpers is promoted to `pub(super)` so other
//! siblings can call them.

use super::manager::PersistenceManager;
use crate::agentic::core::{
    strip_prompt_markup, CompressionState, InMemoryRelationship, Message, MessageContent, Session, SessionConfig,
    SessionState, SessionSummary,
};
use crate::agentic::session::{SessionPromptCache, PROMPT_CACHE_SCHEMA_VERSION};
use crate::agentic::skill_agent_snapshot::TurnSkillAgentSnapshot;
use crate::infrastructure::PathManager;
use crate::service::remote_ssh::workspace_state::{resolve_workspace_session_identity, LOCAL_WORKSPACE_SSH_HOST};
use crate::service::session::{
    DialogTurnData, SessionMetadata, SessionTranscriptExport, SessionTranscriptExportOptions,
    SessionTranscriptIndexEntry, ToolItemData, TranscriptLineRange, SESSION_STORAGE_SCHEMA_VERSION,
};
use crate::service::workspace_runtime::WorkspaceRuntimeService;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::timing::elapsed_ms_u64;
use futures::{stream, StreamExt};
use northhing_runtime_ports::{SessionTurnLoadRequest, SessionTurnLoadTiming};
use northhing_services_core::session::SessionMetadataPage;
use northhing_services_core::{
    json_store::{JsonFileStore, JsonFileStoreError},
    session::{
        build_session_metadata as build_persisted_session_metadata, empty_session_metadata_page,
        refresh_session_metadata_from_turns, try_refresh_session_metadata_for_saved_turn, SessionMetadataBuildFacts,
        SessionMetadataStore, SessionMetadataStoreError, SessionStorageLayout,
    },
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

impl PersistenceManager {
    pub(super) fn session_metadata_store(&self, workspace_path: &Path) -> SessionMetadataStore {
        SessionMetadataStore::new(self.project_sessions_dir(workspace_path))
    }

    pub async fn list_session_metadata(&self, workspace_path: &Path) -> NortHingResult<Vec<SessionMetadata>> {
        if !workspace_path.exists() {
            return Ok(Vec::new());
        }

        if self.existing_project_sessions_dir(workspace_path).is_none() {
            return Ok(Vec::new());
        }

        self.session_metadata_store(workspace_path)
            .list_metadata()
            .await
            .map_err(Self::session_metadata_store_error)
    }

    pub async fn list_session_metadata_page(
        &self,
        workspace_path: &Path,
        cursor: Option<&str>,
        limit: usize,
    ) -> NortHingResult<SessionMetadataPage> {
        if !workspace_path.exists() {
            return Ok(empty_session_metadata_page());
        }

        if self.existing_project_sessions_dir(workspace_path).is_none() {
            return Ok(empty_session_metadata_page());
        }

        self.session_metadata_store(workspace_path)
            .list_metadata_page(cursor, limit)
            .await
            .map_err(Self::session_metadata_store_error)
    }

    pub async fn list_session_metadata_including_internal(
        &self,
        workspace_path: &Path,
    ) -> NortHingResult<Vec<SessionMetadata>> {
        if !workspace_path.exists() {
            return Ok(Vec::new());
        }

        if self.existing_project_sessions_dir(workspace_path).is_none() {
            return Ok(Vec::new());
        }

        self.session_metadata_store(workspace_path)
            .list_metadata_including_internal()
            .await
            .map_err(Self::session_metadata_store_error)
    }

    pub async fn save_session_metadata(&self, workspace_path: &Path, metadata: &SessionMetadata) -> NortHingResult<()> {
        self.ensure_runtime_for_write(workspace_path).await?;
        self.session_metadata_store(workspace_path)
            .save_metadata(metadata)
            .await
            .map_err(Self::session_metadata_store_error)
    }

    pub async fn load_session_metadata(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<Option<SessionMetadata>> {
        self.session_metadata_store(workspace_path)
            .load_metadata(session_id)
            .await
            .map_err(Self::session_metadata_store_error)
    }
}

#[cfg(test)]
mod tests {
    use super::PersistenceManager;
    use crate::agentic::core::SessionKind;
    use crate::infrastructure::PathManager;
    use crate::service::session::{
        SessionMetadata, SessionRelationship, SessionRelationshipKind, StoredSessionIndexFile,
    };
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Instant;
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
    async fn subagent_session_kind_is_hidden_from_visible_session_index() {
        let workspace = TestWorkspace::new();
        let manager = PersistenceManager::new(workspace.path_manager()).expect("persistence manager");

        let mut metadata = SessionMetadata::new(
            Uuid::new_v4().to_string(),
            "Subagent: repo sweep".to_string(),
            "Explore".to_string(),
            "model".to_string(),
        );
        metadata.session_kind = SessionKind::Subagent;

        manager
            .save_session_metadata(workspace.path(), &metadata)
            .await
            .expect("metadata should save");

        let visible = manager
            .list_session_metadata(workspace.path())
            .await
            .expect("visible metadata should load");
        let raw = manager
            .list_session_metadata_including_internal(workspace.path())
            .await
            .expect("raw metadata should load");

        assert!(visible.is_empty());
        assert_eq!(raw.len(), 1);
        assert!(raw[0].is_subagent());
    }

    #[tokio::test]
    async fn legacy_leaked_subagent_is_hidden_from_visible_session_index() {
        let workspace = TestWorkspace::new();
        let manager = PersistenceManager::new(workspace.path_manager()).expect("persistence manager");

        let mut metadata = SessionMetadata::new(
            Uuid::new_v4().to_string(),
            "Subagent: stale task".to_string(),
            "Explore".to_string(),
            "model".to_string(),
        );
        metadata.created_by = Some("session-parent".to_string());

        manager
            .save_session_metadata(workspace.path(), &metadata)
            .await
            .expect("metadata should save");

        let visible = manager
            .list_session_metadata(workspace.path())
            .await
            .expect("visible metadata should load");
        let raw = manager
            .list_session_metadata_including_internal(workspace.path())
            .await
            .expect("raw metadata should load");

        assert!(visible.is_empty());
        assert_eq!(raw.len(), 1);
        assert!(raw[0].is_legacy_leaked_subagent_candidate());
    }

    #[tokio::test]
    async fn list_session_metadata_page_returns_visible_top_level_page_with_children() {
        let workspace = TestWorkspace::new();
        let manager = PersistenceManager::new(workspace.path_manager()).expect("persistence manager");

        for index in 0..12 {
            let mut metadata = SessionMetadata::new(
                format!("parent-{index}"),
                format!("Parent {index}"),
                "agent".to_string(),
                "model".to_string(),
            );
            metadata.last_active_at = 1_000 + index;
            manager
                .save_session_metadata(workspace.path(), &metadata)
                .await
                .expect("parent metadata should save");
        }

        let mut child = SessionMetadata::new(
            "child-latest".to_string(),
            "Child latest".to_string(),
            "agent".to_string(),
            "model".to_string(),
        );
        child.last_active_at = 2_000;
        child.relationship = Some(SessionRelationship {
            kind: Some(SessionRelationshipKind::Btw),
            parent_session_id: Some("parent-11".to_string()),
            ..Default::default()
        });
        manager
            .save_session_metadata(workspace.path(), &child)
            .await
            .expect("child metadata should save");

        let page = manager
            .list_session_metadata_page(workspace.path(), None, 5)
            .await
            .expect("session metadata page should load");
        let session_ids = page
            .sessions
            .iter()
            .map(|metadata| metadata.session_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(page.total_top_level_count, 12);
        assert_eq!(page.loaded_top_level_count, 5);
        assert!(page.next_cursor.is_some());
        assert!(page.has_more);
        assert_eq!(
            session_ids,
            vec![
                "parent-11",
                "child-latest",
                "parent-10",
                "parent-9",
                "parent-8",
                "parent-7",
            ]
        );

        let second_page = manager
            .list_session_metadata_page(workspace.path(), page.next_cursor.as_deref(), 5)
            .await
            .expect("second session metadata page should load");
        let second_page_session_ids = second_page
            .sessions
            .iter()
            .map(|metadata| metadata.session_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(second_page.loaded_top_level_count, 5);
        assert_eq!(
            second_page_session_ids,
            vec!["parent-6", "parent-5", "parent-4", "parent-3", "parent-2"]
        );
    }

    #[tokio::test]
    async fn list_session_metadata_page_rebuilds_stale_visible_page_entry() {
        let workspace = TestWorkspace::new();
        let manager = PersistenceManager::new(workspace.path_manager()).expect("persistence manager");

        let mut older = SessionMetadata::new(
            "older-session".to_string(),
            "Older session".to_string(),
            "agent".to_string(),
            "model".to_string(),
        );
        older.last_active_at = 1_000;
        let mut newer = SessionMetadata::new(
            "newer-session".to_string(),
            "Newer session".to_string(),
            "agent".to_string(),
            "model".to_string(),
        );
        newer.last_active_at = 2_000;

        manager
            .save_session_metadata(workspace.path(), &older)
            .await
            .expect("older metadata should save");
        manager
            .save_session_metadata(workspace.path(), &newer)
            .await
            .expect("newer metadata should save");

        let mut missing = SessionMetadata::new(
            "missing-session".to_string(),
            "Missing session".to_string(),
            "agent".to_string(),
            "model".to_string(),
        );
        missing.last_active_at = 3_000;

        let stale_index = StoredSessionIndexFile::new(0, vec![missing, older]);
        manager
            .write_json_atomic(&manager.index_path(workspace.path()), &stale_index)
            .await
            .expect("stale index should be written");

        let page = manager
            .list_session_metadata_page(workspace.path(), None, 5)
            .await
            .expect("session metadata page should rebuild stale index");
        let session_ids = page
            .sessions
            .iter()
            .map(|metadata| metadata.session_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(page.total_top_level_count, 2);
        assert_eq!(session_ids, vec!["newer-session", "older-session"]);
    }

    #[tokio::test]
    #[ignore = "local performance benchmark; prints timing data only"]
    async fn bench_session_metadata_page_vs_full_list() {
        const SESSION_COUNT: usize = 1_000;
        const ITERATIONS: usize = 10;

        let workspace = TestWorkspace::new();
        let manager = PersistenceManager::new(workspace.path_manager()).expect("persistence manager");

        for index in 0..SESSION_COUNT {
            let mut metadata = SessionMetadata::new(
                format!("bench-parent-{index}"),
                format!("Bench parent {index}"),
                "agent".to_string(),
                "model".to_string(),
            );
            metadata.last_active_at = 1_000_000 + index as u64;
            manager
                .save_session_metadata(workspace.path(), &metadata)
                .await
                .expect("benchmark metadata should save");
        }

        manager
            .list_session_metadata(workspace.path())
            .await
            .expect("warm full list should load");
        manager
            .list_session_metadata_page(workspace.path(), None, 5)
            .await
            .expect("warm page should load");

        let mut full_list_total_ms = 0.0;
        for _ in 0..ITERATIONS {
            let started = Instant::now();
            let full = manager
                .list_session_metadata(workspace.path())
                .await
                .expect("full list should load");
            assert_eq!(full.len(), SESSION_COUNT);
            full_list_total_ms += started.elapsed().as_secs_f64() * 1000.0;
        }

        let mut page_total_ms = 0.0;
        for _ in 0..ITERATIONS {
            let started = Instant::now();
            let page = manager
                .list_session_metadata_page(workspace.path(), None, 5)
                .await
                .expect("page should load");
            assert_eq!(page.loaded_top_level_count, 5);
            assert_eq!(page.total_top_level_count, SESSION_COUNT);
            page_total_ms += started.elapsed().as_secs_f64() * 1000.0;
        }

        let full_avg_ms = full_list_total_ms / ITERATIONS as f64;
        let page_avg_ms = page_total_ms / ITERATIONS as f64;
        println!(
            "session_metadata_bench sessions={} iterations={} full_list_avg_ms={:.3} page5_avg_ms={:.3} speedup={:.1}x",
            SESSION_COUNT,
            ITERATIONS,
            full_avg_ms,
            page_avg_ms,
            full_avg_ms / page_avg_ms.max(0.001)
        );
    }

    #[tokio::test]
    async fn saving_session_metadata_ensures_runtime_layout_before_writing() {
        let workspace = TestWorkspace::new();
        let manager = PersistenceManager::new(workspace.path_manager()).expect("persistence manager");

        let metadata = SessionMetadata::new(
            Uuid::new_v4().to_string(),
            "Runtime ensure".to_string(),
            "agent".to_string(),
            "model".to_string(),
        );

        manager
            .save_session_metadata(workspace.path(), &metadata)
            .await
            .expect("metadata should save");

        let runtime = manager.runtime_service().context_for_local_workspace(workspace.path());
        assert!(runtime.runtime_root.exists());
        assert!(runtime.sessions_dir.exists());
        assert!(runtime.snapshot_by_hash_dir.exists());
        assert!(runtime.snapshot_metadata_dir.exists());
        assert!(runtime.snapshot_operations_dir.exists());
        assert!(runtime.plans_dir.exists());
        assert!(runtime.layout_state_file.exists());
    }
}
