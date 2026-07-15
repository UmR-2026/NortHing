use super::ch_types::is_false;
use serde::{Deserialize, Serialize};

// =====================================================================
// Interaction state tracking types
// =====================================================================

/// Whether the latest screenshot JPEG was the full display, a point crop, or a quadrant-drill region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputerUseScreenshotRefinement {
    FullDisplay,
    RegionAroundPoint {
        center_x: u32,
        center_y: u32,
    },
    /// Partial-screen view from hierarchical quadrant navigation.
    QuadrantNavigation {
        x0: u32,
        y0: u32,
        width: u32,
        height: u32,
        click_ready: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComputerUseInteractionScreenshotKind {
    FullDisplay,
    RegionCrop,
    QuadrantDrill,
    QuadrantTerminal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComputerUseLastMutationKind {
    Screenshot,
    PointerMove,
    Click,
    Scroll,
    KeyChord,
    TypeText,
    Wait,
    Locate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ComputerUseInteractionState {
    pub click_ready: bool,
    pub enter_ready: bool,
    pub requires_fresh_screenshot_before_click: bool,
    pub requires_fresh_screenshot_before_enter: bool,
    /// When true, the last action (click, key, typing, scroll, etc.) changed the UI; take **`screenshot`**
    /// next to **confirm** the outcome (Cowork-style verify step), ideally after **`wait`** if the UI animates.
    #[serde(default, skip_serializing_if = "is_false")]
    pub recommend_screenshot_to_verify_last_action: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_screenshot_kind: Option<ComputerUseInteractionScreenshotKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_mutation: Option<ComputerUseLastMutationKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_next_action: Option<String>,
    /// Snapshot of all displays at the time of this interaction state.
    /// The model should consult this list before issuing screen-coordinate
    /// actions on multi-monitor setups so it can disambiguate targets via
    /// `desktop.focus_display` instead of relying on cursor location.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub displays: Vec<super::ch_types::ComputerUseDisplayInfo>,
    /// Currently pinned display id (set via `desktop.focus_display`).
    /// `None` means "fall back to whichever screen the mouse is on" — the
    /// legacy behavior, kept for compatibility but discouraged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_display_id: Option<u32>,
}

pub type ComputerUseHostRef = std::sync::Arc<dyn crate::agentic::tools::computer_use_host::ComputerUseHost>;
