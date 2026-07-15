use super::*;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::time::timeout;

impl GitService {
    /// Adds files to the staging area.
    pub async fn add_files<P: AsRef<Path>>(path: P, params: GitAddParams) -> Result<GitOperationResult, GitError> {
        let start_time = Instant::now();
        let repo_path = path.as_ref().to_string_lossy();

        let mut args = vec!["add"];

        if params.all.unwrap_or(false) {
            args.push("-A");
        } else if params.update.unwrap_or(false) {
            args.push("-u");
        } else {
            for file in &params.files {
                args.push(file);
            }
        }

        let output = execute_git_command(&repo_path, &args).await?;
        let duration = elapsed_ms_u64(start_time);

        Ok(GitOperationResult {
            success: true,
            data: Some(serde_json::json!({
                "files": params.files,
                "all": params.all,
                "update": params.update
            })),
            error: None,
            output: Some(output),
            duration: Some(duration),
        })
    }

    /// Commits changes.
    pub async fn commit<P: AsRef<Path>>(path: P, params: GitCommitParams) -> Result<GitOperationResult, GitError> {
        let start_time = Instant::now();
        let repo_path = path.as_ref().to_string_lossy();

        let mut args = vec!["commit".to_string(), "-m".to_string(), params.message.clone()];

        if params.amend.unwrap_or(false) {
            args.push("--amend".to_string());
        }

        if params.all.unwrap_or(false) {
            args.push("-a".to_string());
        }

        if params.no_verify.unwrap_or(false) {
            args.push("--no-verify".to_string());
        }

        if let Some(author) = &params.author {
            args.push("--author".to_string());
            args.push(format!("{} <{}>", author.name, author.email));
        }

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = execute_git_command(&repo_path, &args_refs).await?;
        let duration = elapsed_ms_u64(start_time);

        Ok(GitOperationResult {
            success: true,
            data: Some(serde_json::json!({
                "message": params.message,
                "amend": params.amend,
                "all": params.all,
                "noVerify": params.no_verify,
                "author": params.author
            })),
            error: None,
            output: Some(output),
            duration: Some(duration),
        })
    }

    /// Pushes changes.
    pub async fn push<P: AsRef<Path>>(path: P, params: GitPushParams) -> Result<GitOperationResult, GitError> {
        let start_time = Instant::now();
        let repo_path = path.as_ref().to_string_lossy();

        let mut args = vec!["push"];

        if params.force.unwrap_or(false) {
            args.push("--force");
        }

        if params.set_upstream.unwrap_or(false) {
            args.push("-u");
        }

        if let Some(remote) = &params.remote {
            args.push(remote);
        }

        if let Some(branch) = &params.branch {
            args.push(branch);
        }

        let output = timeout(Duration::from_secs(30), execute_git_command(&repo_path, &args))
            .await
            .map_err(|_| GitError::NetworkError("Push operation timed out".to_string()))??;

        let duration = elapsed_ms_u64(start_time);

        Ok(GitOperationResult {
            success: true,
            data: Some(serde_json::json!({
                "remote": params.remote,
                "branch": params.branch,
                "force": params.force,
                "set_upstream": params.set_upstream
            })),
            error: None,
            output: Some(output),
            duration: Some(duration),
        })
    }

    /// Pulls changes.
    pub async fn pull<P: AsRef<Path>>(path: P, params: GitPullParams) -> Result<GitOperationResult, GitError> {
        let start_time = Instant::now();
        let repo_path = path.as_ref().to_string_lossy();

        let mut args = vec!["pull"];

        if params.rebase.unwrap_or(false) {
            args.push("--rebase");
        }

        if let Some(remote) = &params.remote {
            args.push(remote);
        }

        if let Some(branch) = &params.branch {
            args.push(branch);
        }

        let output = timeout(Duration::from_secs(30), execute_git_command(&repo_path, &args))
            .await
            .map_err(|_| GitError::NetworkError("Pull operation timed out".to_string()))??;

        let duration = elapsed_ms_u64(start_time);

        Ok(GitOperationResult {
            success: true,
            data: Some(serde_json::json!({
                "remote": params.remote,
                "branch": params.branch,
                "rebase": params.rebase
            })),
            error: None,
            output: Some(output),
            duration: Some(duration),
        })
    }

    /// Resets to a specific commit.
    ///
    /// # Parameters
    /// - `path`: Repository path
    /// - `commit_hash`: Target commit hash
    /// - `mode`: Reset mode (`soft`, `mixed`, `hard`)
    pub async fn reset_to_commit<P: AsRef<Path>>(
        path: P,
        commit_hash: &str,
        mode: &str,
    ) -> Result<GitOperationResult, GitError> {
        let start_time = Instant::now();
        let repo_path = path.as_ref().to_string_lossy();

        let mode_flag = match mode {
            "soft" => "--soft",
            "mixed" => "--mixed",
            "hard" => "--hard",
            _ => return Err(GitError::CommandFailed(format!("Invalid reset mode: {}", mode))),
        };

        let args = vec!["reset", mode_flag, commit_hash];
        let output = execute_git_command(&repo_path, &args).await?;
        let duration = elapsed_ms_u64(start_time);

        Ok(GitOperationResult {
            success: true,
            data: Some(serde_json::json!({
                "commit": commit_hash,
                "mode": mode
            })),
            error: None,
            output: Some(output),
            duration: Some(duration),
        })
    }

    /// Cherry-picks a commit onto the current branch.
    ///
    /// # Parameters
    /// - `path`: Repository path
    /// - `commit_hash`: Commit hash to cherry-pick
    /// - `no_commit`: Apply changes without committing automatically (default `false`)
    ///
    /// # Returns
    /// - Operation result
    pub async fn cherry_pick<P: AsRef<Path>>(
        path: P,
        commit_hash: &str,
        no_commit: bool,
    ) -> Result<GitOperationResult, GitError> {
        let start_time = Instant::now();
        let repo_path = path.as_ref().to_string_lossy();

        let mut args = vec!["cherry-pick"];

        if no_commit {
            args.push("-n");
        }

        args.push(commit_hash);

        let output = execute_git_command(&repo_path, &args).await?;
        let duration = elapsed_ms_u64(start_time);

        Ok(GitOperationResult {
            success: true,
            data: Some(serde_json::json!({
                "commit": commit_hash,
                "no_commit": no_commit
            })),
            error: None,
            output: Some(output),
            duration: Some(duration),
        })
    }

    /// Aborts the cherry-pick operation.
    ///
    /// # Parameters
    /// - `path`: Repository path
    ///
    /// # Returns
    /// - Operation result
    pub async fn cherry_pick_abort<P: AsRef<Path>>(path: P) -> Result<GitOperationResult, GitError> {
        let start_time = Instant::now();
        let repo_path = path.as_ref().to_string_lossy();

        let args = vec!["cherry-pick", "--abort"];
        let output = execute_git_command(&repo_path, &args).await?;
        let duration = elapsed_ms_u64(start_time);

        Ok(GitOperationResult {
            success: true,
            data: None,
            error: None,
            output: Some(output),
            duration: Some(duration),
        })
    }

    /// Continues the cherry-pick operation (after resolving conflicts).
    ///
    /// # Parameters
    /// - `path`: Repository path
    ///
    /// # Returns
    /// - Operation result
    pub async fn cherry_pick_continue<P: AsRef<Path>>(path: P) -> Result<GitOperationResult, GitError> {
        let start_time = Instant::now();
        let repo_path = path.as_ref().to_string_lossy();

        let args = vec!["cherry-pick", "--continue"];
        let output = execute_git_command(&repo_path, &args).await?;
        let duration = elapsed_ms_u64(start_time);

        Ok(GitOperationResult {
            success: true,
            data: None,
            error: None,
            output: Some(output),
            duration: Some(duration),
        })
    }

    /// Lists all worktrees.
    ///
    /// # Parameters
    /// - `path`: Repository path
    ///
    /// # Returns
    /// - Worktree list
    pub async fn list_worktrees<P: AsRef<Path>>(path: P) -> Result<Vec<GitWorktreeInfo>, GitError> {
        let repo_path = path.as_ref().to_string_lossy();

        let args = vec!["worktree", "list", "--porcelain"];
        let output = execute_git_command(&repo_path, &args).await?;

        Ok(parse_worktree_list(&output))
    }

    /// Adds a new worktree.
    ///
    /// # Parameters
    /// - `path`: Repository path
    /// - `branch`: Branch name
    /// - `create_branch`: Whether to create a new branch
    ///
    /// # Returns
    /// - Newly created worktree information
    pub async fn add_worktree<P: AsRef<Path>>(
        path: P,
        branch: &str,
        create_branch: bool,
    ) -> Result<GitWorktreeInfo, GitError> {
        let repo_path = path.as_ref().to_string_lossy();

        let worktree_dir = path.as_ref().join(".worktrees");
        let worktree_path = worktree_dir.join(branch);
        let worktree_path_str = worktree_path.to_string_lossy().to_string();

        if !worktree_dir.exists() {
            std::fs::create_dir_all(&worktree_dir).map_err(GitError::IoError)?;
        }

        let args = if create_branch {
            vec!["worktree", "add", "-b", branch, &worktree_path_str]
        } else {
            vec!["worktree", "add", &worktree_path_str, branch]
        };

        execute_git_command(&repo_path, &args).await?;

        let worktrees = Self::list_worktrees(&path).await?;

        let normalized_expected = worktree_path_str.replace("\\", "/");

        worktrees
            .into_iter()
            .find(|wt| wt.path == normalized_expected)
            .ok_or_else(|| GitError::CommandFailed("Failed to find newly created worktree".to_string()))
    }

    /// Removes a worktree.
    ///
    /// # Parameters
    /// - `path`: Repository path
    /// - `worktree_path`: Worktree path to remove
    /// - `force`: Whether to force removal
    ///
    /// # Returns
    /// - Operation result
    pub async fn remove_worktree<P: AsRef<Path>>(
        path: P,
        worktree_path: &str,
        force: bool,
    ) -> Result<GitOperationResult, GitError> {
        let start_time = Instant::now();
        let repo_path = path.as_ref().to_string_lossy();

        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(worktree_path);

        let output = execute_git_command(&repo_path, &args).await?;
        let duration = elapsed_ms_u64(start_time);

        Ok(GitOperationResult {
            success: true,
            data: Some(serde_json::json!({
                "worktree_path": worktree_path,
                "force": force
            })),
            error: None,
            output: Some(output),
            duration: Some(duration),
        })
    }
}
