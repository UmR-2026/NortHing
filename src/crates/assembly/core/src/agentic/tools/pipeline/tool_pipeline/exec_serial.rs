use super::pipeline_post::*;
use super::pipeline_pre::*;
use super::pipeline_types::*;
use crate::agentic::core::ToolResult as ModelToolResult;
use crate::agentic::tools::framework::ToolResult as FrameworkToolResult;
use crate::agentic::tools::pipeline::types::*;
use crate::agentic::tools::registry::ToolRegistry;
use crate::agentic::tools::tool_context_runtime::ToolUseContext;
use crate::agentic::tools::tool_result_storage;
use crate::util::errors::{NortHingError, NortHingResult};
use std::sync::Arc;
use tracing::error;

impl ToolPipeline {
    pub(crate) async fn execute_sequential(&self, task_ids: Vec<String>) -> NortHingResult<Vec<ToolExecutionResult>> {
        let mut results = Vec::new();

        let mut task_iter = task_ids.into_iter().peekable();
        while let Some(task_id) = task_iter.next() {
            let task = self.state_manager.get_task(&task_id);

            if task
                .as_ref()
                .is_some_and(|task| self.should_interrupt_for_steering(&task.context))
            {
                let remaining_task_ids = std::iter::once(task_id).chain(task_iter);
                results.extend(self.build_steering_interrupted_results(remaining_task_ids).await);
                break;
            }

            match self.execute_single_tool(task_id.clone()).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    error!("Tool execution failed: error={}", e);
                    let error_result =
                        build_error_execution_result(&task_id, self.state_manager.get_task(&task_id), &e);
                    results.push(error_result);
                }
            }
        }

        Ok(results)
    }
}
