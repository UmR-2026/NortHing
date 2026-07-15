//! Write-side action handlers for `ReviewPlatformTool`.
//!
//! Owns create, reply, submit_review, approve, revoke, request_changes, and
//! resolve actions.

use crate::agentic::tools::framework::ToolResult;
use crate::service::review_platform::{
    ReviewPlatformApprovalRequest, ReviewPlatformCreatePullRequestRequest, ReviewPlatformReplyToThreadRequest,
    ReviewPlatformRequestChangesRequest, ReviewPlatformResolveThreadRequest, ReviewPlatformService,
    ReviewPlatformSubmitReviewRequest,
};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

impl super::ReviewPlatformTool {
    pub(super) async fn handle_create(&self, repository_path: &str, input: &Value) -> NortHingResult<Value> {
        let remote_id = Self::resolve_remote_id(repository_path, input).await?;
        let request = ReviewPlatformCreatePullRequestRequest {
            repository_path: repository_path.to_string(),
            remote_id: Some(remote_id),
            title: Self::string_field(input, "title")?,
            source_branch: Self::string_field(input, "source_branch")?,
            target_branch: Self::string_field(input, "target_branch")?,
            body: Self::optional_string_field(input, "body"),
            draft: input.get("draft").and_then(Value::as_bool),
        };
        let result = ReviewPlatformService::create_pull_request(request)
            .await
            .map_err(|error| NortHingError::tool(error.to_string()))?;
        Ok(json!({ "action": super::ACTION_CREATE, "result": result }))
    }

    pub(super) async fn handle_reply(&self, repository_path: &str, input: &Value) -> NortHingResult<Value> {
        let remote_id = Self::resolve_remote_id(repository_path, input).await?;
        let request = ReviewPlatformReplyToThreadRequest {
            repository_path: repository_path.to_string(),
            remote_id,
            pull_request_id: Self::string_field(input, "pull_request_id")?,
            thread_id: Self::string_field(input, "thread_id")?,
            body: Self::string_field(input, "body")?,
        };
        let result = ReviewPlatformService::reply_to_thread(request)
            .await
            .map_err(|error| NortHingError::tool(error.to_string()))?;
        Ok(json!({ "action": super::ACTION_REPLY, "result": result }))
    }

    pub(super) async fn handle_submit_review(&self, repository_path: &str, input: &Value) -> NortHingResult<Value> {
        let remote_id = Self::resolve_remote_id(repository_path, input).await?;
        let request = ReviewPlatformSubmitReviewRequest {
            repository_path: repository_path.to_string(),
            remote_id,
            pull_request_id: Self::string_field(input, "pull_request_id")?,
            event: Self::submit_event(input)?,
            body: Self::string_field(input, "body")?,
        };
        let result = ReviewPlatformService::submit_review(request)
            .await
            .map_err(|error| NortHingError::tool(error.to_string()))?;
        Ok(json!({ "action": super::ACTION_SUBMIT_REVIEW, "result": result }))
    }

    pub(super) async fn handle_approve(&self, repository_path: &str, input: &Value) -> NortHingResult<Value> {
        let remote_id = Self::resolve_remote_id(repository_path, input).await?;
        let request = ReviewPlatformApprovalRequest {
            repository_path: repository_path.to_string(),
            remote_id,
            pull_request_id: Self::string_field(input, "pull_request_id")?,
            body: Self::optional_string_field(input, "body"),
        };
        let result = ReviewPlatformService::approve_pull_request(request)
            .await
            .map_err(|error| NortHingError::tool(error.to_string()))?;
        Ok(json!({ "action": super::ACTION_APPROVE, "result": result }))
    }

    pub(super) async fn handle_revoke_approval(&self, repository_path: &str, input: &Value) -> NortHingResult<Value> {
        let remote_id = Self::resolve_remote_id(repository_path, input).await?;
        let request = ReviewPlatformApprovalRequest {
            repository_path: repository_path.to_string(),
            remote_id,
            pull_request_id: Self::string_field(input, "pull_request_id")?,
            body: None,
        };
        let result = ReviewPlatformService::revoke_approval(request)
            .await
            .map_err(|error| NortHingError::tool(error.to_string()))?;
        Ok(json!({ "action": super::ACTION_REVOKE_APPROVAL, "result": result }))
    }

    pub(super) async fn handle_request_changes(&self, repository_path: &str, input: &Value) -> NortHingResult<Value> {
        let remote_id = Self::resolve_remote_id(repository_path, input).await?;
        let request = ReviewPlatformRequestChangesRequest {
            repository_path: repository_path.to_string(),
            remote_id,
            pull_request_id: Self::string_field(input, "pull_request_id")?,
            body: Self::string_field(input, "body")?,
        };
        let result = ReviewPlatformService::request_changes(request)
            .await
            .map_err(|error| NortHingError::tool(error.to_string()))?;
        Ok(json!({ "action": super::ACTION_REQUEST_CHANGES, "result": result }))
    }

    pub(super) async fn handle_resolve(&self, repository_path: &str, input: &Value) -> NortHingResult<Value> {
        let remote_id = Self::resolve_remote_id(repository_path, input).await?;
        let request = ReviewPlatformResolveThreadRequest {
            repository_path: repository_path.to_string(),
            remote_id,
            pull_request_id: Self::string_field(input, "pull_request_id")?,
            thread_id: Self::string_field(input, "thread_id")?,
            resolved: input.get("resolved").and_then(Value::as_bool).unwrap_or(true),
        };
        let result = ReviewPlatformService::resolve_thread(request)
            .await
            .map_err(|error| NortHingError::tool(error.to_string()))?;
        Ok(json!({ "action": super::ACTION_RESOLVE, "result": result }))
    }
}
