//! Computer Use desktop and OS/system action implementations.
//!
//! This module owns the action logic that used to live behind ControlHub's
//! desktop/system domains. ControlHub may still share the common error envelope
//! types, but it no longer owns these Computer Use behaviors.

//! Desktop-domain action implementations (legacy desktop + AX-first entrypoint).
//!
//! `handle_desktop` is the unified dispatcher for every `desktop.<action>`
//! call. It owns the legacy `list_displays` / `paste` / `focus_display`
//! shortcuts and forwards the seven AX-first `app_*` / interactive / visual
//! actions to `handle_desktop_ax` (see `desktop_ax_actions.rs`).
//!
//! Anything not handled locally is forwarded to the legacy
//! [`super::super::computer_use_tool::ComputerUseTool`] after stripping
//! ControlHub-only fields and applying the optional `display_id` pin.

use crate::agentic::tools::framework::{Tool, ToolResult, ToolUseContext};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

use super::super::control_hub::{err_response, ControlHubError, ErrorCode};
use super::utilities::clipboard_write;
use super::ComputerUseActions;

impl ComputerUseActions {
    pub(crate) async fn handle_desktop(
        &self,
        action: &str,
        params: &Value,
        context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let host = context.computer_use_host.as_ref().ok_or_else(|| {
            NortHingError::tool("Desktop control is only available in the northhing desktop app".to_string())
        })?;

        // Legacy desktop implementation shared by the dedicated ComputerUse
        // tool while ControlHub's public desktop domain remains disabled.
        match action {
            "list_displays" => {
                let displays = host.list_displays().await?;
                let active = host.focused_display_id();
                let count = displays.len();
                return Ok(vec![ToolResult::ok(
                    json!({
                        "displays": displays,
                        "active_display_id": active,
                    }),
                    Some(format!("{} display(s) detected", count)),
                )]);
            }
            // High-leverage UX primitive: paste arbitrary text into the
            // currently focused input via the system clipboard, optionally
            // clearing first and submitting after. This collapses the
            // canonical IM/search flow:
            //
            //   clipboard_set + key_chord(cmd+v) + key_chord(return)
            //
            // ...into a single tool call. It is also the **only** robust way
            // to enter CJK / emoji / multi-line text — `type_text` goes
            // through the per-character key path and is at the mercy of
            // every IME on the host. This is exactly the pattern Codex
            // uses (`pbcopy` + cmd+v) to keep WeChat / iMessage flows
            // smooth.
            //
            // Params:
            //   - text          (required) — text to paste
            //   - clear_first   (bool, default false) — cmd+a before paste,
            //                   so the new text REPLACES whatever was there
            //   - submit        (bool, default false) — press Return after
            //                   paste; switches to "send the message" mode
            //   - submit_keys   (array, default ["return"]) — override the
            //                   submit chord (e.g. ["command","return"] for
            //                   Slack / multi-line apps)
            //
            // Returns the same envelope as a `key_chord` so the model can
            // chain a verification screenshot exactly as before.
            "paste" => {
                let text = params
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        NortHingError::tool(
                            "[INVALID_PARAMS] desktop.paste requires 'text'\nHints: example { \"action\":\"paste\", \"text\":\"hello\", \"submit\":true }"
                                .to_string(),
                        )
                    })?;
                let clear_first = params.get("clear_first").and_then(|v| v.as_bool()).unwrap_or(false);
                let submit = params.get("submit").and_then(|v| v.as_bool()).unwrap_or(false);
                let submit_keys: Vec<String> = match params.get("submit_keys") {
                    Some(Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
                    Some(Value::String(s)) => vec![s.to_string()],
                    _ => vec!["return".to_string()],
                };

                if let Err(e) = clipboard_write(text).await {
                    return Ok(err_response(
                        "desktop",
                        "paste",
                        ControlHubError::new(ErrorCode::NotAvailable, format!("Clipboard write failed: {}", e))
                            .with_hint(
                                "Fall back to type_text or check that wl-clipboard / xclip is installed (Linux only)",
                            ),
                    ));
                }

                let paste_chord = match std::env::consts::OS {
                    "macos" => vec!["command".to_string(), "v".to_string()],
                    _ => vec!["control".to_string(), "v".to_string()],
                };

                if clear_first {
                    let select_all = match std::env::consts::OS {
                        "macos" => vec!["command".to_string(), "a".to_string()],
                        _ => vec!["control".to_string(), "a".to_string()],
                    };
                    host.key_chord(select_all).await?;
                }
                host.key_chord(paste_chord).await?;
                if submit {
                    host.computer_use_trust_pointer_after_text_input();
                    host.key_chord(submit_keys.clone()).await?;
                }

                let summary = match (clear_first, submit) {
                    (false, false) => format!("Pasted {} chars", text.chars().count()),
                    (true, false) => {
                        format!("Replaced focused field with {} chars", text.chars().count())
                    }
                    (false, true) => format!("Pasted {} chars and submitted", text.chars().count()),
                    (true, true) => {
                        format!("Replaced + submitted ({} chars)", text.chars().count())
                    }
                };
                return Ok(vec![ToolResult::ok(
                    json!({
                        "success": true,
                        "action": "paste",
                        "char_count": text.chars().count(),
                        "byte_length": text.len(),
                        "clear_first": clear_first,
                        "submitted": submit,
                        "submit_keys": if submit { Some(submit_keys) } else { None },
                    }),
                    Some(summary),
                )]);
            }

            // ── AX-first actions (Codex parity) ───────────────────────
            // These operate on the typed AppSelector / AxNode envelope.
            "list_apps"
            | "get_app_state"
            | "app_click"
            | "app_type_text"
            | "app_scroll"
            | "app_key_chord"
            | "app_wait_for"
            | "build_interactive_view"
            | "interactive_click"
            | "interactive_type_text"
            | "interactive_scroll"
            | "build_visual_mark_view"
            | "visual_click" => {
                return self.handle_desktop_ax(host, action, params).await;
            }
            "focus_display" => {
                // Accept `null` (or omitted `display_id`) to clear the pin
                // and fall back to "screen under the pointer". An explicit
                // numeric id pins that display until cleared.
                let display_id = match params.get("display_id") {
                    Some(Value::Null) | None => None,
                    Some(v) => Some(v.as_u64().ok_or_else(|| {
                        NortHingError::tool(
                            "focus_display: 'display_id' must be a non-negative integer or null".to_string(),
                        )
                    })? as u32),
                };
                host.focus_display(display_id).await?;
                let displays = host.list_displays().await?;
                let summary = match display_id {
                    Some(id) => format!("Pinned display {}", id),
                    None => "Cleared display pin (will follow mouse)".to_string(),
                };
                return Ok(vec![ToolResult::ok(
                    json!({
                        "active_display_id": display_id,
                        "displays": displays,
                    }),
                    Some(summary),
                )]);
            }
            _ => {}
        }

        if let Some(err) = self.desktop_action_targets_browser(action, context).await {
            return Ok(err_response("desktop", action, err));
        }

        // UX shortcut: every screen-coordinate action accepts an optional
        // `display_id`. If present (and different from the currently pinned
        // display), pin it BEFORE forwarding so the model doesn't need a
        // separate `focus_display` round-trip. Pin is sticky — subsequent
        // actions on the same screen don't need to re-specify. Pass
        // `display_id: null` to clear the pin in the same call.
        if let Some(v) = params.get("display_id") {
            let target = match v {
                Value::Null => None,
                v => Some(v.as_u64().ok_or_else(|| {
                    NortHingError::tool("display_id must be a non-negative integer or null".to_string())
                })? as u32),
            };
            if host.focused_display_id() != target {
                host.focus_display(target).await?;
            }
        }

        let mut cu_input = params.clone();
        if let Value::Object(ref mut map) = cu_input {
            map.insert("action".to_string(), json!(action));
            // Strip the ControlHub-only field so the legacy ComputerUseTool
            // doesn't trip on an unrecognised parameter.
            map.remove("display_id");
        }

        let cu_tool = super::super::computer_use_tool::ComputerUseTool::new();
        cu_tool.call_impl(&cu_input, context).await
    }
}
