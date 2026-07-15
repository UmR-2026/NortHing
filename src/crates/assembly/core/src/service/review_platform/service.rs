//! `ReviewPlatformService` facade — high-level async operations invoked by the
//! agent runtime, desktop UI, and CLI.

use super::auth::{
    account_for_remote, auth_challenge_for_remote, auth_required_snapshot, auth_state_for_challenge,
    capabilities_for_remote, empty_snapshot, is_auth_http_error, load_stored_token_file, load_stored_tokens,
    provider_context, provider_for, repository_ref, save_stored_token_file, select_remote, select_remote_for_action,
    token_for_remote, token_key,
};
use super::types::{
    ProviderContext, PullRequestPagination, ReviewPlatformActionResult, ReviewPlatformApprovalRequest,
    ReviewPlatformAuthChallengeState, ReviewPlatformCiLog, ReviewPlatformCreatePullRequestRequest,
    ReviewPlatformDetailSection, ReviewPlatformError, ReviewPlatformKind, ReviewPlatformPullRequestDetail,
    ReviewPlatformPullRequestDetailPage, ReviewPlatformRemote, ReviewPlatformReplyToThreadRequest,
    ReviewPlatformRequestChangesRequest, ReviewPlatformResolveThreadRequest, ReviewPlatformService,
    ReviewPlatformSubmitReviewRequest, ReviewPlatformWorkspaceSnapshot, StoredReviewPlatformToken,
};
use crate::service::git::{execute_git_command, get_repository_root};
use std::collections::HashSet;

impl ReviewPlatformService {
    pub async fn discover_remotes(repository_path: &str) -> Result<Vec<ReviewPlatformRemote>, ReviewPlatformError> {
        let auth_tokens = load_stored_tokens().await?;
        Self::discover_remotes_with_tokens(repository_path, &auth_tokens).await
    }

    pub(crate) async fn discover_remotes_with_tokens(
        repository_path: &str,
        auth_tokens: &super::types::ReviewPlatformAuthTokens,
    ) -> Result<Vec<ReviewPlatformRemote>, ReviewPlatformError> {
        let root = get_repository_root(repository_path)
            .map_err(|error| ReviewPlatformError::InvalidRepository(error.to_string()))?;
        let output = execute_git_command(&root, &["remote", "-v"])
            .await
            .map_err(|error| ReviewPlatformError::InvalidRepository(error.to_string()))?;

        let mut seen = HashSet::new();
        let mut remotes = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            if parts.get(2).is_some_and(|kind| *kind != "(fetch)") {
                continue;
            }
            let remote_name = parts[0];
            let remote_url = parts[1];
            let key = format!("{}|{}", remote_name, remote_url);
            if !seen.insert(key) {
                continue;
            }
            if let Some(remote) = super::providers::util::parse_remote(remote_name, remote_url, auth_tokens) {
                remotes.push(remote);
            }
        }

        Ok(remotes)
    }

    pub async fn workspace_snapshot(
        repository_path: &str,
        remote_id: Option<&str>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<ReviewPlatformWorkspaceSnapshot, ReviewPlatformError> {
        if crate::service::remote_ssh::workspace_state::is_remote_path(repository_path).await {
            return Ok(empty_snapshot(
                Vec::new(),
                None,
                None,
                "Pull request browsing is not available for remote SSH workspaces yet.",
            ));
        }

        let pagination_request = PullRequestPagination::new(page, per_page);
        let auth_tokens = load_stored_tokens().await?;
        let root = get_repository_root(repository_path)
            .map_err(|error| ReviewPlatformError::InvalidRepository(error.to_string()))?;
        let remotes = Self::discover_remotes_with_tokens(&root, &auth_tokens).await?;
        let selected_remote = select_remote(&remotes, remote_id).cloned();

        let Some(remote) = selected_remote else {
            return Ok(empty_snapshot(remotes, None, None, "No Git remotes were found"));
        };

        if !remote.supported {
            return Ok(empty_snapshot(
                remotes,
                Some(remote.id.clone()),
                Some(account_for_remote(&remote)),
                remote.message.as_deref().unwrap_or("Unsupported remote provider"),
            ));
        }

        if remote.platform == ReviewPlatformKind::Gitcode && token_for_remote(&remote, &auth_tokens).is_none() {
            return Ok(empty_snapshot(
                remotes,
                Some(remote.id.clone()),
                Some(account_for_remote(&remote)),
                "GitCode pull request APIs require a Personal Access Token. Add a token for this remote and refresh.",
            ));
        }

        let ctx = provider_context(remote.clone(), &auth_tokens)?;
        let provider = provider_for(ctx.remote.platform);
        let repository = Some(repository_ref(&ctx.remote, Some(root)));
        let account = account_for_remote(&ctx.remote);
        let capabilities = capabilities_for_remote(&remote);
        match provider.list_pull_requests(&ctx, pagination_request).await {
            Ok(page) => Ok(ReviewPlatformWorkspaceSnapshot {
                remotes,
                selected_remote_id: Some(remote.id.clone()),
                accounts: vec![account],
                repository,
                pull_requests: page.items,
                pagination: page.pagination,
                capabilities,
                message: None,
                auth_challenge: None,
            }),
            Err(error) if is_auth_http_error(&error) => {
                let challenge =
                    auth_challenge_for_remote(&remote, &error, token_for_remote(&remote, &auth_tokens).is_some());
                let mut account = account;
                account.auth_state = auth_state_for_challenge(challenge.state);
                account.auth_source = if matches!(challenge.state, ReviewPlatformAuthChallengeState::Missing) {
                    super::types::ReviewAuthSource::None
                } else {
                    account.auth_source
                };
                account.message = Some(challenge.message.clone());
                Ok(auth_required_snapshot(
                    remotes,
                    remote,
                    repository,
                    account,
                    capabilities,
                    challenge,
                ))
            }
            Err(error) => Err(error),
        }
    }

    pub async fn pull_request_detail(
        repository_path: &str,
        remote_id: &str,
        pull_request_id: &str,
    ) -> Result<ReviewPlatformPullRequestDetail, ReviewPlatformError> {
        if crate::service::remote_ssh::workspace_state::is_remote_path(repository_path).await {
            return Err(ReviewPlatformError::UnsupportedPlatform(
                "remote SSH workspace".to_string(),
            ));
        }

        let auth_tokens = load_stored_tokens().await?;
        let root = get_repository_root(repository_path)
            .map_err(|error| ReviewPlatformError::InvalidRepository(error.to_string()))?;
        let remotes = Self::discover_remotes_with_tokens(&root, &auth_tokens).await?;
        let remote = remotes
            .into_iter()
            .find(|remote| remote.id == remote_id)
            .ok_or_else(|| ReviewPlatformError::RemoteNotFound(remote_id.to_string()))?;
        if !remote.supported {
            return Err(ReviewPlatformError::UnsupportedPlatform(remote.host));
        }
        let ctx = provider_context(remote, &auth_tokens)?;
        provider_for(ctx.remote.platform)
            .pull_request_detail(&ctx, pull_request_id)
            .await
    }

    pub async fn pull_request_detail_page(
        repository_path: &str,
        remote_id: &str,
        pull_request_id: &str,
        section: ReviewPlatformDetailSection,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<ReviewPlatformPullRequestDetailPage, ReviewPlatformError> {
        let ctx = Self::provider_context_for_repository(repository_path, Some(remote_id)).await?;
        provider_for(ctx.remote.platform)
            .pull_request_detail_page(
                &ctx,
                pull_request_id,
                section,
                PullRequestPagination::new(page, per_page),
            )
            .await
    }

    pub async fn pull_request_ci_log(
        repository_path: &str,
        remote_id: &str,
        pull_request_id: &str,
        ci_item_id: &str,
        ci_item_name: &str,
    ) -> Result<ReviewPlatformCiLog, ReviewPlatformError> {
        let ctx = Self::provider_context_for_repository(repository_path, Some(remote_id)).await?;
        provider_for(ctx.remote.platform)
            .pull_request_ci_log(&ctx, pull_request_id, ci_item_id, ci_item_name)
            .await
    }

    pub async fn create_pull_request(
        request: ReviewPlatformCreatePullRequestRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        let ctx = Self::provider_context_for_repository(&request.repository_path, request.remote_id.as_deref()).await?;
        provider_for(ctx.remote.platform)
            .create_pull_request(&ctx, &request)
            .await
    }

    pub async fn reply_to_thread(
        request: ReviewPlatformReplyToThreadRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        let ctx =
            Self::provider_context_for_repository(&request.repository_path, Some(request.remote_id.as_str())).await?;
        provider_for(ctx.remote.platform).reply_to_thread(&ctx, &request).await
    }

    pub async fn submit_review(
        request: ReviewPlatformSubmitReviewRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        let ctx =
            Self::provider_context_for_repository(&request.repository_path, Some(request.remote_id.as_str())).await?;
        provider_for(ctx.remote.platform).submit_review(&ctx, &request).await
    }

    pub async fn resolve_thread(
        request: ReviewPlatformResolveThreadRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        let ctx =
            Self::provider_context_for_repository(&request.repository_path, Some(request.remote_id.as_str())).await?;
        provider_for(ctx.remote.platform).resolve_thread(&ctx, &request).await
    }

    pub async fn approve_pull_request(
        request: ReviewPlatformApprovalRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        let ctx =
            Self::provider_context_for_repository(&request.repository_path, Some(request.remote_id.as_str())).await?;
        provider_for(ctx.remote.platform)
            .approve_pull_request(&ctx, &request)
            .await
    }

    pub async fn revoke_approval(
        request: ReviewPlatformApprovalRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        let ctx =
            Self::provider_context_for_repository(&request.repository_path, Some(request.remote_id.as_str())).await?;
        provider_for(ctx.remote.platform).revoke_approval(&ctx, &request).await
    }

    pub async fn request_changes(
        request: ReviewPlatformRequestChangesRequest,
    ) -> Result<ReviewPlatformActionResult, ReviewPlatformError> {
        let ctx =
            Self::provider_context_for_repository(&request.repository_path, Some(request.remote_id.as_str())).await?;
        provider_for(ctx.remote.platform).request_changes(&ctx, &request).await
    }

    async fn provider_context_for_repository(
        repository_path: &str,
        remote_id: Option<&str>,
    ) -> Result<ProviderContext, ReviewPlatformError> {
        if crate::service::remote_ssh::workspace_state::is_remote_path(repository_path).await {
            return Err(ReviewPlatformError::UnsupportedPlatform(
                "remote SSH workspace".to_string(),
            ));
        }

        let auth_tokens = load_stored_tokens().await?;
        let root = get_repository_root(repository_path)
            .map_err(|error| ReviewPlatformError::InvalidRepository(error.to_string()))?;
        let remotes = Self::discover_remotes_with_tokens(&root, &auth_tokens).await?;
        let remote = select_remote_for_action(&remotes, remote_id)?.clone();
        if !remote.supported {
            return Err(ReviewPlatformError::UnsupportedPlatform(remote.host));
        }
        provider_context(remote, &auth_tokens)
    }

    pub async fn update_auth_token(
        platform: ReviewPlatformKind,
        host: &str,
        token: &str,
    ) -> Result<(), ReviewPlatformError> {
        let token = token.trim();
        if token.is_empty() {
            return Err(ReviewPlatformError::Api("Token cannot be empty".to_string()));
        }
        let key =
            token_key(platform, host).ok_or_else(|| ReviewPlatformError::UnsupportedPlatform(host.to_string()))?;
        let mut stored = load_stored_token_file().await?;
        stored.tokens.insert(
            key,
            StoredReviewPlatformToken {
                token: token.to_string(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            },
        );
        save_stored_token_file(&stored).await
    }

    pub async fn clear_auth_token(platform: ReviewPlatformKind, host: &str) -> Result<(), ReviewPlatformError> {
        let key =
            token_key(platform, host).ok_or_else(|| ReviewPlatformError::UnsupportedPlatform(host.to_string()))?;
        let mut stored = load_stored_token_file().await?;
        stored.tokens.remove(&key);
        save_stored_token_file(&stored).await
    }
}
