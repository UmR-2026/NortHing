use super::repo_session::{RemoteStdioSessionEntry, RemoteStdioSessionLease};
use super::service_helpers::{
    remote_search_context_key, remote_stdio_session_key, schedule_remote_stdio_session_release,
};
use super::{
    build_remote_scope, join_remote_path, local_flashgrep_bundle_for_arch, looks_like_linux_workspace_root,
    parse_remote_architecture_output, parse_remote_os_output, remote_flashgrep_install_dir, remote_stdio_search_mode,
    remote_workspace_search_storage_root, shell_escape, should_retry_remote_scan_fallback_as_files_with_matches,
    LocalFlashgrepBundle, REMOTE_ARCHITECTURE_PROBES, REMOTE_OS_PROBES,
};
use crate::remote_ssh::{normalize_remote_workspace_path, RemoteWorkspaceEntry};
use crate::workspace_search::flashgrep::error::AppError;
use crate::workspace_search::flashgrep::{
    drain_content_length_messages, log_flashgrep_stderr_line_with_context, ClientCapabilities, ClientInfo,
    ConsistencyMode, GlobOutcome, GlobParams, GlobRequest, FlashgrepInitializeParams, OpenRepoParams,
    ProtocolClient, QuerySpec, RefreshPolicyConfig, RepoConfig, RepoRef, RepoStatus, Request, Response, SearchBackend,
    SearchModeConfig, SearchOutcome, SearchParams, SearchRequest, SearchResults, TaskRef, TaskStatus,
    FLASHGREP_LOG_TARGET,
};
use crate::workspace_search::result_mapping::convert_search_results;
use crate::workspace_search::{
    ContentSearchRequest, ContentSearchResult, GlobSearchRequest, GlobSearchResult, IndexTaskHandle,
    WorkspaceIndexStatus, WorkspaceSearchFileCount, WorkspaceSearchHit, WorkspaceSearchRepoStatus,
};
use async_trait::async_trait;
use northhing_services_core::filesystem::FileSearchOutcome;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, LazyLock,
};
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, timeout};

pub(super) const REMOTE_STDIO_SESSION_IDLE_GRACE: Duration = Duration::from_secs(45);

pub(super) static REMOTE_STDIO_SESSIONS: LazyLock<RwLock<HashMap<String, RemoteStdioSessionEntry>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
pub(super) static REMOTE_STDIO_OPEN_GUARDS: LazyLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
pub(super) static REMOTE_SEARCH_CONTEXTS: LazyLock<RwLock<HashMap<String, RemoteSearchContext>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Clone)]
pub struct RemoteWorkspaceSearchService {
    provider: Arc<dyn super::protocol::RemoteWorkspaceSearchProvider>,
    preferred_connection_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct RemoteSearchContext {
    pub(super) connection: RemoteWorkspaceEntry,
    pub(super) binary_path: String,
    pub(super) repo_root: String,
    pub(super) storage_root: String,
    pub(super) remote_arch: String,
    pub(super) local_binary_sha256: String,
}

impl RemoteWorkspaceSearchService {
    pub fn new(provider: Arc<dyn super::protocol::RemoteWorkspaceSearchProvider>) -> Self {
        Self {
            provider,
            preferred_connection_id: None,
        }
    }

    pub fn with_preferred_connection_id(mut self, preferred_connection_id: Option<String>) -> Self {
        self.preferred_connection_id = preferred_connection_id;
        self
    }

    pub async fn get_index_status(&self, root_path: &str) -> Result<WorkspaceIndexStatus, String> {
        let session = self.get_or_open_stdio_session(root_path).await?;
        let repo_status: WorkspaceSearchRepoStatus = session.status().await?.into();
        let active_task = match repo_status.active_task_id.clone() {
            Some(task_id) => match session.task_status(task_id).await {
                Ok(task) => Some(task.into()),
                Err(error) => {
                    tracing::warn!(
                        target: FLASHGREP_LOG_TARGET,
                        "Failed to fetch active remote flashgrep task status: {}",
                        error
                    );
                    None
                }
            },
            None => None,
        };
        Ok(WorkspaceIndexStatus {
            active_task,
            repo_status,
        })
    }

    pub async fn build_index(&self, root_path: &str) -> Result<IndexTaskHandle, String> {
        let session = self.get_or_open_stdio_session(root_path).await?;
        let task = session.build_index().await?;
        let repo_status = session.status().await?;
        Ok(IndexTaskHandle {
            task: task.into(),
            repo_status: repo_status.into(),
        })
    }

    pub async fn rebuild_index(&self, root_path: &str) -> Result<IndexTaskHandle, String> {
        let session = self.get_or_open_stdio_session(root_path).await?;
        let task = session.rebuild_index().await?;
        let repo_status = session.status().await?;
        Ok(IndexTaskHandle {
            task: task.into(),
            repo_status: repo_status.into(),
        })
    }

    pub async fn search_content(&self, request: ContentSearchRequest) -> Result<ContentSearchResult, String> {
        let repo_root = normalize_remote_workspace_path(&request.repo_root.to_string_lossy());
        let session = self.get_or_open_stdio_session(&repo_root).await?;
        let scope = build_remote_scope(
            &repo_root,
            request.search_path.as_deref(),
            request.globs,
            request.file_types,
            request.exclude_file_types,
        )?;
        let max_results = request.max_results.filter(|limit| *limit > 0);
        let primary_search_mode = remote_stdio_search_mode(request.output_mode);
        let query = QuerySpec {
            pattern: request.pattern.clone(),
            patterns: Vec::new(),
            case_insensitive: !request.case_sensitive,
            multiline: request.multiline,
            dot_matches_new_line: request.multiline,
            fixed_strings: !request.use_regex,
            word_regexp: request.whole_word,
            line_regexp: false,
            before_context: request.before_context,
            after_context: request.after_context,
            top_k_tokens: 6,
            max_count: None,
            global_max_results: max_results,
            search_mode: primary_search_mode,
        };

        let output_mode = request.output_mode;
        let (backend, repo_status, mut raw_results) = session.search(query, scope.clone()).await?;
        if should_retry_remote_scan_fallback_as_files_with_matches(backend, primary_search_mode, &raw_results) {
            tracing::info!(
                "Remote workspace content search re-issuing as FilesWithMatches because daemon ScanFallback returned only summary statistics: pattern_chars={}, primary_search_mode={:?}, primary_matched_lines={}, primary_matched_occurrences={}",
                request.pattern.chars().count(),
                primary_search_mode,
                raw_results.matched_lines,
                raw_results.matched_occurrences,
            );
            let fallback_query = QuerySpec {
                pattern: request.pattern.clone(),
                patterns: Vec::new(),
                case_insensitive: !request.case_sensitive,
                multiline: request.multiline,
                dot_matches_new_line: request.multiline,
                fixed_strings: !request.use_regex,
                word_regexp: request.whole_word,
                line_regexp: false,
                before_context: request.before_context,
                after_context: request.after_context,
                top_k_tokens: 6,
                max_count: None,
                global_max_results: max_results,
                search_mode: SearchModeConfig::FilesWithMatches,
            };
            match session.search(fallback_query, scope).await {
                Ok((_, _, fallback_results)) => {
                    tracing::info!(
                        "Remote workspace content search FilesWithMatches fallback succeeded: matched_paths={}, matched_lines={}, matched_occurrences={}",
                        fallback_results.matched_paths.len(),
                        fallback_results.matched_lines,
                        fallback_results.matched_occurrences,
                    );
                    raw_results = fallback_results;
                }
                Err(error) => {
                    tracing::warn!(
                        "Remote workspace content search FilesWithMatches fallback failed: pattern_chars={}, primary_matched_lines={}, primary_matched_occurrences={}, error={}",
                        request.pattern.chars().count(),
                        raw_results.matched_lines,
                        raw_results.matched_occurrences,
                        error,
                    );
                    return Err(format!(
                        "Remote workspace search returned only summary statistics for {primary_matched_lines} line(s) and the file-list fallback failed: {error}",
                        primary_matched_lines = raw_results.matched_lines,
                    ));
                }
            }
        }

        let mut results = convert_search_results(&raw_results, output_mode);
        tracing::debug!(
            "Remote workspace content search converted: backend={:?}, repo_phase={:?}, hits={}, file_counts={}, file_match_counts={}, matched_paths={}, converted_results={}, matched_lines={}, matched_occurrences={}",
            backend,
            repo_status.phase,
            raw_results.hits.len(),
            raw_results.file_counts.len(),
            raw_results.file_match_counts.len(),
            raw_results.matched_paths.len(),
            results.len(),
            raw_results.matched_lines,
            raw_results.matched_occurrences
        );
        let truncated = max_results.map(|limit| results.len() >= limit).unwrap_or(false);
        if let Some(limit) = max_results {
            results.truncate(limit);
        }

        Ok(ContentSearchResult {
            outcome: FileSearchOutcome { results, truncated },
            file_counts: raw_results
                .file_counts
                .clone()
                .into_iter()
                .map(WorkspaceSearchFileCount::from)
                .collect(),
            hits: raw_results
                .hits
                .clone()
                .into_iter()
                .map(WorkspaceSearchHit::from)
                .collect(),
            backend: backend.into(),
            repo_status: repo_status.into(),
            candidate_docs: raw_results.candidate_docs,
            matched_lines: raw_results.matched_lines,
            matched_occurrences: raw_results.matched_occurrences,
        })
    }

    pub async fn glob(&self, request: GlobSearchRequest) -> Result<GlobSearchResult, String> {
        let repo_root = normalize_remote_workspace_path(&request.repo_root.to_string_lossy());
        let session = self.get_or_open_stdio_session(&repo_root).await?;
        let scope = build_remote_scope(
            &repo_root,
            request.search_path.as_deref(),
            vec![request.pattern],
            Vec::new(),
            Vec::new(),
        )?;
        let (repo_status, mut paths) = session.glob(scope).await?;

        paths.sort();
        if request.limit > 0 {
            paths.truncate(request.limit);
        } else {
            paths.clear();
        }

        Ok(GlobSearchResult {
            paths,
            repo_status: repo_status.into(),
        })
    }

    pub async fn resolve_remote_workspace_entry(&self, root_path: &str) -> Result<RemoteWorkspaceEntry, String> {
        self.provider
            .resolve_workspace_entry(root_path, self.preferred_connection_id.as_deref())
            .await
    }

    async fn get_or_open_stdio_session(&self, root_path: &str) -> Result<RemoteStdioSessionLease, String> {
        let context = self.ensure_remote_search_context(root_path).await?;
        let key = remote_stdio_session_key(&context.connection.connection_id, &context.repo_root);

        if let Some(entry) = REMOTE_STDIO_SESSIONS.read().await.get(&key).cloned() {
            entry.activity_epoch.fetch_add(1, Ordering::Relaxed);
            if !entry.session.client.is_closed() {
                return Ok(RemoteStdioSessionLease::new(entry.session.clone()));
            }
            tracing::warn!(
                target: FLASHGREP_LOG_TARGET,
                "Remote workspace search stdio session became unhealthy, reopening: connection_id={}, path={}",
                context.connection.connection_id,
                context.repo_root
            );
            REMOTE_STDIO_SESSIONS.write().await.remove(&key);
            entry.session.close().await;
            entry.session.client.shutdown().await;
        }

        let guard = {
            let mut guards = REMOTE_STDIO_OPEN_GUARDS.lock().await;
            guards
                .entry(key.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _open_guard = guard.lock().await;

        if let Some(entry) = REMOTE_STDIO_SESSIONS.read().await.get(&key).cloned() {
            entry.activity_epoch.fetch_add(1, Ordering::Relaxed);
            return Ok(RemoteStdioSessionLease::new(entry.session));
        }

        let open_result = async {
            let client = super::repo_session::RemoteStdioDaemonClient::spawn(
                self.provider.clone(),
                context.connection.connection_id.clone(),
                context.binary_path.clone(),
            )
            .await?;
            let repo_config = RepoConfig {
                max_file_size: self.provider.repo_max_file_size().await,
                ..Default::default()
            };
            let session = match client
                .open_repo(OpenRepoParams {
                    repo_path: PathBuf::from(&context.repo_root),
                    storage_root: Some(PathBuf::from(&context.storage_root)),
                    config: repo_config,
                    refresh: RefreshPolicyConfig::default(),
                })
                .await
            {
                Ok(session) => session,
                Err(error) => {
                    client.shutdown().await;
                    return Err(error);
                }
            };
            let activity_epoch = session.activity_epoch.clone();
            Ok::<_, String>((Arc::new(session), activity_epoch))
        }
        .await;
        let (session, activity_epoch) = match open_result {
            Ok(opened) => opened,
            Err(error) => {
                if Arc::strong_count(&guard) <= 2 {
                    REMOTE_STDIO_OPEN_GUARDS.lock().await.remove(&key);
                }
                return Err(error);
            }
        };
        REMOTE_STDIO_SESSIONS.write().await.insert(
            key.clone(),
            RemoteStdioSessionEntry {
                session: session.clone(),
                activity_epoch: activity_epoch.clone(),
            },
        );
        schedule_remote_stdio_session_release(key, activity_epoch);
        Ok(RemoteStdioSessionLease::new(session))
    }

    async fn ensure_remote_search_context(&self, root_path: &str) -> Result<RemoteSearchContext, String> {
        let repo_root = normalize_remote_workspace_path(root_path);
        let connection = self.resolve_remote_workspace_entry(&repo_root).await?;
        let cache_key = remote_search_context_key(&connection.connection_id, &repo_root);
        if let Some(context) = REMOTE_SEARCH_CONTEXTS.read().await.get(&cache_key).cloned() {
            let local_bundle = local_flashgrep_bundle_for_arch(&context.remote_arch).await?;
            if local_bundle.sha256 == context.local_binary_sha256 {
                return Ok(context);
            }

            tracing::info!(
                target: FLASHGREP_LOG_TARGET,
                "Bundled remote flashgrep binary changed; reopening remote search session: connection_id={}, path={}, old_sha256={}, new_sha256={}",
                context.connection.connection_id,
                context.repo_root,
                context.local_binary_sha256,
                local_bundle.sha256
            );
            REMOTE_SEARCH_CONTEXTS.write().await.remove(&cache_key);
            let session_key = remote_stdio_session_key(&context.connection.connection_id, &context.repo_root);
            if let Some(entry) = REMOTE_STDIO_SESSIONS.write().await.remove(&session_key) {
                entry.session.close().await;
                entry.session.client.shutdown().await;
            }
        }

        let cached_server_os_type = self.provider.cached_server_os_type(&connection.connection_id).await;
        let remote_os = if let Some(os_type) = cached_server_os_type {
            if os_type.eq_ignore_ascii_case("unknown") {
                self.detect_remote_os_type(&connection.connection_id)
                    .await
                    .unwrap_or(os_type)
            } else {
                os_type
            }
        } else {
            self.detect_remote_os_type(&connection.connection_id)
                .await
                .unwrap_or_else(|| "unknown".to_string())
        };
        let inferred_linux = remote_os.eq_ignore_ascii_case("unknown") && looks_like_linux_workspace_root(&repo_root);
        if !remote_os.eq_ignore_ascii_case("linux") && !inferred_linux {
            return Err(format!(
                "Remote workspace search currently supports Linux only, but server OS is {}",
                remote_os
            ));
        }

        let remote_arch = self.detect_remote_architecture(&connection.connection_id).await?;
        let local_bundle = local_flashgrep_bundle_for_arch(&remote_arch).await?;
        let binary_path = self
            .ensure_remote_flashgrep_binary(&connection.connection_id, &repo_root, &local_bundle)
            .await?;
        let storage_root = remote_workspace_search_storage_root(&repo_root);

        let context = RemoteSearchContext {
            connection,
            binary_path,
            repo_root,
            storage_root,
            remote_arch,
            local_binary_sha256: local_bundle.sha256,
        };
        REMOTE_SEARCH_CONTEXTS.write().await.insert(cache_key, context.clone());
        Ok(context)
    }

    async fn detect_remote_architecture(&self, connection_id: &str) -> Result<String, String> {
        let mut attempts = Vec::new();

        for probe in REMOTE_ARCHITECTURE_PROBES {
            match self.provider.execute_command(connection_id, probe).await {
                Ok(output) => {
                    if let Some(arch) = parse_remote_architecture_output(&output.stdout, &output.stderr) {
                        return Ok(arch);
                    }
                    attempts.push(format!(
                        "probe=`{probe}` exit_code={} stdout={:?} stderr={:?}",
                        output.exit_code,
                        output.stdout.trim(),
                        output.stderr.trim()
                    ));
                }
                Err(error) => {
                    attempts.push(format!("probe=`{probe}` error={error}"));
                }
            }
        }

        Err(format!(
            "Failed to detect remote architecture from SSH output. Attempts: {}",
            attempts.join("; ")
        ))
    }

    async fn detect_remote_os_type(&self, connection_id: &str) -> Option<String> {
        for probe in REMOTE_OS_PROBES {
            let Ok(output) = self.provider.execute_command(connection_id, probe).await else {
                continue;
            };
            if let Some(os_type) = parse_remote_os_output(&output.stdout, &output.stderr) {
                return Some(os_type);
            }
        }
        None
    }

    async fn ensure_remote_flashgrep_binary(
        &self,
        connection_id: &str,
        repo_root: &str,
        local_bundle: &LocalFlashgrepBundle,
    ) -> Result<String, String> {
        let install_dir = remote_flashgrep_install_dir(repo_root);
        let remote_binary_path = join_remote_path(&install_dir, &local_bundle.binary_name);

        self.provider
            .create_dir_all(connection_id, &install_dir)
            .await
            .map_err(|error| format!("Failed to create remote flashgrep install directory: {error}"))?;
        let remote_sha256 = self.remote_flashgrep_sha256(connection_id, &remote_binary_path).await?;
        if remote_sha256.as_deref() != Some(local_bundle.sha256.as_str()) {
            tracing::info!(
                target: FLASHGREP_LOG_TARGET,
                "Uploading bundled remote flashgrep binary: connection_id={}, path={}, bundle={}, local_path={}, local_sha256={}, remote_sha256={}",
                connection_id,
                remote_binary_path,
                local_bundle.binary_name,
                local_bundle.path.display(),
                local_bundle.sha256,
                remote_sha256.as_deref().unwrap_or("missing")
            );
            let temp_remote_binary_path = format!("{}.upload-{}.tmp", remote_binary_path, local_bundle.sha256);
            self.provider
                .write_file(connection_id, &temp_remote_binary_path, &local_bundle.bytes)
                .await
                .map_err(|error| format!("Failed to upload flashgrep to remote host: {error}"))?;
            self.provider
                .execute_command(
                    connection_id,
                    &format!(
                        "mv -f {} {}",
                        shell_escape(&temp_remote_binary_path),
                        shell_escape(&remote_binary_path)
                    ),
                )
                .await
                .map_err(|error| format!("Failed to install uploaded flashgrep on remote host: {error}"))?;
        }
        self.provider
            .execute_command(
                connection_id,
                &format!("chmod 755 {}", shell_escape(&remote_binary_path)),
            )
            .await
            .map_err(|error| format!("Failed to mark remote flashgrep as executable: {error}"))?;

        Ok(remote_binary_path)
    }

    async fn remote_flashgrep_sha256(
        &self,
        connection_id: &str,
        remote_binary_path: &str,
    ) -> Result<Option<String>, String> {
        let escaped_path = shell_escape(remote_binary_path);
        let command = format!(
            "if [ -f {path} ]; then if command -v sha256sum >/dev/null 2>&1; then sha256sum {path} | awk '{{print $1}}'; elif command -v shasum >/dev/null 2>&1; then shasum -a 256 {path} | awk '{{print $1}}'; fi; fi",
            path = escaped_path
        );
        let output = self
            .provider
            .execute_command(connection_id, &command)
            .await
            .map_err(|error| format!("Failed to hash remote flashgrep binary: {error}"))?;
        if output.exit_code != 0 {
            return Ok(None);
        }
        let hash = output.stdout.trim();
        if hash.len() == 64 && hash.chars().all(|character| character.is_ascii_hexdigit()) {
            Ok(Some(hash.to_ascii_lowercase()))
        } else {
            Ok(None)
        }
    }
}
