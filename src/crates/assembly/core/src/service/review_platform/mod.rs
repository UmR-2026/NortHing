//! Platform-neutral pull request review data service.
//!
//! This module owns provider detection, token handling, and provider-specific
//! HTTP calls. UI and desktop adapters consume only the common DTOs below.
//!
//! Architecture (after R4 split):
//! - [`types`]: pure DTO types and platform-neutral helpers
//! - [`http`]: HTTP client + pagination + header parsing
//! - [`auth`]: provider factory, token storage, auth challenges
//! - [`service`]: `ReviewPlatformService` high-level operations
//! - [`providers`]: GitHub / GitLab / GitCode provider impls + CI + util

mod auth;
mod http;
mod providers;
mod service;
mod types;

// External API: only the public DTOs and ReviewPlatformService. Helper functions
// remain crate-private to keep the surface area identical to the pre-split
// `mod.rs`, where they were `fn` (not `pub fn`).
pub use types::{
    ReviewAuthSource, ReviewAuthState, ReviewChecks, ReviewDecision, ReviewFileStatus, ReviewItemState,
    ReviewPlatformAccount, ReviewPlatformActionResult, ReviewPlatformApprovalRequest, ReviewPlatformAuthChallenge,
    ReviewPlatformAuthChallengeState, ReviewPlatformCapabilities, ReviewPlatformCiItem, ReviewPlatformCiLog,
    ReviewPlatformCommit, ReviewPlatformCreatePullRequestRequest, ReviewPlatformDetailSection, ReviewPlatformError,
    ReviewPlatformFile, ReviewPlatformKind, ReviewPlatformPagination, ReviewPlatformPullRequest,
    ReviewPlatformPullRequestDetail, ReviewPlatformPullRequestDetailPage, ReviewPlatformRemote,
    ReviewPlatformReplyToThreadRequest, ReviewPlatformRepositoryRef, ReviewPlatformRequestChangesRequest,
    ReviewPlatformResolveThreadRequest, ReviewPlatformService, ReviewPlatformSubmitReviewRequest, ReviewPlatformThread,
    ReviewPlatformThreadKind, ReviewPlatformWorkspaceSnapshot, ReviewSubmitEvent,
};

#[cfg(test)]
mod tests {
    use super::providers::ci::{ci_log_value, summarize_ci_items};
    use super::providers::github::{github_review_decision, github_threads};
    use super::providers::gitlab::gitlab_threads;
    use super::providers::util::short_hash;
    use super::types::*;
    use serde_json::json;

    #[test]
    fn github_review_decision_uses_latest_review_per_author() {
        let reviews = json!([
            {
                "id": 1,
                "state": "CHANGES_REQUESTED",
                "user": { "login": "alice" }
            },
            {
                "id": 2,
                "state": "APPROVED",
                "user": { "login": "alice" }
            }
        ]);

        assert_eq!(github_review_decision(&reviews), ReviewDecision::Approved);
    }

    #[test]
    fn github_review_decision_keeps_active_change_request_from_any_reviewer() {
        let reviews = json!([
            {
                "id": 1,
                "state": "APPROVED",
                "user": { "login": "alice" }
            },
            {
                "id": 2,
                "state": "CHANGES_REQUESTED",
                "user": { "login": "bob" }
            }
        ]);

        assert_eq!(github_review_decision(&reviews), ReviewDecision::ChangesRequested);
    }

    #[test]
    fn github_threads_include_issue_comments_and_review_comments() {
        let reviews = json!([]);
        let review_comments = json!([
            {
                "id": 10,
                "path": "src/lib.rs",
                "line": 8,
                "user": { "login": "alice" },
                "body": "Inline comment",
                "updated_at": "2026-05-18T01:00:00Z"
            }
        ]);
        let issue_comments = json!([
            {
                "id": 20,
                "user": { "login": "bob" },
                "body": "Conversation comment",
                "updated_at": "2026-05-18T02:00:00Z"
            }
        ]);

        let threads = github_threads(&reviews, &review_comments, &issue_comments);

        assert_eq!(threads.len(), 2);
        assert_eq!(threads[0].id, "comment-10");
        assert_eq!(threads[0].file_path.as_deref(), Some("src/lib.rs"));
        assert_eq!(threads[1].id, "issue-comment-20");
        assert_eq!(threads[1].file_path, None);
        assert_eq!(threads[1].body, "Conversation comment");
    }

    #[test]
    fn github_threads_keep_empty_body_reviews_visible() {
        let reviews = json!([
            {
                "id": 30,
                "state": "APPROVED",
                "user": { "login": "alice" },
                "body": "",
                "submitted_at": "2026-05-18T03:00:00Z"
            }
        ]);

        let threads = github_threads(&reviews, &json!([]), &json!([]));

        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].id, "review-30");
        assert_eq!(threads[0].body, "Approved this pull request.");
    }

    #[test]
    fn github_review_comment_replies_track_parent_comment() {
        let threads = github_threads(
            &json!([]),
            &json!([
                {
                    "id": 40,
                    "in_reply_to_id": 10,
                    "user": { "login": "alice" },
                    "body": "Reply",
                    "updated_at": "2026-05-18T04:30:00Z"
                }
            ]),
            &json!([]),
        );

        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].kind, ReviewPlatformThreadKind::Comment);
        assert_eq!(threads[0].reply_to_provider_comment_id.as_deref(), Some("10"));
    }

    #[test]
    fn gitlab_threads_include_top_level_notes_without_duplication() {
        let discussions = json!([
            {
                "id": "discussion-1",
                "resolved": false,
                "notes": [
                    {
                        "id": "100",
                        "author": { "username": "alice" },
                        "body": "Inline note",
                        "updated_at": "2026-05-18T04:00:00Z",
                        "position": { "new_path": "src/lib.rs", "new_line": 12 }
                    }
                ]
            }
        ]);
        let notes = json!([
            {
                "id": "100",
                "author": { "username": "alice" },
                "body": "Inline note",
                "updated_at": "2026-05-18T04:00:00Z",
                "position": { "new_path": "src/lib.rs", "new_line": 12 }
            },
            {
                "id": "200",
                "author": { "username": "bob" },
                "body": "Top-level note",
                "updated_at": "2026-05-18T05:00:00Z"
            }
        ]);

        let threads = gitlab_threads(&discussions, &notes);

        assert_eq!(threads.len(), 2);
        assert_eq!(threads[0].id, "discussion-discussion-1:note-100");
        assert_eq!(threads[1].id, "note-200");
        assert_eq!(threads[1].file_path, None);
        assert_eq!(threads[1].body, "Top-level note");
    }

    #[test]
    fn gitlab_discussion_threads_mark_root_as_review_and_replies_as_comments() {
        let discussions = json!([
            {
                "id": "discussion-2",
                "resolved": false,
                "notes": [
                    {
                        "id": "300",
                        "author": { "username": "alice" },
                        "body": "Root note",
                        "updated_at": "2026-05-18T06:00:00Z"
                    },
                    {
                        "id": "301",
                        "author": { "username": "bob" },
                        "body": "Reply note",
                        "updated_at": "2026-05-18T06:05:00Z"
                    }
                ]
            }
        ]);

        let threads = gitlab_threads(&discussions, &json!([]));

        assert_eq!(threads.len(), 2);
        assert_eq!(threads[0].kind, ReviewPlatformThreadKind::Review);
        assert_eq!(threads[0].reply_to_provider_comment_id, None);
        assert_eq!(threads[1].kind, ReviewPlatformThreadKind::Comment);
        assert_eq!(threads[1].reply_to_provider_comment_id.as_deref(), Some("300"));
    }

    #[test]
    fn summarize_ci_items_counts_provider_outcomes() {
        let items = vec![
            ReviewPlatformCiItem {
                id: "build".to_string(),
                name: "Build".to_string(),
                status: "completed".to_string(),
                conclusion: Some("success".to_string()),
                detail: None,
                stage: Some("build".to_string()),
                web_url: None,
                log: None,
                log_truncated: false,
                started_at: None,
                finished_at: None,
            },
            ReviewPlatformCiItem {
                id: "test".to_string(),
                name: "Test".to_string(),
                status: "failed".to_string(),
                conclusion: None,
                detail: None,
                stage: Some("test".to_string()),
                web_url: None,
                log: None,
                log_truncated: false,
                started_at: None,
                finished_at: None,
            },
            ReviewPlatformCiItem {
                id: "deploy".to_string(),
                name: "Deploy".to_string(),
                status: "running".to_string(),
                conclusion: None,
                detail: None,
                stage: Some("deploy".to_string()),
                web_url: None,
                log: None,
                log_truncated: false,
                started_at: None,
                finished_at: None,
            },
        ];

        let checks = summarize_ci_items(&items);

        assert_eq!(checks.total, 3);
        assert_eq!(checks.passed, 1);
        assert_eq!(checks.failed, 1);
        assert_eq!(checks.pending, 1);
    }

    #[test]
    fn ci_log_value_extracts_error_excerpt_only() {
        let text = [
            "running setup",
            "downloading dependencies",
            "cargo test failed with exit code 101",
            "thread 'main' panicked at src/lib.rs:4",
            "uploading artifacts",
        ]
        .join("\n");

        let (log, truncated) = ci_log_value(text);

        let log = log.expect("log should be present");
        assert!(!truncated);
        assert!(log.contains("cargo test failed"));
        assert!(log.contains("panicked at src/lib.rs"));
    }

    #[test]
    fn ci_log_value_reports_when_no_error_lines_match() {
        let (log, truncated) = ci_log_value("all checks passed".to_string());

        assert!(!truncated);
        assert!(log.is_none());
    }

    #[test]
    fn short_hash_truncates_to_seven_chars() {
        assert_eq!(short_hash("abcdef1234567890"), "abcdef1");
    }
}
