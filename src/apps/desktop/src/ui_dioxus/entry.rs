// R3' migration (2026-08-13) - Dioxus desktop shell entry.
//
// Spawns the room main window and the two parallel inner/outer windows.
// Follows the same multi-window pattern as the 0.7 spike
// (`task-dioxus-spike-report.md`) adapted for 0.8.0-alpha.1:
//
//   1. `dioxus::desktop::Config::default().with_window(...)` for each window.
//   2. `Config::with_data_directory(...)` on every window so they share one
//      WebView2 user-data folder (without sharing we observed ~19 helper
//      processes per window; re-spike §3.2 confirms 8 with shared dir).
//   3. The main window's positioning task publishes geometry into a
//      `tokio::sync::watch<Geometry>`; inner + outer windows subscribe via
//      `use_future` and call `w.set_outer_position(Position::Physical(...))`.
//
// Path whitelist: this file lives at `src/apps/desktop/src/ui_dioxus/`
// which is in the brief's allow-list.
//
// R3' delta vs R3 (0.7 -> 0.8 alpha):
//   * `dioxus::desktop::PhysicalPosition` -> `dioxus::desktop::tao::dpi::*`
//   * `LaunchBuilder::desktop().with_cfg(cfg).launch(App_struct)` ->
//     `LaunchBuilder::desktop().with_context(...).with_cfg(cfg).launch(fn() -> Element)`
//   * `w.outer_position().x` -> `w.outer_position()?.x` (Result wrap, 0.8 API)
//   * `WindowBuilderExtWindows` API stable (skip_taskbar extension method).

use dioxus::desktop::tao::dpi::{LogicalPosition, LogicalSize};
use dioxus::desktop::{tao::window::WindowBuilder, Config};
use std::path::PathBuf;
use std::sync::Arc;

use crate::flags::DIOXUS_SHELL;

use super::app::room_app_root;
use super::state::{Geometry, GeometryRxArc, GlobalTheme};

/// Width of the room main window. Matches the truth HTML `#room` max-width
/// (`min(780px, 100%)`) plus the chrome (`padding: 26px 48px`), so the
/// actual rendered room is ~780px and the surrounding window gives the
/// breathing-room padding the brief §3.2 calls for.
pub const ROOM_WINDOW_WIDTH: f64 = 880.0;

/// Height of the room main window. Sized to fit a 14" laptop screen with
/// the room-head (~200px), chronicle bar (~4px), chat-flow + mock session
/// (~480px), and the deck (~88px) plus padding; ~820px is the visual
/// target from the truth HTML's `padding: 26px 48px` + `room` height.
pub const ROOM_WINDOW_HEIGHT: f64 = 820.0;

/// Initial offset (in physical pixels) from the screen origin so the room
/// window opens a few inches from the top-left; keeps the three windows
/// visible side-by-side on a 1920x1080 monitor with the inner/outer
/// windows docked to the room's left/right edges.
pub const ROOM_WINDOW_INITIAL_X: f32 = 220.0;
pub const ROOM_WINDOW_INITIAL_Y: f32 = 120.0;

/// Brief §3.2 - inner/outer window widths (280px / 320px) and the
/// docking gap between room and its floating modules (16px gap; same
/// constant as the Slint `block_registry.rs` to keep both stacks
/// visually equivalent).
pub const INNER_WINDOW_WIDTH: f64 = 280.0;
pub const OUTER_WINDOW_WIDTH: f64 = 320.0;
pub const DOCK_GAP_PX: i32 = 16;

/// Launch the Dioxus consult-room shell: three OS windows (room + inner +
/// outer) running concurrently. The room is the main window that owns
/// the Dioxus event loop; the two floating modules are spawned as
/// additional windows inside `room_app_root`'s `use_effect` callback
/// (which fires once the main window's Dioxus context is up).
///
/// Must only be called when both:
///   * `flags::DIOXUS_SHELL == true`, and
///   * the `ui-dioxus` cargo feature is enabled.
///
/// Returns `Err` if the launch setup itself fails (rare; usually a
/// WebView2 runtime initialization failure on Windows). The actual
/// `LaunchBuilder::launch` is divergent on desktop (`!`), so the
/// function returns `Ok(())` only after the launch was rejected up
/// front (e.g. the `DIOXUS_SHELL == false` guard).
pub fn launch() -> anyhow::Result<()> {
    if !DIOXUS_SHELL {
        anyhow::bail!(
            "ui_dioxus::launch called with DIOXUS_SHELL = false; \
             this is an internal misconfiguration, please report"
        );
    }

    // Per the spike §2 conclusion + re-spike §3.2: every window must share
    // one user-data directory so the underlying WebView2 process pool is
    // reused. Without sharing we observed ~19 msedgewebview2.exe helper
    // processes per window; sharing collapses it to ~8 across all three.
    let data_directory = shared_webview_data_directory()?;

    // Initial geometry for the room - picked once at startup. The room's
    // own positioning task overrides this from frame 1 onward (it reads
    // the actual window position which may differ if Windows snapped it).
    let initial_geometry = Geometry {
        x: ROOM_WINDOW_INITIAL_X as i32,
        y: ROOM_WINDOW_INITIAL_Y as i32,
        width: ROOM_WINDOW_WIDTH as u32,
        height: ROOM_WINDOW_HEIGHT as u32,
    };

    // tokio::sync::watch<Geometry>: producer (room positioning task)
    // sends updates; consumers (inner + outer follow tasks) await
    // `changed()` and call `window.set_outer_position(Position::Physical(
    // ...))`. The channel always holds the latest value, so new
    // consumers can read it immediately.
    //
    // We wrap the Receiver in Arc so it can be cloned into the inner/
    // outer VirtualDoms (props must be Clone for
    // `VirtualDom::new_with_props`).
    let (geometry_tx, geometry_rx) = tokio::sync::watch::channel(initial_geometry);
    let geometry_rx_arc: GeometryRxArc = Arc::new(geometry_rx);

    // Main window: the room itself. The launch path returns once the
    // Dioxus event loop is running; `LaunchBuilder::launch` is divergent
    // on desktop (`!`).
    let room_window = WindowBuilder::new()
        .with_title("northhing - consult room (dioxus)")
        .with_inner_size(LogicalSize::new(ROOM_WINDOW_WIDTH, ROOM_WINDOW_HEIGHT))
        .with_position(LogicalPosition::new(
            ROOM_WINDOW_INITIAL_X,
            ROOM_WINDOW_INITIAL_Y,
        ))
        // Brief §3.2 / §4.1 - main window stays on the taskbar (only
        // inner/outer are skip_taskbar), decorations kept on so the
        // window-chrome is rendered by the OS. The Slint shell also keeps
        // decorations; matching the behavior avoids visible regressions.
        .with_decorations(true);

    let config = Config::default()
        .with_window(room_window)
        .with_data_directory(data_directory);

    // Context injection (0.8 LaunchBuilder API). `LaunchBuilder::with_context`
    // adds a typed value to the root's context - `use_context::<T>()` in
    // the root function reads it back. The Sender is cloned-by-value
    // (it's already Clone), the Receiver is wrapped in Arc so the
    // inner/outer VirtualDoms can clone the Arc without re-subscribing
    // to the channel.
    //
    // R3' panic fix (2026-08-13): `GlobalTheme` must be provided here too -
    // `room_app_root` reads it via `use_context::<GlobalTheme>()`, and in
    // dioxus 0.8-alpha.1 `use_context` panics ("Could not find context ...")
    // when the type is missing. The boxed-`Any` panic surfaced as
    // "Encountered panic: Any { .. }" in the room window.
    dioxus::LaunchBuilder::desktop()
        .with_context(geometry_tx)
        .with_context(geometry_rx_arc)
        .with_context(GlobalTheme::new())
        .with_cfg(config)
        .launch(room_app_root);

    // Unreachable: `launch` is `!` on desktop.
    Ok(())
}

/// Derive the shared WebView2 user-data directory. Per spike + re-spike,
/// all three windows must share one directory to collapse the helper-
/// process count from ~19/window down to ~8 total.
///
/// We use `dirs::cache_dir()` (which already handles the CJK-path-hash
/// backbone invariant from `AGENTS.md`) and append a stable per-app
/// sub-directory. Falls back to the OS temp dir if the cache dir
/// cannot be resolved (e.g. headless CI without HOME).
fn shared_webview_data_directory() -> anyhow::Result<PathBuf> {
    let base = dirs::cache_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(std::env::temp_dir);
    Ok(base.join("northhing-dioxus-dev").join("webview_data"))
}

/// Public so `app.rs` can clone the same path into the inner/outer
/// spawn closures (duplicated logic to avoid exposing the internal
/// data_directory layout).
pub fn shared_webview_data_directory_for_inner() -> PathBuf {
    let base = dirs::cache_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(std::env::temp_dir);
    base.join("northhing-dioxus-dev").join("webview_data")
}