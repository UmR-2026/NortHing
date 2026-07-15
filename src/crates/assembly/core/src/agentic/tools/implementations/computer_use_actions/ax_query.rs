//! AX-tree query / view-build actions: `list_apps`, `get_app_state`,
//! `build_interactive_view`, `build_visual_mark_view`.
//!
//! These actions are read-only: they snapshot the AX tree (possibly with
//! numbered overlays for interactive/visual views) but never drive the
//! keyboard/mouse. Click / scroll / type actions live in `ax_click.rs`
//! and `ax_input.rs`.

use super::ax_types::{
    build_interactive_view_json, build_visual_mark_view_json, interactive_view_result, parse_selector, snap_result,
    snap_state_json, visual_mark_view_result,
};
use crate::agentic::tools::computer_use_host::{ComputerUseHostRef, InteractiveViewOpts};
use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

pub(super) async fn list_apps(
    host: &ComputerUseHostRef,
    params: &Value,
    bg: bool,
    ax: bool,
) -> NortHingResult<Vec<ToolResult>> {
    let include_hidden = params
        .get("include_hidden")
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| !params.get("only_visible").and_then(|v| v.as_bool()).unwrap_or(true));
    let apps = host.list_apps(include_hidden).await?;
    let n = apps.len();
    Ok(vec![ToolResult::ok(
        json!({
            "apps": apps,
            "include_hidden": include_hidden,
            "background_input": bg,
            "ax_tree": ax,
        }),
        Some(format!("{} app(s) listed", n)),
    )])
}

pub(super) async fn get_app_state(
    host: &ComputerUseHostRef,
    params: &Value,
    bg: bool,
    ax: bool,
) -> NortHingResult<Vec<ToolResult>> {
    let app = parse_selector(params)?;
    let max_depth = params.get("max_depth").and_then(|v| v.as_u64()).unwrap_or(32) as u32;
    let focus_window_only = params
        .get("focus_window_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let snap = host.get_app_state(app.clone(), max_depth, focus_window_only).await?;
    let summary = format!(
        "AX state for {} (digest={}, {} nodes)",
        snap.app.name,
        &snap.digest[..snap.digest.len().min(12)],
        snap.nodes.len()
    );
    let data = json!({
        "target_app": app,
        "background_input": bg,
        "ax_tree": ax,
        "app_state": snap_state_json(&snap),
        "app_state_nodes": snap.nodes,
        "before_digest": snap.digest,
        "loop_warning": snap.loop_warning,
    });
    Ok(vec![snap_result(data, Some(summary), &snap)])
}

pub(super) async fn build_interactive_view(
    host: &ComputerUseHostRef,
    params: &Value,
) -> NortHingResult<Vec<ToolResult>> {
    let app = parse_selector(params)?;
    let opts: InteractiveViewOpts = match params.get("opts") {
        Some(v) if !v.is_null() => serde_json::from_value(v.clone()).map_err(|e| {
            NortHingError::tool(format!("[INVALID_PARAMS] build_interactive_view 'opts' invalid: {}", e))
        })?,
        _ => InteractiveViewOpts::default(),
    };
    let view = host.build_interactive_view(app.clone(), opts).await?;
    let view_json = build_interactive_view_json(&view);
    let summary = format!(
        "interactive view for {} ({} elements, digest={})",
        view.app.name,
        view.elements.len(),
        &view.digest[..view.digest.len().min(12)]
    );
    Ok(vec![interactive_view_result(view_json, Some(summary), &view)])
}

pub(super) async fn build_visual_mark_view(
    host: &ComputerUseHostRef,
    params: &Value,
) -> NortHingResult<Vec<ToolResult>> {
    let app = parse_selector(params)?;
    let opts: crate::agentic::tools::computer_use_host::VisualMarkViewOpts = match params.get("opts") {
        Some(v) if !v.is_null() => serde_json::from_value(v.clone()).map_err(|e| {
            NortHingError::tool(format!("[INVALID_PARAMS] build_visual_mark_view 'opts' invalid: {}", e))
        })?,
        _ => crate::agentic::tools::computer_use_host::VisualMarkViewOpts::default(),
    };
    let view = host.build_visual_mark_view(app.clone(), opts).await?;
    let view_json = build_visual_mark_view_json(&view);
    let summary = format!(
        "visual mark view for {} ({} marks, digest={})",
        view.app.name,
        view.marks.len(),
        &view.digest[..view.digest.len().min(12)]
    );
    Ok(vec![visual_mark_view_result(view_json, Some(summary), &view)])
}
