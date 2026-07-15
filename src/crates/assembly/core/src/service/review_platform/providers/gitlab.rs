//! GitLab provider implementation — REST API v4 calls and DTO mapping.

use super::super::auth::require_write_token;
use super::super::http::{
    fetch_array_page, fetch_paginated_array, header_string, header_u64, pagination_from_response,
    pagination_from_total, send_json, send_json_response, slice_page,
};
use super::super::types::{
    ProviderContext, PullRequestPagination, ReviewDecision, ReviewItemState, ReviewPlatformActionResult,
    ReviewPlatformApprovalRequest, ReviewPlatformCiLog, ReviewPlatformCommit, ReviewPlatformCreatePullRequestRequest,
    ReviewPlatformDetailSection, ReviewPlatformError, ReviewPlatformFile, ReviewPlatformPagination,
    ReviewPlatformPullRequest, ReviewPlatformPullRequestDetail, ReviewPlatformPullRequestDetailPage,
    ReviewPlatformPullRequestPage, ReviewPlatformReplyToThreadRequest, ReviewPlatformResolveThreadRequest,
    ReviewPlatformThread, ReviewPlatformThreadKind, ReviewSubmitEvent,
};
use super::ci::{gitlab_pipeline_jobs, gitlab_pipeline_summary_item, gitlab_pull_request_ci_log, summarize_ci_items};
use super::util::{
    apply_files_stats, array_items, count_diff_lines, empty_checks, first_non_empty, parse_provider_thread_id,
    short_hash, value_bool, value_string,
};
use futures::{stream, StreamExt};
use reqwest::header::{HeaderMap, ACCEPT, USER_AGENT};
use serde_json::{json, Value};

pub(crate) struct GitlabProvider;

const USER_AGENT_VALUE: &str = "ReviewPlatform";

#[async_trait::async_trait]
impl super::ReviewProvider for GitlabProvider {
    async fn list_pull_requests(
        &self,
        ctx: &ProviderContext,
        pagination: PullRequestPagination,
    ) -> Result<ReviewPlatformPullRequestPage, ReviewPlatformError> {
        gitlab_list_pull_requests(ctx, pagination).await
    }

    async fn pull_request_detail(
        &self,
        ctx: &ProviderContext,
        pull_request_id: &str,
    ) -> Result<ReviewPlatformPullRequestDetail, ReviewPlatformError> {
        gitlab_pull_request_detail(ctx, pull_request_id).await
    }

    async fn pull_request_detail_page(
        &self,
        ctx: &ProviderContext,
        pull_request_id: &str,
        section: ReviewPlatformDetailSection,
        pagination: PullRequestPagination,
    ) -> Result<ReviewPlatformPullRequestDetailPage, ReviewPlatformError> {
        gitlab_pull_request_detail_page(ctx, pull_request_id, section, pagination).await
    }

    async fn pull_request_ci_log(
        &self,
        ctx: &ProviderContext,
        pull_request_id: &str,
        ci_item_id: &str,
        ci_item_name: &str,
    ) -> Result<ReviewPlatformCiLog, ReviewPlatformError> {
        gitlab_pull_request_ci_log(ctx, pull_request_id, ci_item_id, ci_item_name).await
    }

    async fn create_pull_request(
        &self,
        ctx: &ProviderContext,
        request: &ReviewPlatformCreatePullRequestRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        gitlab_create_pull_request(ctx, request, "merge request").await
    }

    async fn reply_to_thread(
        &self,
        ctx: &ProviderContext,
        request: &ReviewPlatformReplyToThreadRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        gitlab_reply_to_thread(ctx, request, "merge request").await
    }

    async fn submit_review(
        &self,
        ctx: &ProviderContext,
        request: &super::super::types::ReviewPlatformSubmitReviewRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        if request.event != ReviewSubmitEvent::Comment {
            return Err(ReviewPlatformError::UnsupportedPlatform(
                "GitLab submit_review supports comments only; use approve_pull_request for approvals".to_string(),
            ));
        }
        gitlab_add_merge_request_note(
            ctx,
            &request.pull_request_id,
            &request.body,
            "Added merge request comment",
        )
        .await
    }

    async fn resolve_thread(
        &self,
        ctx: &ProviderContext,
        request: &ReviewPlatformResolveThreadRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        gitlab_resolve_thread(ctx, request, "merge request").await
    }

    async fn approve_pull_request(
        &self,
        ctx: &ProviderContext,
        request: &ReviewPlatformApprovalRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        gitlab_approve_pull_request(ctx, request, "merge request").await
    }

    async fn revoke_approval(
        &self,
        ctx: &ProviderContext,
        request: &ReviewPlatformApprovalRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        gitlab_revoke_approval(ctx, request, "merge request").await
    }
}

pub(crate) async fn gitlab_list_pull_requests(
    ctx: &ProviderContext,
    pagination: PullRequestPagination,
) -> Result<ReviewPlatformPullRequestPage, ReviewPlatformError> {
    let project = urlencoding::encode(&ctx.remote.project_path);
    let url = format!("{}/projects/{}/merge_requests", ctx.api_base_url, project);
    let per_page = pagination.per_page.to_string();
    let page = pagination.page.to_string();
    let response = send_json_response(
        gitlab_request(super::super::http::http_client()?, &url, ctx.token.as_deref()).query(&[
            ("state", "all"),
            ("per_page", &per_page),
            ("page", &page),
        ]),
    )
    .await?;
    let items = response
        .value
        .as_array()
        .ok_or_else(|| ReviewPlatformError::Parse("GitLab merge request response was not an array".to_string()))?;
    let total = header_u64(&response.headers, "x-total");
    let has_next = header_string(&response.headers, "x-next-page").is_some_and(|value| !value.trim().is_empty())
        || total
            .map(|total| u64::from(pagination.page) * u64::from(pagination.per_page) < total)
            .unwrap_or(false);

    let pull_requests = items.iter().map(gitlab_pull_request_from_value).collect::<Vec<_>>();
    let pull_requests = enrich_gitlab_pull_request_counts(ctx, pull_requests).await;

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

pub(crate) async fn gitlab_pull_request_detail(
    ctx: &ProviderContext,
    pull_request_id: &str,
) -> Result<ReviewPlatformPullRequestDetail, ReviewPlatformError> {
    let client = super::super::http::http_client()?;
    let project = urlencoding::encode(&ctx.remote.project_path);
    let base = format!(
        "{}/projects/{}/merge_requests/{}",
        ctx.api_base_url, project, pull_request_id
    );
    let detail = send_json(gitlab_request(client.clone(), &base, ctx.token.as_deref())).await?;
    let changes = send_json(gitlab_request(
        client.clone(),
        &format!("{}/changes", base),
        ctx.token.as_deref(),
    ))
    .await?;
    let token = ctx.token.clone();
    let commits_url = format!("{}/commits", base);
    let commits = fetch_paginated_array(
        |page| {
            let page = page.to_string();
            gitlab_request(client.clone(), &commits_url, token.as_deref())
                .query(&[("per_page", "100"), ("page", &page)])
        },
        gitlab_next_page,
    )
    .await?;
    let token = ctx.token.clone();
    let discussions_url = format!("{}/discussions", base);
    let discussions = fetch_paginated_array(
        |page| {
            let page = page.to_string();
            gitlab_request(client.clone(), &discussions_url, token.as_deref())
                .query(&[("per_page", "100"), ("page", &page)])
        },
        gitlab_next_page,
    )
    .await?;
    let token = ctx.token.clone();
    let notes_url = format!("{}/notes", base);
    let notes = fetch_paginated_array(
        |page| {
            let page = page.to_string();
            gitlab_request(client.clone(), &notes_url, token.as_deref()).query(&[("per_page", "100"), ("page", &page)])
        },
        gitlab_next_page,
    )
    .await?;

    let mut pull_request = gitlab_pull_request_from_value(&detail);
    let files = gitlab_files(&changes);
    apply_files_stats(&mut pull_request, &files);
    let ci = gitlab_pipeline_summary_item(&detail).into_iter().collect::<Vec<_>>();
    pull_request.checks = summarize_ci_items(&ci);

    Ok(ReviewPlatformPullRequestDetail {
        body: value_string(&detail, "description"),
        pull_request,
        ci,
        files,
        commits: array_items(&commits).iter().map(gitlab_commit_from_value).collect(),
        threads: gitlab_threads(&discussions, &notes),
    })
}

pub(crate) async fn gitlab_pull_request_detail_page(
    ctx: &ProviderContext,
    pull_request_id: &str,
    section: ReviewPlatformDetailSection,
    pagination: PullRequestPagination,
) -> Result<ReviewPlatformPullRequestDetailPage, ReviewPlatformError> {
    let client = super::super::http::http_client()?;
    let project = urlencoding::encode(&ctx.remote.project_path);
    let base = format!(
        "{}/projects/{}/merge_requests/{}",
        ctx.api_base_url, project, pull_request_id
    );
    let detail = send_json(gitlab_request(client.clone(), &base, ctx.token.as_deref())).await?;
    let mut pull_request = gitlab_pull_request_from_value(&detail);
    let changes = send_json(gitlab_request(
        client.clone(),
        &format!("{}/changes", base),
        ctx.token.as_deref(),
    ))
    .await?;
    let all_files = gitlab_files(&changes);
    apply_files_stats(&mut pull_request, &all_files);
    let mut ci = gitlab_pipeline_summary_item(&detail).into_iter().collect::<Vec<_>>();
    pull_request.checks = summarize_ci_items(&ci);
    let mut files = Vec::new();
    let mut commits = Vec::new();
    let mut threads = Vec::new();
    let mut section_pagination = super::super::http::empty_detail_pagination(section, pagination);

    match section {
        ReviewPlatformDetailSection::Overview => {}
        ReviewPlatformDetailSection::Ci => {
            if let Some(pipeline_id) = detail
                .get("head_pipeline")
                .and_then(|value| value.get("id"))
                .and_then(Value::as_i64)
                .map(|id| id.to_string())
                .or_else(|| {
                    detail
                        .get("head_pipeline")
                        .and_then(|value| value.get("id"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
            {
                let jobs = gitlab_pipeline_jobs(
                    ctx,
                    client.clone(),
                    &urlencoding::encode(&ctx.remote.project_path),
                    &pipeline_id,
                )
                .await;
                if !jobs.is_empty() {
                    ci = jobs;
                    pull_request.checks = summarize_ci_items(&ci);
                }
            }
            section_pagination = pagination_from_total(pagination, ci.len());
            ci = slice_page(ci, pagination);
        }
        ReviewPlatformDetailSection::Files => {
            section_pagination = pagination_from_total(pagination, all_files.len());
            files = slice_page(all_files, pagination);
        }
        ReviewPlatformDetailSection::Commits => {
            let response = fetch_array_page(
                gitlab_request(client.clone(), &format!("{}/commits", base), ctx.token.as_deref()),
                pagination,
            )
            .await?;
            section_pagination = pagination_from_response(&response, pagination);
            commits = array_items(&response.value)
                .iter()
                .map(gitlab_commit_from_value)
                .collect();
        }
        ReviewPlatformDetailSection::Reviews => {
            let discussions = fetch_array_page(
                gitlab_request(client.clone(), &format!("{}/discussions", base), ctx.token.as_deref()),
                pagination,
            )
            .await?;
            let notes = fetch_array_page(
                gitlab_request(client.clone(), &format!("{}/notes", base), ctx.token.as_deref()),
                pagination,
            )
            .await?;
            section_pagination = super::super::http::combine_page_pagination(
                pagination,
                &[
                    pagination_from_response(&discussions, pagination),
                    pagination_from_response(&notes, pagination),
                ],
            );
            threads = gitlab_threads(&discussions.value, &notes.value);
        }
    }

    Ok(ReviewPlatformPullRequestDetailPage {
        pull_request,
        body: value_string(&detail, "description"),
        ci,
        files,
        commits,
        threads,
        section,
        pagination: section_pagination,
    })
}

pub(crate) async fn gitlab_create_pull_request(
    ctx: &ProviderContext,
    request: &ReviewPlatformCreatePullRequestRequest,
    label: &str,
) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
    let token = require_write_token(ctx, &format!("Creating a {}", label))?;
    let project = urlencoding::encode(&ctx.remote.project_path);
    let url = format!("{}/projects/{}/merge_requests", ctx.api_base_url, project);
    let value = send_json(
        gitlab_post_request(super::super::http::http_client()?, &url, Some(token)).json(&json!({
            "title": request.title,
            "source_branch": request.source_branch,
            "target_branch": request.target_branch,
            "description": request.body.clone().unwrap_or_default(),
        })),
    )
    .await?;
    let pull_request = gitlab_pull_request_from_value(&value);
    let web_url = Some(pull_request.web_url.clone());
    Ok(ReviewPlatformActionResult {
        success: true,
        message: format!("Created {} !{}", label, pull_request.number),
        web_url,
        pull_request: Some(pull_request),
        thread: None,
    })
}

pub(crate) async fn gitlab_reply_to_thread(
    ctx: &ProviderContext,
    request: &ReviewPlatformReplyToThreadRequest,
    label: &str,
) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
    let token = require_write_token(ctx, &format!("Replying to a {} thread", label))?;
    let discussion_id = parse_provider_thread_id(&request.thread_id).ok_or_else(|| {
        ReviewPlatformError::Api("Replies require a discussion thread id from pull request detail".to_string())
    })?;
    let project = urlencoding::encode(&ctx.remote.project_path);
    let url = format!(
        "{}/projects/{}/merge_requests/{}/discussions/{}/notes",
        ctx.api_base_url, project, request.pull_request_id, discussion_id
    );
    let value = send_json(
        gitlab_post_request(super::super::http::http_client()?, &url, Some(token))
            .json(&json!({ "body": request.body })),
    )
    .await?;
    let thread = gitlab_thread_from_note(
        &value,
        Some(discussion_id.to_string()),
        false,
        ReviewPlatformThreadKind::Comment,
        None,
    );
    Ok(ReviewPlatformActionResult {
        success: true,
        message: format!("Replied to {} discussion", label),
        web_url: None,
        pull_request: None,
        thread: Some(thread),
    })
}

pub(crate) async fn gitlab_add_merge_request_note(
    ctx: &ProviderContext,
    pull_request_id: &str,
    body: &str,
    message: &str,
) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
    let token = require_write_token(ctx, "Adding a merge request comment")?;
    let project = urlencoding::encode(&ctx.remote.project_path);
    let url = format!(
        "{}/projects/{}/merge_requests/{}/notes",
        ctx.api_base_url, project, pull_request_id
    );
    let value = send_json(
        gitlab_post_request(super::super::http::http_client()?, &url, Some(token)).json(&json!({ "body": body })),
    )
    .await?;
    let thread = gitlab_thread_from_note(&value, None, false, ReviewPlatformThreadKind::Comment, None);
    Ok(ReviewPlatformActionResult {
        success: true,
        message: message.to_string(),
        web_url: None,
        pull_request: None,
        thread: Some(thread),
    })
}

pub(crate) async fn gitlab_resolve_thread(
    ctx: &ProviderContext,
    request: &ReviewPlatformResolveThreadRequest,
    label: &str,
) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
    let token = require_write_token(ctx, &format!("Resolving a {} thread", label))?;
    let discussion_id = parse_provider_thread_id(&request.thread_id).ok_or_else(|| {
        ReviewPlatformError::Api(
            "Thread resolution requires a discussion thread id from pull request detail".to_string(),
        )
    })?;
    let project = urlencoding::encode(&ctx.remote.project_path);
    let url = format!(
        "{}/projects/{}/merge_requests/{}/discussions/{}",
        ctx.api_base_url, project, request.pull_request_id, discussion_id
    );
    send_json(
        gitlab_put_request(super::super::http::http_client()?, &url, Some(token))
            .json(&json!({ "resolved": request.resolved })),
    )
    .await?;
    Ok(ReviewPlatformActionResult {
        success: true,
        message: if request.resolved {
            format!("Resolved {} discussion", label)
        } else {
            format!("Reopened {} discussion", label)
        },
        web_url: None,
        pull_request: None,
        thread: None,
    })
}

pub(crate) async fn gitlab_approve_pull_request(
    ctx: &ProviderContext,
    request: &ReviewPlatformApprovalRequest,
    label: &str,
) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
    let token = require_write_token(ctx, &format!("Approving a {}", label))?;
    let project = urlencoding::encode(&ctx.remote.project_path);
    let url = format!(
        "{}/projects/{}/merge_requests/{}/approve",
        ctx.api_base_url, project, request.pull_request_id
    );
    send_json(gitlab_post_request(
        super::super::http::http_client()?,
        &url,
        Some(token),
    ))
    .await?;
    if let Some(body) = request.body.as_deref().filter(|value| !value.trim().is_empty()) {
        let _ = gitlab_add_merge_request_note(ctx, &request.pull_request_id, body, "Added approval note").await;
    }
    Ok(ReviewPlatformActionResult {
        success: true,
        message: format!("Approved {}", label),
        web_url: None,
        pull_request: None,
        thread: None,
    })
}

pub(crate) async fn gitlab_revoke_approval(
    ctx: &ProviderContext,
    request: &ReviewPlatformApprovalRequest,
    label: &str,
) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
    let token = require_write_token(ctx, &format!("Revoking approval for a {}", label))?;
    let project = urlencoding::encode(&ctx.remote.project_path);
    let url = format!(
        "{}/projects/{}/merge_requests/{}/unapprove",
        ctx.api_base_url, project, request.pull_request_id
    );
    send_json(gitlab_post_request(
        super::super::http::http_client()?,
        &url,
        Some(token),
    ))
    .await?;
    Ok(ReviewPlatformActionResult {
        success: true,
        message: format!("Revoked approval for {}", label),
        web_url: None,
        pull_request: None,
        thread: None,
    })
}

pub(crate) fn gitlab_next_page(headers: &HeaderMap, _current_page: u32) -> Option<u32> {
    header_string(headers, "x-next-page").and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            trimmed.parse::<u32>().ok()
        }
    })
}

pub(crate) async fn enrich_gitlab_pull_request_counts(
    ctx: &ProviderContext,
    pull_requests: Vec<ReviewPlatformPullRequest>,
) -> Vec<ReviewPlatformPullRequest> {
    let Ok(client) = super::super::http::http_client() else {
        return pull_requests;
    };
    let project = urlencoding::encode(&ctx.remote.project_path).to_string();
    let futures = pull_requests.into_iter().map(|mut pull_request| {
        let client = client.clone();
        let url = format!(
            "{}/projects/{}/merge_requests/{}/changes",
            ctx.api_base_url, project, pull_request.id
        );
        let token = ctx.token.clone();
        async move {
            if let Ok(value) = send_json(gitlab_request(client, &url, token.as_deref())).await {
                let files = gitlab_files(&value);
                apply_files_stats(&mut pull_request, &files);
            }
            pull_request
        }
    });
    stream::iter(futures)
        .buffered(super::super::types::PROVIDER_ENRICH_CONCURRENCY)
        .collect()
        .await
}

pub(crate) fn gitlab_request(client: reqwest::Client, url: &str, token: Option<&str>) -> reqwest::RequestBuilder {
    let mut request = client
        .get(url)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .header(ACCEPT, "application/json");
    if let Some(token) = token {
        request = request.header("PRIVATE-TOKEN", token);
    }
    request
}

pub(crate) fn gitlab_post_request(client: reqwest::Client, url: &str, token: Option<&str>) -> reqwest::RequestBuilder {
    let mut request = client
        .post(url)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .header(ACCEPT, "application/json");
    if let Some(token) = token {
        request = request.header("PRIVATE-TOKEN", token);
    }
    request
}

pub(crate) fn gitlab_put_request(client: reqwest::Client, url: &str, token: Option<&str>) -> reqwest::RequestBuilder {
    let mut request = client
        .put(url)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .header(ACCEPT, "application/json");
    if let Some(token) = token {
        request = request.header("PRIVATE-TOKEN", token);
    }
    request
}

pub(crate) use super::gitlab_dto::{
    gitlab_commit_from_value, gitlab_files, gitlab_pull_request_from_value, gitlab_thread_from_note, gitlab_threads,
};
