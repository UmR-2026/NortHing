//! Non-target action handlers for `click` / `mouse_move` / `scroll` / `drag` /
//! `pointer_move_rel` / `key_chord` / `type_text` / `wait` / `open_app` /
//! `run_apple_script`, plus the three legacy free-function entrypoints
//! (`computer_use_execute_mouse_precise`, `_step`, `_click_tool`) consumed
//! by the dedicated `computer_use_mouse_*_tool.rs` modules.

use super::super::computer_use_input::{
    coordinate_mode, ensure_pointer_move_uses_screen_coordinates_only, use_screen_coordinates,
};
use super::metadata::computer_use_augment_result_json;
use super::validation::{computer_use_snapshot_coordinate_basis, ensure_global_xy_on_display, req_i32};
use crate::agentic::tools::computer_use_host::ComputerUseHost;
use crate::agentic::tools::framework::{ToolResult, ToolUseContext};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

use super::ComputerUseTool;

impl ComputerUseTool {
    /// `click` — press at the **current pointer only**; use `mouse_move` /
    /// `move_to_text` separately to position first.
    pub(crate) async fn click_impl(
        host_ref: &dyn ComputerUseHost,
        input: &Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        Self::ensure_click_has_no_coordinate_fields_impl(input)?;

        let button = input.get("button").and_then(|v| v.as_str()).unwrap_or("left");
        let num_clicks = input
            .get("num_clicks")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .clamp(1, 3) as u32;

        host_ref.computer_use_guard_click_allowed()?;

        for _ in 0..num_clicks {
            host_ref.mouse_click_authoritative(button).await?;
        }

        let click_label = match num_clicks {
            2 => "double",
            3 => "triple",
            _ => "single",
        };
        let input_coords = json!({
            "kind": "click",
            "button": button,
            "num_clicks": num_clicks,
            "at_current_pointer_only": true,
        });
        let body = computer_use_augment_result_json(
            host_ref,
            json!({
                "success": true,
                "action": "click",
                "button": button,
                "num_clicks": num_clicks,
            }),
            Some(input_coords),
        )
        .await;
        let summary = format!("{} {} click at current pointer only (no move).", button, click_label);
        Ok(vec![ToolResult::ok(body, Some(summary))])
    }

    /// `mouse_move` — absolute pointer move; consolidated from `ComputerUseMousePrecise`.
    pub(crate) async fn mouse_move_impl(
        host_ref: &dyn ComputerUseHost,
        input: &Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        ensure_pointer_move_uses_screen_coordinates_only(input)?;
        let x = req_i32(input, "x")?;
        let y = req_i32(input, "y")?;
        let (sx64, sy64) = Self::resolve_xy_f64_impl(host_ref, input, x, y)?;
        if use_screen_coordinates(input) {
            ensure_global_xy_on_display(host_ref, sx64, sy64).await?;
        }
        host_ref.mouse_move_global_f64(sx64, sy64).await?;
        let mode = coordinate_mode(input);
        let use_screen = use_screen_coordinates(input);
        let input_coords = json!({
            "kind": "mouse_move",
            "raw": { "x": x, "y": y, "coordinate_mode": mode, "use_screen_coordinates": use_screen },
            "resolved_global": { "x": sx64, "y": sy64 },
        });
        let body = computer_use_augment_result_json(
            host_ref,
            json!({
                "success": true,
                "action": "mouse_move",
                "x": x, "y": y,
                "pointer_x": sx64.round() as i32,
                "pointer_y": sy64.round() as i32,
                "coordinate_mode": mode,
                "use_screen_coordinates": use_screen,
            }),
            Some(input_coords),
        )
        .await;
        let summary = format!("Moved pointer to (~{}, ~{}).", sx64.round() as i32, sy64.round() as i32);
        Ok(vec![ToolResult::ok(body, Some(summary))])
    }

    /// `scroll` — mouse wheel delta at current pointer (optionally after a
    /// `scroll_x` / `scroll_y` reposition).
    pub(crate) async fn scroll_impl(
        host_ref: &dyn ComputerUseHost,
        input: &Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let dx = input.get("delta_x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let dy = input.get("delta_y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        if dx == 0 && dy == 0 {
            return Err(NortHingError::tool(
                "scroll requires non-zero delta_x and/or delta_y".to_string(),
            ));
        }
        // Positional scroll: move pointer to target before scrolling.
        let scroll_pos_x = input.get("scroll_x").and_then(|v| v.as_i64());
        let scroll_pos_y = input.get("scroll_y").and_then(|v| v.as_i64());
        if let (Some(sx), Some(sy)) = (scroll_pos_x, scroll_pos_y) {
            host_ref.mouse_move_global_f64(sx as f64, sy as f64).await?;
            host_ref.wait_ms(30).await?;
        }
        host_ref.scroll(dx, dy).await?;
        let input_coords = json!({ "kind": "scroll", "delta_x": dx, "delta_y": dy });
        let body = computer_use_augment_result_json(
            host_ref,
            json!({ "success": true, "action": "scroll", "delta_x": dx, "delta_y": dy }),
            Some(input_coords),
        )
        .await;
        let summary = format!("Scrolled ({}, {}).", dx, dy);
        Ok(vec![ToolResult::ok(body, Some(summary))])
    }

    /// `drag` — mouse_down at start + move to end + mouse_up.
    pub(crate) async fn drag_impl(
        host_ref: &dyn ComputerUseHost,
        input: &Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        ensure_pointer_move_uses_screen_coordinates_only(input)?;
        let start_x = req_i32(input, "start_x")?;
        let start_y = req_i32(input, "start_y")?;
        let end_x = req_i32(input, "end_x")?;
        let end_y = req_i32(input, "end_y")?;
        let button = input.get("button").and_then(|v| v.as_str()).unwrap_or("left");

        let (sx0, sy0) = Self::resolve_xy_f64_impl(host_ref, input, start_x, start_y)?;
        let (sx1, sy1) = Self::resolve_xy_f64_impl(host_ref, input, end_x, end_y)?;

        // Move to start, press, move to end, release.
        host_ref.mouse_move_global_f64(sx0, sy0).await?;
        host_ref.mouse_down(button).await?;
        // Small pause for apps that need time to register the press.
        host_ref.wait_ms(50).await?;
        host_ref.mouse_move_global_f64(sx1, sy1).await?;
        host_ref.wait_ms(50).await?;
        host_ref.mouse_up(button).await?;
        ComputerUseHost::computer_use_after_committed_ui_action(host_ref);

        let input_coords = json!({
            "kind": "drag",
            "start": { "x": start_x, "y": start_y },
            "end": { "x": end_x, "y": end_y },
            "button": button,
        });
        let body = computer_use_augment_result_json(
            host_ref,
            json!({
                "success": true,
                "action": "drag",
                "start_global": { "x": sx0.round() as i32, "y": sy0.round() as i32 },
                "end_global": { "x": sx1.round() as i32, "y": sy1.round() as i32 },
                "button": button,
            }),
            Some(input_coords),
        )
        .await;
        let summary = format!(
            "Dragged from (~{}, ~{}) to (~{}, ~{}).",
            sx0.round() as i32,
            sy0.round() as i32,
            sx1.round() as i32,
            sy1.round() as i32,
        );
        Ok(vec![ToolResult::ok(body, Some(summary))])
    }

    /// `pointer_move_rel` — relative nudge by `delta_x` / `delta_y` (or `dx` / `dy` aliases).
    pub(crate) async fn pointer_move_rel_impl(
        host_ref: &dyn ComputerUseHost,
        input: &Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        // Accept both `delta_x`/`delta_y` (canonical) and `dx`/`dy` (alias) so that
        // models which guess the natural form do not crash on the schema.
        let dx_alias_used = input.get("delta_x").is_none() && input.get("dx").is_some();
        let dy_alias_used = input.get("delta_y").is_none() && input.get("dy").is_some();
        let dx = input
            .get("delta_x")
            .or_else(|| input.get("dx"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let dy = input
            .get("delta_y")
            .or_else(|| input.get("dy"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        if dx == 0 && dy == 0 {
            return Err(NortHingError::tool(
                "pointer_move_rel requires a non-zero delta. Accepts `delta_x`|`dx` and `delta_y`|`dy` (screen pixels); at least one must be non-zero.".to_string(),
            ));
        }
        host_ref.pointer_move_relative(dx, dy).await?;
        let alias_note = match (dx_alias_used, dy_alias_used) {
            (true, true) => Some("dx|dy"),
            (true, false) => Some("dx"),
            (false, true) => Some("dy"),
            (false, false) => None,
        };
        let mut input_coords = json!({
            "kind": "pointer_move_rel",
            "delta_x": dx,
            "delta_y": dy,
        });
        if let Some(a) = alias_note {
            input_coords["deprecated_alias_used"] = json!(a);
        }
        let mut payload = json!({
            "success": true,
            "action": "pointer_move_rel",
            "delta_x": dx,
            "delta_y": dy,
        });
        if let Some(a) = alias_note {
            payload["deprecated_alias_used"] = json!(a);
        }
        let body = computer_use_augment_result_json(host_ref, payload, Some(input_coords)).await;
        let summary = format!("Moved pointer relatively by ({}, {}) screen pixels.", dx, dy);
        Ok(vec![ToolResult::ok(body, Some(summary))])
    }

    /// `key_chord` — send a key combination (modifiers first, then main key).
    pub(crate) async fn key_chord_impl(
        host_ref: &dyn ComputerUseHost,
        input: &Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        // UX: accept BOTH `keys: ["escape"]` (canonical) AND
        // `keys: "escape"` / `key: "escape"` (common mistakes from
        // the model). The wrong-shape variants are silently
        // coerced — in practice every regression caused by being
        // strict here costs a full round-trip to fix. Genuine
        // missing-keys is reported with an explicit example so
        // the model recovers in one shot.
        let keys: Vec<String> = match input.get("keys") {
            Some(Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
            Some(Value::String(s)) => vec![s.to_string()],
            None => match input.get("key").and_then(|v| v.as_str()) {
                Some(s) => vec![s.to_string()],
                None => {
                    return Err(NortHingError::tool(
                        "[INVALID_PARAMS] key_chord requires `keys` as a JSON array of key names\nHints: example { \"keys\": [\"command\", \"v\"] } | for a single key { \"keys\": [\"return\"] } | use lowercase canonical names: command, control, option, shift, return, escape, tab, space, delete, arrow_up/down/left/right, f1..f12"
                            .to_string(),
                    ));
                }
            },
            _ => {
                return Err(NortHingError::tool(
                    "[INVALID_PARAMS] key_chord `keys` must be a string or array of strings\nHints: example { \"keys\": [\"command\", \"v\"] }".to_string(),
                ));
            }
        };
        if keys.is_empty() {
            return Err(NortHingError::tool(
                "[INVALID_PARAMS] key_chord `keys` must not be empty\nHints: example { \"keys\": [\"return\"] }"
                    .to_string(),
            ));
        }
        host_ref.key_chord(keys.clone()).await?;
        let input_coords = json!({ "kind": "key_chord", "keys": keys });
        let body = computer_use_augment_result_json(
            host_ref,
            json!({ "success": true, "action": "key_chord", "keys": keys }),
            Some(input_coords),
        )
        .await;
        let summary = "Key chord sent.".to_string();
        Ok(vec![ToolResult::ok(body, Some(summary))])
    }

    /// `type_text` — type text into the currently focused target.
    pub(crate) async fn type_text_impl(
        host_ref: &dyn ComputerUseHost,
        input: &Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let text = input
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NortHingError::tool("text is required".to_string()))?;
        host_ref.type_text(text).await?;
        let input_coords = json!({ "kind": "type_text", "char_count": text.chars().count() });
        let body = computer_use_augment_result_json(
            host_ref,
            json!({ "success": true, "action": "type_text", "chars": text.chars().count() }),
            Some(input_coords),
        )
        .await;
        let summary = format!("Typed {} character(s) into the focused target.", text.chars().count());
        Ok(vec![ToolResult::ok(body, Some(summary))])
    }

    /// `wait` — sleep for `ms` milliseconds (UI animation buffer).
    pub(crate) async fn wait_impl(
        host_ref: &dyn ComputerUseHost,
        input: &Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let ms = input
            .get("ms")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| NortHingError::tool("ms is required".to_string()))?;
        host_ref.wait_ms(ms).await?;
        let body =
            computer_use_augment_result_json(host_ref, json!({ "success": true, "action": "wait", "ms": ms }), None)
                .await;
        Ok(vec![ToolResult::ok(body, Some(format!("Waited {} ms.", ms)))])
    }

    /// `open_app` — launch an application by name.
    pub(crate) async fn open_app_impl(
        host_ref: &dyn ComputerUseHost,
        input: &Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let app_name = input
            .get("app_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NortHingError::tool("open_app requires `app_name` parameter.".to_string()))?;
        let result = host_ref.open_app(app_name).await?;
        let body = computer_use_augment_result_json(
            host_ref,
            json!({
                "success": result.success,
                "action": "open_app",
                "app_name": result.app_name,
                "process_id": result.process_id,
                "error_message": result.error_message,
            }),
            None,
        )
        .await;
        let summary = if result.success {
            format!(
                "Opened app '{}'{}.",
                result.app_name,
                result.process_id.map(|p| format!(" (PID {})", p)).unwrap_or_default()
            )
        } else {
            format!(
                "Failed to open '{}': {}",
                result.app_name,
                result.error_message.as_deref().unwrap_or("unknown error")
            )
        };
        Ok(vec![ToolResult::ok(body, Some(summary))])
    }

    /// `run_apple_script` — execute arbitrary AppleScript via `osascript` (macOS only).
    pub(crate) async fn run_apple_script_impl(
        #[cfg_attr(not(target_os = "macos"), allow(unused_variables))] host_ref: &dyn ComputerUseHost,
        input: &Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let script = input
            .get("script")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NortHingError::tool("run_apple_script requires `script` parameter.".to_string()))?;
        #[cfg(not(target_os = "macos"))]
        {
            let _ = script;
            return Err(NortHingError::tool(
                "run_apple_script is only available on macOS.".to_string(),
            ));
        }
        #[cfg(target_os = "macos")]
        {
            let script_owned = script.to_string();
            let output = tokio::task::spawn_blocking(move || {
                std::process::Command::new("/usr/bin/osascript")
                    .args(["-e", &script_owned])
                    .output()
            })
            .await
            .map_err(|e| NortHingError::tool(format!("spawn: {}", e)))?
            .map_err(|e| NortHingError::tool(format!("osascript: {}", e)))?;

            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let success = output.status.success();

            let body = computer_use_augment_result_json(
                host_ref,
                json!({
                    "success": success,
                    "action": "run_apple_script",
                    "stdout": stdout,
                    "stderr": stderr,
                }),
                None,
            )
            .await;
            let summary = if success {
                format!(
                    "AppleScript executed.{}",
                    if stdout.is_empty() {
                        String::new()
                    } else {
                        format!(" Output: {}", crate::util::truncate_at_char_boundary(&stdout, 200))
                    }
                )
            } else {
                format!(
                    "AppleScript error: {}",
                    crate::util::truncate_at_char_boundary(&stderr, 200)
                )
            };
            Ok(vec![ToolResult::ok(body, Some(summary))])
        }
    }
}

// --- Legacy free-function entrypoints consumed by the dedicated
//     `computer_use_mouse_*_tool.rs` modules. Re-exported through `mod.rs`.

/// Absolute pointer move (`ComputerUseMousePrecise` tool).
pub(crate) async fn computer_use_execute_mouse_precise(
    host_ref: &dyn ComputerUseHost,
    input: &Value,
) -> NortHingResult<Vec<ToolResult>> {
    ensure_pointer_move_uses_screen_coordinates_only(input)?;
    let snapshot_basis = computer_use_snapshot_coordinate_basis(host_ref);
    let x = req_i32(input, "x")?;
    let y = req_i32(input, "y")?;
    let mode = coordinate_mode(input);
    let use_screen = use_screen_coordinates(input);
    let (sx64, sy64) = ComputerUseTool::resolve_xy_f64_impl(host_ref, input, x, y)?;
    if use_screen {
        ensure_global_xy_on_display(host_ref, sx64, sy64).await?;
    }
    host_ref.mouse_move_global_f64(sx64, sy64).await?;
    let sx = sx64.round() as i32;
    let sy = sy64.round() as i32;
    let input_coords = json!({
        "kind": "mouse_precise",
        "raw": { "x": x, "y": y, "coordinate_mode": mode, "use_screen_coordinates": use_screen },
        "resolved_global": { "x": sx64, "y": sy64 }
    });
    let body = computer_use_augment_result_json(
        host_ref,
        json!({
            "success": true,
            "tool": "ComputerUseMousePrecise",
            "positioning": "absolute",
            "x": x,
            "y": y,
            "pointer_x": sx,
            "pointer_y": sy,
            "coordinate_mode": mode,
            "use_screen_coordinates": use_screen,
            "snapshot_coordinate_basis": snapshot_basis,
        }),
        Some(input_coords),
    )
    .await;
    let summary = format!(
        "Moved pointer to global screen (~{}, ~{}, sub-point on macOS) (input {:?} {}, {}).",
        sx, sy, mode, x, y
    );
    Ok(vec![ToolResult::ok(body, Some(summary))])
}

/// Cardinal step move (`ComputerUseMouseStep` tool). Same pixel space as `pointer_move_rel`.
pub(crate) async fn computer_use_execute_mouse_step(
    host_ref: &dyn ComputerUseHost,
    input: &Value,
) -> NortHingResult<Vec<ToolResult>> {
    let dir = input.get("direction").and_then(|v| v.as_str()).ok_or_else(|| {
        NortHingError::tool("direction is required for ComputerUseMouseStep (up|down|left|right)".to_string())
    })?;
    let px = input
        .get("pixels")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .unwrap_or(32)
        .clamp(1, 400);
    let (dx, dy) = match dir.to_lowercase().as_str() {
        "up" => (0, -px),
        "down" => (0, px),
        "left" => (-px, 0),
        "right" => (px, 0),
        _ => {
            return Err(NortHingError::tool(
                "direction must be up, down, left, or right".to_string(),
            ));
        }
    };
    host_ref.pointer_move_relative(dx, dy).await?;
    let input_coords = json!({
        "kind": "mouse_step",
        "direction": dir,
        "pixels": px,
        "delta_x": dx,
        "delta_y": dy
    });
    let body = computer_use_augment_result_json(
        host_ref,
        json!({
            "success": true,
            "tool": "ComputerUseMouseStep",
            "direction": dir,
            "pixels": px,
            "delta_x": dx,
            "delta_y": dy,
        }),
        Some(input_coords),
    )
    .await;
    let summary = format!("Stepped pointer by ({}, {}) px (direction {}, {} px).", dx, dy, dir, px);
    Ok(vec![ToolResult::ok(body, Some(summary))])
}

/// Click and mouse-wheel at the **current** pointer (`ComputerUseMouseClick` tool).
pub(crate) async fn computer_use_execute_mouse_click_tool(
    host_ref: &dyn ComputerUseHost,
    input: &Value,
) -> NortHingResult<Vec<ToolResult>> {
    let act = input
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| NortHingError::tool("action is required (click or wheel)".to_string()))?;
    match act {
        "click" => {
            let button = input.get("button").and_then(|v| v.as_str()).unwrap_or("left");
            let num_clicks = input
                .get("num_clicks")
                .and_then(|v| v.as_u64())
                .unwrap_or(1)
                .clamp(1, 3) as u32;
            for _ in 0..num_clicks {
                host_ref.mouse_click(button).await?;
            }
            let click_label = match num_clicks {
                2 => "double",
                3 => "triple",
                _ => "single",
            };
            let input_coords =
                json!({ "kind": "mouse_click", "action": "click", "button": button, "num_clicks": num_clicks });
            let body = computer_use_augment_result_json(
                host_ref,
                json!({
                    "success": true,
                    "tool": "ComputerUseMouseClick",
                    "action": "click",
                    "button": button,
                    "num_clicks": num_clicks,
                }),
                Some(input_coords),
            )
            .await;
            let summary = format!("{} {} click at current pointer (does not move).", button, click_label);
            Ok(vec![ToolResult::ok(body, Some(summary))])
        }
        "wheel" => {
            let dx = input.get("delta_x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let dy = input.get("delta_y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            if dx == 0 && dy == 0 {
                return Err(NortHingError::tool(
                    "wheel requires non-zero delta_x and/or delta_y".to_string(),
                ));
            }
            host_ref.scroll(dx, dy).await?;
            let input_coords = json!({
                "kind": "mouse_click",
                "action": "wheel",
                "delta_x": dx,
                "delta_y": dy
            });
            let body = computer_use_augment_result_json(
                host_ref,
                json!({
                    "success": true,
                    "tool": "ComputerUseMouseClick",
                    "action": "wheel",
                    "delta_x": dx,
                    "delta_y": dy,
                }),
                Some(input_coords),
            )
            .await;
            let summary = format!("Mouse wheel at pointer: delta ({}, {}).", dx, dy);
            Ok(vec![ToolResult::ok(body, Some(summary))])
        }
        _ => Err(NortHingError::tool(
            "ComputerUseMouseClick action must be \"click\" or \"wheel\"".to_string(),
        )),
    }
}
