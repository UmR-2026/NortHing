//! Workspace search session lifecycle and shutdown methods.
//!
//! Implements [`WorkspaceSearchService`] session/open/shutdown methods, the
//! private [`SessionEntry`] helpers, and path normalization utilities used by
//! the search and index siblings.

use super::flashgrep::{
    FlashgrepRepoSession, OpenRepoParams, RefreshPolicyConfig, RepoConfig, RepoSession, FLASHGREP_LOG_TARGET,
};
use super::service::{SessionEntry, WorkspaceSearchResult, WorkspaceSearchService};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::Mutex;

impl WorkspaceSearchService {
    pub async fn schedule_repo_release(self: &Arc<Self>, repo_root: impl AsRef<Path>) {
        let Ok(repo_root) = normalize_repo_root(repo_root.as_ref()) else {
            return;
        };
        let delay = self.session_idle_grace;
        let service = Arc::downgrade(self);
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let Some(service) = service.upgrade() else {
                return;
            };
            service.release_repo_if_idle(repo_root).await;
        });
    }

    pub async fn shutdown_all_daemons(&self) {
        let released_sessions = self.sessions.write().await.drain().count();
        self.open_guards.lock().await.clear();
        if released_sessions > 0 {
            tracing::info!(
                target: FLASHGREP_LOG_TARGET,
                "Workspace search shutdown releasing sessions via daemon shutdown: count={}",
                released_sessions
            );
        }
        if let Err(error) = self.client.shutdown_daemon().await {
            tracing::debug!(
                target: FLASHGREP_LOG_TARGET,
                "Workspace search daemon shutdown skipped: {}",
                error
            );
        }
    }

    pub async fn stop_all_daemons(&self) {
        let released_sessions = self.sessions.write().await.drain().count();
        self.open_guards.lock().await.clear();
        if released_sessions > 0 {
            tracing::info!(
                target: FLASHGREP_LOG_TARGET,
                "Workspace search stop releasing sessions via daemon stop: count={}",
                released_sessions
            );
        }
        if let Err(error) = self.client.stop_daemon().await {
            tracing::debug!(
                target: FLASHGREP_LOG_TARGET,
                "Workspace search daemon stop skipped: {}",
                error
            );
        }
    }

    pub fn shutdown_blocking(self: &Arc<Self>) {
        let service = Arc::clone(self);
        match std::thread::Builder::new()
            .name("workspace-search-shutdown".to_string())
            .spawn(
                move || match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(runtime) => {
                        runtime.block_on(async move {
                            service.shutdown_all_daemons().await;
                        });
                    }
                    Err(error) => {
                        tracing::warn!(
                            target: FLASHGREP_LOG_TARGET,
                            "Failed to create runtime for workspace search shutdown: {}",
                            error
                        );
                    }
                },
            ) {
            Ok(handle) => {
                if handle.join().is_err() {
                    tracing::warn!(
                        target: FLASHGREP_LOG_TARGET,
                        "Workspace search shutdown thread panicked during blocking shutdown"
                    );
                }
            }
            Err(error) => {
                tracing::warn!(
                    target: FLASHGREP_LOG_TARGET,
                    "Failed to spawn workspace search shutdown thread: {}",
                    error
                );
            }
        }
    }

    pub(super) async fn get_or_open_session(&self, repo_root: &Path) -> WorkspaceSearchResult<Arc<RepoSession>> {
        let repo_root = normalize_repo_root(repo_root)?;
        let repo_guard = {
            let mut guards = self.open_guards.lock().await;
            guards
                .entry(repo_root.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _repo_guard = repo_guard.lock().await;

        if let Some(existing) = self.sessions.read().await.get(&repo_root).cloned() {
            existing.activity_epoch.fetch_add(1, Ordering::Relaxed);
            if existing.session.status().await.is_ok() {
                return Ok(existing.session);
            }
            tracing::warn!(
                target: FLASHGREP_LOG_TARGET,
                "Workspace search session became unhealthy, reopening repository session: path={}",
                repo_root.display()
            );
            self.sessions.write().await.remove(&repo_root);
            if let Err(error) = existing.session.close().await {
                tracing::debug!(
                    target: FLASHGREP_LOG_TARGET,
                    "Workspace search repo close after unhealthy session failed: path={}, error={}",
                    repo_root.display(),
                    error
                );
            }
        }

        let repo_config: RepoConfig = self.hooks.repo_config().await.into();
        if let Err(error) = self.hooks.ensure_workspace_ready(&repo_root).await {
            tracing::warn!(
                target: FLASHGREP_LOG_TARGET,
                "Failed to ensure workspace .gitignore ignores .northhing before search warmup: path={}, error={}",
                repo_root.display(),
                error
            );
        }
        let params = OpenRepoParams {
            repo_path: repo_root.clone(),
            storage_root: Some(default_storage_root(&repo_root)),
            config: repo_config,
            refresh: RefreshPolicyConfig::default(),
        };
        let storage_root = params
            .storage_root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "-".to_string());

        let entry = SessionEntry {
            session: Arc::new(self.client.open_repo(params).await.map_err(
                super::service_search::map_flashgrep_error("Failed to open flashgrep repository session"),
            )?),
            activity_epoch: Arc::new(AtomicU64::new(1)),
        };
        tracing::info!(
            target: FLASHGREP_LOG_TARGET,
            "Opened workspace search repository session: path={}, storage_root={}",
            repo_root.display(),
            storage_root
        );

        let mut sessions = self.sessions.write().await;
        Ok(sessions
            .entry(repo_root)
            .or_insert_with(|| entry.clone())
            .session
            .clone())
    }

    pub(super) async fn release_repo_if_idle(&self, repo_root: PathBuf) {
        let Some(expected_epoch) = self
            .sessions
            .read()
            .await
            .get(&repo_root)
            .map(|entry| entry.activity_epoch.load(Ordering::Relaxed))
        else {
            return;
        };

        let entry = {
            let mut sessions = self.sessions.write().await;
            let Some(entry) = sessions.get(&repo_root) else {
                return;
            };
            if entry.activity_epoch.load(Ordering::Relaxed) != expected_epoch {
                return;
            }
            sessions.remove(&repo_root)
        };

        if let Some(entry) = entry {
            tracing::debug!(
                target: FLASHGREP_LOG_TARGET,
                "Releasing idle workspace search repository session: path={}",
                repo_root.display()
            );
            if let Err(error) = FlashgrepRepoSession::close(entry.session.as_ref()).await {
                tracing::warn!(
                    target: FLASHGREP_LOG_TARGET,
                    "Failed to release idle workspace search repository session: path={}, error={}",
                    repo_root.display(),
                    error
                );
            }
            self.open_guards.lock().await.remove(&repo_root);
        }
    }
}

pub(super) fn normalize_repo_root(repo_root: &Path) -> WorkspaceSearchResult<PathBuf> {
    if !repo_root.exists() {
        return Err(format!("Search root does not exist: {}", repo_root.display()));
    }
    if !repo_root.is_dir() {
        return Err(format!("Search root is not a directory: {}", repo_root.display()));
    }

    dunce::canonicalize(repo_root)
        .map_err(|error| format!("Failed to normalize search root {}: {}", repo_root.display(), error))
}

pub(super) fn default_storage_root(repo_root: &Path) -> PathBuf {
    repo_root.join(".northhing").join("search").join("flashgrep-index")
}
