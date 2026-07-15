//! GitLab provider — DTO mapping helpers (JSON `Value` → `ReviewPlatform*` structs).
//!
//! Pure functions, no I/O. Extracted from `gitlab.rs` to keep the provider file
//! under the 800-line spec limit (`docs/handoffs/2026-06-27-r4-review-platform-extended-spec.md`).

use crate::service::review_platform::types::{
    ReviewDecision, ReviewItemState, ReviewPlatformCommit, ReviewPlatformFile, ReviewPlatformPullRequest,
    ReviewPlatformThread, ReviewPlatformThreadKind,
};
use serde_json::Value;

use super::super::types::ReviewFileStatus;
use super::util::{array_items, count_diff_lines, empty_checks, first_non_empty, short_hash, value_bool, value_string};

pub(crate) fn gitlab_pull_request_from_value(value: &Value) -> ReviewPlatformPullRequest {
    let number = super::util::value_i64(value, "iid");
    let state = if value_bool(value, "draft") || value_bool(value, "work_in_progress") {
        ReviewItemState::Draft
    } else {
        match value_string(value, "state").as_str() {
            "merged" => ReviewItemState::Merged,
            "closed" => ReviewItemState::Closed,
            _ => ReviewItemState::Open,
        }
    };
    let changed_files = value_string(value, "changes_count").parse::<i32>().unwrap_or(0);

    ReviewPlatformPullRequest {
        id: number.to_string(),
        number,
        title: value_string(value, "title"),
        state,
        author: first_non_empty(&[
            super::util::nested_string(value, &["author", "username"]),
            super::util::nested_string(value, &["author", "name"]),
        ]),
        source_branch: value_string(value, "source_branch"),
        target_branch: value_string(value, "target_branch"),
        updated_at: value_string(value, "updated_at"),
        web_url: value_string(value, "web_url"),
        additions: 0,
        deletions: 0,
        changed_files,
        comments: super::util::value_i64(value, "user_notes_count") as i32,
        review_decision: ReviewDecision::Pending,
        checks: empty_checks(),
    }
}

pub(crate) fn gitlab_files(value: &Value) -> Vec<ReviewPlatformFile> {
    value
        .get("changes")
        .and_then(Value::as_array)
        .unwrap_or(&Vec::new())
        .iter()
        .map(|change| {
            let diff = value_string(change, "diff");
            let (additions, deletions) = count_diff_lines(&diff);
            let status = if value_bool(change, "new_file") {
                ReviewFileStatus::Added
            } else if value_bool(change, "deleted_file") {
                ReviewFileStatus::Deleted
            } else if value_bool(change, "renamed_file") {
                ReviewFileStatus::Renamed
            } else {
                ReviewFileStatus::Modified
            };
            ReviewPlatformFile {
                path: value_string(change, "new_path"),
                old_path: change.get("old_path").and_then(Value::as_str).map(str::to_string),
                status,
                additions,
                deletions,
                patch: Some(diff),
            }
        })
        .collect()
}

pub(crate) fn gitlab_commit_from_value(value: &Value) -> ReviewPlatformCommit {
    let hash = value_string(value, "id");
    ReviewPlatformCommit {
        short_hash: first_non_empty(&[value_string(value, "short_id"), short_hash(&hash)]),
        hash,
        title: first_non_empty(&[
            value_string(value, "title"),
            super::util::first_line(&value_string(value, "message")),
        ]),
        author: value_string(value, "author_name"),
        committed_at: first_non_empty(&[value_string(value, "committed_date"), value_string(value, "created_at")]),
    }
}

pub(crate) fn gitlab_threads(discussions: &Value, notes: &Value) -> Vec<ReviewPlatformThread> {
    let mut threads = Vec::new();
    let mut seen_comment_ids = std::collections::HashSet::new();
    for discussion in array_items(discussions) {
        let discussion_id = value_string(discussion, "id");
        let resolved = value_bool(discussion, "resolved");
        let discussion_notes = discussion
            .get("notes")
            .and_then(Value::as_array)
            .map(|notes| notes.as_slice())
            .unwrap_or(&[]);
        let mut root_comment_id: Option<String> = None;
        for (index, note) in discussion_notes.iter().enumerate() {
            let kind = if index == 0 {
                ReviewPlatformThreadKind::Review
            } else {
                ReviewPlatformThreadKind::Comment
            };
            let reply_to = if index == 0 { None } else { root_comment_id.clone() };
            let thread = gitlab_thread_from_note(note, Some(discussion_id.clone()), resolved, kind, reply_to);
            if root_comment_id.is_none() {
                root_comment_id = thread.provider_comment_id.clone();
            }
            if let Some(comment_id) = thread.provider_comment_id.clone() {
                seen_comment_ids.insert(comment_id);
            }
            threads.push(thread);
        }
    }
    for note in array_items(notes) {
        let thread = gitlab_thread_from_note(note, None, false, ReviewPlatformThreadKind::Comment, None);
        if let Some(comment_id) = thread.provider_comment_id.as_ref() {
            if seen_comment_ids.contains(comment_id) {
                continue;
            }
            seen_comment_ids.insert(comment_id.clone());
        }
        threads.push(thread);
    }
    threads
}

pub(crate) fn gitlab_thread_from_note(
    note: &Value,
    discussion_id: Option<String>,
    discussion_resolved: bool,
    kind: ReviewPlatformThreadKind,
    reply_to_provider_comment_id: Option<String>,
) -> ReviewPlatformThread {
    let note_id = value_string(note, "id");
    let id = match discussion_id.as_deref() {
        Some(discussion_id) if !discussion_id.trim().is_empty() => {
            format!("discussion-{}:note-{}", discussion_id, note_id)
        }
        _ => format!("note-{}", note_id),
    };

    ReviewPlatformThread {
        id,
        provider_thread_id: discussion_id,
        provider_comment_id: Some(note_id),
        kind,
        reply_to_provider_comment_id,
        file_path: super::util::nested_optional_string(note, &["position", "new_path"])
            .or_else(|| super::util::nested_optional_string(note, &["position", "old_path"])),
        line: note
            .pointer("/position/new_line")
            .and_then(Value::as_i64)
            .or_else(|| note.pointer("/position/old_line").and_then(Value::as_i64)),
        resolved: discussion_resolved || value_bool(note, "resolved"),
        author: first_non_empty(&[
            super::util::nested_string(note, &["author", "username"]),
            super::util::nested_string(note, &["author", "name"]),
        ]),
        body: value_string(note, "body"),
        updated_at: first_non_empty(&[value_string(note, "updated_at"), value_string(note, "created_at")]),
    }
}
