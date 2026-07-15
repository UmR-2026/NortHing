//! Cross-sibling helpers for desktop_ax_actions.
//!
//! These functions are shared by the per-action sibling modules
//! (`ax_query` / `ax_click` / `ax_input`) and live here so each
//! action-body file stays focused on its dispatch arm.

use crate::agentic::tools::computer_use_host::{AppSelector, AppWaitPredicate, ClickTarget};
use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::{NortHingError, NortHingResult};
use base64::Engine as _;
use serde_json::{json, Value};

/// Per-PID consecutive-failure tracker for the AX-first `app_*` actions.
/// Key = target PID, value = `(target_signature, before_digest, count)`.
/// When the same `(action,target)` lands on an unchanged digest twice in a
/// row the dispatcher injects an `app_state.loop_warning` so the model is
/// forced off the failing path on its **next** turn (`/Screenshot policy/
/// Mandatory screenshot moments` in `claw_mode.md`).
pub(super) static APP_LOOP_TRACKER: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<i32, (String, String, u32)>>,
> = std::sync::OnceLock::new();

pub(super) fn loop_tracker_observe(
    pid: Option<i32>,
    action: &str,
    target_sig: &str,
    before_digest: &str,
    after_digest: &str,
) -> Option<String> {
    let pid = pid?;
    // A digest change means the action mutated the tree — that is real
    // progress and resets the streak even if the model picks the same
    // target name on purpose (e.g. clicking "Next" repeatedly).
    let progressed = before_digest != after_digest;
    let sig = format!("{action}:{target_sig}");
    let mut guard = APP_LOOP_TRACKER
        .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
        .lock()
        .ok()?;
    let entry = guard.entry(pid).or_insert_with(|| (String::new(), String::new(), 0));
    if progressed {
        *entry = (sig, after_digest.to_string(), 1);
        return None;
    }
    if entry.0 == sig && entry.1 == before_digest {
        entry.2 = entry.2.saturating_add(1);
    } else {
        *entry = (sig, before_digest.to_string(), 1);
    }
    if entry.2 >= 2 {
        Some(format!(
            "Detected {} consecutive `{}` calls on the same target ({}) without any AX tree mutation (digest unchanged). The target is almost certainly invisible / disabled / in a Canvas-WebGL surface that AX cannot describe. NEXT TURN you MUST: (1) run `desktop.screenshot {{ screenshot_window: false }}` to see the full display, (2) switch tactic — different `node_idx`, different `ocr_text` needle, or a keyboard shortcut.",
            entry.2, action, target_sig
        ))
    } else {
        None
    }
}

pub(super) fn parse_selector(v: &Value) -> NortHingResult<AppSelector> {
    let obj = v.get("app").ok_or_else(|| {
        NortHingError::tool("[INVALID_PARAMS] missing 'app' selector (pid|bundle_id|name)".to_string())
    })?;
    let sel: AppSelector = serde_json::from_value(obj.clone()).map_err(|e| {
        NortHingError::tool(format!(
            "[INVALID_PARAMS] bad 'app' selector: {} (expect {{pid|bundle_id|name}})",
            e
        ))
    })?;
    if sel.pid.is_none() && sel.bundle_id.is_none() && sel.name.is_none() {
        return Err(NortHingError::tool(
            "[INVALID_PARAMS] 'app' must include at least one of pid|bundle_id|name".to_string(),
        ));
    }
    Ok(sel)
}

pub(super) fn parse_click_target(v: &Value) -> NortHingResult<ClickTarget> {
    if v.get("kind").is_some() {
        return serde_json::from_value(v.clone()).map_err(|e| {
            NortHingError::tool(format!(
                "[INVALID_PARAMS] bad ClickTarget: {} (expected {{\"kind\":\"node_idx\",\"idx\":N}}, {{\"kind\":\"image_xy\",\"x\":0,\"y\":0}}, {{\"kind\":\"image_grid\",\"x0\":0,\"y0\":0,\"width\":300,\"height\":300,\"rows\":15,\"cols\":15,\"row\":7,\"col\":7,\"intersections\":true}}, {{\"kind\":\"visual_grid\",\"rows\":15,\"cols\":15,\"row\":7,\"col\":7,\"intersections\":true}}, {{\"kind\":\"screen_xy\",\"x\":0,\"y\":0}}, or {{\"kind\":\"ocr_text\",\"needle\":\"...\"}})",
                e
            ))
        });
    }
    if let Some(idx) = v.get("node_idx").and_then(|x| x.as_u64()) {
        return Ok(ClickTarget::NodeIdx { idx: idx as u32 });
    }
    if let Some(obj) = v.get("screen_xy") {
        let x = obj
            .get("x")
            .and_then(|x| x.as_f64())
            .ok_or_else(|| NortHingError::tool("[INVALID_PARAMS] screen_xy target requires numeric x".to_string()))?;
        let y = obj
            .get("y")
            .and_then(|y| y.as_f64())
            .ok_or_else(|| NortHingError::tool("[INVALID_PARAMS] screen_xy target requires numeric y".to_string()))?;
        return Ok(ClickTarget::ScreenXy { x, y });
    }
    if let Some(obj) = v.get("image_xy") {
        let x = obj
            .get("x")
            .and_then(|x| x.as_i64())
            .ok_or_else(|| NortHingError::tool("[INVALID_PARAMS] image_xy target requires integer x".to_string()))?;
        let y = obj
            .get("y")
            .and_then(|y| y.as_i64())
            .ok_or_else(|| NortHingError::tool("[INVALID_PARAMS] image_xy target requires integer y".to_string()))?;
        return Ok(ClickTarget::ImageXy {
            x: x as i32,
            y: y as i32,
            screenshot_id: obj.get("screenshot_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
        });
    }
    if let Some(obj) = v.get("image_grid") {
        let target = json!({
            "kind": "image_grid",
            "x0": obj.get("x0").cloned().unwrap_or(Value::Null),
            "y0": obj.get("y0").cloned().unwrap_or(Value::Null),
            "width": obj.get("width").cloned().unwrap_or(Value::Null),
            "height": obj.get("height").cloned().unwrap_or(Value::Null),
            "rows": obj.get("rows").cloned().unwrap_or(Value::Null),
            "cols": obj.get("cols").cloned().unwrap_or(Value::Null),
            "row": obj.get("row").cloned().unwrap_or(Value::Null),
            "col": obj.get("col").cloned().unwrap_or(Value::Null),
            "intersections": obj.get("intersections").cloned().unwrap_or(json!(false)),
            "screenshot_id": obj.get("screenshot_id").cloned().unwrap_or(Value::Null),
        });
        return serde_json::from_value(target).map_err(|e| {
            NortHingError::tool(format!(
                "[INVALID_PARAMS] bad image_grid target: {} (need x0,y0,width,height,rows,cols,row,col; optional intersections)",
                e
            ))
        });
    }
    if let Some(obj) = v.get("visual_grid") {
        let target = json!({
            "kind": "visual_grid",
            "rows": obj.get("rows").cloned().unwrap_or(Value::Null),
            "cols": obj.get("cols").cloned().unwrap_or(Value::Null),
            "row": obj.get("row").cloned().unwrap_or(Value::Null),
            "col": obj.get("col").cloned().unwrap_or(Value::Null),
            "intersections": obj.get("intersections").cloned().unwrap_or(json!(false)),
            "wait_ms_after_detection": obj.get("wait_ms_after_detection").cloned().unwrap_or(Value::Null),
        });
        return serde_json::from_value(target).map_err(|e| {
            NortHingError::tool(format!(
                "[INVALID_PARAMS] bad visual_grid target: {} (need rows,cols,row,col; optional intersections)",
                e
            ))
        });
    }
    if v.get("x").is_some() || v.get("y").is_some() {
        let x = v
            .get("x")
            .and_then(|x| x.as_f64())
            .ok_or_else(|| NortHingError::tool("[INVALID_PARAMS] screen target requires numeric x".to_string()))?;
        let y = v
            .get("y")
            .and_then(|y| y.as_f64())
            .ok_or_else(|| NortHingError::tool("[INVALID_PARAMS] screen target requires numeric y".to_string()))?;
        return Ok(ClickTarget::ScreenXy { x, y });
    }
    if let Some(ocr) = v.get("ocr_text") {
        let needle = ocr
            .get("needle")
            .or_else(|| ocr.get("text"))
            .and_then(|x| x.as_str())
            .ok_or_else(|| NortHingError::tool("[INVALID_PARAMS] ocr_text target requires needle".to_string()))?;
        return Ok(ClickTarget::OcrText {
            needle: needle.to_string(),
        });
    }
    Err(NortHingError::tool(
        "[INVALID_PARAMS] unsupported ClickTarget. Use {\"kind\":\"node_idx\",\"idx\":N}, {\"node_idx\":N}, {\"kind\":\"image_xy\",\"x\":0,\"y\":0}, {\"image_xy\":{\"x\":0,\"y\":0}}, {\"kind\":\"image_grid\",\"x0\":0,\"y0\":0,\"width\":300,\"height\":300,\"rows\":15,\"cols\":15,\"row\":7,\"col\":7,\"intersections\":true}, {\"kind\":\"visual_grid\",\"rows\":15,\"cols\":15,\"row\":7,\"col\":7,\"intersections\":true}, {\"kind\":\"screen_xy\",\"x\":0,\"y\":0}, or {\"ocr_text\":{\"needle\":\"...\"}}.".to_string(),
    ))
}

pub(super) fn parse_wait_predicate(v: &Value) -> NortHingResult<AppWaitPredicate> {
    if v.get("kind").is_some() {
        return serde_json::from_value(v.clone())
            .map_err(|e| NortHingError::tool(format!("[INVALID_PARAMS] bad app_wait_for predicate: {}", e)));
    }
    if let Some(obj) = v.get("digest_changed") {
        let prev_digest = obj
            .get("prev_digest")
            .or_else(|| obj.get("from"))
            .and_then(|x| x.as_str())
            .ok_or_else(|| NortHingError::tool("[INVALID_PARAMS] digest_changed requires prev_digest".to_string()))?;
        return Ok(AppWaitPredicate::DigestChanged {
            prev_digest: prev_digest.to_string(),
        });
    }
    if let Some(obj) = v.get("title_contains") {
        let needle = obj
            .get("needle")
            .or_else(|| obj.get("title"))
            .and_then(|x| x.as_str())
            .or_else(|| obj.as_str())
            .ok_or_else(|| NortHingError::tool("[INVALID_PARAMS] title_contains requires needle".to_string()))?;
        return Ok(AppWaitPredicate::TitleContains {
            needle: needle.to_string(),
        });
    }
    if let Some(obj) = v.get("role_enabled") {
        let role = obj
            .get("role")
            .and_then(|x| x.as_str())
            .ok_or_else(|| NortHingError::tool("[INVALID_PARAMS] role_enabled requires role".to_string()))?;
        return Ok(AppWaitPredicate::RoleEnabled { role: role.to_string() });
    }
    if let Some(obj) = v.get("node_enabled") {
        let idx = obj
            .get("idx")
            .and_then(|x| x.as_u64())
            .or_else(|| obj.as_u64())
            .ok_or_else(|| NortHingError::tool("[INVALID_PARAMS] node_enabled requires idx".to_string()))?;
        return Ok(AppWaitPredicate::NodeEnabled { idx: idx as u32 });
    }
    Err(NortHingError::tool(
        "[INVALID_PARAMS] unsupported app_wait_for predicate. Use {\"kind\":\"digest_changed\",\"prev_digest\":\"...\"} or shorthand {\"digest_changed\":{\"prev_digest\":\"...\"}}.".to_string(),
    ))
}

pub(super) fn parse_keys(v: &Value) -> Vec<String> {
    match v.get("keys").or_else(|| v.get("key")) {
        Some(Value::Array(arr)) => arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(),
        Some(Value::String(s)) => vec![s.to_string()],
        _ => Vec::new(),
    }
}

// Build the JSON view of an AppStateSnapshot for the model. Excludes
// the heavy `screenshot` payload (it is attached out-of-band as a
// multimodal image, not as base64 inside the JSON tree, to keep token
// budgets under control and let the provider deliver it as `image_url`).
pub(super) fn snap_state_json(snap: &crate::agentic::tools::computer_use_host::AppStateSnapshot) -> serde_json::Value {
    let mut v = json!({
        "app": snap.app,
        "window_title": snap.window_title,
        "digest": snap.digest,
        "captured_at_ms": snap.captured_at_ms,
        "tree_text": snap.tree_text,
        "has_screenshot": snap.screenshot.is_some(),
    });
    if let Some(shot) = snap.screenshot.as_ref() {
        if let Some(obj) = v.as_object_mut() {
            let meta: serde_json::Value = json!({
                "image_width": shot.image_width,
                "image_height": shot.image_height,
                "screenshot_id": shot.screenshot_id,
                "native_width": shot.native_width,
                "native_height": shot.native_height,
                "vision_scale": shot.vision_scale,
                "mime_type": shot.mime_type,
                "image_content_rect": shot.image_content_rect,
                "image_global_bounds": shot.image_global_bounds,
                "coordinate_hint": "For visual surfaces, click pixels in this attached image with app_click target {kind:\"image_xy\", x, y, screenshot_id}. For known boards/grids/canvases, prefer {kind:\"image_grid\", x0, y0, width, height, rows, cols, row, col, intersections, screenshot_id}. If the grid rectangle is unknown, use {kind:\"visual_grid\", rows, cols, row, col, intersections}; the host detects the grid from app pixels.",
            });
            obj.insert("screenshot_meta".to_string(), meta);
        }
    }
    v
}

// Helper: build a `ToolResult` that *also* carries the focused-window
// screenshot as an Anthropic-style multimodal image attachment. When
// the host couldn't (or chose not to) capture, fall back to a regular
// text-only `ToolResult::ok`.
pub(super) fn snap_result(
    data: serde_json::Value,
    summary: Option<String>,
    snap: &crate::agentic::tools::computer_use_host::AppStateSnapshot,
) -> ToolResult {
    if let Some(shot) = snap.screenshot.as_ref() {
        let attach = crate::util::types::ToolImageAttachment {
            mime_type: shot.mime_type.clone(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(&shot.bytes),
        };
        ToolResult::ok_with_images(data, summary, vec![attach])
    } else {
        ToolResult::ok(data, summary)
    }
}

// Build a JSON view of an InteractiveView that excludes the heavy
// `screenshot.bytes` payload (the JPEG is attached out-of-band as a
// multimodal image attachment, not as base64 inside the tree).
pub(super) fn build_interactive_view_json(
    view: &crate::agentic::tools::computer_use_host::InteractiveView,
) -> serde_json::Value {
    let mut v = json!({
        "app": view.app,
        "window_title": view.window_title,
        "digest": view.digest,
        "captured_at_ms": view.captured_at_ms,
        "elements": view.elements,
        "tree_text": view.tree_text,
        "loop_warning": view.loop_warning,
        "has_screenshot": view.screenshot.is_some(),
    });
    if let Some(shot) = view.screenshot.as_ref() {
        if let Some(obj) = v.as_object_mut() {
            obj.insert(
                "screenshot_meta".to_string(),
                json!({
                    "image_width": shot.image_width,
                    "image_height": shot.image_height,
                    "screenshot_id": shot.screenshot_id,
                    "native_width": shot.native_width,
                    "native_height": shot.native_height,
                    "vision_scale": shot.vision_scale,
                    "mime_type": shot.mime_type,
                    "image_content_rect": shot.image_content_rect,
                    "image_global_bounds": shot.image_global_bounds,
                    "coordinate_hint": "Numbered overlays are in JPEG image-pixel space. Reference elements via their `i` index using interactive_click / interactive_type_text / interactive_scroll. For pointer-only fallback, pass screenshot_id with image_xy/image_grid.",
                }),
            );
        }
    }
    v
}

pub(super) fn build_visual_mark_view_json(
    view: &crate::agentic::tools::computer_use_host::VisualMarkView,
) -> serde_json::Value {
    let mut v = json!({
        "app": view.app,
        "window_title": view.window_title,
        "digest": view.digest,
        "captured_at_ms": view.captured_at_ms,
        "marks": view.marks,
        "has_screenshot": view.screenshot.is_some(),
    });
    if let Some(shot) = view.screenshot.as_ref() {
        if let Some(obj) = v.as_object_mut() {
            obj.insert(
                "screenshot_meta".to_string(),
                json!({
                    "image_width": shot.image_width,
                    "image_height": shot.image_height,
                    "screenshot_id": shot.screenshot_id,
                    "native_width": shot.native_width,
                    "native_height": shot.native_height,
                    "vision_scale": shot.vision_scale,
                    "mime_type": shot.mime_type,
                    "image_content_rect": shot.image_content_rect,
                    "image_global_bounds": shot.image_global_bounds,
                    "coordinate_hint": "Numbered visual marks are in JPEG image-pixel space. Reference marks via their `i` index using visual_click. To refine a dense area, call build_visual_mark_view again with opts.region in these screenshot pixels.",
                }),
            );
        }
    }
    v
}

// Build a JSON envelope for interactive_* action results. Includes
// the post-action AppStateSnapshot (without screenshot bytes) and,
// when present, the rebuilt InteractiveView.
pub(super) fn build_interactive_action_json(
    app: &crate::agentic::tools::computer_use_host::AppSelector,
    res: &crate::agentic::tools::computer_use_host::InteractiveActionResult,
    extras: serde_json::Value,
) -> serde_json::Value {
    let mut v = json!({
        "target_app": app,
        "app_state": snap_state_json(&res.snapshot),
        "app_state_nodes": res.snapshot.nodes,
        "loop_warning": res.snapshot.loop_warning,
        "execution_note": res.execution_note,
        "interactive_view": res.view.as_ref().map(build_interactive_view_json),
    });
    if let (Some(obj), Some(extras_obj)) = (v.as_object_mut(), extras.as_object()) {
        for (k, val) in extras_obj {
            obj.insert(k.clone(), val.clone());
        }
    }
    v
}

pub(super) fn build_visual_action_json(
    app: &crate::agentic::tools::computer_use_host::AppSelector,
    res: &crate::agentic::tools::computer_use_host::VisualActionResult,
    extras: serde_json::Value,
) -> serde_json::Value {
    let mut v = json!({
        "target_app": app,
        "app_state": snap_state_json(&res.snapshot),
        "app_state_nodes": res.snapshot.nodes,
        "loop_warning": res.snapshot.loop_warning,
        "execution_note": res.execution_note,
        "visual_mark_view": res.view.as_ref().map(build_visual_mark_view_json),
    });
    if let (Some(obj), Some(extras_obj)) = (v.as_object_mut(), extras.as_object()) {
        for (k, val) in extras_obj {
            obj.insert(k.clone(), val.clone());
        }
    }
    v
}

// Attach the InteractiveView's annotated screenshot (if present)
// as a multimodal image; otherwise fall back to text-only ok.
pub(super) fn interactive_view_result(
    data: serde_json::Value,
    summary: Option<String>,
    view: &crate::agentic::tools::computer_use_host::InteractiveView,
) -> ToolResult {
    if let Some(shot) = view.screenshot.as_ref() {
        let attach = crate::util::types::ToolImageAttachment {
            mime_type: shot.mime_type.clone(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(&shot.bytes),
        };
        ToolResult::ok_with_images(data, summary, vec![attach])
    } else {
        ToolResult::ok(data, summary)
    }
}

pub(super) fn visual_mark_view_result(
    data: serde_json::Value,
    summary: Option<String>,
    view: &crate::agentic::tools::computer_use_host::VisualMarkView,
) -> ToolResult {
    if let Some(shot) = view.screenshot.as_ref() {
        let attach = crate::util::types::ToolImageAttachment {
            mime_type: shot.mime_type.clone(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(&shot.bytes),
        };
        ToolResult::ok_with_images(data, summary, vec![attach])
    } else {
        ToolResult::ok(data, summary)
    }
}

// Prefer attaching the rebuilt interactive view's screenshot when
// available; otherwise fall back to the post-action snapshot's.
pub(super) fn interactive_action_result(
    data: serde_json::Value,
    summary: Option<String>,
    res: &crate::agentic::tools::computer_use_host::InteractiveActionResult,
) -> ToolResult {
    let shot_opt = res
        .view
        .as_ref()
        .and_then(|v| v.screenshot.as_ref())
        .or(res.snapshot.screenshot.as_ref());
    if let Some(shot) = shot_opt {
        let attach = crate::util::types::ToolImageAttachment {
            mime_type: shot.mime_type.clone(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(&shot.bytes),
        };
        ToolResult::ok_with_images(data, summary, vec![attach])
    } else {
        ToolResult::ok(data, summary)
    }
}

pub(super) fn visual_action_result(
    data: serde_json::Value,
    summary: Option<String>,
    res: &crate::agentic::tools::computer_use_host::VisualActionResult,
) -> ToolResult {
    let shot_opt = res
        .view
        .as_ref()
        .and_then(|v| v.screenshot.as_ref())
        .or(res.snapshot.screenshot.as_ref());
    if let Some(shot) = shot_opt {
        let attach = crate::util::types::ToolImageAttachment {
            mime_type: shot.mime_type.clone(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(&shot.bytes),
        };
        ToolResult::ok_with_images(data, summary, vec![attach])
    } else {
        ToolResult::ok(data, summary)
    }
}
