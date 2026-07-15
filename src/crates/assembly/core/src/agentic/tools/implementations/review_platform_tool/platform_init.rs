//! Authentication and platform kind helpers for `ReviewPlatformTool`.
//!
//! Owns `platform_kind` parsing and auth-token action handlers.

use crate::agentic::tools::framework::ToolResult;
use crate::service::review_platform::{ReviewPlatformError, ReviewPlatformKind, ReviewPlatformService};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

impl super::ReviewPlatformTool {
    pub(super) fn platform_kind(input: &Value) -> NortHingResult<ReviewPlatformKind> {
        match Self::string_field(input, "platform")?.as_str() {
            "github" => Ok(ReviewPlatformKind::Github),
            "gitlab" => Ok(ReviewPlatformKind::Gitlab),
            "gitcode" => Ok(ReviewPlatformKind::Gitcode),
            "unknown" => Ok(ReviewPlatformKind::Unknown),
            other => Err(NortHingError::tool(format!(
                "Unsupported review platform kind: {}",
                other
            ))),
        }
    }

    pub(super) fn auth_required_result(
        action: &str,
        repository_path: &str,
        remote_id: &str,
        error: &ReviewPlatformError,
    ) -> Option<Value> {
        let status = match error {
            ReviewPlatformError::Http { status, .. } if *status == 401 || *status == 403 => *status,
            _ => return None,
        };
        let state = if status == 403 { "insufficient_scope" } else { "invalid" };
        Some(json!({
            "action": action,
            "repositoryPath": repository_path,
            "remoteId": remote_id,
            "status": "needs_auth",
            "authChallenge": {
                "state": state,
                "message": if status == 403 {
                    "Review platform token is missing required permissions. Update the token in the pull request panel, then retry."
                } else {
                    "Review platform authentication is required or the configured token was rejected. Add or update the token in the pull request panel, then retry."
                },
            },
            "openPanel": {
                "type": "review-platform-auth",
                "workspacePath": repository_path,
                "remoteId": remote_id,
            },
        }))
    }

    pub(super) async fn handle_update_auth_token(&self, repository_path: &str, input: &Value) -> NortHingResult<Value> {
        let platform = Self::platform_kind(input)?;
        let host = Self::string_field(input, "host")?;
        let token = Self::string_field(input, "token")?;
        ReviewPlatformService::update_auth_token(platform, &host, &token)
            .await
            .map_err(|error| NortHingError::tool(error.to_string()))?;
        Ok(json!({
            "action": super::ACTION_UPDATE_AUTH_TOKEN,
            "repositoryPath": repository_path,
            "platform": platform,
            "host": host,
            "status": "ok",
        }))
    }

    pub(super) async fn handle_clear_auth_token(&self, repository_path: &str, input: &Value) -> NortHingResult<Value> {
        let platform = Self::platform_kind(input)?;
        let host = Self::string_field(input, "host")?;
        ReviewPlatformService::clear_auth_token(platform, &host)
            .await
            .map_err(|error| NortHingError::tool(error.to_string()))?;
        Ok(json!({
            "action": super::ACTION_CLEAR_AUTH_TOKEN,
            "repositoryPath": repository_path,
            "platform": platform,
            "host": host,
            "status": "ok",
        }))
    }
}
