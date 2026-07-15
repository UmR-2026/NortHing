//! ControlHubTool browser sub-domain: telemetry (network, console, errors, trace).
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

use super::control_hub_tool_browser::browser_sessions;
use super::ControlHubTool;

impl ControlHubTool {
    pub(super) async fn handle_browser_telemetry(
        &self,
        action: &str,
        params: &Value,
        session_id_param: Option<String>,
    ) -> NortHingResult<Vec<ToolResult>> {
        let session = browser_sessions().get(session_id_param.as_deref()).await?;
        let actions = BrowserActions::new(session.client.as_ref());
        match action {
            "network" | "network_requests" => {
                let session = browser_sessions().get(session_id_param.as_deref()).await?;
                let state = &session.state;
                let sub = params.get("sub_command").and_then(|v| v.as_str());
                match sub {
                    Some("clear") => {
                        state.clear_network().await;
                        Ok(vec![ToolResult::ok(
                            json!({ "success": true, "cleared": true }),
                            Some("Network events cleared".to_string()),
                        )])
                    }
                    Some("summary") => {
                        let total = state.query_network(None, None, None, None, usize::MAX).await.len();
                        let requests = state.query_network_requests(None, None, None, None, 50).await;
                        Ok(vec![ToolResult::ok(
                            json!({
                                "total_events": total,
                                "requests": requests,
                            }),
                            Some(format!("Network summary: {} total events", total)),
                        )])
                    }
                    _ => {
                        let filter = params.get("filter").and_then(|v| v.as_str());
                        let method = params.get("method").and_then(|v| v.as_str());
                        let status = params.get("status").and_then(|v| v.as_str());
                        let since = params.get("since").and_then(|v| v.as_str());
                        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                        let events = if sub == Some("requests") {
                            state.query_network_requests(filter, method, status, since, limit).await
                        } else {
                            state.query_network(filter, method, status, since, limit).await
                        };
                        Ok(vec![ToolResult::ok(
                            json!({ "events": events, "count": events.len() }),
                            Some(format!("{} network event(s)", events.len())),
                        )])
                    }
                }
            }

            "console" => {
                let session = browser_sessions().get(session_id_param.as_deref()).await?;
                let state = &session.state;
                let sub = params.get("sub_command").and_then(|v| v.as_str());
                if sub == Some("clear") {
                    state.clear_console().await;
                    return Ok(vec![ToolResult::ok(
                        json!({ "success": true, "cleared": true }),
                        Some("Console events cleared".to_string()),
                    )]);
                }
                let filter = params.get("filter").and_then(|v| v.as_str());
                let since = params.get("since").and_then(|v| v.as_str());
                let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                let events = state.query_console(filter, since, limit).await;
                Ok(vec![ToolResult::ok(
                    json!({ "events": events, "count": events.len() }),
                    Some(format!("{} console event(s)", events.len())),
                )])
            }

            "errors" => {
                let session = browser_sessions().get(session_id_param.as_deref()).await?;
                let state = &session.state;
                let sub = params.get("sub_command").and_then(|v| v.as_str());
                if sub == Some("clear") {
                    state.clear_errors().await;
                    return Ok(vec![ToolResult::ok(
                        json!({ "success": true, "cleared": true }),
                        Some("JS error events cleared".to_string()),
                    )]);
                }
                let filter = params.get("filter").and_then(|v| v.as_str());
                let since = params.get("since").and_then(|v| v.as_str());
                let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                let events = state.query_errors(filter, since, limit).await;
                Ok(vec![ToolResult::ok(
                    json!({ "events": events, "count": events.len() }),
                    Some(format!("{} JS error event(s)", events.len())),
                )])
            }

            "trace" => {
                let session = browser_sessions().get(session_id_param.as_deref()).await?;
                let state = &session.state;
                let sub = params.get("sub_command").and_then(|v| v.as_str());
                match sub {
                    Some("start") => {
                        let result = state.trace_start().await;
                        Ok(vec![ToolResult::ok(
                            result,
                            Some("CDP trace recording started".to_string()),
                        )])
                    }
                    Some("stop") => {
                        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(200) as usize;
                        let result = state.trace_stop(limit).await;
                        Ok(vec![ToolResult::ok(
                            result,
                            Some("CDP trace recording stopped".to_string()),
                        )])
                    }
                    Some("status") => {
                        let result = state.trace_status().await;
                        Ok(vec![ToolResult::ok(result, Some("CDP trace status".to_string()))])
                    }
                    Some("clear") => {
                        let result = state.trace_clear().await;
                        Ok(vec![ToolResult::ok(result, Some("CDP trace cleared".to_string()))])
                    }
                    _ => {
                        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;
                        let result = state.trace_stop(limit).await;
                        Ok(vec![ToolResult::ok(result, Some("CDP trace events".to_string()))])
                    }
                }
            }
            other => Err(NortHingError::tool(format!(
                "action '{}' dispatched to handle_browser_telemetry but is not in its match arms (facade dispatch bug)",
                other
            ))),
        }
    }
}
