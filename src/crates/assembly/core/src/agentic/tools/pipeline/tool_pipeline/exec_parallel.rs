use super::pipeline_post::*;
use super::pipeline_types::*;
use crate::agentic::core::ToolResult as ModelToolResult;
use crate::agentic::tools::framework::ToolResult as FrameworkToolResult;
use crate::agentic::tools::pipeline::types::*;
use crate::agentic::tools::registry::ToolRegistry;
use crate::agentic::tools::tool_context_runtime::ToolUseContext;
use crate::agentic::tools::tool_result_storage;
use crate::util::errors::{NortHingError, NortHingResult};
use futures::future::join_all;
use std::sync::Arc;
use tracing::error;

impl ToolPipeline {
    pub(crate) async fn execute_parallel(&self, task_ids: Vec<String>) -> NortHingResult<Vec<ToolExecutionResult>> {
        let futures: Vec<_> = task_ids.iter().map(|id| self.execute_single_tool(id.clone())).collect();

        let results = join_all(futures).await;

        // Collect results, including failed results
        let mut all_results = Vec::new();
        for (idx, result) in results.into_iter().enumerate() {
            match result {
                Ok(r) => all_results.push(r),
                Err(e) => {
                    error!("Tool execution failed: error={}", e);
                    let task_id = &task_ids[idx];
                    let error_result = build_error_execution_result(task_id, self.state_manager.get_task(task_id), &e);
                    all_results.push(error_result);
                }
            }
        }

        Ok(all_results)
    }
}
