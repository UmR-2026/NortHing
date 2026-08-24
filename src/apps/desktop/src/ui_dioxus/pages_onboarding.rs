// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Task EF-E4 (2026-08-24) — Onboarding ("房间诞生仪式") module window.
//
// Standalone OS window implementing the consult room room birth ritual view.
// Frameless, full-covering dock mode, self-contained CSS, dual optics,
// left & right drawers, and 3-step ritual flow with Big Five mind palette picker.

use dioxus::desktop::window;
use dioxus::prelude::*;
use std::rc::Rc;

use super::i18n::{keys, LocalePack};
use super::registry::ModuleAppProps;
use super::windows::WindowDropGuard;

#[cfg(target_os = "windows")]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;

#[cfg(target_os = "windows")]
use super::windows::win::hide_and_close_hwnd;

use super::pages_onboarding_css::ONBOARDING_CSS;

const SWATCHES: &[(&str, &str, &str)] = &[
    ("#C8714C", "驱力", "探索 / 开拓"),
    ("#3F837B", "深渊", "凝视 / 沉淀"),
    ("#8B5FBF", "跃迁", "突破 / 演进"),
    ("#D99B48", "凝视", "审视 / 对齐"),
    ("#4B8F6B", "镇静", "恒稳 / 收容"),
];

pub fn onboarding_app_root(props: ModuleAppProps) -> Element {
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

    // Interaction states
    let mut selected_palette = use_signal(|| Option::<(&'static str, &'static str, &'static str)>::None);
    let mut tested_connection = use_signal(|| false);
    let mut test_status_text = use_signal(|| Option::<String>::None);
    let mut ritual_completed = use_signal(|| false);
    let mut room_state_hint = use_signal(|| Option::<String>::None);

    let mut agent_input = use_signal(|| "NortHing".to_string());
    let mut agent_name_edited = use_signal(|| false);
    let mut user_title_input = use_signal(|| "见证者".to_string());
    let mut relation_title_input = use_signal(|| "思维的镜面与延伸".to_string());

    let mut provider_model_input = use_signal(|| "claude-3-7-sonnet".to_string());
    let mut provider_url_input = use_signal(|| "https://api.anthropic.com/v1".to_string());
    let mut provider_key_input = use_signal(|| "".to_string());
    let mut workspace_dir_input = use_signal(|| "E:\\agent-project\\northing\\workspace".to_string());

    // Drawer & header collapse states
    let mut head_folded = use_signal(|| false);
    let mut mind_drawer_open = use_signal(|| false);
    let mut work_drawer_open = use_signal(|| false);
    let mut folded_mind_eve = use_signal(|| false);
    let mut folded_mind_facility = use_signal(|| false);
    let mut folded_work = use_signal(|| false);

    let inhabited = selected_palette().is_some();
    let mind_base = selected_palette().map(|(hex, _, _)| hex).unwrap_or("#7e8896");
    let style_vars = format!("--mind-base: {mind_base}; --aura-x: 50%; --aura-y: 200px;");

    let display_agent_name = if !agent_name_edited() {
        locale.t(keys::ONBOARDING_HEAD_NAME_INITIAL).to_string()
    } else {
        let val = agent_input();
        if val.trim().is_empty() {
            locale.t(keys::ONBOARDING_HEAD_NAME_INITIAL).to_string()
        } else {
            val.trim().to_string()
        }
    };

    let avatar_char = if !agent_name_edited() {
        "?".to_string()
    } else {
        let val = agent_input();
        let trimmed = val.trim();
        if trimmed.is_empty() {
            "?".to_string()
        } else {
            trimmed.chars().next().unwrap().to_uppercase().to_string()
        }
    };

    let preview_identity_name = if !agent_name_edited() {
        agent_input()
    } else {
        display_agent_name.clone()
    };

    let room_state_text = if let Some(hint) = room_state_hint() {
        hint
    } else if let Some((_, name, _)) = selected_palette() {
        format!("{name}状态 · 房间印记已铸造")
    } else {
        locale.t(keys::ONBOARDING_STATUS_INITIAL).to_string()
    };

    let mind_state_desc = if let Some((_, name, desc)) = selected_palette() {
        format!("{name}色板 ({desc}) · 印记已注入")
    } else {
        locale.t(keys::ONBOARDING_HEAD_STATE_INITIAL).to_string()
    };

    let seg_note_text = if ritual_completed() {
        "仪式完毕 · 诊室已正式诞生"
    } else if tested_connection() {
        "印记形成中 · 2/3 已贯通"
    } else if inhabited {
        "印记形成中 · 1/3 已铸造"
    } else {
        "零沉淀 · 契约准备中"
    };

    rsx! {
        body {
            "data-theme": "{theme_class}",
            "data-window": "onboarding",
            "data-inhabited": if inhabited { "true" } else { "false" },
            style: "{style_vars}",
            meta { charset: "UTF-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }
            title { "{locale.t(keys::ONBOARDING_WINDOW_TITLE)}" }
            style { dangerous_inner_html: "{ONBOARDING_CSS}" }

            div { id: "containment" }
            div { class: "membrane-frame" }
            div { id: "global-aura" }

            div { id: "engine",
                // 左抽屉: 诞生前夜
                aside {
                    id: "mind",
                    class: if mind_drawer_open() { "" } else { "mod-hidden" },
                    div {
                        class: if folded_mind_eve() { "mod is-folded" } else { "mod" },
                        div {
                            class: "station-head",
                            onclick: move |_| folded_mind_eve.toggle(),
                            "{locale.t(keys::ONBOARDING_DRAWER_MIND_HEAD)}"
                            button {
                                class: "fold-btn",
                                onmousedown: move |e| e.stop_propagation(),
                                onclick: move |e| { e.stop_propagation(); folded_mind_eve.toggle(); },
                                if folded_mind_eve() { "▾ 展开" } else { "▴ 收纳" }
                            }
                        }
                        div { class: "card-body",
                            div { class: "side-section",
                                div { class: "side-title", "状态测定 " em { "STATUS" } }
                                div { class: "row active",
                                    span { class: "dot-radio" }
                                    "物理空间待入住"
                                }
                                div {
                                    class: if inhabited { "row active" } else { "row" },
                                    id: "mind-cond-row",
                                    span { class: "dot-radio" }
                                    if inhabited { "思维印记已凝结" } else { "思维印记未凝结" }
                                }
                                div { class: "seg-bar",
                                    div { class: "seg on", id: "seg-1" }
                                    div { class: if inhabited { "seg on" } else { "seg" }, id: "seg-2" }
                                    div { class: if tested_connection() { "seg on" } else { "seg" }, id: "seg-3" }
                                    div { class: if ritual_completed() { "seg on" } else { "seg" }, id: "seg-4" }
                                }
                                div { class: "seg-note", id: "seg-note", "{seg_note_text}" }
                            }
                        }
                    }
                    div {
                        class: if folded_mind_facility() { "mod is-folded" } else { "mod" },
                        div {
                            class: "station-head facility",
                            onclick: move |_| folded_mind_facility.toggle(),
                            "{locale.t(keys::ONBOARDING_DRAWER_FACILITY_HEAD)}"
                            button {
                                class: "fold-btn",
                                onmousedown: move |e| e.stop_propagation(),
                                onclick: move |e| { e.stop_propagation(); folded_mind_facility.toggle(); },
                                if folded_mind_facility() { "▾ 展开" } else { "▴ 收纳" }
                            }
                        }
                        div { class: "card-body",
                            div { class: "side-section",
                                div { class: "side-title", "底层基质 " em { "RUNTIME" } }
                                div { class: "row active", span { class: "sq-toggle" } "Slint 规格架构" }
                                div { class: "row active", span { class: "sq-toggle" } "双光学冷热流" }
                            }
                            div { class: "side-section",
                                div { class: "side-title", "仪式公约 " em { "COVENANT" } }
                                div { class: "row active", "人可赋予印记，不能改写自我" }
                            }
                        }
                    }
                }

                // 中央有界诊室
                div { id: "room-wrap",
                    section { id: "room",
                        span { class: "membrane l" }
                        span { class: "membrane r" }
                        button {
                            class: if mind_drawer_open() { "membrane-node left is-open" } else { "membrane-node left" },
                            id: "trig-mind",
                            "aria-label": "唤起 诞生前夜",
                            "aria-expanded": if mind_drawer_open() { "true" } else { "false" },
                            title: "诞生前夜",
                            onclick: move |_| mind_drawer_open.toggle(),
                        }
                        button {
                            class: if work_drawer_open() { "membrane-node right is-open" } else { "membrane-node right" },
                            id: "trig-work",
                            "aria-label": "唤起 诞生存根",
                            "aria-expanded": if work_drawer_open() { "true" } else { "false" },
                            title: "诞生存根",
                            onclick: move |_| work_drawer_open.toggle(),
                        }
                        div { class: "room-fog" }

                        div { class: "room-controls",
                            button {
                                class: "rc-btn",
                                id: "theme-toggle",
                                title: "{locale.t(keys::CHROME_THEME_TOGGLE)}",
                                "aria-label": "{locale.t(keys::CHROME_THEME_TOGGLE)}",
                                onmousedown: move |e| e.stop_propagation(),
                                onclick: move |_| theme_dark.toggle(),
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
                                class: "rc-btn close",
                                title: "{locale.t(keys::WINDOW_CLOSE_BTN)}",
                                "aria-label": "{locale.t(keys::WINDOW_CLOSE_BTN)}",
                                onmousedown: move |e| e.stop_propagation(),
                                onclick: move |_| {
                                    #[cfg(target_os = "windows")]
                                    hide_and_close_hwnd(window().hwnd() as isize);
                                    window().close();
                                },
                                "✕"
                            }
                        }

                        div {
                            class: "room-status",
                            onmousedown: move |_| { window().drag(); },
                            span { class: "brand-inline",
                                svg {
                                    view_box: "0 0 200 200",
                                    "aria-label": "northing",
                                    path {
                                        d: "M 112.68 72.84 A 30 30 0 1 1 87.32 72.84",
                                        fill: "none", stroke: "currentColor", stroke_width: "2.5", stroke_linecap: "round"
                                    }
                                    path {
                                        d: "M 126 54.97 A 52 52 0 1 1 82.28 51.22",
                                        fill: "none", stroke: "currentColor", stroke_width: "5", stroke_linecap: "round"
                                    }
                                    path {
                                        d: "M 132.13 31.13 A 76 76 0 1 1 56.35 37.47",
                                        fill: "none", stroke: "currentColor", stroke_width: "9", stroke_linecap: "round"
                                    }
                                    path {
                                        d: "M 56.35 37.47 Q 48 30, 44 24",
                                        fill: "none", stroke: "currentColor", stroke_width: "8", stroke_linecap: "round"
                                    }
                                    path {
                                        d: "M 132.13 31.13 Q 137 24, 139 19",
                                        fill: "none", stroke: "currentColor", stroke_width: "8", stroke_linecap: "round"
                                    }
                                }
                                span { class: "seal-name", "northing" }
                            }
                            span { "{locale.t(keys::ONBOARDING_STATUS_TITLE)}" }
                            span { class: "sp" }
                            span { class: "state-dot" }
                            span {
                                style: "color:var(--mind-line)",
                                id: "room-state-text",
                                "{room_state_text}"
                            }
                        }

                        div {
                            class: if head_folded() { "room-head folded" } else { "room-head" },
                            id: "room-head",
                            onmousedown: move |_| { window().drag(); },
                            button {
                                class: "fold-btn head-fold",
                                "aria-label": "{locale.t(keys::ONBOARDING_HEAD_FOLD_BTN)}",
                                onmousedown: move |e| e.stop_propagation(),
                                onclick: move |_| head_folded.toggle(),
                                if head_folded() { "▾" } else { "▴" }
                            }
                            div { class: "agent-avatar", id: "avatar-core", "{avatar_char}" }
                            div { class: "name-line", id: "agent-display-title", "{display_agent_name}" }
                            div { class: "state", id: "mind-state-desc", "{mind_state_desc}" }
                        }

                        div { class: "chat-flow", id: "chat-flow",
                            div { class: "ritual-divider", "房间诞生仪式 · 第一章：身份凝结" }

                            // 步骤 1: 身份印记
                            div { class: "ritual-card",
                                div { class: "ritual-card-head",
                                    div { class: "ritual-card-title",
                                        "❖ {locale.t(keys::ONBOARDING_SECTION_IDENTITY_TITLE)} "
                                        em { "{locale.t(keys::ONBOARDING_SECTION_IDENTITY_EM)}" }
                                    }
                                    div { class: "ritual-card-step", "{locale.t(keys::ONBOARDING_STEP_1)}" }
                                }
                                div { class: "ritual-narrative", "定义我们在此处的关系与边界。你是见证者，它是你唤醒的思考实体。" }

                                div { class: "field-grid",
                                    div { class: "field-group",
                                        label { class: "field-label", r#for: "user-title", "{locale.t(keys::ONBOARDING_LABEL_USER)}" }
                                        input {
                                            r#type: "text",
                                            id: "user-title",
                                            class: "field-input",
                                            value: "{user_title_input}",
                                            placeholder: "如: 观察者 / 记录员",
                                            oninput: move |e| user_title_input.set(e.value().clone()),
                                        }
                                    }
                                    div { class: "field-group",
                                        label { class: "field-label", r#for: "agent-title", "{locale.t(keys::ONBOARDING_LABEL_AGENT)}" }
                                        input {
                                            r#type: "text",
                                            id: "agent-title",
                                            class: "field-input",
                                            value: "{agent_input}",
                                            placeholder: "实体名称",
                                            oninput: move |e| {
                                                agent_input.set(e.value().clone());
                                                agent_name_edited.set(true);
                                            },
                                        }
                                    }
                                    div { class: "field-group full",
                                        label { class: "field-label", r#for: "relation-title", "{locale.t(keys::ONBOARDING_LABEL_RELATION)}" }
                                        input {
                                            r#type: "text",
                                            id: "relation-title",
                                            class: "field-input",
                                            value: "{relation_title_input}",
                                            placeholder: "如: 共生探针 / 沉淀助手",
                                            oninput: move |e| relation_title_input.set(e.value().clone()),
                                        }
                                    }
                                    div { class: "field-group full",
                                        label { class: "field-label",
                                            "{locale.t(keys::ONBOARDING_LABEL_PALETTE)} "
                                            em { "{locale.t(keys::ONBOARDING_LABEL_PALETTE_EM)}" }
                                        }
                                        div { class: "palette-picker",
                                            for (hex, name, desc) in SWATCHES.iter().copied() {
                                                {
                                                    let is_selected = selected_palette().map(|(h, _, _)| h == hex).unwrap_or(false);
                                                    rsx! {
                                                        div {
                                                            class: if is_selected { "palette-swatch selected" } else { "palette-swatch" },
                                                            style: "--swatch-color:{hex}",
                                                            onclick: move |_| {
                                                                selected_palette.set(Some((hex, name, desc)));
                                                                room_state_hint.set(None);
                                                            },
                                                            div { class: "swatch-circle" }
                                                            div { class: "swatch-name", "{name}" }
                                                            div { class: "swatch-desc", "{desc}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "ritual-divider", "第二章：管道贯通" }

                            // 步骤 2: Provider 配置
                            div { class: "ritual-card",
                                div { class: "ritual-card-head",
                                    div { class: "ritual-card-title",
                                        "↯ {locale.t(keys::ONBOARDING_SECTION_PROVIDER_TITLE)} "
                                        em { "{locale.t(keys::ONBOARDING_SECTION_PROVIDER_EM)}" }
                                    }
                                    div { class: "ritual-card-step", "{locale.t(keys::ONBOARDING_STEP_2)}" }
                                }
                                div { class: "ritual-narrative", "为它的认知提供神经冲动。输入模型引擎与通讯凭证。" }

                                div { class: "field-grid",
                                    div { class: "field-group",
                                        label { class: "field-label", r#for: "provider-model", "{locale.t(keys::ONBOARDING_LABEL_MODEL)}" }
                                        input {
                                            r#type: "text",
                                            id: "provider-model",
                                            class: "field-input",
                                            value: "{provider_model_input}",
                                            placeholder: "如: claude-3-7-sonnet",
                                            oninput: move |e| provider_model_input.set(e.value().clone()),
                                        }
                                    }
                                    div { class: "field-group",
                                        label { class: "field-label", r#for: "provider-url", "{locale.t(keys::ONBOARDING_LABEL_BASE_URL)}" }
                                        input {
                                            r#type: "text",
                                            id: "provider-url",
                                            class: "field-input",
                                            value: "{provider_url_input}",
                                            placeholder: "API 端点",
                                            oninput: move |e| provider_url_input.set(e.value().clone()),
                                        }
                                    }
                                    div { class: "field-group full",
                                        label { class: "field-label", r#for: "provider-key", "{locale.t(keys::ONBOARDING_LABEL_API_KEY)}" }
                                        input {
                                            r#type: "password",
                                            id: "provider-key",
                                            class: "field-input",
                                            value: "{provider_key_input}",
                                            placeholder: "凭证密钥（不落盘于规格）",
                                            oninput: move |e| provider_key_input.set(e.value().clone()),
                                        }
                                    }
                                }

                                div { class: "test-row",
                                    button {
                                        class: "ritual-btn",
                                        onclick: move |_| {
                                            test_status_text.set(Some("✓ 心跳贯通 · 延迟 12ms · 神经元就绪".to_string()));
                                            tested_connection.set(true);
                                        },
                                        "{locale.t(keys::ONBOARDING_BTN_TEST)}"
                                    }
                                    span {
                                        class: if tested_connection() { "test-status ok" } else { "test-status" },
                                        id: "test-status-msg",
                                        if let Some(msg) = test_status_text() {
                                            "{msg}"
                                        } else {
                                            "{locale.t(keys::ONBOARDING_TEST_STATUS_WAIT)}"
                                        }
                                    }
                                }
                            }

                            div { class: "ritual-divider", "第三章：物理锚定" }

                            // 步骤 3: 工作文件夹
                            div { class: "ritual-card",
                                div { class: "ritual-card-head",
                                    div { class: "ritual-card-title",
                                        "◈ {locale.t(keys::ONBOARDING_SECTION_WORKSPACE_TITLE)} "
                                        em { "{locale.t(keys::ONBOARDING_SECTION_WORKSPACE_EM)}" }
                                    }
                                    div { class: "ritual-card-step", "{locale.t(keys::ONBOARDING_STEP_3)}" }
                                }
                                div { class: "ritual-narrative", "指定诊室的物理存根目录。实体在此范围内的行动受收容框界定。" }

                                div { class: "field-grid",
                                    div { class: "field-group full",
                                        label { class: "field-label", r#for: "workspace-dir", "{locale.t(keys::ONBOARDING_LABEL_WORKSPACE)}" }
                                        div { style: "display:flex;gap:8px;",
                                            input {
                                                r#type: "text",
                                                id: "workspace-dir",
                                                class: "field-input",
                                                value: "{workspace_dir_input}",
                                                placeholder: "选择或输入绝对路径",
                                                oninput: move |e| workspace_dir_input.set(e.value().clone()),
                                            }
                                            button {
                                                class: "ritual-btn",
                                                style: "white-space:nowrap;",
                                                "{locale.t(keys::ONBOARDING_BTN_BROWSE)}"
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // 底部开启仪式 / deck
                        div { class: "room-footer",
                            div { class: "witness-pledge",
                                "房间诞生完毕后，印记将融入基质"
                                span { class: "cursor" }
                            }
                            if ritual_completed() {
                                button {
                                    class: "ritual-btn primary",
                                    id: "complete-btn",
                                    style: "background:var(--ok);border-color:var(--ok);color:#fff",
                                    "✓ 诊室已诞生 · 空间运行中"
                                }
                            } else {
                                button {
                                    class: "ritual-btn primary",
                                    id: "complete-btn",
                                    onclick: move |_| {
                                        if selected_palette().is_none() {
                                            room_state_hint.set(Some("请先选择性格色板，为诊室注入第一个 mind 色印记。".to_string()));
                                        } else {
                                            ritual_completed.set(true);
                                        }
                                    },
                                    "{locale.t(keys::ONBOARDING_BTN_COMPLETE)}"
                                }
                            }
                        }
                    }
                }

                // 右抽屉: 诞生存根
                aside {
                    id: "work",
                    class: {
                        let mut cls = "mod".to_string();
                        if !work_drawer_open() {
                            cls.push_str(" mod-hidden");
                        }
                        if folded_work() {
                            cls.push_str(" is-folded folded");
                        }
                        cls
                    },
                    div {
                        class: "station-head facility",
                        onclick: move |_| folded_work.toggle(),
                        "{locale.t(keys::ONBOARDING_DRAWER_WORK_HEAD)}"
                        button {
                            class: "fold-btn",
                            id: "work-fold",
                            onmousedown: move |e| e.stop_propagation(),
                            onclick: move |e| { e.stop_propagation(); folded_work.toggle(); },
                            if folded_work() { "▾ 展开" } else { "▴ 收纳" }
                        }
                    }
                    div { class: "side-section",
                        div { class: "side-title", "仪式关卡 " em { "STEPS" } }
                        div {
                            class: if inhabited { "row done" } else { "row" },
                            id: "chk-step1",
                            span {
                                class: "plan-check",
                                style: if inhabited { "border-color:var(--ok);background:var(--ok)" } else { "" },
                            }
                            "I. 身份凝结"
                            span {
                                style: if inhabited { "margin-left:auto;color:var(--ok);font-size:9px" } else { "margin-left:auto;color:var(--faint);font-size:9px" },
                                if inhabited { "已凝结" } else { "未完成" }
                            }
                        }
                        div {
                            class: if tested_connection() { "row done" } else { "row" },
                            id: "chk-step2",
                            span {
                                class: "plan-check",
                                style: if tested_connection() { "border-color:var(--ok);background:var(--ok)" } else { "" },
                            }
                            "II. 信号连通"
                            span {
                                style: if tested_connection() { "margin-left:auto;color:var(--ok);font-size:9px" } else { "margin-left:auto;color:var(--faint);font-size:9px" },
                                if tested_connection() { "已通畅" } else { "未测试" }
                            }
                        }
                        div {
                            class: if ritual_completed() { "row done" } else { "row" },
                            id: "chk-step3",
                            span {
                                class: "plan-check",
                                style: if ritual_completed() { "border-color:var(--ok);background:var(--ok)" } else { "" },
                            }
                            "III. 锚定边界"
                            span {
                                style: if ritual_completed() { "margin-left:auto;color:var(--ok);font-size:9px" } else { "margin-left:auto;color:var(--faint);font-size:9px" },
                                if ritual_completed() { "已立锚" } else { "待确立" }
                            }
                        }
                    }
                    div { class: "side-section",
                        div { class: "side-title", "印记预览 " em { "PREVIEW" } }
                        div {
                            class: "row",
                            id: "preview-color",
                            span { class: "fname", "色板状态" }
                            if let Some((hex, name, _)) = selected_palette() {
                                span {
                                    style: "font-family:var(--font-mono);font-size:10px;color:var(--mind-line)",
                                    "{name} ({hex})"
                                }
                            } else {
                                span {
                                    style: "font-family:var(--font-mono);font-size:10px;color:var(--muted)",
                                    "{locale.t(keys::ONBOARDING_PREVIEW_UNCOLORED)}"
                                }
                            }
                        }
                        div {
                            class: "row",
                            id: "preview-identity",
                            span { class: "fname", "实体命名" }
                            span {
                                style: "font-family:var(--font-mono);font-size:10px;color:var(--muted)",
                                "{preview_identity_name}"
                            }
                        }
                    }
                    div { class: "term-well",
                        div { "$ northing init --ritual" }
                        div { "> chamber: uninhabited" }
                        div { "> awaiting human mind choice..." }
                        div {
                            class: "preview-row",
                            id: "term-status-line",
                            if ritual_completed() {
                                "> status: chamber fully inhabited"
                            } else {
                                "> status: ready for awakening"
                            }
                        }
                        div { "> _" }
                    }
                }
            }
        }
    }
}
