//! Snapshot manager — workspace-scoped registry + static state.
//!
//! Owns the `HashMap<PathBuf, Arc<SnapshotManager>>` static state, the
//! per-workspace init-lock map (so concurrent cold-starts converge on a
//! single instance), and the public `get_or_create_snapshot_manager` /
//! `get_snapshot_manager_for_workspace` / `ensure_snapshot_manager_for_workspace`
//! / `initialize_snapshot_manager_for_workspace` functions used by
//! bootstrap, session restore, remote-connect, and server entrypoints.
//!
//! Capture / invalidate / query / lock / wrapped paths live in their
//! respective sibling impl blocks. The test hooks (`record_..._for_test`,
//! `reset_..._for_test`, `clear_snapshot_manager_for_test`, etc.) are
//! kept `pub(super)` so the facade's `#[cfg(test)] mod tests` block can
//! drive them without exposing them beyond the `snapshot/` module.
//!
//! This is an R46c split sibling of `manager.rs`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock as StdRwLock};
use std::time::Instant;

use tokio::sync::Mutex as AsyncMutex;
use tracing::{debug, info};

use crate::service::snapshot::types::{SnapshotConfig, SnapshotError, SnapshotResult};

use super::manager::SnapshotManager;

fn snapshot_managers() -> &'static StdRwLock<HashMap<PathBuf, Arc<SnapshotManager>>> {
    static SNAPSHOT_MANAGERS: OnceLock<StdRwLock<HashMap<PathBuf, Arc<SnapshotManager>>>> = OnceLock::new();
    SNAPSHOT_MANAGERS.get_or_init(|| StdRwLock::new(HashMap::new()))
}

fn snapshot_manager_init_locks() -> &'static AsyncMutex<HashMap<PathBuf, Arc<AsyncMutex<()>>>> {
    static SNAPSHOT_MANAGER_INIT_LOCKS: OnceLock<AsyncMutex<HashMap<PathBuf, Arc<AsyncMutex<()>>>>> = OnceLock::new();
    SNAPSHOT_MANAGER_INIT_LOCKS.get_or_init(|| AsyncMutex::new(HashMap::new()))
}

async fn snapshot_manager_init_lock(workspace_dir: &Path) -> Arc<AsyncMutex<()>> {
    let mut locks = snapshot_manager_init_locks().lock().await;
    locks
        .entry(workspace_dir.to_path_buf())
        .or_insert_with(|| Arc::new(AsyncMutex::new(())))
        .clone()
}

#[cfg(test)]
static SNAPSHOT_MANAGER_NEW_COUNT_FOR_TEST: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
#[cfg(test)]
static SNAPSHOT_MANAGER_NEW_DELAY_MS_FOR_TEST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(test)]
pub(super) async fn record_snapshot_manager_new_for_test() {
    use std::sync::atomic::Ordering;
    SNAPSHOT_MANAGER_NEW_COUNT_FOR_TEST.fetch_add(1, Ordering::SeqCst);
    let delay_ms = SNAPSHOT_MANAGER_NEW_DELAY_MS_FOR_TEST.load(Ordering::SeqCst);
    if delay_ms > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
    }
}

#[cfg(test)]
pub(super) fn reset_snapshot_manager_new_count_for_test() {
    use std::sync::atomic::Ordering;
    SNAPSHOT_MANAGER_NEW_COUNT_FOR_TEST.store(0, Ordering::SeqCst);
}

#[cfg(test)]
pub(super) fn snapshot_manager_new_count_for_test() -> usize {
    use std::sync::atomic::Ordering;
    SNAPSHOT_MANAGER_NEW_COUNT_FOR_TEST.load(Ordering::SeqCst)
}

#[cfg(test)]
pub(super) fn set_snapshot_manager_new_delay_for_test(delay: std::time::Duration) {
    use std::sync::atomic::Ordering;
    SNAPSHOT_MANAGER_NEW_DELAY_MS_FOR_TEST.store(delay.as_millis() as u64, Ordering::SeqCst);
}

#[cfg(test)]
pub(super) fn clear_snapshot_manager_for_test(workspace_dir: &Path) {
    if let Ok(mut managers) = snapshot_managers().write() {
        managers.remove(workspace_dir);
    }
}

pub async fn get_or_create_snapshot_manager(
    workspace_dir: PathBuf,
    config: Option<SnapshotConfig>,
) -> SnapshotResult<Arc<SnapshotManager>> {
    if let Some(existing) = get_snapshot_manager_for_workspace(&workspace_dir) {
        return Ok(existing);
    }

    let init_lock = snapshot_manager_init_lock(&workspace_dir).await;
    let _init_guard = init_lock.lock().await;

    if let Some(existing) = get_snapshot_manager_for_workspace(&workspace_dir) {
        debug!(
            "Snapshot manager initialized by concurrent request: workspace={}",
            workspace_dir.display()
        );
        return Ok(existing);
    }

    let started_at = Instant::now();
    info!(
        "Snapshot manager cold initialization started: workspace={}",
        workspace_dir.display()
    );
    let manager = Arc::new(SnapshotManager::new(workspace_dir.clone(), config).await?);
    {
        let mut managers = snapshot_managers()
            .write()
            .map_err(|_| SnapshotError::ConfigError("Snapshot manager store lock poisoned".to_string()))?;
        if let Some(existing) = managers.get(&workspace_dir) {
            return Ok(existing.clone());
        }
        managers.insert(workspace_dir, manager.clone());
    }
    info!(
        "Snapshot manager cold initialization completed: duration_ms={}",
        started_at.elapsed().as_millis()
    );

    Ok(manager)
}

pub fn get_snapshot_manager_for_workspace(workspace_dir: &Path) -> Option<Arc<SnapshotManager>> {
    snapshot_managers()
        .read()
        .ok()
        .and_then(|managers| managers.get(workspace_dir).cloned())
}

pub fn ensure_snapshot_manager_for_workspace(workspace_dir: &Path) -> SnapshotResult<Arc<SnapshotManager>> {
    get_snapshot_manager_for_workspace(workspace_dir).ok_or_else(|| {
        SnapshotError::ConfigError(format!(
            "Snapshot manager not initialized for workspace: {}",
            workspace_dir.display()
        ))
    })
}

/// Initializes a snapshot manager for the provided workspace.
pub async fn initialize_snapshot_manager_for_workspace(
    workspace_dir: PathBuf,
    config: Option<SnapshotConfig>,
) -> SnapshotResult<()> {
    get_or_create_snapshot_manager(workspace_dir, config).await?;
    debug!("Snapshot manager initialized for workspace");
    Ok(())
}
