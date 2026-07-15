//! ControlHubTool browser sub-domain: user interaction (click, fill, type, select, press_key, scroll, hover, check).
//!
//!
//! R17 split: extracted from `control_hub_tool_browser.rs` (the 1272-line
//! god file post-R16) into per-subdomain sibling files. The facade keeps
//! the BROWSER_SESSIONS registry + thin `handle_browser` dispatcher; this
//! sibling owns the actions listed below as `pub(super)` inherent methods
//! on `ControlHubTool`.

use crate::agentic::tools::browser_control::actions::BrowserActions;
use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

use super::control_hub::{err_response, ControlHubError, ErrorCode};
use super::control_hub_tool_browser::browser_sessions;
use super::ControlHubTool;

impl ControlHubTool {
    pub(super) async fn handle_browser_interact(
        &self,
        action: &str,
        params: &Value,
        session_id_param: Option<String>,
    ) -> NortHingResult<Vec<ToolResult>> {
        let session = browser_sessions().get(session_id_param.as_deref()).await?;
        let actions = BrowserActions::new(session.client.as_ref());
        match action {
            "click" => {
                let selector = params
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("click requires 'selector'".to_string()))?;
                let result = actions.click(selector).await?;
                Ok(vec![ToolResult::ok(result, Some(format!("Clicked {}", selector)))])
            }

            "fill" => {
                let selector = params
                    .get("selector")
                    .or_else(|| params.get("ref"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("fill requires 'selector'".to_string()))?;
                let value = params
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("fill requires 'value'".to_string()))?;
                let result = actions.fill(selector, value).await?;
                Ok(vec![ToolResult::ok(
                    result,
                    Some(format!("Filled {} with text", selector)),
                )])
            }

            "type" => {
                let text = params
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("type requires 'text'".to_string()))?;
                let result = actions.type_text(text).await?;
                Ok(vec![ToolResult::ok(result, Some("Typed text".to_string()))])
            }

            "select" => {
                let selector = params
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("select requires 'selector'".to_string()))?;
                let option_text = params
                    .get("option_text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("select requires 'option_text'".to_string()))?;
                let result = actions.select(selector, option_text).await?;
                // Phase 3: the underlying JS returns `{ error, available }`
                // shaped success bodies for "select not found" and
                // "option not found" cases. Lift those into the
                // unified ControlHub error envelope so the model can
                // branch on `error.code` instead of scraping JSON.
                if let Some(err_msg) = result.get("error").and_then(|v| v.as_str()) {
                    let lowered = err_msg.to_lowercase();
                    let (code, hint) = if lowered.contains("select not found") {
                        (
                            ErrorCode::NotFound,
                            format!(
                                "No <select> matched '{}'. Take a fresh snapshot and verify the selector.",
                                selector
                            ),
                        )
                    } else if lowered.contains("option not found") {
                        (
                            ErrorCode::NotFound,
                            "Inspect `available` in error.hints for valid option labels.".to_string(),
                        )
                    } else {
                        (
                            ErrorCode::Internal,
                            "Browser returned an unexpected select error".to_string(),
                        )
                    };
                    let mut chub_err = ControlHubError::new(code, err_msg).with_hint(hint);
                    if let Some(avail) = result.get("available") {
                        chub_err = chub_err.with_hint(format!("available_options={}", avail));
                    }
                    return Ok(err_response("browser", "select", chub_err));
                }
                Ok(vec![ToolResult::ok(
                    result,
                    Some(format!("Selected '{}'", option_text)),
                )])
            }

            "press_key" => {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("press_key requires 'key'".to_string()))?;
                let result = actions.press_key(key).await?;
                Ok(vec![ToolResult::ok(result, Some(format!("Pressed {}", key)))])
            }

            "scroll" => {
                let direction = params.get("direction").and_then(|v| v.as_str()).unwrap_or("down");
                let amount = params.get("amount").and_then(|v| v.as_i64());
                let result = actions.scroll(direction, amount).await?;
                Ok(vec![ToolResult::ok(result, Some(format!("Scrolled {}", direction)))])
            }

            "hover" => {
                let selector = params
                    .get("selector")
                    .or_else(|| params.get("ref"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("hover requires 'selector'".to_string()))?;
                let result = actions.hover(selector).await?;
                Ok(vec![ToolResult::ok(result, Some(format!("Hovered {}", selector)))])
            }

            "check" | "uncheck" => {
                let selector = params
                    .get("selector")
                    .or_else(|| params.get("ref"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("check/uncheck requires 'selector'".to_string()))?;
                let result = actions.set_checked(selector, action == "check").await?;
                Ok(vec![ToolResult::ok(
                    result,
                    Some(format!("Set checked on {}", selector)),
                )])
            }
            other => Err(NortHingError::tool(format!(
                "action '{}' dispatched to handle_browser_interact but is not in its match arms (facade dispatch bug)",
                other
            ))),
        }
    }
}
