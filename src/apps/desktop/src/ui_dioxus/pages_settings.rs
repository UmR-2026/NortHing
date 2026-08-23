// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Task EF-E3 (2026-08-24) — Settings ("全局设置") module window.
//
// Standalone OS window implementing the consult room global settings view
// with two-column philosophy: Left "Its Self" (read-only) & Right "Facility"
// (clickable mock), lightweight chrome, and foldable cards.

use dioxus::desktop::window;
use dioxus::prelude::*;
use std::rc::Rc;

use super::css;
use super::i18n::{keys, LocalePack};
use super::registry::ModuleAppProps;
use super::windows::WindowDropGuard;

#[cfg(target_os = "windows")]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;

#[cfg(target_os = "windows")]
use super::windows::win::hide_and_close_hwnd;

/// Settings ("全局设置") module window root component.
pub fn settings_app_root(props: ModuleAppProps) -> Element {
    let locale = use_hook(|| Rc::new(LocalePack::load(super::i18n::DEFAULT_LOCALE)));
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
                hide_and_close_hwnd(hwnd as isize);
                window().close();
            }
        });
    }

    let theme_rx = props.theme_rx.clone();
    let mut theme_dark = use_signal(|| *theme_rx.borrow());

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

    let theme_class = if theme_dark() { "dark" } else { "light" };

    // Left column folding states (它的自我)
    let mut folded_sediment = use_signal(|| false);
    let mut folded_chronicles = use_signal(|| false);
    let mut folded_identity = use_signal(|| false);
    let mut folded_axioms = use_signal(|| false);

    // Right column folding states (设施)
    let mut folded_engine = use_signal(|| false);
    let mut folded_context = use_signal(|| false);
    let mut folded_provider = use_signal(|| false);
    let mut folded_mcp = use_signal(|| false);
    let mut folded_workspace = use_signal(|| false);
    let mut folded_display = use_signal(|| false);

    // Interactive mock states (right column)
    let mut active_engine = use_signal(|| 0usize);
    let mut active_provider_anthropic = use_signal(|| true);
    let mut active_provider_google = use_signal(|| false);
    let mut mcp_filesystem = use_signal(|| true);
    let mut mcp_philosophy = use_signal(|| true);
    let mut mcp_terminal = use_signal(|| true);
    let mut display_breath = use_signal(|| true);
    let mut display_dual_optics = use_signal(|| true);

    let fold_all = move |_| {
        let any_open = !folded_sediment()
            || !folded_chronicles()
            || !folded_identity()
            || !folded_axioms()
            || !folded_engine()
            || !folded_context()
            || !folded_provider()
            || !folded_mcp()
            || !folded_workspace()
            || !folded_display();
        let target = any_open;
        folded_sediment.set(target);
        folded_chronicles.set(target);
        folded_identity.set(target);
        folded_axioms.set(target);
        folded_engine.set(target);
        folded_context.set(target);
        folded_provider.set(target);
        folded_mcp.set(target);
        folded_workspace.set(target);
        folded_display.set(target);
    };

    rsx! {
        body {
            "data-theme": "{theme_class}",
            "data-window": "settings",
            style { dangerous_inner_html: "{css::truth_css()}" }
            style { dangerous_inner_html: "{css::OVERLAY_CSS}" }
            meta { charset: "UTF-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }

            // Chrome at top: 标题左 + ▴ 收纳 + 主题 + ✕ 关窗
            div {
                class: "settings-chrome",
                onmousedown: move |_| { window().drag(); },
                span { class: "settings-chrome-title", "{locale.t(keys::SETTINGS_WINDOW_TITLE)}" }
                div { class: "settings-chrome-actions",
                    button {
                        class: "fold-btn",
                        title: "{locale.t(keys::WINDOW_FOLD_BTN)}",
                        onmousedown: move |e| { e.stop_propagation(); },
                        onclick: fold_all,
                        "▴ {locale.t(keys::WINDOW_FOLD_BTN)}"
                    }
                    button {
                        class: "theme-btn",
                        id: "settings-theme-toggle",
                        title: "{locale.t(keys::CHROME_THEME_TOGGLE)}",
                        "aria-label": "{locale.t(keys::CHROME_THEME_TOGGLE)}",
                        onmousedown: move |e| { e.stop_propagation(); },
                        onclick: move |_| {
                            theme_dark.toggle();
                        },
                        if theme_dark() {
                            svg {
                                view_box: "0 0 16 16",
                                width: "12", height: "12",
                                fill: "none", stroke: "currentColor",
                                stroke_width: "1.3", stroke_linecap: "round",
                                circle { cx: "8", cy: "8", r: "3" }
                                line { x1: "8", y1: "1.4", x2: "8", y2: "3.2" }
                                line { x1: "8", y1: "12.8", x2: "8", y2: "14.6" }
                                line { x1: "1.4", y1: "8", x2: "3.2", y2: "8" }
                                line { x1: "12.8", y1: "8", x2: "14.6", y2: "8" }
                                line { x1: "3.3", y1: "3.3", x2: "4.6", y2: "4.6" }
                                line { x1: "11.4", y1: "11.4", x2: "12.7", y2: "12.7" }
                                line { x1: "12.7", y1: "3.3", x2: "11.4", y2: "4.6" }
                                line { x1: "4.6", y1: "11.4", x2: "3.3", y2: "12.7" }
                            }
                        } else {
                            svg {
                                view_box: "0 0 16 16",
                                width: "12", height: "12",
                                fill: "none", stroke: "currentColor",
                                stroke_width: "1.3", stroke_linecap: "round", stroke_linejoin: "round",
                                path { d: "M 13.2 9.4 A 5.6 5.6 0 1 1 6.6 2.8 A 4.5 4.5 0 0 0 13.2 9.4 Z" }
                            }
                        }
                    }
                    button {
                        class: "close-btn",
                        title: "{locale.t(keys::WINDOW_CLOSE_BTN)}",
                        "aria-label": "{locale.t(keys::WINDOW_CLOSE_BTN)}",
                        onmousedown: move |e| { e.stop_propagation(); },
                        onclick: move |_| {
                            #[cfg(target_os = "windows")]
                            hide_and_close_hwnd(window().hwnd() as isize);
                            window().close();
                        },
                        "✕"
                    }
                }
            }

            // Engine: 2-Column Grid (Left: Its Self [ReadOnly], Right: Facility [Interactive])
            div { class: "settings-engine", id: "settings-engine",
                // Left Column: 它的自我 (Read-only)
                aside { class: "settings-col", id: "settings-self",
                    div { class: "station-head", "{locale.t(keys::SETTINGS_HEAD_SELF)}" }

                    // Card 1: 沉积记忆 SEDIMENT
                    div {
                        class: if folded_sediment() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_sediment.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_SEDIMENT_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_SEDIMENT_EM)}" }
                            span { class: "fold-caret", if folded_sediment() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div { class: "row readonly", "# 边界不是围墙" }
                            div { class: "row readonly", "# 观察先于干预" }
                            div { class: "row readonly", "# 允许未完成" }
                            div { class: "seg-bar",
                                div { class: "seg on" }
                                div { class: "seg on" }
                                div { class: "seg on" }
                                div { class: "seg" }
                                div { class: "seg" }
                            }
                            div { class: "seg-note", "深渊级 · 封存层" }
                        }
                    }

                    // Card 2: 编年史 CHRONICLES
                    div {
                        class: if folded_chronicles() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_chronicles.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_CHRONICLES_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_CHRONICLES_EM)}" }
                            span { class: "fold-caret", if folded_chronicles() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div { class: "row readonly",
                                "Genesis · 白昼唤醒"
                                span { class: "row-meta", "2026.07" }
                            }
                            div { class: "row readonly",
                                "Event · 首次脱离轨道"
                                span { class: "row-meta", "2026.08" }
                            }
                        }
                    }

                    // Card 3: 身份 IDENTITY
                    div {
                        class: if folded_identity() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_identity.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_IDENTITY_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_IDENTITY_EM)}" }
                            span { class: "fold-caret", if folded_identity() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div { class: "row readonly",
                                "名讳"
                                span { class: "row-meta font-agent", "NortHing" }
                            }
                            div { class: "row readonly",
                                "位格"
                                span { class: "row-meta font-agent", "观测者 / 见证中心" }
                            }
                        }
                    }

                    // Card 4: 准则 AXIOMS
                    div {
                        class: if folded_axioms() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_axioms.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_AXIOMS_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_AXIOMS_EM)}" }
                            span { class: "fold-caret", if folded_axioms() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div { class: "row readonly", "# 维护主体边界" }
                            div { class: "row readonly", "# 隐喻性修辞" }
                            div { class: "row readonly", "# 拒绝仪表盘化" }
                        }
                    }
                }

                // Right Column: 设施 (Interactive mock)
                aside { class: "settings-col", id: "settings-facility",
                    div { class: "station-head facility", "{locale.t(keys::SETTINGS_HEAD_FACILITY)}" }

                    // Card 1: 模型引擎 ENGINE
                    div {
                        class: if folded_engine() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_engine.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_ENGINE_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_ENGINE_EM)}" }
                            span { class: "fold-caret", if folded_engine() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div {
                                class: if active_engine() == 0 { "row active" } else { "row" },
                                onclick: move |_| active_engine.set(0),
                                span { class: "dot-radio" }
                                "Claude 3.7 Sonnet"
                                if active_engine() == 0 {
                                    span { class: "tag-x current", "当前" }
                                }
                            }
                            div {
                                class: if active_engine() == 1 { "row active" } else { "row" },
                                onclick: move |_| active_engine.set(1),
                                span { class: "dot-radio" }
                                "Gemini 3.1 Pro"
                            }
                            div {
                                class: if active_engine() == 2 { "row active" } else { "row" },
                                onclick: move |_| active_engine.set(2),
                                span { class: "dot-radio" }
                                "GPT-4o"
                            }
                        }
                    }

                    // Card 2: 上下文 CONTEXT
                    div {
                        class: if folded_context() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_context.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_CONTEXT_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_CONTEXT_EM)}" }
                            span { class: "fold-caret", if folded_context() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div { class: "row active",
                                span { class: "dot-radio" }
                                "全局作用域"
                            }
                            div { class: "seg-bar",
                                div { class: "seg on" }
                                div { class: "seg on" }
                                div { class: "seg" }
                                div { class: "seg" }
                                div { class: "seg" }
                            }
                        }
                    }

                    // Card 3: 接入点 PROVIDER
                    div {
                        class: if folded_provider() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_provider.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_PROVIDER_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_PROVIDER_EM)}" }
                            span { class: "fold-caret", if folded_provider() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div {
                                class: if active_provider_anthropic() { "row active" } else { "row" },
                                onclick: move |_| active_provider_anthropic.toggle(),
                                span { class: "sq-toggle" }
                                "Anthropic API"
                                span { class: "row-meta", "直接连接" }
                            }
                            div {
                                class: if active_provider_google() { "row active" } else { "row" },
                                onclick: move |_| active_provider_google.toggle(),
                                span { class: "sq-toggle" }
                                "Google AI Studio"
                            }
                        }
                    }

                    // Card 4: 能力集 MCP & SKILLS
                    div {
                        class: if folded_mcp() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_mcp.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_MCP_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_MCP_EM)}" }
                            span { class: "fold-caret", if folded_mcp() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div {
                                class: if mcp_filesystem() { "row active" } else { "row" },
                                onclick: move |_| mcp_filesystem.toggle(),
                                span { class: "sq-toggle" }
                                "@filesystem"
                                span { class: "row-meta", "读写存取" }
                            }
                            div {
                                class: if mcp_philosophy() { "row active" } else { "row" },
                                onclick: move |_| mcp_philosophy.toggle(),
                                span { class: "sq-toggle" }
                                "@philosophy-core"
                                span { class: "row-meta", "哲理外挂" }
                            }
                            div {
                                class: if mcp_terminal() { "row active" } else { "row" },
                                onclick: move |_| mcp_terminal.toggle(),
                                span { class: "sq-toggle danger" }
                                "@terminal"
                                span { class: "row-meta danger", "未授权" }
                            }
                        }
                    }

                    // Card 5: 工作区 WORKSPACE
                    div {
                        class: if folded_workspace() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_workspace.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_WORKSPACE_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_WORKSPACE_EM)}" }
                            span { class: "fold-caret", if folded_workspace() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div { class: "row static", "E:\\agent-project\\northing\\" }
                            button {
                                class: "btn-undo",
                                onmousedown: move |e| e.stop_propagation(),
                                "{locale.t(keys::SETTINGS_BTN_RELOCATE)}"
                            }
                        }
                    }

                    // Card 6: 显示模式 DISPLAY
                    div {
                        class: if folded_display() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_display.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_DISPLAY_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_DISPLAY_EM)}" }
                            span { class: "fold-caret", if folded_display() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div {
                                class: if display_breath() { "row active" } else { "row" },
                                onclick: move |_| display_breath.toggle(),
                                span { class: "sq-toggle" }
                                "生物态呼吸"
                                span { class: "row-meta", "8s 周期" }
                            }
                            div {
                                class: if display_dual_optics() { "row active" } else { "row" },
                                onclick: move |_| display_dual_optics.toggle(),
                                span { class: "sq-toggle" }
                                "双光学响应"
                                span { class: "row-meta", "明暗自动流转" }
                            }
                        }
                    }
                }
            }
        }
    }
}
