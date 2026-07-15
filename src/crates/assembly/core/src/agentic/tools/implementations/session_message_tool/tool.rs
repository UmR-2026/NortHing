//! `SessionMessage` tool — struct definition and `Tool` trait implementation.
//!
//! The struct, `Default` impl, and `Tool` trait impl all live here. Most
//! concrete logic lives in sibling sub-modules:
//!  * `sm_types` — input struct, agent-type enum, and session-id shape
//!    validator.
//!  * `sm_resolve` — workspace / sender / creator resolution helpers and
//!    the cross-session reminder envelope.
//!  * `sm_send` — target-session preparation (existing vs. new) and the
//!    shared submit + result-formatting flow.
//!
//! The `Tool::call_impl` body in this file is intentionally thin: it
//! parses the input, extracts source-side context, builds the runtime
//! handle, and forwards to the matching `prepare_*` + `submit_and_format`
//! helpers in `sm_send`.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::agentic::coordination::{global_coordinator, global_scheduler};
use crate::agentic::tools::framework::{
    Tool, ToolExposure, ToolRenderOptions, ToolResult, ToolUseContext, ValidationResult,
};
use crate::service_agent_runtime::CoreServiceAgentRuntime;
use crate::util::errors::{NortHingError, NortHingResult};

use super::sm_types::{validate_session_id, SessionMessageInput};

/// SessionMessage tool - send a message to another session via the dialog scheduler.
pub struct SessionMessageTool;

impl Default for SessionMessageTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionMessageTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SessionMessageTool {
    fn name(&self) -> &str {
        "SessionMessage"
    }

    async fn description(&self) -> NortHingResult<String> {
        Ok(
            r#"Asynchronously send a message to another agent session. When the target session finishes, its result is automatically sent back to you as a follow-up message.

Usage:
- Create a new session and send: omit "session_id", and provide "workspace", "session_name", "agent_type", and "message".
- Reusing an existing session: provide "session_id" and "message". You may omit "workspace"; the tool will resolve it from the target session when possible.

Allowed agent types when creating a session:
- "agentic": Coding-focused agent for implementation, debugging, and code changes.
- "Plan": Planning agent for clarifying requirements and producing an implementation plan before coding.
- "Cowork": Collaborative agent for office-style work such as research, documentation, presentations, etc.
"#
                .to_string(),
        )
    }

    fn short_description(&self) -> String {
        "Send a message to another agent session and receive the result asynchronously.".to_string()
    }

    fn default_exposure(&self) -> ToolExposure {
        ToolExposure::Collapsed
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "workspace": {
                    "type": "string",
                    "description": "Required absolute target workspace path when creating a new session. Optional when session_id is provided."
                },
                "session_id": {
                    "type": "string",
                    "description": "Optional target session ID. Omit it to create a new session and send the message there."
                },
                "session_name": {
                    "type": "string",
                    "description": "Required when session_id is omitted. Display name for the new session."
                },
                "message": {
                    "type": "string",
                    "description": "Message to send to the target session."
                },
                "agent_type": {
                    "type": "string",
                    "enum": ["agentic", "Plan", "Cowork"],
                    "description": "Required when session_id is omitted. Not allowed when sending to an existing session."
                }
            },
            "required": ["message"],
            "additionalProperties": false
        })
    }

    fn is_readonly(&self) -> bool {
        false
    }

    fn needs_permissions(&self, _input: Option<&Value>) -> bool {
        false
    }

    async fn validate_input(&self, input: &Value, context: Option<&ToolUseContext>) -> ValidationResult {
        let parsed: SessionMessageInput = match serde_json::from_value(input.clone()) {
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

        if parsed.message.trim().is_empty() {
            return ValidationResult {
                result: false,
                message: Some("message cannot be empty".to_string()),
                error_code: Some(400),
                meta: None,
            };
        }

        match parsed.session_id.as_deref() {
            Some(session_id) => {
                if let Err(message) = validate_session_id(session_id) {
                    return ValidationResult {
                        result: false,
                        message: Some(message),
                        error_code: Some(400),
                        meta: None,
                    };
                }

                if parsed.session_name.is_some() {
                    return ValidationResult {
                        result: false,
                        message: Some("session_name is only allowed when session_id is omitted".to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                }

                if parsed.agent_type.is_some() {
                    return ValidationResult {
                        result: false,
                        message: Some("agent_type override is not allowed when session_id is provided".to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                }

                if let Some(workspace) = parsed.workspace.as_deref() {
                    let workspace_validation = self.validate_workspace_shape(workspace, context);
                    if !workspace_validation.result {
                        return workspace_validation;
                    }
                }
            }
            None => {
                if parsed
                    .session_name
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    return ValidationResult {
                        result: false,
                        message: Some("session_name is required when session_id is omitted".to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                }

                if parsed.agent_type.is_none() {
                    return ValidationResult {
                        result: false,
                        message: Some("agent_type is required when session_id is omitted".to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                }

                let Some(workspace) = parsed.workspace.as_deref() else {
                    return ValidationResult {
                        result: false,
                        message: Some("workspace is required when session_id is omitted".to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                };
                let workspace_validation = self.validate_workspace_shape(workspace, context);
                if !workspace_validation.result {
                    return workspace_validation;
                }
            }
        }

        let Some(context) = context else {
            return ValidationResult::default();
        };

        let Some(source_session_id) = context.session_id.as_deref() else {
            return ValidationResult {
                result: false,
                message: Some("SessionMessage requires a source session in tool context".to_string()),
                error_code: Some(400),
                meta: None,
            };
        };

        if let Some(target_session_id) = parsed.session_id.as_deref() {
            if source_session_id == target_session_id {
                return ValidationResult {
                    result: false,
                    message: Some("SessionMessage cannot send a message to the same session".to_string()),
                    error_code: Some(400),
                    meta: None,
                };
            }
        }

        ValidationResult::default()
    }

    fn render_tool_use_message(&self, input: &Value, _options: &ToolRenderOptions) -> String {
        let workspace = input
            .get("workspace")
            .and_then(|value| value.as_str())
            .unwrap_or("resolved workspace");
        if let Some(session_id) = input.get("session_id").and_then(|value| value.as_str()) {
            format!("Send message to session {} in {}", session_id, workspace)
        } else {
            let session_name = input
                .get("session_name")
                .and_then(|value| value.as_str())
                .unwrap_or("new session");
            format!("Create session {} in {} and send message", session_name, workspace)
        }
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        let params: SessionMessageInput =
            serde_json::from_value(input.clone()).map_err(|e| NortHingError::tool(format!("Invalid input: {}", e)))?;
        let source_session_id = self.sender_session_id(context)?.to_string();
        let source_workspace = self.sender_workspace(context)?;

        let coordinator =
            global_coordinator().ok_or_else(|| NortHingError::tool("coordinator not initialized".to_string()))?;
        let scheduler =
            global_scheduler().ok_or_else(|| NortHingError::tool("scheduler not initialized".to_string()))?;
        let runtime = CoreServiceAgentRuntime::agent_runtime_with_dialog_turns(coordinator.clone(), scheduler)
            .map_err(NortHingError::tool)?;

        let target = if let Some(target_session_id) = params.session_id.clone() {
            self.prepare_existing_target(&params, &target_session_id, &source_session_id, context, &runtime)
                .await?
        } else {
            self.prepare_new_target(&params, context, &runtime).await?
        };

        self.submit_and_format(&params, &source_session_id, &source_workspace, target, &runtime)
            .await
    }
}
