// R3' migration (2026-08-13) - room main window component.
//
// Mirrors the truth HTML consult-room-main.html body (LL275..L459).
// Includes:
//   * chrome (LL334..L340) with theme toggle + min/max/close buttons
//   * room-status (LL342..L357) with brand-inline (5-path SVG + Fraunces
//     italic "northing") - see truth-rulings C4: brand goes in the
//     status row, NOT a left-bottom seal
//   * room-head (LL359..L364) with avatar, name, chronicle bar, state
//   * chat-flow (LL366..L416) with mock session seed + 5 record kinds
//   * room-input (LL418..L427) deck with witness note + attach +
//     placeholder + send/stop
//   * membrane (LL328..L329) + membrane-node (LL330..L331) + room-fog
//   * vertical-label (LL215..L218) hidden by default per block-contract
//     §3.1 - only appears when inner/outer are hidden
//   * containment + membrane-frame + global-aura (LL277..L279) -
//     background atmosphere
//
// Brief §4.5 - "CSS 原样内联（禁翻译成 Rust 样式）; id/data-* 锚点不改名".
// We inject the truth CSS as a <style> tag (css.rs) and keep every id
// from the truth HTML byte-for-byte. The CDP acceptance chain (brief
// §1.4) uses these selectors; renaming would break the verification.
//
// R3' delta vs R3 (0.7 -> 0.8 alpha):
//   * The root is now `fn() -> Element` (0.8 desktop API), not a struct.
//     Context (geometry_tx, geometry_rx_arc) is pulled via
//     `use_context` from the providers in entry.rs.
//   * Inner/outer window VirtualDoms are spawned inside a `use_effect`
//     that fires once on mount, since the `dioxus::desktop::window()`
//     context is only valid after the main window's launch has started.
//   * The position publisher runs in a `use_future` that polls every
//     100ms (spike §2 + brief §4.2).
//   * `html` / `doctype` elements are not exported in 0.8 alpha's
//     dioxus-html; we mount the body directly (WebView2 wraps with the
//     html/head envelope automatically).
//
// R3' r3p3 delta (2026-08-13) - Bug B root cause fix:
//   The original implementation called `LocalePack::load(...)` at the
//   top of the room window's body and ran two 100ms `use_future`
//   tasks (theme + geometry) that called `.set(...)` on Signals every
//   tick. Dioxus 0.8-alpha.1 re-renders the component on every Signal
//   `set`, regardless of whether the value actually changed. The room
//   component body therefore re-ran every ~108ms, re-loading the locale
//   pack from disk each time (15 reloads in 1.4s - observed in
//   `build-shots-tmp/runtime-launch.txt`, r3p report §2). Three fixes:
//
//     1. Mount-once `LocalePack` via `use_hook(|| Rc::new(...))`. The
//        closure runs only on the first render, so disk reads happen
//        exactly once per window regardless of how often the component
//        re-renders.
//     2. Theme `use_future` only calls `theme_dark.set(...)` when the
//        polled value actually changed (last-value cache lives in the
//        future's stack frame).
//     3. Geometry `use_future` only calls `geom_tx.send(...)` when the
//        polled Geometry actually changed (last-value cache lives in
//        the future's stack frame; `watch::send` returns Err if there
//        are no receivers, which we keep handling with `let _ =`).
//
//   inner / outer windows in `windows.rs` get only fix #1 because their
//   theme Signals are never updated (they follow the room's theme via
//   context) and they don't poll geometry.
//
// R3' r3p4 delta (2026-08-14) - Bug B root fix, event-driven theme +
// geometry (zero sleeping use_future in the room window):
//   The theme `use_future` (the room window's second sleeping future)
//   is deleted, and so is the geometry polling future. Controlled CPU
//   experiments (fix brief §1 + experiments A/B/C recorded in
//   `task-migrate-room-report-r3p4.md`) proved that ANY sleeping
//   use_future in the room window - even a bare `loop{sleep(100ms)}`
//   with no setters - makes a background thread busy-spin at ~97%
//   single-core CPU on dioxus 0.8.0-alpha.1; and that waking a
//   use_future at drag frequency (geometry updates) hangs all three
//   windows. The poison is the polling/wakeup shape itself, not the
//   content. The theme now flows purely through events: the chrome
//   toggle writes the room's local Signal synchronously and broadcasts
//   over the `GlobalTheme` watch channel (state.rs); inner/outer
//   subscribe via their props. Geometry is published from a tao
//   event-loop handler (entry.rs) and consumed by plain std::threads
//   (windows.rs) - the whole shell has zero use_future with tokio
//   sleeps and zero event-loop wakeup storms.

use dioxus::core::VirtualDom;
use dioxus::desktop::tao::dpi::{PhysicalPosition, PhysicalSize, Position};
use dioxus::desktop::{tao::window::WindowBuilder, Config};
use dioxus::desktop::window;
use dioxus::prelude::*;
use std::rc::Rc;
use tokio::sync::watch;

use super::css;
use super::entry::{
    shared_webview_data_directory_for_inner, DOCK_GAP_PX, INNER_WINDOW_WIDTH,
    OUTER_WINDOW_WIDTH, ROOM_WINDOW_HEIGHT, ROOM_WINDOW_INITIAL_X, ROOM_WINDOW_INITIAL_Y,
    ROOM_WINDOW_WIDTH,
};
use super::i18n::{keys, LocalePack};
use super::session_mock::{seed_session, MockEntry};
use super::state::{
    Geometry, GeometryRxArc, GeometryTx, GlobalTheme, GlobalVisibility, VisibilityState,
};
use super::windows::{inner_app_root, inner_app_root_props, outer_app_root, outer_app_root_props};

/// RSX root for the room main window.
///
/// Pulls `GeometryTx` (to publish position updates) and `GeometryRxArc`
/// (to clone into inner/outer VirtualDoms) from context, then runs:
///   1. `use_effect` once on mount: spawn inner + outer VirtualDoms.
///   2. `use_future` async: 100ms position publisher task.
///   3. RSX rendering of the consult-room shell.
pub fn room_app_root() -> Element {
    // Pull shared state from context. `use_context` returns `Result`
    // in 0.8; we unwrap because the provider in entry.rs guarantees
    // these are present whenever this root is invoked.
    let geometry_tx = use_context::<GeometryTx>();
    let geometry_rx_arc = use_context::<GeometryRxArc>();
    let theme = use_context::<GlobalTheme>();
    // R3' A+B+C fix: shared visibility channel (same watch pattern as
    // `GlobalTheme`). The jewel click handlers write synchronously here;
    // inner/outer subscribe through their props and call
    // `window().set_visible(bool)` (windows.rs).
    let visibility_chan = use_context::<GlobalVisibility>();
    // r3p4 root-fix: shared slot written by entry.rs's tao event handler
    // to identify the room window; `room_app_root` fills it on mount so
    // only the room's Moved/Resized events publish geometry.
    let room_window_id = use_context::<std::sync::Arc<std::sync::Mutex<Option<dioxus::desktop::tao::window::WindowId>>>>();

    // R3' r3p3 fix #1 (Bug B root cause) - mount-once LocalePack.
    // The room window re-renders on every Signal `set`; loading the
    // pack from disk + parsing 147 keys on each re-render caused the
    // ~108ms reload cadence observed in r3p report §2. `use_hook`'s
    // closure runs exactly once on the first render and the resulting
    // `Rc` is returned by reference on subsequent renders, so the disk
    // read happens once for the lifetime of this window.
    let locale = use_hook(|| Rc::new(LocalePack::load(super::i18n::DEFAULT_LOCALE)));

    // Theme signal: written by the chrome toggle synchronously, read by
    // the chrome buttons to render the right `data-theme` attribute on
    // the <html> element.
    //
    // R3' r3p4 delta (2026-08-13) - Bug B root fix: the polling
    // `use_future` (100ms sleep + `theme.is_dark().await`) is deleted
    // entirely - a second sleeping use_future is what made the main
    // thread spin at ~97% CPU under dioxus 0.8-alpha.1 (fix brief §1).
    // The room now updates its local Signal synchronously in the click
    // handler and broadcasts the new value to inner/outer via the
    // `GlobalTheme` watch channel (which they subscribe to through
    // their props - see windows.rs).
    let mut theme_dark = use_signal(|| true);

    // Visibility state for inner / outer windows (brief §3.2 /
    // block-contract §3.1: jewel clicks toggle this, the dock task
    // reads it).
    let mut visibility = use_signal(VisibilityState::new);

    // Streaming state - drives the send/stop toggle (truth HTML L425
    // `sendStop.classList.toggle('streaming')`).
    let mut streaming = use_signal(|| false);

    // Mock chat log - seed with the truth HTML's verbatim record list
    // (L367..L415). New tokens stream in via `push_mock_stream`
    // (called below in a use_effect).
    let _entries = use_signal(|| seed_session());

    // Position publisher - r3p4 root-fix (2026-08-14): REMOVED entirely.
    //
    // The original 100ms `use_future` polling loop (with r3p3's
    // change-guarded send) is gone. Controlled CPU experiments (fix
    // brief §1 root cause + experiments A/B/C recorded in
    // `task-migrate-room-report-r3p4.md`) proved that ANY sleeping
    // use_future in the room window makes one background thread
    // busy-spin at ~97% single-core CPU on dioxus 0.8.0-alpha.1 - even
    // a bare `loop { sleep(100ms).await }` with no window()/send calls.
    // The polling shape itself is the poison.
    //
    // Geometry is now published event-driven from `entry.rs`: a tao
    // event-loop handler listens for the room window's Moved/Resized
    // OS events and sends the composed Geometry into the same watch
    // channel (this future's former contract). Inner/outer follow runs
    // on plain std::threads in `windows.rs` - kept off the dioxus task
    // system because drag experiments proved that waking a use_future
    // at drag frequency hangs all three windows.
    //
    // The mount-once initial publish happens in the spawn use_effect
    // below (room window id registration + one synchronous send).

    // Spawn the inner / outer VirtualDoms once on mount. The
    // `dioxus::desktop::window()` context is only valid after the
    // main window's launch has started, which is exactly when
    // use_effect fires (this is the spike's pattern from main.rs).
    //
    // R3' r3p4 delta: the room's `GlobalTheme` handle is cloned in and
    // `subscribe()` produces the theme receiver handed to the
    // inner/outer props - both windows render the room's initial theme
    // at mount and follow every toggle from then on (event-driven, no
    // polling involved).
    {
        let geometry_tx = geometry_tx.clone();
        let geometry_rx_arc = geometry_rx_arc.clone();
        let room_window_id = room_window_id.clone();
        let theme = theme.clone();
        let visibility_chan = visibility_chan.clone();
        let data_directory = shared_webview_data_directory_for_inner();
        use_effect(move || {
            // r3p4 root-fix: register the room's OS window id so the tao
            // event handler in entry.rs can filter Moved/Resized events,
            // then publish the initial geometry once (the event handler
            // takes over from here - zero polling).
            *room_window_id.lock().unwrap() = Some(window().id());
            if let Ok(pos) = window().outer_position() {
                let size = window().outer_size();
                let _ = geometry_tx.send(Geometry {
                    x: pos.x,
                    y: pos.y,
                    width: size.width,
                    height: size.height,
                });
            }
            let theme_rx = theme.subscribe();
            // R3' A+B+C: one receiver per spawned window (watch Receiver
            // is Clone; each clone consumes the full event stream, so both
            // windows react to every jewel toggle).
            let visibility_rx = visibility_chan.subscribe();
            spawn_inner_window(
                geometry_rx_arc.clone(),
                theme_rx.clone(),
                visibility_rx.clone(),
                data_directory.clone(),
            );
            spawn_outer_window(
                geometry_rx_arc.clone(),
                theme_rx,
                visibility_rx,
                data_directory.clone(),
            );
        });
    }

    let theme_class = if theme_dark() { "dark" } else { "light" };

    // R3' A+B+C (D): per-handler clones of the visibility channel —
    // each jewel onclick is a `move` closure and must own its handle.
    let visibility_chan_inner = visibility_chan.clone();
    let visibility_chan_outer = visibility_chan.clone();

    rsx! {
        body {
            "data-theme": "{theme_class}",
            lang: "zh-CN",
            // The truth HTML's `<head>` is wrapped by WebView2; in 0.8
            // alpha there's no `html` / `doctype` element exported so
            // we mount the body directly. We inject the truth CSS via
            // a `<style>` block as the body's first child so the
            // visual layout matches the HTML.
            style { dangerous_inner_html: "{css::TRUTH_CSS}" }
            // R3' A+B+C: 转写层覆盖样式（scrim 压暗层等）——TRUTH_CSS
            // 逐字节锁死，覆盖规则只能走这个第二 style 块。
            style { dangerous_inner_html: "{css::OVERLAY_CSS}" }
            meta { charset: "UTF-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }
            title { "{locale.t(keys::WINDOW_TITLE_ROOM)}" }

            // Background atmosphere layers - containment,
            // membrane-frame, global-aura (truth HTML LL277..L279).
            div { id: "containment" }
            div { class: "membrane-frame" }
            div { id: "global-aura" }

            // R3' A+B+C (D): scrim 压暗层——inner/outer 任一可见时自绘
            // （block-contract §2 规则 4 降级形态，22% 压暗，随 data-theme
            // 变色）。pointer-events:none 由 OVERLAY_CSS 保证，宝石可穿透。
            if visibility().inner_visible || visibility().outer_visible {
                div { id: "room-scrim" }
            }

            // Room - the central column. The inner / outer windows
            // are no longer siblings in the DOM tree; they're
            // separate OS windows. Brief §3.2 - the room DOM only
            // contains itself.
            div { id: "engine",
                div { id: "room-wrap",
                    section { id: "room",
                        // Membrane + jewel nodes - brief §3.1: jewel
                        // sits at the membrane line, drives
                        // visibility toggles.
                        span { class: "membrane l" }
                        span { class: "membrane r" }
                        // Vertical label - appears only when the
                        // corresponding window is hidden
                        // (block-contract §3.1 "竖签仅隐藏态出现";
                        // three-window docked default = no vertical
                        // label).
                        if !visibility().inner_visible {
                            div {
                                class: "vlabel inner",
                                style: "writing-mode: vertical-rl; letter-spacing: .35em; font-family: var(--font-mono); font-size: 10px; opacity: .55;",
                                "{locale.t(keys::VLABEL_INNER)}"
                            }
                        }
                        if !visibility().outer_visible {
                            div {
                                class: "vlabel outer",
                                style: "writing-mode: vertical-rl; letter-spacing: .35em; font-family: var(--font-mono); font-size: 10px; opacity: .55;",
                                "{locale.t(keys::VLABEL_OUTER)}"
                            }
                        }
                        div { class: "room-fog" }

                        // Window controls (truth HTML LL334..L340) -
                        // brief §3.1 F8: main window gets the full
                        // four-button set.
                        div { class: "room-controls",
                            button {
                                class: "rc-btn head-fold",
                                "aria-label": "收纳中枢",
                                title: "收纳/展开中枢",
                                "▴"
                            }
                            button {
                                class: "rc-btn",
                                id: "theme-toggle",
                                "aria-label": "切换明暗",
                                title: "切换明暗",
                                onclick: move |_| {
                                    // R3' r3p4: synchronous write - no
                                    // spawn, no await. The room's own
                                    // Signal flips instantly; the watch
                                    // channel broadcast reaches
                                    // inner/outer subscribers and they
                                    // re-render on their side.
                                    let next = !theme_dark();
                                    theme_dark.set(next);
                                    theme.set_dark(next);
                                },
                                if theme_dark() { "☀" } else { "☾" }
                            }
                            button { class: "rc-btn", "aria-label": "最小化", "─" }
                            button { class: "rc-btn", "aria-label": "最大化", "□" }
                            button {
                                class: "rc-btn close",
                                "aria-label": "关闭",
                                title: "关闭",
                                "✕"
                            }
                        }

                        // Status row - brand inline (5-path northing
                        // SVG, 200x200 viewBox, stroke=currentColor),
                        // followed by the "知序·在场" identity label
                        // and the state-dot. C4 ruling: brand lives
                        // here, NOT in a left-bottom seal.
                        div { class: "room-status",
                            span { class: "brand-inline",
                                svg {
                                    view_box: "0 0 200 200",
                                    "aria-label": "northing",
                                    path {
                                        d: "M 112.68 72.84 A 30 30 0 1 1 87.32 72.84",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "2.5",
                                        stroke_linecap: "round"
                                    }
                                    path {
                                        d: "M 126 54.97 A 52 52 0 1 1 82.28 51.22",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "5",
                                        stroke_linecap: "round"
                                    }
                                    path {
                                        d: "M 132.13 31.13 A 76 76 0 1 1 56.35 37.47",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "9",
                                        stroke_linecap: "round"
                                    }
                                    path {
                                        d: "M 56.35 37.47 Q 48 30, 44 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "8",
                                        stroke_linecap: "round"
                                    }
                                    path {
                                        d: "M 132.13 31.13 Q 137 24, 139 19",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "8",
                                        stroke_linecap: "round"
                                    }
                                }
                                span { class: "seal-name", "northing" }
                            }
                            span { "architect_sub 介入中" }
                            span { class: "sp" }
                            span { class: "state-dot" }
                        }

                        // Room-head - avatar (Fraunces italic 22px
                        // mono-glyph "序"), name, chronicle bar (4px
                        // tall, gradient drift), state pill (mind color
                        // 22% backdrop, square corners per C2
                        // ruling).
                        div { class: "room-head", id: "room-head",
                            div { class: "agent-avatar", id: "avatar-core",
                                "{locale.t(keys::ROOM_HEAD_INITIAL)}"
                            }
                            div { class: "name-line", "{locale.t(keys::ROOM_HEAD_NAME)}" }
                            div {
                                class: "chronicle-bar",
                                id: "chronicle-bar",
                                title: "它换代表色时：新色自右端进入，旧色慢慢沉向左（双击演示）"
                            }
                            div { class: "state",
                                "{locale.t(keys::ROOM_HEAD_STATE)}"
                            }
                        }

                        // Chat flow - mock session. The five record
                        // kinds render with a single `match`.
                        div { class: "chat-flow", id: "chat-flow",
                            div { class: "session-open",
                                "{locale.t(keys::SESSION_BANNER)}"
                            }
                            {render_entries(_entries.read().iter(), &locale)}
                        }

                        // Room input - deck with witness note,
                        // attach button, placeholder, send/stop.
                        // Send/stop toggles its class (truth HTML
                        // L597..L603, conversion-annotations §2
                        // row 5).
                        div { class: "room-input",
                            div { class: "witness-row",
                                span {
                                    class: "witness-note",
                                    "{locale.t(keys::DECK_WITNESS_NOTE)}"
                                }
                            }
                            div { class: "input-row",
                                button {
                                    class: "attach",
                                    "aria-label": "挂载文件",
                                    "{locale.t(keys::DECK_ATTACH)}"
                                }
                                div {
                                    class: "input-box",
                                    "{locale.t(keys::DECK_PLACEHOLDER)}"
                                    span { class: "cursor" }
                                }
                                button {
                                    class: if streaming() { "send streaming" } else { "send" },
                                    id: "send-stop",
                                    "aria-label": if streaming() { "停止" } else { "发送" },
                                    onclick: move |_| {
                                        streaming.set(!streaming());
                                    },
                                    if streaming() { "■" } else { "➤" }
                                }
                            }
                        }

                        // Membrane nodes (jewels) - drive inner /
                        // outer visibility (brief §3.1,
                        // conversion-annotations §2 row 3). R3' A+B+C:
                        // toggle writes the local Signal (vlabel +
                        // scrim re-render) and broadcasts over the
                        // GlobalVisibility watch channel so the OS
                        // window itself hides/shows (windows.rs).
                        button {
                            class: "membrane-node left",
                            id: "trig-mind",
                            "aria-label": "唤起 它的内在",
                            "aria-expanded": if visibility().inner_visible { "true" } else { "false" },
                            title: "它的内在",
                            onclick: move |_| {
                                visibility.write().toggle_inner();
                                visibility_chan_inner.set_inner(visibility().inner_visible);
                            }
                        }
                        button {
                            class: "membrane-node right",
                            id: "trig-work",
                            "aria-label": "唤起 身外之物",
                            "aria-expanded": if visibility().outer_visible { "true" } else { "false" },
                            title: "身外之物",
                            onclick: move |_| {
                                visibility.write().toggle_outer();
                                visibility_chan_outer.set_outer(visibility().outer_visible);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Spawn the inner-window VirtualDom + its own OS window.
///
/// `theme_rx` is the shared theme watch channel; the inner window reads
/// its initial value at mount and follows every change (see windows.rs).
/// `visibility_rx` is the shared visibility watch channel; the inner
/// window calls `window().set_visible(...)` on every jewel toggle.
fn spawn_inner_window(
    geometry_rx: GeometryRxArc,
    theme_rx: watch::Receiver<bool>,
    visibility_rx: watch::Receiver<VisibilityState>,
    data_directory: std::path::PathBuf,
) {
    let mut inner_builder = WindowBuilder::new()
        .with_title("northhing - inner (dioxus)")
        .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(INNER_WINDOW_WIDTH, ROOM_WINDOW_HEIGHT))
        .with_position(dioxus::desktop::tao::dpi::LogicalPosition::new(
            (ROOM_WINDOW_INITIAL_X as i32 - INNER_WINDOW_WIDTH as i32 - DOCK_GAP_PX) as f64,
            ROOM_WINDOW_INITIAL_Y as f64,
        ))
        .with_decorations(false);

    #[cfg(target_os = "windows")]
    {
        use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;
        inner_builder = inner_builder.with_skip_taskbar(true);
    }

    let cfg = Config::default()
        .with_window(inner_builder)
        .with_data_directory(data_directory);

    let offset_x = ROOM_WINDOW_INITIAL_X as i32 - INNER_WINDOW_WIDTH as i32 - DOCK_GAP_PX;
    let props = inner_app_root_props(geometry_rx, theme_rx, visibility_rx, offset_x);
    let dom = VirtualDom::new_with_props(inner_app_root, props);
    let _ = window().new_window(dom, cfg);
}

/// Symmetric to `spawn_inner_window`.
fn spawn_outer_window(
    geometry_rx: GeometryRxArc,
    theme_rx: watch::Receiver<bool>,
    visibility_rx: watch::Receiver<VisibilityState>,
    data_directory: std::path::PathBuf,
) {
    let mut outer_builder = WindowBuilder::new()
        .with_title("northhing - outer (dioxus)")
        .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(OUTER_WINDOW_WIDTH, ROOM_WINDOW_HEIGHT))
        .with_position(dioxus::desktop::tao::dpi::LogicalPosition::new(
            (ROOM_WINDOW_INITIAL_X as i32 + ROOM_WINDOW_WIDTH as i32 + DOCK_GAP_PX) as f64,
            ROOM_WINDOW_INITIAL_Y as f64,
        ))
        .with_decorations(false);

    #[cfg(target_os = "windows")]
    {
        use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;
        outer_builder = outer_builder.with_skip_taskbar(true);
    }

    let cfg = Config::default()
        .with_window(outer_builder)
        .with_data_directory(data_directory);

    let offset_x = ROOM_WINDOW_INITIAL_X as i32 + ROOM_WINDOW_WIDTH as i32 + DOCK_GAP_PX;
    let props = outer_app_root_props(geometry_rx, theme_rx, visibility_rx, offset_x);
    let dom = VirtualDom::new_with_props(outer_app_root, props);
    let _ = window().new_window(dom, cfg);
}

/// Render the mock chat-flow entries. Five kinds (entity, witness,
/// approval x 2 states) per brief §4.6 + truth HTML LL367..L415.
fn render_entries<'a>(
    iter: impl Iterator<Item = &'a MockEntry>,
    locale: &LocalePack,
) -> Element {
    let entries: Vec<&MockEntry> = iter.collect();
    rsx! {
        for entry in entries.iter() {
            {render_entry(entry, locale)}
        }
    }
}

fn render_entry(entry: &MockEntry, locale: &LocalePack) -> Element {
    match entry {
        MockEntry::Entity { who, body, children } => rsx! {
            div { class: "rec entity",
                div { class: "who", "{who}" }
                div { class: "body",
                    div { class: "msg-agent", "{body}" }
                    for child in children.iter() {
                        {render_child(child, locale)}
                    }
                }
            }
        },
        MockEntry::Witness { who, body } => rsx! {
            div { class: "rec witness",
                div { class: "who", "{who}" }
                div { class: "body", "{body}" }
            }
        },
        MockEntry::Approval { head, main, risk, resolved, state_text } => rsx! {
            div {
                class: "rec entity",
                style: "max-width:100%",
                div {
                    class: if *resolved { "approval-card resolved" } else { "approval-card" },
                    div { class: "approval-main",
                        div { class: "approval-head", "{head}" }
                        div { class: "approval-cmd", "{main}" }
                        div { class: "approval-risk", "{risk}" }
                    }
                    if *resolved {
                        div { class: "approval-state",
                            "{state_text.clone().unwrap_or_default()}"
                        }
                    } else {
                        div { class: "approval-actions",
                            button { class: "btn-approve",
                                "{locale.t(keys::APPROVAL_APPROVE)}"
                            }
                            button { class: "btn-reject",
                                "{locale.t(keys::APPROVAL_REJECT)}"
                            }
                        }
                    }
                }
            }
        },
    }
}

fn render_child(child: &super::session_mock::MockChild, _locale: &LocalePack) -> Element {
    match child {
        super::session_mock::MockChild::ToolLog { label } => rsx! {
            button { class: "tool-log", style: "border:none;padding:0", "{label}" }
        },
        super::session_mock::MockChild::ArtifactChip { label } => rsx! {
            button { class: "artifact-chip", "{label}" }
        },
    }
}