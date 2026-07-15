//! Session usage module
//!
//! R24 split: facade (service.rs) + 5 sibling files.

pub mod aggregation;
pub mod breakdowns_core;
pub mod breakdowns_extra;
pub mod entry;
pub mod format;
pub mod persist;
pub mod service;
pub mod snapshot;
pub mod tracking;
pub mod utilities;

pub use northhing_services_core::session_usage::{classifier, redaction, render, types};
pub use northhing_services_core::session_usage::{
    classify_tool_usage, display_workspace_relative_path, redact_usage_label, render_usage_report_markdown,
    render_usage_report_terminal, RedactedLabel, UsageToolCategory,
};
pub use northhing_services_core::session_usage::{
    SessionUsageReport, UsageCacheCoverage, UsageCompressionBreakdown, UsageCoverage, UsageCoverageKey,
    UsageCoverageLevel, UsageErrorBreakdown, UsageErrorExample, UsageFileBreakdown, UsageFileRow, UsageFileScope,
    UsageModelBreakdown, UsagePrivacy, UsageScope, UsageScopeKind, UsageSlowSpan, UsageSlowSpanKind,
    UsageSnapshotFacts, UsageSnapshotOperationSummary, UsageTimeAccounting, UsageTimeBreakdown, UsageTimeDenominator,
    UsageTokenBreakdown, UsageTokenSource, UsageToolBreakdown, UsageWorkspace, UsageWorkspaceKind,
    SESSION_USAGE_REPORT_SCHEMA_VERSION,
};
pub use service::{
    build_session_usage_report_from_sources, build_session_usage_report_from_turns, generate_session_usage_report,
    SessionUsageReportRequest,
};
