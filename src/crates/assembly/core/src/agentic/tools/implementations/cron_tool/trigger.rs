use crate::agentic::tools::framework::ToolUseContext;
use crate::service::cron::CronJobPayload;
use crate::util::errors::{NortHingError, NortHingResult};

pub fn validate_session_id(session_id: &str) -> Result<(), String> {
    if session_id.is_empty() {
        return Err("session_id cannot be empty".to_string());
    }
    if session_id == "." || session_id == ".." {
        return Err("session_id cannot be '.' or '..'".to_string());
    }
    if session_id.contains('/') || session_id.contains('\\') {
        return Err("session_id cannot contain path separators".to_string());
    }
    if !session_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err("session_id can only contain ASCII letters, numbers, '-' and '_'".to_string());
    }
    Ok(())
}

pub fn validate_job_id(job_id: &str) -> Result<(), String> {
    if job_id.trim().is_empty() {
        return Err("job_id cannot be empty".to_string());
    }
    Ok(())
}

pub fn validate_workspace_format(workspace: &str, context: Option<&ToolUseContext>) -> Result<(), String> {
    if workspace.trim().is_empty() {
        return Err("workspace cannot be empty".to_string());
    }
    let is_remote = context.map(|c| c.is_remote()).unwrap_or(false);
    if is_remote {
        if !crate::agentic::tools::workspace_paths::posix_style_path_is_absolute(workspace.trim()) {
            return Err("workspace must be an absolute POSIX path on the remote host".to_string());
        }
        return Ok(());
    }
    if !std::path::Path::new(workspace.trim()).is_absolute() {
        return Err("workspace must be an absolute path".to_string());
    }
    Ok(())
}

pub fn normalize_add_name(name: Option<String>) -> String {
    match name {
        Some(name) if !name.trim().is_empty() => name.trim().to_string(),
        _ => "Cron job".to_string(),
    }
}

pub fn normalize_optional_name(name: Option<String>) -> NortHingResult<Option<String>> {
    match name {
        Some(name) if name.trim().is_empty() => Err(NortHingError::tool(
            "patch.name cannot be empty when provided".to_string(),
        )),
        Some(name) => Ok(Some(name.trim().to_string())),
        None => Ok(None),
    }
}

pub fn validate_payload(payload: &str, field_name: &str) -> NortHingResult<()> {
    if payload.trim().is_empty() {
        return Err(NortHingError::tool(format!("{}.payload must not be empty", field_name)));
    }
    Ok(())
}

pub fn into_service_payload(payload: String) -> CronJobPayload {
    CronJobPayload { text: payload }
}
