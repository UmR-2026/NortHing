use super::*;
use std::path::Path;
use std::time::Duration;
use tokio::task;
use tokio::time::timeout;

impl GitService {
    /// Checks whether the path is a Git repository.
    pub async fn is_repository<P: AsRef<Path>>(path: P) -> Result<bool, GitError> {
        let path_buf = path.as_ref().to_path_buf();
        task::spawn_blocking(move || Ok(is_git_repository(path_buf)))
            .await
            .map_err(|e| GitError::CommandFailed(format!("spawn_blocking join: {e}")))?
    }

    /// Gets repository information.
    pub async fn get_repository<P: AsRef<Path>>(path: P) -> Result<GitRepository, GitError> {
        let path_buf = path.as_ref().to_path_buf();
        task::spawn_blocking(move || {
            let repo = Repository::open(&path_buf).map_err(|e| GitError::RepositoryNotFound(e.to_string()))?;

            let current_branch = get_current_branch(&repo)?;
            let is_bare = repo.is_bare();
            let has_changes = !get_file_statuses(&repo)?.is_empty();

            let remotes = repo
                .remotes()
                .map_err(|e| GitError::CommandFailed(e.to_string()))?
                .iter()
                .filter_map(|name| name.ok().flatten().map(str::to_string))
                .collect();

            let path_str = path_buf.to_string_lossy().to_string();
            let name = path_buf.file_name().unwrap_or_default().to_string_lossy().to_string();

            Ok(GitRepository {
                path: path_str,
                name,
                current_branch,
                is_bare,
                has_changes,
                remotes,
            })
        })
        .await
        .map_err(|e| GitError::CommandFailed(format!("spawn_blocking join: {e}")))?
    }

    /// Gets lightweight repository information without scanning worktree status.
    pub async fn get_repository_basic<P: AsRef<Path>>(path: P) -> Result<GitRepository, GitError> {
        let path_buf = path.as_ref().to_path_buf();
        task::spawn_blocking(move || {
            let repo = Repository::open(&path_buf).map_err(|e| GitError::RepositoryNotFound(e.to_string()))?;

            let current_branch = get_current_branch(&repo)?;
            let is_bare = repo.is_bare();
            let path_str = path_buf.to_string_lossy().to_string();
            let name = path_buf.file_name().unwrap_or_default().to_string_lossy().to_string();

            Ok(GitRepository {
                path: path_str,
                name,
                current_branch,
                is_bare,
                has_changes: false,
                remotes: Vec::new(),
            })
        })
        .await
        .map_err(|e| GitError::CommandFailed(format!("spawn_blocking join: {e}")))?
    }

    /// Gets repository status.
    pub async fn get_status<P: AsRef<Path>>(path: P) -> Result<GitStatus, GitError> {
        let path_buf = path.as_ref().to_path_buf();

        timeout(
            Duration::from_secs(10),
            task::spawn_blocking(move || {
                let repo = Repository::open(&path_buf).map_err(|e| GitError::RepositoryNotFound(e.to_string()))?;

                let current_branch = get_current_branch(&repo)?;
                let file_statuses = get_file_statuses(&repo)?;

                let mut staged = Vec::new();
                let mut unstaged = Vec::new();
                let mut untracked = Vec::new();

                for status in file_statuses {
                    if status.status.contains('C') {
                        staged.push(status.clone());
                        unstaged.push(status);
                    } else if status.status.contains('?') {
                        untracked.push(status.path);
                    } else {
                        if status.index_status.is_some() {
                            staged.push(status.clone());
                        }
                        if status.workdir_status.is_some() {
                            unstaged.push(status);
                        }
                    }
                }

                let (ahead, behind) = get_ahead_behind_count(&repo, &current_branch).unwrap_or((0, 0));

                Ok(GitStatus {
                    staged,
                    unstaged,
                    untracked,
                    current_branch,
                    ahead,
                    behind,
                })
            }),
        )
        .await
        .map_err(|_| GitError::CommandFailed("Git status timed out after 10s".to_string()))?
        .map_err(|e| GitError::CommandFailed(format!("spawn_blocking join: {e}")))?
    }

    /// Gets file content.
    ///
    /// # Parameters
    /// - `path`: Repository path
    /// - `file_path`: File relative path
    /// - `commit`: Commit reference (optional, defaults to `HEAD`)
    ///
    /// # Returns
    /// - File content string
    pub async fn get_file_content<P: AsRef<Path>>(
        path: P,
        file_path: &str,
        commit: Option<&str>,
    ) -> Result<String, GitError> {
        let repo_path = path.as_ref().to_string_lossy();

        let commit_ref = commit.unwrap_or("HEAD");
        let object_spec = format!("{}:{}", commit_ref, file_path);

        let args = vec!["show", &object_spec];

        execute_git_command(&repo_path, &args).await
    }

    /// Resets file changes (discarding working tree changes).
    ///
    /// # Parameters
    /// - `path`: Repository path
    /// - `files`: List of file paths
    /// - `staged`: Whether to reset the index (`true`: reset staged, `false`: restore worktree)
    ///
    /// # Returns
    /// - Operation result
    pub async fn reset_files<P: AsRef<Path>>(path: P, files: &[String], staged: bool) -> Result<String, GitError> {
        let repo_path = path.as_ref().to_string_lossy();

        if staged {
            let mut args = vec!["restore", "--staged"];
            for file in files {
                args.push(file);
            }
            execute_git_command(&repo_path, &args).await
        } else {
            let mut args = vec!["restore"];
            for file in files {
                args.push(file);
            }
            execute_git_command(&repo_path, &args).await
        }
    }
}
