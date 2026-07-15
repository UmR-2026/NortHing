use super::pipeline_post::*;
use super::pipeline_pre::*;
use super::pipeline_types::*;
use crate::agentic::core::ToolCall;
use crate::agentic::tools::pipeline::types::*;
use crate::agentic::tools::registry::ToolRegistry;
use crate::agentic::tools::tool_context_runtime::ToolUseContext;
use crate::util::errors::{NortHingError, NortHingResult};
use dashmap::DashMap;
use futures::future::join_all;
use std::sync::Arc;
use std::time::Instant;
use tool_runtime::pipeline::partition_tool_batches;
use tracing::{debug, info};

impl ToolPipeline {
    /// Execute multiple tool calls using partitioned mixed scheduling.
    ///
    /// Consecutive concurrency-safe calls are grouped into a single batch and
    /// run in parallel; each non-safe call forms its own batch and runs serially.
    /// Batches are executed in order so that write-after-read dependencies are
    /// respected while reads still benefit from parallelism.
    pub async fn execute_tools(
        &self,
        tool_calls: Vec<ToolCall>,
        context: ToolExecutionContext,
        options: ToolExecutionOptions,
    ) -> NortHingResult<Vec<ToolExecutionResult>> {
        if tool_calls.is_empty() {
            return Ok(vec![]);
        }

        info!("Executing tools: count={}", tool_calls.len());
        let tool_names: Vec<String> = tool_calls.iter().map(|tool_call| tool_call.tool_name.clone()).collect();

        // Determine concurrency safety for each tool call
        let concurrency_flags: Vec<bool> = {
            let registry = self.tool_registry.read().await;
            tool_calls
                .iter()
                .map(|tc| {
                    registry
                        .get_tool(&tc.tool_name)
                        .map(|tool| tool.is_concurrency_safe(Some(&tc.arguments)))
                        .unwrap_or(false)
                })
                .collect()
        };
        let concurrency_safe_count = concurrency_flags.iter().filter(|&&flag| flag).count();

        // Create tasks for all tool calls
        let mut task_ids = Vec::with_capacity(tool_calls.len());
        for tool_call in tool_calls {
            let task = ToolTask::new(tool_call, context.clone(), options.clone());
            let tool_id = self.state_manager.create_task(task).await;
            task_ids.push(tool_id);
        }

        if !options.allow_parallel {
            debug!(
                "Tool execution plan: total_tools={}, batches=1, concurrency_safe={}, non_concurrency_safe={}, allow_parallel=false, tools={}",
                task_ids.len(),
                concurrency_safe_count,
                task_ids.len().saturating_sub(concurrency_safe_count),
                tool_names.join(", ")
            );
            return self.execute_sequential(task_ids).await;
        }

        // Partition into batches of consecutive same-safety tool calls
        let batches = partition_tool_batches(&task_ids, &concurrency_flags);
        debug!(
            "Tool execution plan: total_tools={}, batches={}, concurrency_safe={}, non_concurrency_safe={}, allow_parallel=true, tools={}",
            task_ids.len(),
            batches.len(),
            concurrency_safe_count,
            task_ids.len().saturating_sub(concurrency_safe_count),
            tool_names.join(", ")
        );

        debug!(
            "Partitioned {} tools into {} batches for mixed execution",
            task_ids.len(),
            batches.len()
        );

        let mut all_results = Vec::with_capacity(task_ids.len());
        let mut batch_iter = batches.into_iter().enumerate().peekable();
        while let Some((batch_idx, batch)) = batch_iter.next() {
            let batch_context = batch
                .task_ids
                .first()
                .and_then(|task_id| self.state_manager.get_task(task_id))
                .map(|task| task.context);
            if batch_context
                .as_ref()
                .is_some_and(|context| self.should_interrupt_for_steering(context))
            {
                let remaining_task_ids = batch
                    .task_ids
                    .into_iter()
                    .chain(batch_iter.flat_map(|(_, batch)| batch.task_ids.into_iter()));
                all_results.extend(self.build_steering_interrupted_results(remaining_task_ids).await);
                break;
            }

            debug!(
                "Executing batch {}: {} tool(s), concurrent={}",
                batch_idx,
                batch.task_ids.len(),
                batch.is_concurrent
            );
            let batch_results = if batch.is_concurrent {
                self.execute_parallel(batch.task_ids).await?
            } else {
                self.execute_sequential(batch.task_ids).await?
            };
            all_results.extend(batch_results);
        }

        Ok(all_results)
    }
}
