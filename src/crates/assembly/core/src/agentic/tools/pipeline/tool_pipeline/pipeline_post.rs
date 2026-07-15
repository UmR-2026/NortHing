use super::pipeline_logging::{classify_tool_error, elapsed_ms_since};
use crate::agentic::core::ToolResult as ModelToolResult;
use crate::agentic::tools::framework::ToolResult as FrameworkToolResult;
use crate::agentic::tools::pipeline::types::*;
use crate::agentic::tools::tool_result_storage;
use crate::util::errors::NortHingError;
use northhing_agent_tools::{
    build_tool_call_truncation_recovery_notice, build_tool_execution_error_presentation,
    build_user_steering_interrupted_presentation, render_tool_result_for_assistant,
    truncate_raw_tool_arguments_preview, truncate_tool_arguments_preview,
};
use std::sync::Arc;
use tracing::warn;

pub fn convert_tool_result(framework_result: FrameworkToolResult, tool_id: &str, tool_name: &str) -> ModelToolResult {
    match framework_result {
        FrameworkToolResult::Result {
            data,
            result_for_assistant,
            image_attachments,
        } => {
            let assistant_text =
                result_for_assistant.or_else(|| Some(render_tool_result_for_assistant(tool_name, &data)));

            ModelToolResult {
                tool_id: tool_id.to_string(),
                tool_name: tool_name.to_string(),
                result: data,
                result_for_assistant: assistant_text,
                is_error: false,
                duration_ms: None,
                image_attachments,
            }
        }
        FrameworkToolResult::Progress { content, .. } => {
            let assistant_text = Some(render_tool_result_for_assistant(tool_name, &content));

            ModelToolResult {
                tool_id: tool_id.to_string(),
                tool_name: tool_name.to_string(),
                result: content,
                result_for_assistant: assistant_text,
                is_error: false,
                duration_ms: None,
                image_attachments: None,
            }
        }
        FrameworkToolResult::StreamChunk { data, .. } => {
            let assistant_text = Some(render_tool_result_for_assistant(tool_name, &data));

            ModelToolResult {
                tool_id: tool_id.to_string(),
                tool_name: tool_name.to_string(),
                result: data,
                result_for_assistant: assistant_text,
                is_error: false,
                duration_ms: None,
                image_attachments: None,
            }
        }
    }
}

pub fn convert_to_framework_result(model_result: &ModelToolResult) -> FrameworkToolResult {
    FrameworkToolResult::Result {
        data: model_result.result.clone(),
        result_for_assistant: model_result.result_for_assistant.clone(),
        image_attachments: model_result.image_attachments.clone(),
    }
}

pub fn build_error_execution_result(
    task_id: &str,
    task: Option<ToolTask>,
    error: &NortHingError,
) -> ToolExecutionResult {
    let (tool_id, tool_name, execution_time_ms, provided_arguments) = if let Some(task) = task {
        let preview = task
            .tool_call
            .raw_arguments
            .as_deref()
            .map(truncate_raw_tool_arguments_preview)
            .unwrap_or_else(|| truncate_tool_arguments_preview(&task.tool_call.arguments));
        (
            task.tool_call.tool_id,
            task.tool_call.tool_name,
            elapsed_ms_since(task.created_at),
            Some(preview),
        )
    } else {
        warn!("Task not found in state manager: {}", task_id);
        (task_id.to_string(), "unknown".to_string(), 0, None)
    };
    let error_message = error.to_string();
    let category = classify_tool_error(error);
    let presentation =
        build_tool_execution_error_presentation(&tool_name, category, &error_message, provided_arguments);

    ToolExecutionResult {
        tool_id: tool_id.clone(),
        tool_name: tool_name.clone(),
        result: ModelToolResult {
            tool_id,
            tool_name,
            result: presentation.result_json,
            result_for_assistant: Some(presentation.result_for_assistant),
            is_error: true,
            duration_ms: Some(execution_time_ms),
            image_attachments: None,
        },
        execution_time_ms,
    }
}

pub fn build_user_steering_interrupted_result(task_id: &str, task: Option<ToolTask>) -> ToolExecutionResult {
    let (tool_id, tool_name, execution_time_ms) = if let Some(task) = task {
        (
            task.tool_call.tool_id,
            task.tool_call.tool_name,
            elapsed_ms_since(task.created_at),
        )
    } else {
        warn!("Task not found while building steering-interrupted result: {}", task_id);
        (task_id.to_string(), "unknown".to_string(), 0)
    };

    let presentation = build_user_steering_interrupted_presentation(&tool_name);

    ToolExecutionResult {
        tool_id: tool_id.clone(),
        tool_name: tool_name.clone(),
        result: ModelToolResult {
            tool_id,
            tool_name,
            result: presentation.result_json,
            result_for_assistant: Some(presentation.result_for_assistant),
            is_error: true,
            duration_ms: Some(execution_time_ms),
            image_attachments: None,
        },
        execution_time_ms,
    }
}

pub fn build_success_result(
    tool_id: String,
    tool_name: String,
    result: ModelToolResult,
    duration_ms: u64,
) -> ToolExecutionResult {
    ToolExecutionResult {
        tool_id,
        tool_name,
        result,
        execution_time_ms: duration_ms,
    }
}
