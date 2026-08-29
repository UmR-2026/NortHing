// SPDX-License-Identifier: MIT OR Apache-2.0
//
// W10-2 split — Facility ("设施") module window root component.

use crate::ui_dioxus::css;
use crate::ui_dioxus::i18n::{keys, LocalePack};
use crate::ui_dioxus::registry::ModuleAppProps;
use crate::ui_dioxus::state::Geometry;
use crate::ui_dioxus::windows::WindowDropGuard;
use dioxus::desktop::window;
use dioxus::prelude::*;
#[cfg(target_os = "windows")]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;
use std::rc::Rc;
use tokio::sync::watch;

use super::fmt_tokens;
use crate::ui_dioxus::windows::win;

/// Facility ("设施") module window root component.
pub fn facility_app_root(props: ModuleAppProps) -> Element {
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
                .name("facility-geometry-follow".into())
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
                        let off_x = ((280.0) * scale) as i32;
                        let half_h = (cur.height as f64 / 2.0) as i32;
                        let target_x = cur.x.saturating_sub(off_x);
                        let target_y = cur.y + half_h;
                        let target_w = (280.0 * scale) as i32;
                        let target_h = cur.height as i32 - half_h;

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
                .expect("spawn facility geometry follow thread");
        });
    }

    let theme_rx_for_settings = props.theme_rx.clone();
    let theme_rx_for_future = props.theme_rx.clone();
    use_future(move || {
        let mut theme_rx = theme_rx_for_future.clone();
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
    // W2 视觉解耦：设施窗 = 两卡。卡1 RUNTIME（模型引擎 + 上下文 +
    // token 消耗/清空 + 全局设置），卡2 AXIOMS 独立浮卡。RAG 已迁入
    // 沉积窗（self_app_root）。token 消耗为 mock 计数，「清空」归零。
    let mut token_used = use_signal(|| 128_437u64);
    let token_text = fmt_tokens(token_used());
    rsx! {
        body {
            "data-theme": "{class}",
            "data-window": "inner",
            style { dangerous_inner_html: "{css::truth_css()}" }
            style { dangerous_inner_html: "{css::OVERLAY_CSS}" }
            meta { charset: "UTF-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }
            aside {
                id: "mind",
                div {
                    class: "station-head facility w2-head",
                    "data-drag": "true",
                    onmousedown: move |_| { window().drag(); },
                    "{locale.t(keys::INNER_HEAD_FACILITY_TITLE)}",
                    button {
                        class: "fold-btn",
                        onmousedown: move |e| { e.stop_propagation(); },
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
                div { class: "mod w2c-runtime",
                    div { class: "side-title w2-pin",
                        "{locale.t(keys::INNER_SECTION_RUNTIME_TITLE)} "
                        em { "{locale.t(keys::INNER_SECTION_RUNTIME_EM)}" }
                    }
                    div { class: "w2-scroll",
                        div { class: "row active", span { class: "dot-radio" }, "Claude 3.7 · 主人格" }
                        div { class: "row", span { class: "dot-radio" }, "route.search: Haiku" }
                        div { class: "row active", span { class: "dot-radio" }, "还宽，慢慢来" }
                        div { class: "seg-bar",
                            div { class: "seg on" }
                            div { class: "seg on" }
                            div { class: "seg" }
                            div { class: "seg" }
                            div { class: "seg" }
                        }
                        div { class: "row w2-token",
                            span { class: "w2-token-label", "{locale.t(keys::INNER_RUNTIME_TOKEN_USAGE)}" }
                            span { class: "w2-token-value", "{token_text}" }
                            button {
                                class: "w2-token-clear",
                                disabled: token_used() == 0,
                                onclick: move |_| { token_used.set(0); },
                                "{locale.t(keys::INNER_RUNTIME_TOKEN_CLEAR)}"
                            }
                        }
                    }
                    button {
                        class: "sys-config w2-foot",
                        onclick: {
                            let mgr = manager.clone();
                            let rx = rx_arc.clone();
                            let theme_rx = theme_rx_for_settings.clone();
                            move |e| {
                                e.stop_propagation();
                                crate::ui_dioxus::app::spawn_module_window_with_theme_rx("settings", &mgr, &rx, theme_rx.clone());
                            }
                        },
                        "≡ {locale.t(keys::INNER_GLOBAL_SETTINGS)}"
                    }
                    button {
                        class: "sys-config w2-foot",
                        onclick: {
                            let mgr_mem = manager.clone();
                            let rx_mem = rx_arc.clone();
                            let theme_mem = theme_rx_for_settings.clone();
                            move |e| {
                                e.stop_propagation();
                                crate::ui_dioxus::app::spawn_module_window_with_theme_rx("memory", &mgr_mem, &rx_mem, theme_mem.clone());
                            }
                        },
                        "记忆浏览"
                    }
                }
                div { class: "mod w2c-axioms",
                    div { class: "side-title w2-pin",
                        "{locale.t(keys::INNER_SECTION_AXIOMS_TITLE)} "
                        em { "{locale.t(keys::INNER_SECTION_AXIOMS_EM)}" }
                    }
                    div { class: "w2-scroll",
                        div { class: "row active", span { class: "sq-toggle" }, "维护主体边界" }
                        div { class: "row", span { class: "sq-toggle" }, "隐喻性修辞" }
                    }
                }
            }
        }
    }
}
