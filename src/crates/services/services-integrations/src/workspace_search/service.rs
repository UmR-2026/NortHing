//! Local workspace search service facade.
//!
//! [`WorkspaceSearchService`] owns the local flashgrep daemon/session
//! lifecycle. The public surface re-exports through `workspace_search::*`.
//! Implementation methods live in sibling files (`service_index`,
//! `service_search`, `service_session`, `service_daemon`); this module
//! keeps the public API, type aliases, runtime hooks, struct definition,
//! constructors, daemon binary public API, and the unit-test module.

use super::flashgrep::{ManagedClient, RepoConfig, FLASHGREP_LOG_TARGET};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{atomic::AtomicU64, Arc};
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};

pub type WorkspaceSearchResult<T> = Result<T, String>;

#[derive(Debug, Clone)]
pub struct WorkspaceSearchRepoConfig {
    pub max_file_size: u64,
}

impl Default for WorkspaceSearchRepoConfig {
    fn default() -> Self {
        let default = RepoConfig::default();
        Self {
            max_file_size: default.max_file_size,
        }
    }
}

impl From<WorkspaceSearchRepoConfig> for RepoConfig {
    fn from(value: WorkspaceSearchRepoConfig) -> Self {
        RepoConfig {
            max_file_size: value.max_file_size,
            ..Default::default()
        }
    }
}

#[async_trait]
pub trait WorkspaceSearchRuntimeHooks: Send + Sync {
    async fn repo_config(&self) -> WorkspaceSearchRepoConfig;

    async fn ensure_workspace_ready(&self, _repo_root: &Path) -> WorkspaceSearchResult<()> {
        Ok(())
    }
}

struct DefaultWorkspaceSearchRuntimeHooks;

#[async_trait]
impl WorkspaceSearchRuntimeHooks for DefaultWorkspaceSearchRuntimeHooks {
    async fn repo_config(&self) -> WorkspaceSearchRepoConfig {
        WorkspaceSearchRepoConfig::default()
    }
}

pub(super) const DEFAULT_TOP_K_TOKENS: usize = 6;
pub(super) const DEFAULT_SESSION_IDLE_GRACE: Duration = Duration::from_secs(45);

#[derive(Debug, Clone)]
pub(super) struct SessionEntry {
    pub(super) session: Arc<super::flashgrep::RepoSession>,
    pub(super) activity_epoch: Arc<AtomicU64>,
}

pub struct WorkspaceSearchService {
    pub(super) client: ManagedClient,
    pub(super) sessions: RwLock<HashMap<PathBuf, SessionEntry>>,
    pub(super) open_guards: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
    pub(super) session_idle_grace: Duration,
    pub(super) hooks: Arc<dyn WorkspaceSearchRuntimeHooks>,
}

impl WorkspaceSearchService {
    pub fn new() -> Self {
        Self::new_with_hooks(Arc::new(DefaultWorkspaceSearchRuntimeHooks))
    }

    pub fn new_with_hooks(hooks: Arc<dyn WorkspaceSearchRuntimeHooks>) -> Self {
        let mut client = ManagedClient::new()
            .with_start_timeout(Duration::from_secs(10))
            .with_retry_interval(Duration::from_millis(100));
        let program = super::service_daemon::resolve_daemon_program();
        if let Some(program) = program {
            tracing::info!(
                target: FLASHGREP_LOG_TARGET,
                "WorkspaceSearchService daemon configured: program={}",
                PathBuf::from(&program).display()
            );
            client = client.with_daemon_program(program);
        } else {
            tracing::info!(
                target: FLASHGREP_LOG_TARGET,
                "WorkspaceSearchService daemon configured: program=flashgrep"
            );
        }

        Self {
            client,
            sessions: RwLock::new(HashMap::new()),
            open_guards: Mutex::new(HashMap::new()),
            session_idle_grace: DEFAULT_SESSION_IDLE_GRACE,
            hooks,
        }
    }
}

impl Default for WorkspaceSearchService {
    fn default() -> Self {
        Self::new()
    }
}

pub fn workspace_search_daemon_binary_names() -> &'static [&'static str] {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        &["flashgrep-x86_64-pc-windows-msvc.exe"]
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        &["flashgrep-aarch64-pc-windows-msvc.exe"]
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        &["flashgrep-x86_64-apple-darwin"]
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        &["flashgrep-aarch64-apple-darwin"]
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        &[
            "flashgrep-x86_64-unknown-linux-musl",
            "flashgrep-x86_64-unknown-linux-gnu",
        ]
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        &[
            "flashgrep-aarch64-unknown-linux-musl",
            "flashgrep-aarch64-unknown-linux-gnu",
        ]
    } else if cfg!(windows) {
        &["flashgrep.exe"]
    } else {
        &["flashgrep"]
    }
}

pub fn workspace_search_daemon_binary_name() -> &'static str {
    workspace_search_daemon_binary_names()
        .first()
        .copied()
        .unwrap_or("flashgrep")
}

pub fn workspace_search_daemon_missing_hint() -> String {
    let bundled_paths = workspace_search_daemon_binary_names()
        .iter()
        .map(|name| format!("flashgrep/{name}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "workspace search daemon binary is missing; expected one of bundled resources [{}] or a valid FLASHGREP_DAEMON_BIN override",
        bundled_paths
    )
}

pub fn workspace_search_daemon_available() -> bool {
    resolve_workspace_search_daemon_program_path().is_some()
}

pub fn resolve_workspace_search_daemon_program_path() -> Option<PathBuf> {
    if let Some(program) = std::env::var_os("FLASHGREP_DAEMON_BIN") {
        let path = PathBuf::from(program);
        if path.exists() {
            return Some(path);
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("../../../..");
    let binary_names = workspace_search_daemon_binary_names();
    let profile = std::env::var("PROFILE").ok();

    for candidate in super::service_daemon::daemon_binary_candidates(&workspace_root, binary_names, profile.as_deref())
    {
        if candidate.exists() {
            return Some(candidate);
        }
    }

    which::which("flashgrep").ok()
}

#[cfg(test)]
mod tests {
    use super::super::flashgrep::SearchResults;
    use super::super::result_mapping::convert_search_results;
    use super::super::types::ContentSearchOutputMode;

    fn empty_search_results() -> SearchResults {
        serde_json::from_value(serde_json::json!({
            "candidate_docs": 0,
            "searches_with_match": 0,
            "bytes_searched": 0,
            "matched_lines": 0,
            "matched_occurrences": 0
        }))
        .expect("empty search results should decode with defaulted collections")
    }

    #[test]
    fn content_search_output_modes_use_current_flashgrep_protocol_modes() {
        assert_eq!(
            ContentSearchOutputMode::Content.search_mode(),
            super::super::flashgrep::SearchModeConfig::LineMatches
        );
        assert_eq!(
            ContentSearchOutputMode::Count.search_mode(),
            super::super::flashgrep::SearchModeConfig::CountOnly
        );
        assert_eq!(
            ContentSearchOutputMode::FilesWithMatches.search_mode(),
            super::super::flashgrep::SearchModeConfig::FilesWithMatches
        );
    }

    #[test]
    fn content_search_converts_legacy_line_matches() {
        let mut search_results = empty_search_results();
        search_results.line_matches = serde_json::from_value(serde_json::json!([{
            "path": "src/search.rs",
            "line_number": 42,
            "line_text": "pub enum SearchMode"
        }]))
        .expect("legacy line_matches should decode");

        let results = convert_search_results(&search_results, ContentSearchOutputMode::Content);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "src/search.rs");
        assert_eq!(results[0].name, "search.rs");
        assert_eq!(results[0].line_number, Some(42));
        assert_eq!(results[0].matched_content.as_deref(), Some("pub enum SearchMode"));
    }
}
