//! Pull request / review platform tool.
//!
//! This tool exposes hosted review-platform operations to the agent while
//! keeping provider-specific HTTP behavior inside `ReviewPlatformService`.

use crate::agentic::tools::framework::{
    Tool, ToolExposure, ToolRenderOptions, ToolResult, ToolUseContext, ValidationResult,
};
use crate::service::review_platform::{
    ReviewPlatformApprovalRequest, ReviewPlatformCreatePullRequestRequest, ReviewPlatformDetailSection,
    ReviewPlatformError, ReviewPlatformKind, ReviewPlatformRemote, ReviewPlatformReplyToThreadRequest,
    ReviewPlatformRequestChangesRequest, ReviewPlatformResolveThreadRequest, ReviewPlatformService,
    ReviewPlatformSubmitReviewRequest, ReviewSubmitEvent,
};
use crate::util::errors::{NortHingError, NortHingResult};
use async_trait::async_trait;
use serde_json::{json, Value};

mod platform_action;
mod platform_format;
mod platform_init;
mod platform_query;

const ACTION_WORKSPACE_SNAPSHOT: &str = "get_workspace_snapshot";
const ACTION_LIST_REMOTES: &str = "list_remotes";
const ACTION_LIST: &str = "list_pull_requests";
const ACTION_COUNT: &str = "count_pull_requests";
const ACTION_GET: &str = "get_pull_request";
const ACTION_GET_DETAIL_PAGE: &str = "get_pull_request_detail_page";
const ACTION_GET_CI_LOG: &str = "get_pull_request_ci_log";
const ACTION_CREATE: &str = "create_pull_request";
const ACTION_REPLY: &str = "reply_to_thread";
const ACTION_SUBMIT_REVIEW: &str = "submit_review";
const ACTION_APPROVE: &str = "approve_pull_request";
const ACTION_REVOKE_APPROVAL: &str = "revoke_approval";
const ACTION_REQUEST_CHANGES: &str = "request_changes";
const ACTION_RESOLVE: &str = "resolve_thread";
const ACTION_UPDATE_AUTH_TOKEN: &str = "update_auth_token";
const ACTION_CLEAR_AUTH_TOKEN: &str = "clear_auth_token";

const WRITE_ACTIONS: &[&str] = &[
    ACTION_CREATE,
    ACTION_REPLY,
    ACTION_SUBMIT_REVIEW,
    ACTION_APPROVE,
    ACTION_REVOKE_APPROVAL,
    ACTION_REQUEST_CHANGES,
    ACTION_RESOLVE,
    ACTION_UPDATE_AUTH_TOKEN,
    ACTION_CLEAR_AUTH_TOKEN,
];

pub struct ReviewPlatformTool;

impl ReviewPlatformTool {
    pub fn new() -> Self {
        Self
    }

    fn repository_path(input: &Value, context: &ToolUseContext) -> NortHingResult<String> {
        let requested = input
            .get("repository_path")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());

        if let Some(path) = requested {
            return context.resolve_workspace_tool_path(path);
        }

        context
            .workspace
            .as_ref()
            .map(|workspace| workspace.root_path_string())
            .ok_or_else(|| NortHingError::tool("repository_path is required".to_string()))
    }

    fn string_field(input: &Value, key: &str) -> NortHingResult<String> {
        input
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .ok_or_else(|| NortHingError::tool(format!("{} is required", key)))
    }

    fn optional_string_field(input: &Value, key: &str) -> Option<String> {
        input
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    fn submit_event(input: &Value) -> NortHingResult<ReviewSubmitEvent> {
        match input.get("event").and_then(Value::as_str).unwrap_or("comment") {
            "comment" => Ok(ReviewSubmitEvent::Comment),
            "approve" => Ok(ReviewSubmitEvent::Approve),
            "request_changes" => Ok(ReviewSubmitEvent::RequestChanges),
            other => Err(NortHingError::tool(format!("Unsupported review event: {}", other))),
        }
    }

    fn detail_section(input: &Value) -> NortHingResult<ReviewPlatformDetailSection> {
        match input.get("section").and_then(Value::as_str).unwrap_or("overview") {
            "overview" => Ok(ReviewPlatformDetailSection::Overview),
            "ci" => Ok(ReviewPlatformDetailSection::Ci),
            "files" => Ok(ReviewPlatformDetailSection::Files),
            "commits" => Ok(ReviewPlatformDetailSection::Commits),
            "reviews" => Ok(ReviewPlatformDetailSection::Reviews),
            other => Err(NortHingError::tool(format!(
                "Unsupported pull request detail section: {}",
                other
            ))),
        }
    }

    fn action(input: &Value) -> Option<&str> {
        input.get("action").and_then(Value::as_str)
    }
}

#[async_trait]
impl Tool for ReviewPlatformTool {
    fn name(&self) -> &str {
        "ReviewPlatform"
    }

    async fn description(&self) -> NortHingResult<String> {
        Ok(r#"Read and operate on hosted pull requests / merge requests.

Use this for remote review-platform operations such as discovering remotes, loading the workspace PR snapshot, counting pull requests, listing pull requests, opening full or paginated pull request detail, loading CI logs, creating a pull request, replying to review threads, submitting a comment review, approving, revoking approval, requesting changes, or resolving a review thread. Use the Git tool for local repository state and branch/commit/push operations.

Authentication-token actions are available only when the user explicitly provides a token or asks to clear a stored token. Never guess or expose token values.

When returning pull request results to the user, include the provider web URL so the chat UI can open the pull request detail panel naturally."#.to_string())
    }

    fn short_description(&self) -> String {
        "Inspect and operate on hosted pull requests / merge requests.".to_string()
    }

    fn default_exposure(&self) -> ToolExposure {
        ToolExposure::Collapsed
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [
                        ACTION_WORKSPACE_SNAPSHOT,
                        ACTION_LIST_REMOTES,
                        ACTION_LIST,
                        ACTION_COUNT,
                        ACTION_GET,
                        ACTION_GET_DETAIL_PAGE,
                        ACTION_GET_CI_LOG,
                        ACTION_CREATE,
                        ACTION_REPLY,
                        ACTION_SUBMIT_REVIEW,
                        ACTION_APPROVE,
                        ACTION_REVOKE_APPROVAL,
                        ACTION_REQUEST_CHANGES,
                        ACTION_RESOLVE,
                        ACTION_UPDATE_AUTH_TOKEN,
                        ACTION_CLEAR_AUTH_TOKEN
                    ],
                    "description": "Review platform action to perform."
                },
                "repository_path": {
                    "type": "string",
                    "description": "Repository path. Omit to use the current workspace."
                },
                "remote_id": {
                    "type": "string",
                    "description": "Review platform remote id. Omit to use the only supported remote; provide it explicitly when the repository has multiple supported review-platform remotes."
                },
                "pull_request_id": {
                    "type": "string",
                    "description": "Pull request or merge request number/id."
                },
                "page": {
                    "type": "integer",
                    "description": "Page number for list_pull_requests, get_workspace_snapshot, or get_pull_request_detail_page."
                },
                "per_page": {
                    "type": "integer",
                    "description": "Page size for list_pull_requests, get_workspace_snapshot, or get_pull_request_detail_page."
                },
                "section": {
                    "type": "string",
                    "enum": ["overview", "ci", "files", "commits", "reviews"],
                    "description": "Detail section for get_pull_request_detail_page."
                },
                "ci_item_id": {
                    "type": "string",
                    "description": "CI item id for get_pull_request_ci_log."
                },
                "ci_item_name": {
                    "type": "string",
                    "description": "CI item display name for get_pull_request_ci_log; used by providers that need a job name fallback."
                },
                "platform": {
                    "type": "string",
                    "enum": ["github", "gitlab", "gitcode", "unknown"],
                    "description": "Review platform kind for update_auth_token or clear_auth_token."
                },
                "host": {
                    "type": "string",
                    "description": "Review platform host for update_auth_token or clear_auth_token."
                },
                "token": {
                    "type": "string",
                    "description": "Personal access token for update_auth_token. Only provide this when the user explicitly asks to store that token."
                },
                "title": {
                    "type": "string",
                    "description": "Pull request title for create_pull_request."
                },
                "source_branch": {
                    "type": "string",
                    "description": "Source/head branch for create_pull_request."
                },
                "target_branch": {
                    "type": "string",
                    "description": "Target/base branch for create_pull_request."
                },
                "body": {
                    "type": "string",
                    "description": "Pull request body, review body, or comment body depending on action."
                },
                "draft": {
                    "type": "boolean",
                    "description": "Create a draft pull request when the provider supports it."
                },
                "thread_id": {
                    "type": "string",
                    "description": "Thread id returned by get_pull_request for reply_to_thread or resolve_thread."
                },
                "event": {
                    "type": "string",
                    "enum": ["comment", "approve", "request_changes"],
                    "description": "Review event for submit_review."
                },
                "resolved": {
                    "type": "boolean",
                    "description": "Whether resolve_thread should mark the thread resolved or reopened."
                }
            },
            "required": ["action"],
            "additionalProperties": false
        })
    }

    fn is_readonly(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self, input: Option<&Value>) -> bool {
        input
            .and_then(Self::action)
            .is_some_and(|action| !WRITE_ACTIONS.contains(&action))
    }

    fn needs_permissions(&self, input: Option<&Value>) -> bool {
        input
            .and_then(Self::action)
            .map(|action| WRITE_ACTIONS.contains(&action))
            .unwrap_or(true)
    }

    async fn validate_input(&self, input: &Value, _context: Option<&ToolUseContext>) -> ValidationResult {
        let Some(action) = Self::action(input) else {
            return ValidationResult {
                result: false,
                message: Some("action is required".to_string()),
                error_code: Some(400),
                meta: None,
            };
        };
        let valid = [
            ACTION_WORKSPACE_SNAPSHOT,
            ACTION_LIST_REMOTES,
            ACTION_LIST,
            ACTION_COUNT,
            ACTION_GET,
            ACTION_GET_DETAIL_PAGE,
            ACTION_GET_CI_LOG,
            ACTION_CREATE,
            ACTION_REPLY,
            ACTION_SUBMIT_REVIEW,
            ACTION_APPROVE,
            ACTION_REVOKE_APPROVAL,
            ACTION_REQUEST_CHANGES,
            ACTION_RESOLVE,
            ACTION_UPDATE_AUTH_TOKEN,
            ACTION_CLEAR_AUTH_TOKEN,
        ];
        if !valid.contains(&action) {
            return ValidationResult {
                result: false,
                message: Some(format!("Unsupported ReviewPlatform action: {}", action)),
                error_code: Some(400),
                meta: None,
            };
        }
        ValidationResult {
            result: true,
            message: None,
            error_code: None,
            meta: None,
        }
    }

    fn render_tool_use_message(&self, input: &Value, _options: &ToolRenderOptions) -> String {
        let action = Self::action(input).unwrap_or("unknown");
        match action {
            ACTION_WORKSPACE_SNAPSHOT => "Load review platform workspace snapshot".to_string(),
            ACTION_LIST_REMOTES => "List review platform remotes".to_string(),
            ACTION_LIST => "List pull requests".to_string(),
            ACTION_COUNT => "Count pull requests".to_string(),
            ACTION_GET => format!(
                "Open pull request {}",
                input.get("pull_request_id").and_then(Value::as_str).unwrap_or("detail")
            ),
            ACTION_GET_DETAIL_PAGE => "Load pull request detail page".to_string(),
            ACTION_GET_CI_LOG => "Load pull request CI log".to_string(),
            ACTION_CREATE => "Create pull request".to_string(),
            ACTION_REPLY => "Reply to pull request thread".to_string(),
            ACTION_SUBMIT_REVIEW => "Submit pull request review".to_string(),
            ACTION_APPROVE => "Approve pull request".to_string(),
            ACTION_REVOKE_APPROVAL => "Revoke pull request approval".to_string(),
            ACTION_REQUEST_CHANGES => "Request pull request changes".to_string(),
            ACTION_RESOLVE => "Resolve pull request thread".to_string(),
            ACTION_UPDATE_AUTH_TOKEN => "Update review platform auth token".to_string(),
            ACTION_CLEAR_AUTH_TOKEN => "Clear review platform auth token".to_string(),
            _ => format!("Review platform action: {}", action),
        }
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        let action = Self::string_field(input, "action")?;
        let repository_path = match action.as_str() {
            ACTION_UPDATE_AUTH_TOKEN | ACTION_CLEAR_AUTH_TOKEN => Self::optional_string_field(input, "repository_path")
                .map(|path| context.resolve_workspace_tool_path(&path))
                .transpose()?
                .or_else(|| context.workspace.as_ref().map(|workspace| workspace.root_path_string()))
                .unwrap_or_default(),
            _ => Self::repository_path(input, context)?,
        };

        let data = match action.as_str() {
            ACTION_LIST_REMOTES => self.handle_list_remotes(&repository_path).await?,
            ACTION_WORKSPACE_SNAPSHOT => self.handle_workspace_snapshot(&repository_path, input).await?,
            ACTION_COUNT => self.handle_count(&repository_path, input).await?,
            ACTION_LIST => self.handle_list(&repository_path, input).await?,
            ACTION_GET => self.handle_get(&repository_path, input).await?,
            ACTION_GET_DETAIL_PAGE => self.handle_get_detail_page(&repository_path, input).await?,
            ACTION_GET_CI_LOG => self.handle_get_ci_log(&repository_path, input).await?,
            ACTION_CREATE => self.handle_create(&repository_path, input).await?,
            ACTION_REPLY => self.handle_reply(&repository_path, input).await?,
            ACTION_SUBMIT_REVIEW => self.handle_submit_review(&repository_path, input).await?,
            ACTION_APPROVE => self.handle_approve(&repository_path, input).await?,
            ACTION_REVOKE_APPROVAL => self.handle_revoke_approval(&repository_path, input).await?,
            ACTION_REQUEST_CHANGES => self.handle_request_changes(&repository_path, input).await?,
            ACTION_RESOLVE => self.handle_resolve(&repository_path, input).await?,
            ACTION_UPDATE_AUTH_TOKEN => self.handle_update_auth_token(&repository_path, input).await?,
            ACTION_CLEAR_AUTH_TOKEN => self.handle_clear_auth_token(&repository_path, input).await?,
            _ => return Err(NortHingError::tool(format!("Unsupported action: {}", action))),
        };

        let result_for_assistant = self.render_result_for_assistant(&data);
        Ok(vec![ToolResult::Result {
            data,
            result_for_assistant: Some(result_for_assistant),
            image_attachments: None,
        }])
    }
}

impl Default for ReviewPlatformTool {
    fn default() -> Self {
        Self::new()
    }
}

fn supported_remotes(remotes: &[ReviewPlatformRemote]) -> Vec<&ReviewPlatformRemote> {
    remotes.iter().filter(|remote| remote.supported).collect()
}

fn remote_ambiguity_message(remotes: &[&ReviewPlatformRemote]) -> String {
    let mut lines = vec![
        "Multiple supported review platform remotes were found. Provide remote_id explicitly.".to_string(),
        "Candidate remotes:".to_string(),
    ];
    lines.extend(remotes.iter().map(|remote| {
        format!(
            "- remote_id: {} | name: {} | platform: {:?} | project: {} | url: {}",
            remote.id, remote.name, remote.platform, remote.project_path, remote.web_url
        )
    }));
    lines.join("\n")
}
