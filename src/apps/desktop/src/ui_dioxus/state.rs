// R3' migration (2026-08-13) - shared state for the three-window shell.
//
// The room main window owns the canonical geometry (`Geometry { x, y,
// width, height }` in physical pixels) and publishes every change on a
// `tokio::sync::watch` channel. The inner / outer windows subscribe and
// reposition themselves accordingly.
//
// Signal-based UI state (theme toggle, mind-color, mock chat tokens)
// lives in `dioxus::prelude::Signal` instances per-window. Global state
// shared across windows is exposed through `use_context_provider` (0.8
// API) so changes in one window are reflected in the others - brief
// §2.2 / §4.4 ("three-window same-toggle is C2 regression point").

use std::sync::Arc;
use tokio::sync::watch;

/// Geometry of the room main window in physical pixels. Inner/outer
/// windows dock to the left/right edges of this rectangle.
#[derive(Debug, Clone, Copy)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Channel alias: the room sends `Geometry` updates, the inner/outer
/// follow tasks receive them.
pub type GeometryTx = watch::Sender<Geometry>;
pub type GeometryRx = watch::Receiver<Geometry>;
pub type GeometryRxArc = Arc<watch::Receiver<Geometry>>;

/// Theme state shared by all three windows. Per brief §1.1 the theme
/// toggle must propagate to every window (C2 regression point - the
/// Slint `RedesignTheme` global was per-instance, which broke
/// light/dark sync across inner/outer). The Dioxus shell solves this
/// by routing the toggle through a shared `tokio::sync::watch<bool>`
/// channel: the room window writes the new value synchronously and
/// every window that subscribed re-renders.
///
/// R3' r3p4 delta (2026-08-13) - Bug B root fix: the previous
/// `Arc<Mutex<bool>>` + `spawn_watcher` polling loop (50ms sleep) is
/// gone. Polling futures are what keep the main thread busy-spinning
/// (~97% CPU) under dioxus 0.8-alpha.1 (see fix brief §1); the watch
/// channel replaces both the storage and the notification mechanism:
/// `set_dark` / `toggle` are synchronous writes, subscribers react via
/// `changed().await` (event-driven, zero polling).
#[derive(Debug, Clone)]
pub struct GlobalTheme {
    tx: watch::Sender<bool>,
}

impl Default for GlobalTheme {
    fn default() -> Self {
        Self {
            tx: watch::channel(true).0, // default dark
        }
    }
}

impl GlobalTheme {
    pub fn new() -> Self {
        Self::default()
    }

    /// Current theme, synchronously (the watch channel always holds the
    /// latest value).
    ///
    /// `#[allow(dead_code)]`: kept as part of the documented synchronous
    /// API surface (fix brief §2.1). The room window currently reads the
    /// theme from its own local Signal and writes via `set_dark`, so
    /// there is no live call site; the method stays for symmetry with
    /// the other accessors and future callers.
    #[allow(dead_code)]
    pub fn is_dark(&self) -> bool {
        *self.tx.borrow()
    }

    /// Set the theme and notify every subscriber. `Err` when the channel
    /// was closed (all receivers dropped) - callers treat it as a no-op,
    /// matching the geometry channel's `let _ =` convention.
    pub fn set_dark(&self, dark: bool) {
        let _ = self.tx.send(dark);
    }

    /// Flip the theme and return the new value.
    ///
    /// `#[allow(dead_code)]`: same rationale as `is_dark` - documented
    /// synchronous API surface, no live call site in the current shell.
    #[allow(dead_code)]
    pub fn toggle(&self) -> bool {
        let next = !self.is_dark();
        self.set_dark(next);
        next
    }

    /// Subscribe to theme changes. The receiver starts at the current
    /// value; `changed()` wakes on every later `set_dark` / `toggle`.
    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.tx.subscribe()
    }
}