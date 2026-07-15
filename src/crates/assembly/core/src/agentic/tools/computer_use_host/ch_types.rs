use serde::{Deserialize, Serialize};

// =====================================================================
// Geometry / crop / navigation types
// =====================================================================

/// Center of a **point crop** in **full-display native capture pixels** (same origin as full-screen computer-use JPEG pixels).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScreenshotCropCenter {
    pub x: u32,
    pub y: u32,
}

/// Native-pixel rectangle on the **captured display bitmap** (0..`native_width`, 0..`native_height`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputerUseNavigationRect {
    pub x0: u32,
    pub y0: u32,
    pub width: u32,
    pub height: u32,
}

/// Subdivide the current navigation view into four tiles (model picks one per `screenshot` step).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputerUseNavigateQuadrant {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Center for host-applied **implicit** 500×500 confirmation crops (when a fresh screenshot is required).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputerUseImplicitScreenshotCenter {
    #[default]
    Mouse,
    /// Best-effort focused text field / insertion area (macOS AX); other platforms fall back to mouse.
    TextCaret,
}

// =====================================================================
// OCR types
// =====================================================================

/// Optional **global native** rectangle (same space as pointer / `display_origin` + capture) to limit
/// OCR to a screen region (e.g. one app window) and avoid matching text in other windows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OcrRegionNative {
    pub x0: i32,
    pub y0: i32,
    pub width: u32,
    pub height: u32,
}

/// A single OCR text match with global display coordinates.
/// Returned by [`ComputerUseHost::ocr_find_text_matches`].
#[derive(Debug, Clone)]
pub struct OcrTextMatch {
    pub text: String,
    pub confidence: f32,
    pub center_x: f64,
    pub center_y: f64,
    pub bounds_left: f64,
    pub bounds_top: f64,
    pub bounds_width: f64,
    pub bounds_height: f64,
}

/// Hit-tested accessibility node at a global screen point (OCR disambiguation).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct OcrAccessibilityHit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_context: Option<String>,
    /// One-line summary for the model (role, title, parent).
    pub description: String,
}

// =====================================================================
// Accessibility tree query / result types
// =====================================================================

/// Filter for native accessibility (macOS AX) BFS search — role/title/identifier substrings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiElementLocateQuery {
    #[serde(default)]
    pub title_contains: Option<String>,
    /// **Wide** text needle: matched against `title | value | description | help` of each AX node
    /// (case-insensitive substring). Use this when the on-screen visible text is not in `AXTitle`
    /// (e.g. a card whose label sits in `AXValue` of a child `AXStaticText`, or a button labelled
    /// only via `AXDescription`). Independent of `title_contains` — both can be supplied and
    /// `filter_combine` controls the boolean.
    #[serde(default)]
    pub text_contains: Option<String>,
    #[serde(default)]
    pub role_substring: Option<String>,
    #[serde(default)]
    pub identifier_contains: Option<String>,
    /// BFS depth from the application root (default 48, max 200).
    #[serde(default)]
    pub max_depth: Option<u32>,
    /// `"all"` (default): every non-empty filter must match the **same** element (AND).
    /// `"any"`: at least one non-empty filter matches (OR) — useful when title and role are not both present on one node (e.g. search field with empty AXTitle).
    #[serde(default)]
    pub filter_combine: Option<String>,
    /// Direct AX-node-index pin from the most recent `get_app_state` snapshot for the same
    /// application. When present the host SHORT-CIRCUITS BFS and resolves the node from its
    /// per-pid cache. Always preferred over text/role filters when an `AppStateSnapshot` is
    /// available — guarantees the exact node the model already saw, not a re-ranked guess.
    #[serde(default)]
    pub node_idx: Option<u32>,
    /// Optional digest from the same `AppStateSnapshot` that produced `node_idx`. When set the
    /// host returns `AX_IDX_STALE` if the cached snapshot has rotated. Omit for a "loose" lookup.
    #[serde(default)]
    pub app_state_digest: Option<String>,
}

/// Matched element geometry from the accessibility tree: center plus **axis-aligned bounds** (four corners).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElementLocateResult {
    /// Same space as `ComputerUse` `use_screen_coordinates` / host pointer moves.
    pub global_center_x: f64,
    pub global_center_y: f64,
    /// Use with `ComputerUse` `screenshot_crop_center_x` / `y` (full-capture native indices).
    pub native_center_x: u32,
    pub native_center_y: u32,
    /// Element frame in **global** pointer space: top-left `(left, top)`, size `(width, height)`.
    /// Four corners: `(left, top)`, `(left+width, top)`, `(left, top+height)`, `(left+width, top+height)`.
    pub global_bounds_left: f64,
    pub global_bounds_top: f64,
    pub global_bounds_width: f64,
    pub global_bounds_height: f64,
    /// Tight **native** pixel bounds on the capture bitmap (full-display indices), derived from the global frame
    /// (mapping uses the display that contains the center; large spans may be approximate).
    pub native_bounds_min_x: u32,
    pub native_bounds_min_y: u32,
    pub native_bounds_max_x: u32,
    pub native_bounds_max_y: u32,
    pub matched_role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_identifier: Option<String>,
    /// Parent element role + title for disambiguation (e.g. "AXWindow: Settings").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_context: Option<String>,
    /// Total number of elements that matched the query (before ranking).
    /// If > 1, the model should consider whether this is the right one.
    #[serde(default)]
    pub total_matches: u32,
    /// Brief descriptions of other matches (up to 4) for disambiguation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other_matches: Vec<String>,
    /// AX-tree node index of the matched element when resolvable from the most recent
    /// `get_app_state` cache (e.g. macOS). Pass back as `node_idx` for the cheapest possible
    /// follow-up `click_element` / `locate` call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_node_idx: Option<u32>,
    /// Which filter type produced the match: one of `"node_idx" | "text_contains" |
    /// "title_contains" | "role_substring" | "identifier_contains" | "climbed"`.
    /// `"climbed"` indicates a static-text leaf was promoted to its nearest clickable ancestor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_via: Option<String>,
}

// =====================================================================
// Codex-style AX data types
// =====================================================================

/// Identifies a target application for the Codex-style `app_*` actions.
/// At least one of `name` / `bundle_id` / `pid` must be set; hosts pick
/// the most specific available (pid > bundle_id > name).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSelector {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<i32>,
}

impl AppSelector {
    /// Convenience: select by name only (e.g. `"Safari"`).
    pub fn by_name(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            bundle_id: None,
            pid: None,
        }
    }

    /// Convenience: select by pid only.
    pub fn by_pid(pid: i32) -> Self {
        Self {
            name: None,
            bundle_id: None,
            pid: Some(pid),
        }
    }

    /// Convenience: select by bundle id (macOS).
    pub fn by_bundle_id(bundle_id: impl Into<String>) -> Self {
        Self {
            name: None,
            bundle_id: Some(bundle_id.into()),
            pid: None,
        }
    }

    /// True when no selector field is populated.
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.bundle_id.is_none() && self.pid.is_none()
    }
}

/// One running application, returned by [`ComputerUseHost::list_apps`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<i32>,
    /// Whether the application currently has at least one running process.
    pub running: bool,
    /// Unix-epoch milliseconds of last user activation, when the host can
    /// resolve it from LaunchServices / equivalent. Used for ordering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used_ms: Option<i64>,
    /// Cumulative launch count, when the host can resolve it.
    #[serde(default)]
    pub launch_count: u64,
}

/// One node of a Codex-style accessibility tree.
///
/// Indices are dense and stable **within a single
/// [`AppStateSnapshot`]** — they are only valid until the next
/// `get_app_state` / `app_*` call, after which the host re-dumps the tree
/// and assigns fresh indices. Callers that need to chain mutations should
/// use the snapshot returned from the previous mutation as the new
/// addressing basis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AxNode {
    /// Stable index inside this snapshot. Zero is the application root.
    pub idx: u32,
    /// Parent index, `None` for the root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_idx: Option<u32>,
    /// Native role string (e.g. macOS AX `AXButton`).
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    pub enabled: bool,
    pub focused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
    /// Frame in **global** pointer space: `(x, y, width, height)`. `None`
    /// when the AX backend cannot resolve the position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_global: Option<(f64, f64, f64, f64)>,
    /// Names of supported AX actions (e.g. `kAXPress`, `kAXShowMenu`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
    /// Localized role description (`AXRoleDescription` on macOS), e.g.
    /// "standard window", "close button", "scroll area", "HTML content",
    /// "tab group". Codex-style renderers prefer this over [`Self::role`]
    /// because it matches what a sighted user would call the element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_description: Option<String>,
    /// Native AX subrole (e.g. `AXCloseButton`, `AXFullScreenButton`,
    /// `AXMinimizeButton`, `AXSecureTextField`). Useful for button
    /// disambiguation when `role` is generic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subrole: Option<String>,
    /// `AXHelp` / tooltip text — frequently the only place an icon-only
    /// button explains itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// `AXURL` for `AXWebArea` / "HTML content" nodes (e.g. Tauri
    /// `tauri://localhost`, Electron `file://…`, Safari pages).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// `AXExpanded` for disclosure controls / collapsible sidebars.
    /// `Some(true)` = expanded, `Some(false)` = collapsed, `None` =
    /// attribute not exposed by the element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expanded: Option<bool>,
}

// =====================================================================
// Click / action targeting types
// =====================================================================

/// Where an [`ComputerUseHost::app_click`] should land. `Index`
/// is the canonical addressing mode; the other variants exist only so
/// hosts can transparently fall back to existing `app_click` paths when
/// AX press is rejected for a given element.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ClickTarget {
    /// Global screen-space coordinates (same space as `mouse_move`).
    ScreenXy { x: f64, y: f64 },
    /// Pixel coordinates in the most recent screenshot attached by
    /// `get_app_state` / `screenshot`. This is the preferred target for
    /// visual surfaces such as Canvas, SVG boards, and WebGL scenes.
    ImageXy {
        x: i32,
        y: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        screenshot_id: Option<String>,
    },
    /// Grid target inside the most recent screenshot attached by
    /// `get_app_state` / `app_click`. This is for non-text visual surfaces
    /// such as boards and canvases where a single guessed pixel is brittle.
    ///
    /// `x0/y0/width/height` describe the board/grid rectangle in screenshot
    /// image pixels. `row` and `col` are zero-based. When `intersections` is
    /// true, rows/cols are line intersections (e.g. Go/Gomoku 15x15); when
    /// false, rows/cols are cells and the click lands in the cell center.
    ImageGrid {
        x0: i32,
        y0: i32,
        width: u32,
        height: u32,
        rows: u32,
        cols: u32,
        row: u32,
        col: u32,
        #[serde(default)]
        intersections: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        screenshot_id: Option<String>,
    },
    /// Self-locating regular visual grid target. The host captures the app
    /// screenshot, detects a regular line grid, then clicks the requested
    /// row/col in the detected grid. Use when the surface is custom-drawn and
    /// the grid rectangle is not exposed by AX/OCR.
    VisualGrid {
        rows: u32,
        cols: u32,
        row: u32,
        col: u32,
        #[serde(default)]
        intersections: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wait_ms_after_detection: Option<u32>,
    },
    /// AX node addressed by index inside the most recent
    /// [`AppStateSnapshot`] for this app.
    NodeIdx { idx: u32 },
    /// OCR text needle: the host screenshots the target app, runs OCR,
    /// and clicks the centre of the highest-confidence match. Used as a
    /// fallback when the AX tree does not expose the desired element
    /// (e.g. inside a Canvas / WebGL / custom-drawn surface).
    OcrText { needle: String },
}

/// Parameters for [`ComputerUseHost::app_click`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppClickParams {
    pub app: AppSelector,
    pub target: ClickTarget,
    /// Number of clicks (1 = single, 2 = double, 3 = triple).
    #[serde(default = "AppClickParams::default_click_count")]
    pub click_count: u8,
    /// `"left"` / `"right"` / `"middle"`.
    #[serde(default = "AppClickParams::default_button")]
    pub mouse_button: String,
    /// Modifier names held during the click (e.g. `["command"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifier_keys: Vec<String>,
    /// Optional settle delay before returning the after-state screenshot.
    /// Useful for game boards, WebViews, animations, and delayed AI moves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_ms_after: Option<u32>,
}

impl AppClickParams {
    fn default_click_count() -> u8 {
        1
    }
    fn default_button() -> String {
        "left".to_string()
    }
}

/// Predicate for [`ComputerUseHost::app_wait_for`].
///
/// Hosts that don't yet implement AX waiting can simply return the
/// `app_wait_for is not available` default error; consumers fall back to
/// `wait_ms` + `get_app_state`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AppWaitPredicate {
    /// Wait until the AX tree digest changes from `prev_digest`.
    DigestChanged { prev_digest: String },
    /// Wait until any node's `title` contains the given substring.
    TitleContains { needle: String },
    /// Wait until any node has the given role and `enabled == true`.
    RoleEnabled { role: String },
    /// Wait until the node identified by `idx` reports `enabled=true`.
    NodeEnabled { idx: u32 },
}

/// One physical display reported by the desktop host. Returned by
/// [`ComputerUseHost::list_displays`] and surfaced to the model in
/// `interaction_state.displays` so it can pick the right screen explicitly
/// instead of falling back to whichever screen the mouse pointer happens
/// to be on (the original "computer use 在多屏时搞错操作的屏幕" failure mode).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComputerUseDisplayInfo {
    /// Stable per-session id of the display. Pass back to
    /// [`ComputerUseHost::focus_display`] to pin subsequent screenshots /
    /// clicks to this screen.
    pub display_id: u32,
    /// Whether the OS marks this as the primary display.
    pub is_primary: bool,
    /// Whether this is the display ControlHub will currently capture by
    /// default (matches the host's `preferred_display_id`, falling back to
    /// the screen under the mouse pointer if no preference is pinned).
    pub is_active: bool,
    /// Whether the cursor is on this display right now.
    pub has_pointer: bool,
    /// Top-left corner in **global** logical coordinate space.
    pub origin_x: i32,
    pub origin_y: i32,
    /// Logical (DIP) size; native pixels = logical × `scale_factor`.
    pub width_logical: u32,
    pub height_logical: u32,
    pub scale_factor: f32,
    /// Best-effort name of the foreground window's app on this display, if
    /// the host can determine it. Useful for the model to confirm it is
    /// targeting the "right" screen (e.g. the one with the editor).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreground_app: Option<String>,
}

/// Result of launching an application via [`ComputerUseHost::open_app`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAppResult {
    pub app_name: String,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_id: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Where an [`ComputerUseHost::interactive_click`] should land. `Index`
/// is the canonical addressing mode; the other variants exist only so
/// hosts can transparently fall back to existing `app_click` paths when
/// AX press is rejected for a given element.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ClickIndexTarget {
    /// `i` value from [`InteractiveElement::i`].
    Index { i: u32 },
    /// Authoritative AX node index (used internally when the host falls
    /// back from a stale interactive index).
    NodeIdx { idx: u32 },
}

/// Pixel rectangle of the **screen capture** in JPEG image coordinates (offset is zero when there is no frame padding).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputerUseImageContentRect {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}

/// Approximate global screen rectangle covered by the screenshot image. Values
/// are in the same coordinate space as `ClickTarget::ScreenXy`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComputerUseImageGlobalBounds {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

// =====================================================================
// Constants
// =====================================================================

/// Longest side of the navigation region must be **strictly below** this to allow `click` without a separate point crop (desktop).
pub const COMPUTER_USE_QUADRANT_CLICK_READY_MAX_LONG_EDGE: u32 = 500;

/// Native pixels added on **each** side after a quadrant choice before compositing the JPEG (avoids controls sitting exactly on the split line).
pub const COMPUTER_USE_QUADRANT_EDGE_EXPAND_PX: u32 = 50;

/// Default **half** extent (native px) for point crop around `screenshot_crop_center_*` → total region up to **500×500**.
pub const COMPUTER_USE_POINT_CROP_HALF_DEFAULT: u32 = 250;

/// Minimum **half** extent for point crop (native px) — total region **≥ 128×128** when the display is large enough.
pub const COMPUTER_USE_POINT_CROP_HALF_MIN: u32 = 64;

/// Maximum **half** extent for point crop (native px). Historically capped at
/// 250 (= 500×500) to keep the "implicit confirmation" crop tight, but that
/// crop mode has been removed. The only consumer left is the focused-window
/// crop path, which legitimately needs to cover the entire window — anywhere
/// up to the full display in either dimension. Set high enough that
/// `screenshot_display`'s own per-display clamp is the effective ceiling.
pub const COMPUTER_USE_POINT_CROP_HALF_MAX: u32 = 16384;

// =====================================================================
// Helper functions
// =====================================================================

/// Clamp optional model/host request to a valid point-crop half extent.
#[inline]
pub fn clamp_point_crop_half_extent(requested: Option<u32>) -> u32 {
    let v = requested.unwrap_or(COMPUTER_USE_POINT_CROP_HALF_DEFAULT);
    v.clamp(COMPUTER_USE_POINT_CROP_HALF_MIN, COMPUTER_USE_POINT_CROP_HALF_MAX)
}

/// Suggest a tighter half-extent from AX **native** bounds size (smaller controls → smaller JPEG).
#[inline]
pub fn suggested_point_crop_half_extent_from_native_bounds(native_w: u32, native_h: u32) -> u32 {
    let max_edge = native_w.max(native_h).max(1);
    let half = max_edge.saturating_div(2).saturating_add(32);
    clamp_point_crop_half_extent(Some(half))
}

pub fn is_false(b: &bool) -> bool {
    !*b
}

pub fn default_focus_window_only_true() -> bool {
    true
}
pub fn default_annotate_true() -> bool {
    true
}
pub fn default_include_tree_text_true() -> bool {
    true
}

pub fn default_true() -> bool {
    true
}

pub fn default_click_count_one() -> u8 {
    1
}
pub fn default_left_button() -> String {
    "left".to_string()
}
