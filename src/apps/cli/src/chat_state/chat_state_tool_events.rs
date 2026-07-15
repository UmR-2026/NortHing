// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chat state tool event handling
//!
//! `handle_tool_event` — dispatches `ToolEventData` variants into
//! state updates: tool card updates, permission/question prompt lifecycle,
//! AskUserQuestion auto-prompt creation, and HmosCompilation failure detection.

use northhing_events::ToolEventData;

use crate::ui::permission::PermissionPrompt;
use crate::ui::question::QuestionPrompt;

use super::display_types::{ToolDisplayState, ToolDisplayStatus};
use super::helpers::extract_fallback_summary;

use super::chat_state_core::ChatState;

impl ChatState {
    /// Handle a tool event.
    /// New tools are appended to current_flow_items in chronological order.
    /// Existing tools are updated in-place via tool_index for O(1) lookup.
    pub fn handle_tool_event(&mut self, tool_event: &ToolEventData) {
        match tool_event {
            ToolEventData::EarlyDetected { tool_id, tool_name } => {
                self.insert_or_update_tool(
                    tool_id,
                    |_existing| {
                        // Should not exist yet, but handle gracefully
                    },
                    || ToolDisplayState {
                        tool_id: tool_id.clone(),
                        tool_name: tool_name.clone(),
                        parameters: serde_json::Value::Null,
                        status: ToolDisplayStatus::EarlyDetected,
                        result: None,
                        progress_message: None,
                        duration_ms: None,
                        metadata: None,
                        subagent_progress: None,
                    },
                );
                self.rebuild_streaming_message();
            }

            ToolEventData::ParamsPartial { tool_id, params, .. } => {
                self.update_tool(tool_id, |tool| {
                    // Only update status if not yet in an advanced execution state.
                    // Due to priority queue ordering, ParamsPartial (Normal priority) may
                    // arrive after Started (High priority), which would incorrectly
                    // revert the status from Running back to ParamsPartial.
                    if !tool.status.is_execution_phase() {
                        tool.status = ToolDisplayStatus::ParamsPartial;
                    }
                    tool.progress_message = Some(params.clone());
                });
                self.rebuild_streaming_message();
            }

            ToolEventData::Queued { tool_id, position, .. } => {
                self.update_tool(tool_id, |tool| {
                    if !tool.status.is_execution_phase() {
                        tool.status = ToolDisplayStatus::Queued;
                    }
                    tool.progress_message = Some(format!("Queue position: {}", position));
                });
                self.rebuild_streaming_message();
            }

            ToolEventData::Waiting {
                tool_id, dependencies, ..
            } => {
                self.update_tool(tool_id, |tool| {
                    if !tool.status.is_execution_phase() {
                        tool.status = ToolDisplayStatus::Waiting;
                    }
                    tool.progress_message = Some(format!("Waiting for: {:?}", dependencies));
                });
                self.rebuild_streaming_message();
            }

            ToolEventData::Started {
                tool_id,
                tool_name,
                params,
                timeout_seconds: _,
            } => {
                let params_for_update = params.clone();
                let params_for_create = params.clone();
                let tool_name_clone = tool_name.clone();
                self.insert_or_update_tool(
                    tool_id,
                    |tool| {
                        tool.status = ToolDisplayStatus::Running;
                        tool.parameters = params_for_update;
                    },
                    || ToolDisplayState {
                        tool_id: tool_id.clone(),
                        tool_name: tool_name_clone,
                        parameters: params_for_create,
                        status: ToolDisplayStatus::Running,
                        result: None,
                        progress_message: None,
                        duration_ms: None,
                        metadata: None,
                        subagent_progress: None,
                    },
                );
                self.metadata.tool_calls += 1;

                // Auto-create question prompt for AskUserQuestion tool
                if tool_name == "AskUserQuestion" {
                    if let Some(prompt) = QuestionPrompt::from_params(tool_id.clone(), params) {
                        self.question_prompt = Some(prompt);
                    }
                }

                self.rebuild_streaming_message();
            }

            ToolEventData::Progress { tool_id, message, .. } => {
                self.update_tool(tool_id, |tool| {
                    tool.progress_message = Some(message.clone());
                });
                self.rebuild_streaming_message();
            }

            ToolEventData::Streaming {
                tool_id,
                chunks_received,
                ..
            } => {
                self.update_tool(tool_id, |tool| {
                    tool.status = ToolDisplayStatus::Streaming;
                    tool.progress_message = Some(format!("Received {} chunks", chunks_received));
                });
                self.rebuild_streaming_message();
            }

            ToolEventData::ConfirmationNeeded {
                tool_id,
                tool_name,
                params,
            } => {
                self.update_tool(tool_id, |tool| {
                    tool.status = ToolDisplayStatus::ConfirmationNeeded;
                    tool.progress_message = Some("Waiting for user confirmation".to_string());
                });
                // Auto-create permission prompt for user interaction
                self.permission_prompt = Some(PermissionPrompt::new(
                    tool_id.clone(),
                    tool_name.clone(),
                    params.clone(),
                ));
                self.rebuild_streaming_message();
            }

            ToolEventData::Confirmed { tool_id, .. } => {
                self.update_tool(tool_id, |tool| {
                    tool.status = ToolDisplayStatus::Confirmed;
                });
                // Clear permission prompt if it matches this tool
                if self.permission_prompt.as_ref().map(|p| &p.tool_id) == Some(tool_id) {
                    self.permission_prompt = None;
                }
                self.rebuild_streaming_message();
            }

            ToolEventData::Rejected { tool_id, .. } => {
                self.update_tool(tool_id, |tool| {
                    tool.status = ToolDisplayStatus::Rejected;
                    tool.result = Some("User rejected execution".to_string());
                });
                // Clear permission prompt if it matches this tool
                if self.permission_prompt.as_ref().map(|p| &p.tool_id) == Some(tool_id) {
                    self.permission_prompt = None;
                }
                self.rebuild_streaming_message();
            }

            ToolEventData::Completed {
                tool_id,
                tool_name,
                result,
                result_for_assistant,
                duration_ms,
                ..
            } => {
                // Prefer result_for_assistant from tool, fallback to extracting from JSON
                let result_str = result_for_assistant
                    .clone()
                    .unwrap_or_else(|| extract_fallback_summary(result));
                let metadata = result.clone();
                let dur = *duration_ms;
                self.update_tool(tool_id, |tool| {
                    let is_hmos_failed = tool_name == "HmosCompilation"
                        && result.get("success").and_then(|v| v.as_bool()) == Some(false);
                    tool.status = if is_hmos_failed {
                        ToolDisplayStatus::Failed
                    } else {
                        ToolDisplayStatus::Success
                    };
                    tool.result = Some(result_str);
                    tool.metadata = Some(metadata);
                    tool.duration_ms = Some(dur);
                });
                // Clear question prompt if this tool completed
                if self.question_prompt.as_ref().map(|p| &p.tool_id) == Some(tool_id) {
                    self.question_prompt = None;
                }
                self.rebuild_streaming_message();
            }

            ToolEventData::Failed { tool_id, error, .. } => {
                let err = error.clone();
                self.update_tool(tool_id, |tool| {
                    tool.status = ToolDisplayStatus::Failed;
                    tool.result = Some(err);
                });
                // Clear question prompt if this tool failed
                if self.question_prompt.as_ref().map(|p| &p.tool_id) == Some(tool_id) {
                    self.question_prompt = None;
                }
                self.rebuild_streaming_message();
            }

            ToolEventData::Cancelled { tool_id, reason, .. } => {
                let rsn = reason.clone();
                self.update_tool(tool_id, |tool| {
                    tool.status = ToolDisplayStatus::Cancelled;
                    tool.result = Some(rsn);
                });
                // Clear question prompt if this tool was cancelled
                if self.question_prompt.as_ref().map(|p| &p.tool_id) == Some(tool_id) {
                    self.question_prompt = None;
                }
                self.rebuild_streaming_message();
            }

            // StreamChunk and other variants we don't need to display
            _ => {}
        }
    }
}
