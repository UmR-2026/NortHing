//! Pure DTO types for the review platform service.
//!
//! This module owns the platform-neutral data contracts exchanged with the
//! agent runtime, desktop UI, and CLI. Provider-specific behavior lives in
//! [`super::providers`].

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEFAULT_PR_PAGE: u32 = 1;
const DEFAULT_PR_PAGE_SIZE: u32 = 10;
const MAX_PR_PAGE_SIZE: u32 = 50;
pub(super) const PROVIDER_ENRICH_CONCURRENCY: usize = 4;
pub(super) const MAX_CI_LOG_CHARS: usize = 80_000;

#[derive(Debug, thiserror::Error)]
pub enum ReviewPlatformError {
    #[error("Invalid repository path: {0}")]
    InvalidRepository(String),
    #[error("Remote not found: {0}")]
    RemoteNotFound(String),
    #[error("Unsupported review platform: {0}")]
    UnsupportedPlatform(String),
    #[error("Provider API failed: {0}")]
    Api(String),
    #[error("Provider API failed: HTTP {status}{message}")]
    Http { status: u16, message: String },
    #[error("Network error: {0}")]
    Network(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewPlatformKind {
    Github,
    Gitlab,
    Gitcode,
    Unknown,
}

impl ReviewPlatformKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Gitlab => "gitlab",
            Self::Gitcode => "gitcode",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewAuthState {
    NotConnected,
    NotRequired,
    Connected,
    Expired,
    Error,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewAuthSource {
    Env,
    Stored,
    None,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewItemState {
    Open,
    Merged,
    Closed,
    Draft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Approved,
    ChangesRequested,
    Commented,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewFileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformAccount {
    pub id: String,
    pub platform: ReviewPlatformKind,
    pub label: String,
    pub username: Option<String>,
    pub host: String,
    pub auth_state: ReviewAuthState,
    pub auth_source: ReviewAuthSource,
    pub scopes: Vec<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformRepositoryRef {
    pub provider_id: String,
    pub platform: ReviewPlatformKind,
    pub host: String,
    pub owner: String,
    pub name: String,
    pub project_path: String,
    pub default_branch: String,
    pub workspace_path: Option<String>,
    pub web_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformRemote {
    pub id: String,
    pub name: String,
    pub url: String,
    pub platform: ReviewPlatformKind,
    pub host: String,
    pub owner: String,
    pub repository_name: String,
    pub project_path: String,
    pub web_url: String,
    pub supported: bool,
    pub auth_state: ReviewAuthState,
    pub auth_source: ReviewAuthSource,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewChecks {
    pub total: i32,
    pub passed: i32,
    pub failed: i32,
    pub pending: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformCiItem {
    pub id: String,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub detail: Option<String>,
    pub stage: Option<String>,
    pub web_url: Option<String>,
    pub log: Option<String>,
    pub log_truncated: bool,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformPullRequest {
    pub id: String,
    pub number: i64,
    pub title: String,
    pub state: ReviewItemState,
    pub author: String,
    pub source_branch: String,
    pub target_branch: String,
    pub updated_at: String,
    pub web_url: String,
    pub additions: i32,
    pub deletions: i32,
    pub changed_files: i32,
    pub comments: i32,
    pub review_decision: ReviewDecision,
    pub checks: ReviewChecks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformFile {
    pub path: String,
    pub old_path: Option<String>,
    pub status: ReviewFileStatus,
    pub additions: i32,
    pub deletions: i32,
    pub patch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformCommit {
    pub hash: String,
    pub short_hash: String,
    pub title: String,
    pub author: String,
    pub committed_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewPlatformThreadKind {
    Review,
    Comment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformThread {
    pub id: String,
    pub provider_thread_id: Option<String>,
    pub provider_comment_id: Option<String>,
    pub kind: ReviewPlatformThreadKind,
    pub reply_to_provider_comment_id: Option<String>,
    pub file_path: Option<String>,
    pub line: Option<i64>,
    pub resolved: bool,
    pub author: String,
    pub body: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformPullRequestDetail {
    #[serde(flatten)]
    pub pull_request: ReviewPlatformPullRequest,
    pub body: String,
    pub ci: Vec<ReviewPlatformCiItem>,
    pub files: Vec<ReviewPlatformFile>,
    pub commits: Vec<ReviewPlatformCommit>,
    pub threads: Vec<ReviewPlatformThread>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewPlatformDetailSection {
    Overview,
    Ci,
    Files,
    Commits,
    Reviews,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformPullRequestDetailPage {
    #[serde(flatten)]
    pub pull_request: ReviewPlatformPullRequest,
    pub body: String,
    pub ci: Vec<ReviewPlatformCiItem>,
    pub files: Vec<ReviewPlatformFile>,
    pub commits: Vec<ReviewPlatformCommit>,
    pub threads: Vec<ReviewPlatformThread>,
    pub section: ReviewPlatformDetailSection,
    pub pagination: ReviewPlatformPagination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformCiLog {
    pub ci_item_id: String,
    pub log: Option<String>,
    pub truncated: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformCapabilities {
    pub can_create_review: bool,
    pub can_create_pull_request: bool,
    pub can_reply_to_thread: bool,
    pub can_resolve_thread: bool,
    pub can_approve: bool,
    pub can_revoke_approval: bool,
    pub can_request_changes: bool,
    pub can_merge: bool,
    pub supports_draft_review: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewSubmitEvent {
    Comment,
    Approve,
    RequestChanges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformCreatePullRequestRequest {
    pub repository_path: String,
    pub remote_id: Option<String>,
    pub title: String,
    pub source_branch: String,
    pub target_branch: String,
    pub body: Option<String>,
    pub draft: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformReplyToThreadRequest {
    pub repository_path: String,
    pub remote_id: String,
    pub pull_request_id: String,
    pub thread_id: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformSubmitReviewRequest {
    pub repository_path: String,
    pub remote_id: String,
    pub pull_request_id: String,
    pub event: ReviewSubmitEvent,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformResolveThreadRequest {
    pub repository_path: String,
    pub remote_id: String,
    pub pull_request_id: String,
    pub thread_id: String,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformApprovalRequest {
    pub repository_path: String,
    pub remote_id: String,
    pub pull_request_id: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformRequestChangesRequest {
    pub repository_path: String,
    pub remote_id: String,
    pub pull_request_id: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformActionResult {
    pub success: bool,
    pub message: String,
    pub web_url: Option<String>,
    pub pull_request: Option<ReviewPlatformPullRequest>,
    pub thread: Option<ReviewPlatformThread>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewPlatformAuthChallengeState {
    Missing,
    Invalid,
    InsufficientScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformAuthChallenge {
    pub platform: ReviewPlatformKind,
    pub host: String,
    pub remote_id: String,
    pub project_path: String,
    pub state: ReviewPlatformAuthChallengeState,
    pub message: String,
    pub required_scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformWorkspaceSnapshot {
    pub remotes: Vec<ReviewPlatformRemote>,
    pub selected_remote_id: Option<String>,
    pub accounts: Vec<ReviewPlatformAccount>,
    pub repository: Option<ReviewPlatformRepositoryRef>,
    pub pull_requests: Vec<ReviewPlatformPullRequest>,
    pub pagination: ReviewPlatformPagination,
    pub capabilities: ReviewPlatformCapabilities,
    pub message: Option<String>,
    pub auth_challenge: Option<ReviewPlatformAuthChallenge>,
}

pub struct ReviewPlatformService;

#[derive(Debug, Clone, Copy)]
pub(crate) struct PullRequestPagination {
    pub(crate) page: u32,
    pub(crate) per_page: u32,
}

impl PullRequestPagination {
    pub(crate) fn new(page: Option<u32>, per_page: Option<u32>) -> Self {
        Self {
            page: page.unwrap_or(DEFAULT_PR_PAGE).max(1),
            per_page: per_page.unwrap_or(DEFAULT_PR_PAGE_SIZE).clamp(1, MAX_PR_PAGE_SIZE),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlatformPagination {
    pub page: u32,
    pub per_page: u32,
    pub total: Option<u64>,
    pub has_next: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ReviewPlatformPullRequestPage {
    pub(crate) items: Vec<ReviewPlatformPullRequest>,
    pub(crate) pagination: ReviewPlatformPagination,
}

#[derive(Debug, Clone)]
pub(crate) struct ProviderContext {
    pub(crate) remote: ReviewPlatformRemote,
    pub(crate) api_base_url: String,
    pub(crate) token: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ReviewPlatformAuthTokens {
    pub(crate) tokens: HashMap<String, String>,
}

impl ReviewPlatformAuthTokens {
    pub(crate) fn get(&self, platform: ReviewPlatformKind, host: &str) -> Option<&str> {
        super::auth::token_key(platform, host).and_then(|key| self.tokens.get(&key).map(String::as_str))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredReviewPlatformTokens {
    #[serde(default)]
    pub(crate) tokens: HashMap<String, StoredReviewPlatformToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredReviewPlatformToken {
    pub(crate) token: String,
    pub(crate) updated_at: String,
}
