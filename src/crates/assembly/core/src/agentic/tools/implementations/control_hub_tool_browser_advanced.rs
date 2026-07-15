//! ControlHubTool browser sub-domain: low-level escape hatches (cdp, dialog, frame, frame_main).
//!
//!
//! R17 split: extracted from `control_hub_tool_browser.rs` (the 1272-line
//! god file post-R16) into per-subdomain sibling files. The facade keeps
//! the BROWSER_SESSIONS registry + thin `handle_browser` dispatcher; this
//! sibling owns the actions listed below as `pub(super)` inherent methods
//! on `ControlHubTool`.

use crate::agentic::tools::browser_control::actions::BrowserActions;
use crate::agentic::tools::browser_control::session_registry::DialogHandler;
use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

use super::control_hub::{err_response, ControlHubError, ErrorCode};
use super::control_hub_tool_browser::browser_sessions;
use super::ControlHubTool;
// Note: is_allowed_browser_cdp_method is an inherent method on ControlHubTool
// (pub(super)) defined in control_hub_tool_browser.rs. Call as
// Self::is_allowed_browser_cdp_method(method).

impl ControlHubTool {
    pub(super) async fn handle_browser_advanced(
        &self,
        action: &str,
        params: &Value,
        session_id_param: Option<String>,
    ) -> NortHingResult<Vec<ToolResult>> {
        let session = browser_sessions().get(session_id_param.as_deref()).await?;
        let actions = BrowserActions::new(session.client.as_ref());
        match action {
            "cdp" => {
                let method = params
                    .get("method")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("cdp requires 'method'".to_string()))?;
                if !Self::is_allowed_browser_cdp_method(method) {
                    return Ok(err_response(
                            "browser",
                            "cdp",
                            ControlHubError::new(
                                ErrorCode::InvalidParams,
                                format!("CDP method '{}' is not in the allowlist", method),
                            )
                            .with_hint("Only safe DOM/Input/Page/Network/Runtime/Emulation methods are allowed for sandbox protection"),
                        ));
                }
                let cdp_params = params.get("params").cloned();
                let result = session.client.send(method, cdp_params).await?;
                Ok(vec![ToolResult::ok(
                    json!({ "success": true, "method": method, "result": result }),
                    Some(format!("CDP {} executed", method)),
                )])
            }

            "dialog" => {
                let response = params.get("response").and_then(|v| v.as_str()).unwrap_or("accept");
                let accept = response != "dismiss";
                let prompt_text = params.get("prompt_text").and_then(|v| v.as_str()).map(str::to_string);
                session.state.arm_dialog(DialogHandler { accept, prompt_text }).await;
                let _ = session.client.send("Page.enable", None).await;
                Ok(vec![ToolResult::ok(
                    json!({ "success": true, "dialog_armed": true, "accept": accept }),
                    Some("Dialog handler armed".to_string()),
                )])
            }

            "frame" => {
                let selector = params
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("frame requires 'selector'".to_string()))?;
                let script = format!(
                    r#"(function(){{
                            const el = document.querySelector('{}');
                            if (!el) return JSON.stringify({{ found: false }});
                            return JSON.stringify({{ found: true, selector: '{}', name: el.name || '', url: el.src || '' }});
                        }})()"#,
                    selector.replace('\'', "\\'"),
                    selector.replace('\'', "\\'"),
                );
                let result = actions.evaluate(&script).await?;
                let raw = result
                    .get("result")
                    .and_then(|r| r.get("value"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}");
                let parsed: Value = serde_json::from_str(raw).unwrap_or(json!({}));
                if !parsed.get("found").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Ok(err_response(
                        "browser",
                        "frame",
                        ControlHubError::new(ErrorCode::NotFound, format!("iframe not found: {}", selector)),
                    ));
                }
                session.state.set_active_frame(Some(selector.to_string())).await;
                Ok(vec![ToolResult::ok(
                    json!({ "frame": parsed }),
                    Some("Frame context noted".to_string()),
                )])
            }

            "frame_main" => {
                session.state.set_active_frame(None).await;
                Ok(vec![ToolResult::ok(
                    json!({ "frame": "main" }),
                    Some("Frame context reset".to_string()),
                )])
            }
            other => Err(NortHingError::tool(format!(
                "action '{}' dispatched to handle_browser_advanced but is not in its match arms (facade dispatch bug)",
                other
            ))),
        }
    }
}
