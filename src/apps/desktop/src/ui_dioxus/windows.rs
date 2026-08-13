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

use dioxus::desktop::tao::dpi::{PhysicalPosition, Position};
use dioxus::desktop::window;
use dioxus::prelude::*;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::watch;

use super::css;
use super::i18n::{keys, LocalePack};
use super::state::{Geometry, GeometryRxArc};

/// Props for the inner (它的自我) window.
///
/// `rx` is the geometry watch channel (Arc-wrapped Receiver). `offset_x`
/// is the inner window's left-dock offset relative to the room's x;
/// the follow task uses this to position the window each time the
/// room moves.
#[derive(Props, Clone)]
pub struct InnerAppProps {
    pub rx: GeometryRxArc,
    pub offset_x: i32,
}

/// Props for the outer (身外之物) window. Same shape as the inner; kept
/// as a separate type so the two windows can evolve independently.
#[derive(Props, Clone)]
pub struct OuterAppProps {
    pub rx: GeometryRxArc,
    pub offset_x: i32,
}

/// Helper to build `InnerAppProps` from the main window. Kept as a free
/// function so `app.rs` can call it without exposing the Props type's
/// fields.
pub fn inner_app_root_props(rx: GeometryRxArc, offset_x: i32) -> InnerAppProps {
    InnerAppProps { rx, offset_x }
}

/// Helper to build `OuterAppProps` from the main window. Symmetric to
/// `inner_app_root_props`.
pub fn outer_app_root_props(rx: GeometryRxArc, offset_x: i32) -> OuterAppProps {
    OuterAppProps { rx, offset_x }
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
    let mut theme_dark = use_signal(|| true);

    // Follow task: subscribe to the room's geometry channel and dock the
    // inner window to the room's left edge whenever the room moves.
    // `use_future` returns a `Task`; the closure runs once on mount and
    // continues until the future completes.
    use_future(move || {
        let rx_arc = rx_arc.clone();
        let off = offset_x;
        async move {
            let mut rx: watch::Receiver<Geometry> = (*rx_arc).clone();
            loop {
                if rx.changed().await.is_err() {
                    break;
                }
                let g = *rx.borrow();
                let w = window();
                // DOCK_GAP_PX (16) + INNER_WINDOW_WIDTH (280) is the
                // constant offset; the inner sits that far to the left
                // of the room. `saturating_sub` keeps it at zero if the
                // room is dragged off the left edge.
                let _ = w.set_outer_position(Position::Physical(PhysicalPosition::new(
                    g.x.saturating_sub(280 + 16),
                    g.y,
                )));
                let _ = w.request_redraw();
                let _ = off;
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
    let rx_arc = props.rx.clone();
    let mut theme_dark = use_signal(|| true);

    use_future(move || {
        let rx_arc = rx_arc.clone();
        let off = props.offset_x;
        async move {
            let mut rx: watch::Receiver<Geometry> = (*rx_arc).clone();
            loop {
                if rx.changed().await.is_err() {
                    break;
                }
                let g = *rx.borrow();
                let w = window();
                let _ = w.set_outer_position(Position::Physical(PhysicalPosition::new(
                    g.x + g.width as i32 + 16,
                    g.y,
                )));
                let _ = w.request_redraw();
                let _ = off;
            }
        }
    });

    let class = if theme_dark() { "dark" } else { "light" };
    rsx! {
        body {
            "data-theme": "{class}",
            "data-window": "outer",
            style { dangerous_inner_html: "{css::TRUTH_CSS}" }
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