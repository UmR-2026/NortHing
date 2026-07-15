use super::super::types::{
    RuntimeMigrationRecord, WorkspaceRuntimeContext, WorkspaceRuntimeTarget, WORKSPACE_RUNTIME_LAYOUT_VERSION,
};
use crate::infrastructure::PathManager;
use crate::util::errors::{NortHingError, NortHingResult};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::Mutex as AsyncMutex;

#[derive(Debug)]
pub struct WorkspaceRuntimeService {
    pub(crate) path_manager: Arc<PathManager>,
    pub(crate) verified_runtime_roots: Mutex<HashSet<PathBuf>>,
}

#[derive(Debug, Serialize)]
struct RuntimeLayoutState {
    layout_version: u32,
    runtime_root: String,
    target_kind: String,
    target_descriptor: String,
    migrated_entries: Vec<RuntimeMigrationRecordState>,
}

#[derive(Debug, Serialize)]
struct RuntimeMigrationRecordState {
    source: String,
    target: String,
    strategy: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeMigrationSpec {
    pub(crate) source: PathBuf,
    pub(crate) target: PathBuf,
    pub(crate) strategy: RuntimeMigrationStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum RuntimeMigrationStrategy {
    MoveIfTargetMissing,
    MergeSessions,
}
impl WorkspaceRuntimeService {
    pub(crate) fn is_runtime_verified(&self, runtime_root: &Path) -> bool {
        self.verified_runtime_roots
            .lock()
            .expect("workspace runtime verified cache poisoned")
            .contains(runtime_root)
    }

    pub(crate) fn mark_runtime_verified(&self, runtime_root: &Path) {
        self.verified_runtime_roots
            .lock()
            .expect("workspace runtime verified cache poisoned")
            .insert(runtime_root.to_path_buf());
    }

    pub(crate) async fn persist_layout_state(
        &self,
        context: &WorkspaceRuntimeContext,
        migrated_entries: &[RuntimeMigrationRecord],
    ) -> NortHingResult<()> {
        let target_descriptor = match &context.target {
            WorkspaceRuntimeTarget::LocalWorkspace { workspace_root } => workspace_root.display().to_string(),
            WorkspaceRuntimeTarget::RemoteWorkspaceMirror { ssh_host, remote_root } => {
                format!("{}:{}", ssh_host, remote_root)
            }
        };

        let state = RuntimeLayoutState {
            layout_version: WORKSPACE_RUNTIME_LAYOUT_VERSION,
            runtime_root: context.runtime_root.display().to_string(),
            target_kind: context.target.kind().to_string(),
            target_descriptor,
            migrated_entries: migrated_entries
                .iter()
                .map(|record| RuntimeMigrationRecordState {
                    source: record.source.display().to_string(),
                    target: record.target.display().to_string(),
                    strategy: record.strategy.clone(),
                })
                .collect(),
        };

        let bytes = serde_json::to_vec_pretty(&state)
            .map_err(|e| NortHingError::service(format!("Failed to serialize runtime state: {}", e)))?;
        tokio::fs::write(&context.layout_state_file, bytes).await.map_err(|e| {
            NortHingError::service(format!(
                "Failed to write runtime layout state '{}': {}",
                context.layout_state_file.display(),
                e
            ))
        })?;
        Ok(())
    }
}

pub fn runtime_lock_for(runtime_root: &Path) -> Arc<AsyncMutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<AsyncMutex<()>>>>> = OnceLock::new();

    let locks = LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = locks.lock().expect("workspace runtime lock store poisoned");
    guard
        .entry(runtime_root.to_path_buf())
        .or_insert_with(|| Arc::new(AsyncMutex::new(())))
        .clone()
}
