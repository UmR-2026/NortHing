// R3' migration (2026-08-13) - inner / outer window entry components.
//
// Mirrors the truth HTML `#mind` / `#work` aside (LL283..L457) in Dioxus
// RSX. The window-creation plumbing (Config + skip_taskbar +
// with_data_directory) lives in `app.rs`'s `spawn_inner_window` /
// `spawn_outer_window` helpers; this file only contains the render
// functions + the Props types that get passed via
// `VirtualDom::new_with_props(...)`.
//
// R3' delta vs R3 (0.7 -> 0.8 alpha): the `watch::Receiver` is wrapped
// in Arc because dioxus 0.8 root components take props by value and
// props must be Clone for `VirtualDom::new_with_props`; Arc provides
// the Clone impl automatically.
//
// R3' r3p3 delta (2026-08-13) - Bug B root cause fix, mount-once
// LocalePack. The original `inner_app_root` / `outer_app_root` called
// `LocalePack::load(...)` at the top of the body on every render. In
// practice these windows don't re-render often (their `theme_dark`
// Signal is never updated) so the cost was low, but we still apply
// the same fix as the room window so all three windows behave
// identically and a future change to theme propagation doesn't
// silently regress them. See `app.rs` file header for the full
// root-cause analysis.
//
// R3' r3p4 delta (2026-08-13) - Bug B root fix, event-driven theme:
//   The inner/outer windows now receive the shared theme watch channel
//   (`theme_rx`) through their props (same passing convention as the
//   geometry receiver) and fold it into the existing dock `use_future`
//   with `tokio::select!`: the `rx.changed()` (geometry) arm keeps its
//   original docking logic, the `theme_rx.changed()` arm updates the
//   local `theme_dark` Signal. No new futures, no polling - the
//   geometry future was already one of the proven-quiet shapes.

use dioxus::desktop::tao::dpi::{PhysicalPosition, Position};
use dioxus::desktop::window;
use dioxus::prelude::*;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::watch;

use super::css;
use super::entry::{DOCK_GAP_PX, INNER_WINDOW_WIDTH, OUTER_WINDOW_WIDTH};
use super::i18n::{keys, LocalePack};
use super::state::{Geometry, GeometryRxArc, VisibilityState};

/// Windows-only helpers for the geometry follow threads (r3p4 root-fix):
/// the geometry watch channel is consumed on a plain std::thread that
/// moves the OS window with Win32 SetWindowPos - off the dioxus task
/// system entirely (see `inner_app_root` for the full rationale).
#[cfg(target_os = "windows")]
mod win {
    use std::ffi::c_void;

    /// Move a top-level window. SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE
    /// are passed by callers via `u_flags`. The HWND travels across the
    /// thread boundary as a `usize` (plain integer, trivially Send).
    unsafe extern "system" {
        pub fn SetWindowPos(
            h_wnd: *mut c_void,
            h_wnd_insert_after: *mut c_void,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            u_flags: u32,
        ) -> i32;
    }
}

/// Props for the inner (它的自我) window.
///
/// `rx` is the geometry watch channel (Arc-wrapped Receiver). `offset_x`
/// is the inner window's left-dock offset relative to the room's x;
/// the follow task uses this to position the window each time the
/// room moves. `theme_rx` is the shared theme watch channel; the
/// initial value seeds the window's `data-theme` and every later
/// change re-renders it. `visibility_rx` is the shared visibility
/// watch channel; `window().set_visible(...)` follows every jewel
/// toggle from the room.
#[derive(Props, Clone)]
pub struct InnerAppProps {
    pub rx: GeometryRxArc,
    pub theme_rx: watch::Receiver<bool>,
    pub visibility_rx: watch::Receiver<VisibilityState>,
    pub offset_x: i32,
}

/// Props for the outer (身外之物) window. Same shape as the inner; kept
/// as a separate type so the two windows can evolve independently.
#[derive(Props, Clone)]
pub struct OuterAppProps {
    pub rx: GeometryRxArc,
    pub theme_rx: watch::Receiver<bool>,
    pub visibility_rx: watch::Receiver<VisibilityState>,
    pub offset_x: i32,
}

/// Helper to build `InnerAppProps` from the main window. Kept as a free
/// function so `app.rs` can call it without exposing the Props type's
/// fields.
pub fn inner_app_root_props(
    rx: GeometryRxArc,
    theme_rx: watch::Receiver<bool>,
    visibility_rx: watch::Receiver<VisibilityState>,
    offset_x: i32,
) -> InnerAppProps {
    InnerAppProps {
        rx,
        theme_rx,
        visibility_rx,
        offset_x,
    }
}

/// Helper to build `OuterAppProps` from the main window. Symmetric to
/// `inner_app_root_props`.
pub fn outer_app_root_props(
    rx: GeometryRxArc,
    theme_rx: watch::Receiver<bool>,
    visibility_rx: watch::Receiver<VisibilityState>,
    offset_x: i32,
) -> OuterAppProps {
    OuterAppProps {
        rx,
        theme_rx,
        visibility_rx,
        offset_x,
    }
}

/// Manual `PartialEq` impl: dioxus 0.8 still requires `Props` to be
/// `PartialEq` (used by the vdom diff). The receiver is a streaming
/// channel that does not meaningfully implement PartialEq; we follow
/// the spike's "恒真" hack (main.rs 行 127-131) and return `true`
/// unconditionally. The follow task inside `inner_app_root` /
/// `outer_app_root` reads via `watch::Receiver::borrow()` so the
/// diff-on-equality path is never actually taken.
impl PartialEq for InnerAppProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl PartialEq for OuterAppProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

/// Inner (它的自我) window root. Mirrors the truth HTML `#mind` aside
/// (LL283..L324) — two stacked modules (它的自我 station-head + facility
/// station-head) with the same section breakdown (沉积记忆 / 模型引擎 /
/// 上下文 / 核心准则 / 知识沉积).
///
/// Dioxus 0.8 entry point: `fn(InnerAppProps) -> Element` (called by
/// `VirtualDom::new_with_props(...)` from `room_app_root`'s use_effect).
pub fn inner_app_root(props: InnerAppProps) -> Element {
    let locale = use_hook(|| Rc::new(LocalePack::load(super::i18n::DEFAULT_LOCALE)));
    let offset_x = props.offset_x;
    let rx_arc = props.rx.clone();
    let theme_rx = props.theme_rx.clone();
    let visibility_rx = props.visibility_rx.clone();
    // R3' r3p4: the initial theme comes from the shared channel instead
    // of a hardcoded `true`, so the window opens with the room's actual
    // theme even if it was toggled before this window spawned.
    let theme_dark = use_signal(|| *theme_rx.borrow());

    // R3' r3p4 root-fix (2026-08-14): geometry follow moved OFF the
    // dioxus task system on Windows.
    //
    // Drag experiments (recorded in `task-migrate-room-report-r3p4.md`)
    // proved that waking a dioxus use_future at drag frequency (60+
    // wakes/s via the geometry watch channel) makes dioxus 0.8.0-alpha.1
    // busy-spin at ~97% single-core and hang all three windows; the
    // `UserWindowEvent::Poll` round-trips pile up faster than poll_vdom
    // drains them. The fix removes geometry from the task system
    // entirely: a plain std::thread waits on the watch channel and moves
    // the OS window with Win32 SetWindowPos directly - no dioxus task,
    // no waker, no event-loop round trip. The HWND is captured once at
    // mount. The thread exits when the channel closes (app teardown).
    //
    // The theme channel stays in use_future: it is woken only by user
    // toggle (low frequency), which the experiments showed is safe.
    //
    // R3' A+B+C fix (Bug A): the geometry channel is physical px but
    // the dock offsets (INNER_WINDOW_WIDTH + DOCK_GAP_PX) are logical
    // (the window is created with LogicalSize). The scale factor is
    // captured at mount and the offsets are converted before
    // SetWindowPos — at 125% DPI the old code subtracted 296 physical
    // where 350+20=370 was required, overlapping the room by ~74px.
    #[cfg(target_os = "windows")]
    {
        use dioxus::desktop::tao::platform::windows::WindowExtWindows;

        let rx = rx_arc.clone();
        // HWND as usize: a plain integer so it is trivially Send across
        // the thread boundary (raw pointers are !Send).
        let hwnd_usize = window().hwnd() as usize;
        let off = ((INNER_WINDOW_WIDTH + DOCK_GAP_PX as f64) * window().scale_factor()) as i32;
        use_hook(move || {
            std::thread::Builder::new()
                .name("inner-geometry-follow".into())
                .spawn(move || {
                    let hwnd_ptr = hwnd_usize as *mut std::ffi::c_void;
                    let mut rx: watch::Receiver<Geometry> = (*rx).clone();
                    let mut last = *rx.borrow();
                    loop {
                        // OS-level blocking sleep (not tokio): the thread
                        // is truly parked between checks, so this costs
                        // ~0% CPU - unlike the dioxus-task sleep loops
                        // that the drag experiments proved busy-spin.
                        std::thread::sleep(std::time::Duration::from_millis(16));
                        let cur = *rx.borrow_and_update();
                        // Geometry has no PartialEq (r3p3: state.rs stays
                        // whitelist-clean); compare field-by-field.
                        if cur.x == last.x
                            && cur.y == last.y
                            && cur.width == last.width
                            && cur.height == last.height
                        {
                            continue;
                        }
                        last = cur;
                        // SWP_NOSIZE(0x0001) | SWP_NOZORDER(0x0004) |
                        // SWP_NOACTIVATE(0x0010): move only, keep size and
                        // z-order, do not steal focus. Same dock offset as
                        // the original tao set_outer_position path, now
                        // converted to physical px.
                        unsafe {
                            let _ = win::SetWindowPos(
                                hwnd_ptr,
                                std::ptr::null_mut(),
                                cur.x.saturating_sub(off),
                                cur.y,
                                0,
                                0,
                                0x0001 | 0x0004 | 0x0010,
                            );
                        }
                        let _ = offset_x;
                    }
                })
                .expect("spawn inner geometry follow thread");
        });
    }

    // Non-Windows fallback: keep the original dioxus-task follow (the
    // busy-spin bug is Windows-specific to the WebView2 stack).
    #[cfg(not(target_os = "windows"))]
    use_future(move || {
        let _ = offset_x; // offset_x is only consumed by the Win32 path; keep the API live
        let rx_arc = rx_arc.clone();
        let off = ((INNER_WINDOW_WIDTH + DOCK_GAP_PX as f64) * window().scale_factor()) as i32;
        async move {
            let mut rx: watch::Receiver<Geometry> = (*rx_arc).clone();
            loop {
                if rx.changed().await.is_err() {
                    break;
                }
                let g = *rx.borrow();
                let w = window();
                let _ = w.set_outer_position(Position::Physical(PhysicalPosition::new(
                    g.x.saturating_sub(off),
                    g.y,
                )));
                let _ = w.request_redraw();
            }
        }
    });

    // Theme follower (low-frequency; safe on the dioxus task system).
    use_future(move || {
        let mut theme_rx = theme_rx.clone();
        let mut theme_dark = theme_dark.clone();
        async move {
            loop {
                if theme_rx.changed().await.is_err() {
                    break;
                }
                theme_dark.set(*theme_rx.borrow());
            }
        }
    });

    // R3' A+B+C (D): visibility follower — same event-driven watch
    // pattern as the theme follower (no sleep anywhere; woken only by
    // the room's jewel clicks, which are user-paced). Initial sync on
    // mount covers the case where the window spawned after a toggle.
    use_future(move || {
        let mut visibility_rx = visibility_rx.clone();
        async move {
            let state = *visibility_rx.borrow();
            window().set_visible(state.inner_visible);
            loop {
                if visibility_rx.changed().await.is_err() {
                    break;
                }
                let state = *visibility_rx.borrow();
                window().set_visible(state.inner_visible);
            }
        }
    });

    let class = if theme_dark() { "dark" } else { "light" };
    rsx! {
        body {
            "data-theme": "{class}",
            "data-window": "inner",
            // The truth HTML's `<head>` is wrapped by WebView2; in 0.8
            // alpha there's no `html` / `doctype` element exported so
            // we mount the body directly. We inject the truth CSS via
            // a `<style>` block so the visual layout matches the HTML.
            style { dangerous_inner_html: "{css::TRUTH_CSS}" }
            // R3' A+B+C (C): 转写层覆盖样式收口（宽度/横溢），TRUTH_CSS
            // 逐字节锁死，覆盖规则只能走这个第二 style 块。
            style { dangerous_inner_html: "{css::OVERLAY_CSS}" }
            meta { charset: "UTF-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }
            aside {
                id: "mind",
                class: "mod",
                div {
                    class: "mod",
                    "data-drag": "true",
                    div {
                        class: "station-head",
                        "{locale.t(keys::INNER_HEAD_TITLE)}",
                        button { class: "fold-btn", "▴ 收纳" }
                    }
                    div { class: "card-body",
                        div { class: "side-section",
                            div { class: "side-title",
                                "{locale.t(keys::INNER_SECTION_SEDIMENT_TITLE)} "
                                em { "{locale.t(keys::INNER_SECTION_SEDIMENT_EM)}" }
                            }
                            div { class: "row", "# 边界不是围墙" button { class: "tag-x", "×" } }
                            div { class: "row", "# 观察先于干预" button { class: "tag-x", "×" } }
                            div { class: "row", "# 允许未完成" button { class: "tag-x", "×" } }
                            div { class: "seg-bar",
                                div { class: "seg on" }
                                div { class: "seg on" }
                                div { class: "seg on" }
                                div { class: "seg" }
                                div { class: "seg" }
                            }
                            div { class: "seg-note",
                                "{locale.t(keys::INNER_SECTION_SEDIMENT_NOTE)}"
                            }
                        }
                    }
                }
                div {
                    class: "mod",
                    "data-drag": "true",
                    div {
                        class: "station-head facility",
                        "{locale.t(keys::INNER_HEAD_FACILITY_TITLE)}",
                        button { class: "fold-btn", "▴ 收纳" }
                    }
                    div { class: "card-body",
                        div { class: "side-section",
                            div { class: "side-title",
                                "{locale.t(keys::INNER_SECTION_ENGINE_TITLE)} "
                                em { "{locale.t(keys::INNER_SECTION_ENGINE_EM)}" }
                            }
                            div { class: "row active", span { class: "dot-radio" }, "Claude 3.7 · 主人格" }
                            div { class: "row", span { class: "dot-radio" }, "route.search: Haiku" }
                        }
                        div { class: "side-section", id: "ctx-section",
                            div { class: "side-title",
                                "{locale.t(keys::INNER_SECTION_CONTEXT_TITLE)} "
                                em { "{locale.t(keys::INNER_SECTION_CONTEXT_EM)}" }
                                button { class: "fold-btn ctx-fold", "▴" }
                            }
                            div { class: "ctx-body",
                                div { class: "row active", span { class: "dot-radio" }, "还宽，慢慢来" }
                                div { class: "seg-bar",
                                    div { class: "seg on" }
                                    div { class: "seg on" }
                                    div { class: "seg" }
                                    div { class: "seg" }
                                    div { class: "seg" }
                                }
                            }
                        }
                        div { class: "side-section",
                            div { class: "side-title",
                                "{locale.t(keys::INNER_SECTION_AXIOMS_TITLE)} "
                                em { "{locale.t(keys::INNER_SECTION_AXIOMS_EM)}" }
                            }
                            div { class: "row active", span { class: "sq-toggle" }, "维护主体边界" }
                            div { class: "row", span { class: "sq-toggle" }, "隐喻性修辞" }
                        }
                        div { class: "side-section",
                            div { class: "side-title",
                                "{locale.t(keys::INNER_SECTION_RAG_TITLE)} "
                                em { "{locale.t(keys::INNER_SECTION_RAG_EM)}" }
                            }
                            div { class: "row active",
                                "@philosophy-core "
                                span {
                                    class: "tag-x",
                                    style: "color:var(--mind-line);cursor:default",
                                    "{locale.t(keys::INNER_RAG_MOUNTED)}"
                                }
                            }
                        }
                        button { class: "sys-config", "≡ {locale.t(keys::INNER_GLOBAL_SETTINGS)}" }
                    }
                }
            }
        }
    }
}

/// Outer (身外之物) window root. Mirrors the truth HTML `#work` aside
/// (LL432..L457) — a single station with 子体路由 / 目标拆解 /
/// 文件差异审查 / 终端井 sections.
pub fn outer_app_root(props: OuterAppProps) -> Element {
    let locale = use_hook(|| Rc::new(LocalePack::load(super::i18n::DEFAULT_LOCALE)));
    let offset_x = props.offset_x;
    let rx_arc = props.rx.clone();
    let theme_rx = props.theme_rx.clone();
    let visibility_rx = props.visibility_rx.clone();
    // R3' r3p4: initial theme from the shared channel (see inner_app_root).
    let theme_dark = use_signal(|| *theme_rx.borrow());

    // R3' r3p4 root-fix: same as inner_app_root - geometry follow runs
    // on a plain std::thread with Win32 SetWindowPos (see inner for the
    // full rationale; the outer docks to the room's right edge).
    // R3' A+B+C fix (Bug A): DOCK_GAP_PX is logical; convert with the
    // mount-time scale factor so the dock gap is physical px at the
    // room's right edge (16 logical -> 20 physical at 125% DPI).
    #[cfg(target_os = "windows")]
    {
        use dioxus::desktop::tao::platform::windows::WindowExtWindows;

        let rx = rx_arc.clone();
        let hwnd_usize = window().hwnd() as usize;
        let off = (DOCK_GAP_PX as f64 * window().scale_factor()) as i32;
        use_hook(move || {
            std::thread::Builder::new()
                .name("outer-geometry-follow".into())
                .spawn(move || {
                    let hwnd_ptr = hwnd_usize as *mut std::ffi::c_void;
                    let mut rx: watch::Receiver<Geometry> = (*rx).clone();
                    let mut last = *rx.borrow();
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(16));
                        let cur = *rx.borrow_and_update();
                        if cur.x == last.x
                            && cur.y == last.y
                            && cur.width == last.width
                            && cur.height == last.height
                        {
                            continue;
                        }
                        last = cur;
                        unsafe {
                            let _ = win::SetWindowPos(
                                hwnd_ptr,
                                std::ptr::null_mut(),
                                cur.x + cur.width as i32 + off,
                                cur.y,
                                0,
                                0,
                                0x0001 | 0x0004 | 0x0010,
                            );
                        }
                        let _ = offset_x;
                    }
                })
                .expect("spawn outer geometry follow thread");
        });
    }

    // Non-Windows fallback: keep the original dioxus-task follow.
    #[cfg(not(target_os = "windows"))]
    use_future(move || {
        let _ = offset_x; // offset_x is only consumed by the Win32 path; keep the API live
        let rx_arc = rx_arc.clone();
        let off = (DOCK_GAP_PX as f64 * window().scale_factor()) as i32;
        async move {
            let mut rx: watch::Receiver<Geometry> = (*rx_arc).clone();
            loop {
                if rx.changed().await.is_err() {
                    break;
                }
                let g = *rx.borrow();
                let w = window();
                let _ = w.set_outer_position(Position::Physical(PhysicalPosition::new(
                    g.x + g.width as i32 + off,
                    g.y,
                )));
                let _ = w.request_redraw();
            }
        }
    });

    // Theme follower (low-frequency; safe on the dioxus task system).
    use_future(move || {
        let mut theme_rx = theme_rx.clone();
        let mut theme_dark = theme_dark.clone();
        async move {
            loop {
                if theme_rx.changed().await.is_err() {
                    break;
                }
                theme_dark.set(*theme_rx.borrow());
            }
        }
    });

    // R3' A+B+C (D): visibility follower (event-driven watch, no sleep).
    use_future(move || {
        let mut visibility_rx = visibility_rx.clone();
        async move {
            let state = *visibility_rx.borrow();
            window().set_visible(state.outer_visible);
            loop {
                if visibility_rx.changed().await.is_err() {
                    break;
                }
                let state = *visibility_rx.borrow();
                window().set_visible(state.outer_visible);
            }
        }
    });

    let class = if theme_dark() { "dark" } else { "light" };
    rsx! {
        body {
            "data-theme": "{class}",
            "data-window": "outer",
            style { dangerous_inner_html: "{css::TRUTH_CSS}" }
            // R3' A+B+C (C): 转写层覆盖样式收口（宽度/横溢），TRUTH_CSS
            // 逐字节锁死，覆盖规则只能走这个第二 style 块。
            style { dangerous_inner_html: "{css::OVERLAY_CSS}" }
            meta { charset: "UTF-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }
            aside {
                id: "work",
                class: "mod",
                "data-drag": "true",
                div {
                    class: "station-head facility",
                    "{locale.t(keys::OUTER_HEAD_TITLE)}",
                    button { class: "fold-btn", id: "work-fold", "▾ 收纳" }
                }
                div { class: "side-section",
                    div { class: "side-title",
                        "{locale.t(keys::OUTER_SECTION_ROUTING_TITLE)} "
                        em { "{locale.t(keys::OUTER_SECTION_ROUTING_EM)}" }
                    }
                    div { class: "row active",
                        span { class: "dot-radio" },
                        "架构师",
                        span {
                            style: "margin-left:auto;color:var(--mind-line);font-size:10px",
                            "{locale.t(keys::OUTER_SECTION_ROUTING_INTERVENING)}"
                        }
                    }
                    div { class: "row",
                        span { class: "dot-radio" },
                        "search · Haiku",
                        span {
                            style: "margin-left:auto;color:var(--faint);font-size:10px",
                            "{locale.t(keys::OUTER_SECTION_ROUTING_STANDBY)}"
                        }
                    }
                }
                div { class: "side-section",
                    div { class: "side-title",
                        "{locale.t(keys::OUTER_SECTION_PLANNER_TITLE)} "
                        em { "{locale.t(keys::OUTER_SECTION_PLANNER_EM)}" }
                    }
                    div { class: "row active",
                        span {
                            class: "plan-check",
                            style: "border-color:var(--accent-solid);background:var(--accent-solid)"
                        },
                        "重新定义对齐 ",
                        span {
                            style: "margin-left:auto;color:var(--mind-line);font-size:10px",
                            "{locale.t(keys::OUTER_SECTION_PLANNER_INPROGRESS)}"
                        }
                    }
                    div { class: "sub-step", "├ 读取沉积记忆" }
                    div { class: "sub-step", "└ 写入行动准则" }
                    div { class: "row done",
                        span { class: "plan-check" },
                        "建立隔离沙盒"
                    }
                }
                div { class: "side-section",
                    div { class: "side-title",
                        "{locale.t(keys::OUTER_SECTION_DIFF_TITLE)} "
                        em { "{locale.t(keys::OUTER_SECTION_DIFF_EM)}" }
                    }
                    div { class: "row",
                        span { class: "fname", "alignment.md" },
                        span { class: "diff-add", "+18" },
                        span { class: "diff-del", "-06" }
                    }
                    div { class: "btn-undo", "{locale.t(keys::OUTER_DIFF_REVERTED)}" }
                }
                div { class: "term-well",
                    "$ northing inspect --boundary\n> 3 observers / clean\n> "
                    span { class: "preview-row", "preview: localhost:4173" }
                    "\n> _"
                }
            }
        }
    }
}