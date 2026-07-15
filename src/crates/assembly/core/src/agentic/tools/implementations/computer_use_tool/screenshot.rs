//! Screenshot-related helpers and the `screenshot` action handler.
//!
//! `try_save_screenshot_for_debug` writes the exact JPEG the agent saw to
//! the workspace's `.northhing/computer_use_debug` directory so a human can
//! replay a confused screenshot session. `pack_screenshot_tool_output`
//! produces the JSON envelope + ToolImageAttachment + assistant hint that
//! the agent loop expects for any screenshot call.

use super::super::computer_use_input::parse_screenshot_params;
use super::metadata::computer_use_augment_result_json;
use super::ComputerUseTool;
use crate::agentic::tools::computer_use_host::{
    ComputerScreenshot, ComputerUseHost, ComputerUseNavigateQuadrant, ScreenshotCropCenter,
    COMPUTER_USE_POINT_CROP_HALF_MAX, COMPUTER_USE_POINT_CROP_HALF_MIN,
    COMPUTER_USE_QUADRANT_CLICK_READY_MAX_LONG_EDGE, COMPUTER_USE_QUADRANT_EDGE_EXPAND_PX,
};
use crate::agentic::tools::computer_use_optimizer::hash_screenshot_bytes;
use crate::agentic::tools::framework::{ToolResult, ToolUseContext};
use crate::util::errors::NortHingResult;
use crate::util::types::ToolImageAttachment;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde_json::{json, Value};
use tracing::{debug, warn};

/// On-disk copy of each Computer use screenshot (pointer overlay included) for debugging.
/// Filenames: `cu_<ms>_full.jpg` (whole display) or `cu_<ms>_crop_<x>_<y>.jpg` when a point crop was requested.
const COMPUTER_USE_DEBUG_SUBDIR: &str = ".northhing/computer_use_debug";

impl ComputerUseTool {
    /// Writes the exact JPEG sent to the model (including pointer overlay) under the workspace for debugging.
    pub(crate) async fn try_save_screenshot_for_debug_impl(
        bytes: &[u8],
        context: &ToolUseContext,
        crop: Option<ScreenshotCropCenter>,
        nav_label: Option<&str>,
    ) -> Option<String> {
        let root = context.workspace_root()?;
        let dir = root.join(COMPUTER_USE_DEBUG_SUBDIR);
        if let Err(e) = tokio::fs::create_dir_all(&dir).await {
            warn!("computer_use debug screenshot mkdir: {}", e);
            return None;
        }
        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let suffix = crop
            .map(|c| format!("crop_{}_{}", c.x, c.y))
            .or_else(|| nav_label.map(|s| s.to_string()))
            .unwrap_or_else(|| "full".to_string());
        let fname = format!("cu_{}_{}.jpg", ms, suffix);
        let path = dir.join(&fname);
        if let Err(e) = tokio::fs::write(&path, bytes).await {
            warn!("computer_use debug screenshot write {}: {}", path.display(), e);
            return None;
        }
        match (crop, nav_label) {
            (Some(c), _) => debug!(
                "computer_use debug: wrote point crop center=({}, {}) -> {}",
                c.x,
                c.y,
                path.display()
            ),
            (None, Some(lab)) => debug!("computer_use debug: wrote screenshot ({}) -> {}", lab, path.display()),
            (None, None) => debug!("computer_use debug: wrote full-screen screenshot -> {}", path.display()),
        }
        Some(format!("{}/{}", COMPUTER_USE_DEBUG_SUBDIR.replace('\\', "/"), fname))
    }

    /// Build tool JSON + one JPEG attachment + assistant hint from an already-captured [`ComputerScreenshot`].
    pub(crate) async fn pack_screenshot_tool_output_impl(
        shot: &ComputerScreenshot,
        debug_rel: Option<String>,
    ) -> NortHingResult<(Value, ToolImageAttachment, String)> {
        let b64 = B64.encode(&shot.bytes);
        let pointer_marker_note = match (shot.pointer_image_x, shot.pointer_image_y) {
            (Some(_), Some(_)) => "The JPEG includes a **synthetic red cursor with gray border** marking the **actual mouse position** on this bitmap (not the OS arrow). The **tip** is the true hotspot for **visual confirmation** only — **do not** use JPEG pixel indices for `mouse_move`; use `use_screen_coordinates: true` with globals from tool results (`pointer_global`, `move_to_text` global_center_*, `locate`, AX) or `move_to_text` / `click_element`.",
            _ => "No pointer overlay in this JPEG (pointer_image_x/y null): the cursor is not on this bitmap (e.g. another display). Do not infer position from the image; use global coordinates with `use_screen_coordinates: true`, or move the pointer onto this display and screenshot again.",
        };
        let mut data = json!({
            "success": true,
            "mime_type": shot.mime_type,
            "image_width": shot.image_width,
            "image_height": shot.image_height,
            "display_width_px": shot.image_width,
            "display_height_px": shot.image_height,
            "native_width": shot.native_width,
            "native_height": shot.native_height,
            "display_origin_x": shot.display_origin_x,
            "display_origin_y": shot.display_origin_y,
            "vision_scale": shot.vision_scale,
            "pointer_image_x": shot.pointer_image_x,
            "pointer_image_y": shot.pointer_image_y,
            "pointer_marker": pointer_marker_note,
            "screenshot_crop_center": shot.screenshot_crop_center,
            "point_crop_half_extent_native": shot.point_crop_half_extent_native,
            "navigation_native_rect": shot.navigation_native_rect,
            "quadrant_navigation_click_ready": shot.quadrant_navigation_click_ready,
            "image_content_rect": shot.image_content_rect,
            "image_global_bounds": shot.image_global_bounds,
            "implicit_confirmation_crop_applied": shot.implicit_confirmation_crop_applied,
            "debug_screenshot_path": debug_rel,
            "ui_tree_text": shot.ui_tree_text,
        });
        let shortcut_policy = format!(
            "**Verify step:** after **`click`**, **`key_chord`**, **`type_text`**, **`scroll`**, or **`drag`**, check **`interaction_state.recommend_screenshot_to_verify_last_action`** — when true, call **`screenshot`** next to confirm UI state (Cowork-style). \
 **Targeting priority:** `click_element` → **`move_to_text`** (OCR + move; no prior `screenshot` for targeting) → **`screenshot`** (confirm / drill) + **`mouse_move`** (**`use_screen_coordinates`: true only**) + **`click`** last. **Screenshots are for confirmation and navigation — do not guess move targets from JPEG pixels.** **`click`** never moves the pointer. **Host-only mandatory screenshot:** before **`click`** or Enter **`key_chord`** when the pointer changed since the last capture — **not** before `mouse_move`, `scroll`, `type_text`, `locate`, `wait`, or non-Enter `key_chord`. **Valid basis for a guarded `click`:** `FullDisplay`, `quadrant_navigation_click_ready`, or point crop; or bare **`screenshot`** after a pointer-changing action (**~500×500** implicit confirmation around mouse/caret). **`mouse_move`** must use **global** coordinates (from `move_to_text` global_center_*, `locate`, AX, or `pointer_global`). **Bare confirmation `screenshot`:** whenever the host still requires a capture before **`click`** or Enter **`key_chord`** (`requires_fresh_screenshot_*`), a bare `screenshot` (no crop / no reset) is **~500×500** centered on **mouse** (`screenshot_implicit_center` default `mouse`) — **including during quadrant drill** and the **first** such capture in a session. Before Enter in a text field, set **`screenshot_implicit_center`**: `text_caret`. Use **`screenshot_reset_navigation`**: true for a **full-screen** capture instead. **If AX failed:** try **`move_to_text`** before a long screenshot drill. **Optional refinement** for tiny targets: `screenshot_navigate_quadrant` until `quadrant_navigation_click_ready` (long edge < {} px) or point crop. Small moves: **ComputerUseMouseStep** over tiny **ComputerUseMousePrecise** (screen globals only).",
            COMPUTER_USE_QUADRANT_CLICK_READY_MAX_LONG_EDGE
        );
        let region_crop_size_note = shot
            .point_crop_half_extent_native
            .map(|h| {
                let edge = h.saturating_mul(2);
                format!(
                    "Crop frame (~{}×{} native, half-extent {} px; clamped {}..{}): ",
                    edge, edge, h, COMPUTER_USE_POINT_CROP_HALF_MIN, COMPUTER_USE_POINT_CROP_HALF_MAX
                )
            })
            .unwrap_or_else(|| "Crop frame (~500×500 native, half-extent 250 px): ".to_string());
        let hierarchical_navigation = if shot.screenshot_crop_center.is_some() {
            json!({
                "phase": "region_crop",
                "image_is_crop_only": true,
                "shortcut_policy": shortcut_policy,
                "instruction": format!(
                    "{}**Image pixel (0,0)** is the **top-left of this crop** in **full-capture native** space (same whole-screen bitmap as a full-screen shot — not local 0..crop only). This view is for **confirmation / drill** — do **not** use JPEG pixels for `mouse_move`. For another view, call screenshot with new `screenshot_crop_center_*` in that same full-capture space; optional `screenshot_crop_half_extent_native` adjusts crop size. See shortcut_policy.",
                    region_crop_size_note
                )
            })
        } else if shot.quadrant_navigation_click_ready {
            json!({
                "phase": "quadrant_terminal",
                "image_is_crop_only": true,
                "shortcut_policy": shortcut_policy,
                "instruction": "Region is small enough for precise pointer: **`quadrant_navigation_click_ready`** is true. **Do not** use **`ComputerUseMouseStep`** / **`pointer_move_rel`** immediately after a **`screenshot`** (host blocks — vision nudges are wrong). First **`move_to_text`**, **`mouse_move`** (`use_screen_coordinates`: true), or **`click_element`**, then optional **`ComputerUseMouseStep`** / **`ComputerUseMousePrecise`**. Then **`ComputerUseMouseClick`** (`action`: click). Host requires a **fresh** screenshot before the next **`click`** or Enter **`key_chord`** if pointer state changed since last capture (see shortcut_policy)."
            })
        } else if !Self::shot_covers_full_display_impl(shot) {
            json!({
                "phase": "quadrant_drill",
                "image_is_crop_only": true,
                "shortcut_policy": shortcut_policy,
                "instruction": format!(
                    "**Keep drilling (default):** call **`screenshot`** again with **`screenshot_navigate_quadrant`**: `top_left` | `top_right` | `bottom_left` | `bottom_right` — pick the tile that contains your target. The host expands the chosen quadrant by **{} px** on each side (clamped) so split-edge controls stay in-frame. Repeat until `quadrant_navigation_click_ready`. To restart from the full display, set **`screenshot_reset_navigation`**: true on the next screenshot. Coordinates remain **full-display native**. See shortcut_policy.",
                    COMPUTER_USE_QUADRANT_EDGE_EXPAND_PX
                )
            })
        } else {
            json!({
                "phase": "full_display",
                "image_is_crop_only": false,
                "host_auto_quadrant": false,
                "next_step_for_mouse_click": "**First:** **`move_to_text`** if visible text can name the target (OCR + move pointer; then **`click`** if you need a press). **If you must move by globals:** **`mouse_move`** with **`use_screen_coordinates`: true** and coordinates from **`locate`**, **`move_to_text`**, or **`pointer_global`** — **not** from guessing JPEG pixels. Then **`click`** when the host allows (`interaction_state.click_ready`). **Optional refinement:** `screenshot_crop_center_*`, quadrant drill, or **`screenshot_navigate_quadrant`** for smaller targets. Host never splits the screen unless you pass `screenshot_navigate_quadrant`.",
                "shortcut_policy": shortcut_policy,
                "instruction": "Full frame: JPEG aligns with **full-display native** space for **visual confirmation** only. **Prefer `move_to_text`** when readable text exists (then **`click`**). **Do not** derive `mouse_move` targets from this bitmap — use **`use_screen_coordinates`: true** with globals from tools, or AX/OCR actions. Then **`click`** when host allows (`click_ready`). For tiny targets, optionally narrow with `screenshot_crop_center_*` or quadrant drill. **`screenshot`**-heavy paths are **last** for targeting. See `next_step_for_mouse_click`, `recommended_next_for_click_targeting`, shortcut_policy."
            })
        };
        if let Some(obj) = data.as_object_mut() {
            obj.insert("hierarchical_navigation".to_string(), hierarchical_navigation);
            if shot.screenshot_crop_center.is_none() && !shot.quadrant_navigation_click_ready {
                if Self::shot_covers_full_display_impl(shot) {
                    obj.insert(
                        "recommended_next_for_click_targeting".to_string(),
                        Value::String("move_to_text_then_click_or_mouse_move_screen_globals_then_click".to_string()),
                    );
                } else {
                    let rec = format!(
                        "move_to_text_first_then_{}",
                        "screenshot_navigate_quadrant_until_click_ready"
                    );
                    obj.insert("recommended_next_for_click_targeting".to_string(), Value::String(rec));
                }
            }
        }
        let attach = ToolImageAttachment {
            mime_type: shot.mime_type.clone(),
            data_base64: b64,
        };
        let pointer_line = match (shot.pointer_image_x, shot.pointer_image_y) {
            (Some(px), Some(py)) => format!(
                " TRUE POINTER: **red cursor with gray border** (tip = hotspot) in the JPEG at image x={}, y={} — **confirmation only**; use **`mouse_move`** with **`use_screen_coordinates`: true** using globals from tool JSON (`pointer_global`, `move_to_text`, `locate`), then **`click`**. **Do not** use **`pointer_move_rel`** / **ComputerUseMouseStep** as the next action after this **`screenshot`** (host blocks). Prior screenshot is stale after **ComputerUseMousePrecise** / **ComputerUseMouseStep** / `pointer_move_rel` until you screenshot again.",
                px, py
            ),
            _ => " TRUE POINTER: not on this capture (pointer_image_x/y null). No red synthetic cursor — OS mouse may be on another display; use use_screen_coordinates with global coords or bring the pointer here and re-screenshot."
                .to_string(),
        };
        let debug_line = debug_rel
            .as_ref()
            .map(|p| {
                format!(
                    " Same JPEG saved under workspace: {} (verify red cursor tip vs pointer_image_*).",
                    p
                )
            })
            .unwrap_or_default();
        let hint = if let Some(c) = shot.screenshot_crop_center {
            format!(
                "Region crop screenshot {}x{} around full-display native center ({}, {}). **Confirm** UI state here — do **not** use JPEG pixels for `mouse_move`.{}.{} After pointer moves, screenshot again before click (host).",
                shot.image_width,
                shot.image_height,
                c.x,
                c.y,
                pointer_line,
                debug_line
            )
        } else if shot.quadrant_navigation_click_ready {
            format!(
                "Quadrant terminal {}x{} (native region {:?}). **`quadrant_navigation_click_ready`**: align with **ComputerUseMouseStep** / **`mouse_move`** (**`use_screen_coordinates`: true** only) / **ComputerUseMousePrecise**, then **`ComputerUseMouseClick`** (`action`: click) — **`click`** has no coordinates.{}.{}",
                shot.image_width,
                shot.image_height,
                shot.navigation_native_rect,
                pointer_line,
                debug_line
            )
        } else if !Self::shot_covers_full_display_impl(shot) {
            format!(
                "Quadrant drill view {}x{} (native region {:?}). Call **`screenshot`** with **`screenshot_navigate_quadrant`** to subdivide, or **`screenshot_reset_navigation`**: true for full screen.{}.{}",
                shot.image_width,
                shot.image_height,
                shot.navigation_native_rect,
                pointer_line,
                debug_line
            )
        } else {
            let nx = shot.native_width.saturating_sub(1);
            let ny = shot.native_height.saturating_sub(1);
            format!(
                "Full screenshot {}x{} (vision_scale={}). **Display native** range **0..={}** x **0..={}** (JPEG matches this rect for **confirmation**). **Targeting:** prefer **`move_to_text`** when text is visible; **`screenshot` + quad** is lowest priority. **`mouse_move`** uses **`use_screen_coordinates`: true** with globals from tools — **not** JPEG guesses; then **`click`** when allowed (see `interaction_state`). **Only** guarded **`click`** / Enter **`key_chord`** need a fresh capture after pointer moves (see shortcut_policy).{}.{}",
                shot.image_width,
                shot.image_height,
                shot.vision_scale,
                nx,
                ny,
                pointer_line,
                debug_line
            )
        };
        Ok((data, attach, hint))
    }

    /// True when the captured shot represents the full display (no crop /
    /// quadrant nav in flight).
    pub(crate) fn shot_covers_full_display_impl(shot: &ComputerScreenshot) -> bool {
        if shot.screenshot_crop_center.is_some() {
            return false;
        }
        match shot.navigation_native_rect {
            None => true,
            Some(n) => n.x0 == 0 && n.y0 == 0 && n.width == shot.native_width && n.height == shot.native_height,
        }
    }

    /// `screenshot` action handler — captures, hashes for change-detection,
    /// writes debug copy, and packages the JPEG + JSON envelope.
    pub(crate) async fn screenshot_action_impl(
        host_ref: &dyn ComputerUseHost,
        input: &Value,
        context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        Self::require_multimodal_tool_output_for_screenshot_impl(context)?;
        let (params, ignored_crop_for_quadrant) = parse_screenshot_params(input)?;
        let crop_for_debug = params.crop_center;
        let nav_debug = params.navigate_quadrant.map(|q| match q {
            ComputerUseNavigateQuadrant::TopLeft => "nav_tl",
            ComputerUseNavigateQuadrant::TopRight => "nav_tr",
            ComputerUseNavigateQuadrant::BottomLeft => "nav_bl",
            ComputerUseNavigateQuadrant::BottomRight => "nav_br",
        });
        let shot = host_ref.screenshot_display(params).await?;
        // Update screenshot hash for visual change detection
        let shot_hash = hash_screenshot_bytes(&shot.bytes);
        host_ref.update_screenshot_hash(shot_hash);
        let crop_for_debug = shot.screenshot_crop_center.or(crop_for_debug);
        let debug_rel = Self::try_save_screenshot_for_debug_impl(&shot.bytes, context, crop_for_debug, nav_debug).await;
        let input_coords = json!({
            "kind": "screenshot",
            "screenshot_reset_navigation": params.reset_navigation,
            "screenshot_crop_ignored_for_quadrant": ignored_crop_for_quadrant,
            "screenshot_crop_center": shot.screenshot_crop_center.map(|c| json!({ "x": c.x, "y": c.y })),
            "screenshot_crop_half_extent_native": shot.point_crop_half_extent_native,
            "screenshot_implicit_confirmation_crop_applied": shot.implicit_confirmation_crop_applied,
            "screenshot_navigate_quadrant": params.navigate_quadrant.map(|q| match q {
                ComputerUseNavigateQuadrant::TopLeft => "top_left",
                ComputerUseNavigateQuadrant::TopRight => "top_right",
                ComputerUseNavigateQuadrant::BottomLeft => "bottom_left",
                ComputerUseNavigateQuadrant::BottomRight => "bottom_right",
            }),
        });
        let (mut data, attach, mut hint) = Self::pack_screenshot_tool_output_impl(&shot, debug_rel).await?;
        if let Some(obj) = data.as_object_mut() {
            obj.insert("action".to_string(), Value::String("screenshot".to_string()));
            if ignored_crop_for_quadrant {
                obj.insert("screenshot_crop_center_ignored".to_string(), Value::Bool(true));
                obj.insert(
                    "screenshot_params_note".to_string(),
                    Value::String(
                        "screenshot_navigate_quadrant was set; screenshot_crop_center_x/y in this request were ignored."
                            .to_string(),
                    ),
                );
                hint = format!(
                    "{} `screenshot_crop_center_*` were ignored because `screenshot_navigate_quadrant` takes precedence.",
                    hint
                );
            }
        }
        let data = computer_use_augment_result_json(host_ref, data, Some(input_coords)).await;
        Ok(vec![ToolResult::ok_with_images(data, Some(hint), vec![attach])])
    }
}
