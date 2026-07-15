//! HTTP client, pagination, and header parsing helpers shared by the
//! provider implementations.

use crate::service::review_platform::types::{PullRequestPagination, ReviewPlatformError, ReviewPlatformPagination};
use reqwest::header::HeaderMap;
use serde_json::Value;
use std::time::Duration;

pub(super) fn http_client() -> Result<reqwest::Client, ReviewPlatformError> {
    reqwest::Client::builder()
        .use_native_tls()
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|error| ReviewPlatformError::Network(error.to_string()))
}

pub(super) struct JsonResponse {
    pub(super) value: Value,
    pub(super) headers: HeaderMap,
}

pub(super) async fn send_json(request: reqwest::RequestBuilder) -> Result<Value, ReviewPlatformError> {
    send_json_response(request).await.map(|response| response.value)
}

pub(super) async fn send_json_response(request: reqwest::RequestBuilder) -> Result<JsonResponse, ReviewPlatformError> {
    let response = request
        .send()
        .await
        .map_err(|error| ReviewPlatformError::Network(error.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let preview = body.chars().take(280).collect::<String>();
        return Err(ReviewPlatformError::Http {
            status: status.as_u16(),
            message: preview,
        });
    }
    let headers = response.headers().clone();
    let value = response
        .json::<Value>()
        .await
        .map_err(|error| ReviewPlatformError::Parse(error.to_string()))?;
    Ok(JsonResponse { value, headers })
}

pub(super) async fn send_text(request: reqwest::RequestBuilder) -> Result<String, ReviewPlatformError> {
    let response = request
        .send()
        .await
        .map_err(|error| ReviewPlatformError::Network(error.to_string()))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| ReviewPlatformError::Network(error.to_string()))?;
    if !status.is_success() {
        let preview = text.chars().take(280).collect::<String>();
        return Err(ReviewPlatformError::Http {
            status: status.as_u16(),
            message: preview,
        });
    }
    Ok(text)
}

pub(super) async fn fetch_paginated_array<F>(
    mut build_request: F,
    next_page: fn(&HeaderMap, u32) -> Option<u32>,
) -> Result<Value, ReviewPlatformError>
where
    F: FnMut(u32) -> reqwest::RequestBuilder,
{
    let mut page = 1;
    let mut values = Vec::new();

    loop {
        let response = send_json_response(build_request(page)).await?;
        let items = response
            .value
            .as_array()
            .ok_or_else(|| ReviewPlatformError::Parse("Provider paginated response was not an array".to_string()))?;
        values.extend(items.iter().cloned());

        let Some(next) = next_page(&response.headers, page).filter(|next| *next > page) else {
            break;
        };
        page = next;
    }

    Ok(Value::Array(values))
}

pub(super) async fn fetch_array_page(
    request: reqwest::RequestBuilder,
    pagination: PullRequestPagination,
) -> Result<JsonResponse, ReviewPlatformError> {
    let page = pagination.page.to_string();
    let per_page = pagination.per_page.to_string();
    let response = send_json_response(request.query(&[("per_page", &per_page), ("page", &page)])).await?;
    response
        .value
        .as_array()
        .ok_or_else(|| ReviewPlatformError::Parse("Provider paginated response was not an array".to_string()))?;
    Ok(response)
}

pub(super) fn pagination_from_response(
    response: &JsonResponse,
    pagination: PullRequestPagination,
) -> ReviewPlatformPagination {
    let item_count = response.value.as_array().map(Vec::len).unwrap_or(0);
    let total = header_u64(&response.headers, "x-total")
        .or_else(|| pagination_total_from_links(&response.headers, pagination, item_count));
    ReviewPlatformPagination {
        page: pagination.page,
        per_page: pagination.per_page,
        total,
        has_next: link_header_has_rel(&response.headers, "next")
            || header_string(&response.headers, "x-next-page").is_some_and(|value| !value.trim().is_empty())
            || total
                .map(|total| u64::from(pagination.page) * u64::from(pagination.per_page) < total)
                .unwrap_or(false),
    }
}

pub(super) fn combine_page_pagination(
    pagination: PullRequestPagination,
    pages: &[ReviewPlatformPagination],
) -> ReviewPlatformPagination {
    let totals = if pages.iter().any(|page| page.has_next) {
        None
    } else {
        pages
            .iter()
            .map(|page| page.total)
            .collect::<Option<Vec<_>>>()
            .map(|values| values.into_iter().sum())
    };
    ReviewPlatformPagination {
        page: pagination.page,
        per_page: pagination.per_page,
        total: totals,
        has_next: pages.iter().any(|page| page.has_next),
    }
}

pub(super) fn header_string(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

pub(super) fn header_u64(headers: &HeaderMap, name: &str) -> Option<u64> {
    header_string(headers, name).and_then(|value| value.parse::<u64>().ok())
}

pub(super) fn link_header_has_rel(headers: &HeaderMap, rel: &str) -> bool {
    header_string(headers, "link")
        .as_deref()
        .is_some_and(|value| value.split(',').any(|part| part.contains(&format!("rel=\"{}\"", rel))))
}

pub(super) fn link_header_last_page(headers: &HeaderMap) -> Option<u32> {
    let link = header_string(headers, "link")?;
    for part in link.split(',') {
        if !part.contains("rel=\"last\"") {
            continue;
        }
        let url = part
            .split(';')
            .next()?
            .trim()
            .trim_start_matches('<')
            .trim_end_matches('>');
        return query_param_u32(url, "page");
    }
    None
}

pub(super) fn pagination_total_from_links(
    headers: &HeaderMap,
    pagination: PullRequestPagination,
    item_count: usize,
) -> Option<u64> {
    if let Some(last_page) = link_header_last_page(headers) {
        if pagination.per_page == 1 {
            return Some(u64::from(last_page));
        }
        if last_page == pagination.page {
            return Some(
                u64::from(pagination.page.saturating_sub(1)) * u64::from(pagination.per_page) + item_count as u64,
            );
        }
        return None;
    }

    Some(u64::from(pagination.page.saturating_sub(1)) * u64::from(pagination.per_page) + item_count as u64)
}

pub(super) fn pagination_from_total(pagination: PullRequestPagination, total: usize) -> ReviewPlatformPagination {
    ReviewPlatformPagination {
        page: pagination.page,
        per_page: pagination.per_page,
        total: Some(total as u64),
        has_next: usize::try_from(pagination.page)
            .ok()
            .is_some_and(|page| page * (pagination.per_page as usize) < total),
    }
}

pub(super) fn slice_page<T>(items: Vec<T>, pagination: PullRequestPagination) -> Vec<T> {
    let start = pagination.page.saturating_sub(1).saturating_mul(pagination.per_page) as usize;
    items
        .into_iter()
        .skip(start)
        .take(pagination.per_page as usize)
        .collect()
}

pub(super) fn empty_detail_pagination(
    section: super::types::ReviewPlatformDetailSection,
    pagination: PullRequestPagination,
) -> ReviewPlatformPagination {
    ReviewPlatformPagination {
        page: pagination.page,
        per_page: pagination.per_page,
        total: if section == super::types::ReviewPlatformDetailSection::Overview {
            Some(0)
        } else {
            None
        },
        has_next: false,
    }
}

pub(super) fn query_param_u32(url: &str, name: &str) -> Option<u32> {
    let query = url.split_once('?')?.1;
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            if key == name {
                return value.parse::<u32>().ok();
            }
        }
    }
    None
}
