//! AX-first input / wait actions: type-text, key-chord, scroll, wait-for
//! (native `app_*` and `interactive_*`).
//!
//! `app_*` actions drive the keyboard/wheel on a focused app selector;
//! `interactive_*` actions address the same input primitives by an
//! `i` element index from a previously built interactive view.
//! `app_wait_for` is the only purely-side-effecting read action here —
//! it polls the host's AX tree until a predicate (digest change,
//! title contains, role/node enabled) fires.

use super::ax_types::{
    build_interactive_action_json, interactive_action_result, loop_tracker_observe, parse_click_target, parse_keys,
    parse_selector, parse_wait_predicate, snap_result, snap_state_json,
};
use crate::agentic::tools::computer_use_host::{
    ComputerUseHostRef, InteractiveScrollParams, InteractiveTypeTextParams,
};
use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

pub(super) async fn app_type_text(
    host: &ComputerUseHostRef,
    params: &Value,
    bg: bool,
) -> NortHingResult<Vec<ToolResult>> {
    let app = parse_selector(params)?;
    let text = params
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| NortHingError::tool("[INVALID_PARAMS] app_type_text requires 'text'".to_string()))?
        .to_string();
    let focus: Option<crate::agentic::tools::computer_use_host::ClickTarget> = match params.get("focus") {
        Some(v) if !v.is_null() => Some(parse_click_target(v)?),
        _ => None,
    };
    let before = host.get_app_state(app.clone(), 8, false).await.ok().map(|s| s.digest);
    let mut after = host.app_type_text(app.clone(), &text, focus.clone()).await?;
    if after.loop_warning.is_none() {
        let target_sig = format!(
            "focus={};len={}",
            serde_json::to_string(&focus).unwrap_or_default(),
            text.chars().count()
        );
        after.loop_warning = loop_tracker_observe(
            app.pid,
            "app_type_text",
            &target_sig,
            before.as_deref().unwrap_or(""),
            &after.digest,
        );
    }
    let data = json!({
        "target_app": app,
        "background_input": bg,
        "char_count": text.chars().count(),
        "focus": focus,
        "before_digest": before,
        "app_state": snap_state_json(&after),
        "app_state_nodes": after.nodes,
        "loop_warning": after.loop_warning,
    });
    Ok(vec![snap_result(
        data,
        Some(format!("typed {} chars", text.chars().count())),
        &after,
    )])
}

pub(super) async fn app_scroll(host: &ComputerUseHostRef, params: &Value, bg: bool) -> NortHingResult<Vec<ToolResult>> {
    let app = parse_selector(params)?;
    let dx = params.get("dx").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let dy = params.get("dy").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let focus: Option<crate::agentic::tools::computer_use_host::ClickTarget> = match params.get("focus") {
        Some(v) if !v.is_null() => Some(parse_click_target(v)?),
        _ => None,
    };
    let after = host.app_scroll(app.clone(), focus.clone(), dx, dy).await?;
    let data = json!({
        "target_app": app,
        "background_input": bg,
        "dx": dx,
        "dy": dy,
        "focus": focus,
        "app_state": snap_state_json(&after),
        "app_state_nodes": after.nodes,
        "loop_warning": after.loop_warning,
    });
    Ok(vec![snap_result(
        data,
        Some(format!("scrolled ({},{})", dx, dy)),
        &after,
    )])
}

pub(super) async fn app_key_chord(
    host: &ComputerUseHostRef,
    params: &Value,
    bg: bool,
) -> NortHingResult<Vec<ToolResult>> {
    let app = parse_selector(params)?;
    let keys = parse_keys(params);
    if keys.is_empty() {
        return Err(NortHingError::tool(
            "[INVALID_PARAMS] app_key_chord requires non-empty 'keys'".to_string(),
        ));
    }
    let focus_idx: Option<u32> = params.get("focus_idx").and_then(|v| v.as_u64()).map(|n| n as u32);
    let after = host.app_key_chord(app.clone(), keys.clone(), focus_idx).await?;
    let data = json!({
        "target_app": app,
        "background_input": bg,
        "keys": keys,
        "focus_idx": focus_idx,
        "app_state": snap_state_json(&after),
        "app_state_nodes": after.nodes,
        "loop_warning": after.loop_warning,
    });
    Ok(vec![snap_result(data, Some("key chord sent".to_string()), &after)])
}

pub(super) async fn app_wait_for(
    host: &ComputerUseHostRef,
    params: &Value,
    bg: bool,
) -> NortHingResult<Vec<ToolResult>> {
    let app = parse_selector(params)?;
    let predicate_v = params
        .get("predicate")
        .cloned()
        .ok_or_else(|| NortHingError::tool("[INVALID_PARAMS] app_wait_for requires 'predicate'".to_string()))?;
    let predicate = parse_wait_predicate(&predicate_v)?;
    let timeout_ms = params.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(8000) as u32;
    let poll_ms = params.get("poll_ms").and_then(|v| v.as_u64()).unwrap_or(150) as u32;
    let after = host
        .app_wait_for(app.clone(), predicate.clone(), timeout_ms, poll_ms)
        .await?;
    let data = json!({
        "target_app": app,
        "background_input": bg,
        "predicate": predicate,
        "app_state": snap_state_json(&after),
        "app_state_nodes": after.nodes,
        "loop_warning": after.loop_warning,
    });
    Ok(vec![snap_result(data, Some("predicate satisfied".to_string()), &after)])
}

pub(super) async fn interactive_type_text(
    host: &ComputerUseHostRef,
    params: &Value,
) -> NortHingResult<Vec<ToolResult>> {
    let app = parse_selector(params)?;
    let p: InteractiveTypeTextParams = serde_json::from_value(params.clone())
        .map_err(|e| NortHingError::tool(format!("[INVALID_PARAMS] interactive_type_text params invalid: {}", e)))?;
    let i = p.i;
    let text_len = p.text.chars().count();
    let res = host.interactive_type_text(app.clone(), p).await?;
    let data = build_interactive_action_json(
        &app,
        &res,
        json!({
            "i": i,
            "action": "interactive_type_text",
            "text_chars": text_len,
        }),
    );
    let summary = match i {
        Some(idx) => format!("interactive_type_text i={} ({} chars)", idx, text_len),
        None => format!("interactive_type_text focused ({} chars)", text_len),
    };
    Ok(vec![interactive_action_result(data, Some(summary), &res)])
}

pub(super) async fn interactive_scroll(host: &ComputerUseHostRef, params: &Value) -> NortHingResult<Vec<ToolResult>> {
    let app = parse_selector(params)?;
    let p: InteractiveScrollParams = serde_json::from_value(params.clone())
        .map_err(|e| NortHingError::tool(format!("[INVALID_PARAMS] interactive_scroll params invalid: {}", e)))?;
    let (i, dx, dy) = (p.i, p.dx, p.dy);
    let res = host.interactive_scroll(app.clone(), p).await?;
    let data = build_interactive_action_json(
        &app,
        &res,
        json!({
            "i": i,
            "dx": dx,
            "dy": dy,
            "action": "interactive_scroll",
        }),
    );
    let summary = format!("interactive_scroll i={:?} dx={} dy={}", i, dx, dy);
    Ok(vec![interactive_action_result(data, Some(summary), &res)])
}
