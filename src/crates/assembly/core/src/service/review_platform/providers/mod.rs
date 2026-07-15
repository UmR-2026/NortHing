//! Provider abstractions and concrete implementations for GitHub, GitLab, and
//! GitCode. Sibling modules keep their own helpers; cross-provider shared
//! helpers live in [`ci`] and [`util`].

use crate::service::review_platform::auth::platform_label;
use crate::service::review_platform::types::{
    ProviderContext, PullRequestPagination, ReviewPlatformActionResult, ReviewPlatformApprovalRequest,
    ReviewPlatformCiLog, ReviewPlatformCreatePullRequestRequest, ReviewPlatformDetailSection, ReviewPlatformError,
    ReviewPlatformPullRequestDetail, ReviewPlatformPullRequestDetailPage, ReviewPlatformPullRequestPage,
    ReviewPlatformReplyToThreadRequest, ReviewPlatformRequestChangesRequest, ReviewPlatformResolveThreadRequest,
    ReviewPlatformSubmitReviewRequest,
};

pub mod ci;
pub mod gitcode;
pub mod github;
pub mod gitlab;
pub mod gitlab_dto;
pub mod util;

pub(crate) struct UnsupportedProvider;

#[async_trait::async_trait]
impl ReviewProvider for UnsupportedProvider {
    async fn list_pull_requests(
        &self,
        ctx: &ProviderContext,
        _pagination: PullRequestPagination,
    ) -> Result<ReviewPlatformPullRequestPage, ReviewPlatformError> {
        Err(ReviewPlatformError::UnsupportedPlatform(ctx.remote.host.clone()))
    }

    async fn pull_request_detail(
        &self,
        ctx: &ProviderContext,
        _pull_request_id: &str,
    ) -> Result<ReviewPlatformPullRequestDetail, ReviewPlatformError> {
        Err(ReviewPlatformError::UnsupportedPlatform(ctx.remote.host.clone()))
    }
}

#[async_trait::async_trait]
pub trait ReviewProvider: Sync {
    async fn list_pull_requests(
        &self,
        ctx: &ProviderContext,
        pagination: PullRequestPagination,
    ) -> Result<ReviewPlatformPullRequestPage, ReviewPlatformError>;

    async fn pull_request_detail(
        &self,
        ctx: &ProviderContext,
        pull_request_id: &str,
    ) -> Result<ReviewPlatformPullRequestDetail, ReviewPlatformError>;

    async fn pull_request_detail_page(
        &self,
        ctx: &ProviderContext,
        pull_request_id: &str,
        section: ReviewPlatformDetailSection,
        pagination: PullRequestPagination,
    ) -> Result<ReviewPlatformPullRequestDetailPage, ReviewPlatformError> {
        let detail = self.pull_request_detail(ctx, pull_request_id).await?;
        let ci_total = detail.ci.len();
        let file_total = detail.files.len();
        let commit_total = detail.commits.len();
        let thread_total = detail.threads.len();
        let (ci, files, commits, threads) = match section {
            ReviewPlatformDetailSection::Overview => (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
            ReviewPlatformDetailSection::Ci => (
                crate::service::review_platform::http::slice_page(detail.ci, pagination),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            ReviewPlatformDetailSection::Files => (
                Vec::new(),
                crate::service::review_platform::http::slice_page(detail.files, pagination),
                Vec::new(),
                Vec::new(),
            ),
            ReviewPlatformDetailSection::Commits => (
                Vec::new(),
                Vec::new(),
                crate::service::review_platform::http::slice_page(detail.commits, pagination),
                Vec::new(),
            ),
            ReviewPlatformDetailSection::Reviews => (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                crate::service::review_platform::http::slice_page(detail.threads, pagination),
            ),
        };
        let total = match section {
            ReviewPlatformDetailSection::Overview => 0,
            ReviewPlatformDetailSection::Ci => ci_total,
            ReviewPlatformDetailSection::Files => file_total,
            ReviewPlatformDetailSection::Commits => commit_total,
            ReviewPlatformDetailSection::Reviews => thread_total,
        };
        Ok(ReviewPlatformPullRequestDetailPage {
            pull_request: detail.pull_request,
            body: detail.body,
            ci,
            files,
            commits,
            threads,
            section,
            pagination: crate::service::review_platform::http::pagination_from_total(pagination, total),
        })
    }

    async fn pull_request_ci_log(
        &self,
        ctx: &ProviderContext,
        _pull_request_id: &str,
        _ci_item_id: &str,
        _ci_item_name: &str,
    ) -> Result<ReviewPlatformCiLog, ReviewPlatformError> {
        Err(ReviewPlatformError::UnsupportedPlatform(format!(
            "{} CI logs",
            platform_label(ctx.remote.platform)
        )))
    }

    async fn create_pull_request(
        &self,
        ctx: &ProviderContext,
        _request: &ReviewPlatformCreatePullRequestRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        Err(ReviewPlatformError::UnsupportedPlatform(format!(
            "{} pull request creation",
            platform_label(ctx.remote.platform)
        )))
    }

    async fn reply_to_thread(
        &self,
        ctx: &ProviderContext,
        _request: &ReviewPlatformReplyToThreadRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        Err(ReviewPlatformError::UnsupportedPlatform(format!(
            "{} thread replies",
            platform_label(ctx.remote.platform)
        )))
    }

    async fn submit_review(
        &self,
        ctx: &ProviderContext,
        _request: &ReviewPlatformSubmitReviewRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        Err(ReviewPlatformError::UnsupportedPlatform(format!(
            "{} review submission",
            platform_label(ctx.remote.platform)
        )))
    }

    async fn resolve_thread(
        &self,
        ctx: &ProviderContext,
        _request: &ReviewPlatformResolveThreadRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        Err(ReviewPlatformError::UnsupportedPlatform(format!(
            "{} thread resolution",
            platform_label(ctx.remote.platform)
        )))
    }

    async fn approve_pull_request(
        &self,
        ctx: &ProviderContext,
        _request: &ReviewPlatformApprovalRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        Err(ReviewPlatformError::UnsupportedPlatform(format!(
            "{} pull request approval",
            platform_label(ctx.remote.platform)
        )))
    }

    async fn revoke_approval(
        &self,
        ctx: &ProviderContext,
        _request: &ReviewPlatformApprovalRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        Err(ReviewPlatformError::UnsupportedPlatform(format!(
            "{} approval revocation",
            platform_label(ctx.remote.platform)
        )))
    }

    async fn request_changes(
        &self,
        ctx: &ProviderContext,
        _request: &ReviewPlatformRequestChangesRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        Err(ReviewPlatformError::UnsupportedPlatform(format!(
            "{} native change requests",
            platform_label(ctx.remote.platform)
        )))
    }
}
