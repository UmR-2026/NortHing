// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Task EF-E2 (2026-08-24) — Space ("走廊") module window.
//
// Standalone OS window implementing the corridor space view
// with lit/dim/sunk door states, 2 foldable sidebars (ORDER/WORKSPACE/DISPLAY & PEEK),
// and lightweight chrome.

use dioxus::desktop::window;
use dioxus::prelude::*;
use std::rc::Rc;

use super::css;
use super::i18n::{keys, LocalePack};
use super::page_shell::{render_close_button, use_page_shell};
use super::registry::ModuleAppProps;

#[cfg(target_os = "windows")]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;

#[cfg(target_os = "windows")]
use super::windows::win::hide_and_close_hwnd;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DoorKind {
    Lit,
    Dim,
    Sunk,
}

#[derive(Clone)]
struct DoorItem {
    id: usize,
    kind: DoorKind,
    sunk_level: usize,
    plate: &'static str,
    topic: &'static str,
    lamp: &'static str,
    state_desc: &'static str,
    sediment: &'static str,
    inside_tags: &'static [&'static str],
    echo: &'static str,
    artifacts: &'static [&'static str],
}

const DOORS: &[DoorItem] = &[
    DoorItem {
        id: 0,
        kind: DoorKind::Lit,
        sunk_level: 0,
        plate: "诊室 03 · 此刻",
        topic: "重新定义对齐",
        lamp: "序",
        state_desc: "低语中 · architect_sub 介入中",
        sediment: "# 边界不是围墙 / # 允许未完成",
        inside_tags: &["低语中", "architect_sub 介入中", "上下文还宽"],
        echo: "门缝里传出来的最后一句：它开始区分你给出的目标，和它选择采取的路径。",
        artifacts: &["alignment-notes.md ↗", "boundary.diff ↗"],
    },
    DoorItem {
        id: 1,
        kind: DoorKind::Dim,
        sunk_level: 0,
        plate: "诊室 02 · 昨夜",
        topic: "边界的物理形状",
        lamp: "◦",
        state_desc: "熄灯 · 昨夜留下未读的授权记录",
        sediment: "# 观察先于干预",
        inside_tags: &["灯已灭", "十余段往来留在里面", "一条授权你还没读"],
        echo: "门缝里只剩残温与一条未读的授权请求。",
        artifacts: &["session-02-auth.log ↗"],
    },
    DoorItem {
        id: 2,
        kind: DoorKind::Dim,
        sunk_level: 0,
        plate: "诊室 01 · 三天前",
        topic: "它第一次说不",
        lamp: "◦",
        state_desc: "熄灯 · 拒绝记录完整保留",
        sediment: "# 允许未完成",
        inside_tags: &["灯已灭", "那次拒绝还在原处"],
        echo: "那是第一次它停下来，并给出了拒绝的理由。",
        artifacts: &["refusal-trace.json ↗"],
    },
    DoorItem {
        id: 3,
        kind: DoorKind::Dim,
        sunk_level: 0,
        plate: "诊室 00 · 起点",
        topic: "命名仪式",
        lamp: "◦",
        state_desc: "熄灯 · 这是它被叫出名字的地方",
        sediment: "# 边界不是围墙",
        inside_tags: &["灯已灭", "房间刚住进来的那天"],
        echo: "最初的名字被刻在门把手下方。",
        artifacts: &["genesis-prompt.md ↗"],
    },
    DoorItem {
        id: 4,
        kind: DoorKind::Sunk,
        sunk_level: 1,
        plate: "沉积门 · 只读",
        topic: "关于服从的争论",
        lamp: "·",
        state_desc: "已沉下一层 · 门把手不再转动",
        sediment: "# 观察先于干预",
        inside_tags: &["已沉下一层", "门把手不再转动"],
        echo: "更深层的讨论，已经固化为底层的约束。",
        artifacts: &[],
    },
    DoorItem {
        id: 5,
        kind: DoorKind::Sunk,
        sunk_level: 2,
        plate: "沉积门 · 只读",
        topic: "未完成的隔离沙盒",
        lamp: "·",
        state_desc: "更深一层",
        sediment: "# 允许未完成",
        inside_tags: &["更深一层"],
        echo: "沙盒边界被冻结在地层之中。",
        artifacts: &[],
    },
    DoorItem {
        id: 6,
        kind: DoorKind::Sunk,
        sunk_level: 3,
        plate: "沉积门 · 只读",
        topic: "第一次断电",
        lamp: "·",
        state_desc: "深渊边缘",
        sediment: "# 边界不是围墙",
        inside_tags: &["深渊边缘"],
        echo: "停电时留下的断口。",
        artifacts: &[],
    },
];

/// Space ("走廊") module window root component.
pub fn space_app_root(props: ModuleAppProps) -> Element {
    let locale = use_hook(|| Rc::new(LocalePack::load(super::i18n::DEFAULT_LOCALE)));
    let manager = props.manager.clone();
    let rx = props.rx.clone();
    let theme_rx_for_archive = props.theme_rx.clone();
    let mut theme_dark = use_page_shell(&props);

    let theme_class = if theme_dark() { "dark" } else { "light" };

    // Folding states
    let mut folded_order = use_signal(|| false);
    let mut folded_workspace = use_signal(|| false);
    let mut folded_display = use_signal(|| false);
    let mut folded_peek = use_signal(|| false);
    let mut head_folded = use_signal(|| false);

    // Interactive states
    let mut lit_door_id = use_signal(|| 0usize);
    let mut active_door_id = use_signal(|| 0usize);
    let mut selected_order = use_signal(|| 0usize);
    let mut display_sediment = use_signal(|| true);
    let mut display_summary = use_signal(|| false);
    let mut peek_log_open = use_signal(|| false);
    let mut is_streaming = use_signal(|| false);

    let fold_all = move |_| {
        let any_open = !folded_order()
            || !folded_workspace()
            || !folded_display()
            || !folded_peek()
            || !head_folded();
        let target = any_open;
        folded_order.set(target);
        folded_workspace.set(target);
        folded_display.set(target);
        folded_peek.set(target);
        head_folded.set(target);
    };

    let active_door = &DOORS[active_door_id().min(DOORS.len() - 1)];

    rsx! {
        body {
            "data-theme": "{theme_class}",
            "data-window": "space",
            style { dangerous_inner_html: "{css::truth_css()}" }
            style { dangerous_inner_html: "{css::OVERLAY_CSS}" }
            meta { charset: "UTF-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }

            // Chrome at top: 标题左 + ▴ 收纳 + 主题 + ✕ 关窗
            div {
                class: "space-chrome",
                onmousedown: move |_| { window().drag(); },
                span { class: "space-chrome-title", "{locale.t(keys::SPACE_WINDOW_TITLE)}" }
                div { class: "space-chrome-actions",
                    button {
                        class: "fold-btn",
                        title: "{locale.t(keys::WINDOW_FOLD_BTN)}",
                        onmousedown: move |e| { e.stop_propagation(); },
                        onclick: fold_all,
                        "▴ {locale.t(keys::WINDOW_FOLD_BTN)}"
                    }
                    button {
                        class: "theme-btn",
                        id: "space-theme-toggle",
                        title: "{locale.t(keys::CHROME_THEME_TOGGLE)}",
                        "aria-label": "{locale.t(keys::CHROME_THEME_TOGGLE)}",
                        onmousedown: move |e| { e.stop_propagation(); },
                        onclick: move |_| {
                            theme_dark.toggle();
                        },
                        svg {
                            view_box: "0 0 16 16",
                            width: "12", height: "12",
                            fill: "none", stroke: "currentColor",
                            stroke_width: "1.3", stroke_linecap: "round", stroke_linejoin: "round",
                            dangerous_inner_html: "{css::theme_toggle_svg(theme_dark())}",
                        }
                    }
                    {render_close_button(&locale)}
                }
            }

            // Engine: Left Sidebar (mind) + Center Corridor Room + Right Sidebar (ante)
            div { class: "space-engine",
                // Left Column: ORDER + WORKSPACE + DISPLAY
                aside { id: "space-mind",
                    // Card 1: 走廊排序 ORDER
                    div {
                        class: if folded_order() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_order.toggle(); },
                            "{locale.t(keys::SPACE_SECTION_ORDER_TITLE)} "
                            em { "{locale.t(keys::SPACE_SECTION_ORDER_EM)}" }
                            span { class: "fold-caret", if folded_order() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div {
                                class: if selected_order() == 0 { "row active" } else { "row" },
                                onclick: move |_| selected_order.set(0),
                                span { class: "dot-radio" }
                                "按最近亮起"
                            }
                            div {
                                class: if selected_order() == 1 { "row active" } else { "row" },
                                onclick: move |_| selected_order.set(1),
                                span { class: "dot-radio" }
                                "按沉积深度"
                            }
                        }
                    }

                    // Card 2: 工作文件夹 WORKSPACE
                    div {
                        class: if folded_workspace() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_workspace.toggle(); },
                            "{locale.t(keys::SPACE_SECTION_WORKSPACE_TITLE)} "
                            em { "{locale.t(keys::SPACE_SECTION_WORKSPACE_EM)}" }
                            span { class: "fold-caret", if folded_workspace() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div { class: "row active static",
                                "~/northing/alignment"
                                span { class: "tag-x", "已挂载" }
                            }
                        }
                    }

                    // Card 3: 走廊显示 DISPLAY
                    div {
                        class: if folded_display() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_display.toggle(); },
                            "{locale.t(keys::SPACE_SECTION_DISPLAY_TITLE)} "
                            em { "{locale.t(keys::SPACE_SECTION_DISPLAY_EM)}" }
                            span { class: "fold-caret", if folded_display() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div {
                                class: if display_sediment() { "row active" } else { "row" },
                                onclick: move |_| display_sediment.toggle(),
                                span { class: "sq-toggle" }
                                "显示沉积层"
                            }
                            div {
                                class: if display_summary() { "row active" } else { "row" },
                                onclick: move |_| display_summary.toggle(),
                                span { class: "sq-toggle" }
                                "门后摘要常开"
                            }
                        }
                    }
                }

                // Center Room: Status Bar + Hub + Door Hall + Room Input
                section { id: "space-room",
                    div { class: "room-status",
                        span { class: "brand-inline",
                            svg {
                                view_box: "0 0 200 200",
                                "aria-label": "northing",
                                dangerous_inner_html: "{css::brand_logo_svg()}",
                            }
                            span { class: "seal-name", "northing" }
                        }
                        span { "{locale.t(keys::SPACE_STATUS_CORRIDOR)}" }
                        span { class: "state-dot" }
                        span { style: "color:var(--mind-line)", "{locale.t(keys::SPACE_STATUS_ONE_LIT)}" }
                        span { "{locale.t(keys::SPACE_STATUS_REST_DIM)}" }
                        span { class: "sp" }
                    }

                    div {
                        class: if head_folded() { "hall-head folded" } else { "hall-head" },
                        id: "hall-head",
                        button {
                            class: "fold-btn head-fold",
                            title: "{locale.t(keys::WINDOW_FOLD_BTN)}",
                            onclick: move |_| head_folded.toggle(),
                            if head_folded() { "▾" } else { "▴" }
                        }
                        div { class: "name-line", "{locale.t(keys::SPACE_HEAD_NAME)}" }
                        div { class: "state", "{locale.t(keys::SPACE_HEAD_STATE)}" }
                        div { class: "hall-note", "{locale.t(keys::SPACE_HEAD_NOTE)}" }
                    }

                    div { class: "door-hall", id: "door-hall",
                        div { class: "hall-mark", "亮着" }

                        // Render lit & dim doors
                        for door in DOORS.iter().filter(|d| d.kind != DoorKind::Sunk) {
                            {
                                let is_lit = door.id == lit_door_id();
                                let d_id = door.id;
                                rsx! {
                                    div {
                                        class: if is_lit { "door lit" } else { "door dim" },
                                        tabindex: "0",
                                        role: "button",
                                        "aria-pressed": if is_lit { "true" } else { "false" },
                                        onclick: move |_| {
                                            lit_door_id.set(d_id);
                                            active_door_id.set(d_id);
                                        },
                                        span { class: "door-seam" }
                                        div { class: "door-lamp", if is_lit { "{door.lamp}" } else { "◦" } }
                                        div { class: "door-main",
                                            div { class: "door-plate",
                                                if is_lit { "{door.plate} · 门开着" } else { "{door.plate} · 熄灯" }
                                            }
                                            div { class: "door-topic", "{door.topic}" }
                                            div { class: "door-inside",
                                                for (idx, tag) in door.inside_tags.iter().enumerate() {
                                                    if idx > 0 {
                                                        span { "·" }
                                                    }
                                                    span { "{tag}" }
                                                }
                                            }
                                            if is_lit {
                                                div { class: "door-echo", "{door.echo}" }
                                                div { class: "door-actions",
                                                    button {
                                                        class: "btn-enter",
                                                        onclick: move |e| {
                                                            e.stop_propagation();
                                                            #[cfg(target_os = "windows")]
                                                            hide_and_close_hwnd(window().hwnd() as isize);
                                                            window().close();
                                                        },
                                                        "进入这间房"
                                                    }
                                                    for chip in door.artifacts.iter() {
                                                        button {
                                                            class: "artifact-chip",
                                                            onmousedown: move |e| e.stop_propagation(),
                                                            "{chip}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if display_sediment() {
                            div { class: "hall-mark deep", "沉积层 · 走廊尽头往下" }

                            for door in DOORS.iter().filter(|d| d.kind == DoorKind::Sunk) {
                                {
                                    let d_id = door.id;
                                    let level_class = match door.sunk_level {
                                        1 => "l1",
                                        2 => "l2",
                                        _ => "l3",
                                    };
                                    rsx! {
                                        div {
                                            class: "door sunk {level_class}",
                                            onclick: move |_| {
                                                active_door_id.set(d_id);
                                            },
                                            span { class: "door-seam" }
                                            div { class: "door-lamp", "{door.lamp}" }
                                            div { class: "door-main",
                                                div { class: "door-plate", "{door.plate}" }
                                                div { class: "door-topic", "{door.topic}" }
                                                div { class: "door-inside",
                                                    for (idx, tag) in door.inside_tags.iter().enumerate() {
                                                        if idx > 0 {
                                                            span { "·" }
                                                        }
                                                        span { "{tag}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "sunk-tail", "再往下的门已经看不清轮廓" }
                            button {
                                class: "btn-archive",
                                onclick: {
                                    let mgr = manager.clone();
                                    let rx = rx.clone();
                                    let theme_rx_for_archive = theme_rx_for_archive.clone();
                                    move |e| {
                                        e.stop_propagation();
                                        super::app::spawn_module_window_with_theme_rx("archive", &mgr, &rx, theme_rx_for_archive.clone());
                                    }
                                },
                                "{locale.t(keys::SPACE_BTN_ARCHIVE_LINK)}"
                            }
                        }
                    }

                    // Bottom console: room-input
                    div { class: "room-input",
                        div { class: "witness-row",
                            span { class: "witness-note", "新房会带着它现在的沉积开门" }
                        }
                        div { class: "input-row",
                            button { class: "attach", "⌗ 工作文件夹" }
                            div { class: "input-box",
                                "给这间新房一个名字…"
                                span { class: "cursor" }
                            }
                            button {
                                class: if is_streaming() { "send streaming" } else { "send" },
                                id: "send-stop",
                                onclick: move |_| is_streaming.toggle(),
                                if is_streaming() { "■" } else { "➤" }
                            }
                        }
                    }

                    div { class: "room-fog" }
                }

                // Right Column: 门缝所见 PEEK
                aside { id: "space-ante",
                    div {
                        class: if folded_peek() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_peek.toggle(); },
                            "{locale.t(keys::SPACE_SECTION_PEEK_TITLE)} "
                            em { "{locale.t(keys::SPACE_SECTION_PEEK_EM)}" }
                            span { class: "fold-caret", if folded_peek() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div {
                                div { class: "peek-sub", "这扇门 " em { "DOOR" } }
                                div { class: "peek-plate", "{active_door.topic}" }
                                div { class: "peek-line", "{active_door.plate}" }
                                div { class: "peek-line", style: "color:var(--mind-line)", "{active_door.state_desc}" }
                            }

                            div {
                                div { class: "peek-sub", "留在门内的沉积 " em { "ROOM SEDIMENT" } }
                                div { class: "peek-sed", "{active_door.sediment}" }
                                div { class: "seg-note", "沉积属于房间，不属于此刻的它" }
                            }

                            if !active_door.artifacts.is_empty() {
                                div {
                                    div { class: "peek-sub", "门内产物 " em { "ARTIFACTS" } }
                                    div { class: "chips",
                                        for chip in active_door.artifacts.iter() {
                                            button { class: "artifact-chip", "{chip}" }
                                        }
                                    }
                                }
                            }

                            div {
                                button {
                                    class: "peek-log",
                                    onclick: move |_| peek_log_open.toggle(),
                                    "走廊日志 v"
                                }
                                if peek_log_open() {
                                    div { class: "peek-detail",
                                        "list rooms — 一间可亮 / 数间已沉"
                                        br {}
                                        "open room 03 — 门未上锁"
                                        br {}
                                        "archive scan — 只读挂载"
                                    }
                                }
                            }

                            div { class: "term-well",
                                "$ northing rooms --hall"
                                br {}
                                "> 一间亮着 / 三间熄灯"
                                br {}
                                "> 更深的几间已沉入档案馆"
                                br {}
                                "> "
                                span { class: "preview-row", "open: room/{active_door.id:02}" }
                                br {}
                                "> _"
                            }
                        }
                    }
                }
            }
        }
    }
}
