//! GitHub provider implementation — REST API v3 calls and DTO mapping.

use super::super::auth::require_write_token;
use super::super::http::{
    combine_page_pagination, empty_detail_pagination, fetch_array_page, fetch_paginated_array, link_header_has_rel,
    pagination_from_response, pagination_from_total, pagination_total_from_links, send_json, send_json_response,
    slice_page,
};
use super::super::types::{
    ProviderContext, PullRequestPagination, ReviewDecision, ReviewItemState, ReviewPlatformActionResult,
    ReviewPlatformApprovalRequest, ReviewPlatformCiLog, ReviewPlatformCommit, ReviewPlatformCreatePullRequestRequest,
    ReviewPlatformDetailSection, ReviewPlatformError, ReviewPlatformFile, ReviewPlatformPagination,
    ReviewPlatformPullRequest, ReviewPlatformPullRequestDetail, ReviewPlatformPullRequestDetailPage,
    ReviewPlatformPullRequestPage, ReviewPlatformReplyToThreadRequest, ReviewPlatformRequestChangesRequest,
    ReviewPlatformSubmitReviewRequest, ReviewPlatformThread, ReviewPlatformThreadKind, ReviewSubmitEvent,
    PROVIDER_ENRICH_CONCURRENCY,
};
use super::ci::{github_actions_log_for_check_run_item, github_checks_and_ci};
use super::util::{
    array_items, empty_checks, file_status, first_line, first_non_empty, nested_string, optional_string,
    parse_provider_comment_id, short_hash, value_bool, value_i64, value_string,
};
use futures::{stream, StreamExt};
use reqwest::header::{HeaderMap, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde_json::{json, Value};
use std::collections::HashMap;

pub(crate) struct GithubProvider;

const USER_AGENT_VALUE: &str = "ReviewPlatform";

#[async_trait::async_trait]
impl super::ReviewProvider for GithubProvider {
    async fn list_pull_requests(
        &self,
        ctx: &ProviderContext,
        pagination: PullRequestPagination,
    ) -> Result<ReviewPlatformPullRequestPage, ReviewPlatformError> {
        let url = format!(
            "{}/repos/{}/{}/pulls",
            ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name
        );
        let per_page = pagination.per_page.to_string();
        let page = pagination.page.to_string();
        let response = send_json_response(
            github_request(super::super::http::http_client()?, &url, ctx.token.as_deref()).query(&[
                ("state", "all"),
                ("per_page", &per_page),
                ("page", &page),
            ]),
        )
        .await?;
        let items = response
            .value
            .as_array()
            .ok_or_else(|| ReviewPlatformError::Parse("GitHub pull response was not an array".to_string()))?;
        let total = pagination_total_from_links(&response.headers, pagination, items.len());
        let has_next = link_header_has_rel(&response.headers, "next");

        let pull_requests = items.iter().map(github_pull_request_from_value).collect::<Vec<_>>();
        let pull_requests = enrich_github_pull_request_counts(ctx, pull_requests).await;

        Ok(ReviewPlatformPullRequestPage {
            items: pull_requests,
            pagination: ReviewPlatformPagination {
                page: pagination.page,
                per_page: pagination.per_page,
                total,
                has_next,
            },
        })
    }

    async fn pull_request_detail(
        &self,
        ctx: &ProviderContext,
        pull_request_id: &str,
    ) -> Result<ReviewPlatformPullRequestDetail, ReviewPlatformError> {
        let base = format!(
            "{}/repos/{}/{}/pulls/{}",
            ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, pull_request_id
        );
        let client = super::super::http::http_client()?;
        let detail = send_json(github_request(client.clone(), &base, ctx.token.as_deref())).await?;
        let token = ctx.token.clone();
        let files_url = format!("{}/files", base);
        let files = fetch_paginated_array(
            |page| {
                let page = page.to_string();
                github_request(client.clone(), &files_url, token.as_deref())
                    .query(&[("per_page", "100"), ("page", &page)])
            },
            github_next_page,
        )
        .await?;
        let token = ctx.token.clone();
        let commits_url = format!("{}/commits", base);
        let commits = fetch_paginated_array(
            |page| {
                let page = page.to_string();
                github_request(client.clone(), &commits_url, token.as_deref())
                    .query(&[("per_page", "100"), ("page", &page)])
            },
            github_next_page,
        )
        .await?;
        let token = ctx.token.clone();
        let reviews_url = format!("{}/reviews", base);
        let reviews = fetch_paginated_array(
            |page| {
                let page = page.to_string();
                github_request(client.clone(), &reviews_url, token.as_deref())
                    .query(&[("per_page", "100"), ("page", &page)])
            },
            github_next_page,
        )
        .await?;
        let token = ctx.token.clone();
        let review_comments_url = format!("{}/comments", base);
        let review_comments = fetch_paginated_array(
            |page| {
                let page = page.to_string();
                github_request(client.clone(), &review_comments_url, token.as_deref())
                    .query(&[("per_page", "100"), ("page", &page)])
            },
            github_next_page,
        )
        .await?;
        let token = ctx.token.clone();
        let issue_comments_url = format!(
            "{}/repos/{}/{}/issues/{}/comments",
            ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, pull_request_id
        );
        let issue_comments = fetch_paginated_array(
            |page| {
                let page = page.to_string();
                github_request(client.clone(), &issue_comments_url, token.as_deref())
                    .query(&[("per_page", "100"), ("page", &page)])
            },
            github_next_page,
        )
        .await?;

        let mut pull_request = github_pull_request_from_value(&detail);
        pull_request.review_decision = github_review_decision(&reviews);
        let (checks, ci) = github_checks_and_ci(ctx, &client, &detail).await;
        pull_request.checks = checks;

        Ok(ReviewPlatformPullRequestDetail {
            body: value_string(&detail, "body"),
            pull_request,
            ci,
            files: array_items(&files).iter().map(github_file_from_value).collect(),
            commits: array_items(&commits).iter().map(github_commit_from_value).collect(),
            threads: github_threads(&reviews, &review_comments, &issue_comments),
        })
    }

    async fn pull_request_detail_page(
        &self,
        ctx: &ProviderContext,
        pull_request_id: &str,
        section: ReviewPlatformDetailSection,
        pagination: PullRequestPagination,
    ) -> Result<ReviewPlatformPullRequestDetailPage, ReviewPlatformError> {
        github_pull_request_detail_page(ctx, pull_request_id, section, pagination).await
    }

    async fn pull_request_ci_log(
        &self,
        ctx: &ProviderContext,
        pull_request_id: &str,
        ci_item_id: &str,
        ci_item_name: &str,
    ) -> Result<ReviewPlatformCiLog, ReviewPlatformError> {
        if ci_item_id.starts_with("status-") {
            return Ok(ReviewPlatformCiLog {
                ci_item_id: ci_item_id.to_string(),
                log: None,
                truncated: false,
                message: Some(
                    "GitHub commit statuses do not expose logs; use the linked target URL instead.".to_string(),
                ),
            });
        }

        let client = super::super::http::http_client()?;
        let base = format!(
            "{}/repos/{}/{}/pulls/{}",
            ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, pull_request_id
        );
        let detail = send_json(github_request(client.clone(), &base, ctx.token.as_deref())).await?;
        let sha = nested_string(&detail, &["head", "sha"]);
        if sha.trim().is_empty() {
            return Ok(ReviewPlatformCiLog {
                ci_item_id: ci_item_id.to_string(),
                log: None,
                truncated: false,
                message: Some("GitHub pull request head SHA was not available.".to_string()),
            });
        }

        let check_run_id = ci_item_id.strip_prefix("check-run-").unwrap_or(ci_item_id);
        github_actions_log_for_check_run_item(ctx, &client, check_run_id, ci_item_name, &sha).await
    }

    async fn create_pull_request(
        &self,
        ctx: &ProviderContext,
        request: &ReviewPlatformCreatePullRequestRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        let token = require_write_token(ctx, "Creating a pull request")?;
        let url = format!(
            "{}/repos/{}/{}/pulls",
            ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name
        );
        let payload = json!({
            "title": request.title,
            "head": request.source_branch,
            "base": request.target_branch,
            "body": request.body.clone().unwrap_or_default(),
            "draft": request.draft.unwrap_or(false),
        });
        let value =
            send_json(github_post_request(super::super::http::http_client()?, &url, Some(token)).json(&payload))
                .await?;
        let pull_request = github_pull_request_from_value(&value);
        let web_url = Some(pull_request.web_url.clone());
        Ok(ReviewPlatformActionResult {
            success: true,
            message: format!("Created pull request #{}", pull_request.number),
            web_url,
            pull_request: Some(pull_request),
            thread: None,
        })
    }

    async fn reply_to_thread(
        &self,
        ctx: &ProviderContext,
        request: &ReviewPlatformReplyToThreadRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        let token = require_write_token(ctx, "Replying to a pull request thread")?;
        let comment_id = parse_provider_comment_id(&request.thread_id).ok_or_else(|| {
            ReviewPlatformError::Api(
                "GitHub replies require a review comment thread id such as comment-123".to_string(),
            )
        })?;
        let url = format!(
            "{}/repos/{}/{}/pulls/{}/comments/{}/replies",
            ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, request.pull_request_id, comment_id
        );
        let value = send_json(
            github_post_request(super::super::http::http_client()?, &url, Some(token))
                .json(&json!({ "body": request.body })),
        )
        .await?;
        let thread = github_thread_from_review_comment(&value);
        Ok(ReviewPlatformActionResult {
            success: true,
            message: "Replied to pull request thread".to_string(),
            web_url: value.get("html_url").and_then(Value::as_str).map(str::to_string),
            pull_request: None,
            thread: Some(thread),
        })
    }

    async fn submit_review(
        &self,
        ctx: &ProviderContext,
        request: &ReviewPlatformSubmitReviewRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        let event = match request.event {
            ReviewSubmitEvent::Comment => "COMMENT",
            ReviewSubmitEvent::Approve => "APPROVE",
            ReviewSubmitEvent::RequestChanges => "REQUEST_CHANGES",
        };
        github_submit_review(ctx, &request.pull_request_id, event, &request.body).await
    }

    async fn approve_pull_request(
        &self,
        ctx: &ProviderContext,
        request: &ReviewPlatformApprovalRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        github_submit_review(
            ctx,
            &request.pull_request_id,
            "APPROVE",
            request.body.as_deref().unwrap_or(""),
        )
        .await
    }

    async fn request_changes(
        &self,
        ctx: &ProviderContext,
        request: &ReviewPlatformRequestChangesRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        github_submit_review(ctx, &request.pull_request_id, "REQUEST_CHANGES", &request.body).await
    }
}

pub(crate) async fn github_submit_review(
    ctx: &ProviderContext,
    pull_request_id: &str,
    event: &str,
    body: &str,
) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
    let token = require_write_token(ctx, "Submitting a pull request review")?;
    let url = format!(
        "{}/repos/{}/{}/pulls/{}/reviews",
        ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, pull_request_id
    );
    let value = send_json(
        github_post_request(super::super::http::http_client()?, &url, Some(token)).json(&json!({
            "body": body,
            "event": event,
        })),
    )
    .await?;
    Ok(ReviewPlatformActionResult {
        success: true,
        message: format!("Submitted GitHub review with event {}", event),
        web_url: value.get("html_url").and_then(Value::as_str).map(str::to_string),
        pull_request: None,
        thread: None,
    })
}

pub(crate) async fn github_pull_request_detail_page(
    ctx: &ProviderContext,
    pull_request_id: &str,
    section: ReviewPlatformDetailSection,
    pagination: PullRequestPagination,
) -> Result<ReviewPlatformPullRequestDetailPage, ReviewPlatformError> {
    let client = super::super::http::http_client()?;
    let base = format!(
        "{}/repos/{}/{}/pulls/{}",
        ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, pull_request_id
    );
    let detail = send_json(github_request(client.clone(), &base, ctx.token.as_deref())).await?;
    let mut pull_request = github_pull_request_from_value(&detail);
    let (checks, ci_all) = github_checks_and_ci(ctx, &client, &detail).await;
    pull_request.checks = checks;

    let mut files = Vec::new();
    let mut commits = Vec::new();
    let mut threads = Vec::new();
    let mut ci = Vec::new();
    let mut section_pagination = empty_detail_pagination(section, pagination);

    match section {
        ReviewPlatformDetailSection::Overview => {}
        ReviewPlatformDetailSection::Ci => {
            section_pagination = pagination_from_total(pagination, ci_all.len());
            ci = slice_page(ci_all, pagination);
        }
        ReviewPlatformDetailSection::Files => {
            let response = fetch_array_page(
                github_request(client.clone(), &format!("{}/files", base), ctx.token.as_deref()),
                pagination,
            )
            .await?;
            section_pagination = pagination_from_response(&response, pagination);
            files = array_items(&response.value)
                .iter()
                .map(github_file_from_value)
                .collect();
        }
        ReviewPlatformDetailSection::Commits => {
            let response = fetch_array_page(
                github_request(client.clone(), &format!("{}/commits", base), ctx.token.as_deref()),
                pagination,
            )
            .await?;
            section_pagination = pagination_from_response(&response, pagination);
            commits = array_items(&response.value)
                .iter()
                .map(github_commit_from_value)
                .collect();
        }
        ReviewPlatformDetailSection::Reviews => {
            let reviews_url = format!("{}/reviews", base);
            let reviews = fetch_array_page(
                github_request(client.clone(), &reviews_url, ctx.token.as_deref()),
                pagination,
            )
            .await?;
            let review_comments = fetch_array_page(
                github_request(client.clone(), &format!("{}/comments", base), ctx.token.as_deref()),
                pagination,
            )
            .await?;
            let issue_comments = fetch_array_page(
                github_request(
                    client.clone(),
                    &format!(
                        "{}/repos/{}/{}/issues/{}/comments",
                        ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, pull_request_id
                    ),
                    ctx.token.as_deref(),
                ),
                pagination,
            )
            .await?;
            pull_request.review_decision = github_review_decision(&reviews.value);
            section_pagination = combine_page_pagination(
                pagination,
                &[
                    pagination_from_response(&reviews, pagination),
                    pagination_from_response(&review_comments, pagination),
                    pagination_from_response(&issue_comments, pagination),
                ],
            );
            threads = github_threads(&reviews.value, &review_comments.value, &issue_comments.value);
        }
    }

    Ok(ReviewPlatformPullRequestDetailPage {
        pull_request,
        body: value_string(&detail, "body"),
        ci,
        files,
        commits,
        threads,
        section,
        pagination: section_pagination,
    })
}

pub(crate) fn github_next_page(headers: &HeaderMap, current_page: u32) -> Option<u32> {
    if link_header_has_rel(headers, "next") {
        Some(current_page.saturating_add(1))
    } else {
        None
    }
}

pub(crate) async fn enrich_github_pull_request_counts(
    ctx: &ProviderContext,
    pull_requests: Vec<ReviewPlatformPullRequest>,
) -> Vec<ReviewPlatformPullRequest> {
    let Ok(client) = super::super::http::http_client() else {
        return pull_requests;
    };
    let futures = pull_requests.into_iter().map(|mut pull_request| {
        let client = client.clone();
        let url = format!(
            "{}/repos/{}/{}/pulls/{}",
            ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, pull_request.id
        );
        let token = ctx.token.clone();
        async move {
            if let Ok(value) = send_json(github_request(client, &url, token.as_deref())).await {
                pull_request.additions = value_i64(&value, "additions") as i32;
                pull_request.deletions = value_i64(&value, "deletions") as i32;
                pull_request.changed_files = value_i64(&value, "changed_files") as i32;
                pull_request.comments = (value_i64(&value, "comments") + value_i64(&value, "review_comments")) as i32;
            }
            pull_request
        }
    });
    stream::iter(futures)
        .buffered(PROVIDER_ENRICH_CONCURRENCY)
        .collect()
        .await
}

pub(crate) fn github_request(client: reqwest::Client, url: &str, token: Option<&str>) -> reqwest::RequestBuilder {
    let mut request = client
        .get(url)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .header(ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(token) = token {
        request = request.header(AUTHORIZATION, format!("Bearer {}", token));
    }
    request
}

pub(crate) fn github_post_request(client: reqwest::Client, url: &str, token: Option<&str>) -> reqwest::RequestBuilder {
    let mut request = client
        .post(url)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .header(ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(token) = token {
        request = request.header(AUTHORIZATION, format!("Bearer {}", token));
    }
    request
}

pub(crate) fn github_pull_request_from_value(value: &Value) -> ReviewPlatformPullRequest {
    let number = value_i64(value, "number");
    let state = if value_bool(value, "draft") {
        ReviewItemState::Draft
    } else if !value_string(value, "merged_at").is_empty() {
        ReviewItemState::Merged
    } else {
        match value_string(value, "state").as_str() {
            "closed" => ReviewItemState::Closed,
            _ => ReviewItemState::Open,
        }
    };

    ReviewPlatformPullRequest {
        id: number.to_string(),
        number,
        title: value_string(value, "title"),
        state,
        author: nested_string(value, &["user", "login"]),
        source_branch: nested_string(value, &["head", "ref"]),
        target_branch: nested_string(value, &["base", "ref"]),
        updated_at: value_string(value, "updated_at"),
        web_url: value_string(value, "html_url"),
        additions: value_i64(value, "additions") as i32,
        deletions: value_i64(value, "deletions") as i32,
        changed_files: value_i64(value, "changed_files") as i32,
        comments: (value_i64(value, "comments") + value_i64(value, "review_comments")) as i32,
        review_decision: ReviewDecision::Pending,
        checks: empty_checks(),
    }
}

pub(crate) fn github_file_from_value(value: &Value) -> ReviewPlatformFile {
    ReviewPlatformFile {
        path: value_string(value, "filename"),
        old_path: value
            .get("previous_filename")
            .and_then(Value::as_str)
            .map(str::to_string),
        status: file_status(&value_string(value, "status")),
        additions: value_i64(value, "additions") as i32,
        deletions: value_i64(value, "deletions") as i32,
        patch: optional_string(value, "patch"),
    }
}

pub(crate) fn github_commit_from_value(value: &Value) -> ReviewPlatformCommit {
    let hash = value_string(value, "sha");
    ReviewPlatformCommit {
        short_hash: short_hash(&hash),
        hash,
        title: first_line(&nested_string(value, &["commit", "message"])),
        author: first_non_empty(&[
            nested_string(value, &["author", "login"]),
            nested_string(value, &["commit", "author", "name"]),
        ]),
        committed_at: nested_string(value, &["commit", "author", "date"]),
    }
}

pub(crate) fn github_review_decision(reviews: &Value) -> ReviewDecision {
    let mut latest_by_author: HashMap<String, String> = HashMap::new();
    let mut anonymous_states = Vec::new();
    for review in array_items(reviews) {
        let state = value_string(review, "state");
        if state == "DISMISSED" || state.trim().is_empty() {
            continue;
        }
        let author = nested_string(review, &["user", "login"]);
        if author.trim().is_empty() {
            anonymous_states.push(state);
        } else {
            latest_by_author.insert(author, state);
        }
    }

    let states = latest_by_author
        .values()
        .chain(anonymous_states.iter())
        .map(String::as_str)
        .collect::<Vec<_>>();

    if states.contains(&"CHANGES_REQUESTED") {
        return ReviewDecision::ChangesRequested;
    }
    if states.contains(&"APPROVED") {
        return ReviewDecision::Approved;
    }
    if states.contains(&"COMMENTED") {
        return ReviewDecision::Commented;
    }
    ReviewDecision::Pending
}

pub(crate) fn github_threads(
    reviews: &Value,
    review_comments: &Value,
    issue_comments: &Value,
) -> Vec<ReviewPlatformThread> {
    let mut threads = Vec::new();
    for review in array_items(reviews) {
        let body = github_review_body(review);
        threads.push(ReviewPlatformThread {
            id: format!("review-{}", value_i64(review, "id")),
            provider_thread_id: None,
            provider_comment_id: value_i64(review, "id").checked_abs().map(|id| id.to_string()),
            kind: ReviewPlatformThreadKind::Review,
            reply_to_provider_comment_id: None,
            file_path: None,
            line: None,
            resolved: false,
            author: nested_string(review, &["user", "login"]),
            body,
            updated_at: first_non_empty(&[value_string(review, "submitted_at"), value_string(review, "updated_at")]),
        });
    }
    for comment in array_items(review_comments) {
        threads.push(github_thread_from_review_comment(comment));
    }
    for comment in array_items(issue_comments) {
        threads.push(github_thread_from_issue_comment(comment));
    }
    threads
}

fn github_review_body(review: &Value) -> String {
    let body = value_string(review, "body");
    if !body.trim().is_empty() {
        return body;
    }
    match value_string(review, "state").as_str() {
        "APPROVED" => "Approved this pull request.".to_string(),
        "CHANGES_REQUESTED" => "Requested changes.".to_string(),
        "COMMENTED" => "Submitted a pull request review.".to_string(),
        state if !state.trim().is_empty() => format!("Submitted a {} review.", state),
        _ => "Submitted a pull request review.".to_string(),
    }
}

fn github_thread_from_review_comment(comment: &Value) -> ReviewPlatformThread {
    let comment_id = first_non_empty(&[value_string(comment, "id"), value_i64(comment, "id").to_string()]);
    ReviewPlatformThread {
        id: format!("comment-{}", comment_id),
        provider_thread_id: None,
        provider_comment_id: Some(comment_id),
        kind: ReviewPlatformThreadKind::Comment,
        reply_to_provider_comment_id: value_i64(comment, "in_reply_to_id")
            .checked_abs()
            .map(|id| id.to_string())
            .or_else(|| {
                comment
                    .get("in_reply_to_id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            }),
        file_path: comment.get("path").and_then(Value::as_str).map(str::to_string),
        line: comment
            .get("line")
            .and_then(Value::as_i64)
            .or_else(|| comment.get("original_line").and_then(Value::as_i64)),
        resolved: false,
        author: nested_string(comment, &["user", "login"]),
        body: value_string(comment, "body"),
        updated_at: value_string(comment, "updated_at"),
    }
}

fn github_thread_from_issue_comment(comment: &Value) -> ReviewPlatformThread {
    let comment_id = first_non_empty(&[value_string(comment, "id"), value_i64(comment, "id").to_string()]);
    ReviewPlatformThread {
        id: format!("issue-comment-{}", comment_id),
        provider_thread_id: None,
        provider_comment_id: Some(comment_id),
        kind: ReviewPlatformThreadKind::Comment,
        reply_to_provider_comment_id: None,
        file_path: None,
        line: None,
        resolved: false,
        author: nested_string(comment, &["user", "login"]),
        body: value_string(comment, "body"),
        updated_at: value_string(comment, "updated_at"),
    }
}
