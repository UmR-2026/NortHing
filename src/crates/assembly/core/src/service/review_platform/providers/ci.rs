//! CI helpers shared across providers — generic outcome parsing, log
//! truncation/excerpt, and provider-specific CI adapters.

use super::super::http::{fetch_paginated_array, send_json, send_text};
use super::super::types::{
    ProviderContext, ReviewChecks, ReviewPlatformCiItem, ReviewPlatformCiLog, ReviewPlatformError,
};
use super::util::{array_items, empty_checks, first_non_empty, nested_optional_string, optional_string, value_string};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiOutcome {
    Passed,
    Failed,
    Pending,
}

pub fn summarize_ci_items(items: &[ReviewPlatformCiItem]) -> ReviewChecks {
    let mut checks = empty_checks();
    for item in items {
        match ci_item_outcome(item) {
            CiOutcome::Passed => checks.passed += 1,
            CiOutcome::Failed => checks.failed += 1,
            CiOutcome::Pending => checks.pending += 1,
        }
    }
    checks.total = checks.passed + checks.failed + checks.pending;
    checks
}

pub fn ci_item_outcome(item: &ReviewPlatformCiItem) -> CiOutcome {
    let status = item.status.trim().to_ascii_lowercase();
    let conclusion = item.conclusion.as_deref().unwrap_or("").trim().to_ascii_lowercase();

    if conclusion.is_empty() {
        return ci_status_outcome(&status);
    }

    match conclusion.as_str() {
        "success" | "neutral" | "skipped" | "passed" => CiOutcome::Passed,
        "failure" | "timed_out" | "timed-out" | "cancelled" | "canceled" | "action_required" | "error" => {
            CiOutcome::Failed
        }
        "queued"
        | "pending"
        | "running"
        | "in_progress"
        | "in progress"
        | "created"
        | "manual"
        | "scheduled"
        | "waiting_for_resource"
        | "preparing"
        | "requested" => CiOutcome::Pending,
        _ => ci_status_outcome(&status),
    }
}

pub(super) fn ci_status_outcome(status: &str) -> CiOutcome {
    match status.trim().to_ascii_lowercase().as_str() {
        "success" | "passed" | "pass" | "skipped" | "ok" | "available" | "can_be_merged" | "mergeable" | "true"
        | "enabled" | "active" => CiOutcome::Passed,
        "failure" | "failed" | "fail" | "error" | "cancelled" | "canceled" | "cannot_be_merged" | "conflict"
        | "blocked" | "false" | "disabled" | "inactive" => CiOutcome::Failed,
        "pending"
        | "queued"
        | "running"
        | "in_progress"
        | "in progress"
        | "created"
        | "manual"
        | "scheduled"
        | "waiting_for_resource"
        | "preparing"
        | "requested"
        | "checking"
        | "unchecked"
        | "completed" => CiOutcome::Pending,
        _ => CiOutcome::Pending,
    }
}

pub fn ci_log_value(text: String) -> (Option<String>, bool) {
    let extracted = ci_error_excerpt(&text);
    let Some(excerpt) = extracted else {
        return (None, false);
    };
    let char_count = excerpt.chars().count();
    if char_count <= super::super::types::MAX_CI_LOG_CHARS {
        return (Some(excerpt), false);
    }

    (
        Some(format!(
            "[Error excerpt truncated: showing first {} of {} chars]\n{}",
            super::super::types::MAX_CI_LOG_CHARS,
            char_count,
            excerpt
                .chars()
                .take(super::super::types::MAX_CI_LOG_CHARS)
                .collect::<String>()
        )),
        true,
    )
}

pub(super) fn empty_ci_log() -> (Option<String>, bool) {
    (None, false)
}

pub(super) fn ci_error_excerpt(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return None;
    }

    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !is_ci_error_line(line) {
            continue;
        }

        let start = index.saturating_sub(2);
        let mut end = (index + 6).min(lines.len());
        while end < lines.len() && lines[end].trim().is_empty() {
            end += 1;
        }
        ranges.push((start, end));
    }

    if ranges.is_empty() {
        return None;
    }

    ranges.sort_unstable_by_key(|range| range.0);
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in ranges {
        if let Some(last) = merged.last_mut() {
            if start <= last.1.saturating_add(1) {
                last.1 = last.1.max(end);
                continue;
            }
        }
        merged.push((start, end));
    }

    let mut output = String::new();
    for (index, (start, end)) in merged.iter().enumerate() {
        if index > 0 {
            output.push_str("\n...\n");
        }
        for line in &lines[*start..*end] {
            output.push_str(line);
            output.push('\n');
        }
    }

    let output = output.trim_end_matches('\n').trim().to_string();
    if output.is_empty() {
        None
    } else {
        Some(output)
    }
}

pub(super) fn is_ci_error_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("##[error]")
        || lower.contains("error:")
        || lower.contains(" failed")
        || lower.contains("failure")
        || lower.contains("fatal")
        || lower.contains("exception")
        || lower.contains("traceback")
        || lower.contains("panic")
        || lower.contains("assertion failed")
        || lower.contains("command failed")
        || lower.contains("exited with code")
        || lower.contains("return code")
        || lower.contains("build failed")
        || lower.contains("test failed")
}

pub(super) async fn github_checks_and_ci(
    ctx: &ProviderContext,
    client: &reqwest::Client,
    pull_detail: &Value,
) -> (ReviewChecks, Vec<ReviewPlatformCiItem>) {
    let sha = super::util::nested_string(pull_detail, &["head", "sha"]);
    if sha.trim().is_empty() {
        return (empty_checks(), Vec::new());
    }

    let mut ci_items = Vec::new();
    let status_url = format!(
        "{}/repos/{}/{}/commits/{}/status",
        ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, sha
    );
    if let Ok(status) = send_json(super::github::github_request(
        client.clone(),
        &status_url,
        ctx.token.as_deref(),
    ))
    .await
    {
        let statuses = status
            .get("statuses")
            .and_then(Value::as_array)
            .map(|items| items.as_slice())
            .unwrap_or(&[]);
        for (index, item) in statuses.iter().enumerate() {
            ci_items.push(ReviewPlatformCiItem {
                id: format!(
                    "status-{}",
                    first_non_empty(&[value_string(item, "id"), index.to_string()])
                ),
                name: first_non_empty(&[
                    value_string(item, "context"),
                    value_string(item, "description"),
                    "Status".to_string(),
                ]),
                status: value_string(item, "state"),
                conclusion: None,
                detail: optional_string(item, "description"),
                stage: None,
                web_url: optional_string(item, "target_url"),
                log: None,
                log_truncated: false,
                started_at: None,
                finished_at: None,
            });
        }
    }

    let check_runs_url = format!(
        "{}/repos/{}/{}/commits/{}/check-runs",
        ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, sha
    );
    if let Ok(check_runs) = send_json(
        super::github::github_request(client.clone(), &check_runs_url, ctx.token.as_deref())
            .query(&[("per_page", "100")]),
    )
    .await
    {
        for (index, item) in check_runs
            .get("check_runs")
            .and_then(Value::as_array)
            .map(|items| items.as_slice())
            .unwrap_or(&[])
            .iter()
            .enumerate()
        {
            ci_items.push(ReviewPlatformCiItem {
                id: format!(
                    "check-run-{}",
                    first_non_empty(&[value_string(item, "id"), index.to_string()])
                ),
                name: first_non_empty(&[value_string(item, "name"), "Check run".to_string()]),
                status: value_string(item, "status"),
                conclusion: optional_string(item, "conclusion"),
                detail: nested_optional_string(item, &["output", "summary"])
                    .or_else(|| nested_optional_string(item, &["output", "text"]))
                    .or_else(|| optional_string(item, "details_url")),
                stage: None,
                web_url: optional_string(item, "html_url").or_else(|| optional_string(item, "details_url")),
                log: None,
                log_truncated: false,
                started_at: optional_string(item, "started_at"),
                finished_at: optional_string(item, "completed_at"),
            });
        }
    }

    let checks = summarize_ci_items(&ci_items);
    (checks, ci_items)
}

pub(super) async fn github_actions_jobs_for_head_sha(
    ctx: &ProviderContext,
    client: &reqwest::Client,
    sha: &str,
) -> Vec<Value> {
    let runs_url = format!(
        "{}/repos/{}/{}/actions/runs",
        ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name
    );
    let runs = match send_json(
        super::github::github_request(client.clone(), &runs_url, ctx.token.as_deref())
            .query(&[("head_sha", sha), ("per_page", "100")]),
    )
    .await
    {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };

    let mut jobs = Vec::new();
    for run in runs
        .get("workflow_runs")
        .and_then(Value::as_array)
        .map(|items| items.as_slice())
        .unwrap_or(&[])
    {
        let run_id = value_string(run, "id");
        if run_id.trim().is_empty() {
            continue;
        }
        let jobs_url = format!(
            "{}/repos/{}/{}/actions/runs/{}/jobs",
            ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, run_id
        );
        if let Ok(value) = send_json(
            super::github::github_request(client.clone(), &jobs_url, ctx.token.as_deref())
                .query(&[("per_page", "100")]),
        )
        .await
        {
            jobs.extend(
                value
                    .get("jobs")
                    .and_then(Value::as_array)
                    .map(|items| items.as_slice())
                    .unwrap_or(&[])
                    .iter()
                    .cloned(),
            );
        }
    }

    jobs
}

pub(super) async fn github_actions_log_for_check_run_item(
    ctx: &ProviderContext,
    client: &reqwest::Client,
    check_run_id: &str,
    check_run_name: &str,
    head_sha: &str,
) -> Result<ReviewPlatformCiLog, ReviewPlatformError> {
    let action_jobs = github_actions_jobs_for_head_sha(ctx, client, head_sha).await;
    let check_run = action_jobs
        .iter()
        .find(|job| {
            let check_run_url = value_string(job, "check_run_url");
            check_run_url.ends_with(&format!("/check-runs/{}", check_run_id))
                || value_string(job, "name") == check_run_name
        })
        .cloned();

    let Some(job) = check_run else {
        return Ok(ReviewPlatformCiLog {
            ci_item_id: format!("check-run-{}", check_run_id),
            log: None,
            truncated: false,
            message: Some("No matching GitHub Actions job was found for this check run.".to_string()),
        });
    };

    let job_id = value_string(&job, "id");
    if job_id.trim().is_empty() {
        return Ok(ReviewPlatformCiLog {
            ci_item_id: format!("check-run-{}", check_run_id),
            log: None,
            truncated: false,
            message: Some("The matching GitHub Actions job does not expose a job id.".to_string()),
        });
    }

    let logs_url = format!(
        "{}/repos/{}/{}/actions/jobs/{}/logs",
        ctx.api_base_url, ctx.remote.owner, ctx.remote.repository_name, job_id
    );
    let text = send_text(super::github::github_request(
        client.clone(),
        &logs_url,
        ctx.token.as_deref(),
    ))
    .await?;
    let (log, truncated) = ci_log_value(text);
    let message = log
        .as_ref()
        .is_none()
        .then_some("No error lines were detected in the GitHub Actions job log.".to_string());
    Ok(ReviewPlatformCiLog {
        ci_item_id: format!("check-run-{}", check_run_id),
        log,
        truncated,
        message,
    })
}

pub(super) fn gitlab_pipeline_summary_item(detail: &Value) -> Option<ReviewPlatformCiItem> {
    let pipeline = detail.get("head_pipeline")?;
    let status = value_string(pipeline, "status");
    if status.trim().is_empty() {
        return None;
    }
    Some(ReviewPlatformCiItem {
        id: first_non_empty(&[
            value_string(pipeline, "id"),
            value_string(pipeline, "iid"),
            "head-pipeline".to_string(),
        ]),
        name: "Pipeline".to_string(),
        status,
        conclusion: None,
        detail: nested_optional_string(pipeline, &["detailed_status", "text"])
            .or_else(|| nested_optional_string(pipeline, &["detailed_status", "label"])),
        stage: None,
        web_url: optional_string(pipeline, "web_url"),
        log: None,
        log_truncated: false,
        started_at: optional_string(pipeline, "started_at"),
        finished_at: optional_string(pipeline, "finished_at"),
    })
}

pub(super) async fn gitlab_pipeline_jobs(
    ctx: &ProviderContext,
    client: reqwest::Client,
    project: &str,
    pipeline_id: &str,
) -> Vec<ReviewPlatformCiItem> {
    let jobs_url = format!(
        "{}/projects/{}/pipelines/{}/jobs",
        ctx.api_base_url, project, pipeline_id
    );
    if let Ok(response) = fetch_paginated_array(
        |page| {
            let page = page.to_string();
            super::gitlab::gitlab_request(client.clone(), &jobs_url, ctx.token.as_deref())
                .query(&[("per_page", "100"), ("page", &page)])
        },
        super::gitlab::gitlab_next_page,
    )
    .await
    {
        let mut jobs = Vec::new();
        for (index, job) in array_items(&response).iter().enumerate() {
            let provider_id = value_string(job, "id");
            let id = first_non_empty(&[provider_id.clone(), index.to_string()]);
            jobs.push(ReviewPlatformCiItem {
                id,
                name: first_non_empty(&[value_string(job, "name"), "Job".to_string()]),
                status: value_string(job, "status"),
                conclusion: None,
                detail: optional_string(job, "failure_reason"),
                stage: optional_string(job, "stage"),
                web_url: optional_string(job, "web_url"),
                log: None,
                log_truncated: false,
                started_at: optional_string(job, "started_at"),
                finished_at: optional_string(job, "finished_at"),
            });
        }
        return jobs;
    }
    Vec::new()
}

pub(super) async fn gitlab_job_trace(
    ctx: &ProviderContext,
    client: reqwest::Client,
    project: &str,
    job_id: &str,
) -> (Option<String>, bool) {
    if job_id.trim().is_empty() {
        return empty_ci_log();
    }
    let trace_url = format!("{}/projects/{}/jobs/{}/trace", ctx.api_base_url, project, job_id);
    match send_text(super::gitlab::gitlab_request(client, &trace_url, ctx.token.as_deref())).await {
        Ok(text) => ci_log_value(text),
        Err(_) => empty_ci_log(),
    }
}

pub(super) async fn gitlab_pull_request_ci_log(
    ctx: &ProviderContext,
    _pull_request_id: &str,
    ci_item_id: &str,
    _ci_item_name: &str,
) -> Result<ReviewPlatformCiLog, ReviewPlatformError> {
    if ci_item_id == "head-pipeline" || ci_item_id == "pipeline" {
        return Ok(ReviewPlatformCiLog {
            ci_item_id: ci_item_id.to_string(),
            log: None,
            truncated: false,
            message: Some("Pipeline summaries do not expose a separate job trace.".to_string()),
        });
    }

    let client = super::super::http::http_client()?;
    let project = urlencoding::encode(&ctx.remote.project_path).to_string();
    let (log, truncated) = gitlab_job_trace(ctx, client, &project, ci_item_id).await;
    let message = log
        .as_ref()
        .is_none()
        .then_some("No error lines were detected in the job trace.".to_string());
    Ok(ReviewPlatformCiLog {
        ci_item_id: ci_item_id.to_string(),
        log,
        truncated,
        message,
    })
}

pub(super) fn gitcode_ci_items(detail: &Value) -> Vec<ReviewPlatformCiItem> {
    let mut items = Vec::new();
    let pipeline_status = first_non_empty(&[
        value_string(detail, "pipeline_status"),
        value_string(detail, "pipeline_status_with_code_quality"),
    ]);
    if !pipeline_status.trim().is_empty() {
        items.push(ReviewPlatformCiItem {
            id: first_non_empty(&[value_string(detail, "head_pipeline_id"), "pipeline".to_string()]),
            name: "Pipeline".to_string(),
            status: pipeline_status,
            conclusion: None,
            detail: optional_string(detail, "pipeline_status_with_code_quality"),
            stage: None,
            web_url: optional_string(detail, "web_url").or_else(|| optional_string(detail, "html_url")),
            log: None,
            log_truncated: false,
            started_at: None,
            finished_at: None,
        });
    }

    let codequality_status = value_string(detail, "codequality_status");
    if !codequality_status.trim().is_empty() {
        items.push(ReviewPlatformCiItem {
            id: first_non_empty(&[
                format!("{}-codequality", value_string(detail, "head_pipeline_id")),
                "codequality".to_string(),
            ]),
            name: "Code quality".to_string(),
            status: codequality_status,
            conclusion: None,
            detail: None,
            stage: None,
            web_url: optional_string(detail, "web_url").or_else(|| optional_string(detail, "html_url")),
            log: None,
            log_truncated: false,
            started_at: None,
            finished_at: None,
        });
    }

    items
}
