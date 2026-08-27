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
use dioxus::desktop::tao::event::{Event, WindowEvent};
use dioxus::desktop::tao::window::{WindowBuilder, WindowId};
use dioxus::desktop::{tao::event_loop::EventLoopWindowTarget, Config};
use std::path::PathBuf;
use std::sync::Arc;

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

/// Initial offset (in logical pixels) from the screen origin so the room
/// window opens a few inches from the top-left; keeps the three windows
/// visible side-by-side on a 1920x1080 monitor with the inner/outer
/// windows docked to the room's left/right edges.
///
/// R3' A+B+C fix (2026-08-14): must satisfy `x >= INNER_WINDOW_WIDTH +
/// DOCK_GAP_PX` (280 + 16 = 296 logical) so the inner window's initial
/// dock position (`x - 280 - 16`) never lands off-screen — the previous
/// 220.0 put the inner window at x = -76 (left edge clipped). Right-edge
/// budget on a 1920 logical workspace: 296 + 880 + 16 + 320 = 1512 ✓.
pub const ROOM_WINDOW_INITIAL_X: f32 = 296.0;
pub const ROOM_WINDOW_INITIAL_Y: f32 = 120.0;

/// Docking gap between room and its floating modules (16px gap; same
/// constant as the Slint `block_registry.rs` to keep both stacks
/// visually equivalent).
pub const DOCK_GAP_PX: i32 = 16;

/// Startup DPI scale for converting the logical launch constants into
/// the physical geometry channel (Bug A: the channel is physical — tao
/// `Moved(PhysicalPosition)` / `Resized(PhysicalSize)` events, see
/// `state.rs::Geometry` doc — but the constants are logical because the
/// windows are created with `LogicalSize`/`LogicalPosition`).
///
/// Windows: `GetDpiForSystem()` returns the system DPI (primary
/// display), which is where the room opens by default; scale = dpi/96
/// (96 = 100% DPI baseline). Non-Windows: 1.0 — the placeholder is
/// immediately superseded by the real geometry `room_app_root` publishes
/// on mount (entry.rs event handler + app.rs use_effect), so this is a
/// startup-only fallback either way.
#[cfg(target_os = "windows")]
pub fn startup_scale_factor() -> f64 {
    unsafe extern "system" {
        fn GetDpiForSystem() -> u32;
    }
    unsafe { GetDpiForSystem() as f64 / 96.0 }
}

#[cfg(not(target_os = "windows"))]
pub fn startup_scale_factor() -> f64 {
    1.0
}

/// Launch the Dioxus consult-room shell: three OS windows (room + inner +
/// outer) running concurrently. The room is the main window that owns
/// the Dioxus event loop; the two floating modules are spawned as
/// additional windows inside `room_app_root`'s `use_effect` callback
/// (which fires once the main window's Dioxus context is up).
///
/// `on_shutdown` is invoked on `Event::LoopDestroyed` (or upon exit) to
/// ensure graceful termination of worker threads and MCP child processes.
///
/// Returns `Err` if the launch setup itself fails (rare; usually a
/// WebView2 runtime initialization failure on Windows).
pub fn launch(on_shutdown: Arc<dyn Fn() + Send + Sync + 'static>) -> anyhow::Result<()> {
    // Per the spike §2 conclusion + re-spike §3.2: every window must share
    // one user-data directory so the underlying WebView2 process pool is
    // reused. Without sharing we observed ~19 msedgewebview2.exe helper
    // processes per window; sharing collapses it to ~8 across all three.
    let data_directory = shared_webview_data_directory()?;

    // Initial geometry for the room - picked once at startup. The room's
    // own positioning task overrides this from frame 1 onward (it reads
    // the actual window position which may differ if Windows snapped it).
    //
    // R3' A+B+C fix (2026-08-14): the channel is physical px, so the
    // logical launch constants are converted with the startup scale
    // factor (previous code stored the logical values verbatim, which
    // made the first-frame geometry ~25% off at 125% DPI).
    let scale = startup_scale_factor();
    let initial_geometry = Geometry {
        x: (ROOM_WINDOW_INITIAL_X as f64 * scale) as i32,
        y: (ROOM_WINDOW_INITIAL_Y as f64 * scale) as i32,
        width: (ROOM_WINDOW_WIDTH * scale) as u32,
        height: (ROOM_WINDOW_HEIGHT * scale) as u32,
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

    // r3p4 root-fix + W5-4 F6: shared channels for the tao event handler.
    // `room_window_id` uses tokio::sync::watch (single writer on mount,
    // lock-free readers in the event handler). `geometry_tx.send_modify`
    // updates geometry in place from Moved/Resized events without needing
    // an intermediate Mutex.
    let (room_window_id_tx, room_window_id_rx) = tokio::sync::watch::channel::<Option<WindowId>>(None);
    let window_manager = super::registry::ShellWindowManager::default();

    // Main window: the room itself. The launch path returns once the
    // Dioxus event loop is running; `LaunchBuilder::launch` is divergent
    // on desktop (`!`).
    let room_window = WindowBuilder::new()
        .with_title("northhing - consult room (dioxus)")
        .with_inner_size(LogicalSize::new(ROOM_WINDOW_WIDTH, ROOM_WINDOW_HEIGHT))
        .with_position(LogicalPosition::new(ROOM_WINDOW_INITIAL_X, ROOM_WINDOW_INITIAL_Y))
        // R4 W1 (2026-08-14): frameless per user ruling (handoff-20260814
        // §4, D = 方案一) — the old "Slint shell keeps decorations" matching
        // rationale is revoked. OS chrome is replaced by the self-drawn
        // room-controls (app.rs ─□✕ wired to real window ops). tao 0.16.2
        // gives 8-way border resize for free once MARKER_DECORATIONS is
        // gone (platform_impl WM_NCHITTEST → hit_test); the native drop
        // shadow is kept via `with_undecorated_shadow` so the floating
        // window still reads as a window.
        .with_decorations(false);

    #[cfg(target_os = "windows")]
    let room_window = {
        use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;
        room_window.with_undecorated_shadow(true)
    };

    let config = Config::default()
        .with_window(room_window)
        .with_data_directory(data_directory)
        // R4 W1: kill the dioxus default menu bar (Window/Edit/Help).
        // Root cause: `MenuBuilderState::Unset` resolves to
        // `Some(default_menu_bar())` (config.rs) — dioxus-desktop ships a
        // muda menu on the main window unless told otherwise. inner/outer
        // never showed it because frameless windows get it swapped out;
        // explicit None pins the intent regardless of that swap path.
        .with_menu(None)
        // r3p4 root-fix (2026-08-14): event-driven geometry publishing.
        //
        // The previous design polled the room window's position from a
        // 100ms `use_future`. Controlled CPU measurements (experiments
        // A/B/C in `task-migrate-room-report-r3p4.md`) proved that ANY
        // sleeping use_future in the room window - including a bare
        // `loop { sleep(100ms).await }` with no window()/send calls -
        // makes one background thread busy-spin at ~97% single-core
        // CPU on dioxus 0.8.0-alpha.1. The polling shape itself is the
        // poison, so geometry publishing must not use a future at all.
        //
        // Instead we hook the tao event loop directly: the room window's
        // Moved/Resized OS events become the publish trigger. Zero
        // polling, zero use_future. `room_window_id` is registered by
        // `room_app_root` on mount (it cannot be known at Config build
        // time - the OS window does not exist yet), so early window
        // events are skipped and the channel's initial_geometry covers
        // the startup window.
        .with_custom_event_handler({
            let room_window_id_rx = room_window_id_rx.clone();
            let geometry_tx = geometry_tx.clone();
            let window_manager = window_manager.clone();
            let on_shutdown = on_shutdown.clone();
            // Event-driven geometry publish: the room window's
            // Moved/Resized OS events become the publish trigger (see
            // the comment above this builder chain).
            move |event, _event_loop_target: &EventLoopWindowTarget<_>| {
                if let Event::LoopDestroyed = event {
                    crate::app_state::log::log_debug_event(
                        northhing_debug_log::COMP_UI_DIOXUS_WIN,
                        "loop_destroyed",
                        "event_loop",
                        "triggering graceful shutdown callback",
                        None,
                    );
                    on_shutdown();
                    return;
                }
                let Event::WindowEvent { window_id, event, .. } = event else {
                    return;
                };
                // Pre-mount acceptance (r3p5 A+B+C): before `room_app_root`
                // registers the room's window id (first use_effect), the
                // only window that can raise events IS the room — inner/
                // outer are spawned after the registration inside that same
                // use_effect. Accepting those early events replaces the
                // startup placeholder with the real physical geometry at
                // the very first Moved/Resized (window creation), instead
                // of carrying the logical-cast placeholder until mount.
                let is_room = {
                    let registered = *room_window_id_rx.borrow();
                    registered.is_none() || registered == Some(*window_id)
                };
                if !is_room {
                    return;
                }
                if matches!(event, WindowEvent::CloseRequested) {
                    let targets = window_manager.mark_all_closing_targets();
                    for (_id, _wid, hwnd) in targets {
                        super::app::win_ops::close_os_window(hwnd);
                    }
                    return;
                }
                match event {
                    WindowEvent::Moved(pos) => {
                        geometry_tx.send_modify(|geom| {
                            geom.x = pos.x;
                            geom.y = pos.y;
                        });
                    }
                    WindowEvent::Resized(size) => {
                        geometry_tx.send_modify(|geom| {
                            geom.width = size.width;
                            geom.height = size.height;
                        });
                    }
                    _ => {}
                }
            }
        });

    // Context injection (0.8 LaunchBuilder API). `LaunchBuilder::with_context`
    // adds a typed value to the root's context - `use_context::<T>()` in
    // the root function reads it back. The Sender is cloned-by-value
    // (it's already Clone), the Receiver is wrapped in Arc so the
    // inner/outer VirtualDoms can clone the Arc without re-subscribing
    // to the channel.
    //
    // `room_window_id_tx` is the channel the tao event handler's receiver
    // checks to filter for the room window; `room_app_root` writes it on mount.
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
        .with_context(window_manager)
        .with_context(room_window_id_tx)
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
