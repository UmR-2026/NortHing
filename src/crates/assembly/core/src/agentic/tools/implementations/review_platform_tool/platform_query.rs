//! Query and list operations for `ReviewPlatformTool`.
//!
//! Owns remote resolution and read-side actions such as listing pull requests,
//! loading workspace snapshots, and fetching PR/CI detail.

use crate::agentic::tools::framework::ToolResult;
use crate::service::review_platform::{
    ReviewPlatformCiLog, ReviewPlatformDetailSection, ReviewPlatformError, ReviewPlatformRemote, ReviewPlatformService,
};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

impl super::ReviewPlatformTool {
    pub(super) async fn resolve_remote_id(repository_path: &str, input: &Value) -> NortHingResult<String> {
        if let Some(remote_id) = Self::optional_string_field(input, "remote_id") {
            return Ok(remote_id);
        }

        let remotes = ReviewPlatformService::discover_remotes(repository_path)
            .await
            .map_err(|error| NortHingError::tool(error.to_string()))?;
        let supported = super::supported_remotes(&remotes);
        match supported.as_slice() {
            [] => Err(NortHingError::tool(
                "No supported review platform remote found".to_string(),
            )),
            [remote] => Ok(remote.id.clone()),
            _ => Err(NortHingError::tool(super::remote_ambiguity_message(&supported))),
        }
    }

    pub(super) async fn resolve_remote_id_for_list(
        repository_path: &str,
        input: &Value,
    ) -> NortHingResult<Result<String, Value>> {
        if let Some(remote_id) = Self::optional_string_field(input, "remote_id") {
            return Ok(Ok(remote_id));
        }

        let remotes = ReviewPlatformService::discover_remotes(repository_path)
            .await
            .map_err(|error| NortHingError::tool(error.to_string()))?;
        let supported = super::supported_remotes(&remotes);
        match supported.as_slice() {
            [] => Err(NortHingError::tool(
                "No supported review platform remote found".to_string(),
            )),
            [remote] => Ok(Ok(remote.id.clone())),
            _ => Ok(Err(json!({
                "action": super::ACTION_LIST,
                "repositoryPath": repository_path,
                "status": "needs_remote_selection",
                "message": "Multiple supported review platform remotes were found. Provide remote_id explicitly.",
                "candidateRemotes": supported,
            }))),
        }
    }

    pub(super) async fn handle_list_remotes(&self, repository_path: &str) -> NortHingResult<Value> {
        let remotes = ReviewPlatformService::discover_remotes(repository_path)
            .await
            .map_err(|error| NortHingError::tool(error.to_string()))?;
        Ok(json!({
            "action": super::ACTION_LIST_REMOTES,
            "repositoryPath": repository_path,
            "remotes": remotes,
        }))
    }

    pub(super) async fn handle_workspace_snapshot(
        &self,
        repository_path: &str,
        input: &Value,
    ) -> NortHingResult<Value> {
        let page = input.get("page").and_then(Value::as_u64).map(|value| value as u32);
        let per_page = input.get("per_page").and_then(Value::as_u64).map(|value| value as u32);
        let remote_id = Self::optional_string_field(input, "remote_id");
        let snapshot = ReviewPlatformService::workspace_snapshot(repository_path, remote_id.as_deref(), page, per_page)
            .await
            .map_err(|error| NortHingError::tool(error.to_string()))?;
        let status = if snapshot.auth_challenge.is_some() {
            "needs_auth"
        } else {
            "ok"
        };
        let auth_challenge = snapshot.auth_challenge.clone();
        let selected_remote_id = snapshot.selected_remote_id.clone();
        let panel_remote_id = selected_remote_id.clone();
        Ok(json!({
            "action": super::ACTION_WORKSPACE_SNAPSHOT,
            "repositoryPath": repository_path,
            "remoteId": selected_remote_id,
            "status": status,
            "authChallenge": auth_challenge,
            "snapshot": snapshot,
            "openPanel": if status == "needs_auth" {
                json!({
                    "type": "review-platform-auth",
                    "workspacePath": repository_path,
                    "remoteId": panel_remote_id,
                })
            } else {
                Value::Null
            },
        }))
    }

    pub(super) async fn handle_count(&self, repository_path: &str, input: &Value) -> NortHingResult<Value> {
        let remote_id = match Self::resolve_remote_id_for_list(repository_path, input).await? {
            Ok(remote_id) => remote_id,
            Err(mut selection_result) => {
                if let Some(obj) = selection_result.as_object_mut() {
                    obj.insert("action".to_string(), json!(super::ACTION_COUNT));
                }
                return Ok(selection_result);
            }
        };
        let snapshot =
            ReviewPlatformService::workspace_snapshot(repository_path, Some(remote_id.as_str()), Some(1), Some(1))
                .await
                .map_err(|error| NortHingError::tool(error.to_string()))?;
        if snapshot.auth_challenge.is_some() {
            Ok(json!({
                "action": super::ACTION_COUNT,
                "repositoryPath": repository_path,
                "remoteId": remote_id,
                "status": "needs_auth",
                "authChallenge": snapshot.auth_challenge,
                "snapshot": snapshot,
                "openPanel": {
                    "type": "review-platform-auth",
                    "workspacePath": repository_path,
                    "remoteId": remote_id,
                },
            }))
        } else {
            Ok(json!({
                "action": super::ACTION_COUNT,
                "repositoryPath": repository_path,
                "remoteId": remote_id,
                "total": snapshot.pagination.total,
                "hasNext": snapshot.pagination.has_next,
            }))
        }
    }

    pub(super) async fn handle_list(&self, repository_path: &str, input: &Value) -> NortHingResult<Value> {
        let page = input.get("page").and_then(Value::as_u64).map(|value| value as u32);
        let per_page = input.get("per_page").and_then(Value::as_u64).map(|value| value as u32);
        let remote_id = match Self::resolve_remote_id_for_list(repository_path, input).await? {
            Ok(remote_id) => remote_id,
            Err(selection_result) => return Ok(selection_result),
        };
        let snapshot =
            ReviewPlatformService::workspace_snapshot(repository_path, Some(remote_id.as_str()), page, per_page)
                .await
                .map_err(|error| NortHingError::tool(error.to_string()))?;
        if snapshot.auth_challenge.is_some() {
            Ok(json!({
                "action": super::ACTION_LIST,
                "repositoryPath": repository_path,
                "remoteId": remote_id,
                "status": "needs_auth",
                "authChallenge": snapshot.auth_challenge,
                "snapshot": snapshot,
                "openPanel": {
                    "type": "review-platform-auth",
                    "workspacePath": repository_path,
                    "remoteId": remote_id,
                },
            }))
        } else {
            Ok(json!({
                "action": super::ACTION_LIST,
                "repositoryPath": repository_path,
                "remoteId": remote_id,
                "snapshot": snapshot,
            }))
        }
    }

    pub(super) async fn handle_get(&self, repository_path: &str, input: &Value) -> NortHingResult<Value> {
        let pull_request_id = Self::string_field(input, "pull_request_id")?;
        let remote_id = Self::resolve_remote_id(repository_path, input).await?;
        match ReviewPlatformService::pull_request_detail(repository_path, &remote_id, &pull_request_id).await {
            Ok(detail) => Ok(json!({
                "action": super::ACTION_GET,
                "repositoryPath": repository_path,
                "remoteId": remote_id,
                "pullRequest": detail.pull_request,
                "body": detail.body,
                "ci": detail.ci,
                "files": detail.files,
                "commits": detail.commits,
                "threads": detail.threads,
            })),
            Err(error) => {
                if let Some(result) = Self::auth_required_result(super::ACTION_GET, repository_path, &remote_id, &error)
                {
                    Ok(result)
                } else {
                    Err(NortHingError::tool(error.to_string()))
                }
            }
        }
    }

    pub(super) async fn handle_get_detail_page(&self, repository_path: &str, input: &Value) -> NortHingResult<Value> {
        let pull_request_id = Self::string_field(input, "pull_request_id")?;
        let remote_id = Self::resolve_remote_id(repository_path, input).await?;
        let section = Self::detail_section(input)?;
        let page = input.get("page").and_then(Value::as_u64).map(|value| value as u32);
        let per_page = input.get("per_page").and_then(Value::as_u64).map(|value| value as u32);
        match ReviewPlatformService::pull_request_detail_page(
            repository_path,
            &remote_id,
            &pull_request_id,
            section,
            page,
            per_page,
        )
        .await
        {
            Ok(detail) => {
                let items = match section {
                    ReviewPlatformDetailSection::Overview => json!([]),
                    ReviewPlatformDetailSection::Ci => json!(detail.ci),
                    ReviewPlatformDetailSection::Files => json!(detail.files),
                    ReviewPlatformDetailSection::Commits => json!(detail.commits),
                    ReviewPlatformDetailSection::Reviews => json!(detail.threads),
                };
                Ok(json!({
                    "action": super::ACTION_GET_DETAIL_PAGE,
                    "repositoryPath": repository_path,
                    "remoteId": remote_id,
                    "pullRequest": detail.pull_request,
                    "body": detail.body,
                    "section": detail.section,
                    "pagination": detail.pagination,
                    "items": items,
                    "detailPage": detail,
                }))
            }
            Err(error) => {
                if let Some(result) =
                    Self::auth_required_result(super::ACTION_GET_DETAIL_PAGE, repository_path, &remote_id, &error)
                {
                    Ok(result)
                } else {
                    Err(NortHingError::tool(error.to_string()))
                }
            }
        }
    }

    pub(super) async fn handle_get_ci_log(&self, repository_path: &str, input: &Value) -> NortHingResult<Value> {
        let pull_request_id = Self::string_field(input, "pull_request_id")?;
        let remote_id = Self::resolve_remote_id(repository_path, input).await?;
        let ci_item_id = Self::string_field(input, "ci_item_id")?;
        let ci_item_name = Self::string_field(input, "ci_item_name")?;
        match ReviewPlatformService::pull_request_ci_log(
            repository_path,
            &remote_id,
            &pull_request_id,
            &ci_item_id,
            &ci_item_name,
        )
        .await
        {
            Ok(ci_log) => Ok(json!({
                "action": super::ACTION_GET_CI_LOG,
                "repositoryPath": repository_path,
                "remoteId": remote_id,
                "pullRequestId": pull_request_id,
                "ciItemId": ci_log.ci_item_id,
                "log": ci_log.log,
                "truncated": ci_log.truncated,
                "message": ci_log.message,
            })),
            Err(error) => {
                if let Some(result) =
                    Self::auth_required_result(super::ACTION_GET_CI_LOG, repository_path, &remote_id, &error)
                {
                    Ok(result)
                } else {
                    Err(NortHingError::tool(error.to_string()))
                }
            }
        }
    }
}
