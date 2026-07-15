//! Response formatting helpers for the agentic::coordination runtime ports.

use crate::util::errors::NortHingError;

/// Format a `<result>`/`<partial_result>`/`<error>` block describing
/// how a background subagent completed.
pub(crate) fn format_background_subagent_delivery_text(
    background_task_id: &str,
    agent_type: &str,
    outcome: Result<&super::coordinator::SubagentResult, &NortHingError>,
) -> String {
    match outcome {
        Ok(result) => {
            if result.is_partial_timeout() {
                format!(
                    "Background subagent '{}' (background_task_id='{}') completed with partial timeout result:\n<partial_result status=\"partial_timeout\">\n{}\n</partial_result>",
                    agent_type, background_task_id, result.text
                )
            } else {
                format!(
                    "Background subagent '{}' (background_task_id='{}') completed successfully:\n<result>\n{}\n</result>",
                    agent_type, background_task_id, result.text
                )
            }
        }
        Err(error) => {
            format!(
                "Background subagent '{}' (background_task_id='{}') failed before producing a final result.\nError: {}",
                agent_type, background_task_id, error
            )
        }
    }
}

/// Short description of how a background subagent completed.
pub(crate) fn format_background_subagent_display_text(
    outcome: Result<&super::coordinator::SubagentResult, &NortHingError>,
) -> String {
    match outcome {
        Ok(result) => {
            if result.is_partial_timeout() {
                "Background subagent completed with a partial timeout result.".to_string()
            } else {
                "Background subagent completed successfully.".to_string()
            }
        }
        Err(_) => "Background subagent failed before producing a final result.".to_string(),
    }
}
