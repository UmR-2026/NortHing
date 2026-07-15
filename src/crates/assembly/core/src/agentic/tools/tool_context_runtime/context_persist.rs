use crate::agentic::coordination::global_coordinator;
use crate::agentic::session::EvidenceLedgerCheckpoint;
use crate::agentic::tools::workspace_paths::is_northhing_runtime_uri;
use crate::service::git::{GitDiffParams, GitService};
use northhing_agent_runtime::checkpoint::{
    build_light_checkpoint as build_runtime_light_checkpoint, GitStatusCheckpointFacts, LightCheckpoint,
    LightCheckpointWorkspaceFacts,
};
use sha2::{Digest, Sha256};
use std::path::Path;
use tracing::warn;

impl From<LightCheckpoint> for EvidenceLedgerCheckpoint {
    fn from(value: LightCheckpoint) -> Self {
        Self {
            current_branch: value.current_branch,
            dirty_state_summary: value.dirty_state_summary,
            touched_files: value.touched_files,
            diff_hash: value.diff_hash,
        }
    }
}

impl super::context_init::ToolUseContext {
    pub async fn record_light_checkpoint(&self, tool_name: &str, target: &str, touched_files: Vec<String>) {
        let Some(session_id) = self.session_id.as_deref() else {
            return;
        };
        let Some(turn_id) = self.dialog_turn_id.as_deref() else {
            return;
        };
        let Some(coordinator) = global_coordinator() else {
            return;
        };

        let checkpoint = self.build_light_checkpoint(touched_files).await;
        coordinator
            .session_manager()
            .record_checkpoint_created(session_id, turn_id, tool_name, target, checkpoint);
    }

    async fn build_light_checkpoint(&self, touched_files: Vec<String>) -> EvidenceLedgerCheckpoint {
        if self.is_remote() {
            return build_runtime_light_checkpoint(touched_files, LightCheckpointWorkspaceFacts::RemoteWorkspace)
                .into();
        }

        let Some(workspace_root) = self.workspace_root() else {
            return build_runtime_light_checkpoint(touched_files, LightCheckpointWorkspaceFacts::WorkspaceUnavailable)
                .into();
        };

        let git_status = GitService::get_status(workspace_root)
            .await
            .map(|status| GitStatusCheckpointFacts {
                current_branch: status.current_branch,
                staged_count: status.staged.len(),
                unstaged_count: status.unstaged.len(),
                untracked_count: status.untracked.len(),
            })
            .map_err(|error| error.to_string());
        let diff_hash = self.checkpoint_diff_hash(workspace_root, &touched_files).await;
        build_runtime_light_checkpoint(
            touched_files,
            LightCheckpointWorkspaceFacts::LocalWorkspace { git_status, diff_hash },
        )
        .into()
    }

    async fn checkpoint_diff_hash(&self, workspace_root: &Path, touched_files: &[String]) -> Option<String> {
        let files = touched_files
            .iter()
            .filter_map(|file| git_relative_path(workspace_root, file))
            .collect::<Vec<_>>();

        if files.is_empty() {
            return None;
        }

        let mut diff = String::new();
        for staged in [false, true] {
            let params = GitDiffParams {
                files: Some(files.clone()),
                staged: Some(staged),
                ..Default::default()
            };
            match GitService::get_diff(workspace_root, &params).await {
                Ok(part) => diff.push_str(&part),
                Err(error) => {
                    warn!(
                        "Failed to collect checkpoint diff hash: staged={}, error={}",
                        staged, error
                    );
                    return None;
                }
            }
        }

        if diff.is_empty() {
            return None;
        }

        Some(hex::encode(Sha256::digest(diff.as_bytes())))
    }
}

fn git_relative_path(workspace_root: &Path, path: &str) -> Option<String> {
    if is_northhing_runtime_uri(path) {
        return None;
    }

    let path = Path::new(path);
    let relative = if path.is_absolute() {
        path.strip_prefix(workspace_root).ok()?
    } else {
        path
    };

    Some(relative.to_string_lossy().replace('\\', "/"))
}
