// SPDX-License-Identifier: MIT OR Apache-2.0
//
// R3' / W1 migration (2026-08-15) - room main window component.
//
// Mirrors the truth HTML consult-room-main.html body (LL275..L459).
// Includes:
//   * chrome (LL334..L340) with theme toggle + min/max/close buttons
//   * room-status (LL342..L357) with brand-inline
//   * room-head (LL359..L364) with avatar, name, chronicle bar, state
//   * chat-flow (LL366..L416) with mock session seed
//   * room-input (LL418..L427) deck with attach + input + send/stop
//   * membrane + membrane-nodes (jewels) for dynamic module window toggles
//
// Brief §4.5 - "CSS 原样内联（禁翻译成 Rust 样式）; id/data-* 锚点不改名".

use dioxus::core::VirtualDom;
use dioxus::desktop::tao::dpi::{LogicalPosition, LogicalSize};
#[cfg(target_os = "windows")]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;
use dioxus::desktop::tao::window::WindowBuilder;
use dioxus::desktop::{window, Config, WindowCloseBehaviour};
use dioxus::prelude::*;
use std::rc::Rc;

use super::api;
use super::css;
use super::entry::{shared_webview_data_directory_for_inner, startup_scale_factor, DOCK_GAP_PX};
use super::i18n::{keys, LocalePack};
use super::registry::{DockSide, ModuleAppProps, ShellWindowManager};
use super::session_mock::{seed_session, MockEntry};
use super::state::{Geometry, GeometryRxArc, GeometryTx, GlobalTheme, RoomWindowIdTx};
use northhing_kernel_api::events::{KernelEventDto, ToolCallPhase};
use northhing_kernel_api::turn::{TurnId, TurnStateKind};
use tokio::sync::watch;

#[cfg(target_os = "windows")]
pub(crate) mod win_ops {
    use std::ffi::c_void;

    unsafe extern "system" {
        pub fn ShowWindow(h_wnd: *mut c_void, n_cmd_show: i32) -> i32;
        pub fn PostMessageW(h_wnd: *mut c_void, msg: u32, wparam: usize, lparam: isize) -> i32;
        pub fn IsWindow(h_wnd: *mut c_void) -> i32;
    }

    pub const WM_CLOSE: u32 = 0x0010;
    pub const SW_HIDE: i32 = 0;

    /// Hides and posts WM_CLOSE to an OS window by HWND, with a background watchdog
    /// (std thread, never use_future) to guarantee window destruction.
    pub fn close_os_window(hwnd: usize) {
        if hwnd == 0 {
            return;
        }
        unsafe {
            ShowWindow(hwnd as *mut c_void, SW_HIDE);
            PostMessageW(hwnd as *mut c_void, WM_CLOSE, 0, 0);
        }
        std::thread::Builder::new()
            .name("window-close-watchdog".into())
            .spawn(move || {
                let hwnd_ptr = hwnd as *mut c_void;
                for _ in 0..5 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    unsafe {
                        if IsWindow(hwnd_ptr) == 0 {
                            break;
                        }
                        ShowWindow(hwnd_ptr, SW_HIDE);
                        PostMessageW(hwnd_ptr, WM_CLOSE, 0, 0);
                    }
                }
            })
            .ok();
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) mod win_ops {
    pub fn close_os_window(_hwnd: usize) {}
}

fn close_module(id: &'static str, wm: &ShellWindowManager) {
    if let Some((wid, hwnd)) = wm.mark_closing_target(id) {
        window().close_window(wid);
        win_ops::close_os_window(hwnd);
    }
}

fn close_all_modules(wm: &ShellWindowManager) {
    for (_id, wid, hwnd) in wm.mark_all_closing_targets() {
        window().close_window(wid);
        win_ops::close_os_window(hwnd);
    }
}

fn quit_shell(wm: &ShellWindowManager) {
    close_all_modules(wm);
    #[cfg(target_os = "windows")]
    win_ops::close_os_window(window().hwnd() as usize);
    window().close();
}

/// RSX root for the room main window.
pub fn room_app_root() -> Element {
    let geometry_tx = use_context::<GeometryTx>();
    let geometry_rx_arc = use_context::<GeometryRxArc>();
    let theme = use_context::<GlobalTheme>();
    let window_manager = use_context::<ShellWindowManager>();
    let room_window_id_tx = use_context::<RoomWindowIdTx>();

    let locale = use_hook(|| Rc::new(LocalePack::load(super::i18n::DEFAULT_LOCALE)));

    let mut theme_dark = use_signal(|| true);
    let mut head_folded = use_signal(|| false);
    let mut streaming = use_signal(|| false);
    let mut active_turn_id: Signal<Option<TurnId>> = use_signal(|| None);
    let session_id_signal: Signal<Option<String>> = use_signal(|| None);
    let mut assistant_draft: Signal<Option<String>> = use_signal(|| None);
    let send_error: Signal<Option<String>> = use_signal(|| None);
    let mut user_input = use_signal(String::new);
    let mut entries = use_signal(|| seed_session());
    let mut mind_base = use_signal(|| "#C8714C".to_string());
    let mut mind_history = use_signal(|| vec!["#DAD6CF".to_string(), "#3F837B".to_string(), "#8B5FBF".to_string()]);

    let mut active_set = use_signal(|| window_manager.subscribe_active().borrow().clone());

    use_future(move || {
        let mut session_id_signal = session_id_signal;
        let mut entries = entries;
        async move {
            match api::ensure_room_session().await {
                Ok(sid) => {
                    session_id_signal.set(Some(sid.clone()));
                    match api::get_messages(&sid).await {
                        Ok(msgs) => {
                            let converted = super::session_mock::messages_to_entries(msgs);
                            if !converted.is_empty() {
                                entries.set(converted);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("ui_dioxus::app get_messages failed: {e}");
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("ui_dioxus::app ensure_room_session failed: {e}");
                }
            }
        }
    });

    let wm_future = window_manager.clone();
    use_future(move || {
        let wm = wm_future.clone();
        async move {
            let mut active_rx = wm.subscribe_active();
            loop {
                if active_rx.changed().await.is_err() {
                    break;
                }
                active_set.set(active_rx.borrow().clone());
            }
        }
    });

    use_future(move || {
        let mut rx = api::event_channel();
        let sid = session_id_signal;
        async move {
            while let Some(dto) = rx.recv().await {
                match dto {
                    KernelEventDto::TextChunk { session_id, text } => {
                        if sid.read().as_ref().map(|s| s == &session_id).unwrap_or(true) {
                            let mut d = assistant_draft.write();
                            let cur = d.get_or_insert_with(String::new);
                            cur.push_str(&text);
                        }
                    }
                    KernelEventDto::ToolCall(tc)
                        if tc.phase == ToolCallPhase::AwaitingConfirmation
                            && sid.read().as_ref().map(|s| s == &tc.session_id).unwrap_or(true) =>
                    {
                        let mut entries_guard = entries.write();
                        let already_exists = entries_guard.iter().any(|e| match e {
                            MockEntry::Approval { call_id, .. } => call_id == &tc.call_id,
                            _ => false,
                        });
                        if !already_exists {
                            entries_guard.push(MockEntry::Approval {
                                call_id: tc.call_id,
                                head: tc.name,
                                main: tc.summary,
                                risk: tc.detail.unwrap_or_default(),
                                resolved: false,
                                state_text: None,
                            });
                        }
                    }
                    KernelEventDto::TurnState {
                        session_id,
                        turn_id: _,
                        state,
                        error,
                        ..
                    } => {
                        if sid.read().as_ref().map(|s| s == &session_id).unwrap_or(true) {
                            match state {
                                TurnStateKind::Completed => {
                                    if let Some(draft) = assistant_draft.write().take() {
                                        if !draft.is_empty() {
                                            entries.write().push(MockEntry::Entity {
                                                who: "它".into(),
                                                body: draft,
                                                children: vec![],
                                            });
                                        }
                                    }
                                    streaming.set(false);
                                    active_turn_id.set(None);
                                }
                                TurnStateKind::Failed => {
                                    let err_text = error.unwrap_or_else(|| "Turn failed".into());
                                    let body = if let Some(draft) = assistant_draft.write().take() {
                                        if draft.is_empty() {
                                            format!("[Error: {err_text}]")
                                        } else {
                                            format!("{draft}\n[Error: {err_text}]")
                                        }
                                    } else {
                                        format!("[Error: {err_text}]")
                                    };
                                    entries.write().push(MockEntry::Entity {
                                        who: "它".into(),
                                        body,
                                        children: vec![],
                                    });
                                    streaming.set(false);
                                    active_turn_id.set(None);
                                }
                                TurnStateKind::Cancelled => {
                                    let body = if let Some(draft) = assistant_draft.write().take() {
                                        if draft.is_empty() {
                                            "[Cancelled]".to_string()
                                        } else {
                                            format!("{draft}\n[Cancelled]")
                                        }
                                    } else {
                                        "[Cancelled]".to_string()
                                    };
                                    entries.write().push(MockEntry::Entity {
                                        who: "它".into(),
                                        body,
                                        children: vec![],
                                    });
                                    streaming.set(false);
                                    active_turn_id.set(None);
                                }
                                TurnStateKind::Started => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    // Register room window ID on mount
    {
        let geometry_tx = geometry_tx.clone();
        let room_window_id_tx = room_window_id_tx.clone();
        use_effect(move || {
            let _ = room_window_id_tx.send(Some(window().id()));
            if let Ok(pos) = window().outer_position() {
                let size = window().outer_size();
                let _ = geometry_tx.send(Geometry {
                    x: pos.x,
                    y: pos.y,
                    width: size.width,
                    height: size.height,
                });
            }
        });
    }

    let theme_class = if theme_dark() { "dark" } else { "light" };

    let (left_open, right_open) = {
        let active = active_set.read();
        (active.contains("self"), active.contains("work"))
    };

    // chrome 控件文案 i18n 化（2026-08-22，审查 M1 + 终审 Minor×2 合并修）：
    // aria-label/title 全走 locale 键；条件文案在此预计算（单 guard 复用，
    // 同轮修 FYI-2 双 borrow）。
    let head_seam_label = if head_folded() {
        locale.t(keys::CHROME_HEAD_UNFOLD).to_string()
    } else {
        locale.t(keys::CHROME_HEAD_FOLD).to_string()
    };
    let send_label = if streaming() {
        locale.t(keys::DECK_SEND_STREAMING).to_string()
    } else {
        locale.t(keys::DECK_SEND).to_string()
    };

    let send_action = move || {
        let text = user_input.read().trim().to_string();
        if text.is_empty() {
            return;
        }
        let mut active_turn_id = active_turn_id;
        let mut streaming = streaming;
        let mut user_input = user_input;
        let mut send_error = send_error;
        let mut session_id_signal = session_id_signal;
        let mut entries = entries;
        spawn(async move {
            let sid = match session_id_signal() {
                Some(s) => s,
                None => match api::ensure_room_session().await {
                    Ok(s) => {
                        session_id_signal.set(Some(s.clone()));
                        s
                    }
                    Err(e) => {
                        send_error.set(Some(format!("Session error: {e}")));
                        return;
                    }
                },
            };

            match api::submit_turn(&sid, text.clone()).await {
                Ok(turn_id) => {
                    active_turn_id.set(Some(turn_id));
                    streaming.set(true);
                    user_input.set(String::new());
                    send_error.set(None);
                    entries.write().push(MockEntry::Witness {
                        who: "见证者".into(),
                        body: text,
                    });
                }
                Err(e) => {
                    send_error.set(Some(format!("Submit error: {e}")));
                }
            }
        });
    };

    let stop_action = move || {
        let mut streaming = streaming;
        let mut active_turn_id = active_turn_id;
        if let Some(turn_id) = active_turn_id() {
            spawn(async move {
                let _ = api::stop_turn(&turn_id).await;
            });
        }
        streaming.set(false);
        active_turn_id.set(None);
    };

    let (wm_left, geom_rx_left, theme_left) = (window_manager.clone(), geometry_rx_arc.clone(), theme.clone());
    let (wm_right, geom_rx_right, theme_right) = (window_manager.clone(), geometry_rx_arc.clone(), theme.clone());
    let (wm_nav_archive, geom_rx_nav_archive, theme_nav_archive) =
        (window_manager.clone(), geometry_rx_arc.clone(), theme.clone());
    let (wm_nav_space, geom_rx_nav_space, theme_nav_space) =
        (window_manager.clone(), geometry_rx_arc.clone(), theme.clone());
    let (wm_nav_onboarding, geom_rx_nav_onboarding, theme_nav_onboarding) =
        (window_manager.clone(), geometry_rx_arc.clone(), theme.clone());
    let wm_close = window_manager.clone();

    rsx! {
        body {
            "data-theme": "{theme_class}",
            lang: "zh-CN",
            style { dangerous_inner_html: "{css::truth_css()}" }
            style { dangerous_inner_html: "{css::OVERLAY_CSS}" }
            meta { charset: "UTF-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }
            title { "{locale.t(keys::WINDOW_TITLE_ROOM)}" }

            div { id: "containment" }
            div { class: "membrane-frame" }
            div { id: "global-aura" }

            div { id: "engine",
                div { id: "room-wrap",
                    section { id: "room",
                        span { class: "membrane l" }
                        span { class: "membrane r" }
                        div { class: "room-fog" }

                        div { class: "room-controls",
                            button {
                                class: "rc-btn",
                                id: "theme-toggle",
                                "aria-label": "{locale.t(keys::CHROME_THEME_TOGGLE)}",
                                title: "{locale.t(keys::CHROME_THEME_TOGGLE)}",
                                onclick: move |_| {
                                    let next = !theme_dark();
                                    theme_dark.set(next);
                                    theme.set_dark(next);
                                },
                                svg {
                                    view_box: "0 0 16 16",
                                    width: "12", height: "12",
                                    fill: "none", stroke: "currentColor",
                                    stroke_width: "1.3", stroke_linecap: "round", stroke_linejoin: "round",
                                    dangerous_inner_html: "{css::theme_toggle_svg(theme_dark())}",
                                }
                            }
                            button {
                                class: "rc-btn",
                                "aria-label": "{locale.t(keys::CHROME_MINIMIZE)}",
                                title: "{locale.t(keys::CHROME_MINIMIZE)}",
                                onclick: move |_| {
                                    window().set_minimized(true);
                                },
                                "─"
                            }
                            button {
                                class: "rc-btn",
                                "aria-label": "{locale.t(keys::CHROME_MAXIMIZE)}",
                                title: "{locale.t(keys::CHROME_MAXIMIZE)}",
                                onclick: move |_| {
                                    window().toggle_maximized();
                                },
                                "□"
                            }
                            button {
                                class: "rc-btn close",
                                "aria-label": "{locale.t(keys::CHROME_CLOSE)}",
                                title: "{locale.t(keys::CHROME_CLOSE)}",
                                onclick: move |_| {
                                    quit_shell(&wm_close);
                                },
                                "✕"
                            }
                        }

                        div { class: "room-status",
                            onmousedown: move |_| {
                                window().drag();
                            },
                            span { class: "brand-inline",
                                svg {
                                    view_box: "0 0 200 200",
                                    "aria-label": "northing",
                                    dangerous_inner_html: "{css::brand_logo_svg()}",
                                }
                                span { class: "seal-name", "northing" }
                            }
                            span { "{locale.t(keys::STATUS_PILL)}" }
                            button {
                                class: "status-nav-link",
                                id: "nav-archive",
                                title: "{locale.t(keys::NAV_ARCHIVE)}",
                                onmousedown: move |e| {
                                    e.stop_propagation();
                                },
                                onclick: move |_| {
                                    spawn_module_window("archive", &wm_nav_archive, &geom_rx_nav_archive, &theme_nav_archive);
                                },
                                "{locale.t(keys::NAV_ARCHIVE)}"
                            }
                            button {
                                class: "status-nav-link",
                                id: "nav-space",
                                title: "{locale.t(keys::NAV_SPACE)}",
                                onmousedown: move |e| {
                                    e.stop_propagation();
                                },
                                onclick: move |_| {
                                    spawn_module_window("space", &wm_nav_space, &geom_rx_nav_space, &theme_nav_space);
                                },
                                "{locale.t(keys::NAV_SPACE)}"
                            }
                            button {
                                class: "status-nav-link",
                                id: "nav-onboarding",
                                title: "{locale.t(keys::NAV_ONBOARDING)}",
                                onmousedown: move |e| {
                                    e.stop_propagation();
                                },
                                onclick: move |_| {
                                    spawn_module_window("onboarding", &wm_nav_onboarding, &geom_rx_nav_onboarding, &theme_nav_onboarding);
                                },
                                "{locale.t(keys::NAV_ONBOARDING)}"
                            }
                            span { class: "sp" }
                        }

                        div {
                            class: if head_folded() { "room-head folded" } else { "room-head" },
                            id: "room-head",
                            onmousedown: move |_| {
                                window().drag();
                            },
                            div { class: "agent-avatar", id: "avatar-core",
                                onmousedown: move |e| {
                                    e.stop_propagation();
                                },
                                "{locale.t(keys::ROOM_HEAD_INITIAL)}"
                            }
                            div { class: "name-line", "{locale.t(keys::ROOM_HEAD_NAME)}" }
                            div {
                                class: "chronicle-bar",
                                id: "chronicle-bar",
                                style: format!("background: {}", chronicle_gradient(&mind_history.read(), &mind_base.read())),
                                title: "它换代表色时：新色自右端进入，旧色慢慢沉向左（双击演示）",
                                onmousedown: move |e| {
                                    e.stop_propagation();
                                },
                                ondoubleclick: move |_| {
                                    let cur = mind_base();
                                    mind_history.write().push(cur.clone());
                                    let minds = ["#C8714C", "#3F837B", "#8B5FBF", "#D99B48", "#4B8F6B"];
                                    let next = minds[(minds.iter().position(|m| *m == cur).unwrap_or(0) + 1) % 5];
                                    mind_base.set(next.to_string());
                                },
                            }
                            div { class: "state",
                                "{locale.t(keys::ROOM_HEAD_STATE)}"
                            }
                            button {
                                class: "head-seam-fold",
                                "aria-label": "{head_seam_label}",
                                title: "{head_seam_label}",
                                onmousedown: move |e| {
                                    e.stop_propagation();
                                },
                                onclick: move |_| {
                                    head_folded.set(!head_folded());
                                },
                                span { class: if head_folded() { "seam-bar folded" } else { "seam-bar" } }
                            }
                        }

                        div { class: "chat-flow", id: "chat-flow",
                            div { class: "session-open",
                                "{locale.t(keys::SESSION_BANNER)}"
                            }
                            {render_entries(entries.read().iter(), entries, &locale)}
                            if let Some(ref draft) = *assistant_draft.read() {
                                div { class: "rec entity",
                                    div { class: "who", "它" }
                                    div { class: "body",
                                        div { class: "msg-agent", "{draft}" }
                                    }
                                }
                            }
                        }

                        div { class: "room-input",
                            if let Some(ref err) = *send_error.read() {
                                div { class: "send-error", style: "color: var(--faint); font-size: 11px; padding-bottom: 4px;", "{err}" }
                            }
                            div { class: "input-row",
                                button {
                                    class: "attach",
                                    "aria-label": "{locale.t(keys::DECK_ATTACH)}",
                                    title: "{locale.t(keys::DECK_ATTACH)}",
                                    svg {
                                        view_box: "0 0 18 18",
                                        width: "16", height: "16",
                                        fill: "none", stroke: "currentColor",
                                        stroke_width: "1.2", stroke_linecap: "round",
                                        circle { cx: "9", cy: "9", r: "7.2" }
                                        line { x1: "9", y1: "5.8", x2: "9", y2: "12.2" }
                                        line { x1: "5.8", y1: "9", x2: "12.2", y2: "9" }
                                    }
                                }
                                input {
                                    class: "input-box",
                                    r#type: "text",
                                    value: "{user_input}",
                                    placeholder: "{locale.t(keys::DECK_PLACEHOLDER)}",
                                    oninput: move |e| user_input.set(e.value()),
                                    onkeydown: move |e| {
                                        if !e.is_composing() && e.key() == Key::Enter {
                                            if !streaming() {
                                                send_action();
                                            }
                                        }
                                    },
                                }
                                button {
                                    class: if streaming() { "send streaming" } else { "send" },
                                    id: "send-stop",
                                    "aria-label": "{send_label}",
                                    onclick: move |_| {
                                        if streaming() {
                                            stop_action();
                                        } else {
                                            send_action();
                                        }
                                    },
                                    if streaming() { "■" } else { "➤" }
                                }
                            }
                        }

                        // Left Jewel: 单扇满高「沉积与设施」（W2.7，半高对切退役）。
                        button {
                            class: if left_open { "membrane-node left is-open" } else { "membrane-node left" },
                            id: "trig-mind",
                            "aria-label": "{locale.t(keys::GEM_LEFT_LABEL)}",
                            "aria-expanded": if left_open { "true" } else { "false" },
                            title: "{locale.t(keys::GEM_LEFT_TITLE)}",
                            onclick: move |_| {
                                if wm_left.is_active("self") {
                                    close_module("self", &wm_left);
                                } else {
                                    spawn_module_window("self", &wm_left, &geom_rx_left, &theme_left);
                                }
                            }
                        }
                        // Right Jewel: toggles work window
                        button {
                            class: if right_open { "membrane-node right is-open" } else { "membrane-node right" },
                            id: "trig-work",
                            "aria-label": "{locale.t(keys::GEM_RIGHT_LABEL)}",
                            "aria-expanded": if right_open { "true" } else { "false" },
                            title: "{locale.t(keys::GEM_RIGHT_TITLE)}",
                            onclick: move |_| {
                                if wm_right.is_active("work") {
                                    close_module("work", &wm_right);
                                } else {
                                    spawn_module_window("work", &wm_right, &geom_rx_right, &theme_right);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Dynamic module window spawner using `new_window` and `WindowCloseBehaviour::WindowCloses`.
pub fn spawn_module_window(
    id: &'static str,
    manager: &ShellWindowManager,
    geometry_rx: &GeometryRxArc,
    theme: &GlobalTheme,
) {
    let theme_rx = theme.subscribe();
    spawn_module_window_with_theme_rx(id, manager, geometry_rx, theme_rx);
}

/// Dynamic module window spawner accepting a theme receiver.
pub fn spawn_module_window_with_theme_rx(
    id: &'static str,
    manager: &ShellWindowManager,
    geometry_rx: &GeometryRxArc,
    theme_rx: watch::Receiver<bool>,
) {
    let plugin = match manager.registry().get(id) {
        Some(p) => p.clone(),
        None => return,
    };

    let gen = match manager.mark_opening(id) {
        Some(g) => g,
        None => return,
    };

    let data_directory = shared_webview_data_directory_for_inner();

    // I2 审查降级证据（2026-08-22，review-w2 I2 不修的决定依据）：
    // 此处 borrow 到的几何在 gem 可点击前必然已是真实值——两层保证：
    //   1. 通道初值 = 房间创建位（entry.rs initial_geometry 与
    //      with_position 同源常量，非病态占位）；
    //   2. entry.rs tao 事件处理器 pre-mount 接纳（r3p5）：窗口创建
    //      的首个 Moved 事件即发布真实物理几何，早于 webview 渲染。
    // gem 位于 room webview 内，渲染完成才可点击，故「首帧前点击」
    // 时序不可达；残留风险仅 cosmetic。行为不改，避免触碰 W1 取证区。
    let room_geom = *geometry_rx.borrow();
    let scale = startup_scale_factor();
    let scale = if scale > 0.0 { scale } else { 1.0 };
    let room_x_log = room_geom.x as f64 / scale;
    let room_y_log = room_geom.y as f64 / scale;
    let room_w_log = room_geom.width as f64 / scale;
    let room_h_log = room_geom.height as f64 / scale;

    let (initial_x, initial_y, initial_w, initial_h) = match plugin.dock_side {
        DockSide::LeftFull => (
            room_x_log - plugin.initial_width - DOCK_GAP_PX as f64,
            room_y_log,
            plugin.initial_width,
            if room_h_log > 0.0 {
                room_h_log
            } else {
                plugin.initial_height
            },
        ),
        DockSide::RightFull => (
            room_x_log + room_w_log + DOCK_GAP_PX as f64,
            room_y_log,
            plugin.initial_width,
            if room_h_log > 0.0 {
                room_h_log
            } else {
                plugin.initial_height
            },
        ),
        DockSide::Center => (
            room_x_log + (room_w_log - plugin.initial_width) / 2.0,
            room_y_log + 24.0,
            plugin.initial_width,
            plugin.initial_height,
        ),
        DockSide::Fullscreen => (
            room_x_log,
            room_y_log,
            if room_w_log > 0.0 {
                room_w_log
            } else {
                plugin.initial_width
            },
            if room_h_log > 0.0 {
                room_h_log
            } else {
                plugin.initial_height
            },
        ),
    };

    let mut builder = WindowBuilder::new()
        .with_title(plugin.title)
        .with_inner_size(LogicalSize::new(initial_w, initial_h))
        .with_position(LogicalPosition::new(initial_x, initial_y))
        .with_decorations(false);

    #[cfg(target_os = "windows")]
    {
        use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;
        builder = builder.with_skip_taskbar(true);
    }

    let cfg = Config::default()
        .with_window(builder)
        .with_close_behaviour(WindowCloseBehaviour::WindowCloses)
        .with_data_directory(data_directory);

    let props = ModuleAppProps {
        plugin_id: id,
        gen,
        rx: geometry_rx.clone(),
        theme_rx,
        manager: manager.clone(),
    };

    let dom = VirtualDom::new_with_props(plugin.component, props);

    // T7 裁定（③-c 接受+注释）：new_window 返回的 PendingDesktopContext 有意丢弃。
    // 影响面 = 放弃经 dioxus DesktopContext API 操控本窗；模块窗生命周期（开/关/析构）
    // 已由 registry + HWND 通道全权负责（W1 racefix，见 registry.rs close_os_window），
    // 且本窗 chrome 只有 收纳/✕，min/max/drag 等 DesktopContext 能力用不上。
    // 若未来确需 dioxus 原生窗控，再透传并 resolve()。
    let _ = window().new_window(dom, cfg);
}

fn render_entries<'a>(
    iter: impl Iterator<Item = &'a MockEntry>,
    entries: Signal<Vec<MockEntry>>,
    locale: &LocalePack,
) -> Element {
    let items: Vec<&MockEntry> = iter.collect();
    rsx! {
        for entry in items.iter() {
            {render_entry(entry, entries, locale)}
        }
    }
}

fn render_entry(entry: &MockEntry, entries: Signal<Vec<MockEntry>>, locale: &LocalePack) -> Element {
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
        MockEntry::Approval {
            call_id,
            head,
            main,
            risk,
            resolved,
            state_text,
        } => {
            let handle_action = |approved: bool, status: &'static str| {
                let cid = call_id.clone();
                move |_| {
                    let cid = cid.clone();
                    let mut entries = entries;
                    spawn(async move {
                        if api::respond_to_tool_confirmation(&cid, approved).await.is_ok() {
                            let mut guard = entries.write();
                            if let Some(MockEntry::Approval {
                                resolved, state_text, ..
                            }) = guard.iter_mut().find(|e| match e {
                                MockEntry::Approval { call_id, .. } => call_id == &cid,
                                _ => false,
                            }) {
                                *resolved = true;
                                *state_text = Some(status.to_string());
                            }
                        }
                    });
                }
            };
            rsx! {
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
                                button {
                                    class: "btn-approve",
                                    onclick: handle_action(true, "已授权操作"),
                                    "{locale.t(keys::APPROVAL_APPROVE)}"
                                }
                                button {
                                    class: "btn-reject",
                                    onclick: handle_action(false, "已拒绝操作"),
                                    "{locale.t(keys::APPROVAL_REJECT)}"
                                }
                            }
                        }
                    }
                }
            }
        }
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

fn parse_hex_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let s = hex.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b))
}

pub fn mix_hex(a: &str, b: &str, t: f64) -> String {
    let (ar, ag, ab) = match parse_hex_rgb(a) {
        Some(c) => c,
        None => return b.to_string(),
    };
    let (br, bg, bb) = match parse_hex_rgb(b) {
        Some(c) => c,
        None => return a.to_string(),
    };
    let t = t.clamp(0.0, 1.0);
    let r = (ar as f64 + (br as f64 - ar as f64) * t).round().clamp(0.0, 255.0) as u8;
    let g = (ag as f64 + (bg as f64 - ag as f64) * t).round().clamp(0.0, 255.0) as u8;
    let b_val = (ab as f64 + (bb as f64 - ab as f64) * t).round().clamp(0.0, 255.0) as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b_val)
}

pub fn chronicle_gradient(history: &[String], current: &str) -> String {
    const BIRTH: &str = "#DAD6CF";
    let n = history.len();
    let mut stops = Vec::with_capacity(n + 1);

    if n == 0 {
        stops.push(format!("{BIRTH} 0.00%"));
    } else {
        for (i, c) in history.iter().enumerate() {
            let (pos, col) = if n == 1 {
                (0.0, c.clone())
            } else {
                let frac = i as f64 / (n - 1) as f64;
                let pos = frac * 70.0;
                let col = if i == 0 {
                    c.clone()
                } else {
                    let t = 0.18 + 0.82 * frac;
                    mix_hex(BIRTH, c, t)
                };
                (pos, col)
            };
            stops.push(format!("{col} {pos:.2}%"));
        }
    }

    stops.push(format!("{current} 100%"));
    format!("linear-gradient(90deg, {})", stops.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mix_hex() {
        assert_eq!(mix_hex("#DAD6CF", "#3F837B", 1.0), "#3F837B");
        assert_eq!(mix_hex("#DAD6CF", "#3F837B", 0.0), "#DAD6CF");
    }

    #[test]
    fn test_chronicle_gradient_single() {
        let grad = chronicle_gradient(&["#DAD6CF".to_string()], "#C8714C");
        assert!(grad.contains("0.00%"));
        assert!(grad.contains("100%"));
    }

    #[test]
    fn test_chronicle_gradient_three_history() {
        let history = vec!["#DAD6CF".to_string(), "#3F837B".to_string(), "#8B5FBF".to_string()];
        let grad = chronicle_gradient(&history, "#C8714C");
        assert!(grad.contains("0.00%"));
        assert!(grad.contains("35.00%"));
        assert!(grad.contains("70.00%"));
        assert!(grad.contains("100%"));
    }
}
