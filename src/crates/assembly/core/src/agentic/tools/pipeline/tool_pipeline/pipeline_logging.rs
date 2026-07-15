use crate::util::errors::NortHingError;
use std::time::SystemTime;
use tool_runtime::pipeline::{should_retry_tool_attempt, ToolRetryAttemptFacts};

pub fn elapsed_ms_since(time: SystemTime) -> u64 {
    time.elapsed()
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

pub fn classify_tool_error(error: &NortHingError) -> &'static str {
    match error {
        NortHingError::Validation(_) => "invalid_arguments",
        NortHingError::Cancelled(_) => "cancelled",
        NortHingError::Timeout(_) => "timeout",
        NortHingError::NotFound(_) => "not_found",
        _ => "execution_error",
    }
}

pub fn classify_tool_retry_error(error: &NortHingError) -> tool_runtime::pipeline::ToolExecutionErrorClass {
    if should_retry_tool_error(error) {
        tool_runtime::pipeline::ToolExecutionErrorClass::Retryable
    } else {
        tool_runtime::pipeline::ToolExecutionErrorClass::Terminal
    }
}

pub fn should_retry_tool_error(error: &NortHingError) -> bool {
    matches!(
        error,
        NortHingError::Timeout(_)
            | NortHingError::Io(_)
            | NortHingError::Http(_)
            | NortHingError::Service(_)
            | NortHingError::MCPError(_)
            | NortHingError::ProcessError(_)
            | NortHingError::Other(_)
    )
}
