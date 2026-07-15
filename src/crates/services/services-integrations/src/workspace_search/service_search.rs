//! Workspace search content/glob search methods and shared helpers.
//!
//! Implements [`WorkspaceSearchService::search_content`] and
//! [`WorkspaceSearchService::glob`], plus the scope/path/error-mapping
//! helpers used by the index and session siblings.

use super::flashgrep::{ConsistencyMode, FlashgrepRepoSession, GlobRequest, PathScope, QuerySpec, SearchRequest};
use super::result_mapping::convert_search_results;
use super::service::{WorkspaceSearchResult, WorkspaceSearchService, DEFAULT_TOP_K_TOKENS};
use super::types::{
    ContentSearchRequest, ContentSearchResult, GlobSearchRequest, GlobSearchResult, WorkspaceSearchFileCount,
    WorkspaceSearchHit,
};
use northhing_services_core::filesystem::FileSearchOutcome;
use std::path::{Path, PathBuf};
use std::time::Instant;

impl WorkspaceSearchService {
    pub async fn search_content(&self, request: ContentSearchRequest) -> WorkspaceSearchResult<ContentSearchResult> {
        let started_at = Instant::now();
        let pattern_for_log = abbreviate_pattern_for_log(&request.pattern);
        let repo_root = super::service_session::normalize_repo_root(&request.repo_root)?;
        let normalized_at = Instant::now();
        let scope = build_scope(
            &repo_root,
            request.search_path.as_deref(),
            request.globs,
            request.file_types,
            request.exclude_file_types,
        )?;
        let scope_built_at = Instant::now();
        let scope_roots_count = scope.roots.len();
        let scope_globs_count = scope.globs.len();
        let scope_types_count = scope.types.len();
        let max_results = request.max_results.filter(|limit| *limit > 0);
        let query = QuerySpec {
            pattern: request.pattern,
            patterns: Vec::new(),
            case_insensitive: !request.case_sensitive,
            multiline: request.multiline,
            dot_matches_new_line: request.multiline,
            fixed_strings: !request.use_regex,
            word_regexp: request.whole_word,
            line_regexp: false,
            before_context: request.before_context,
            after_context: request.after_context,
            top_k_tokens: DEFAULT_TOP_K_TOKENS,
            max_count: None,
            global_max_results: max_results,
            search_mode: request.output_mode.search_mode(),
        };

        let session = self.get_or_open_session(&repo_root).await?;
        let session_ready_at = Instant::now();
        let search = FlashgrepRepoSession::search(
            session.as_ref(),
            SearchRequest::new(query)
                .with_scope(scope)
                .with_consistency(ConsistencyMode::WorkspaceEventual)
                .with_scan_fallback(true),
        )
        .await
        .map_err(map_flashgrep_error("Content search failed"))?;
        let search_completed_at = Instant::now();

        let mut results = convert_search_results(&search.results, request.output_mode);
        let converted_at = Instant::now();
        let truncated = max_results.map(|limit| results.len() >= limit).unwrap_or(false);
        if let Some(limit) = max_results {
            results.truncate(limit);
        }

        let result = ContentSearchResult {
            outcome: FileSearchOutcome { results, truncated },
            file_counts: search
                .results
                .file_counts
                .clone()
                .into_iter()
                .map(WorkspaceSearchFileCount::from)
                .collect(),
            hits: search
                .results
                .hits
                .clone()
                .into_iter()
                .map(WorkspaceSearchHit::from)
                .collect(),
            backend: search.backend.into(),
            repo_status: search.status.into(),
            candidate_docs: search.results.candidate_docs,
            matched_lines: search.results.matched_lines,
            matched_occurrences: search.results.matched_occurrences,
        };

        tracing::debug!(
            target: super::flashgrep::FLASHGREP_LOG_TARGET,
            "Workspace content search completed: repo_root={}, pattern={}, output_mode={:?}, search_mode={:?}, scope_roots={}, globs={}, file_types={}, max_results={:?}, backend={:?}, repo_phase={:?}, rebuild_recommended={}, dirty_modified={}, dirty_deleted={}, dirty_new={}, candidate_docs={}, matched_lines={}, matched_occurrences={}, returned_results={}, truncated={}, normalize_ms={}, build_scope_ms={}, session_ms={}, search_ms={}, convert_ms={}, total_ms={}",
            repo_root.display(),
            pattern_for_log,
            request.output_mode,
            request.output_mode.search_mode(),
            scope_roots_count,
            scope_globs_count,
            scope_types_count,
            max_results,
            result.backend,
            result.repo_status.phase,
            result.repo_status.rebuild_recommended,
            result.repo_status.dirty_files.modified,
            result.repo_status.dirty_files.deleted,
            result.repo_status.dirty_files.new,
            result.candidate_docs,
            result.matched_lines,
            result.matched_occurrences,
            result.outcome.results.len(),
            result.outcome.truncated,
            normalized_at.duration_since(started_at).as_millis(),
            scope_built_at.duration_since(normalized_at).as_millis(),
            session_ready_at.duration_since(scope_built_at).as_millis(),
            search_completed_at.duration_since(session_ready_at).as_millis(),
            converted_at.duration_since(search_completed_at).as_millis(),
            converted_at.duration_since(started_at).as_millis(),
        );

        Ok(result)
    }

    pub async fn glob(&self, request: GlobSearchRequest) -> WorkspaceSearchResult<GlobSearchResult> {
        let repo_root = super::service_session::normalize_repo_root(&request.repo_root)?;
        let scope = build_scope(
            &repo_root,
            request.search_path.as_deref(),
            vec![request.pattern],
            vec![],
            vec![],
        )?;
        let session = self.get_or_open_session(&repo_root).await?;
        let mut outcome = FlashgrepRepoSession::glob(session.as_ref(), GlobRequest::new().with_scope(scope))
            .await
            .map_err(map_flashgrep_error("Glob search failed"))?;
        outcome.paths.sort();
        if request.limit > 0 {
            outcome.paths.truncate(request.limit);
        } else {
            outcome.paths.clear();
        }

        Ok(GlobSearchResult {
            paths: outcome.paths,
            repo_status: outcome.status.into(),
        })
    }
}

pub(super) fn build_scope(
    repo_root: &Path,
    search_path: Option<&Path>,
    globs: Vec<String>,
    file_types: Vec<String>,
    exclude_file_types: Vec<String>,
) -> WorkspaceSearchResult<PathScope> {
    let roots = match search_path {
        Some(path) => {
            let normalized = normalize_scope_path(repo_root, path)?;
            if normalized == repo_root {
                Vec::new()
            } else {
                vec![normalized]
            }
        }
        None => Vec::new(),
    };

    Ok(PathScope {
        roots,
        globs,
        iglobs: Vec::new(),
        type_add: Vec::new(),
        type_clear: Vec::new(),
        types: file_types,
        type_not: exclude_file_types,
    })
}

pub(super) fn normalize_scope_path(repo_root: &Path, search_path: &Path) -> WorkspaceSearchResult<PathBuf> {
    let normalized = dunce::canonicalize(search_path)
        .map_err(|error| format!("Failed to normalize search path {}: {}", search_path.display(), error))?;
    if !normalized.starts_with(repo_root) {
        return Err(format!(
            "Search path is outside workspace root: {}",
            normalized.display()
        ));
    }
    Ok(normalized)
}

pub(super) fn abbreviate_pattern_for_log(pattern: &str) -> String {
    const MAX_CHARS: usize = 120;
    let mut chars = pattern.chars();
    let abbreviated: String = chars.by_ref().take(MAX_CHARS).collect();
    if chars.next().is_some() {
        format!("{}...", abbreviated)
    } else {
        abbreviated
    }
}

pub(super) fn map_flashgrep_error(prefix: &'static str) -> impl Fn(super::flashgrep::error::AppError) -> String {
    move |error| {
        let detail = match &error {
            super::flashgrep::error::AppError::Io(io_error) if io_error.kind() == std::io::ErrorKind::NotFound => {
                format!("{error}. {}", super::service::workspace_search_daemon_missing_hint())
            }
            _ => error.to_string(),
        };
        format!("{prefix}: {detail}")
    }
}
