//! AX-first click actions: `app_click`, `interactive_click`, `visual_click`.
//!
//! All three actions ultimately drive a mouse click. They differ in how the
//! target is selected (by `node_idx`/image coords versus by element `i`
//! index on a previously built view) but the loop-warning + post-snapshot
//! envelope pattern is shared.

use super::ax_types::{
    build_interactive_action_json, build_visual_action_json, interactive_action_result, loop_tracker_observe,
    parse_click_target, parse_selector, snap_result, snap_state_json, visual_action_result,
};
use crate::agentic::tools::computer_use_host::{
    AppClickParams, ComputerUseHostRef, InteractiveClickParams, VisualClickParams,
};
use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

pub(super) async fn app_click(host: &ComputerUseHostRef, params: &Value, bg: bool) -> NortHingResult<Vec<ToolResult>> {
    let app = parse_selector(params)?;
    let target_v = params.get("target").cloned().ok_or_else(|| {
        NortHingError::tool(
            "[INVALID_PARAMS] app_click requires 'target' ({node_idx|image_xy|screen_xy|ocr_text})".to_string(),
        )
    })?;
    let target = parse_click_target(&target_v)?;
    let click_count = params.get("click_count").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
    let mouse_button = params
        .get("mouse_button")
        .and_then(|v| v.as_str())
        .unwrap_or("left")
        .to_string();
    let modifier_keys: Vec<String> = params
        .get("modifier_keys")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    let wait_ms_after = params
        .get("wait_ms_after")
        .or_else(|| params.get("post_click_wait_ms"))
        .and_then(|v| v.as_u64())
        .map(|v| v.min(5_000) as u32);

    let before = host.get_app_state(app.clone(), 8, false).await.ok().map(|s| s.digest);

    let mut after = host
        .app_click(AppClickParams {
            app: app.clone(),
            target: target.clone(),
            click_count,
            mouse_button,
            modifier_keys,
            wait_ms_after,
        })
        .await?;

    if after.loop_warning.is_none() {
        let target_sig = serde_json::to_string(&target).unwrap_or_default();
        after.loop_warning = loop_tracker_observe(
            app.pid,
            "app_click",
            &target_sig,
            before.as_deref().unwrap_or(""),
            &after.digest,
        );
    }

    let data = json!({
        "target_app": app,
        "click_target": target,
        "background_input": bg,
        "before_digest": before,
        "app_state": snap_state_json(&after),
        "app_state_nodes": after.nodes,
        "loop_warning": after.loop_warning,
    });
    Ok(vec![snap_result(data, Some("clicked".to_string()), &after)])
}

pub(super) async fn interactive_click(host: &ComputerUseHostRef, params: &Value) -> NortHingResult<Vec<ToolResult>> {
    let app = parse_selector(params)?;
    let p: InteractiveClickParams = serde_json::from_value(params.clone())
        .map_err(|e| NortHingError::tool(format!("[INVALID_PARAMS] interactive_click params invalid: {}", e)))?;
    let i = p.i;
    let res = host.interactive_click(app.clone(), p).await?;
    let data = build_interactive_action_json(&app, &res, json!({ "i": i, "action": "interactive_click" }));
    let summary = format!("interactive_click i={}", i);
    Ok(vec![interactive_action_result(data, Some(summary), &res)])
}

pub(super) async fn visual_click(host: &ComputerUseHostRef, params: &Value) -> NortHingResult<Vec<ToolResult>> {
    let app = parse_selector(params)?;
    let p: VisualClickParams = serde_json::from_value(params.clone())
        .map_err(|e| NortHingError::tool(format!("[INVALID_PARAMS] visual_click params invalid: {}", e)))?;
    let i = p.i;
    let res = host.visual_click(app.clone(), p).await?;
    let data = build_visual_action_json(&app, &res, json!({ "i": i, "action": "visual_click" }));
    let summary = format!("visual_click i={}", i);
    Ok(vec![visual_action_result(data, Some(summary), &res)])
}
