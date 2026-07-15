use crate::service::snapshot::snapshot_system::FileSnapshotSystem;
use crate::service::snapshot::types::{
    DiffSummary, FileOperation, OperationType, SessionFileDiffStats, SnapshotError, SnapshotResult, ToolContext,
};
use crate::service::workspace_runtime::WorkspaceRuntimeContext;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};
use tracing::{debug, info, warn};
use uuid::Uuid;

mod capture;
mod format;
mod persist;
mod restore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub session_id: String,
    pub total_files: usize,
    pub total_turns: usize,
    pub total_changes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeEntry {
    pub session_id: String,
    pub turn_index: usize,
    pub snapshot_id: String,
    pub timestamp: SystemTime,
    pub operation_type: OperationType,
    pub tool_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeQueue {
    pub file_path: PathBuf,
    pub changes: Vec<FileChangeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TurnHistory {
    turn_index: usize,
    operations: Vec<FileOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionHistory {
    session_id: String,
    turns: BTreeMap<usize, TurnHistory>,
    created_at: SystemTime,
    last_updated: SystemTime,
}

/// Per-side size budget: above this we avoid loading baseline/disk texts for UI badge stats.
const SESSION_FILE_DIFF_STATS_MAX_SOURCE_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone)]
struct SessionFileBoundary {
    before_snapshot_id: Option<String>,
    after_snapshot_id: Option<String>,
    file_created_in_session: bool,
    file_deleted_in_session: bool,
}

impl SessionHistory {
    fn new(session_id: String) -> Self {
        let now = SystemTime::now();
        Self {
            session_id,
            turns: BTreeMap::new(),
            created_at: now,
            last_updated: now,
        }
    }

    fn ensure_turn_mut(&mut self, turn_index: usize) -> &mut TurnHistory {
        self.turns.entry(turn_index).or_insert_with(|| TurnHistory {
            turn_index,
            operations: Vec::new(),
        })
    }

    fn all_operations_iter(&self) -> impl Iterator<Item = &FileOperation> {
        self.turns.values().flat_map(|t| t.operations.iter())
    }

    // reason: all_operations_iter_mut() is reserved for the upcoming mutable-iteration API (today callers use the read-only all_operations_iter)
    fn all_operations_iter_mut(&mut self) -> impl Iterator<Item = &mut FileOperation> {
        self.turns.values_mut().flat_map(|t| t.operations.iter_mut())
    }
}

/// Snapshot core: keep operation history and snapshots (before/after).
pub struct SnapshotCore {
    sessions: HashMap<String, SessionHistory>,
    operation_index: HashMap<String, (String, usize, usize)>,
    snapshot_system: FileSnapshotSystem,
    sessions_dir: PathBuf,
}

impl SnapshotCore {
    pub fn new(runtime_context: WorkspaceRuntimeContext, snapshot_system: FileSnapshotSystem) -> Self {
        let sessions_dir = runtime_context.snapshot_operations_dir.clone();
        Self {
            sessions: HashMap::new(),
            operation_index: HashMap::new(),
            snapshot_system,
            sessions_dir,
        }
    }

    pub async fn initialize(&mut self) -> SnapshotResult<()> {
        let total_started_at = Instant::now();
        info!("Initializing operation history system");

        let snapshot_system_started_at = Instant::now();
        self.snapshot_system.initialize().await?;
        debug!(
            "Operation history initialize step completed: step=file_snapshot_system duration_ms={}",
            snapshot_system_started_at.elapsed().as_millis()
        );

        let sessions_started_at = Instant::now();
        self.load_all_sessions().await?;
        debug!(
            "Operation history initialize step completed: step=load_sessions duration_ms={}",
            sessions_started_at.elapsed().as_millis()
        );
        info!(
            "Operation history system initialized: loaded_sessions={} duration_ms={}",
            self.sessions.len(),
            total_started_at.elapsed().as_millis()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::snapshot::snapshot_system::FileSnapshotSystem;
    use crate::service::workspace_runtime::{WorkspaceRuntimeContext, WorkspaceRuntimeTarget};
    use serde_json::json;
    use std::fs;

    struct TestRuntime {
        core: SnapshotCore,
        root: PathBuf,
        workspace: PathBuf,
    }

    impl Drop for TestRuntime {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    async fn make_test_runtime(name: &str) -> TestRuntime {
        let root = std::env::temp_dir().join(format!("northhing_snapshot_core_{}_{}", name, Uuid::new_v4()));
        let workspace = root.join("workspace");
        let runtime_root = root.join("runtime");
        fs::create_dir_all(&workspace).unwrap();

        let runtime_context = WorkspaceRuntimeContext::new(
            WorkspaceRuntimeTarget::LocalWorkspace {
                workspace_root: workspace.clone(),
            },
            runtime_root,
        );
        for dir in runtime_context.required_directories() {
            fs::create_dir_all(dir).unwrap();
        }

        let snapshot_system = FileSnapshotSystem::new(runtime_context.clone());
        let mut core = SnapshotCore::new(runtime_context, snapshot_system);
        core.initialize().await.unwrap();

        TestRuntime { core, root, workspace }
    }

    #[tokio::test]
    async fn session_file_diff_stats_use_completed_session_snapshots_not_current_workspace() {
        let mut runtime = make_test_runtime("session_snapshots").await;
        let file_path = runtime.workspace.join("src/lib.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        tokio::fs::write(&file_path, "base\n").await.unwrap();

        let operation_id = runtime
            .core
            .start_file_operation(
                "session-1",
                0,
                file_path.clone(),
                OperationType::Modify,
                "Edit".to_string(),
                json!({ "file_path": "src/lib.rs" }),
                None,
            )
            .await
            .unwrap();
        tokio::fs::write(&file_path, "base\nsession\n").await.unwrap();
        runtime
            .core
            .complete_file_operation("session-1", &operation_id, 1)
            .await
            .unwrap();

        tokio::fs::write(&file_path, "base\nsession\noutside\noutside2\n")
            .await
            .unwrap();

        let stats = runtime
            .core
            .get_session_file_diff_stats("session-1", &file_path)
            .await
            .unwrap();
        assert_eq!(stats.lines_added, 1);
        assert_eq!(stats.lines_removed, 0);
        assert_eq!(stats.change_kind, "modify");

        let (before, after) = runtime.core.get_file_diff(&file_path, "session-1").await.unwrap();
        assert_eq!(before, "base\n");
        assert_eq!(after, "base\nsession\n");
    }

    #[tokio::test]
    async fn session_files_ignore_unfinished_operations() {
        let mut runtime = make_test_runtime("unfinished_ops").await;
        let file_path = runtime.workspace.join("src/lib.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        tokio::fs::write(&file_path, "base\n").await.unwrap();

        runtime
            .core
            .start_file_operation(
                "session-1",
                0,
                file_path.clone(),
                OperationType::Modify,
                "Edit".to_string(),
                json!({ "file_path": "src/lib.rs" }),
                None,
            )
            .await
            .unwrap();
        tokio::fs::write(&file_path, "base\noutside\n").await.unwrap();

        assert!(runtime.core.get_session_files("session-1").is_empty());

        let stats = runtime
            .core
            .get_session_file_diff_stats("session-1", &file_path)
            .await
            .unwrap();
        assert_eq!(stats.lines_added, 0);
        assert_eq!(stats.lines_removed, 0);
    }
}
