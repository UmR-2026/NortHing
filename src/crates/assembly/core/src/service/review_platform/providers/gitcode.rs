//! GitCode provider implementation — REST API v5 calls and DTO mapping.

use super::super::auth::require_write_token;
use super::super::http::{
    empty_detail_pagination, fetch_array_page, fetch_paginated_array, header_u64, link_header_has_rel,
    link_header_last_page, pagination_from_response, pagination_from_total, send_json, send_json_response, slice_page,
};
use super::super::types::{
    ProviderContext, PullRequestPagination, ReviewDecision, ReviewItemState, ReviewPlatformActionResult,
    ReviewPlatformApprovalRequest, ReviewPlatformCommit, ReviewPlatformCreatePullRequestRequest,
    ReviewPlatformDetailSection, ReviewPlatformError, ReviewPlatformFile, ReviewPlatformPagination,
    ReviewPlatformPullRequest, ReviewPlatformPullRequestDetail, ReviewPlatformPullRequestDetailPage,
    ReviewPlatformPullRequestPage, ReviewPlatformThread, ReviewPlatformThreadKind, ReviewSubmitEvent,
};
use super::ci::{gitcode_ci_items, summarize_ci_items};
use super::util::{
    array_items, empty_checks, file_status, first_non_empty, first_non_zero, nested_string, optional_string,
    short_hash, value_i64, value_string,
};
use futures::{stream, StreamExt};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde_json::{json, Value};

pub(crate) struct GitcodeProvider;

const USER_AGENT_VALUE: &str = "ReviewPlatform";

#[async_trait::async_trait]
impl super::ReviewProvider for GitcodeProvider {
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
            gitcode_request(super::super::http::http_client()?, &url, ctx.token.as_deref()).query(&[
                ("state", "all"),
                ("per_page", &per_page),
                ("page", &page),
            ]),
        )
        .await?;
        let items = response
            .value
            .as_array()
            .ok_or_else(|| ReviewPlatformError::Parse("GitCode pull response was not an array".to_string()))?;
        let total = header_u64(&response.headers, "x-total").or_else(|| {
            link_header_last_page(&response.headers).map(|last_page| {
                if last_page == pagination.page {
                    (u64::from(last_page.saturating_sub(1)) * u64::from(pagination.per_page)) + items.len() as u64
                } else {
                    u64::from(last_page) * u64::from(pagination.per_page)
                }
            })
        });
        let has_next = link_header_has_rel(&response.headers, "next")
            || total
                .map(|total| u64::from(pagination.page) * u64::from(pagination.per_page) < total)
                .unwrap_or(items.len() == pagination.per_page as usize);

        let pull_requests = items.iter().map(gitcode_pull_request_from_value).collect::<Vec<_>>();
        let pull_requests = enrich_gitcode_pull_request_counts(ctx, pull_requests).await;

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
        let client = super::super::http::http_client()?;
        let base = format!(
            "{}/repos/{}/{}/pulls/{}",
            ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, pull_request_id
        );
        let detail = send_json(gitcode_request(client.clone(), &base, ctx.token.as_deref())).await?;
        let token = ctx.token.clone();
        let files_url = format!("{}/files", base);
        let files = fetch_paginated_array(
            |page| {
                let page = page.to_string();
                gitcode_request(client.clone(), &files_url, token.as_deref())
                    .query(&[("per_page", "100"), ("page", &page)])
            },
            super::github::github_next_page,
        )
        .await
        .unwrap_or(Value::Array(Vec::new()));
        let token = ctx.token.clone();
        let commits_url = format!("{}/commits", base);
        let commits = fetch_paginated_array(
            |page| {
                let page = page.to_string();
                gitcode_request(client.clone(), &commits_url, token.as_deref())
                    .query(&[("per_page", "100"), ("page", &page)])
            },
            super::github::github_next_page,
        )
        .await
        .unwrap_or(Value::Array(Vec::new()));
        let token = ctx.token.clone();
        let comments_url = format!("{}/comments", base);
        let comments = fetch_paginated_array(
            |page| {
                let page = page.to_string();
                gitcode_request(client.clone(), &comments_url, token.as_deref())
                    .query(&[("per_page", "100"), ("page", &page)])
            },
            super::github::github_next_page,
        )
        .await
        .unwrap_or(Value::Array(Vec::new()));
        let ci = gitcode_ci_items(&detail);
        let mut pull_request = gitcode_pull_request_from_value(&detail);
        pull_request.checks = summarize_ci_items(&ci);

        Ok(ReviewPlatformPullRequestDetail {
            body: first_non_empty(&[value_string(&detail, "body"), value_string(&detail, "description")]),
            pull_request,
            ci,
            files: array_items(&files).iter().map(gitcode_file_from_value).collect(),
            commits: array_items(&commits).iter().map(gitcode_commit_from_value).collect(),
            threads: gitcode_threads(&comments),
        })
    }

    async fn pull_request_detail_page(
        &self,
        ctx: &ProviderContext,
        pull_request_id: &str,
        section: ReviewPlatformDetailSection,
        pagination: PullRequestPagination,
    ) -> Result<ReviewPlatformPullRequestDetailPage, ReviewPlatformError> {
        gitcode_pull_request_detail_page(ctx, pull_request_id, section, pagination).await
    }

    async fn pull_request_ci_log(
        &self,
        _ctx: &ProviderContext,
        _pull_request_id: &str,
        ci_item_id: &str,
        _ci_item_name: &str,
    ) -> Result<super::super::types::ReviewPlatformCiLog, ReviewPlatformError> {
        Ok(super::super::types::ReviewPlatformCiLog {
            ci_item_id: ci_item_id.to_string(),
            log: None,
            truncated: false,
            message: Some("GitCode CI log retrieval is not available through a documented API.".to_string()),
        })
    }

    async fn create_pull_request(
        &self,
        ctx: &ProviderContext,
        request: &ReviewPlatformCreatePullRequestRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        let token = require_write_token(ctx, "Creating a GitCode pull request")?;
        let url = format!(
            "{}/repos/{}/{}/pulls",
            ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name
        );
        let value = send_json(
            gitcode_post_request(super::super::http::http_client()?, &url, Some(token)).json(&json!({
                "title": request.title,
                "head": request.source_branch,
                "base": request.target_branch,
                "body": request.body.clone().unwrap_or_default(),
                "draft": request.draft.unwrap_or(false),
            })),
        )
        .await?;
        let pull_request = gitcode_pull_request_from_value(&value);
        let web_url = Some(pull_request.web_url.clone());
        Ok(ReviewPlatformActionResult {
            success: true,
            message: format!("Created GitCode pull request #{}", pull_request.number),
            web_url,
            pull_request: Some(pull_request),
            thread: None,
        })
    }

    async fn submit_review(
        &self,
        ctx: &ProviderContext,
        request: &super::super::types::ReviewPlatformSubmitReviewRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        if request.event != ReviewSubmitEvent::Comment {
            return Err(ReviewPlatformError::UnsupportedPlatform(
                "GitCode submit_review supports comments only; use approve_pull_request for review processing"
                    .to_string(),
            ));
        }
        gitcode_add_pull_request_comment(ctx, &request.pull_request_id, &request.body).await
    }

    async fn approve_pull_request(
        &self,
        ctx: &ProviderContext,
        request: &ReviewPlatformApprovalRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        let token = require_write_token(ctx, "Approving a GitCode pull request")?;
        let url = format!(
            "{}/repos/{}/{}/pulls/{}/review",
            ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, request.pull_request_id
        );
        send_json(
            gitcode_post_request(super::super::http::http_client()?, &url, Some(token))
                .json(&json!({ "force": false })),
        )
        .await?;
        if let Some(body) = request.body.as_deref().filter(|value| !value.trim().is_empty()) {
            let _ = gitcode_add_pull_request_comment(ctx, &request.pull_request_id, body).await;
        }
        Ok(ReviewPlatformActionResult {
            success: true,
            message: "Approved GitCode pull request".to_string(),
            web_url: None,
            pull_request: None,
            thread: None,
        })
    }
}

pub(crate) async fn gitcode_add_pull_request_comment(
    ctx: &ProviderContext,
    pull_request_id: &str,
    body: &str,
) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
    let token = require_write_token(ctx, "Adding a GitCode pull request comment")?;
    let url = format!(
        "{}/repos/{}/{}/pulls/{}/comments",
        ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, pull_request_id
    );
    let value = send_json(
        gitcode_post_request(super::super::http::http_client()?, &url, Some(token)).json(&json!({ "body": body })),
    )
    .await?;
    let thread = gitcode_threads(&Value::Array(vec![value])).into_iter().next();
    Ok(ReviewPlatformActionResult {
        success: true,
        message: "Added GitCode pull request comment".to_string(),
        web_url: None,
        pull_request: None,
        thread,
    })
}

pub(crate) async fn gitcode_pull_request_detail_page(
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
    let detail = send_json(gitcode_request(client.clone(), &base, ctx.token.as_deref())).await?;
    let mut ci = gitcode_ci_items(&detail);
    let mut pull_request = gitcode_pull_request_from_value(&detail);
    pull_request.checks = summarize_ci_items(&ci);
    let mut files = Vec::new();
    let mut commits = Vec::new();
    let mut threads = Vec::new();
    let mut section_pagination = empty_detail_pagination(section, pagination);

    match section {
        ReviewPlatformDetailSection::Overview => {}
        ReviewPlatformDetailSection::Ci => {
            section_pagination = pagination_from_total(pagination, ci.len());
            ci = slice_page(ci, pagination);
        }
        ReviewPlatformDetailSection::Files => {
            if let Ok(response) = fetch_array_page(
                gitcode_request(client.clone(), &format!("{}/files", base), ctx.token.as_deref()),
                pagination,
            )
            .await
            {
                section_pagination = pagination_from_response(&response, pagination);
                files = array_items(&response.value)
                    .iter()
                    .map(gitcode_file_from_value)
                    .collect();
            }
        }
        ReviewPlatformDetailSection::Commits => {
            if let Ok(response) = fetch_array_page(
                gitcode_request(client.clone(), &format!("{}/commits", base), ctx.token.as_deref()),
                pagination,
            )
            .await
            {
                section_pagination = pagination_from_response(&response, pagination);
                commits = array_items(&response.value)
                    .iter()
                    .map(gitcode_commit_from_value)
                    .collect();
            }
        }
        ReviewPlatformDetailSection::Reviews => {
            if let Ok(response) = fetch_array_page(
                gitcode_request(client.clone(), &format!("{}/comments", base), ctx.token.as_deref()),
                pagination,
            )
            .await
            {
                section_pagination = pagination_from_response(&response, pagination);
                threads = gitcode_threads(&response.value);
            }
        }
    }

    Ok(ReviewPlatformPullRequestDetailPage {
        body: first_non_empty(&[value_string(&detail, "body"), value_string(&detail, "description")]),
        pull_request,
        ci,
        files,
        commits,
        threads,
        section,
        pagination: section_pagination,
    })
}

pub(crate) async fn enrich_gitcode_pull_request_counts(
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
            if let Ok(value) = send_json(gitcode_request(client, &url, token.as_deref())).await {
                let detail = gitcode_pull_request_from_value(&value);
                pull_request.additions = detail.additions;
                pull_request.deletions = detail.deletions;
                pull_request.changed_files = detail.changed_files;
                pull_request.comments = detail.comments;
            }
            pull_request
        }
    });
    stream::iter(futures)
        .buffered(super::super::types::PROVIDER_ENRICH_CONCURRENCY)
        .collect()
        .await
}

pub(crate) fn gitcode_request(client: reqwest::Client, url: &str, token: Option<&str>) -> reqwest::RequestBuilder {
    let mut request = client
        .get(url)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .header(ACCEPT, "application/json");
    if let Some(token) = token {
        request = request
            .header("PRIVATE-TOKEN", token)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .query(&[("access_token", token)]);
    }
    request
}

pub(crate) fn gitcode_post_request(client: reqwest::Client, url: &str, token: Option<&str>) -> reqwest::RequestBuilder {
    let mut request = client
        .post(url)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .header(ACCEPT, "application/json");
    if let Some(token) = token {
        request = request
            .header("PRIVATE-TOKEN", token)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .query(&[("access_token", token)]);
    }
    request
}

pub(crate) fn gitcode_pull_request_from_value(value: &Value) -> ReviewPlatformPullRequest {
    let number = first_non_zero(&[value_i64(value, "number"), value_i64(value, "id")]);
    let state = match value_string(value, "state").as_str() {
        "merged" => ReviewItemState::Merged,
        "closed" => ReviewItemState::Closed,
        _ => ReviewItemState::Open,
    };
    ReviewPlatformPullRequest {
        id: number.to_string(),
        number,
        title: value_string(value, "title"),
        state,
        author: first_non_empty(&[
            nested_string(value, &["user", "login"]),
            nested_string(value, &["user", "name"]),
            nested_string(value, &["author", "login"]),
        ]),
        source_branch: first_non_empty(&[
            nested_string(value, &["head", "ref"]),
            value_string(value, "head_branch"),
        ]),
        target_branch: first_non_empty(&[
            nested_string(value, &["base", "ref"]),
            value_string(value, "base_branch"),
        ]),
        updated_at: value_string(value, "updated_at"),
        web_url: first_non_empty(&[value_string(value, "html_url"), value_string(value, "web_url")]),
        additions: value_i64(value, "additions") as i32,
        deletions: value_i64(value, "deletions") as i32,
        changed_files: value_i64(value, "changed_files") as i32,
        comments: value_i64(value, "comments") as i32,
        review_decision: ReviewDecision::Pending,
        checks: empty_checks(),
    }
}

pub(crate) fn gitcode_file_from_value(value: &Value) -> ReviewPlatformFile {
    ReviewPlatformFile {
        path: first_non_empty(&[value_string(value, "filename"), value_string(value, "new_path")]),
        old_path: value
            .get("previous_filename")
            .and_then(Value::as_str)
            .map(str::to_string),
        status: file_status(&value_string(value, "status")),
        additions: value_i64(value, "additions") as i32,
        deletions: value_i64(value, "deletions") as i32,
        patch: optional_string(value, "patch").or_else(|| optional_string(value, "diff")),
    }
}

pub(crate) fn gitcode_commit_from_value(value: &Value) -> ReviewPlatformCommit {
    let hash = first_non_empty(&[value_string(value, "sha"), value_string(value, "id")]);
    ReviewPlatformCommit {
        short_hash: short_hash(&hash),
        hash,
        title: first_non_empty(&[
            nested_string(value, &["commit", "message"])
                .lines()
                .next()
                .unwrap_or_default()
                .to_string(),
            value_string(value, "message"),
        ]),
        author: first_non_empty(&[
            nested_string(value, &["author", "login"]),
            nested_string(value, &["commit", "author", "name"]),
        ]),
        committed_at: first_non_empty(&[
            nested_string(value, &["commit", "author", "date"]),
            value_string(value, "created_at"),
        ]),
    }
}

pub(crate) fn gitcode_threads(value: &Value) -> Vec<ReviewPlatformThread> {
    array_items(value)
        .iter()
        .map(|comment| ReviewPlatformThread {
            id: value_string(comment, "id"),
            provider_thread_id: None,
            provider_comment_id: Some(value_string(comment, "id")),
            kind: ReviewPlatformThreadKind::Comment,
            reply_to_provider_comment_id: comment
                .get("in_reply_to_id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    comment
                        .get("in_reply_to_id")
                        .and_then(Value::as_i64)
                        .map(|id| id.to_string())
                }),
            file_path: comment.get("path").and_then(Value::as_str).map(str::to_string),
            line: comment.get("line").and_then(Value::as_i64),
            resolved: false,
            author: first_non_empty(&[
                nested_string(comment, &["user", "login"]),
                nested_string(comment, &["user", "name"]),
            ]),
            body: value_string(comment, "body"),
            updated_at: first_non_empty(&[value_string(comment, "updated_at"), value_string(comment, "created_at")]),
        })
        .collect()
}
