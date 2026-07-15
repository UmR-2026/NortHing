//! Target resolution for `click_target` / `move_to_target` / `click_element` /
//! `move_to_text` actions, plus the OCR disambiguation helpers used when
//! `move_to_text` finds multiple matches.
//!
//! All action handlers here take a `&dyn ComputerUseHost` rather than `&self`
//! because `ComputerUseTool` has no fields — the routing `call_impl` in
//! `mod.rs` already owns the host acquisition.
//!
//! Cross-sibling free functions (`parse_locate_query`, `parse_ocr_region_native`,
//! `req_i32`, `computer_use_augment_result_json`) are pulled in via
//! `super::validation::*` / `super::metadata::*` imports.

use super::super::computer_use_input::ensure_pointer_move_uses_screen_coordinates_only;
use super::metadata::computer_use_augment_result_json;
use super::validation::{ensure_global_xy_on_display, parse_locate_query, parse_ocr_region_native, req_i32};
use crate::agentic::tools::computer_use_host::{ComputerUseHost, OcrRegionNative, UiElementLocateQuery};
use crate::agentic::tools::framework::{ToolResult, ToolUseContext};
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::types::ToolImageAttachment;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde_json::{json, Value};

use super::ComputerUseTool;

/// Max OCR hits to attach as preview crops + AX (multimodal disambiguation).
const MOVE_TO_TEXT_DISAMBIGUATION_MAX: usize = 8;
/// Half-size in native screen pixels for each candidate preview (~400×400 logical crop).
const MOVE_TO_TEXT_PREVIEW_HALF_NATIVE: u32 = 200;

#[derive(Debug, Clone)]
struct ResolvedDesktopTarget {
    source: String,
    x: f64,
    y: f64,
    matched_text: Option<String>,
    matched_role: Option<String>,
    matched_identifier: Option<String>,
    total_matches: Option<u32>,
    selected_match_index: Option<u32>,
    warning: Option<String>,
    ax_error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ScreenOcrTextMatch {
    text: String,
    confidence: f32,
    center_x: f64,
    center_y: f64,
    bounds_left: f64,
    bounds_top: f64,
    bounds_width: f64,
    bounds_height: f64,
}

impl ComputerUseTool {
    /// OCR `find_text_matches` adapter that flattens the host's OCR match type
    /// into the local [`ScreenOcrTextMatch`] shape used by all `move_to_text`
    /// disambig helpers.
    pub(crate) async fn find_text_on_screen_impl(
        host_ref: &dyn ComputerUseHost,
        text_query: &str,
        region_native: Option<OcrRegionNative>,
    ) -> NortHingResult<Vec<ScreenOcrTextMatch>> {
        let matches = host_ref.ocr_find_text_matches(text_query, region_native).await?;
        Ok(matches
            .into_iter()
            .map(|m| ScreenOcrTextMatch {
                text: m.text,
                confidence: m.confidence,
                center_x: m.center_x,
                center_y: m.center_y,
                bounds_left: m.bounds_left,
                bounds_top: m.bounds_top,
                bounds_width: m.bounds_width,
                bounds_height: m.bounds_height,
            })
            .collect())
    }

    fn locate_query_has_any_target_impl(query: &UiElementLocateQuery) -> bool {
        query.node_idx.is_some()
            || query.text_contains.is_some()
            || query.title_contains.is_some()
            || query.role_substring.is_some()
            || query.identifier_contains.is_some()
    }

    fn target_text_query_impl<'a>(input: &'a Value, query: &'a UiElementLocateQuery) -> Option<&'a str> {
        input
            .get("target_text")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .or_else(|| {
                input
                    .get("text_query")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
            })
            .or_else(|| query.text_contains.as_deref().map(str::trim).filter(|s| !s.is_empty()))
            .or_else(|| query.title_contains.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    }

    /// Resolve `move_to_target` / `click_target` into a global screen point.
    /// Tries AX first (when `node_idx` or any AX filter is set), then OCR
    /// (when `target_text` / `text_query` is set), then explicit `x` / `y`
    /// with `use_screen_coordinates: true`.
    async fn resolve_target_point_impl(
        host_ref: &dyn ComputerUseHost,
        input: &Value,
    ) -> NortHingResult<ResolvedDesktopTarget> {
        let mut query = parse_locate_query(input);
        if query.text_contains.is_none() {
            if let Some(target_text) = input
                .get("target_text")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                query.text_contains = Some(target_text.to_string());
            }
        }

        let mut ax_error: Option<String> = None;
        if Self::locate_query_has_any_target_impl(&query) {
            match host_ref.locate_ui_element_screen_center(query.clone()).await {
                Ok(res) => {
                    return Ok(ResolvedDesktopTarget {
                        source: "ax".to_string(),
                        x: res.global_center_x,
                        y: res.global_center_y,
                        matched_text: res.matched_title.clone(),
                        matched_role: Some(res.matched_role),
                        matched_identifier: res.matched_identifier,
                        total_matches: Some(res.total_matches.max(1)),
                        selected_match_index: Some(1),
                        warning: (res.total_matches > 1).then(|| {
                            format!(
                                "{} AX elements matched; selected the host-ranked best match.",
                                res.total_matches
                            )
                        }),
                        ax_error: None,
                    });
                }
                Err(err) => {
                    ax_error = Some(err.to_string());
                }
            }
        }

        if let Some(text_query) = Self::target_text_query_impl(input, &query) {
            let ocr_region_native = parse_ocr_region_native(input)?;
            let matches = Self::find_text_on_screen_impl(host_ref, text_query, ocr_region_native).await?;
            if !matches.is_empty() {
                let requested_index = input
                    .get("move_to_text_match_index")
                    .or_else(|| input.get("target_match_index"))
                    .and_then(|v| v.as_u64())
                    .map(|u| u as usize);
                let selected = match requested_index {
                    Some(idx) if idx >= 1 && idx <= matches.len() => idx - 1,
                    Some(idx) => {
                        return Err(NortHingError::tool(format!(
                            "target_match_index/move_to_text_match_index must be between 1 and {} (got {}).",
                            matches.len(),
                            idx
                        )));
                    }
                    None => matches
                        .iter()
                        .enumerate()
                        .max_by(|(_, a), (_, b)| {
                            a.confidence
                                .partial_cmp(&b.confidence)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|(idx, _)| idx)
                        .unwrap_or(0),
                };
                let m = &matches[selected];
                return Ok(ResolvedDesktopTarget {
                    source: "ocr".to_string(),
                    x: m.center_x,
                    y: m.center_y,
                    matched_text: Some(m.text.clone()),
                    matched_role: None,
                    matched_identifier: None,
                    total_matches: Some(matches.len() as u32),
                    selected_match_index: Some((selected + 1) as u32),
                    warning: (matches.len() > 1 && requested_index.is_none()).then(|| {
                        format!(
                            "{} OCR matches found for {:?}; selected the highest-confidence match. Pass target_match_index to pin another candidate.",
                            matches.len(),
                            text_query
                        )
                    }),
                    ax_error,
                });
            }
        }

        if input.get("x").is_some() || input.get("y").is_some() {
            ensure_pointer_move_uses_screen_coordinates_only(input)?;
            let x = req_i32(input, "x")?;
            let y = req_i32(input, "y")?;
            let (sx64, sy64) = Self::resolve_xy_f64_impl(host_ref, input, x, y)?;
            if use_screen_coordinates_for_input(input) {
                ensure_global_xy_on_display(host_ref, sx64, sy64).await?;
            }
            return Ok(ResolvedDesktopTarget {
                source: "screen_xy".to_string(),
                x: sx64,
                y: sy64,
                matched_text: None,
                matched_role: None,
                matched_identifier: None,
                total_matches: None,
                selected_match_index: None,
                warning: None,
                ax_error,
            });
        }

        Err(NortHingError::tool(
            "move_to_target/click_target requires a target: node_idx, target_text/text_query/text_contains/title_contains, role_substring, identifier_contains, or x/y with use_screen_coordinates: true.".to_string(),
        ))
    }

    /// `click_target` / `move_to_target` action handler.
    pub(crate) async fn target_action_impl(
        host_ref: &dyn ComputerUseHost,
        action: &str,
        input: &Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let should_click = action == "click_target";
        let target = Self::resolve_target_point_impl(host_ref, input).await?;
        host_ref.mouse_move_global_f64(target.x, target.y).await?;
        if target.source == "ocr" {
            ComputerUseHost::computer_use_trust_pointer_after_ocr_move(host_ref);
        }

        let button = input.get("button").and_then(|v| v.as_str()).unwrap_or("left");
        let num_clicks = input
            .get("num_clicks")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .clamp(1, 3) as u32;

        if should_click {
            for _ in 0..num_clicks {
                host_ref.mouse_click_authoritative(button).await?;
            }
        }

        let target_source = target.source.clone();
        let input_coords = json!({
            "kind": action,
            "source": target_source,
            "resolved_global": { "x": target.x, "y": target.y },
            "button": if should_click { Some(button) } else { None },
            "num_clicks": if should_click { Some(num_clicks) } else { None },
        });
        let mut result_json = json!({
            "success": true,
            "action": action,
            "target_resolution_source": target.source,
            "global_center_x": target.x,
            "global_center_y": target.y,
            "matched_text": target.matched_text,
            "matched_role": target.matched_role,
            "matched_identifier": target.matched_identifier,
            "total_matches": target.total_matches,
            "selected_match_index": target.selected_match_index,
            "clicked": should_click,
            "button": if should_click { Some(button) } else { None },
            "num_clicks": if should_click { Some(num_clicks) } else { None },
        });
        if let Some(warning) = target.warning {
            result_json["warning"] = json!(warning);
        }
        if let Some(ax_error) = target.ax_error {
            result_json["ax_fallback_error"] = json!(ax_error);
        }
        let body = computer_use_augment_result_json(host_ref, result_json, Some(input_coords)).await;
        let summary = if should_click {
            format!(
                "Resolved target via {} and clicked at ({:.0}, {:.0}).",
                body.get("target_resolution_source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("target"),
                target.x,
                target.y
            )
        } else {
            format!(
                "Resolved target via {} and moved pointer to ({:.0}, {:.0}).",
                body.get("target_resolution_source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("target"),
                target.x,
                target.y
            )
        };
        Ok(vec![ToolResult::ok(body, Some(summary))])
    }

    /// `click_element` action handler (locate + move + click in one call).
    pub(crate) async fn click_element_impl(
        host_ref: &dyn ComputerUseHost,
        input: &Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let query = parse_locate_query(input);
        // Accept ANY locator that can plausibly identify a node:
        // - text_contains: wide needle over title|value|description|help
        // - node_idx: direct AX-snapshot pin (zero-ambiguity)
        // - title_contains / role_substring / identifier_contains: legacy filters
        // The previous restriction (title/role/identifier only) blocked
        // the most useful path — clicking by visible label that lives
        // in AXValue/AXDescription — and forced models into brittle
        // role guessing.
        if query.title_contains.is_none()
            && query.text_contains.is_none()
            && query.role_substring.is_none()
            && query.identifier_contains.is_none()
            && query.node_idx.is_none()
        {
            return Err(NortHingError::tool(
                "click_element requires at least one of text_contains, title_contains, role_substring, identifier_contains, or node_idx.".to_string(),
            ));
        }
        let button = input.get("button").and_then(|v| v.as_str()).unwrap_or("left");
        let num_clicks = input
            .get("num_clicks")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .clamp(1, 3) as u32;

        let res = host_ref.locate_ui_element_screen_center(query.clone()).await?;

        // Move pointer to AX center using global screen coordinates (authoritative).
        host_ref
            .mouse_move_global_f64(res.global_center_x, res.global_center_y)
            .await?;

        // Relaxed guard: AX coordinates are authoritative, no fine-screenshot needed.
        host_ref.computer_use_guard_click_allowed_relaxed()?;

        for _ in 0..num_clicks {
            host_ref.mouse_click_authoritative(button).await?;
        }

        let click_label = match num_clicks {
            2 => "double",
            3 => "triple",
            _ => "single",
        };
        let input_coords = json!({
            "kind": "click_element",
            "query": {
                "title_contains": query.title_contains,
                "role_substring": query.role_substring,
                "identifier_contains": query.identifier_contains,
                "filter_combine": query.filter_combine,
            },
            "button": button,
            "num_clicks": num_clicks,
        });
        let mut result_json = json!({
            "success": true,
            "action": "click_element",
            "matched_role": res.matched_role,
            "matched_title": res.matched_title,
            "matched_identifier": res.matched_identifier,
            "global_center_x": res.global_center_x,
            "global_center_y": res.global_center_y,
            "button": button,
            "num_clicks": num_clicks,
        });
        if let Some(ref pc) = res.parent_context {
            result_json["parent_context"] = json!(pc);
        }
        if res.total_matches > 1 {
            result_json["total_matches"] = json!(res.total_matches);
            result_json["warning"] = json!(format!(
                "{} elements matched; clicked the best-ranked one. See other_matches if wrong.",
                res.total_matches
            ));
        }
        if !res.other_matches.is_empty() {
            result_json["other_matches"] = json!(res.other_matches);
        }
        let body = computer_use_augment_result_json(host_ref, result_json, Some(input_coords)).await;
        let match_info = if res.total_matches > 1 {
            format!(" ({} matches)", res.total_matches)
        } else {
            String::new()
        };
        let summary = format!(
            "AX click_element: {} {} click on role={} at ({:.0}, {:.0}).{}",
            button, click_label, res.matched_role, res.global_center_x, res.global_center_y, match_info,
        );
        Ok(vec![ToolResult::ok(body, Some(summary))])
    }

    /// `move_to_text` action handler — OCR text match + pointer move (no click).
    /// Returns disambiguation candidates when multiple matches exist and the
    /// primary model supports image understanding.
    pub(crate) async fn move_to_text_impl(
        host_ref: &dyn ComputerUseHost,
        input: &Value,
        context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let text_query = input
            .get("text_query")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                NortHingError::tool("move_to_text requires non-empty string field `text_query`.".to_string())
            })?;
        let ocr_region_native = parse_ocr_region_native(input)?;
        let move_to_text_match_index = input
            .get("move_to_text_match_index")
            .and_then(|v| v.as_u64())
            .map(|u| u as u32);

        let matches = Self::find_text_on_screen_impl(host_ref, text_query, ocr_region_native.clone()).await?;
        if matches.is_empty() {
            return Err(NortHingError::tool(format!(
                "move_to_text found no visible OCR match for {:?}. Take a fresh screenshot and try a shorter or more distinctive substring, or use click_element.",
                text_query
            )));
        }

        let n = matches.len();
        if n > 1 && move_to_text_match_index.is_none() {
            if context.primary_model_supports_image_understanding() {
                return Self::move_to_text_disambiguation_response_impl(
                    host_ref,
                    context,
                    text_query,
                    ocr_region_native.clone(),
                    &matches,
                )
                .await;
            }
            return Self::move_to_text_disambiguation_text_only_impl(
                host_ref,
                text_query,
                ocr_region_native.clone(),
                &matches,
            )
            .await;
        }

        let sel: usize = match move_to_text_match_index {
            None => 0,
            Some(idx) => {
                if idx < 1 || idx > n as u32 {
                    return Err(NortHingError::tool(format!(
                        "move_to_text_match_index must be between 1 and {} ({} OCR matches for {:?}).",
                        n, n, text_query
                    )));
                }
                (idx - 1) as usize
            }
        };

        let matched = &matches[sel];
        host_ref
            .mouse_move_global_f64(matched.center_x, matched.center_y)
            .await?;
        ComputerUseHost::computer_use_trust_pointer_after_ocr_move(host_ref);

        let other_matches = matches
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != sel)
            .take(4)
            .map(|(_, m)| {
                json!({
                    "text": m.text,
                    "confidence": m.confidence,
                    "center_x": m.center_x,
                    "center_y": m.center_y,
                })
            })
            .collect::<Vec<_>>();

        let input_coords = json!({
            "kind": "move_to_text",
            "text_query": text_query,
            "ocr_region_native": &ocr_region_native,
            "move_to_text_match_index": move_to_text_match_index,
        });
        let body = computer_use_augment_result_json(
            host_ref,
            json!({
                "success": true,
                "action": "move_to_text",
                "move_to_text_phase": "move",
                "text_query": text_query,
                "ocr_region_native": ocr_region_native,
                "matched_text": matched.text,
                "confidence": matched.confidence,
                "global_center_x": matched.center_x,
                "global_center_y": matched.center_y,
                "bounds_left": matched.bounds_left,
                "bounds_top": matched.bounds_top,
                "bounds_width": matched.bounds_width,
                "bounds_height": matched.bounds_height,
                "total_matches": matches.len(),
                "move_to_text_match_index": move_to_text_match_index.unwrap_or(1),
                "other_matches": other_matches,
            }),
            Some(input_coords),
        )
        .await;
        let summary = format!(
            "OCR move_to_text: matched {:?} at ({:.0}, {:.0}) [index {} of {}]. Pointer is from trusted global OCR — you may **`click`** next without a separate **`screenshot`** (host clears stale-capture guard).",
            matched.text,
            matched.center_x,
            matched.center_y,
            sel + 1,
            matches.len()
        );
        Ok(vec![ToolResult::ok(body, Some(summary))])
    }

    /// Multimodal disambiguation response — attaches preview JPEGs + AX per candidate.
    async fn move_to_text_disambiguation_response_impl(
        host_ref: &dyn ComputerUseHost,
        context: &ToolUseContext,
        text_query: &str,
        ocr_region_native: Option<OcrRegionNative>,
        matches: &[ScreenOcrTextMatch],
    ) -> NortHingResult<Vec<ToolResult>> {
        Self::require_multimodal_tool_output_for_screenshot_impl(context)?;
        let take = matches.len().min(MOVE_TO_TEXT_DISAMBIGUATION_MAX);
        let mut attachments: Vec<ToolImageAttachment> = Vec::with_capacity(take);
        let mut candidates: Vec<Value> = Vec::with_capacity(take);
        for (i, m) in matches.iter().take(take).enumerate() {
            let idx_1based = i + 1;
            let ax = host_ref
                .accessibility_hit_at_global_point(m.center_x, m.center_y)
                .await?;
            let jpeg = host_ref
                .ocr_preview_crop_jpeg(m.center_x, m.center_y, MOVE_TO_TEXT_PREVIEW_HALF_NATIVE)
                .await?;
            attachments.push(ToolImageAttachment {
                mime_type: "image/jpeg".to_string(),
                data_base64: B64.encode(&jpeg),
            });
            candidates.push(json!({
                "match_index": idx_1based,
                "ocr_text": m.text,
                "confidence": m.confidence,
                "global_center_x": m.center_x,
                "global_center_y": m.center_y,
                "bounds_left": m.bounds_left,
                "bounds_top": m.bounds_top,
                "bounds_width": m.bounds_width,
                "bounds_height": m.bounds_height,
                "accessibility": ax,
                "preview_image_attachment_index": i,
            }));
        }
        let input_coords = json!({
            "kind": "move_to_text",
            "text_query": text_query,
            "ocr_region_native": ocr_region_native,
            "move_to_text_phase": "disambiguation",
        });
        let mut body = json!({
            "success": true,
            "action": "move_to_text",
            "move_to_text_phase": "disambiguation",
            "text_query": text_query,
            "ocr_region_native": ocr_region_native,
            "disambiguation_required": true,
            "instruction": "Several OCR hits for this substring. Each candidate has a **preview JPEG** (same order as `candidates`) and **accessibility** metadata at the OCR center. **Do not** derive `mouse_move` from JPEG pixels. Pick `match_index`, then call **`move_to_text` again** with the same `text_query`, same `ocr_region_native`, and **`move_to_text_match_index`** = that index. Pointer was not moved.",
            "candidates": candidates,
            "total_ocr_matches": matches.len(),
            "candidates_previewed": take,
        });
        if take < matches.len() {
            if let Some(obj) = body.as_object_mut() {
                obj.insert(
                    "truncation_note".to_string(),
                    json!(format!(
                        "Only the first {} of {} OCR matches are previewed; narrow `ocr_region_native` or `text_query` if needed.",
                        take, matches.len()
                    )),
                );
            }
        }
        let body = computer_use_augment_result_json(host_ref, body, Some(input_coords)).await;
        let hint = format!(
            "move_to_text: {} OCR matches — set move_to_text_match_index after viewing {} preview JPEGs + AX. Pointer not moved.",
            matches.len(),
            take
        );
        Ok(vec![ToolResult::ok_with_images(body, Some(hint), attachments)])
    }

    /// Same as [`Self::move_to_text_disambiguation_response_impl`] but **no image attachments** (primary model is text-only).
    async fn move_to_text_disambiguation_text_only_impl(
        host_ref: &dyn ComputerUseHost,
        text_query: &str,
        ocr_region_native: Option<OcrRegionNative>,
        matches: &[ScreenOcrTextMatch],
    ) -> NortHingResult<Vec<ToolResult>> {
        let take = matches.len().min(MOVE_TO_TEXT_DISAMBIGUATION_MAX);
        let mut candidates: Vec<Value> = Vec::with_capacity(take);
        for (i, m) in matches.iter().take(take).enumerate() {
            let idx_1based = i + 1;
            let ax = host_ref
                .accessibility_hit_at_global_point(m.center_x, m.center_y)
                .await?;
            candidates.push(json!({
                "match_index": idx_1based,
                "ocr_text": m.text,
                "confidence": m.confidence,
                "global_center_x": m.center_x,
                "global_center_y": m.center_y,
                "bounds_left": m.bounds_left,
                "bounds_top": m.bounds_top,
                "bounds_width": m.bounds_width,
                "bounds_height": m.bounds_height,
                "accessibility": ax,
            }));
        }
        let input_coords = json!({
            "kind": "move_to_text",
            "text_query": text_query,
            "ocr_region_native": ocr_region_native,
            "move_to_text_phase": "disambiguation",
        });
        let mut body = json!({
            "success": true,
            "action": "move_to_text",
            "move_to_text_phase": "disambiguation",
            "text_query": text_query,
            "ocr_region_native": ocr_region_native,
            "disambiguation_required": true,
            "instruction": "Several OCR hits for this substring. The primary model **cannot** view screenshots — pick **`move_to_text_match_index`** using **`candidates`** (global_center_* + accessibility) only. Call **`move_to_text` again** with the same `text_query`, same `ocr_region_native`, and **`move_to_text_match_index`** = that index. Pointer was not moved.",
            "candidates": candidates,
            "total_ocr_matches": matches.len(),
            "candidates_previewed": take,
        });
        if take < matches.len() {
            if let Some(obj) = body.as_object_mut() {
                obj.insert(
                    "truncation_note".to_string(),
                    json!(format!(
                        "Only the first {} of {} OCR matches are listed; narrow `ocr_region_native` or `text_query` if needed.",
                        take, matches.len()
                    )),
                );
            }
        }
        let body = computer_use_augment_result_json(host_ref, body, Some(input_coords)).await;
        let hint = format!(
            "move_to_text: {} OCR matches — set move_to_text_match_index using text candidates (no image previews). Pointer not moved.",
            matches.len(),
        );
        Ok(vec![ToolResult::ok(body, Some(hint))])
    }
}

// Tiny adapter so resolve_target_point_impl can stay readable without pulling
// the full `use_screen_coordinates` module path in.
fn use_screen_coordinates_for_input(input: &Value) -> bool {
    input
        .get("use_screen_coordinates")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}
