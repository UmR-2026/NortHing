//! Method bodies for `PendingToolCall` and `PendingToolCalls`.
//!
//! This sibling owns the behavioral logic that turns streaming deltas into
//! finalized tool calls:
//!
//! * [`PendingToolCall`] — per-call state machine: argument buffering, Git
//!   command normalization, and `finalize` (which handles parse errors,
//!   truncation repair, and write-vs-shell classification).
//! * [`PendingToolCalls`] — collection-level state machine: route deltas to
//!   the right pending slot, emit `EarlyDetectedToolCall` /
//!   `ToolCallParamsChunk` / `FinalizedToolCall` outcomes, batch finalize at
//!   stream end or graceful shutdown.
//!
//! Type definitions live in `tool_call_types.rs`; JSON repair lives in
//! `tool_call_repair.rs`. This file is the only place that knows how to call
//! them.

use serde_json::{json, Value};
use tracing::{error, warn};

use crate::tool_call_repair::repair_truncated_json;
use crate::tool_call_types::{
    is_truncation_safe_to_recover, EarlyDetectedToolCall, FinalizedToolCall, PendingToolCall, PendingToolCalls,
    ToolCallBoundary, ToolCallDeltaOutcome, ToolCallParamsChunk, ToolCallStreamKey,
};

impl PendingToolCall {
    /// Strip Markdown-style triple-backtick fences (and any single backticks)
    /// from raw arguments that some tools emit wrapped in code fences.
    fn strip_argument_wrapping(raw_arguments: &str) -> &str {
        let trimmed = raw_arguments.trim();
        let Some(stripped) = trimmed.strip_prefix("```").and_then(|value| value.strip_suffix("```")) else {
            return trimmed.trim_matches('`').trim();
        };

        let stripped = stripped.trim();
        if let Some((first_line, rest)) = stripped.split_once('\n') {
            if first_line
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
            {
                return rest.trim();
            }
        }

        stripped
    }

    /// Best-effort repair for Git tool calls whose arguments came back as a raw
    /// shell-style command (e.g. `git status`, `"git diff --staged"`).
    fn parse_git_command_arguments(raw_arguments: &str) -> Option<Value> {
        let trimmed = Self::strip_argument_wrapping(raw_arguments);
        let command = trimmed.strip_prefix("git ").map(str::trim).unwrap_or(trimmed);
        let mut parts = command.splitn(2, char::is_whitespace);
        let operation = parts.next()?.trim();
        if operation.is_empty()
            || !operation
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        {
            return None;
        }

        let args = parts.next().map(str::trim).filter(|args| !args.is_empty());
        let mut value = json!({ "operation": operation });
        if let Some(args) = args {
            value["args"] = json!(args);
        }
        Some(value)
    }

    fn normalize_git_tool_arguments(arguments: Value) -> Value {
        if let Value::String(raw) = &arguments {
            if let Some(repaired) = Self::parse_git_command_arguments(raw) {
                warn!("Git tool call arguments repaired from JSON string command");
                return repaired;
            }
        }
        arguments
    }

    fn parse_arguments(tool_name: &str, raw_arguments: &str) -> Result<Value, String> {
        match serde_json::from_str::<Value>(raw_arguments) {
            Ok(arguments) => {
                if tool_name == "Git" {
                    Ok(Self::normalize_git_tool_arguments(arguments))
                } else {
                    Ok(arguments)
                }
            }
            Err(primary_error) => {
                if tool_name == "Git" {
                    if let Some(arguments) = Self::parse_git_command_arguments(raw_arguments) {
                        warn!("Git tool call arguments repaired from raw command");
                        return Ok(arguments);
                    }
                }
                Err(primary_error.to_string())
            }
        }
    }

    pub fn has_pending(&self) -> bool {
        !self.tool_id.is_empty()
    }

    pub fn has_meaningful_payload(&self) -> bool {
        !self.tool_name.is_empty() || !self.raw_arguments.is_empty()
    }

    pub fn tool_id(&self) -> &str {
        &self.tool_id
    }

    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    pub fn start_new(&mut self, tool_id: String, tool_name: Option<String>) {
        self.tool_id = tool_id;
        self.tool_name = tool_name.unwrap_or_default();
        self.raw_arguments.clear();
        self.early_detected_emitted = false;
    }

    pub fn update_tool_name_if_missing(&mut self, tool_name: Option<String>) {
        if self.tool_name.is_empty() {
            self.tool_name = tool_name.unwrap_or_default();
        }
    }

    pub fn append_arguments(&mut self, arguments_chunk: &str) {
        self.raw_arguments.push_str(arguments_chunk);
    }

    pub fn replace_arguments(&mut self, arguments_snapshot: &str) {
        self.raw_arguments.clear();
        self.raw_arguments.push_str(arguments_snapshot);
    }

    pub fn raw_arguments(&self) -> &str {
        &self.raw_arguments
    }

    pub fn finalize(&mut self, boundary: ToolCallBoundary) -> Option<FinalizedToolCall> {
        if !self.has_pending() {
            return None;
        }

        if !self.has_meaningful_payload() {
            self.tool_id.clear();
            self.tool_name.clear();
            self.raw_arguments.clear();
            self.early_detected_emitted = false;
            return None;
        }

        let tool_id = std::mem::take(&mut self.tool_id);
        let tool_name = std::mem::take(&mut self.tool_name);
        let raw_arguments = std::mem::take(&mut self.raw_arguments);
        self.early_detected_emitted = false;
        let parsed_arguments = Self::parse_arguments(&tool_name, &raw_arguments);

        let (arguments, is_error, recovered_from_truncation) = match parsed_arguments {
            Ok(value) => (value, false, false),
            Err(parse_err) => {
                let repaired = repair_truncated_json(&raw_arguments)
                    .and_then(|candidate| Self::parse_arguments(&tool_name, &candidate).ok());
                match repaired {
                    Some(value) if is_truncation_safe_to_recover(&tool_name) => {
                        warn!(
                            "Tool call arguments recovered from truncation at boundary={}: tool_id={}, tool_name={}, raw_len={}",
                            boundary.as_str(),
                            tool_id,
                            tool_name,
                            raw_arguments.len()
                        );
                        (value, false, true)
                    }
                    Some(_) => {
                        // We *could* repair but the tool's semantics make
                        // executing a partial call unsafe (Bash, Edit, ...).
                        // Surface as an error so the user/model knows the
                        // truncation happened and can retry sensibly.
                        warn!(
                            "Tool call arguments truncated at boundary={}: tool_id={}, tool_name={} — refusing to execute partial call (tool not in safe-recovery list)",
                            boundary.as_str(),
                            tool_id,
                            tool_name
                        );
                        (json!({}), true, true)
                    }
                    None => {
                        error!(
                            "Tool call arguments parsing failed at boundary={}: tool_id={}, tool_name={}, error={}, raw_arguments={}",
                            boundary.as_str(),
                            tool_id,
                            tool_name,
                            parse_err,
                            raw_arguments
                        );
                        (json!({}), true, false)
                    }
                }
            }
        };

        Some(FinalizedToolCall {
            tool_id,
            tool_name,
            raw_arguments,
            arguments,
            is_error,
            recovered_from_truncation,
        })
    }
}

impl PendingToolCalls {
    pub fn new() -> Self {
        Self {
            pending: std::collections::BTreeMap::new(),
        }
    }

    pub fn apply_delta(
        &mut self,
        key: ToolCallStreamKey,
        tool_id: Option<String>,
        tool_name: Option<String>,
        arguments: Option<String>,
        arguments_is_snapshot: bool,
    ) -> ToolCallDeltaOutcome {
        let mut outcome = ToolCallDeltaOutcome::default();

        let has_tool_id = tool_id.as_ref().is_some_and(|tool_id| !tool_id.is_empty());
        if !self.pending.contains_key(&key) {
            if has_tool_id {
                self.pending.insert(key.clone(), PendingToolCall::default());
            } else {
                return outcome;
            }
        }

        let Some(pending) = self.pending.get_mut(&key) else {
            return outcome;
        };

        if let Some(tool_id) = tool_id.filter(|tool_id| !tool_id.is_empty()) {
            let is_new_tool = pending.tool_id() != tool_id;
            if is_new_tool {
                outcome.finalized_previous = pending.finalize(ToolCallBoundary::NewTool);
                pending.start_new(tool_id, tool_name.clone());
            } else {
                pending.update_tool_name_if_missing(tool_name.clone());
            }
        } else if tool_name.as_ref().is_some_and(|tool_name| !tool_name.is_empty()) {
            pending.update_tool_name_if_missing(tool_name.clone());
        }

        if pending.has_pending() && !pending.tool_name().is_empty() && !pending.early_detected_emitted {
            pending.early_detected_emitted = true;
            outcome.early_detected = Some(EarlyDetectedToolCall {
                tool_id: pending.tool_id().to_string(),
                tool_name: pending.tool_name().to_string(),
            });
        }

        if let Some(arguments) = arguments.filter(|arguments| !arguments.is_empty()) {
            if pending.has_pending() {
                if arguments_is_snapshot {
                    pending.replace_arguments(&arguments);
                } else {
                    pending.append_arguments(&arguments);
                }
                let tool_name = pending.tool_name().to_string();
                let params_chunk = arguments;
                if !params_chunk.is_empty() {
                    outcome.params_partial = Some(ToolCallParamsChunk {
                        tool_id: pending.tool_id().to_string(),
                        tool_name,
                        params_chunk,
                    });
                }
            }
        }

        outcome
    }

    pub fn finalize_key(&mut self, key: &ToolCallStreamKey, boundary: ToolCallBoundary) -> Option<FinalizedToolCall> {
        let mut pending = self.pending.remove(key)?;
        pending.finalize(boundary)
    }

    pub fn finalize_all(&mut self, boundary: ToolCallBoundary) -> Vec<FinalizedToolCall> {
        let keys: Vec<_> = self.pending.keys().cloned().collect();
        keys.into_iter()
            .filter_map(|key| self.finalize_key(&key, boundary))
            .collect()
    }
}
