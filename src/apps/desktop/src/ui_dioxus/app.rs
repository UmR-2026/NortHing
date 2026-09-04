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

use dioxus::desktop::window;
use dioxus::prelude::*;
use std::collections::HashSet;
use std::rc::Rc;

use super::api;
use super::color::chronicle_gradient;
use super::css;
use super::i18n::{keys, LocalePack};
use super::markdown_render::render_markdown;
use super::pages_chat_md_css;
use super::registry::ShellWindowManager;
use super::session_mock::MockEntry;
use super::state::{Geometry, GeometryRxArc, GeometryTx, GlobalTheme, RoomWindowIdTx};
use super::turn_banner::{cancelled_body, error_draft_body, kernel_error_message, maybe_set_degraded};
use super::window_ops::{close_module, quit_shell};
use northhing_kernel_api::events::{KernelEventDto, ToolCallPhase};
use northhing_kernel_api::turn::{TurnId, TurnStateKind};
use tokio::sync::oneshot;

pub use super::window_ops::{spawn_module_window, spawn_module_window_with_theme_rx};

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
    let mut degraded: Signal<Option<String>> = use_signal(|| None);
    let mut user_input = use_signal(String::new);
    let mut entries = use_signal(Vec::<MockEntry>::new);
    // W9-1: session-scoped tool allow-list (tool name → auto-approve).
    let session_allow_list = use_signal(|| HashSet::<String>::new());
    let mut mind_base = use_signal(|| "#C8714C".to_string());
    let mut mind_history = use_signal(|| vec!["#DAD6CF".to_string(), "#3F837B".to_string(), "#8B5FBF".to_string()]);

    let mut active_set = use_signal(|| window_manager.subscribe_active().borrow().clone());

    use_future(move || {
        let mut session_id_signal = session_id_signal;
        let mut entries = entries;
        async move {
            let Some(rt) = crate::app_state::turn_runtime::turn_runtime() else {
                tracing::warn!("ui_dioxus::app turn_runtime handle unavailable for room session initialization");
                return;
            };

            let (tx, rx) = oneshot::channel();
            rt.spawn(async move {
                let session_res = api::ensure_room_session().await;
                let messages_res = match &session_res {
                    Ok(sid) => Some(api::get_messages(sid).await),
                    Err(_) => None,
                };
                let _ = tx.send((session_res, messages_res));
            });

            match rx.await {
                Ok((Ok(sid), msgs_opt)) => {
                    session_id_signal.set(Some(sid.clone()));
                    if let Some(msgs_res) = msgs_opt {
                        match msgs_res {
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
                }
                Ok((Err(e), _)) => {
                    tracing::warn!("ui_dioxus::app ensure_room_session failed: {e}");
                }
                Err(e) => {
                    tracing::warn!("ui_dioxus::app room session initialization channel closed: {e}");
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
        let session_allow_list = session_allow_list;
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
                        let tool_name = tc.name.clone();
                        if session_allow_list.read().contains(tool_name.as_str()) {
                            match api::respond_to_tool_confirmation(&tc.call_id, true).await {
                                Ok(()) => {
                                    entries.write().push(MockEntry::Approval {
                                        call_id: tc.call_id,
                                        head: tc.name,
                                        main: tc.summary,
                                        risk: tc.detail.unwrap_or_default(),
                                        resolved: true,
                                        state_text: Some("已自动允许（本会话）".to_string()),
                                    });
                                }
                                Err(e) => {
                                    // S2 fix: log failure (call_id + tool + err), then fall back to a pending card so the user retains manual approve/reject affordance. Tool stays in allow-list — failure is per-call (likely transient), not a tool verdict.
                                    tracing::warn!(
                                        "ui_dioxus::app session_allow_list auto-approve failed: call_id={} tool={}: {}",
                                        tc.call_id,
                                        tool_name,
                                        e
                                    );
                                    super::approval_card::push_pending_approval(
                                        entries,
                                        tc.call_id,
                                        tc.name,
                                        tc.summary,
                                        tc.detail.unwrap_or_default(),
                                    );
                                }
                            }
                        } else {
                            super::approval_card::push_pending_approval(
                                entries,
                                tc.call_id,
                                tc.name,
                                tc.summary,
                                tc.detail.unwrap_or_default(),
                            );
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
                                    degraded.set(None);
                                }
                                TurnStateKind::Failed => {
                                    let err_text = error.unwrap_or_else(|| "Turn failed".into());
                                    maybe_set_degraded(&err_text, degraded);
                                    let body = error_draft_body(assistant_draft.write().take(), err_text);
                                    entries.write().push(MockEntry::Entity {
                                        who: "它".into(),
                                        body,
                                        children: vec![],
                                    });
                                    streaming.set(false);
                                    active_turn_id.set(None);
                                }
                                TurnStateKind::Cancelled => {
                                    let body = cancelled_body(assistant_draft.write().take());
                                    entries.write().push(MockEntry::Entity {
                                        who: "它".into(),
                                        body,
                                        children: vec![],
                                    });
                                    streaming.set(false);
                                    active_turn_id.set(None);
                                    degraded.set(None);
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
        let degraded = degraded;
        let existing_sid = session_id_signal();
        let text_witness = text.clone();
        spawn(async move {
            enum SendOutcome {
                Success {
                    new_sid: Option<String>,
                    turn_id: TurnId,
                },
                SessionError(northhing_kernel_api::error::KernelError),
                SubmitError {
                    new_sid: Option<String>,
                    error: northhing_kernel_api::error::KernelError,
                },
            }

            let res = api::spawn_on_turn_runtime("send_action", async move {
                let (new_sid, sid) = match existing_sid {
                    Some(s) => (None, s),
                    None => match api::ensure_room_session().await {
                        Ok(s) => (Some(s.clone()), s),
                        Err(e) => return SendOutcome::SessionError(e),
                    },
                };

                match api::submit_turn(&sid, text).await {
                    Ok(turn_id) => SendOutcome::Success { new_sid, turn_id },
                    Err(error) => SendOutcome::SubmitError { new_sid, error },
                }
            })
            .await;

            match res {
                Ok(SendOutcome::Success { new_sid, turn_id }) => {
                    if let Some(s) = new_sid {
                        session_id_signal.set(Some(s));
                    }
                    active_turn_id.set(Some(turn_id));
                    streaming.set(true);
                    user_input.set(String::new());
                    send_error.set(None);
                    entries.write().push(MockEntry::Witness {
                        who: "见证者".into(),
                        body: text_witness,
                    });
                }
                Ok(SendOutcome::SubmitError { new_sid, error }) => {
                    if let Some(s) = new_sid {
                        session_id_signal.set(Some(s));
                    }
                    let err_text = kernel_error_message(&error);
                    maybe_set_degraded(&err_text, degraded);
                    send_error.set(Some(format!("Submit error: {error}")));
                }
                Ok(SendOutcome::SessionError(e)) => {
                    send_error.set(Some(format!("Session error: {e}")));
                }
                Err(()) => {
                    send_error.set(Some("Background runtime unavailable".to_string()));
                }
            }
        });
    };

    let stop_action = move || {
        let mut streaming = streaming;
        let mut active_turn_id = active_turn_id;
        if let Some(turn_id) = active_turn_id() {
            spawn(async move {
                let res =
                    api::spawn_on_turn_runtime("stop_action", async move { api::stop_turn(&turn_id).await }).await;
                if let Ok(Err(e)) = res {
                    tracing::warn!("ui_dioxus::stop_action stop_turn failed: {e}");
                }
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
            style { dangerous_inner_html: "{pages_chat_md_css::CHAT_MD_CSS}" }
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

                    if let Some(reason) = degraded.read().as_ref() {
                        div { class: "degraded-banner", "{reason}" }
                    }

                    div { class: "chat-flow", id: "chat-flow",
                            div { class: "session-open",
                                "{locale.t(keys::SESSION_BANNER)}"
                            }
                            {render_entries(entries.read().iter(), entries, &locale, session_allow_list)}
                            if let Some(ref draft) = *assistant_draft.read() {
                                div { class: "rec entity",
                                    div { class: "who", "它" }
                                    div { class: "body",
                                        div { class: "msg-agent md-rendered", {render_markdown(draft)} }
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

fn render_entries<'a>(
    iter: impl Iterator<Item = &'a MockEntry>,
    entries: Signal<Vec<MockEntry>>,
    locale: &LocalePack,
    session_allow_list: Signal<HashSet<String>>,
) -> Element {
    let items: Vec<&MockEntry> = iter.collect();
    rsx! {
        for entry in items.iter() {
            {render_entry(entry, entries, locale, session_allow_list)}
        }
    }
}

fn render_entry(
    entry: &MockEntry,
    entries: Signal<Vec<MockEntry>>,
    locale: &LocalePack,
    session_allow_list: Signal<HashSet<String>>,
) -> Element {
    match entry {
        MockEntry::Entity { who, body, children } => rsx! {
            div { class: "rec entity",
                div { class: "who", "{who}" }
                div { class: "body",
                    div { class: "msg-agent md-rendered", {render_markdown(body)} }
                    for child in children.iter() {
                        {render_child(child, locale)}
                    }
                }
            }
        },
        MockEntry::Witness { who, body } => rsx! {
            div { class: "rec witness",
                div { class: "who", "{who}" }
                div { class: "body md-rendered", {render_markdown(body)} }
            }
        },
        MockEntry::Approval {
            call_id,
            head,
            main,
            risk,
            resolved,
            state_text,
        } => super::approval_card::render_approval_card(
            call_id.clone(),
            head.clone(),
            main.clone(),
            risk.clone(),
            *resolved,
            state_text.clone(),
            entries,
            session_allow_list,
            head.clone(), // tool name for the allow-list button (raw name from Tc.name in event-driven entries)
            locale,
        ),
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
