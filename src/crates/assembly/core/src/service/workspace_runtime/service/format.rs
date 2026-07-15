use super::super::types::{WorkspaceRuntimeContext, WorkspaceRuntimeEnsureResult};

pub fn cached_ensure_result(context: WorkspaceRuntimeContext) -> WorkspaceRuntimeEnsureResult {
    WorkspaceRuntimeEnsureResult {
        context,
        created_directories: Vec::new(),
        migrated_entries: Vec::new(),
    }
}
