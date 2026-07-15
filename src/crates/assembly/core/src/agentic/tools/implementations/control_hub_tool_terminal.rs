//! ControlHubTool terminal domain.

//!

//! R16 split: handle_terminal extracted out as a sibling impl ControlHubTool

//! block. Delegates to TerminalControlTool after resolving an optional

//! `terminal_session_id` (auto-pick when exactly one live session).

use crate::agentic::tools::framework::{Tool, ToolResult, ToolUseContext};

use crate::service::terminal::api::TerminalApi;

use crate::util::errors::{NortHingError, NortHingResult};

use serde_json::{json, Value};

use super::control_hub::{err_response, ControlHubError, ErrorCode};

use super::terminal_control_tool::TerminalControlTool;
use super::ControlHubTool;

impl ControlHubTool {
    pub(super) async fn handle_terminal(
        &self,
        action: &str,
        params: &Value,
        context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        // Phase 4: enumerate live terminal sessions so the model can resolve
        // a `terminal_session_id` *before* attempting `kill` / `interrupt`.
        // Previously this required digging through earlier `Bash` results.
        if action == "list_sessions" {
            let api = crate::service::terminal::api::TerminalApi::from_singleton()
                .map_err(|e| NortHingError::tool(format!("TerminalApi unavailable: {}", e)))?;
            let sessions = api
                .list_sessions()
                .await
                .map_err(|e| NortHingError::tool(format!("list_sessions failed: {}", e)))?;
            let summary: Vec<Value> = sessions
                .iter()
                .map(|s| {
                    json!({
                        "terminal_session_id": s.id,
                        "name": s.name,
                        "cwd": s.cwd,
                        "pid": s.pid,
                        "status": s.status,
                    })
                })
                .collect();
            let count = summary.len();
            return Ok(vec![ToolResult::ok(
                json!({ "sessions": summary, "count": count }),
                Some(format!("{} terminal session(s) live", count)),
            )]);
        }

        // UX shortcut: when there is exactly one live terminal session,
        // make `terminal_session_id` optional. The 95th-percentile flow is
        // "Bash launched a long-running command, please interrupt it" and
        // the user has no other terminals open — forcing a `list_sessions`
        // round-trip just to copy the only id back wastes a turn.
        let resolved_id: String = match params.get("terminal_session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                let api = crate::service::terminal::api::TerminalApi::from_singleton()
                    .map_err(|e| NortHingError::tool(format!("TerminalApi unavailable: {}", e)))?;
                let sessions = api
                    .list_sessions()
                    .await
                    .map_err(|e| NortHingError::tool(format!("list_sessions failed: {}", e)))?;
                let live: Vec<_> = sessions
                    .iter()
                    .filter(|s| {
                        s.status.eq_ignore_ascii_case("running")
                            || s.status.eq_ignore_ascii_case("active")
                            || s.status.eq_ignore_ascii_case("idle")
                    })
                    .collect();
                if live.len() == 1 {
                    live[0].id.clone()
                } else if live.is_empty() {
                    return Ok(err_response(
                        "terminal",
                        action,
                        ControlHubError::new(ErrorCode::MissingSession, "No live terminal sessions to target")
                            .with_hint("Use the Bash tool to start a command, then this action becomes meaningful"),
                    ));
                } else {
                    let ids: Vec<&str> = live.iter().map(|s| s.id.as_str()).collect();
                    return Ok(err_response(
                        "terminal",
                        action,
                        ControlHubError::new(
                            ErrorCode::Ambiguous,
                            format!(
                                "{} live terminal sessions; pass 'terminal_session_id' to disambiguate",
                                live.len()
                            ),
                        )
                        .with_hint(format!("live_session_ids={:?}", ids))
                        .with_hint("Call terminal.list_sessions to see names + cwd"),
                    ));
                }
            }
        };

        let mut input = params.clone();
        if let Value::Object(ref mut map) = input {
            map.insert("action".to_string(), json!(action));
            map.insert("terminal_session_id".to_string(), json!(resolved_id));
        }

        // call_impl lives on impl Tool for ControlHubTool (delegate through self)
        self.call_impl(&input, context).await
    }
}
