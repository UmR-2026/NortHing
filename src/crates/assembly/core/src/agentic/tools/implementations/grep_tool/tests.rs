//! `Grep` tool — unit tests for the split siblings.
//!
//! These tests cover the static input-parsing helpers in `filter.rs`, the
//! standalone workspace search renderers in `workspace.rs`, and the
//! content-mode rendering path through `format_workspace_search_output`.

use serde_json::json;
use tool_runtime::search::grep_search::relativize_result_text;

use super::filter::DEFAULT_HEAD_LIMIT;
use super::tool::GrepTool;
use super::workspace::{render_workspace_search_content_lines, render_workspace_search_result_lines};
use crate::infrastructure::{FileSearchOutcome, FileSearchResult, SearchMatchType};
use crate::service::search::{
    ContentSearchResult, WorkspaceSearchBackend, WorkspaceSearchHit, WorkspaceSearchLine, WorkspaceSearchMatch,
    WorkspaceSearchMatchLocation, WorkspaceSearchRepoPhase, WorkspaceSearchRepoStatus,
};

#[test]
fn head_limit_defaults_and_zero_escape_hatch() {
    assert_eq!(GrepTool::resolve_head_limit(&json!({})), Some(DEFAULT_HEAD_LIMIT));
    assert_eq!(GrepTool::resolve_head_limit(&json!({ "head_limit": 25 })), Some(25));
    assert_eq!(GrepTool::resolve_head_limit(&json!({ "head_limit": 0 })), None);
}

#[test]
fn backend_max_results_only_uses_explicit_limit() {
    assert_eq!(
        GrepTool::backend_max_results(&json!({}), 0, Some(DEFAULT_HEAD_LIMIT)),
        None
    );
    assert_eq!(
        GrepTool::backend_max_results(&json!({ "head_limit": 25 }), 3, Some(25)),
        Some(28)
    );
    assert_eq!(
        GrepTool::backend_max_results(&json!({ "head_limit": 0 }), 7, None),
        None
    );
}

#[test]
fn relativizes_prefixed_result_lines() {
    let text = "/repo/src/main.rs:12:fn main()\n/repo/src/lib.rs:3:pub fn lib()";
    let relativized = relativize_result_text(text, Some("/repo"));

    assert_eq!(relativized, "src/main.rs:12:fn main()\nsrc/lib.rs:3:pub fn lib()");
}

#[test]
fn renders_workspace_search_context_lines_in_rg_style() {
    let lines = render_workspace_search_content_lines(
        &[WorkspaceSearchHit {
            path: "/repo/src/main.rs".to_string(),
            matches: vec![WorkspaceSearchMatch {
                location: WorkspaceSearchMatchLocation { line: 12, column: 5 },
                snippet: "panic!(\"x\")".to_string(),
                matched_text: "panic".to_string(),
            }],
            lines: vec![
                WorkspaceSearchLine::Context {
                    value: crate::service::search::WorkspaceSearchContextLine {
                        line_number: 10,
                        snippet: "let a = 1".to_string(),
                    },
                },
                WorkspaceSearchLine::Context {
                    value: crate::service::search::WorkspaceSearchContextLine {
                        line_number: 11,
                        snippet: "let b = 2".to_string(),
                    },
                },
                WorkspaceSearchLine::Match {
                    value: WorkspaceSearchMatch {
                        location: WorkspaceSearchMatchLocation { line: 12, column: 5 },
                        snippet: "panic!(\"x\")".to_string(),
                        matched_text: "panic".to_string(),
                    },
                },
                WorkspaceSearchLine::Context {
                    value: crate::service::search::WorkspaceSearchContextLine {
                        line_number: 13,
                        snippet: "cleanup()".to_string(),
                    },
                },
                WorkspaceSearchLine::ContextBreak,
                WorkspaceSearchLine::Context {
                    value: crate::service::search::WorkspaceSearchContextLine {
                        line_number: 20,
                        snippet: "return".to_string(),
                    },
                },
            ],
        }],
        true,
    );

    assert_eq!(
        lines,
        vec![
            "/repo/src/main.rs-10:let a = 1",
            "/repo/src/main.rs-11:let b = 2",
            "/repo/src/main.rs:12:panic!(\"x\")",
            "/repo/src/main.rs-13:cleanup()",
            "--",
            "/repo/src/main.rs-20:return",
        ]
    );
}

#[test]
fn content_workspace_output_uses_hits_for_context_lines() {
    let tool = GrepTool::new();
    let result = ContentSearchResult {
        outcome: FileSearchOutcome {
            results: Vec::new(),
            truncated: false,
        },
        file_counts: Vec::new(),
        hits: vec![WorkspaceSearchHit {
            path: "/repo/src/main.rs".to_string(),
            matches: vec![WorkspaceSearchMatch {
                location: WorkspaceSearchMatchLocation { line: 12, column: 5 },
                snippet: "panic!(\"x\")".to_string(),
                matched_text: "panic".to_string(),
            }],
            lines: vec![
                WorkspaceSearchLine::Context {
                    value: crate::service::search::WorkspaceSearchContextLine {
                        line_number: 11,
                        snippet: "let b = 2".to_string(),
                    },
                },
                WorkspaceSearchLine::Match {
                    value: WorkspaceSearchMatch {
                        location: WorkspaceSearchMatchLocation { line: 12, column: 5 },
                        snippet: "panic!(\"x\")".to_string(),
                        matched_text: "panic".to_string(),
                    },
                },
            ],
        }],
        backend: WorkspaceSearchBackend::Indexed,
        repo_status: WorkspaceSearchRepoStatus {
            repo_id: "repo".to_string(),
            repo_path: "/repo".to_string(),
            storage_root: "/repo/.northhing/search/flashgrep-index".to_string(),
            base_snapshot_root: "/repo/.northhing/search/flashgrep-index/base-snapshot".to_string(),
            workspace_overlay_root: "/repo/.northhing/search/flashgrep-index/workspace-overlay".to_string(),
            phase: WorkspaceSearchRepoPhase::Ready,
            snapshot_key: None,
            last_probe_unix_secs: None,
            last_rebuild_unix_secs: None,
            dirty_files: crate::service::search::WorkspaceSearchDirtyFiles {
                modified: 0,
                deleted: 0,
                new: 0,
            },
            rebuild_recommended: false,
            active_task_id: None,
            probe_healthy: true,
            last_error: None,
            overlay: None,
        },
        candidate_docs: 1,
        matched_lines: 1,
        matched_occurrences: 1,
    };

    let (rendered, file_count, total_matches) =
        tool.format_workspace_search_output("content", true, 0, None, &result, Some("/repo"));

    assert_eq!(rendered, "src/main.rs-11:let b = 2\nsrc/main.rs:12:panic!(\"x\")");
    assert_eq!(file_count, 1);
    assert_eq!(total_matches, 1);
}

#[test]
fn content_workspace_output_falls_back_to_converted_line_results() {
    let tool = GrepTool::new();
    let result = ContentSearchResult {
        outcome: FileSearchOutcome {
            results: vec![
                FileSearchResult {
                    path: "/repo/src/main.rs".to_string(),
                    name: "main.rs".to_string(),
                    is_directory: false,
                    match_type: SearchMatchType::Content,
                    line_number: Some(12),
                    matched_content: Some("panic!(\"x\")".to_string()),
                    preview_before: None,
                    preview_inside: Some("panic!(\"x\")".to_string()),
                    preview_after: None,
                },
                FileSearchResult {
                    path: "/repo/src/lib.rs".to_string(),
                    name: "lib.rs".to_string(),
                    is_directory: false,
                    match_type: SearchMatchType::Content,
                    line_number: Some(3),
                    matched_content: Some("pub fn lib() {}".to_string()),
                    preview_before: None,
                    preview_inside: Some("pub fn lib() {}".to_string()),
                    preview_after: None,
                },
            ],
            truncated: false,
        },
        file_counts: Vec::new(),
        hits: Vec::new(),
        backend: WorkspaceSearchBackend::Indexed,
        repo_status: WorkspaceSearchRepoStatus {
            repo_id: "repo".to_string(),
            repo_path: "/repo".to_string(),
            storage_root: "/repo/.northhing/search/flashgrep-index".to_string(),
            base_snapshot_root: "/repo/.northhing/search/flashgrep-index/base-snapshot".to_string(),
            workspace_overlay_root: "/repo/.northhing/search/flashgrep-index/workspace-overlay".to_string(),
            phase: WorkspaceSearchRepoPhase::Ready,
            snapshot_key: None,
            last_probe_unix_secs: None,
            last_rebuild_unix_secs: None,
            dirty_files: crate::service::search::WorkspaceSearchDirtyFiles {
                modified: 0,
                deleted: 0,
                new: 0,
            },
            rebuild_recommended: false,
            active_task_id: None,
            probe_healthy: true,
            last_error: None,
            overlay: None,
        },
        candidate_docs: 2,
        matched_lines: 2,
        matched_occurrences: 2,
    };

    let (rendered, file_count, total_matches) =
        tool.format_workspace_search_output("content", true, 0, None, &result, Some("/repo"));

    assert_eq!(rendered, "src/main.rs:12:panic!(\"x\")\nsrc/lib.rs:3:pub fn lib() {}");
    assert_eq!(file_count, 2);
    assert_eq!(total_matches, 2);
}

#[test]
fn renders_workspace_search_result_lines_without_line_numbers() {
    let lines = render_workspace_search_result_lines(
        &[FileSearchResult {
            path: "/repo/src/main.rs".to_string(),
            name: "main.rs".to_string(),
            is_directory: false,
            match_type: SearchMatchType::Content,
            line_number: Some(12),
            matched_content: Some("panic!(\"x\")".to_string()),
            preview_before: None,
            preview_inside: Some("panic!(\"x\")".to_string()),
            preview_after: None,
        }],
        false,
    );

    assert_eq!(lines, vec!["/repo/src/main.rs:panic!(\"x\")"]);
}
