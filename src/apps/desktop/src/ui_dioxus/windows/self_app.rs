// SPDX-License-Identifier: MIT OR Apache-2.0
//
// W10-2 split — Self ("它的自我") module window root component.

use crate::ui_dioxus::css;
use crate::ui_dioxus::entry::DOCK_GAP_PX;
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

/// Self ("它的自我") module window root component.
pub fn self_app_root(props: ModuleAppProps) -> Element {
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
                .name("self-geometry-follow".into())
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
                        let off_x = ((280.0 + DOCK_GAP_PX as f64) * scale) as i32;
                        let target_x = cur.x.saturating_sub(off_x);
                        let target_y = cur.y;
                        let target_w = (280.0 * scale) as i32;
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
                .expect("spawn self geometry follow thread");
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
    // W2.7（2026-08-23）：左列单扇满高。五卡同窗——沉积三卡 + 运行/准则
    // （原 facility 半高窗并入）。chrome 文案走 GEM_LEFT_TITLE（沉积与设施）。
    let mut token_used = use_signal(|| 128_437u64);
    let token_text = fmt_tokens(token_used());

    let mut folded_sediment = use_signal(|| false);
    let mut folded_rag = use_signal(|| false);
    let mut folded_skill = use_signal(|| false);
    let mut folded_runtime = use_signal(|| false);
    let mut folded_axioms = use_signal(|| false);

    let fold_all = move |_| {
        let any_open = !folded_sediment() || !folded_rag() || !folded_skill() || !folded_runtime() || !folded_axioms();
        let target = any_open;
        folded_sediment.set(target);
        folded_rag.set(target);
        folded_skill.set(target);
        folded_runtime.set(target);
        folded_axioms.set(target);
    };

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
                    class: "station-head w2-head",
                    "data-drag": "true",
                    onmousedown: move |_| { window().drag(); },
                    "{locale.t(keys::GEM_LEFT_TITLE)}",
                    button {
                        class: "fold-btn",
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
                    class: if folded_sediment() { "mod w2c-sediment is-folded" } else { "mod w2c-sediment" },
                    div {
                        class: "side-title w2-pin",
                        onclick: move |_| { folded_sediment.toggle(); },
                        "{locale.t(keys::INNER_SECTION_SEDIMENT_TITLE)} "
                        em { "{locale.t(keys::INNER_SECTION_SEDIMENT_EM)}" }
                        span { class: "fold-caret", if folded_sediment() { "▸" } else { "▾" } }
                    }
                    div { class: "w2-scroll",
                        div { class: "row", "# 边界不是围墙" button { class: "tag-x", "×" } }
                        div { class: "row", "# 观察先于干预" button { class: "tag-x", "×" } }
                        div { class: "row", "# 允许未完成" button { class: "tag-x", "×" } }
                    }
                    div { class: "w2-foot",
                        div { class: "seg-bar", div { class: "seg on" } div { class: "seg on" } div { class: "seg on" } div { class: "seg" } div { class: "seg" } }
                        div { class: "seg-note", "{locale.t(keys::INNER_SECTION_SEDIMENT_NOTE)}" }
                    }
                }
                div {
                    class: if folded_rag() { "mod w2c-rag is-folded" } else { "mod w2c-rag" },
                    div {
                        class: "side-title w2-pin",
                        onclick: move |_| { folded_rag.toggle(); },
                        "{locale.t(keys::INNER_SECTION_RAG_TITLE)} "
                        em { "{locale.t(keys::INNER_SECTION_RAG_EM)}" }
                        span { class: "fold-caret", if folded_rag() { "▸" } else { "▾" } }
                    }
                    div { class: "w2-scroll",
                        div { class: "row active",
                            "@philosophy-core "
                            span { class: "tag-x", style: "color:var(--mind-line);cursor:default", "{locale.t(keys::INNER_RAG_MOUNTED)}" }
                        }
                    }
                }
                div {
                    class: if folded_skill() { "mod w2c-skill is-folded" } else { "mod w2c-skill" },
                    div {
                        class: "side-title w2-pin",
                        onclick: move |_| { folded_skill.toggle(); },
                        "{locale.t(keys::INNER_SECTION_SKILL_TITLE)} "
                        em { "{locale.t(keys::INNER_SECTION_SKILL_EM)}" }
                        span { class: "fold-caret", if folded_skill() { "▸" } else { "▾" } }
                    }
                    div { class: "w2-scroll",
                        div { class: "row", "{locale.t(keys::INNER_SKILL_CAND_1)}" span { class: "w2-stat", "{locale.t(keys::INNER_SKILL_STAT_SHAPE)}" } }
                        div { class: "row", "{locale.t(keys::INNER_SKILL_CAND_2)}" span { class: "w2-stat", "{locale.t(keys::INNER_SKILL_STAT_SHAPE)}" } }
                        div { class: "row", "{locale.t(keys::INNER_SKILL_CAND_3)}" span { class: "w2-stat", "{locale.t(keys::INNER_SKILL_STAT_WATCH)}" } }
                    }
                }
                div {
                    class: "w2-group-seam",
                    span { class: "w2-group-label", "{locale.t(keys::INNER_HEAD_FACILITY_TITLE)}" }
                }
                div {
                    class: if folded_runtime() { "mod w2c-runtime is-folded" } else { "mod w2c-runtime" },
                    div {
                        class: "side-title w2-pin",
                        onclick: move |_| { folded_runtime.toggle(); },
                        "{locale.t(keys::INNER_SECTION_RUNTIME_TITLE)} "
                        em { "{locale.t(keys::INNER_SECTION_RUNTIME_EM)}" }
                        span { class: "fold-caret", if folded_runtime() { "▸" } else { "▾" } }
                    }
                    div { class: "w2-scroll",
                        div { class: "row active", span { class: "dot-radio" }, "Claude 3.7 · 主人格" }
                        div { class: "row", span { class: "dot-radio" }, "route.search: Haiku" }
                        div { class: "row active", span { class: "dot-radio" }, "还宽，慢慢来" }
                        div { class: "seg-bar", div { class: "seg on" } div { class: "seg on" } div { class: "seg" } div { class: "seg" } div { class: "seg" } }
                        div { class: "row w2-token",
                            span { class: "w2-token-label", "{locale.t(keys::INNER_RUNTIME_TOKEN_USAGE)}" }
                            span { class: "w2-token-value", "{token_text}" }
                            button {
                                class: "w2-token-clear",
                                disabled: token_used() == 0,
                                onclick: move |e| { e.stop_propagation(); token_used.set(0); },
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
                }
                div {
                    class: if folded_axioms() { "mod w2c-axioms is-folded" } else { "mod w2c-axioms" },
                    div {
                        class: "side-title w2-pin",
                        onclick: move |_| { folded_axioms.toggle(); },
                        "{locale.t(keys::INNER_SECTION_AXIOMS_TITLE)} "
                        em { "{locale.t(keys::INNER_SECTION_AXIOMS_EM)}" }
                        span { class: "fold-caret", if folded_axioms() { "▸" } else { "▾" } }
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
