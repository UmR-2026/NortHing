use serde::{Deserialize, Serialize};

use super::ch_platform::ComputerScreenshot;
use super::ch_types::{
    default_annotate_true, default_click_count_one, default_focus_window_only_true, default_include_tree_text_true,
    default_left_button, default_true, is_false, AppInfo, AxNode, ClickIndexTarget,
};

// =====================================================================
// Interactive-View (Set-of-Mark) data types — TuriX-CUA inspired.
// =====================================================================

/// Options for [`ComputerUseHost::build_interactive_view`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InteractiveViewOpts {
    /// When `true` (default) only emit elements inside the focused window
    /// of the target application; when `false` emit every interactive
    /// element across all windows of the app (heavier overlay).
    #[serde(default = "super::ch_types::default_focus_window_only_true")]
    pub focus_window_only: bool,
    /// Maximum number of interactive elements to include / annotate. The
    /// host trims by visual area (largest first) when exceeded so the
    /// overlay stays legible. `None` → host default (typically ~80).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_elements: Option<u32>,
    /// When `true` (default), the host paints numbered coloured boxes on a
    /// fresh focused-window screenshot. Set `false` to skip the overlay
    /// (text-only payload — cheaper, useful for retries / loop probes).
    #[serde(default = "super::ch_types::default_annotate_true")]
    pub annotate_screenshot: bool,
    /// When `true` (default), include the compact `tree_text` rendering of
    /// the filtered elements alongside the structured `elements` array.
    #[serde(default = "super::ch_types::default_include_tree_text_true")]
    pub include_tree_text: bool,
}

impl Default for InteractiveViewOpts {
    fn default() -> Self {
        Self {
            focus_window_only: true,
            max_elements: None,
            annotate_screenshot: true,
            include_tree_text: true,
        }
    }
}

/// One interactive element inside an [`InteractiveView`]. The [`Self::i`]
/// field is the only handle the model is expected to use — every other
/// field is informational so the model can disambiguate between visually
/// similar boxes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InteractiveElement {
    /// Dense per-view index (0-based). The single source of truth the
    /// model passes back via [`ClickIndexTarget::Index`] /
    /// [`InteractiveClickParams::i`].
    pub i: u32,
    /// Underlying [`AxNode::idx`] in the snapshot embedded in this view.
    /// Hosts use this to round-trip back to existing `app_click` /
    /// `app_type_text` plumbing.
    pub node_idx: u32,
    /// Native AX role (`AXButton`, `AXTextField`, …). The overlay colour
    /// is derived from this.
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subrole: Option<String>,
    /// Best human-readable label for the element (title → description →
    /// help → value, whichever is non-empty first).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Frame in **JPEG image pixel** space of the overlay screenshot
    /// (`x, y, width, height`). When `annotate_screenshot=false` the host
    /// may return `None` for elements outside the captured window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_image: Option<(u32, u32, u32, u32)>,
    /// Frame in **global pointer** space (`x, y, width, height`). Useful
    /// for hosts that need a coordinate fallback when AX press fails.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_global: Option<(f64, f64, f64, f64)>,
    /// `true` when the element is focusable / actionable right now.
    #[serde(default = "super::ch_types::default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub focused: bool,
    /// Whether the host can dispatch a press via AX (vs. falling back to a
    /// pointer click).
    #[serde(default = "super::ch_types::default_true")]
    pub ax_actionable: bool,
}

/// Set-of-Mark interactive snapshot returned by
/// [`ComputerUseHost::build_interactive_view`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InteractiveView {
    /// Identity of the captured application.
    pub app: AppInfo,
    /// Title of the focused window (or `None` when the host could not
    /// resolve it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_title: Option<String>,
    /// Filtered + sorted interactive elements with dense `i` indices.
    pub elements: Vec<InteractiveElement>,
    /// Compact text rendering of `elements` (one element per line, prefixed
    /// with `[i] role "label"`). Empty string when
    /// `opts.include_tree_text=false`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tree_text: String,
    /// Stable lowercase-hex SHA1 over the canonical element payload.
    /// Subsequent `interactive_*` calls echo this back as
    /// `before_view_digest` so the host can detect "stale index" usage.
    pub digest: String,
    /// Unix-epoch milliseconds when the view was captured.
    pub captured_at_ms: u64,
    /// Annotated focused-window screenshot (numbered coloured boxes).
    /// `None` when `opts.annotate_screenshot=false` or the capture failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<ComputerScreenshot>,
    /// Loop / no-progress warning, mirrored from
    /// [`AppStateSnapshot::loop_warning`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_warning: Option<String>,
}

/// Parameters for [`ComputerUseHost::interactive_click`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InteractiveClickParams {
    /// Required: the `i` index from the most recent interactive view.
    pub i: u32,
    /// Echo of [`InteractiveView::digest`] so the host can detect stale
    /// indices when the UI changed between view + click.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_view_digest: Option<String>,
    #[serde(default = "super::ch_types::default_click_count_one")]
    pub click_count: u8,
    /// `"left"` / `"right"` / `"middle"`.
    #[serde(default = "super::ch_types::default_left_button")]
    pub mouse_button: String,
    /// Modifier names (e.g. `["command"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifier_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_ms_after: Option<u32>,
    /// Whether the host should re-build the interactive view after the
    /// click (default `true` — the model gets a fresh annotated screenshot
    /// for the next turn). Set `false` when chaining many `interactive_*`
    /// calls in a row to save on overlay rendering.
    #[serde(default = "super::ch_types::default_true")]
    pub return_view: bool,
}

/// Parameters for [`ComputerUseHost::interactive_type_text`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InteractiveTypeTextParams {
    /// `i` index of the text field. `None` types into whatever element is
    /// currently focused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub i: Option<u32>,
    pub text: String,
    /// When `true`, host clears the field via `cmd+a` + `delete` (macOS)
    /// or equivalent before typing.
    #[serde(default, skip_serializing_if = "is_false")]
    pub clear_first: bool,
    /// When `true`, host presses `return` after typing.
    #[serde(default, skip_serializing_if = "is_false")]
    pub press_enter_after: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_view_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_ms_after: Option<u32>,
    #[serde(default = "super::ch_types::default_true")]
    pub return_view: bool,
}

/// Parameters for [`ComputerUseHost::interactive_scroll`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InteractiveScrollParams {
    /// `i` index of the scroll target. `None` scrolls at pointer / focused
    /// window centre.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub i: Option<u32>,
    /// Vertical scroll amount in lines / "wheel ticks" (positive = down).
    #[serde(default)]
    pub dy: i32,
    /// Horizontal scroll amount in lines / "wheel ticks" (positive = right).
    #[serde(default)]
    pub dx: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_view_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_ms_after: Option<u32>,
    #[serde(default = "super::ch_types::default_true")]
    pub return_view: bool,
}

/// Result envelope for `interactive_*` actions. Always carries the bare
/// AX snapshot; the rendered [`InteractiveView`] is only populated when
/// the caller asked for it via `return_view=true`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InteractiveActionResult {
    pub snapshot: AppStateSnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<InteractiveView>,
    /// Best-effort note about how the host actually executed the request
    /// (e.g. `"ax_press"`, `"pointer_click_fallback"`,
    /// `"index_resolved_via_node_idx"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_note: Option<String>,
}

/// Options for generic visual marking. This is intentionally UI-agnostic:
/// hosts should produce useful candidate points even when AX/OCR exposes
/// nothing, such as Canvas, games, maps, drawings, and icon-only controls.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisualMarkViewOpts {
    /// Max candidate points to emit. Default keeps the overlay readable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_points: Option<u32>,
    /// Optional region in screenshot image pixels to mark. When omitted,
    /// the host marks the whole app screenshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<VisualImageRegion>,
    /// Include regular grid points. Default true.
    #[serde(default = "super::ch_types::default_true")]
    pub include_grid: bool,
}

impl Default for VisualMarkViewOpts {
    fn default() -> Self {
        Self {
            max_points: None,
            region: None,
            include_grid: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisualImageRegion {
    pub x0: u32,
    pub y0: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualMark {
    pub i: u32,
    pub x: i32,
    pub y: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_image: Option<(u32, u32, u32, u32)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualMarkView {
    pub app: AppInfo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_title: Option<String>,
    pub marks: Vec<VisualMark>,
    pub digest: String,
    pub captured_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<ComputerScreenshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualClickParams {
    pub i: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_view_digest: Option<String>,
    #[serde(default = "super::ch_types::default_click_count_one")]
    pub click_count: u8,
    #[serde(default = "super::ch_types::default_left_button")]
    pub mouse_button: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifier_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_ms_after: Option<u32>,
    #[serde(default = "super::ch_types::default_true")]
    pub return_view: bool,
}

/// Result envelope for `visual_*` actions. This mirrors
/// [`InteractiveActionResult`], but carries a [`VisualMarkView`] because the
/// addressing basis is screenshot marks rather than AX elements.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualActionResult {
    pub snapshot: AppStateSnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<VisualMarkView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_note: Option<String>,
}

/// Snapshot of an application's AX tree. Returned by
/// [`ComputerUseHost::get_app_state`] and as the after-state of every
/// `app_*` mutation so the model can verify changes in one round-trip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppStateSnapshot {
    /// Identity of the captured application.
    pub app: AppInfo,
    /// Title of the focused window when `focus_window_only=true`, else
    /// the frontmost-window title (best effort).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_title: Option<String>,
    /// Codex-style human-readable text rendering of the tree (used in the
    /// model prompt). Indices in `tree_text` match `nodes[i].idx`.
    pub tree_text: String,
    /// Structured nodes, dense indexing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<AxNode>,
    /// Stable digest of the snapshot (lowercase hex SHA1 of the canonical
    /// node payload). Used as `before_app_state_digest` to detect "no-op"
    /// mutations and as a cheap equality check between successive
    /// snapshots.
    pub digest: String,
    /// Unix-epoch milliseconds when the snapshot was captured.
    pub captured_at_ms: u64,
    /// **Auto-attached** focused-window screenshot (Codex parity). The host
    /// captures the visible pixels of the target app's frontmost window
    /// every time `get_app_state` (or any `app_*` mutation) returns, so the
    /// model is never blind on canvas / WebView / WebGL surfaces that
    /// the AX tree cannot describe (e.g. the Gobang board). `None` only
    /// when the host explicitly opted out (e.g. inner `app_wait_for`
    /// polls) or the capture itself failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<ComputerScreenshot>,
    /// Optional per-snapshot warning emitted by the host when it detects
    /// the agent is targeting the same node / coordinate repeatedly without
    /// progress. The recommended remediation is encoded directly in the
    /// message and the model is expected to switch tactic (take a real
    /// `screenshot`, fall back to keyboard, re-locate via OCR, …) on the
    /// **very next** turn rather than retry the failing target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_warning: Option<String>,
}
