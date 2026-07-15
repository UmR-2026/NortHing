//! Response formatting for `ReviewPlatformTool`.
//!
//! Owns `render_action_result` and `render_result_for_assistant` so the
//! read-side output can stay user-friendly without polluting action handlers.

use crate::service::review_platform::ReviewPlatformRemote;
use serde_json::Value;

impl super::ReviewPlatformTool {
    pub(super) fn render_action_result(output: &Value) -> Option<String> {
        let result = output.get("result")?;
        let message = result
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Review platform action completed");
        let web_url = result.get("webUrl").and_then(Value::as_str);
        let pr = result.get("pullRequest");

        let mut lines = vec![message.to_string()];
        if let Some(pr) = pr {
            let title = pr.get("title").and_then(Value::as_str).unwrap_or("Pull request");
            let number = pr.get("number").and_then(Value::as_i64).unwrap_or_default();
            let url = pr.get("webUrl").and_then(Value::as_str).or(web_url);
            if let Some(url) = url {
                lines.push(format!("[#{} {}]({})", number, title, url));
            }
        } else if let Some(url) = web_url {
            lines.push(url.to_string());
        }
        Some(lines.join("\n"))
    }

    pub(super) fn render_result_for_assistant(&self, output: &Value) -> String {
        let action = output.get("action").and_then(Value::as_str).unwrap_or("");
        if output
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| status == "needs_auth")
        {
            let message = output
                .pointer("/authChallenge/message")
                .and_then(Value::as_str)
                .unwrap_or("Review platform authentication is required.");
            return format!(
                "{} Ask the user to configure the token in the pull request panel, then retry this action.",
                message
            );
        }
        if let Some(action_result) = Self::render_action_result(output) {
            return action_result;
        }

        match action {
            super::ACTION_LIST_REMOTES => {
                let remotes = output
                    .get("remotes")
                    .and_then(Value::as_array)
                    .map(|items| items.as_slice())
                    .unwrap_or(&[]);
                let mut lines = vec![format!("Found {} review platform remotes.", remotes.len())];
                lines.extend(remotes.iter().map(|remote| {
                    let id = remote.get("id").and_then(Value::as_str).unwrap_or("");
                    let name = remote.get("name").and_then(Value::as_str).unwrap_or("");
                    let platform = remote.get("platform").and_then(Value::as_str).unwrap_or("");
                    let project = remote.get("projectPath").and_then(Value::as_str).unwrap_or("");
                    let url = remote.get("webUrl").and_then(Value::as_str).unwrap_or("");
                    format!(
                        "- remote_id: {} | name: {} | platform: {} | project: {} | url: {}",
                        id, name, platform, project, url
                    )
                }));
                lines.join("\n")
            }
            super::ACTION_WORKSPACE_SNAPSHOT => {
                let snapshot = output.get("snapshot");
                let Some(snapshot) = snapshot else {
                    return "Review platform workspace snapshot loaded.".to_string();
                };
                let remotes = snapshot
                    .get("remotes")
                    .and_then(Value::as_array)
                    .map(|items| items.len())
                    .unwrap_or(0);
                let prs = snapshot
                    .get("pullRequests")
                    .and_then(Value::as_array)
                    .map(|items| items.len())
                    .unwrap_or(0);
                let selected = snapshot
                    .get("selectedRemoteId")
                    .and_then(Value::as_str)
                    .unwrap_or("none");
                let message = snapshot.get("message").and_then(Value::as_str);
                match message {
                    Some(message) if !message.is_empty() => format!(
                        "Loaded review platform snapshot: selected remote {}, {} remotes, {} pull requests. {}",
                        selected, remotes, prs, message
                    ),
                    _ => format!(
                        "Loaded review platform snapshot: selected remote {}, {} remotes, {} pull requests.",
                        selected, remotes, prs
                    ),
                }
            }
            super::ACTION_COUNT => {
                if output
                    .get("status")
                    .and_then(Value::as_str)
                    .is_some_and(|status| status == "needs_remote_selection")
                {
                    let remotes = output
                        .get("candidateRemotes")
                        .and_then(Value::as_array)
                        .map(|items| items.as_slice())
                        .unwrap_or(&[]);
                    let mut lines = vec![
                        "Multiple review platform remotes were found. Ask the user which remote to use, then retry with remote_id.".to_string(),
                        "Candidate remotes:".to_string(),
                    ];
                    lines.extend(remotes.iter().map(|remote| {
                        let id = remote.get("id").and_then(Value::as_str).unwrap_or("");
                        let name = remote.get("name").and_then(Value::as_str).unwrap_or("");
                        let platform = remote.get("platform").and_then(Value::as_str).unwrap_or("");
                        let project = remote.get("projectPath").and_then(Value::as_str).unwrap_or("");
                        let url = remote.get("webUrl").and_then(Value::as_str).unwrap_or("");
                        format!(
                            "- remote_id: {} | name: {} | platform: {} | project: {} | url: {}",
                            id, name, platform, project, url
                        )
                    }));
                    return lines.join("\n");
                }

                let remote_id = output.get("remoteId").and_then(Value::as_str).unwrap_or("");
                let total = output.get("total").and_then(Value::as_u64);
                match total {
                    Some(total) => format!("Remote {} has {} pull requests.", remote_id, total),
                    None => format!("Remote {} did not return an exact pull request count.", remote_id),
                }
            }
            super::ACTION_LIST => {
                if output
                    .get("status")
                    .and_then(Value::as_str)
                    .is_some_and(|status| status == "needs_remote_selection")
                {
                    let remotes = output
                        .get("candidateRemotes")
                        .and_then(Value::as_array)
                        .map(|items| items.as_slice())
                        .unwrap_or(&[]);
                    let mut lines = vec![
                        "Multiple review platform remotes were found. Ask the user which remote to use, then retry with remote_id.".to_string(),
                        "Candidate remotes:".to_string(),
                    ];
                    lines.extend(remotes.iter().map(|remote| {
                        let id = remote.get("id").and_then(Value::as_str).unwrap_or("");
                        let name = remote.get("name").and_then(Value::as_str).unwrap_or("");
                        let platform = remote.get("platform").and_then(Value::as_str).unwrap_or("");
                        let project = remote.get("projectPath").and_then(Value::as_str).unwrap_or("");
                        let url = remote.get("webUrl").and_then(Value::as_str).unwrap_or("");
                        format!(
                            "- remote_id: {} | name: {} | platform: {} | project: {} | url: {}",
                            id, name, platform, project, url
                        )
                    }));
                    return lines.join("\n");
                }

                let prs = output
                    .pointer("/snapshot/pullRequests")
                    .and_then(Value::as_array)
                    .map(|items| items.as_slice())
                    .unwrap_or(&[]);
                let pagination = output.get("snapshot").and_then(|snapshot| snapshot.get("pagination"));
                let page = pagination
                    .and_then(|value| value.get("page"))
                    .and_then(Value::as_u64)
                    .unwrap_or(1);
                let per_page = pagination
                    .and_then(|value| value.get("perPage"))
                    .and_then(Value::as_u64)
                    .unwrap_or(prs.len() as u64);
                let total = pagination.and_then(|value| value.get("total")).and_then(Value::as_u64);
                let has_next = pagination
                    .and_then(|value| value.get("hasNext"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let remote_id = output.get("remoteId").and_then(Value::as_str).unwrap_or("");

                let mut lines = vec![match total {
                    Some(total) => format!(
                        "Remote {} has {} pull requests. Showing {} from page {} (page size {}).",
                        remote_id,
                        total,
                        prs.len(),
                        page,
                        per_page
                    ),
                    None => format!(
                        "Remote {} returned {} pull requests on page {} (page size {}).{}",
                        remote_id,
                        prs.len(),
                        page,
                        per_page,
                        if has_next {
                            " More pages are available; this is not the total count."
                        } else {
                            ""
                        }
                    ),
                }];
                if prs.is_empty() {
                    return lines.join("\n");
                }
                lines.extend(prs.iter().take(10).map(|pr| {
                    let number = pr.get("number").and_then(Value::as_i64).unwrap_or_default();
                    let title = pr.get("title").and_then(Value::as_str).unwrap_or("Untitled");
                    let state = pr.get("state").and_then(Value::as_str).unwrap_or("unknown");
                    let url = pr.get("webUrl").and_then(Value::as_str).unwrap_or("");
                    if url.is_empty() {
                        format!("#{} {} ({})", number, title, state)
                    } else {
                        format!("[#{} {}]({}) ({})", number, title, url, state)
                    }
                }));
                lines.join("\n")
            }
            super::ACTION_GET => {
                let pr = output.get("pullRequest");
                let Some(pr) = pr else {
                    return "Pull request detail loaded.".to_string();
                };
                let number = pr.get("number").and_then(Value::as_i64).unwrap_or_default();
                let title = pr.get("title").and_then(Value::as_str).unwrap_or("Untitled");
                let url = pr.get("webUrl").and_then(Value::as_str).unwrap_or("");
                let files = output
                    .get("files")
                    .and_then(Value::as_array)
                    .map(|items| items.len())
                    .unwrap_or(0);
                let threads = output
                    .get("threads")
                    .and_then(Value::as_array)
                    .map(|items| items.len())
                    .unwrap_or(0);
                if url.is_empty() {
                    format!("Loaded PR #{} {} ({} files, {} threads)", number, title, files, threads)
                } else {
                    format!(
                        "Loaded [#{} {}]({}) ({} files, {} threads)",
                        number, title, url, files, threads
                    )
                }
            }
            super::ACTION_GET_DETAIL_PAGE => {
                let section = output.get("section").and_then(Value::as_str).unwrap_or("detail");
                let items = output
                    .get("items")
                    .and_then(Value::as_array)
                    .map(|items| items.len())
                    .unwrap_or(0);
                let pagination = output.get("pagination");
                let page = pagination
                    .and_then(|value| value.get("page"))
                    .and_then(Value::as_u64)
                    .unwrap_or(1);
                let has_next = pagination
                    .and_then(|value| value.get("hasNext"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                format!(
                    "Loaded pull request {} page {} with {} items.{}",
                    section,
                    page,
                    items,
                    if has_next { " More pages are available." } else { "" }
                )
            }
            super::ACTION_GET_CI_LOG => {
                let ci_item_id = output.get("ciItemId").and_then(Value::as_str).unwrap_or("");
                let truncated = output.get("truncated").and_then(Value::as_bool).unwrap_or(false);
                let log_chars = output.get("log").and_then(Value::as_str).map(str::len).unwrap_or(0);
                format!(
                    "Loaded CI log for {} ({} characters).{}",
                    ci_item_id,
                    log_chars,
                    if truncated { " The log was truncated." } else { "" }
                )
            }
            super::ACTION_UPDATE_AUTH_TOKEN => "Review platform auth token updated.".to_string(),
            super::ACTION_CLEAR_AUTH_TOKEN => "Review platform auth token cleared.".to_string(),
            _ => "Review platform action completed.".to_string(),
        }
    }
}
