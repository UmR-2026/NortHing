//! Provider factory, token storage, auth challenges, and remote capability
//! selection for the review platform service.

use crate::infrastructure::try_get_path_manager_arc;
use crate::service::review_platform::types::{
    ProviderContext, ReviewAuthSource, ReviewAuthState, ReviewPlatformAccount, ReviewPlatformAuthChallenge,
    ReviewPlatformAuthChallengeState, ReviewPlatformAuthTokens, ReviewPlatformCapabilities, ReviewPlatformError,
    ReviewPlatformKind, ReviewPlatformPagination, ReviewPlatformRemote, ReviewPlatformRepositoryRef,
    ReviewPlatformWorkspaceSnapshot, StoredReviewPlatformTokens,
};
use std::path::PathBuf;
use tokio::fs;

const DEFAULT_PR_PAGE: u32 = 1;
const DEFAULT_PR_PAGE_SIZE: u32 = 10;

pub(crate) fn provider_for(platform: ReviewPlatformKind) -> &'static dyn super::providers::ReviewProvider {
    match platform {
        ReviewPlatformKind::Github => &super::providers::github::GithubProvider,
        ReviewPlatformKind::Gitlab => &super::providers::gitlab::GitlabProvider,
        ReviewPlatformKind::Gitcode => &super::providers::gitcode::GitcodeProvider,
        ReviewPlatformKind::Unknown => &super::providers::UnsupportedProvider,
    }
}

pub(crate) fn require_write_token<'a>(ctx: &'a ProviderContext, action: &str) -> Result<&'a str, ReviewPlatformError> {
    ctx.token.as_deref().ok_or_else(|| {
        ReviewPlatformError::Api(format!(
            "{} requires a {} token for {}",
            action,
            platform_label(ctx.remote.platform),
            ctx.remote.host
        ))
    })
}

pub(crate) fn provider_context(
    remote: ReviewPlatformRemote,
    auth_tokens: &ReviewPlatformAuthTokens,
) -> Result<ProviderContext, ReviewPlatformError> {
    let api_base_url = match remote.platform {
        ReviewPlatformKind::Github => "https://api.github.com".to_string(),
        ReviewPlatformKind::Gitlab => format!("https://{}/api/v4", remote.host),
        ReviewPlatformKind::Gitcode => "https://api.gitcode.com/api/v5".to_string(),
        ReviewPlatformKind::Unknown => {
            return Err(ReviewPlatformError::UnsupportedPlatform(remote.host));
        }
    };
    let token = token_for_remote(&remote, auth_tokens);
    Ok(ProviderContext {
        remote,
        api_base_url,
        token,
    })
}

pub(crate) fn token_for_remote(
    remote: &ReviewPlatformRemote,
    auth_tokens: &ReviewPlatformAuthTokens,
) -> Option<String> {
    auth_tokens
        .get(remote.platform, &remote.host)
        .map(str::to_string)
        .or_else(|| env_token_for_platform(remote.platform))
}

fn env_token_for_platform(platform: ReviewPlatformKind) -> Option<String> {
    let names: &[&str] = match platform {
        ReviewPlatformKind::Github => &["GITHUB_TOKEN", "GH_TOKEN"],
        ReviewPlatformKind::Gitlab => &["GITLAB_TOKEN", "GITLAB_PRIVATE_TOKEN"],
        ReviewPlatformKind::Gitcode => &["GITCODE_TOKEN"],
        ReviewPlatformKind::Unknown => &[],
    };
    names.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

pub(crate) fn auth_for_platform_host(
    platform: ReviewPlatformKind,
    host: &str,
    auth_tokens: &ReviewPlatformAuthTokens,
) -> (ReviewAuthState, ReviewAuthSource) {
    if platform == ReviewPlatformKind::Unknown {
        return (ReviewAuthState::Unsupported, ReviewAuthSource::Unsupported);
    }
    if auth_tokens.get(platform, host).is_some() {
        return (ReviewAuthState::Connected, ReviewAuthSource::Stored);
    }
    if env_token_for_platform(platform).is_some() {
        return (ReviewAuthState::Connected, ReviewAuthSource::Env);
    }
    if platform == ReviewPlatformKind::Gitcode {
        (ReviewAuthState::NotConnected, ReviewAuthSource::None)
    } else {
        (ReviewAuthState::NotRequired, ReviewAuthSource::None)
    }
}

pub(crate) fn token_key(platform: ReviewPlatformKind, host: &str) -> Option<String> {
    if platform == ReviewPlatformKind::Unknown {
        return None;
    }
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }
    Some(format!("{}:{}", platform.as_str(), host))
}

pub(crate) fn stored_token_file_path() -> Result<PathBuf, ReviewPlatformError> {
    let path_manager = try_get_path_manager_arc().map_err(|error| ReviewPlatformError::Api(error.to_string()))?;
    Ok(path_manager.user_data_dir().join("review-platform-tokens.json"))
}

pub(crate) async fn load_stored_tokens() -> Result<ReviewPlatformAuthTokens, ReviewPlatformError> {
    let stored = load_stored_token_file().await?;
    Ok(ReviewPlatformAuthTokens {
        tokens: stored
            .tokens
            .into_iter()
            .filter_map(|(key, entry)| {
                let token = entry.token.trim().to_string();
                if token.is_empty() {
                    None
                } else {
                    Some((key, token))
                }
            })
            .collect(),
    })
}

pub(crate) async fn load_stored_token_file() -> Result<StoredReviewPlatformTokens, ReviewPlatformError> {
    let path = stored_token_file_path()?;
    match fs::read_to_string(&path).await {
        Ok(content) => serde_json::from_str::<StoredReviewPlatformTokens>(&content)
            .map_err(|error| ReviewPlatformError::Parse(error.to_string())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(StoredReviewPlatformTokens::default()),
        Err(error) => Err(ReviewPlatformError::Api(format!(
            "Failed to read review platform token store: {}",
            error
        ))),
    }
}

pub(crate) async fn save_stored_token_file(stored: &StoredReviewPlatformTokens) -> Result<(), ReviewPlatformError> {
    let path = stored_token_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.map_err(|error| {
            ReviewPlatformError::Api(format!(
                "Failed to create review platform token store directory: {}",
                error
            ))
        })?;
    }
    let content =
        serde_json::to_string_pretty(stored).map_err(|error| ReviewPlatformError::Parse(error.to_string()))?;
    fs::write(&path, content)
        .await
        .map_err(|error| ReviewPlatformError::Api(format!("Failed to write review platform token store: {}", error)))
}

pub(crate) fn select_remote<'a>(
    remotes: &'a [ReviewPlatformRemote],
    remote_id: Option<&str>,
) -> Option<&'a ReviewPlatformRemote> {
    if let Some(remote_id) = remote_id {
        if let Some(remote) = remotes.iter().find(|remote| remote.id == remote_id) {
            return Some(remote);
        }
    }
    remotes
        .iter()
        .find(|remote| remote.supported)
        .or_else(|| remotes.first())
}

pub(crate) fn select_remote_for_action<'a>(
    remotes: &'a [ReviewPlatformRemote],
    remote_id: Option<&str>,
) -> Result<&'a ReviewPlatformRemote, ReviewPlatformError> {
    if let Some(remote_id) = remote_id {
        return remotes
            .iter()
            .find(|remote| remote.id == remote_id)
            .ok_or_else(|| ReviewPlatformError::RemoteNotFound(remote_id.to_string()));
    }

    let supported = remotes.iter().filter(|remote| remote.supported).collect::<Vec<_>>();
    match supported.as_slice() {
        [] => remotes
            .first()
            .ok_or_else(|| ReviewPlatformError::RemoteNotFound("default".to_string())),
        [remote] => Ok(remote),
        _ => Err(ReviewPlatformError::Api(format!(
            "Multiple supported review platform remotes were found. Provide remote_id explicitly. Candidate remotes:\n{}",
            supported
                .iter()
                .map(|remote| format!(
                    "- remote_id: {} | name: {} | platform: {:?} | project: {} | url: {}",
                    remote.id, remote.name, remote.platform, remote.project_path, remote.web_url
                ))
                .collect::<Vec<_>>()
                .join("\n")
        ))),
    }
}

pub(crate) fn empty_snapshot(
    remotes: Vec<ReviewPlatformRemote>,
    selected_remote_id: Option<String>,
    account: Option<ReviewPlatformAccount>,
    message: &str,
) -> ReviewPlatformWorkspaceSnapshot {
    let mut accounts = account.into_iter().collect::<Vec<_>>();
    if let Some(account) = accounts.first_mut() {
        if account.message.is_none() && !message.trim().is_empty() {
            account.message = Some(message.to_string());
        }
    }

    ReviewPlatformWorkspaceSnapshot {
        remotes,
        selected_remote_id,
        accounts,
        repository: None,
        pull_requests: Vec::new(),
        pagination: ReviewPlatformPagination {
            page: DEFAULT_PR_PAGE,
            per_page: DEFAULT_PR_PAGE_SIZE,
            total: Some(0),
            has_next: false,
        },
        capabilities: ReviewPlatformCapabilities {
            can_create_review: false,
            can_create_pull_request: false,
            can_reply_to_thread: false,
            can_resolve_thread: false,
            can_approve: false,
            can_revoke_approval: false,
            can_request_changes: false,
            can_merge: false,
            supports_draft_review: false,
        },
        message: if message.trim().is_empty() {
            None
        } else {
            Some(message.to_string())
        },
        auth_challenge: None,
    }
}

pub(crate) fn auth_required_snapshot(
    remotes: Vec<ReviewPlatformRemote>,
    remote: ReviewPlatformRemote,
    repository: Option<ReviewPlatformRepositoryRef>,
    account: ReviewPlatformAccount,
    capabilities: ReviewPlatformCapabilities,
    challenge: ReviewPlatformAuthChallenge,
) -> ReviewPlatformWorkspaceSnapshot {
    ReviewPlatformWorkspaceSnapshot {
        remotes,
        selected_remote_id: Some(remote.id),
        accounts: vec![account],
        repository,
        pull_requests: Vec::new(),
        pagination: ReviewPlatformPagination {
            page: DEFAULT_PR_PAGE,
            per_page: DEFAULT_PR_PAGE_SIZE,
            total: Some(0),
            has_next: false,
        },
        capabilities,
        message: Some(challenge.message.clone()),
        auth_challenge: Some(challenge),
    }
}

pub(crate) fn repository_ref(
    remote: &ReviewPlatformRemote,
    workspace_path: Option<String>,
) -> ReviewPlatformRepositoryRef {
    ReviewPlatformRepositoryRef {
        provider_id: remote.id.clone(),
        platform: remote.platform,
        host: remote.host.clone(),
        owner: remote.owner.clone(),
        name: remote.repository_name.clone(),
        project_path: remote.project_path.clone(),
        default_branch: "main".to_string(),
        workspace_path,
        web_url: remote.web_url.clone(),
    }
}

pub(crate) fn account_for_remote(remote: &ReviewPlatformRemote) -> ReviewPlatformAccount {
    ReviewPlatformAccount {
        id: remote.id.clone(),
        platform: remote.platform,
        label: format!("{} ({})", platform_label(remote.platform), remote.host),
        username: None,
        host: remote.host.clone(),
        auth_state: remote.auth_state,
        auth_source: remote.auth_source,
        scopes: if matches!(remote.auth_source, ReviewAuthSource::Env | ReviewAuthSource::Stored) {
            vec!["pull_request:read".to_string()]
        } else {
            Vec::new()
        },
        message: remote.message.clone(),
    }
}

pub(crate) fn capabilities_for_remote(_remote: &ReviewPlatformRemote) -> ReviewPlatformCapabilities {
    let platform = _remote.platform;
    ReviewPlatformCapabilities {
        can_create_review: matches!(
            platform,
            ReviewPlatformKind::Github | ReviewPlatformKind::Gitlab | ReviewPlatformKind::Gitcode
        ),
        can_create_pull_request: matches!(
            platform,
            ReviewPlatformKind::Github | ReviewPlatformKind::Gitlab | ReviewPlatformKind::Gitcode
        ),
        can_reply_to_thread: matches!(platform, ReviewPlatformKind::Github | ReviewPlatformKind::Gitlab),
        can_resolve_thread: matches!(platform, ReviewPlatformKind::Gitlab),
        can_approve: matches!(
            platform,
            ReviewPlatformKind::Github | ReviewPlatformKind::Gitlab | ReviewPlatformKind::Gitcode
        ),
        can_revoke_approval: matches!(platform, ReviewPlatformKind::Gitlab),
        can_request_changes: matches!(platform, ReviewPlatformKind::Github),
        can_merge: false,
        supports_draft_review: matches!(platform, ReviewPlatformKind::Github),
    }
}

pub(crate) fn platform_label(platform: ReviewPlatformKind) -> &'static str {
    match platform {
        ReviewPlatformKind::Github => "GitHub",
        ReviewPlatformKind::Gitlab => "GitLab",
        ReviewPlatformKind::Gitcode => "GitCode",
        ReviewPlatformKind::Unknown => "Git",
    }
}

fn required_scopes_for_platform(platform: ReviewPlatformKind) -> Vec<String> {
    match platform {
        ReviewPlatformKind::Github => vec!["repo".to_string(), "pull_requests:read".to_string()],
        ReviewPlatformKind::Gitlab => {
            vec!["read_api".to_string(), "api for write actions".to_string()]
        }
        ReviewPlatformKind::Gitcode => vec!["pull_request".to_string()],
        ReviewPlatformKind::Unknown => Vec::new(),
    }
}

pub(crate) fn auth_state_for_challenge(state: ReviewPlatformAuthChallengeState) -> ReviewAuthState {
    match state {
        ReviewPlatformAuthChallengeState::Missing => ReviewAuthState::NotConnected,
        ReviewPlatformAuthChallengeState::Invalid => ReviewAuthState::Expired,
        ReviewPlatformAuthChallengeState::InsufficientScope => ReviewAuthState::Error,
    }
}

pub(crate) fn is_auth_http_error(error: &ReviewPlatformError) -> bool {
    matches!(error, ReviewPlatformError::Http { status: 401 | 403, .. })
}

pub(crate) fn auth_challenge_for_remote(
    remote: &ReviewPlatformRemote,
    error: &ReviewPlatformError,
    has_token: bool,
) -> ReviewPlatformAuthChallenge {
    let status = match error {
        ReviewPlatformError::Http { status, .. } => *status,
        _ => 0,
    };
    let state = if !has_token {
        ReviewPlatformAuthChallengeState::Missing
    } else if status == 403 {
        ReviewPlatformAuthChallengeState::InsufficientScope
    } else {
        ReviewPlatformAuthChallengeState::Invalid
    };
    let action = match state {
        ReviewPlatformAuthChallengeState::Missing => "Add",
        ReviewPlatformAuthChallengeState::Invalid => "Update",
        ReviewPlatformAuthChallengeState::InsufficientScope => "Update",
    };
    let reason = match state {
        ReviewPlatformAuthChallengeState::Missing => "a token is required to access this repository",
        ReviewPlatformAuthChallengeState::Invalid => "the saved or environment token was rejected",
        ReviewPlatformAuthChallengeState::InsufficientScope => "the token does not have enough permissions",
    };
    ReviewPlatformAuthChallenge {
        platform: remote.platform,
        host: remote.host.clone(),
        remote_id: remote.id.clone(),
        project_path: remote.project_path.clone(),
        state,
        message: format!(
            "{} {} token for {}: {}.",
            action,
            platform_label(remote.platform),
            remote.host,
            reason
        ),
        required_scopes: required_scopes_for_platform(remote.platform),
    }
}
