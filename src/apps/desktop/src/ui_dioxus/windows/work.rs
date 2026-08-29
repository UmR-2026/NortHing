// SPDX-License-Identifier: MIT OR Apache-2.0
//
// W10-2 split — Work ("身外之物") module window root component.

use crate::ui_dioxus::css;
use crate::ui_dioxus::entry::DOCK_GAP_PX;
use crate::ui_dioxus::i18n::{keys, LocalePack};
use crate::ui_dioxus::panel_files;
use crate::ui_dioxus::registry::ModuleAppProps;
use crate::ui_dioxus::state::Geometry;
use crate::ui_dioxus::windows::WindowDropGuard;
use dioxus::desktop::window;
use dioxus::prelude::*;
#[cfg(target_os = "windows")]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;
use std::rc::Rc;
use tokio::sync::watch;

use crate::ui_dioxus::windows::{win};

/// Work ("身外之物") module window root component.
pub fn work_app_root(props: ModuleAppProps) -> Element {
    let locale = use_hook(|| Rc::new(LocalePack::load(crate::ui_dioxus::i18n::DEFAULT_LOCALE)));
    let plugin_id = props.plugin_id;
    let gen = props.gen;
    let manager = props.manager.clone();

    let mgr_guard = manager.clone();
    use_hook(move || Rc::new(WindowDropGuard::new(plugin_id, gen, mgr_guard)));

    {
        let manager = manager.clone();
        use_effect(move || {
            let wid = window().id();
            #[cfg(target_os = "windows")]
            let hwnd = window().hwnd() as usize;
            #[cfg(not(target_os = "windows"))]
            let hwnd = 0usize;

            if !manager.register_window_with_hwnd(plugin_id, gen, wid, hwnd) {
                #[cfg(target_os = "windows")]
                win::hide_and_close_hwnd(hwnd as isize);
                window().close();
            }
        });
    }

    let rx_arc = props.rx.clone();
    let theme_rx = props.theme_rx.clone();
    let theme_dark = use_signal(|| *theme_rx.borrow());

    #[cfg(target_os = "windows")]
    {
        let rx = rx_arc.clone();
        let hwnd_usize = window().hwnd() as usize;
        use_hook(move || {
            std::thread::Builder::new()
                .name("work-geometry-follow".into())
                .spawn(move || {
                    let hwnd_ptr = hwnd_usize as *mut std::ffi::c_void;
                    let mut rx: watch::Receiver<Geometry> = (*rx).clone();
                    let mut last = *rx.borrow();
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(16));
                        if rx.has_changed().is_err() {
                            break;
                        }
                        let cur = *rx.borrow_and_update();
                        if cur.x == last.x && cur.y == last.y && cur.width == last.width && cur.height == last.height {
                            continue;
                        }
                        last = cur;
                        let dpi = unsafe { win::GetDpiForWindow(hwnd_ptr) };
                        let scale = if dpi > 0 { dpi as f64 / 96.0 } else { 1.0 };
                        let off_x = (DOCK_GAP_PX as f64 * scale) as i32;
                        let target_x = cur.x + cur.width as i32 + off_x;
                        let target_y = cur.y;
                        let target_w = (320.0 * scale) as i32;
                        let target_h = cur.height as i32;

                        unsafe {
                            if win::IsWindow(hwnd_ptr) == 0 || win::IsWindowVisible(hwnd_ptr) == 0 {
                                break;
                            }
                            let _ = win::SetWindowPos(
                                hwnd_ptr,
                                std::ptr::null_mut(),
                                target_x,
                                target_y,
                                target_w,
                                target_h,
                                0x0004 | 0x0010,
                            );
                        }
                    }
                })
                .expect("spawn work geometry follow thread");
        });
    }

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

    let class = if theme_dark() { "dark" } else { "light" };

    let mut folded_routing = use_signal(|| false);
    let mut folded_planner = use_signal(|| false);
    let mut folded_diff = use_signal(|| false);
    // folded_files opts out of fold_all by design (see panel_files::render_files_section).
    let folded_files = use_signal(|| false);

    let fold_all = move |_| {
        let target = !folded_routing() || !folded_planner() || !folded_diff();
        folded_routing.set(target);
        folded_planner.set(target);
        folded_diff.set(target);
    };

    rsx! {
        body {
            "data-theme": "{class}",
            "data-window": "outer",
            style { dangerous_inner_html: "{css::truth_css()}" }
            style { dangerous_inner_html: "{css::OVERLAY_CSS}" }
            meta { charset: "UTF-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }
            aside {
                id: "work",
                class: "mod",
                "data-drag": "true",
                div {
                    class: "station-head w2-head",
                    onmousedown: move |_| { window().drag(); },
                    "{locale.t(keys::OUTER_HEAD_TITLE)}",
                    button {
                        class: "fold-btn",
                        id: "work-fold",
                        onmousedown: move |e| { e.stop_propagation(); },
                        onclick: fold_all,
                        "▴ {locale.t(keys::WINDOW_FOLD_BTN)}"
                    }
                    button {
                        class: "close-btn",
                        title: "{locale.t(keys::WINDOW_CLOSE_BTN)}",
                        onmousedown: move |e| { e.stop_propagation(); },
                        onclick: move |_| {
                            #[cfg(target_os = "windows")]
                            win::hide_and_close_hwnd(window().hwnd());
                            window().close();
                        },
                        "✕"
                    }
                }
                div {
                    class: if folded_routing() { "side-section is-folded" } else { "side-section" },
                    div {
                        class: "side-title",
                        onclick: move |_| { folded_routing.toggle(); },
                        "{locale.t(keys::OUTER_SECTION_ROUTING_TITLE)} "
                        em { "{locale.t(keys::OUTER_SECTION_ROUTING_EM)}" }
                        span { class: "fold-caret", if folded_routing() { "▸" } else { "▾" } }
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
                div {
                    class: if folded_planner() { "side-section is-folded" } else { "side-section" },
                    div {
                        class: "side-title",
                        onclick: move |_| { folded_planner.toggle(); },
                        "{locale.t(keys::OUTER_SECTION_PLANNER_TITLE)} "
                        em { "{locale.t(keys::OUTER_SECTION_PLANNER_EM)}" }
                        span { class: "fold-caret", if folded_planner() { "▸" } else { "▾" } }
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
                div {
                    class: if folded_diff() { "side-section is-folded" } else { "side-section" },
                    div {
                        class: "side-title",
                        onclick: move |_| { folded_diff.toggle(); },
                        "{locale.t(keys::OUTER_SECTION_DIFF_TITLE)} "
                        em { "{locale.t(keys::OUTER_SECTION_DIFF_EM)}" }
                        span { class: "fold-caret", if folded_diff() { "▸" } else { "▾" } }
                    }
                    div { class: "row",
                        span { class: "fname", "alignment.md" },
                        span { class: "diff-add", "+18" },
                        span { class: "diff-del", "-06" }
                    }
                    div { class: "btn-undo", "{locale.t(keys::OUTER_DIFF_REVERTED)}" }
                }{panel_files::render_files_section(&locale, folded_files)}
                div { class: "term-well",
                    "$ northing inspect --boundary\n> 3 observers / clean\n> "
                    span { class: "preview-row", "preview: localhost:4173" }
                    "\n> _"
                }
            }
        }
    }
}
