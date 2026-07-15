//! Cross-provider shared helpers — remote URL parsing, pull request file diff
//! counters, and provider-comment-id format parsing.

use crate::service::review_platform::auth::auth_for_platform_host;
use crate::service::review_platform::types::{
    ReviewAuthState, ReviewChecks, ReviewFileStatus, ReviewPlatformAuthTokens, ReviewPlatformFile, ReviewPlatformKind,
    ReviewPlatformPullRequest, ReviewPlatformRemote,
};
use serde_json::Value;

#[derive(Debug)]
pub(crate) struct ParsedRemoteUrl {
    pub(crate) scheme: String,
    pub(crate) host: String,
    pub(crate) path: String,
}

pub(crate) fn parse_remote(
    remote_name: &str,
    remote_url: &str,
    auth_tokens: &ReviewPlatformAuthTokens,
) -> Option<ReviewPlatformRemote> {
    let parsed = parse_remote_url(remote_url)?;
    let host_lower = parsed.host.to_ascii_lowercase();
    let platform = if host_lower.contains("github.com") {
        ReviewPlatformKind::Github
    } else if host_lower.contains("gitlab") {
        ReviewPlatformKind::Gitlab
    } else if host_lower.contains("gitcode") {
        ReviewPlatformKind::Gitcode
    } else {
        ReviewPlatformKind::Unknown
    };

    let segments: Vec<&str> = parsed
        .path
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.len() < 2 {
        return None;
    }
    let owner = segments.first()?.to_string();
    let repository_name = segments.last()?.trim_end_matches(".git").to_string();
    let project_path = segments
        .iter()
        .map(|segment| segment.trim_end_matches(".git"))
        .collect::<Vec<_>>()
        .join("/");

    let supported = platform != ReviewPlatformKind::Unknown;
    let (auth_state, auth_source) = auth_for_platform_host(platform, &parsed.host, auth_tokens);
    let web_url = format!("{}://{}/{}", parsed.scheme, parsed.host, project_path);

    Some(ReviewPlatformRemote {
        id: format!(
            "{}:{}:{}",
            remote_name,
            platform.as_str(),
            project_path.replace('/', "__")
        ),
        name: remote_name.to_string(),
        url: sanitize_remote_url(remote_url),
        platform,
        host: parsed.host,
        owner,
        repository_name,
        project_path,
        web_url,
        supported,
        auth_state,
        auth_source,
        message: if !supported {
            Some("This remote is detected, but no provider adapter is available yet.".to_string())
        } else if platform == ReviewPlatformKind::Gitcode && auth_state == ReviewAuthState::NotConnected {
            Some("Add a GitCode token to load pull requests.".to_string())
        } else {
            None
        },
    })
}

pub(crate) fn parse_remote_url(remote_url: &str) -> Option<ParsedRemoteUrl> {
    if let Some(scheme_end) = remote_url.find("://") {
        let scheme = &remote_url[..scheme_end];
        let rest = &remote_url[scheme_end + 3..];
        let slash = rest.find('/')?;
        let authority = &rest[..slash];
        let host_part = authority.rsplit('@').next().unwrap_or(authority);
        let host = host_part.split(':').next().unwrap_or(host_part);
        let path = rest[slash + 1..].trim_end_matches(".git").to_string();
        return Some(ParsedRemoteUrl {
            scheme: if scheme == "ssh" { "https" } else { scheme }.to_string(),
            host: host.to_string(),
            path,
        });
    }

    if let Some((user_host, path)) = remote_url.split_once(':') {
        if user_host.contains('@') && !path.contains('\\') {
            let host = user_host.rsplit('@').next()?.to_string();
            return Some(ParsedRemoteUrl {
                scheme: "https".to_string(),
                host,
                path: path.trim_end_matches(".git").to_string(),
            });
        }
    }

    None
}

fn sanitize_remote_url(remote_url: &str) -> String {
    if let Some(scheme_end) = remote_url.find("://") {
        let scheme = &remote_url[..scheme_end];
        let rest = &remote_url[scheme_end + 3..];
        if let Some(slash) = rest.find('/') {
            let authority = &rest[..slash];
            if authority.contains('@') {
                let host = authority.rsplit('@').next().unwrap_or(authority);
                return format!("{}://{}/{}", scheme, host, &rest[slash + 1..]);
            }
        }
    }
    remote_url.to_string()
}

pub(crate) fn parse_provider_comment_id(thread_id: &str) -> Option<&str> {
    let trimmed = thread_id.trim();
    trimmed
        .strip_prefix("comment-")
        .or_else(|| trimmed.strip_prefix("note-"))
        .or_else(|| trimmed.split_once(":note-").map(|(_, note_id)| note_id))
        .or_else(|| {
            if trimmed.chars().all(|ch| ch.is_ascii_digit()) {
                Some(trimmed)
            } else {
                None
            }
        })
        .filter(|value| !value.trim().is_empty())
}

pub(crate) fn parse_provider_thread_id(thread_id: &str) -> Option<&str> {
    let trimmed = thread_id.trim();
    trimmed
        .strip_prefix("discussion-")
        .map(|value| value.split_once(":note-").map(|(id, _)| id).unwrap_or(value))
        .or_else(|| {
            if trimmed.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
                Some(trimmed)
            } else {
                None
            }
        })
        .filter(|value| !value.trim().is_empty())
}

pub(crate) fn empty_checks() -> ReviewChecks {
    ReviewChecks {
        total: 0,
        passed: 0,
        failed: 0,
        pending: 0,
    }
}

pub(crate) fn file_status(status: &str) -> ReviewFileStatus {
    match status {
        "added" | "new" => ReviewFileStatus::Added,
        "removed" | "deleted" => ReviewFileStatus::Deleted,
        "renamed" => ReviewFileStatus::Renamed,
        _ => ReviewFileStatus::Modified,
    }
}

pub(crate) fn count_diff_lines(diff: &str) -> (i32, i32) {
    let mut additions = 0;
    let mut deletions = 0;
    for line in diff.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if line.starts_with('+') {
            additions += 1;
        } else if line.starts_with('-') {
            deletions += 1;
        }
    }
    (additions, deletions)
}

pub(crate) fn apply_files_stats(pull_request: &mut ReviewPlatformPullRequest, files: &[ReviewPlatformFile]) {
    pull_request.changed_files = files.len() as i32;
    let (additions, deletions) = files
        .iter()
        .fold((0, 0), |acc, file| (acc.0 + file.additions, acc.1 + file.deletions));
    pull_request.additions = additions;
    pull_request.deletions = deletions;
}

pub(crate) fn array_items(value: &Value) -> &[Value] {
    value.as_array().map(|items| items.as_slice()).unwrap_or(&[])
}

pub(crate) fn value_string(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(flag)) => flag.to_string(),
        _ => String::new(),
    }
}

pub(crate) fn optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
}

pub(crate) fn nested_string(value: &Value, path: &[&str]) -> String {
    nested_optional_string(value, path).unwrap_or_default()
}

pub(crate) fn nested_optional_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    match current {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

pub(crate) fn value_i64(value: &Value, key: &str) -> i64 {
    value
        .get(key)
        .and_then(|value| value.as_i64().or_else(|| value.as_str()?.parse::<i64>().ok()))
        .unwrap_or(0)
}

pub(crate) fn value_bool(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(|value| {
            value
                .as_bool()
                .or_else(|| value.as_str().map(|text| text.eq_ignore_ascii_case("true")))
        })
        .unwrap_or(false)
}

pub(crate) fn first_non_empty(values: &[String]) -> String {
    values
        .iter()
        .find(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn first_non_zero(values: &[i64]) -> i64 {
    values.iter().copied().find(|value| *value != 0).unwrap_or(0)
}

pub(crate) fn first_line(value: &str) -> String {
    value.lines().next().unwrap_or_default().to_string()
}

pub(crate) fn short_hash(hash: &str) -> String {
    hash.chars().take(7).collect()
}
