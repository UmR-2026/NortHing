//! ComputerUse coordinate / schema validation and the small free-function helpers
//! (`parse_locate_query`, `parse_ocr_region_native`, `req_i32`,
//! `computer_use_snapshot_coordinate_basis`, `ensure_global_xy_on_display`).
//!
//! Sibling free functions cross the module boundary via `super::validation::*`
//! imports; inherent methods on `ComputerUseTool` are reachable across siblings
//! without an explicit import (Rust resolves inherent impls across the crate).

use super::super::computer_use_input::{coordinate_mode, use_screen_coordinates};
use crate::agentic::tools::computer_use_host::{
    ComputerUseHost, ComputerUseScreenshotRefinement, OcrRegionNative, UiElementLocateQuery,
};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

use super::ComputerUseTool;

impl ComputerUseTool {
    /// Resolve input (x, y) into host-pointer-space (fx, fy).
    /// - `use_screen_coordinates`: true → pass through (already in globals)
    /// - `coordinate_mode`: "normalized" → delegate to `map_normalized_coords_to_pointer_f64`
    /// - otherwise → delegate to `map_image_coords_to_pointer_f64`
    pub(crate) fn resolve_xy_f64_impl(
        host: &dyn ComputerUseHost,
        input: &Value,
        x: i32,
        y: i32,
    ) -> NortHingResult<(f64, f64)> {
        if use_screen_coordinates(input) {
            return Ok((x as f64, y as f64));
        }
        if coordinate_mode(input) == "normalized" {
            host.map_normalized_coords_to_pointer_f64(x, y)
        } else {
            host.map_image_coords_to_pointer_f64(x, y)
        }
    }

    /// `click` must not carry coordinate fields — use `mouse_move` (or `move_to_text`, etc.) separately.
    pub(crate) fn ensure_click_has_no_coordinate_fields_impl(input: &Value) -> NortHingResult<()> {
        if input.get("x").is_some() || input.get("y").is_some() {
            return Err(NortHingError::tool(
                "click does not accept x or y. Position with move_to_text, click_element, or `mouse_move` with use_screen_coordinates: true (globals from tool results), then `click` with only button and num_clicks.".to_string(),
            ));
        }
        if input.get("coordinate_mode").is_some() {
            return Err(NortHingError::tool(
                "click does not accept coordinate_mode. Use `mouse_move` with use_screen_coordinates: true, then `click`.".to_string(),
            ));
        }
        if input.get("use_screen_coordinates").is_some() {
            return Err(NortHingError::tool(
                "click does not accept use_screen_coordinates. Use `mouse_move` with use_screen_coordinates, then `click`.".to_string(),
            ));
        }
        Ok(())
    }
}

/// Helper: build `UiElementLocateQuery` from tool input JSON.
pub(crate) fn parse_locate_query(input: &Value) -> UiElementLocateQuery {
    UiElementLocateQuery {
        title_contains: input
            .get("title_contains")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        role_substring: input
            .get("role_substring")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        identifier_contains: input
            .get("identifier_contains")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        max_depth: input.get("max_depth").and_then(|v| v.as_u64()).map(|v| v as u32),
        filter_combine: input
            .get("filter_combine")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        text_contains: input
            .get("text_contains")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        node_idx: input.get("node_idx").and_then(|v| v.as_u64()).map(|v| v as u32),
        app_state_digest: input
            .get("app_state_digest")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    }
}

/// Parse `ocr_region_native` (or alias `ocr_region`) into [`OcrRegionNative`].
pub(crate) fn parse_ocr_region_native(input: &Value) -> NortHingResult<Option<OcrRegionNative>> {
    let v = input.get("ocr_region_native").or_else(|| input.get("ocr_region"));
    let Some(val) = v else {
        return Ok(None);
    };
    if val.is_null() {
        return Ok(None);
    }
    let o = val.as_object().ok_or_else(|| {
        NortHingError::tool(
            "ocr_region_native must be an object { x0, y0, width, height } in global native pixels.".to_string(),
        )
    })?;
    let x0 = o
        .get("x0")
        .and_then(|x| x.as_i64())
        .ok_or_else(|| NortHingError::tool("ocr_region_native.x0 (integer) is required.".to_string()))?
        as i32;
    let y0 = o
        .get("y0")
        .and_then(|x| x.as_i64())
        .ok_or_else(|| NortHingError::tool("ocr_region_native.y0 (integer) is required.".to_string()))?
        as i32;
    let width = o
        .get("width")
        .and_then(|x| x.as_u64())
        .ok_or_else(|| NortHingError::tool("ocr_region_native.width (positive integer) is required.".to_string()))?
        as u32;
    let height =
        o.get("height").and_then(|x| x.as_u64()).ok_or_else(|| {
            NortHingError::tool("ocr_region_native.height (positive integer) is required.".to_string())
        })? as u32;
    if width == 0 || height == 0 {
        return Err(NortHingError::tool(
            "ocr_region_native width and height must be greater than zero.".to_string(),
        ));
    }
    Ok(Some(OcrRegionNative { x0, y0, width, height }))
}

pub(crate) fn req_i32(input: &Value, key: &str) -> NortHingResult<i32> {
    input
        .get(key)
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .ok_or_else(|| NortHingError::tool(format!("{} is required (integer)", key)))
}

/// JSON for `snapshot_coordinate_basis` in mouse tool results (last screenshot refinement).
pub(crate) fn computer_use_snapshot_coordinate_basis(host_ref: &dyn ComputerUseHost) -> Value {
    let last_ref = host_ref.last_screenshot_refinement();
    match last_ref {
        None => Value::Null,
        Some(ComputerUseScreenshotRefinement::FullDisplay) => json!("full_display"),
        Some(ComputerUseScreenshotRefinement::RegionAroundPoint { center_x, center_y }) => {
            json!({
                "region_crop_center_full_display_native": { "x": center_x, "y": center_y }
            })
        }
        Some(ComputerUseScreenshotRefinement::QuadrantNavigation {
            x0,
            y0,
            width,
            height,
            click_ready,
        }) => {
            json!({
                "quadrant_native_rect": { "x0": x0, "y0": y0, "w": width, "h": height },
                "quadrant_navigation_click_ready": click_ready,
            })
        }
    }
}

/// Verify a global (gx, gy) coordinate falls within at least one display reported by
/// the host. Returns a structured `DESKTOP_COORD_OUT_OF_DISPLAY` error otherwise.
///
/// This is the guard rail that prevents models from passing image-pixel coordinates
/// (taken from a screenshot crop) straight into `mouse_move(use_screen_coordinates=true)`.
pub(crate) async fn ensure_global_xy_on_display(host: &dyn ComputerUseHost, gx: f64, gy: f64) -> NortHingResult<()> {
    let displays = host.list_displays().await.unwrap_or_default();
    if displays.is_empty() {
        // Host can't enumerate displays (non-desktop runtime) — skip the guard.
        return Ok(());
    }
    let on_any = displays.iter().any(|d| {
        let x0 = d.origin_x as f64;
        let y0 = d.origin_y as f64;
        let x1 = x0 + d.width_logical as f64;
        let y1 = y0 + d.height_logical as f64;
        gx >= x0 && gx < x1 && gy >= y0 && gy < y1
    });
    if on_any {
        return Ok(());
    }
    let bounds: Vec<String> = displays
        .iter()
        .map(|d| {
            format!(
                "display_id={} bounds=({},{})-({},{}) scale={:.2}",
                d.display_id,
                d.origin_x,
                d.origin_y,
                d.origin_x + d.width_logical as i32,
                d.origin_y + d.height_logical as i32,
                d.scale_factor
            )
        })
        .collect();
    Err(NortHingError::tool(format!(
        "[DESKTOP_COORD_OUT_OF_DISPLAY] global=({:.1},{:.1}) does not lie on any visible display. \
         Visible displays: [{}]. Hint: image-pixel coordinates are NOT screen coordinates. \
         Use screenshot.pointer_global, click_element/locate result.global_center_x/y, or move_to_text. \
         To convert image→global, use the screenshot's display_id + scale_factor.",
        gx,
        gy,
        bounds.join("; ")
    )))
}
