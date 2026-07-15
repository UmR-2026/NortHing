use crate::agentic::coordination::global_coordinator;
use crate::agentic::tools::framework::ToolUseContext;
use crate::service::cron::CronJob;
use crate::service_agent_runtime::CoreServiceAgentRuntime;
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_agent_runtime::runtime::AgentRuntime;
use northhing_runtime_ports::{AgentSessionListRequest, AgentSessionWorkspaceRequest};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

use super::schedule::CronToolScheduleOutput;
use super::trigger;

pub async fn resolve_workspace(workspace: &str, context: Option<&ToolUseContext>) -> NortHingResult<String> {
    trigger::validate_workspace_format(workspace, context).map_err(NortHingError::tool)?;

    if let Some(ctx) = context {
        if ctx.is_remote() {
            return ctx.resolve_workspace_tool_path(workspace.trim());
        }
    }

    let resolved = crate::agentic::tools::implementations::util::normalize_path(workspace.trim());
    let path = Path::new(&resolved);
    if !path.exists() {
        return Err(NortHingError::tool(format!("Workspace does not exist: {}", resolved)));
    }
    if !path.is_dir() {
        return Err(NortHingError::tool(format!(
            "Workspace is not a directory: {}",
            resolved
        )));
    }
    Ok(resolved)
}

pub async fn resolve_workspace_from_context(context: &ToolUseContext) -> NortHingResult<String> {
    let workspace = context.workspace_root().ok_or_else(|| {
        NortHingError::tool("workspace is required when the current workspace is unavailable".to_string())
    })?;
    resolve_workspace(workspace.to_string_lossy().as_ref(), Some(context)).await
}

pub async fn resolve_effective_workspace_for_session(
    session_id: &str,
    context: &ToolUseContext,
) -> NortHingResult<String> {
    if let Some(runtime) = agent_runtime_if_available()? {
        if let Some(resolved) = runtime
            .resolve_session_workspace_path(AgentSessionWorkspaceRequest {
                session_id: session_id.to_string(),
            })
            .await
            .map_err(|error| NortHingError::tool(CoreServiceAgentRuntime::runtime_error_message(error)))?
        {
            return Ok(resolved);
        }
    }

    if context.session_id.as_deref() == Some(session_id) {
        return resolve_workspace_from_context(context).await;
    }

    Err(NortHingError::tool(format!(
        "Unable to resolve workspace for session '{}'",
        session_id
    )))
}

pub fn resolve_effective_session_id(session_id: Option<&str>, context: &ToolUseContext) -> NortHingResult<String> {
    let resolved = match session_id {
        Some(session_id) => session_id.trim().to_string(),
        None => context.session_id.as_deref().unwrap_or_default().trim().to_string(),
    };

    trigger::validate_session_id(&resolved).map_err(NortHingError::tool)?;
    Ok(resolved)
}

pub async fn ensure_session_exists(workspace: &str, session_id: &str) -> NortHingResult<()> {
    let runtime = require_agent_runtime()?;
    let sessions = runtime
        .list_sessions(AgentSessionListRequest {
            workspace_path: workspace.to_string(),
        })
        .await
        .map_err(|error| NortHingError::tool(CoreServiceAgentRuntime::runtime_error_message(error)))?;
    if sessions.iter().any(|session| session.session_id == session_id) {
        return Ok(());
    }

    Err(NortHingError::NotFound(format!(
        "Session '{}' not found in workspace '{}'",
        session_id, workspace
    )))
}

pub fn agent_runtime_if_available() -> NortHingResult<Option<AgentRuntime>> {
    let Some(coordinator) = global_coordinator() else {
        return Ok(None);
    };
    CoreServiceAgentRuntime::agent_runtime(coordinator)
        .map(Some)
        .map_err(NortHingError::tool)
}

pub fn require_agent_runtime() -> NortHingResult<AgentRuntime> {
    agent_runtime_if_available()?.ok_or_else(|| NortHingError::tool("coordinator not initialized".to_string()))
}

pub fn serialize_job(job: &CronJob) -> NortHingResult<Value> {
    serde_json::to_value(CronToolJobOutput::try_from(job)?).map_err(|err| NortHingError::serialization(err.to_string()))
}

pub fn serialize_jobs(jobs: &[CronJob]) -> NortHingResult<Vec<Value>> {
    jobs.iter().map(serialize_job).collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CronToolJobStateOutput {
    pub next_run_at_ms: Option<i64>,
    pub pending_trigger_at_ms: Option<i64>,
    pub retry_at_ms: Option<i64>,
    pub last_trigger_at_ms: Option<i64>,
    pub last_enqueued_at_ms: Option<i64>,
    pub last_run_started_at_ms: Option<i64>,
    pub last_run_finished_at_ms: Option<i64>,
    pub last_duration_ms: Option<u64>,
    pub last_run_status: Option<crate::service::cron::CronJobRunStatus>,
    pub last_error: Option<String>,
    pub active_turn_id: Option<String>,
    pub consecutive_failures: u32,
    pub coalesced_run_count: u32,
}

impl From<&crate::service::cron::CronJobState> for CronToolJobStateOutput {
    fn from(state: &crate::service::cron::CronJobState) -> Self {
        Self {
            next_run_at_ms: state.next_run_at_ms,
            pending_trigger_at_ms: state.pending_trigger_at_ms,
            retry_at_ms: state.retry_at_ms,
            last_trigger_at_ms: state.last_trigger_at_ms,
            last_enqueued_at_ms: state.last_enqueued_at_ms,
            last_run_started_at_ms: state.last_run_started_at_ms,
            last_run_finished_at_ms: state.last_run_finished_at_ms,
            last_duration_ms: state.last_duration_ms,
            last_run_status: state.last_run_status,
            last_error: state.last_error.clone(),
            active_turn_id: state.active_turn_id.clone(),
            consecutive_failures: state.consecutive_failures,
            coalesced_run_count: state.coalesced_run_count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CronToolJobOutput {
    pub id: String,
    pub name: String,
    pub schedule: CronToolScheduleOutput,
    pub payload: String,
    pub enabled: bool,
    pub session_id: String,
    pub workspace_path: String,
    pub created_at_ms: i64,
    pub config_updated_at_ms: i64,
    pub updated_at_ms: i64,
    pub state: CronToolJobStateOutput,
}

impl TryFrom<&CronJob> for CronToolJobOutput {
    type Error = NortHingError;

    fn try_from(job: &CronJob) -> NortHingResult<Self> {
        Ok(Self {
            id: job.id.clone(),
            name: job.name.clone(),
            schedule: CronToolScheduleOutput::try_from(&job.schedule)?,
            payload: job.payload.text.clone(),
            enabled: job.enabled,
            session_id: job.session_id().unwrap_or_default().to_string(),
            workspace_path: job.workspace().workspace_path.clone(),
            created_at_ms: job.created_at_ms,
            config_updated_at_ms: job.config_updated_at_ms,
            updated_at_ms: job.updated_at_ms,
            state: CronToolJobStateOutput::from(&job.state),
        })
    }
}
