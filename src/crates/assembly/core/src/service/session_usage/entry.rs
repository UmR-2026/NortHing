//! R24 session_usage/entry sibling — extracted from service.rs L1-L128.
//!
//! Visibility: `pub(super)` for cross-sibling use within `session_usage` module.
//! Cross-sibling function calls use `super::sibling_name::fn_name(...)`.

use crate::agentic::persistence::PersistenceManager;
use crate::service::session::{DialogTurnData, DialogTurnKind, ModelRoundData, ToolItemData, TurnStatus};
use crate::service::session_usage::classifier::classify_tool_usage;
use crate::service::session_usage::redaction::{
    display_workspace_relative_path, redact_usage_input_summary, redact_usage_label,
};
use crate::service::snapshot::get_snapshot_manager_for_workspace;
use crate::service::snapshot::types::FileOperation;
use crate::service::token_usage::{TimeRange, TokenUsageQuery, TokenUsageRecord, TokenUsageService};
use crate::util::errors::{NortHingError, NortHingResult};
use chrono::Utc;
use northhing_services_core::session_usage::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionUsageReportRequest {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_connection_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_ssh_host: Option<String>,
    #[serde(default)]
    pub include_hidden_subagents: bool,
}

pub async fn generate_session_usage_report(
    persistence_manager: &PersistenceManager,
    token_usage_service: Option<&TokenUsageService>,
    request: SessionUsageReportRequest,
) -> NortHingResult<SessionUsageReport> {
    let workspace_path = request
        .workspace_path
        .clone()
        .ok_or_else(|| NortHingError::validation("Workspace path is required for usage reports"))?;
    let turns = persistence_manager
        .load_session_turns(Path::new(&workspace_path), &request.session_id)
        .await?;
    let token_records = if let Some(service) = token_usage_service {
        service
            .query_records(TokenUsageQuery {
                model_id: None,
                session_id: Some(request.session_id.clone()),
                time_range: TimeRange::All,
                limit: None,
                offset: None,
                include_subagent: request.include_hidden_subagents,
            })
            .await
            .map_err(|error| NortHingError::service(format!("Failed to query token usage records: {}", error)))?
    } else {
        Vec::new()
    };

    let snapshot_facts = super::snapshot::load_snapshot_facts(&request).await;

    Ok(build_session_usage_report_from_sources(
        request,
        &turns,
        &token_records,
        &snapshot_facts,
        Utc::now().timestamp_millis(),
    ))
}

pub fn build_session_usage_report_from_turns(
    request: SessionUsageReportRequest,
    turns: &[DialogTurnData],
    token_records: &[TokenUsageRecord],
    generated_at: i64,
) -> SessionUsageReport {
    build_session_usage_report_from_sources(
        request,
        turns,
        token_records,
        &UsageSnapshotFacts::default(),
        generated_at,
    )
}

pub fn build_session_usage_report_from_sources(
    request: SessionUsageReportRequest,
    turns: &[DialogTurnData],
    token_records: &[TokenUsageRecord],
    snapshot_facts: &UsageSnapshotFacts,
    generated_at: i64,
) -> SessionUsageReport {
    let reportable_turns: Vec<DialogTurnData> = turns
        .iter()
        .filter(|turn| super::snapshot::is_reportable_usage_turn(turn))
        .cloned()
        .collect();
    let turns = reportable_turns.as_slice();
    let mut report = SessionUsageReport::partial_unavailable(&request.session_id, generated_at);
    report.report_id = format!("usage-{}-{}", request.session_id, generated_at);
    report.workspace = super::snapshot::build_workspace(&request);
    report.scope = super::snapshot::build_scope(turns, request.include_hidden_subagents);
    report.coverage = super::snapshot::build_coverage(&request, turns, token_records, snapshot_facts);
    report.time = super::breakdowns_core::build_time_breakdown(turns, generated_at);
    report.tokens = super::breakdowns_core::build_token_breakdown(token_records);
    report.models = super::breakdowns_core::build_model_breakdown(turns, token_records);
    report.tools = super::breakdowns_core::build_tool_breakdown(turns);
    report.files =
        super::breakdowns_extra::build_file_breakdown(request.workspace_path.as_deref(), turns, snapshot_facts);
    report.compression = super::breakdowns_extra::build_compression_breakdown(turns);
    report.errors = super::breakdowns_extra::build_error_breakdown(turns);
    report.slowest = super::breakdowns_extra::build_slowest_spans(turns, token_records);
    report.privacy = UsagePrivacy {
        prompt_content_included: false,
        tool_inputs_included: report.slowest.iter().any(|span| span.input_summary.is_some()),
        command_outputs_included: false,
        file_contents_included: false,
        redacted_fields: super::breakdowns_extra::collect_redacted_fields(&report),
    };
    report
}
