// R39c helpers extracted from service.rs (L1005-1315 of original)
// Session-key free fn helpers used by RemoteWorkspaceSearchService.

use super::repo_session::RemoteStdioSessionEntry;
use super::service::{
    REMOTE_STDIO_OPEN_GUARDS, REMOTE_STDIO_SESSIONS, REMOTE_STDIO_SESSION_IDLE_GRACE,
};
use crate::remote_ssh::normalize_remote_workspace_path;
use crate::workspace_search::flashgrep::FLASHGREP_LOG_TARGET;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, LazyLock,
};
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;

pub(super) fn remote_stdio_session_key(connection_id: &str, repo_root: &str) -> String {
    format!("{connection_id}\0{}", normalize_remote_workspace_path(repo_root))
}

pub(super) fn remote_search_context_key(connection_id: &str, repo_root: &str) -> String {
    format!("{connection_id}\0{}", normalize_remote_workspace_path(repo_root))
}

pub(super) fn schedule_remote_stdio_session_release(key: String, activity_epoch: Arc<AtomicU64>) {
    tokio::spawn(async move {
        let expected_epoch = activity_epoch.load(Ordering::Relaxed);
        sleep(REMOTE_STDIO_SESSION_IDLE_GRACE).await;
        let entry = {
            let sessions = REMOTE_STDIO_SESSIONS.read().await;
            let Some(entry) = sessions.get(&key) else {
                return;
            };
            if entry.session.active_operations.load(Ordering::Relaxed) > 0 {
                schedule_remote_stdio_session_release(key.clone(), entry.activity_epoch.clone());
                return;
            }
            if entry.activity_epoch.load(Ordering::Relaxed) != expected_epoch {
                schedule_remote_stdio_session_release(key.clone(), entry.activity_epoch.clone());
                return;
            }
            entry.clone()
        };

        match entry.session.status_without_activity_lease().await {
            Ok(status) if status.active_task_id.is_some() => {
                schedule_remote_stdio_session_release(key.clone(), entry.activity_epoch.clone());
                return;
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(
                    target: FLASHGREP_LOG_TARGET,
                    "Failed to check idle remote workspace search status before release: key={}, error={}",
                    key.replace('\0', ":"),
                    error
                );
            }
        }

        let entry = {
            let mut sessions = REMOTE_STDIO_SESSIONS.write().await;
            let Some(current_entry) = sessions.get(&key) else {
                return;
            };
            if !Arc::ptr_eq(&current_entry.session, &entry.session) {
                return;
            }
            if current_entry.session.active_operations.load(Ordering::Relaxed) > 0 {
                schedule_remote_stdio_session_release(key.clone(), current_entry.activity_epoch.clone());
                return;
            }
            if current_entry.activity_epoch.load(Ordering::Relaxed) != expected_epoch {
                schedule_remote_stdio_session_release(key.clone(), current_entry.activity_epoch.clone());
                return;
            }
            sessions.remove(&key)
        };

        if let Some(entry) = entry {
            tracing::debug!(
                target: FLASHGREP_LOG_TARGET,
                "Releasing idle remote workspace search stdio session: key={}",
                key.replace('\0', ":")
            );
            entry.session.close().await;
            entry.session.client.shutdown().await;
            REMOTE_STDIO_OPEN_GUARDS.lock().await.remove(&key);
        }
    });
}

#[cfg(test)]
pub(crate) fn test_remote_stdio_session_key(connection_id: &str, repo_root: &str) -> String {
    remote_stdio_session_key(connection_id, repo_root)
}

#[cfg(test)]
pub(crate) fn test_remote_search_context_key(connection_id: &str, repo_root: &str) -> String {
    remote_search_context_key(connection_id, repo_root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote_ssh::RemoteWorkspaceEntry;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tokio::sync::mpsc;

    use super::super::protocol::{
        RemoteCommandOutput, RemoteWorkspaceSearchProvider, RemoteWorkspaceSearchStdioProtocol,
    };
    use super::super::repo_session::RemoteStdioRepoSession;
    use super::super::service::{
        RemoteSearchContext, RemoteWorkspaceSearchService, REMOTE_SEARCH_CONTEXTS,
    };

    static REMOTE_SEARCH_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    async fn clear_remote_search_test_state() {
        REMOTE_STDIO_SESSIONS.write().await.clear();
        REMOTE_STDIO_OPEN_GUARDS.lock().await.clear();
        REMOTE_SEARCH_CONTEXTS.write().await.clear();
    }

    #[test]
    fn remote_search_cache_keys_normalize_workspace_root() {
        assert_eq!(
            test_remote_stdio_session_key("conn-1", "/home/user/repo/"),
            "conn-1\0/home/user/repo"
        );
        assert_eq!(
            test_remote_search_context_key("conn-1", "/home/user/repo/"),
            "conn-1\0/home/user/repo"
        );
    }

    #[tokio::test]
    async fn remote_search_rejects_non_linux_before_stdio_open() {
        let _test_guard = REMOTE_SEARCH_TEST_LOCK.lock().await;
        clear_remote_search_test_state().await;
        let provider = Arc::new(FakeRemoteSearchProvider {
            cached_os_type: Some("Darwin".to_string()),
            connection_id: "conn-1".to_string(),
            remote_root: "/Users/example/project".to_string(),
            fail_stdio_spawn: false,
            resolve_count: AtomicU64::new(0),
            stdio_spawn_count: AtomicU64::new(0),
        });
        let service = RemoteWorkspaceSearchService::new(provider.clone());

        let error = service
            .get_index_status("/Users/example/project")
            .await
            .expect_err("non-linux remotes must fail before opening flashgrep");

        assert!(error.contains("supports Linux only"));
        assert!(error.contains("Darwin"));
        assert_eq!(provider.stdio_spawn_count.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn remote_search_context_ignores_stale_cache_before_resolving_connection() {
        let _test_guard = REMOTE_SEARCH_TEST_LOCK.lock().await;
        clear_remote_search_test_state().await;
        let repo_root = "/home/user/repo";
        let stale_empty_connection_key = format!("\0{repo_root}");
        REMOTE_SEARCH_CONTEXTS.write().await.insert(
            stale_empty_connection_key,
            RemoteSearchContext {
                connection: RemoteWorkspaceEntry {
                    connection_id: "conn-stale".to_string(),
                    connection_name: "stale".to_string(),
                    ssh_host: "stale.example.test".to_string(),
                    remote_root: repo_root.to_string(),
                },
                binary_path: "/stale/flashgrep".to_string(),
                repo_root: repo_root.to_string(),
                storage_root: "/stale/search".to_string(),
                remote_arch: "riscv64".to_string(),
                local_binary_sha256: "stale".to_string(),
            },
        );
        let provider = Arc::new(FakeRemoteSearchProvider {
            cached_os_type: Some("Darwin".to_string()),
            connection_id: "conn-new".to_string(),
            remote_root: repo_root.to_string(),
            fail_stdio_spawn: false,
            resolve_count: AtomicU64::new(0),
            stdio_spawn_count: AtomicU64::new(0),
        });
        let service = RemoteWorkspaceSearchService::new(provider.clone());

        let error = service
            .get_index_status(repo_root)
            .await
            .expect_err("resolved non-Linux connection should reject without using stale cache");

        assert_eq!(provider.resolve_count.load(Ordering::Relaxed), 1);
        assert!(error.contains("Darwin"));
        assert!(!error.contains("riscv64"));
    }

    #[tokio::test]
    async fn remote_search_open_guard_is_removed_when_stdio_spawn_fails() {
        let _test_guard = REMOTE_SEARCH_TEST_LOCK.lock().await;
        clear_remote_search_test_state().await;
        let repo_root = "/home/user/repo";
        let provider = Arc::new(FakeRemoteSearchProvider {
            cached_os_type: Some("Linux".to_string()),
            connection_id: "conn-guard".to_string(),
            remote_root: repo_root.to_string(),
            fail_stdio_spawn: true,
            resolve_count: AtomicU64::new(0),
            stdio_spawn_count: AtomicU64::new(0),
        });
        let service = RemoteWorkspaceSearchService::new(provider.clone());

        let error = service
            .get_index_status(repo_root)
            .await
            .expect_err("fake provider rejects stdio spawn");

        assert!(error.contains("spawn failed"));
        assert_eq!(provider.stdio_spawn_count.load(Ordering::Relaxed), 1);
        let key = remote_stdio_session_key("conn-guard", repo_root);
        assert!(
            !REMOTE_STDIO_OPEN_GUARDS.lock().await.contains_key(&key),
            "failed stdio opens must not leave a global guard entry behind"
        );
    }

    struct FakeRemoteSearchProvider {
        cached_os_type: Option<String>,
        connection_id: String,
        remote_root: String,
        fail_stdio_spawn: bool,
        resolve_count: AtomicU64,
        stdio_spawn_count: AtomicU64,
    }

    #[async_trait]
    impl RemoteWorkspaceSearchProvider for FakeRemoteSearchProvider {
        async fn resolve_workspace_entry(
            &self,
            _root_path: &str,
            _preferred_connection_id: Option<&str>,
        ) -> Result<RemoteWorkspaceEntry, String> {
            self.resolve_count.fetch_add(1, Ordering::Relaxed);
            Ok(RemoteWorkspaceEntry {
                connection_id: self.connection_id.clone(),
                connection_name: "test".to_string(),
                ssh_host: "example.test".to_string(),
                remote_root: self.remote_root.clone(),
            })
        }

        async fn cached_server_os_type(&self, _connection_id: &str) -> Option<String> {
            self.cached_os_type.clone()
        }

        async fn execute_command(&self, _connection_id: &str, command: &str) -> Result<RemoteCommandOutput, String> {
            if command == "uname -m" || command == "arch" || command.contains("uname -m") {
                return Ok(RemoteCommandOutput {
                    stdout: "x86_64\n".to_string(),
                    stderr: String::new(),
                    exit_code: 0,
                });
            }
            if command.contains("sha256sum") || command.starts_with("mv -f ") || command.starts_with("chmod 755 ") {
                return Ok(RemoteCommandOutput {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: 0,
                });
            }
            Err(format!("unexpected remote command: {command}"))
        }

        async fn create_dir_all(&self, _connection_id: &str, _path: &str) -> Result<(), String> {
            Ok(())
        }

        async fn write_file(&self, _connection_id: &str, _path: &str, _contents: &[u8]) -> Result<(), String> {
            Ok(())
        }

        async fn repo_max_file_size(&self) -> u64 {
            0
        }

        async fn spawn_stdio_daemon(
            &self,
            _connection_id: &str,
            _command: &str,
            _write_rx: mpsc::Receiver<Vec<u8>>,
            _protocol: RemoteWorkspaceSearchStdioProtocol,
        ) -> Result<(), String> {
            self.stdio_spawn_count.fetch_add(1, Ordering::Relaxed);
            if self.fail_stdio_spawn {
                Err("spawn failed".to_string())
            } else {
                Err("unexpected stdio spawn".to_string())
            }
        }
    }
}
