//! Git service implementation facade.
//!
//! Split into sub-domain siblings to keep each concern small and reviewable.

use super::*;
use git2::{BranchType, Repository};
use std::time::Instant;

pub struct GitService;

/// Shared helper: measure elapsed wall time in milliseconds.
pub(crate) fn elapsed_ms_u64(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis() as u64
}

/// Shared helper: compute ahead/behind counts between a local branch and its upstream.
pub(crate) fn get_ahead_behind_count(
    repo: &Repository,
    branch_name: &str,
) -> Result<(i32, i32), GitError> {
    let local_branch = repo
        .find_branch(branch_name, BranchType::Local)
        .map_err(|e| GitError::BranchNotFound(e.to_string()))?;

    if let Ok(upstream) = local_branch.upstream() {
        let local_oid = local_branch
            .get()
            .target()
            .ok_or_else(|| GitError::CommandFailed("Failed to get local branch target".to_string()))?;
        let upstream_oid = upstream
            .get()
            .target()
            .ok_or_else(|| GitError::CommandFailed("Failed to get upstream branch target".to_string()))?;

        let (ahead, behind) = repo
            .graph_ahead_behind(local_oid, upstream_oid)
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;

        Ok((ahead as i32, behind as i32))
    } else {
        Ok((0, 0))
    }
}

#[path = "repository.rs"]
mod repository;
#[path = "branch.rs"]
mod branch;
#[path = "log.rs"]
mod log;
#[path = "operations.rs"]
mod operations;
