//! Snapshot manager facade — R46c split.
//!
//! Owns the public `SnapshotManager` struct and the two non-delegating
//! methods (`new`, `get_snapshot_service`). All other methods on
//! `SnapshotManager` are pure delegations to the underlying
//! `SnapshotService` and live in sibling `impl SnapshotManager { ... }`
//! blocks split by sub-domain:
//!
//! - `manager_capture.rs`    — `record_file_change` (write path).
//! - `manager_invalidate.rs` — rollback / accept / reject (5 methods).
//! - `manager_query.rs`      — read-only queries (11 methods).
//! - `manager_lock.rs`       — file lock + conflict + Git isolation (5 methods).
//! - `manager_wrapped.rs`    — `WrappedTool` struct + Tool trait impl +
//!                             `wrap_tool_for_snapshot_tracking` /
//!                             `get_snapshot_wrapped_tools` free functions.
//! - `manager_registry.rs`   — workspace-scoped static state, test hooks,
//!                             and the public registry functions
//!                             (`get_or_create_snapshot_manager` etc.).
//!
//! Module wiring (`mod manager_capture;` etc.) lives in `snapshot/mod.rs`
//! so each sibling resolves as a peer of this facade, not as a child of
//! it. Visibility from siblings into the facade struct is
//! `super::manager::SnapshotManager`. Public free functions and the
//! struct itself are re-exported here so external callers keep using
//! `crate::service::snapshot::manager::X` /
//! `crate::service::snapshot::SnapshotManager` paths unchanged.
//!
//! This split mirrors R42c (`snapshot_core.rs` 1309 -> facade + 4 sibling)
//! and R33 (`snapshot_system.rs` 920 -> facade + 2 sibling).

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::info;

use crate::service::snapshot::service::SnapshotService;
use crate::service::snapshot::types::{SnapshotConfig, SnapshotResult};
use crate::service::workspace_runtime::workspace_runtime_service_arc;

/// Snapshot manager
///
/// Manages all components of the snapshot system.
pub struct SnapshotManager {
    pub(super) snapshot_service: Arc<RwLock<SnapshotService>>,
}

impl SnapshotManager {
    /// Creates a new snapshot manager.
    pub async fn new(workspace_dir: PathBuf, config: Option<SnapshotConfig>) -> SnapshotResult<Self> {
        #[cfg(test)]
        super::manager_registry::record_snapshot_manager_new_for_test().await;

        info!("Creating snapshot manager: workspace={}", workspace_dir.display());

        let runtime_service = workspace_runtime_service_arc();
        let runtime_context = runtime_service
            .ensure_local_workspace_runtime(&workspace_dir)
            .await
            .map_err(|e| crate::service::snapshot::types::SnapshotError::ConfigError(e.to_string()))?
            .context;

        let mut snapshot_service = SnapshotService::new(workspace_dir, runtime_context, config);
        snapshot_service.initialize().await?;
        let snapshot_service = Arc::new(RwLock::new(snapshot_service));
        Ok(Self { snapshot_service })
    }

    /// Returns a reference to the snapshot service (for advanced operations).
    pub fn snapshot_service(&self) -> Arc<RwLock<SnapshotService>> {
        self.snapshot_service.clone()
    }
}

// Re-exports: keep external paths (`crate::service::snapshot::manager::X`
// and `crate::service::snapshot::wrap_tool_for_snapshot_tracking` etc.)
// working unchanged after the R46c split.
pub use super::manager_registry::{
    ensure_snapshot_manager_for_workspace, get_or_create_snapshot_manager, get_snapshot_manager_for_workspace,
    initialize_snapshot_manager_for_workspace,
};
pub use super::manager_wrapped::{snapshot_wrapped_tools, wrap_tool_for_snapshot_tracking};

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;

    use super::super::manager_registry::{
        clear_snapshot_manager_for_test, get_or_create_snapshot_manager, reset_snapshot_manager_new_count_for_test,
        set_snapshot_manager_new_delay_for_test, snapshot_manager_new_count_for_test,
    };

    struct TestWorkspace {
        path: PathBuf,
    }

    impl TestWorkspace {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("northhing-snapshot-manager-test-{}", Uuid::new_v4()));
            std::fs::create_dir_all(&path).expect("test workspace should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestWorkspace {
        fn drop(&mut self) {
            clear_snapshot_manager_for_test(&self.path);
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_get_or_create_initializes_snapshot_manager_once_per_workspace() {
        let workspace = TestWorkspace::new();
        clear_snapshot_manager_for_test(workspace.path());
        reset_snapshot_manager_new_count_for_test();
        set_snapshot_manager_new_delay_for_test(Duration::from_millis(80));

        let first = get_or_create_snapshot_manager(workspace.path().to_path_buf(), None);
        let second = get_or_create_snapshot_manager(workspace.path().to_path_buf(), None);
        let (first, second) = tokio::join!(first, second);

        set_snapshot_manager_new_delay_for_test(Duration::ZERO);

        let first = first.expect("first snapshot manager should initialize");
        let second = second.expect("second snapshot manager should initialize");

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(snapshot_manager_new_count_for_test(), 1);
    }
}
