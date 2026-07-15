// REFERENCE — copied from tools/plan-compliance-checker/src/task.rs
// Last synced: ec1902e (v3-restructure)
// Mirror only — NOT compiled. Original file lives in src/.
// If you change the source, re-run: node scripts/copy_reference.js

use std::path::Path;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::git_inspector::{commits_since, Commit};
use crate::path_resolver::{detect_path_mismatch, find_workspace_root};
use crate::plan::Plan;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult {
    Pending { task_id: String, checks: Vec<CheckResult> },
    Pass { task_id: String, checks: Vec<CheckResult> },
    Fail { task_id: String, checks: Vec<CheckResult> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckResult {
    FileExists { path: String, ok: bool },
    FileModified { path: String, ok: bool, sha: Option<String> },
    CommitPresent { ok: bool, sha: Option<String> },
    CommitFilesMatch { ok: bool, expected: Vec<String>, actual: Vec<String> },
    PathConsistency { path: String, ok: bool, suggestion: Option<String> },
}

fn path_matches(commit_file: &str, task_file: &str) -> bool {
    if commit_file == task_file {
        return true;
    }
    // Check if one is a suffix of the other with a path separator boundary
    // e.g., "src/crates/foo/Cargo.toml" ends with "crates/foo/Cargo.toml" (if task_file is relative)
    // or "crates/foo/Cargo.toml" is contained in "src/crates/foo/Cargo.toml"
    if commit_file.ends_with(task_file) {
        let boundary = commit_file.len() - task_file.len();
        return boundary == 0 || commit_file.as_bytes()[boundary - 1] == b'/';
    }
    if task_file.ends_with(commit_file) {
        let boundary = task_file.len() - commit_file.len();
        return boundary == 0 || task_file.as_bytes()[boundary - 1] == b'/';
    }
    false
}

pub fn check_plan(plan: &Plan, cwd: &Path) -> Result<Vec<TaskResult>> {
    let workspace_root = find_workspace_root(cwd).unwrap_or_else(|| cwd.to_path_buf());
    let commits = commits_since(&workspace_root, &plan.plan_start_sha).unwrap_or_default();

    let mut results = Vec::new();
    for task in &plan.tasks {
        let mut checks = Vec::new();
        let mut task_has_work = false;
        let mut all_ok = true;
        let mut has_commit = false;

        // Check file existence and path consistency for all create paths
        for create_path in &task.files.create {
            let abs = workspace_root.join(create_path);
            let exists = abs.exists();
            task_has_work = true;
            checks.push(CheckResult::FileExists { path: create_path.to_string_lossy().into_owned(), ok: exists });

            // Path consistency check
            let m = detect_path_mismatch(create_path, &workspace_root);
            if !m.exists_relative && m.suggestion.is_some() {
                all_ok = false;
                checks.push(CheckResult::PathConsistency {
                    path: create_path.to_string_lossy().into_owned(),
                    ok: false,
                    suggestion: m.suggestion.map(|p| p.to_string_lossy().into_owned()),
                });
            }
        }

        // Commit presence: find any commit that touches one of task's files
        let task_files: Vec<String> = task.files.create.iter().chain(
            task.files.modify.iter().map(|m| &m.path)
        ).map(|p| p.to_string_lossy().into_owned()).collect();

        let matching_commit: Option<&Commit> = commits.iter().rev().find(|c| {
            c.files.iter().any(|f| task_files.iter().any(|tf| path_matches(f, tf)))
        });

        if let Some(commit) = matching_commit {
            has_commit = true;
            checks.push(CheckResult::CommitPresent { ok: true, sha: Some(commit.sha.clone()) });
            // If commit exists but file is missing on disk → fail
            for create_path in &task.files.create {
                let abs = workspace_root.join(create_path);
                if !abs.exists() { all_ok = false; }
            }
            let commit_files: Vec<String> = commit.files.clone();
            let all_match = task_files.iter().all(|tf| commit_files.iter().any(|cf| path_matches(cf, tf)));
            if !all_match { all_ok = false; }
            checks.push(CheckResult::CommitFilesMatch {
                ok: all_match,
                expected: task_files.clone(),
                actual: commit_files,
            });
        } else if task_has_work {
            checks.push(CheckResult::CommitPresent { ok: false, sha: None });
        }

        let status = if !task_has_work {
            TaskResult::Pending { task_id: task.id.clone(), checks }
        } else if !has_commit {
            // No commit yet → Pending regardless of file existence
            TaskResult::Pending { task_id: task.id.clone(), checks }
        } else if all_ok {
            TaskResult::Pass { task_id: task.id.clone(), checks }
        } else {
            TaskResult::Fail { task_id: task.id.clone(), checks }
        };
        results.push(status);
    }

    Ok(results)
}
