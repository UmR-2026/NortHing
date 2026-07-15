//! Service facade and core-owned product service assembly.
//!
//! Owner-crate implementations are re-exported here when they are safely
//! isolated. High-coupling runtime services stay here until their port
//! contracts and equivalence tests are explicit.

#[cfg(feature = "product-full")]
pub(crate) mod agent_memory; // Agent memory prompt helpers
#[cfg(feature = "service-integrations")]
pub mod announcement; // Announcement / feature-demo / tips system
pub mod audit_log; // R1 shell-exec audit log
pub(crate) mod bootstrap; // Workspace persona bootstrap helpers
pub mod config; // Config management
#[cfg(feature = "product-full")]
pub mod cron; // Scheduled jobs
pub mod filesystem; // FileSystem management
#[cfg(feature = "service-integrations")]
pub mod git; // Git service
pub mod i18n; // I18n service
pub mod lsp; // LSP (Language Server Protocol) system
#[cfg(all(feature = "service-integrations", feature = "product-full"))]
pub mod mcp; // MCP (Model Context Protocol) system
#[cfg(all(feature = "service-integrations", feature = "product-full"))]
pub mod remote_connect; // Remote Connect (phone → desktop)
pub mod remote_ssh; // Remote SSH (desktop → server)
#[cfg(feature = "service-integrations")]
pub mod review_platform; // Pull request review platform adapters
pub mod runtime; // Managed runtime and capability management
#[cfg(feature = "product-full")]
pub mod search; // Workspace search via managed flashgrep daemon
pub mod session; // Session persistence
#[cfg(feature = "product-full")]
pub mod session_usage; // Session runtime usage reports
#[cfg(feature = "product-full")]
pub mod snapshot; // Snapshot-based change tracking
#[cfg(feature = "product-full")]
pub mod token_usage; // Token usage tracking
pub mod workspace; // Workspace management // Diff calculation and merge service
pub mod workspace_runtime; // Workspace runtime layout / migration / initialization

// Terminal is implemented in the workspace-level `terminal-core` crate.
// This re-export preserves the legacy `northhing_core::service::terminal` path.
pub use terminal_core as terminal;

// Re-export main components.
#[cfg(feature = "service-integrations")]
pub use announcement::{AnnouncementCard, AnnouncementScheduler, AnnouncementSchedulerRef};
pub use bootstrap::reset_workspace_persona_files_to_default;
pub use config::{ConfigManager, ConfigProvider, ConfigService};
#[cfg(feature = "product-full")]
pub use cron::{global_cron_service, set_global_cron_service, CronEventSubscriber, CronService};
pub use diff::{DiffConfig, DiffHunk, DiffLine, DiffLineType, DiffOptions, DiffResult, DiffService};
#[cfg(feature = "service-integrations")]
pub use file_watch::{
    get_watched_paths, global_file_watch_service, initialize_file_watch_service, start_file_watch, stop_file_watch,
    FileWatchEvent, FileWatchEventKind, FileWatchService, FileWatcherConfig,
};
pub use filesystem::{DirectoryStats, FileSystemService, FileSystemServiceFactory};
#[cfg(feature = "service-integrations")]
pub use git::GitService;
pub use i18n::{get_global_i18n_service, I18nConfig, I18nService, LocaleId, LocaleMetadata};
pub use lsp::LspManager;
#[cfg(all(feature = "service-integrations", feature = "product-full"))]
pub use mcp::MCPService;
pub use northhing_services_core::{diagnostics, diff, system};
#[cfg(feature = "service-integrations")]
pub use northhing_services_integrations::file_watch;
#[cfg(feature = "service-integrations")]
pub use review_platform::{
    ReviewAuthSource, ReviewAuthState, ReviewChecks, ReviewDecision, ReviewFileStatus, ReviewItemState,
    ReviewPlatformAccount, ReviewPlatformAuthChallenge, ReviewPlatformAuthChallengeState, ReviewPlatformCapabilities,
    ReviewPlatformCiLog, ReviewPlatformCommit, ReviewPlatformError, ReviewPlatformFile, ReviewPlatformKind,
    ReviewPlatformPullRequest, ReviewPlatformPullRequestDetail, ReviewPlatformRemote, ReviewPlatformRepositoryRef,
    ReviewPlatformService, ReviewPlatformThread, ReviewPlatformWorkspaceSnapshot,
};
pub use runtime::{ResolvedCommand, RuntimeCommandCapability, RuntimeManager, RuntimeSource};
#[cfg(feature = "product-full")]
pub use search::{
    global_workspace_search_service, set_global_workspace_search_service, ContentSearchRequest, ContentSearchResult,
    GlobSearchRequest, GlobSearchResult, IndexTaskHandle, WorkspaceIndexStatus, WorkspaceSearchBackend,
    WorkspaceSearchContextLine, WorkspaceSearchDirtyFiles, WorkspaceSearchFileCount, WorkspaceSearchHit,
    WorkspaceSearchLine, WorkspaceSearchMatch, WorkspaceSearchMatchLocation, WorkspaceSearchOverlayStatus,
    WorkspaceSearchRepoPhase, WorkspaceSearchRepoStatus, WorkspaceSearchService, WorkspaceSearchTaskKind,
    WorkspaceSearchTaskPhase, WorkspaceSearchTaskState, WorkspaceSearchTaskStatus,
};
#[cfg(feature = "product-full")]
pub use snapshot::SnapshotService;
pub use system::{
    check_command, check_commands, run_command, run_command_simple, CheckCommandResult, CommandOutput, SystemError,
};
#[cfg(feature = "product-full")]
pub use token_usage::{
    ModelTokenStats, SessionTokenStats, TimeRange, TokenUsageQuery, TokenUsageRecord, TokenUsageService,
    TokenUsageSummary,
};
pub use workspace::{WorkspaceManager, WorkspaceProvider, WorkspaceService};
pub use workspace_runtime::{
    try_get_workspace_runtime_service_arc, workspace_runtime_service_arc, RuntimeMigrationRecord,
    WorkspaceRuntimeContext, WorkspaceRuntimeEnsureResult, WorkspaceRuntimeService, WorkspaceRuntimeTarget,
};
