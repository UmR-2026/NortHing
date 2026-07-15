//! Cron tool - manage scheduled jobs for agent sessions.
//!
//! Split into domain-specific sibling modules:
//! - `schedule`: cron expression parsing + schedule calculation
//! - `trigger`: input validation + normalization
//! - `execution`: workspace/session resolution + runtime interaction
//! - `format`: response formatting + output types
//! - `tests`: #[cfg(test)] mod tests

use crate::agentic::tools::framework::{
    Tool, ToolExposure, ToolRenderOptions, ToolResult, ToolUseContext, ValidationResult,
};
use crate::service::global_cron_service;
use crate::util::errors::{NortHingError, NortHingResult};
use async_trait::async_trait;
use chrono::{Local, SecondsFormat};
use serde::Deserialize;
use serde_json::{json, Value};

mod execution;
mod format;
mod schedule;
#[cfg(test)]
mod tests;
mod trigger;

#[derive(Debug, Clone, Deserialize)]
pub struct CronToolJobInput {
    pub name: Option<String>,
    pub schedule: schedule::CronToolScheduleInput,
    pub payload: String,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CronToolJobPatchInput {
    pub name: Option<String>,
    pub schedule: Option<schedule::CronToolScheduleInput>,
    pub payload: Option<String>,
    pub enabled: Option<bool>,
}

impl CronToolJobPatchInput {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.schedule.is_none() && self.payload.is_none() && self.enabled.is_none()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CronToolInput {
    pub action: schedule::CronAction,
    pub session_id: Option<String>,
    pub job: Option<CronToolJobInput>,
    pub patch: Option<CronToolJobPatchInput>,
    pub job_id: Option<String>,
}

/// Cron tool - manage scheduled jobs for agent sessions.
pub struct CronTool;

impl CronTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CronTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CronTool {
    fn name(&self) -> &str {
        "Cron"
    }

    async fn description(&self) -> NortHingResult<String> {
        Ok(r#"Manage scheduled jobs for agent sessions.

Defaults:
- "session_id": defaults to the current session for "list" and "add".

Actions:
- "get_time": Return the current local time including timezone information.
- "list": List all jobs for the effective session scope.
- "add": Create a job. Requires "job". When "job.name" is omitted, uses "Cron job".
- "update": Update a job. Requires "job_id" and "patch".
- "remove": Delete a job. Requires "job_id".
- "run": Trigger a job immediately. Requires "job_id".

Job schema for "add":
{
  "name": "string (optional)",
  "schedule": { ... },
  "payload": "string (sent to the target session as a user message)",
  "enabled": true | false
}

Schedule schema:
- One-shot at absolute time:
  { "kind": "at", "at": "2026-03-17T12:00:00+08:00" }
- Recurring interval:
  { "kind": "every", "every": 3600, "anchor": "2026-03-17T12:00:00+08:00" }
  - "every" is in seconds.
  - "anchor" is optional and uses the same ISO-8601 format as "at". Defaults to the current time.
- Cron expression:
  { "kind": "cron", "expr": "0 9 * * 1-5", "tz": "Asia/Shanghai" }
  - "tz" is optional. Defaults to the local timezone.

Patch schema for "update":
- Same fields as "job", but every field is optional."#
            .to_string())
    }

    fn short_description(&self) -> String {
        "Manage scheduled jobs for agent sessions.".to_string()
    }

    fn default_exposure(&self) -> ToolExposure {
        ToolExposure::Collapsed
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "Optional target session ID. Defaults to the current session for list/add."
                },
                "action": {
                    "type": "string",
                    "enum": ["get_time", "list", "add", "update", "remove", "run"],
                    "description": "Cron action to perform."
                },
                "job": {
                    "type": "object",
                    "description": "Required for add.",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Optional job name. Defaults to 'Cron job'."
                        },
                        "schedule": {
                            "type": "object",
                            "description": "Required schedule definition. Use { \"kind\": \"at\", \"at\": \"<ISO-8601>\" }, { \"kind\": \"every\", \"every\": <seconds>, \"anchor\": \"<optional ISO-8601>\" }, or { \"kind\": \"cron\", \"expr\": \"<cron-expression>\", \"tz\": \"<optional timezone>\" }. anchor defaults to the current time. tz defaults to the local timezone."
                        },
                        "payload": {
                            "type": "string",
                            "description": "Required execution payload text. It will be sent to the target session as a user message."
                        },
                        "enabled": {
                            "type": "boolean",
                            "description": "Optional enabled flag. Defaults to true."
                        }
                    },
                    "required": ["schedule", "payload"],
                    "additionalProperties": false
                },
                "patch": {
                    "type": "object",
                    "description": "Required for update. Same fields as job, but all optional.",
                    "properties": {
                        "name": {
                            "type": "string"
                        },
                        "schedule": {
                            "type": "object"
                        },
                        "payload": {
                            "type": "string",
                            "description": "Optional updated payload text. It will be sent to the target session as a user message."
                        },
                        "enabled": {
                            "type": "boolean"
                        }
                    },
                    "additionalProperties": false
                },
                "job_id": {
                    "type": "string",
                    "description": "Required for update, remove, and run."
                }
            },
            "required": ["action"],
            "additionalProperties": false
        })
    }

    fn is_readonly(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self, input: Option<&Value>) -> bool {
        let Some(input) = input else {
            return false;
        };
        let Some(action) = input.get("action").and_then(|value| value.as_str()) else {
            return false;
        };
        matches!(action, "get_time" | "list")
    }

    fn needs_permissions(&self, _input: Option<&Value>) -> bool {
        false
    }

    async fn validate_input(&self, input: &Value, context: Option<&ToolUseContext>) -> ValidationResult {
        let parsed: CronToolInput = match serde_json::from_value(input.clone()) {
            Ok(value) => value,
            Err(err) => {
                return ValidationResult {
                    result: false,
                    message: Some(format!("Invalid input: {}", err)),
                    error_code: Some(400),
                    meta: None,
                };
            }
        };

        if let Some(session_id) = parsed.session_id.as_deref() {
            if let Err(message) = trigger::validate_session_id(session_id.trim()) {
                return ValidationResult {
                    result: false,
                    message: Some(message),
                    error_code: Some(400),
                    meta: None,
                };
            }
        }

        match parsed.action {
            schedule::CronAction::GetTime => ValidationResult::default(),
            schedule::CronAction::List => {
                let has_effective_session = parsed.session_id.is_some()
                    || context
                        .and_then(|tool_context| tool_context.session_id.as_deref())
                        .is_some();
                if !has_effective_session {
                    return ValidationResult {
                        result: false,
                        message: Some(
                            "session_id is required for list when the current session is unavailable".to_string(),
                        ),
                        error_code: Some(400),
                        meta: None,
                    };
                }
                if parsed.session_id.is_none()
                    && context.and_then(|tool_context| tool_context.workspace_root()).is_none()
                {
                    return ValidationResult {
                        result: false,
                        message: Some(
                            "the current workspace is required for list when session_id is omitted".to_string(),
                        ),
                        error_code: Some(400),
                        meta: None,
                    };
                }
                ValidationResult::default()
            }
            schedule::CronAction::Add => {
                let Some(job) = parsed.job.as_ref() else {
                    return ValidationResult {
                        result: false,
                        message: Some("job is required for add".to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                };

                if let Err(error) = trigger::validate_payload(&job.payload, "job") {
                    return ValidationResult {
                        result: false,
                        message: Some(error.to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                }
                if let Err(error) = job.schedule.to_service_schedule("job.schedule") {
                    return ValidationResult {
                        result: false,
                        message: Some(error.to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                }

                let has_effective_session = parsed.session_id.is_some()
                    || context
                        .and_then(|tool_context| tool_context.session_id.as_deref())
                        .is_some();
                if !has_effective_session {
                    return ValidationResult {
                        result: false,
                        message: Some(
                            "session_id is required for add when the current session is unavailable".to_string(),
                        ),
                        error_code: Some(400),
                        meta: None,
                    };
                }
                if parsed.session_id.is_none()
                    && context.and_then(|tool_context| tool_context.workspace_root()).is_none()
                {
                    return ValidationResult {
                        result: false,
                        message: Some(
                            "the current workspace is required for add when session_id is omitted".to_string(),
                        ),
                        error_code: Some(400),
                        meta: None,
                    };
                }

                ValidationResult::default()
            }
            schedule::CronAction::Update => {
                let Some(job_id) = parsed.job_id.as_deref() else {
                    return ValidationResult {
                        result: false,
                        message: Some("job_id is required for update".to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                };
                if let Err(message) = trigger::validate_job_id(job_id) {
                    return ValidationResult {
                        result: false,
                        message: Some(message),
                        error_code: Some(400),
                        meta: None,
                    };
                }

                let Some(patch) = parsed.patch.as_ref() else {
                    return ValidationResult {
                        result: false,
                        message: Some("patch is required for update".to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                };
                if patch.is_empty() {
                    return ValidationResult {
                        result: false,
                        message: Some("patch must include at least one field".to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                }
                if let Some(name) = patch.name.as_deref() {
                    if name.trim().is_empty() {
                        return ValidationResult {
                            result: false,
                            message: Some("patch.name cannot be empty when provided".to_string()),
                            error_code: Some(400),
                            meta: None,
                        };
                    }
                }
                if let Some(payload) = patch.payload.as_ref() {
                    if let Err(error) = trigger::validate_payload(payload, "patch") {
                        return ValidationResult {
                            result: false,
                            message: Some(error.to_string()),
                            error_code: Some(400),
                            meta: None,
                        };
                    };
                }
                if let Some(schedule) = patch.schedule.as_ref() {
                    if let Err(error) = schedule.to_service_schedule("patch.schedule") {
                        return ValidationResult {
                            result: false,
                            message: Some(error.to_string()),
                            error_code: Some(400),
                            meta: None,
                        };
                    }
                }
                ValidationResult::default()
            }
            schedule::CronAction::Remove => {
                let Some(job_id) = parsed.job_id.as_deref() else {
                    return ValidationResult {
                        result: false,
                        message: Some("job_id is required for remove".to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                };
                if let Err(message) = trigger::validate_job_id(job_id) {
                    return ValidationResult {
                        result: false,
                        message: Some(message),
                        error_code: Some(400),
                        meta: None,
                    };
                }
                ValidationResult::default()
            }
            schedule::CronAction::Run => {
                let Some(job_id) = parsed.job_id.as_deref() else {
                    return ValidationResult {
                        result: false,
                        message: Some("job_id is required for run".to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                };
                if let Err(message) = trigger::validate_job_id(job_id) {
                    return ValidationResult {
                        result: false,
                        message: Some(message),
                        error_code: Some(400),
                        meta: None,
                    };
                }
                ValidationResult::default()
            }
        }
    }

    fn render_tool_use_message(&self, input: &Value, _options: &ToolRenderOptions) -> String {
        let action = input
            .get("action")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        let job_id = input.get("job_id").and_then(|value| value.as_str()).unwrap_or("auto");

        match action {
            "get_time" => "Get current ISO-8601 time".to_string(),
            "list" => "List scheduled jobs".to_string(),
            "add" => "Create scheduled job".to_string(),
            "update" => format!("Update scheduled job {}", job_id),
            "remove" => format!("Delete scheduled job {}", job_id),
            "run" => format!("Run scheduled job {}", job_id),
            _ => "Manage scheduled jobs".to_string(),
        }
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        let params: CronToolInput = serde_json::from_value(input.clone())
            .map_err(|err| NortHingError::tool(format!("Invalid input: {}", err)))?;

        match params.action {
            schedule::CronAction::GetTime => {
                let now = Local::now();
                let iso = now.to_rfc3339_opts(SecondsFormat::Secs, false);
                let result_for_assistant = format!("Current local time: {}", iso);

                Ok(vec![ToolResult::Result {
                    data: json!({
                        "success": true,
                        "action": "get_time",
                        "now": iso,
                    }),
                    result_for_assistant: Some(result_for_assistant),
                    image_attachments: None,
                }])
            }
            schedule::CronAction::List => {
                let cron_service = global_cron_service()
                    .ok_or_else(|| NortHingError::tool("cron service not initialized".to_string()))?;
                let session_id = execution::resolve_effective_session_id(params.session_id.as_deref(), context)?;
                let workspace = execution::resolve_effective_workspace_for_session(&session_id, context).await?;
                let mut jobs = cron_service
                    .list_jobs_filtered(
                        Some(&workspace),
                        None,
                        None,
                        Some(&session_id),
                        Some(crate::service::cron::CronJobTargetKind::Session),
                    )
                    .await;
                jobs.sort_by(|left, right| {
                    left.created_at_ms
                        .cmp(&right.created_at_ms)
                        .then_with(|| left.id.cmp(&right.id))
                });
                let serialized_jobs = execution::serialize_jobs(&jobs)?;

                let result_for_assistant = format::build_list_result_for_assistant(&workspace, &session_id, &jobs);

                Ok(vec![ToolResult::Result {
                    data: json!({
                        "success": true,
                        "action": "list",
                        "workspace": workspace,
                        "session_id": session_id,
                        "count": jobs.len(),
                        "jobs": serialized_jobs,
                    }),
                    result_for_assistant: Some(result_for_assistant),
                    image_attachments: None,
                }])
            }
            schedule::CronAction::Add => {
                let cron_service = global_cron_service()
                    .ok_or_else(|| NortHingError::tool("cron service not initialized".to_string()))?;
                let session_id = execution::resolve_effective_session_id(params.session_id.as_deref(), context)?;
                let workspace = execution::resolve_effective_workspace_for_session(&session_id, context).await?;
                let job = params
                    .job
                    .ok_or_else(|| NortHingError::tool("job is required for add".to_string()))?;

                trigger::validate_payload(&job.payload, "job")?;
                execution::ensure_session_exists(&workspace, &session_id).await?;

                let created = cron_service
                    .create_job(crate::service::cron::CreateCronJobRequest {
                        name: trigger::normalize_add_name(job.name),
                        schedule: job.schedule.to_service_schedule("job.schedule")?,
                        payload: trigger::into_service_payload(job.payload),
                        enabled: job.enabled.unwrap_or(true),
                        target: crate::service::cron::CronJobTarget::Session {
                            session_id: session_id.clone(),
                            workspace: crate::service::cron::CronWorkspaceRef {
                                workspace_id: None,
                                workspace_path: workspace.clone(),
                                remote_connection_id: None,
                                remote_ssh_host: None,
                            },
                        },
                    })
                    .await?;
                let serialized_job = execution::serialize_job(&created)?;
                let result_for_assistant = format!(
                    "Created scheduled job '{}' ({}) for session '{}' in workspace '{}'.",
                    created.name,
                    created.id,
                    created.session_id().unwrap_or(""),
                    created.workspace().workspace_path
                );

                Ok(vec![ToolResult::Result {
                    data: json!({
                        "success": true,
                        "action": "add",
                        "workspace": workspace,
                        "session_id": session_id,
                        "job": serialized_job,
                    }),
                    result_for_assistant: Some(result_for_assistant),
                    image_attachments: None,
                }])
            }
            schedule::CronAction::Update => {
                let cron_service = global_cron_service()
                    .ok_or_else(|| NortHingError::tool("cron service not initialized".to_string()))?;
                let job_id = params
                    .job_id
                    .ok_or_else(|| NortHingError::tool("job_id is required for update".to_string()))?;
                trigger::validate_job_id(&job_id).map_err(NortHingError::tool)?;
                let patch = params
                    .patch
                    .ok_or_else(|| NortHingError::tool("patch is required for update".to_string()))?;
                if patch.is_empty() {
                    return Err(NortHingError::tool("patch must include at least one field".to_string()));
                }
                if let Some(payload) = patch.payload.as_ref() {
                    trigger::validate_payload(payload, "patch")?;
                }

                let updated = cron_service
                    .update_job(
                        &job_id,
                        crate::service::cron::UpdateCronJobRequest {
                            name: trigger::normalize_optional_name(patch.name)?,
                            schedule: patch
                                .schedule
                                .as_ref()
                                .map(|value| value.to_service_schedule("patch.schedule"))
                                .transpose()?,
                            payload: patch.payload.map(trigger::into_service_payload),
                            enabled: patch.enabled,
                            target: None,
                        },
                    )
                    .await?;
                let serialized_job = execution::serialize_job(&updated)?;
                let result_for_assistant = format!("Updated scheduled job '{}' ({})", updated.name, updated.id);

                Ok(vec![ToolResult::Result {
                    data: json!({
                        "success": true,
                        "action": "update",
                        "job_id": job_id,
                        "job": serialized_job,
                    }),
                    result_for_assistant: Some(result_for_assistant),
                    image_attachments: None,
                }])
            }
            schedule::CronAction::Remove => {
                let cron_service = global_cron_service()
                    .ok_or_else(|| NortHingError::tool("cron service not initialized".to_string()))?;
                let job_id = params
                    .job_id
                    .ok_or_else(|| NortHingError::tool("job_id is required for remove".to_string()))?;
                trigger::validate_job_id(&job_id).map_err(NortHingError::tool)?;

                let deleted = cron_service.delete_job(&job_id).await?;
                let result_for_assistant = if deleted {
                    format!("Deleted scheduled job '{}'.", job_id)
                } else {
                    format!("No scheduled job found for '{}'.", job_id)
                };

                Ok(vec![ToolResult::Result {
                    data: json!({
                        "success": true,
                        "action": "remove",
                        "job_id": job_id,
                        "deleted": deleted,
                    }),
                    result_for_assistant: Some(result_for_assistant),
                    image_attachments: None,
                }])
            }
            schedule::CronAction::Run => {
                let cron_service = global_cron_service()
                    .ok_or_else(|| NortHingError::tool("cron service not initialized".to_string()))?;
                let job_id = params
                    .job_id
                    .ok_or_else(|| NortHingError::tool("job_id is required for run".to_string()))?;
                trigger::validate_job_id(&job_id).map_err(NortHingError::tool)?;

                let updated = cron_service.run_job_now(&job_id).await?;
                let serialized_job = execution::serialize_job(&updated)?;
                let result_for_assistant = format!(
                    "Triggered scheduled job '{}' ({}) for immediate execution.",
                    updated.name, updated.id
                );

                Ok(vec![ToolResult::Result {
                    data: json!({
                        "success": true,
                        "action": "run",
                        "job_id": job_id,
                        "job": serialized_job,
                    }),
                    result_for_assistant: Some(result_for_assistant),
                    image_attachments: None,
                }])
            }
        }
    }
}
