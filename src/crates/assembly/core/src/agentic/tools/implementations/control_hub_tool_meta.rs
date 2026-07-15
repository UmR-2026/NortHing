//! ControlHubTool meta domain.

//!

//! R16 split: handle_meta extracted out as a sibling impl ControlHubTool

//! block. The `pub(super)` visibility makes it reachable from the

//! facade's `dispatch()` via inherent-method resolution.

#[cfg(target_os = "linux")]
use super::computer_use_actions::linux_session_info;

use super::computer_use_actions::which_exists;

use super::control_hub_tool_browser::browser_sessions;

use super::control_hub::{err_response, ControlHubError, ErrorCode};

use crate::agentic::tools::framework::{ToolResult, ToolUseContext};

use crate::util::errors::{NortHingError, NortHingResult};

use super::ControlHubTool;
use serde_json::{json, Value};

impl ControlHubTool {
    pub(super) async fn handle_meta(
        &self,
        action: &str,
        params: &Value,
        context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        match action {
            "capabilities" => {
                // `terminal` (TerminalApi) is delivered through a global
                // registry rather than a field on the context, so we can't be
                // 100% sure here without round-tripping. We report "likely
                // available iff a desktop host is present" because that bridge
                // only exists in northhing's desktop runtime; the actual call will
                // surface a clean error if the bridge is offline.
                let likely_terminal_available = context.computer_use_host.is_some();
                let browser_default = browser_sessions().default_id().await;
                let browser_session_count = browser_sessions().list().await.len();
                let os = std::env::consts::OS;
                let arch = std::env::consts::ARCH;

                // Probe which browser the host considers default. We surface
                // both the kind AND whether it is CDP-driveable (Safari/
                // Firefox aren't, so the model can fall back to system.open_url
                // instead of attempting a doomed `browser.connect`).
                let (browser_kind, browser_cdp_supported) =
                match crate::agentic::tools::browser_control::browser_launcher::BrowserLauncher::detect_default_browser() {
                    Ok(k) => {
                        let supported = !matches!(
                            k,
                            crate::agentic::tools::browser_control::browser_launcher::BrowserKind::Unknown(_)
                        );
                        (Some(k.to_string()), supported)
                    }
                    Err(_) => (None, false),
                };

                // Same script_types probe as get_os_info — duplicated here
                // because callers often hit `meta.capabilities` first and we
                // don't want to force an extra system round-trip.
                let mut _script_types: Vec<&'static str> = vec!["shell"];
                if cfg!(target_os = "macos") {
                    _script_types.push("applescript");
                }
                if which_exists("bash") {
                    _script_types.push("bash");
                }
                if which_exists("pwsh") || which_exists("powershell") {
                    _script_types.push("powershell");
                }
                if cfg!(target_os = "windows") {
                    _script_types.push("cmd");
                }

                #[cfg(target_os = "linux")]
                let (display_server, desktop_env) = linux_session_info();
                #[cfg(not(target_os = "linux"))]
                let (display_server, desktop_env): (Option<String>, Option<String>) = (None, None);

                let body = json!({
                    "domains": {
                        "browser":  {
                            "available": true,
                            "default_session_id": browser_default,
                            "session_count": browser_session_count,
                            "default_browser": browser_kind,
                            "cdp_supported": browser_cdp_supported,
                        },
                        "terminal": { "available": likely_terminal_available, "reason": if likely_terminal_available { Value::Null } else { json!("TerminalApi is only available in contexts that registered it") } },
                        "meta":     { "available": true },
                    },
                    "host": {
                        "os": os,
                        "arch": arch,
                        "display_server": display_server,
                        "desktop_environment": desktop_env,
                    },
                    "schema_version": "1.1",
                });
                Ok(vec![ToolResult::ok(
                    body,
                    Some("ControlHub capabilities snapshot".to_string()),
                )])
            }
            "route_hint" => {
                // Best-effort heuristic mapping a free-form intent to one
                // (or two ranked) domains. The model is still expected to
                // make the final call — this is a hint, not a binding.
                let intent = params
                    .get("intent")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("route_hint requires 'intent' (string)".to_string()))?;
                let lower = intent.to_lowercase();

                let mut suggestions: Vec<(&'static str, u32, &'static str)> = vec![];
                let push = |s: &mut Vec<(&'static str, u32, &'static str)>,
                            domain: &'static str,
                            score: u32,
                            why: &'static str| {
                    s.push((domain, score, why));
                };

                let browser_kw = [
                    "http",
                    "https",
                    "url",
                    "browser",
                    "google",
                    "tab",
                    "网页",
                    "浏览器",
                    "网站",
                ];
                let desktop_kw = [
                    "screenshot",
                    "click on",
                    "window",
                    "dialog",
                    "finder",
                    "vscode",
                    "桌面",
                    "应用窗口",
                    "外部应用",
                ];
                let terminal_kw = ["kill terminal", "interrupt", "ctrl+c", "stop process"];
                let system_kw = [
                    "open ",
                    "applescript",
                    "shell script",
                    "运行脚本",
                    "启动应用",
                    "open app",
                ];

                for kw in browser_kw {
                    if lower.contains(kw) {
                        push(&mut suggestions, "browser", 85, "Matches browser/URL keywords");
                        break;
                    }
                }
                for kw in desktop_kw {
                    if lower.contains(kw) {
                        push(
                            &mut suggestions,
                            "ComputerUse",
                            75,
                            "Matches local desktop/system keywords; use the ComputerUse tool/agent",
                        );
                        break;
                    }
                }
                for kw in terminal_kw {
                    if lower.contains(kw) {
                        push(&mut suggestions, "terminal", 80, "Matches terminal-signal keywords");
                        break;
                    }
                }
                for kw in system_kw {
                    if lower.contains(kw) {
                        push(
                            &mut suggestions,
                            "ComputerUse",
                            70,
                            "Matches OS/launch keywords; use the ComputerUse tool/agent",
                        );
                        break;
                    }
                }
                suggestions.sort_by_key(|b| std::cmp::Reverse(b.1));

                let ranked: Vec<Value> = suggestions
                    .iter()
                    .map(|(d, score, why)| json!({ "domain": d, "score": score, "why": why }))
                    .collect();
                let suggested = suggestions.first().map(|(d, _, _)| (*d).to_string());
                Ok(vec![ToolResult::ok(
                    json!({
                        "intent": intent,
                        "suggested_domain": suggested,
                        "ranked": ranked,
                        "note": "Heuristic only — confirm by reading meta.capabilities and the domain-specific docs.",
                    }),
                    Some(match &suggested {
                        Some(d) => format!("Best guess: domain={}", d),
                        None => "No confident routing match".to_string(),
                    }),
                )])
            }
            other => Err(NortHingError::tool(format!(
                "Unknown meta action: '{}'. Valid actions: capabilities, route_hint",
                other
            ))),
        }
    }
}
