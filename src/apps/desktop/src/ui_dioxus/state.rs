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
/// by routing the toggle through a shared `Arc<Mutex<bool>>` so
/// flipping in any window updates all three.
#[derive(Debug, Clone)]
pub struct GlobalTheme {
    inner: Arc<tokio::sync::Mutex<bool>>,
}

impl Default for GlobalTheme {
    fn default() -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(true)), // default dark
        }
    }
}

impl GlobalTheme {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn is_dark(&self) -> bool {
        *self.inner.lock().await
    }

    pub async fn set_dark(&self, dark: bool) {
        *self.inner.lock().await = dark;
    }

    pub async fn toggle(&self) -> bool {
        let mut g = self.inner.lock().await;
        *g = !*g;
        *g
    }

    /// Spawn a background task that pushes every theme change into the
    /// Dioxus Signal used by the window-local UI. Each window registers
    /// one of these so all three render the same theme.
    pub fn spawn_watcher<F>(&self, mut apply: F)
    where
        F: FnMut(bool) + Send + 'static,
    {
        let inner = self.inner.clone();
        tokio::spawn(async move {
            let mut last = *inner.lock().await;
            apply(last);
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                let now = *inner.lock().await;
                if now != last {
                    last = now;
                    apply(last);
                }
            }
        });
    }
}

/// Visibility of the inner / outer windows. When `false` (the default
/// per block-contract §3.2 - three windows are visible by default), the
/// room's jewel triggers toggle this flag and the corresponding window
/// is hidden via `window.set_visible(false)`.
#[derive(Debug, Clone, Copy, Default)]
pub struct VisibilityState {
    pub inner_visible: bool,
    pub outer_visible: bool,
}

impl VisibilityState {
    pub const fn new() -> Self {
        Self {
            inner_visible: true,
            outer_visible: true,
        }
    }

    pub fn toggle_inner(&mut self) {
        self.inner_visible = !self.inner_visible;
    }

    pub fn toggle_outer(&mut self) {
        self.outer_visible = !self.outer_visible;
    }
}