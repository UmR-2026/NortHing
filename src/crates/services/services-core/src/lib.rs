#![allow(clippy::too_many_arguments)]
//! Core service owner crate.
//!
//! This crate owns platform-agnostic service building blocks that can be
//! tested without compiling the full northhing product runtime.

pub mod diagnostics;
pub mod diff;
pub mod filesystem;
pub mod json_store;
pub mod process_manager;
pub mod session;
pub mod session_usage;
pub mod system;
pub mod token_usage;

// Re-export common stable types so cross-crate callers do not need deep
// module paths. Each group preserves the original module ownership; only
// the public surface is flattened here.
pub use diagnostics::{redact_diagnostic_log_text, redact_diagnostic_log_text_with_report, RedactedDiagnosticLog};
pub use diff::{DiffConfig, DiffHunk, DiffLine, DiffLineType, DiffOptions, DiffResult, DiffService};
pub use filesystem::{
    format_directory_listing, get_formatted_directory_listing, list_directory_entries,
    normalize_text_for_editor_disk_sync, DirectoryListingEntry, DirectoryScanResult, DirectoryStats,
    FileContentSearchOptions, FileSearchOptions, FileSearchResult, FileSearchResultGroup, FileSystemConfig,
    FileSystemError, FileSystemResult, FileSystemService, FileSystemServiceFactory, FileTreeNode, FileTreeOptions,
    FileTreeService, FileTreeStatistics, FormattedDirectoryListing, SearchMatchType,
};
pub use json_store::{JsonFileStore, JsonFileStoreError};
pub use process_manager::{cleanup_all_processes, create_command, create_tokio_command, ProcessManager};
pub use session::{
    build_branched_session_metadata, build_session_index_snapshot, build_session_metadata, build_session_metadata_page,
    collect_hidden_subagent_cascade, empty_session_metadata_page, merge_session_custom_metadata,
    refresh_session_metadata_from_turns, remove_session_index_entry, set_deep_review_cache,
    set_deep_review_run_manifest, set_session_relationship, try_refresh_session_metadata_for_saved_turn,
    upsert_session_index_entry, BranchSessionMetadataFacts, SessionBranchRequest, SessionBranchResult, SessionKind,
    SessionMetadataBuildFacts, SessionMetadataPage, SessionMetadataStore, SessionMetadataStoreError,
    SessionStorageLayout,
};
pub use session_usage::{
    classify_tool_usage, redact_usage_label, render_usage_report_markdown, render_usage_report_terminal,
    SessionUsageReport, UsageCoverage, UsageCoverageKey, UsageErrorBreakdown, UsageFileBreakdown, UsageModelBreakdown,
    UsagePrivacy, UsageScope, UsageSlowSpan, UsageSlowSpanKind, UsageSnapshotFacts, UsageTimeAccounting,
    UsageTimeBreakdown, UsageTokenBreakdown, UsageToolBreakdown, UsageWorkspace, UsageWorkspaceKind,
    SESSION_USAGE_REPORT_SCHEMA_VERSION,
};
pub use system::{
    check_command, check_commands, run_command, run_command_simple, CheckCommandResult, CommandOutput, SystemError,
    SystemInfo,
};
pub use token_usage::{
    ModelTokenStats, SessionTokenStats, TimeRange, TokenUsageQuery, TokenUsageRecord, TokenUsageSummary,
};
