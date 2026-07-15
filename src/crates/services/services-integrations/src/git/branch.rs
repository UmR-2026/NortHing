use super::*;
use std::path::Path;
use std::time::Instant;
use tokio::task;

impl GitService {
    /// Gets the branch list.
    pub async fn get_branches<P: AsRef<Path>>(path: P, include_remote: bool) -> Result<Vec<GitBranch>, GitError> {
        let path_buf = path.as_ref().to_path_buf();
        task::spawn_blocking(move || {
            let repo = Repository::open(&path_buf).map_err(|e| GitError::RepositoryNotFound(e.to_string()))?;

            let mut branches = Vec::new();
            let current_branch = get_current_branch(&repo)?;

            let local_branches = repo
                .branches(Some(BranchType::Local))
                .map_err(|e| GitError::CommandFailed(e.to_string()))?;

            for branch_result in local_branches {
                let (branch, _) = branch_result.map_err(|e| GitError::CommandFailed(e.to_string()))?;

                if let Some(name) = branch.name().map_err(|e| GitError::CommandFailed(e.to_string()))? {
                    let is_current = name == current_branch;
                    let upstream = branch
                        .upstream()
                        .ok()
                        .and_then(|upstream_branch| upstream_branch.name().ok().flatten().map(|s| s.to_string()));

                    let (last_commit, last_commit_date) = if let Ok(commit) = branch.get().peel_to_commit() {
                        (
                            Some(commit.id().to_string()),
                            Some(format_timestamp(commit.time().seconds())),
                        )
                    } else {
                        (None, None)
                    };

                    let (ahead, behind) = if is_current {
                        get_ahead_behind_count(&repo, name).unwrap_or((0, 0))
                    } else {
                        (0, 0)
                    };

                    branches.push(GitBranch {
                        name: name.to_string(),
                        current: is_current,
                        remote: false,
                        upstream,
                        ahead,
                        behind,
                        last_commit,
                        last_commit_date: last_commit_date.clone(),

                        base_branch: None,
                        child_branches: None,
                        merged_branches: None,
                        branch_type: Some(Self::determine_branch_type(name)),
                        has_conflicts: None,
                        can_merge: None,
                        is_stale: None,
                        merge_status: None,
                        stats: None,
                        created_at: None,
                        last_activity_at: last_commit_date,
                        tags: None,
                        description: None,
                        linked_issues: None,
                    });
                }
            }

            if include_remote {
                let remote_branches = repo
                    .branches(Some(BranchType::Remote))
                    .map_err(|e| GitError::CommandFailed(e.to_string()))?;

                for branch_result in remote_branches {
                    let (branch, _) = branch_result.map_err(|e| GitError::CommandFailed(e.to_string()))?;

                    if let Some(name) = branch.name().map_err(|e| GitError::CommandFailed(e.to_string()))? {
                        let (last_commit, last_commit_date) = if let Ok(commit) = branch.get().peel_to_commit() {
                            (
                                Some(commit.id().to_string()),
                                Some(format_timestamp(commit.time().seconds())),
                            )
                        } else {
                            (None, None)
                        };

                        branches.push(GitBranch {
                            name: name.to_string(),
                            current: false,
                            remote: true,
                            upstream: None,
                            ahead: 0,
                            behind: 0,
                            last_commit,
                            last_commit_date: last_commit_date.clone(),

                            base_branch: None,
                            child_branches: None,
                            merged_branches: None,
                            branch_type: Some(Self::determine_branch_type(name)),
                            has_conflicts: None,
                            can_merge: None,
                            is_stale: None,
                            merge_status: None,
                            stats: None,
                            created_at: None,
                            last_activity_at: last_commit_date,
                            tags: None,
                            description: None,
                            linked_issues: None,
                        });
                    }
                }
            }

            Ok(branches)
        })
        .await
        .map_err(|e| GitError::CommandFailed(format!("spawn_blocking join: {e}")))?
    }

    /// Gets branches with detailed information.
    pub async fn get_enhanced_branches<P: AsRef<Path>>(
        path: P,
        include_remote: bool,
    ) -> Result<Vec<GitBranch>, GitError> {
        let mut branches = Self::get_branches(&path, include_remote).await?;

        Self::analyze_branch_relations(&mut branches)?;

        let path_buf = path.as_ref().to_path_buf();
        task::spawn_blocking(move || {
            let repo = Repository::open(&path_buf).map_err(|e| GitError::RepositoryNotFound(e.to_string()))?;
            let current_branch = get_current_branch(&repo)?;

            for branch in &mut branches {
                if !branch.remote {
                    branch.stats = GitService::calculate_branch_stats(&repo, &branch.name).ok();
                    branch.is_stale = Some(GitService::is_branch_stale(branch));
                    if branch.name != current_branch {
                        branch.can_merge = GitService::can_merge_safely(&repo, &branch.name).ok();
                        branch.has_conflicts = branch.can_merge.map(|can| !can);
                    }
                }
            }

            Ok(branches)
        })
        .await
        .map_err(|e| GitError::CommandFailed(format!("spawn_blocking join: {e}")))?
    }

    /// Determines the branch type.
    fn determine_branch_type(branch_name: &str) -> String {
        if branch_name.starts_with("feature/") || branch_name.starts_with("feat/") {
            "feature".to_string()
        } else if branch_name.starts_with("hotfix/") || branch_name.starts_with("fix/") {
            "hotfix".to_string()
        } else if branch_name.starts_with("release/") || branch_name.starts_with("rel/") {
            "release".to_string()
        } else if branch_name.starts_with("bugfix/") || branch_name.starts_with("bug/") {
            "bugfix".to_string()
        } else if branch_name.starts_with("chore/") {
            "chore".to_string()
        } else if branch_name.starts_with("docs/") {
            "docs".to_string()
        } else if branch_name.starts_with("test/") {
            "test".to_string()
        } else if ["main", "master", "develop", "development"].contains(&branch_name) {
            "main".to_string()
        } else {
            "other".to_string()
        }
    }

    /// Analyzes branch relationships.
    fn analyze_branch_relations(branches: &mut [GitBranch]) -> Result<(), GitError> {
        let main_branches = ["main", "master", "develop"];

        let available_main_branches: Vec<String> = branches
            .iter()
            .filter(|b| !b.remote && main_branches.contains(&b.name.as_str()))
            .map(|b| b.name.clone())
            .collect();

        for branch in branches.iter_mut() {
            if !branch.remote && !main_branches.contains(&branch.name.as_str()) {
                if let Some(main_branch) = available_main_branches.first() {
                    branch.base_branch = Some(main_branch.clone());
                }
            }
        }

        let mut child_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

        for branch in branches.iter() {
            if let Some(base) = &branch.base_branch {
                child_map.entry(base.clone()).or_default().push(branch.name.clone());
            }
        }

        for branch in branches.iter_mut() {
            if let Some(children) = child_map.get(&branch.name) {
                branch.child_branches = Some(children.clone());
            }
        }

        Ok(())
    }

    /// Computes branch statistics.
    fn calculate_branch_stats(repo: &Repository, branch_name: &str) -> Result<GitBranchStats, GitError> {
        let branch_ref = repo
            .find_branch(branch_name, BranchType::Local)
            .map_err(|e| GitError::BranchNotFound(e.to_string()))?;

        let target = branch_ref
            .get()
            .target()
            .ok_or_else(|| GitError::CommandFailed("Branch has no target".to_string()))?;

        let mut revwalk = repo.revwalk().map_err(|e| GitError::CommandFailed(e.to_string()))?;
        revwalk
            .push(target)
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;

        // Only count recent commits, avoid full-history traversal.
        const STATS_COMMIT_LIMIT: usize = 1000;
        let commit_count = revwalk.take(STATS_COMMIT_LIMIT).count() as i32;

        Ok(GitBranchStats {
            commit_count,
            contributor_count: 1,
            file_changes: 0,
            lines_changed: GitLinesChanged {
                additions: 0,
                deletions: 0,
            },
            activity_score: std::cmp::min(commit_count * 2, 100),
        })
    }

    /// Branches with no activity in this many days are considered stale.
    const STALE_DAYS_THRESHOLD: i64 = 90;

    /// Checks whether a branch is stale.
    fn is_branch_stale(branch: &GitBranch) -> bool {
        match branch.last_activity_at.as_ref().or(branch.last_commit_date.as_ref()) {
            Some(date_str) => {
                // format_timestamp produces "YYYY-MM-DD HH:MM:SS UTC"
                chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S UTC")
                    .map(|dt| (chrono::Utc::now().naive_utc() - dt).num_days() > Self::STALE_DAYS_THRESHOLD)
                    .unwrap_or(false)
            }
            None => true,
        }
    }

    /// Checks whether a branch can be merged safely into HEAD via
    /// three-way merge analysis (merge-base, merge-trees).
    fn can_merge_safely(repo: &Repository, branch_name: &str) -> Result<bool, GitError> {
        let branch = repo
            .find_branch(branch_name, BranchType::Local)
            .map_err(|e| GitError::BranchNotFound(e.to_string()))?;
        let branch_commit = branch
            .get()
            .peel_to_commit()
            .map_err(|e| GitError::CommandFailed(format!("Failed to peel branch: {e}")))?;

        let head_commit = repo
            .head()
            .map_err(|e| GitError::CommandFailed(format!("Failed to get HEAD: {e}")))?
            .peel_to_commit()
            .map_err(|e| GitError::CommandFailed(format!("Failed to peel HEAD: {e}")))?;

        let base_oid = repo
            .merge_base(head_commit.id(), branch_commit.id())
            .map_err(|e| GitError::CommandFailed(format!("Failed to find merge base: {e}")))?;
        let base_commit = repo
            .find_commit(base_oid)
            .map_err(|e| GitError::CommandFailed(format!("Failed to find merge base commit: {e}")))?;

        let base_tree = base_commit
            .tree()
            .map_err(|e| GitError::CommandFailed(format!("Failed to get base tree: {e}")))?;
        let head_tree = head_commit
            .tree()
            .map_err(|e| GitError::CommandFailed(format!("Failed to get HEAD tree: {e}")))?;
        let branch_tree = branch_commit
            .tree()
            .map_err(|e| GitError::CommandFailed(format!("Failed to get branch tree: {e}")))?;

        let index = repo
            .merge_trees(&base_tree, &head_tree, &branch_tree, None)
            .map_err(|e| GitError::MergeConflict(format!("Merge analysis failed: {e}")))?;

        Ok(!index.has_conflicts())
    }

    /// Checks out a branch.
    pub async fn checkout_branch<P: AsRef<Path>>(path: P, branch_name: &str) -> Result<GitOperationResult, GitError> {
        let start_time = Instant::now();
        let repo_path = path.as_ref().to_string_lossy();

        let args = vec!["checkout", branch_name];
        let output = execute_git_command(&repo_path, &args).await?;
        let duration = elapsed_ms_u64(start_time);

        Ok(GitOperationResult {
            success: true,
            data: Some(serde_json::json!({
                "branch": branch_name
            })),
            error: None,
            output: Some(output),
            duration: Some(duration),
        })
    }

    /// Creates a branch.
    pub async fn create_branch<P: AsRef<Path>>(
        path: P,
        branch_name: &str,
        start_point: Option<&str>,
    ) -> Result<GitOperationResult, GitError> {
        let start_time = Instant::now();
        let repo_path = path.as_ref().to_string_lossy();

        let mut args = vec!["checkout", "-b", branch_name];
        let effective_start_point = start_point.filter(|s| !s.trim().is_empty());
        if let Some(start) = effective_start_point {
            args.push(start);
        }

        let output = execute_git_command(&repo_path, &args).await?;
        let duration = elapsed_ms_u64(start_time);

        Ok(GitOperationResult {
            success: true,
            data: Some(serde_json::json!({
                "branch": branch_name,
                "start_point": effective_start_point
            })),
            error: None,
            output: Some(output),
            duration: Some(duration),
        })
    }

    /// Deletes a branch.
    pub async fn delete_branch<P: AsRef<Path>>(
        path: P,
        branch_name: &str,
        force: bool,
    ) -> Result<GitOperationResult, GitError> {
        let start_time = Instant::now();
        let repo_path = path.as_ref().to_string_lossy();

        let flag = if force { "-D" } else { "-d" };
        let args = vec!["branch", flag, branch_name];
        let output = execute_git_command(&repo_path, &args).await?;
        let duration = elapsed_ms_u64(start_time);

        Ok(GitOperationResult {
            success: true,
            data: Some(serde_json::json!({
                "branch": branch_name,
                "force": force
            })),
            error: None,
            output: Some(output),
            duration: Some(duration),
        })
    }
}
