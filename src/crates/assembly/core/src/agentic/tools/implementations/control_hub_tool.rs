//! ControlHub — unified entry point for browser, terminal, and routing metadata.
//!
//! Routes requests by `domain` to the appropriate backend:
//!   browser  → CDP-based browser control (new)
//!   terminal → TerminalApi (existing)
//!   meta     → capability and route introspection
//!
//! Local desktop and OS/system actions are intentionally surfaced through the
//! dedicated ComputerUse tool/agent, not through public ControlHub domains.
//!
//! R16 split: physical extraction of sub-domain handlers into sibling files.
//! The facade keeps the public `ControlHubTool` struct, the `Default` impl,
//! `new()`, the dispatcher, and the `Tool` trait impl. Cross-sibling handler
//! calls resolve via inherent-method resolution across `pub(super)`
//! `impl ControlHubTool { ... }` blocks in sibling files.

use crate::agentic::tools::framework::{
    Tool, ToolExposure, ToolRenderOptions, ToolResult, ToolUseContext, ValidationResult,
};
use crate::util::errors::{NortHingError, NortHingResult};
use async_trait::async_trait;
use serde_json::{json, Value};

use super::control_hub::{err_response, ControlHubError, ErrorCode};
use super::control_hub_tool_descriptions::description_text;
use super::control_hub_tool_envelope::{envelope_wrap_results, map_dispatch_error};
// handle_browser / handle_meta / handle_terminal are methods on ControlHubTool
// defined in sibling files (control_hub_tool_browser.rs / _meta.rs / _terminal.rs)
// via `impl ControlHubTool { ... }` blocks. They resolve via inherent-method
// dispatch from the facade's dispatch() call site -- no explicit import needed.

pub struct ControlHubTool;

impl Default for ControlHubTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ControlHubTool {
    pub fn new() -> Self {
        Self
    }

    /// Route a `domain/action/params` call to the matching sibling handler
    /// (browser / terminal / meta). Unknown domains return a structured error
    /// listing the valid ControlHub domains and pointing the model at
    /// `ComputerUse` for desktop/system work.
    pub(super) async fn dispatch(
        &self,
        domain: &str,
        action: &str,
        params: &Value,
        context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        match domain {
            "desktop" => {
                Ok(err_response(
                    "desktop",
                    action,
                    ControlHubError::new(
                        ErrorCode::InvalidParams,
                        "The desktop domain has moved out of ControlHub.",
                    )
                    .with_hint(
                        "Use the dedicated ComputerUse tool/agent for screenshots, OCR, mouse, keyboard, and desktop app control.",
                    ),
                ))
            }
            "browser" => self.handle_browser(action, params).await,
            "terminal" => self.handle_terminal(action, params, context).await,
            "system" => Ok(err_response(
                "system",
                action,
                ControlHubError::new(
                    ErrorCode::InvalidParams,
                    "The system domain has moved out of ControlHub.",
                )
                .with_hint(
                    "Use the dedicated ComputerUse tool/agent for open_app, open_url, open_file, clipboard, OS info, and local scripts.",
                ),
            )),
            "meta" => self.handle_meta(action, params, context).await,
            other => Err(NortHingError::tool(format!(
                "Unknown domain: '{}'. Valid ControlHub domains: browser, terminal, meta. Use ComputerUse for desktop/system actions.",
                other
            ))),
        }
    }
}

#[async_trait]
impl Tool for ControlHubTool {
    fn name(&self) -> &str {
        "ControlHub"
    }

    async fn description(&self) -> NortHingResult<String> {
        Ok(description_text())
    }

    fn short_description(&self) -> String {
        "Control browser, terminal, and desktop helper domains through one tool.".to_string()
    }

    fn default_exposure(&self) -> ToolExposure {
        ToolExposure::Collapsed
    }

    async fn description_with_context(&self, _context: Option<&ToolUseContext>) -> NortHingResult<String> {
        Ok(description_text())
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "domain": {
                    "type": "string",
                    "enum": ["browser", "terminal", "meta"],
                    "description": "The control domain to target."
                },
                "action": {
                    "type": "string",
                    "description": "The atomic action to perform within the domain."
                },
                "params": {
                    "type": "object",
                    "description": "Action-specific parameters. See domain documentation for details.",
                    "additionalProperties": true
                }
            },
            "required": ["domain", "action"]
        })
    }

    fn needs_permissions(&self, _input: Option<&Value>) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: Option<&Value>) -> bool {
        false
    }

    async fn is_enabled(&self) -> bool {
        true
    }

    async fn validate_input(&self, input: &Value, _context: Option<&ToolUseContext>) -> ValidationResult {
        let domain = input.get("domain").and_then(|v| v.as_str());
        let action = input.get("action").and_then(|v| v.as_str());

        if domain.is_none() {
            return ValidationResult {
                result: false,
                message: Some("Missing required field: domain".to_string()),
                error_code: None,
                meta: None,
            };
        }
        if action.is_none() {
            return ValidationResult {
                result: false,
                message: Some("Missing required field: action".to_string()),
                error_code: None,
                meta: None,
            };
        }
        ValidationResult::default()
    }

    fn render_tool_use_message(&self, input: &Value, _options: &ToolRenderOptions) -> String {
        let domain = input.get("domain").and_then(|v| v.as_str()).unwrap_or("?");
        let action = input.get("action").and_then(|v| v.as_str()).unwrap_or("?");
        format!("ControlHub: {}.{}", domain, action)
    }

    fn render_result_for_assistant(&self, output: &Value) -> String {
        // New unified envelope: prefer ok=true → data summary, ok=false → error.message.
        if let Some(ok) = output.get("ok").and_then(|v| v.as_bool()) {
            if ok {
                if let Some(s) = output.get("summary").and_then(|v| v.as_str()) {
                    return s.to_string();
                }
                return output.to_string();
            } else if let Some(err) = output.get("error") {
                let code = err.get("code").and_then(|v| v.as_str()).unwrap_or("ERROR");
                let msg = err.get("message").and_then(|v| v.as_str()).unwrap_or("");
                return format!("{}: {}", code, msg);
            }
        }
        // Legacy fallback: previous tool result shape with `result` field.
        if let Some(result) = output.get("result").and_then(|v| v.as_str()) {
            return result.to_string();
        }
        output.to_string()
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        let domain = input.get("domain").and_then(|v| v.as_str()).unwrap_or("");
        let action = input.get("action").and_then(|v| v.as_str()).unwrap_or("");

        if domain.is_empty() {
            return Ok(err_response(
                "?",
                action,
                ControlHubError::new(ErrorCode::InvalidParams, "Missing required field 'domain'.").with_hint(
                    "Set domain to one of: browser, terminal, meta. Use ComputerUse for desktop/system actions.",
                ),
            ));
        }
        if action.is_empty() {
            return Ok(err_response(
                domain,
                "?",
                ControlHubError::new(ErrorCode::InvalidParams, "Missing required field 'action'.")
                    .with_hint("Pick a valid action for this domain (see ControlHub description)."),
            ));
        }

        let params = input.get("params").cloned().unwrap_or(json!({}));
        let dispatched = self.dispatch(domain, action, &params, context).await;

        // Wrap legacy handler results into the unified envelope.
        match dispatched {
            Ok(results) => Ok(envelope_wrap_results(domain, action, results)),
            Err(err) => Ok(err_response(domain, action, map_dispatch_error(domain, action, err))),
        }
    }
}
