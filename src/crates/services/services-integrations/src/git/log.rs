use super::*;
use git2::Commit;
use std::path::Path;
use tokio::task;

type CommitStats = (Option<i32>, Option<i32>, Option<i32>);

impl GitService {
    /// Gets commit history.
    pub async fn get_commits<P: AsRef<Path>>(path: P, params: GitLogParams) -> Result<Vec<GitCommit>, GitError> {
        let path_buf = path.as_ref().to_path_buf();
        task::spawn_blocking(move || {
            let repo = Repository::open(&path_buf).map_err(|e| GitError::RepositoryNotFound(e.to_string()))?;

            let mut revwalk = repo.revwalk().map_err(|e| GitError::CommandFailed(e.to_string()))?;

            // Support commit range via since..until or since..HEAD semantics.
            let has_range = params.since.is_some() || params.until.is_some();
            if let Some(until_ref) = &params.until {
                let until_oid = repo
                    .revparse_single(until_ref)
                    .map_err(|e| GitError::CommandFailed(format!("Failed to resolve 'until' ref: {e}")))?
                    .id();
                revwalk
                    .push(until_oid)
                    .map_err(|e| GitError::CommandFailed(e.to_string()))?;
            } else {
                revwalk
                    .push_head()
                    .map_err(|e| GitError::CommandFailed(e.to_string()))?;
            }

            if let Some(since_ref) = &params.since {
                let since_oid = repo
                    .revparse_single(since_ref)
                    .map_err(|e| GitError::CommandFailed(format!("Failed to resolve 'since' ref: {e}")))?
                    .id();
                revwalk
                    .hide(since_oid)
                    .map_err(|e| GitError::CommandFailed(e.to_string()))?;
            }

            // Safety valve: maximum revwalk steps for filtered queries.
            const MAX_REVWALK_STEPS: usize = 500;
            let has_filter = params.author.is_some() || params.grep.is_some();
            let step_limit = if has_range || has_filter {
                MAX_REVWALK_STEPS
            } else {
                usize::MAX
            };

            let mut commits = Vec::new();
            let mut count = 0;
            let skip = params.skip.unwrap_or(0);
            let max_count = params.max_count.unwrap_or(50);
            let mut walk_steps = 0;

            for oid_result in revwalk {
                walk_steps += 1;
                if walk_steps > step_limit {
                    break;
                }
                if count < skip as usize {
                    count += 1;
                    continue;
                }

                if commits.len() >= max_count as usize {
                    break;
                }

                let oid = oid_result.map_err(|e| GitError::CommandFailed(e.to_string()))?;

                let commit = repo
                    .find_commit(oid)
                    .map_err(|e| GitError::CommandFailed(e.to_string()))?;

                let author = commit.author();
                let message = commit.message().unwrap_or("").to_string();

                if let Some(author_filter) = &params.author {
                    if !author.name().unwrap_or("").contains(author_filter) {
                        count += 1;
                        continue;
                    }
                }

                if let Some(grep_filter) = &params.grep {
                    if !message.contains(grep_filter) {
                        count += 1;
                        continue;
                    }
                }

                let parents: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();

                let (additions, deletions, files_changed) = if params.stat.unwrap_or(false) {
                    GitService::get_commit_stats(&repo, &commit).unwrap_or((None, None, None))
                } else {
                    (None, None, None)
                };

                commits.push(GitCommit {
                    hash: commit.id().to_string(),
                    short_hash: commit.id().to_string()[..7].to_string(),
                    message,
                    author: author.name().unwrap_or("Unknown").to_string(),
                    author_email: author.email().unwrap_or("").to_string(),
                    date: format_timestamp(commit.time().seconds()),
                    parents,
                    additions,
                    deletions,
                    files_changed,
                });

                count += 1;
            }

            Ok(commits)
        })
        .await
        .map_err(|e| GitError::CommandFailed(format!("spawn_blocking join: {e}")))?
    }

    fn get_commit_stats(repo: &Repository, commit: &Commit) -> Result<CommitStats, GitError> {
        let tree = commit
            .tree()
            .map_err(|e| GitError::CommandFailed(format!("Failed to get tree: {e}")))?;

        let parent_tree = if commit.parent_count() > 0 {
            commit.parent(0).ok().and_then(|p| p.tree().ok())
        } else {
            None
        };

        let diff = repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
            .map_err(|e| GitError::CommandFailed(format!("Failed to diff: {e}")))?;

        let stats = diff
            .stats()
            .map_err(|e| GitError::CommandFailed(format!("Failed to get diff stats: {e}")))?;

        Ok((
            Some(stats.insertions() as i32),
            Some(stats.deletions() as i32),
            Some(stats.files_changed() as i32),
        ))
    }

    /// Gets the diff.
    pub async fn get_diff<P: AsRef<Path>>(path: P, params: &GitDiffParams) -> Result<String, GitError> {
        let repo_path = path.as_ref().to_string_lossy();
        let args = build_git_diff_args(params);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

        execute_git_command(&repo_path, &arg_refs).await
    }

    /// Gets changed files using `git diff --name-status`.
    pub async fn get_changed_files<P: AsRef<Path>>(
        path: P,
        params: &GitChangedFilesParams,
    ) -> Result<Vec<GitChangedFile>, GitError> {
        let repo_path = path.as_ref().to_string_lossy();
        let args = build_git_changed_files_args(params);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

        let output = execute_git_command(&repo_path, &arg_refs).await?;
        Ok(parse_name_status_output(&output))
    }

    /// Gets Git commit graph data.
    pub async fn get_git_graph<P: AsRef<Path>>(path: P, max_count: Option<usize>) -> Result<GitGraph, GitError> {
        let path_buf = path.as_ref().to_path_buf();
        task::spawn_blocking(move || {
            let repo = Repository::open(&path_buf).map_err(|e| GitError::RepositoryNotFound(e.to_string()))?;
            build_git_graph(&repo, max_count).map_err(|e| GitError::CommandFailed(e.to_string()))
        })
        .await
        .map_err(|e| GitError::CommandFailed(format!("spawn_blocking join: {e}")))?
    }

    /// Gets Git commit graph data for a specific branch.
    pub async fn get_git_graph_for_branch<P: AsRef<Path>>(
        path: P,
        max_count: Option<usize>,
        branch_name: Option<String>,
    ) -> Result<GitGraph, GitError> {
        let path_buf = path.as_ref().to_path_buf();
        task::spawn_blocking(move || {
            let repo = Repository::open(&path_buf).map_err(|e| GitError::RepositoryNotFound(e.to_string()))?;
            build_git_graph_for_branch(&repo, max_count, branch_name.as_deref())
                .map_err(|e| GitError::CommandFailed(e.to_string()))
        })
        .await
        .map_err(|e| GitError::CommandFailed(format!("spawn_blocking join: {e}")))?
    }
}
