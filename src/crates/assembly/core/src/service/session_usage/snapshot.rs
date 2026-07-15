//! R24 session_usage/snapshot sibling — extracted from service.rs L130-L293.
//!
//! Visibility: `pub(super)` for cross-sibling use within `session_usage` module.
//! Cross-sibling function calls use `super::sibling_name::fn_name(...)`.

use super::entry::SessionUsageReportRequest;
use crate::service::session::{DialogTurnData, DialogTurnKind, ModelRoundData, ToolItemData};
use crate::service::session_usage::redaction::redact_usage_label;
use crate::service::snapshot::get_snapshot_manager_for_workspace;
use crate::service::snapshot::types::FileOperation;
use crate::service::token_usage::TokenUsageRecord;
use crate::util::errors::NortHingResult;
use northhing_services_core::session_usage::types::*;
use std::collections::HashSet;
use std::path::Path;
pub async fn load_snapshot_facts(request: &SessionUsageReportRequest) -> UsageSnapshotFacts {
    let Some(workspace_path) = request.workspace_path.as_deref() else {
        return UsageSnapshotFacts::default();
    };

    let Some(manager) = get_snapshot_manager_for_workspace(Path::new(workspace_path)) else {
        return UsageSnapshotFacts::default();
    };

    match manager.get_session(&request.session_id).await {
        Ok(session) => UsageSnapshotFacts {
            source_available: true,
            operations: session
                .operations
                .into_iter()
                .map(snapshot_operation_from_file_operation)
                .collect(),
        },
        Err(_) => UsageSnapshotFacts::default(),
    }
}

pub fn is_reportable_usage_turn(turn: &DialogTurnData) -> bool {
    turn.kind != DialogTurnKind::LocalCommand
}

pub fn snapshot_operation_from_file_operation(operation: FileOperation) -> UsageSnapshotOperationSummary {
    UsageSnapshotOperationSummary {
        operation_id: operation.operation_id,
        session_id: operation.session_id,
        turn_index: operation.turn_index,
        file_path: operation.file_path.to_string_lossy().to_string(),
        lines_added: operation.diff_summary.lines_added as u64,
        lines_removed: operation.diff_summary.lines_removed as u64,
    }
}

pub fn build_workspace(request: &SessionUsageReportRequest) -> UsageWorkspace {
    UsageWorkspace {
        kind: if request.remote_connection_id.is_some() || request.remote_ssh_host.is_some() {
            UsageWorkspaceKind::RemoteSsh
        } else if request.workspace_path.is_some() {
            UsageWorkspaceKind::Local
        } else {
            UsageWorkspaceKind::Unknown
        },
        path_label: request
            .workspace_path
            .as_deref()
            .map(|path| redact_usage_label(path, 120).value),
        workspace_id: None,
        remote_connection_id: request.remote_connection_id.clone(),
        remote_ssh_host: request.remote_ssh_host.clone(),
    }
}

pub fn build_scope(turns: &[DialogTurnData], includes_subagents: bool) -> UsageScope {
    UsageScope {
        kind: UsageScopeKind::EntireSession,
        turn_count: turns.len(),
        from_turn_id: turns.first().map(|turn| turn.turn_id.clone()),
        to_turn_id: turns.last().map(|turn| turn.turn_id.clone()),
        includes_subagents,
    }
}

pub fn build_coverage(
    request: &SessionUsageReportRequest,
    turns: &[DialogTurnData],
    token_records: &[TokenUsageRecord],
    snapshot_facts: &UsageSnapshotFacts,
) -> UsageCoverage {
    let mut available = vec![UsageCoverageKey::WorkspaceIdentity];
    if request.include_hidden_subagents {
        available.push(UsageCoverageKey::SubagentScope);
    }
    if turns
        .iter()
        .flat_map(|turn| turn.model_rounds.iter())
        .any(super::utilities::has_model_timing_fact)
    {
        available.push(UsageCoverageKey::ModelRoundTiming);
    }
    if super::utilities::iter_tools(turns).any(super::utilities::has_tool_phase_timing_fact) {
        available.push(UsageCoverageKey::ToolPhaseTiming);
    }
    if token_records.iter().any(|record| record.cached_tokens_available) {
        available.push(UsageCoverageKey::CachedTokens);
    }
    if token_records.iter().any(|record| record.token_details.is_some()) {
        available.push(UsageCoverageKey::TokenDetailBreakdown);
    }
    if snapshot_facts.source_available {
        available.push(UsageCoverageKey::FileLineStats);
    }

    let mut missing = vec![
        UsageCoverageKey::ToolPhaseTiming,
        UsageCoverageKey::CachedTokens,
        UsageCoverageKey::TokenDetailBreakdown,
        UsageCoverageKey::FileLineStats,
        UsageCoverageKey::SubagentScope,
    ];
    if !available.contains(&UsageCoverageKey::ModelRoundTiming) {
        missing.push(UsageCoverageKey::ModelRoundTiming);
    }
    for available_key in &available {
        missing.retain(|key| key != available_key);
    }

    if request.remote_connection_id.is_some() || request.remote_ssh_host.is_some() {
        if snapshot_facts.source_available {
            available.push(UsageCoverageKey::RemoteSnapshotStats);
        } else {
            missing.push(UsageCoverageKey::RemoteSnapshotStats);
        }
    }

    available.sort_by_key(|key| format!("{:?}", key));
    available.dedup();
    missing.sort_by_key(|key| format!("{:?}", key));
    missing.dedup();

    let mut notes = vec![
        "Report is based on persisted turns, token records, and cached snapshot summaries that already exist."
            .to_string(),
    ];
    if missing.contains(&UsageCoverageKey::CachedTokens) {
        notes.push("Cached token source is unavailable when provider events do not report cache counts.".to_string());
    }
    if missing.contains(&UsageCoverageKey::SubagentScope) {
        notes.push("Subagent rows are excluded from this report scope.".to_string());
    }
    if snapshot_facts.source_available {
        notes.push("File line stats use cached snapshot operation summaries and do not read file bodies.".to_string());
    } else if request.remote_connection_id.is_some() || request.remote_ssh_host.is_some() {
        notes.push(
            "Remote snapshot summaries are unavailable for this workspace, so file line stats remain partial."
                .to_string(),
        );
    }

    UsageCoverage {
        level: UsageCoverageLevel::Partial,
        available,
        missing,
        notes,
    }
}
