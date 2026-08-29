// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Task W9-4 (2026-08-29) — Archive session management (search/rename/delete/export + subagent visibility).
//
// Standalone OS window for archive session management.

use dioxus::desktop::window;
use dioxus::prelude::*;
use northhing_kernel_api::session::{MessageDto, SessionId, SessionSummaryDto};
use std::rc::Rc;

use super::api;
use super::css;
use super::i18n::{keys, LocalePack};
use super::page_shell::{render_close_button, use_page_shell};
use super::registry::ModuleAppProps;

#[cfg(target_os = "windows")]
#[allow(unused_imports)]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;

// ── Helpers ──────────────────────────────────────────────────────────

fn is_subagent_session(name: &str, parent_id: &Option<String>) -> bool {
    name.starts_with("Subagent: ") || parent_id.is_some()
}

fn fmt_ts(ts: i64) -> String {
    let secs = ts / 1000;
    let nanos = ((ts % 1000) * 1_000_000) as u32;
    if let Some(dt) = chrono::DateTime::from_timestamp(secs, nanos) {
        dt.format("%Y-%m-%d %H:%M").to_string()
    } else {
        format!("{}", ts)
    }
}

fn fmt_status(status: &northhing_kernel_api::session::SessionStatusDto) -> &'static str {
    match status {
        northhing_kernel_api::session::SessionStatusDto::Active => "活跃",
        northhing_kernel_api::session::SessionStatusDto::Archived => "已归档",
        northhing_kernel_api::session::SessionStatusDto::Completed => "已完成",
    }
}

fn message_role_label(role: &northhing_kernel_api::session::MessageRoleDto) -> &'static str {
    match role {
        northhing_kernel_api::session::MessageRoleDto::User => "用户",
        northhing_kernel_api::session::MessageRoleDto::Assistant => "助手",
        northhing_kernel_api::session::MessageRoleDto::Tool => "工具",
        northhing_kernel_api::session::MessageRoleDto::System => "系统",
    }
}

fn message_content_text(msg: &MessageDto) -> String {
    match &msg.content {
        northhing_kernel_api::session::MessageContentDto::Text(t) => t.clone(),
        northhing_kernel_api::session::MessageContentDto::Multimodal { text, .. } => text.clone(),
        northhing_kernel_api::session::MessageContentDto::ToolResult {
            tool_name,
            result,
            is_error,
            ..
        } => {
            let err_tag = if *is_error { " [ERROR]" } else { "" };
            let summary = result.as_str().unwrap_or_default();
            format!("[{tool_name}{err_tag}] {summary}")
        }
        northhing_kernel_api::session::MessageContentDto::Mixed { text, tool_calls, .. } => {
            if tool_calls.is_empty() {
                text.clone()
            } else {
                let names: Vec<&str> = tool_calls.iter().map(|t| t.tool_name.as_str()).collect();
                format!("{text} | 工具: {}", names.join(", "))
            }
        }
    }
}

// ── Session row model ────────────────────────────────────────────────

#[derive(Clone)]
struct SessionRow {
    summary: SessionSummaryDto,
    is_subagent: bool,
    is_room: bool,
}

// ── Export formatter (pure, unit-tested) ─────────────────────────────

/// Formats a session's messages as a Markdown document.
///
/// # Panics
/// Never panics – falls back to placeholder strings on malformed content.
pub fn format_session_export(session_id: &str, messages: &[MessageDto]) -> String {
    let header = format!(
        "# Session Export\n\n**Session ID:** `{session_id}`\n**Exported:** {}\n**Messages:** {}\n\n---\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        messages.len()
    );

    let body = messages
        .iter()
        .map(|msg| {
            let role = match msg.role {
                northhing_kernel_api::session::MessageRoleDto::User => "User",
                northhing_kernel_api::session::MessageRoleDto::Assistant => "Assistant",
                northhing_kernel_api::session::MessageRoleDto::Tool => "Tool",
                northhing_kernel_api::session::MessageRoleDto::System => "System",
            };
            let ts = fmt_ts(msg.timestamp);

            let content = match &msg.content {
                northhing_kernel_api::session::MessageContentDto::Text(t) => t.clone(),
                northhing_kernel_api::session::MessageContentDto::Multimodal { text, .. } => text.clone(),
                northhing_kernel_api::session::MessageContentDto::ToolResult {
                    tool_name,
                    result,
                    is_error,
                    ..
                } => {
                    let err_tag = if *is_error { " [ERROR]" } else { "" };
                    let summary = result.as_str().unwrap_or_default();
                    format!("[Tool: {tool_name}{err_tag}] {summary}")
                }
                northhing_kernel_api::session::MessageContentDto::Mixed { text, tool_calls, .. } => {
                    let tc = if tool_calls.is_empty() {
                        String::new()
                    } else {
                        let names: Vec<&str> = tool_calls.iter().map(|t| t.tool_name.as_str()).collect();
                        format!("\n**Tool calls:** {}", names.join(", "))
                    };
                    format!("{text}{tc}")
                }
            };

            format!("### [{ts}] {role}\n\n{content}\n")
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("{header}{body}")
}

// ── Rename input validator (pure, unit-tested) ────────────────────────

/// Maximum allowed session-title length, counted in Unicode scalar values (chars), not UTF-8 bytes.
/// CJK ideographs are 3 UTF-8 bytes each; counting bytes would silently truncate ~26-char titles.
pub const MAX_SESSION_NAME_CHARS: usize = 80;

/// Reason a candidate session title was rejected.
#[derive(Debug, PartialEq, Eq)]
pub enum RenameError {
    /// `name.trim().is_empty()` — nothing to save (or only whitespace).
    Empty,
    /// `name.chars().count() > MAX_SESSION_NAME_CHARS`.
    TooLong,
}

/// Pure validator for the rename input box. Returns `Ok(())` when the trimmed title is
/// non-empty AND at most `MAX_SESSION_NAME_CHARS` Unicode chars; otherwise the specific reason.
pub fn validate_rename(name: &str) -> Result<(), RenameError> {
    if name.trim().is_empty() {
        return Err(RenameError::Empty);
    }
    if name.chars().count() > MAX_SESSION_NAME_CHARS {
        return Err(RenameError::TooLong);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_session_export_empty_messages() {
        let out = format_session_export("s1", &[]);
        assert!(out.contains("Session ID"));
        assert!(out.contains("Messages:** 0"));
    }

    #[test]
    fn format_session_export_includes_content() {
        let msgs = vec![MessageDto {
            id: "m1".into(),
            role: northhing_kernel_api::session::MessageRoleDto::User,
            content: northhing_kernel_api::session::MessageContentDto::Text("hello".into()),
            metadata: None,
            timestamp: 1_700_000_000_000,
        }];
        let out = format_session_export("s1", &msgs);
        assert!(out.contains("hello"));
        assert!(out.contains("User"));
    }

    #[test]
    fn validate_rename_accepts_ascii_under_limit() {
        assert!(validate_rename("hello world").is_ok());
        let s = "a".repeat(MAX_SESSION_NAME_CHARS);
        assert!(validate_rename(&s).is_ok());
    }

    #[test]
    fn validate_rename_rejects_empty_and_whitespace() {
        assert_eq!(validate_rename(""), Err(RenameError::Empty));
        assert_eq!(validate_rename("   "), Err(RenameError::Empty));
        assert_eq!(validate_rename("\t\n"), Err(RenameError::Empty));
    }

    #[test]
    fn validate_rename_rejects_too_long_ascii() {
        let s = "a".repeat(MAX_SESSION_NAME_CHARS + 1);
        assert_eq!(validate_rename(&s), Err(RenameError::TooLong));
    }

    /// Regression for M-2 (CJK byte-vs-char truncation): 80 CJK ideographs are 240 UTF-8 bytes,
    /// which the old `len() > 80` check would have flagged as too long.
    #[test]
    fn validate_rename_accepts_cjk_at_char_limit() {
        let s: String = "测".repeat(MAX_SESSION_NAME_CHARS);
        assert_eq!(s.len(), MAX_SESSION_NAME_CHARS * 3, "sanity: 80 CJK chars = 240 bytes");
        assert!(validate_rename(&s).is_ok(), "80 CJK chars must pass; only bytes tripped the old check");
    }

    #[test]
    fn validate_rename_rejects_cjk_over_char_limit() {
        let s: String = "测".repeat(MAX_SESSION_NAME_CHARS + 1);
        assert_eq!(validate_rename(&s), Err(RenameError::TooLong));
    }
}

// ── Component ────────────────────────────────────────────────────────

pub fn archive_app_root(props: ModuleAppProps) -> Element {
    let locale = use_hook(|| Rc::new(LocalePack::load(super::i18n::DEFAULT_LOCALE)));
    let mut theme_dark = use_page_shell(&props);

    let theme_class = if theme_dark() { "dark" } else { "light" };

    // State signals
    let mut all_sessions = use_signal(|| Vec::<SessionRow>::new());
    let loading = use_signal(|| false);
    let error_msg = use_signal(String::new);
    let mut search_query = use_signal(String::new);
    let mut selected_ids = use_signal(|| Vec::<SessionId>::new());
    let mut session_messages = use_signal(|| Vec::<MessageDto>::new());
    let mut msgs_loading = use_signal(|| false);
    let mut msgs_error = use_signal(String::new);

    // Rename state
    let mut renaming_id = use_signal(|| None::<SessionId>);
    let mut rename_value = use_signal(String::new);

    // Delete confirmation state: Some(id) when awaiting second click
    let mut confirming_delete = use_signal(|| None::<SessionId>);

    // Export path toast
    let mut export_path = use_signal(String::new);
    let mut op_error = use_signal(String::new);

    // Room session id cache (read once on mount)
    let room_session_id = use_signal(|| None::<String>);

    // Load sessions on mount
    {
        let locale_for_load = locale.clone();
        let mut error_msg = error_msg;
        let mut loading = loading;
        let mut all_sessions = all_sessions;
        let mut room_session_id = room_session_id;
        spawn(async move {
            loading.set(true);
            error_msg.set(String::new());

            // Resolve room session id for delete guard
            let room_id = api::get_room_session_id().await;
            let room_id_for_guard = room_id.clone();
            room_session_id.set(room_id);

            match api::list_sessions_all_workspaces().await {
                Ok(groups) => {
                    let mut rows = Vec::new();
                    for group in groups {
                        for summary in group.sessions {
                            let is_subagent = is_subagent_session(&summary.name, &summary.parent_session_id);
                            let is_room = room_id_for_guard
                                .as_ref()
                                .map(|rid| rid == &summary.id)
                                .unwrap_or(false);
                            rows.push(SessionRow {
                                summary,
                                is_subagent,
                                is_room,
                            });
                        }
                    }
                    // Sort by updated_at descending
                    rows.sort_by(|a, b| b.summary.updated_at.cmp(&a.summary.updated_at));
                    all_sessions.set(rows);
                    loading.set(false);
                }
                Err(e) => {
                    error_msg.set(format!("{} {}", locale_for_load.t(keys::ARCHIVE_LOAD_FAIL), e));
                    loading.set(false);
                }
            }
        });
    }

    // Filtered list
    let q = search_query.read().trim().to_lowercase();
    let binding = all_sessions.read();
    let filtered: Vec<&SessionRow> = binding
        .iter()
        .filter(|r| {
            if q.is_empty() {
                true
            } else {
                r.summary.name.to_lowercase().contains(&q)
            }
        })
        .collect();

    // Action handlers
    let mut start_rename = move |id: SessionId, name: String| {
        renaming_id.set(Some(id));
        rename_value.set(name);
        confirming_delete.set(None);
    };

    let mut cancel_rename = move || {
        renaming_id.set(None);
        rename_value.set(String::new());
    };

    let mut confirm_delete = move |id: SessionId| {
        confirming_delete.set(Some(id));
    };

    let cancel_delete = move |_| {
        confirming_delete.set(None);
    };

    let mut view_detail = move |id: SessionId| {
        selected_ids.set(vec![id.clone()]);
        msgs_error.set(String::new());
        session_messages.set(Vec::new());
        msgs_loading.set(true);
        let sid = id;
        spawn(async move {
            match api::get_messages(&sid).await {
                Ok(msgs) => {
                    session_messages.set(msgs);
                    msgs_loading.set(false);
                }
                Err(e) => {
                    msgs_error.set(format!("加载消息失败: {}", e));
                    msgs_loading.set(false);
                }
            }
        });
    };

    let close_detail = move |_| {
        selected_ids.set(Vec::new());
        session_messages.set(Vec::new());
    };

    // Is this session the active room? (helper kept for clarity; per-row `room` flag is captured inline below)

    rsx! {
            body {
                "data-theme": "{theme_class}",
                "data-window": "archive",
                style { dangerous_inner_html: "{css::truth_css()}" }
                style { dangerous_inner_html: "{css::OVERLAY_CSS}" }
                meta { charset: "UTF-8" }
                meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }

                // Chrome at top
                div {
                    class: "archive-chrome",
                    onmousedown: move |_| { window().drag(); },
                    span { class: "archive-chrome-title", "{locale.t(keys::ARCHIVE_WINDOW_TITLE)}" }
                    div { class: "archive-chrome-actions",
                        button {
                            class: "theme-btn",
                            title: "{locale.t(keys::CHROME_THEME_TOGGLE)}",
                            "aria-label": "{locale.t(keys::CHROME_THEME_TOGGLE)}",
                            onmousedown: move |e| { e.stop_propagation(); },
                            onclick: move |_| { theme_dark.toggle(); },
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

                // Main engine: sidebar + content
                div { class: "archive-engine",
                    aside { id: "archive-mind",
                        div {
                            class: "mod",
                            div { class: "side-title w2-pin", "{locale.t(keys::ARCHIVE_SECTION_DEPTH_TITLE)} " em { "{locale.t(keys::ARCHIVE_SECTION_DEPTH_EM)}" } }
                            div { class: "w2-scroll",
                                div { class: "row active", span { class: "dot-radio" }, "已归档会话" }
                            }
                        }
                        div {
                            class: "mod",
                            div { class: "side-title w2-pin", "{locale.t(keys::ARCHIVE_SECTION_SOLAR_TITLE)} " em { "{locale.t(keys::ARCHIVE_SECTION_SOLAR_EM)}" } }
                            div { class: "w2-scroll",
                                div { class: "row", span { class: "dot-radio" }, "全部" }
                                div { class: "row", span { class: "dot-radio" }, "本周" }
                                div { class: "row", span { class: "dot-radio" }, "更早" }
                            }
                        }
                    }

                    section { id: "archive-room",
                        // Room status bar
                        div { class: "room-status",
                            span { class: "brand-inline",
                                svg { view_box: "0 0 200 200", "aria-label": "northing", dangerous_inner_html: "{css::brand_logo_svg()}" }
                                span { class: "seal-name", "northing" }
                            }
                            span { "{locale.t(keys::ARCHIVE_STATUS_MODE)}" }
                            span { class: "sp" }
                            span { "{locale.t(keys::ARCHIVE_STATUS_TAG)}" }
                            span { class: "state-dot" }
                        }

                        // Head
                        div { class: "room-head", id: "archive-room-head",
                            div { class: "depth-marker", id: "avatar-core", "{locale.t(keys::ARCHIVE_HEAD_INITIAL)}" }
                            div { class: "name-line", "{locale.t(keys::ARCHIVE_HEAD_NAME)}" }
                            div { class: "state", "{locale.t(keys::ARCHIVE_HEAD_STATE)}" }
                        }

                        // Search bar
                        div { class: "mem-toolbar",
                            input {
                                class: "mem-search",
                                r#type: "text",
                                placeholder: "{locale.t(keys::ARCHIVE_SEARCH_PLACEHOLDER)}",
                                value: "{search_query}",
                                oninput: move |e| search_query.set(e.value()),
                            }
                            if !search_query.read().trim().is_empty() {
                                button {
                                    class: "mem-btn mem-btn-clear",
                                    onclick: move |_| search_query.set(String::new()),
                                    "{locale.t(keys::ARCHIVE_SEARCH_CLEAR)}"
                                }
                            }
                        }

                        // Op error (rename/delete/export error)
                        if !op_error.read().is_empty() {
                            div { class: "mem-error", "{op_error}" }
                        }

                        // Export path toast
                        if !export_path.read().is_empty() {
                            div { class: "mem-export-path",
                                "{locale.t(keys::ARCHIVE_EXPORT_PATH)} "
                                span { style: "font-family: var(--font-mono); font-size: 11px;", "{export_path}" }
                            }
                        }

                        // Loading
                        if loading() {
                            div { class: "mem-loading", "加载中..." }
                        }

                        // Error state
                        if !error_msg.read().is_empty() && !loading() {
                            div { class: "mem-error", "{error_msg}" }
                        }

                        // Empty state
                        if !loading() && error_msg.read().is_empty() && filtered.is_empty() {
                            div { class: "mem-empty",
                                if q.is_empty() { "{locale.t(keys::ARCHIVE_EMPTY)}" }
                                else { "{locale.t(keys::ARCHIVE_EMPTY_SEARCH)}" }
                            }
                        }

                        // Session list
                        if !filtered.is_empty() {
                            div { class: "strata-flow", id: "strata-flow",
                                for row in filtered.iter() {
                                    {
                                        let row = row;
                                        let sid: Rc<String> = Rc::new(row.summary.id.clone());
                                        let sname: Rc<String> = Rc::new(row.summary.name.clone());
                                        // Per-clone Rc handles so multiple `move` closures can capture independent references.
                                        let sid_click = Rc::clone(&sid);
                                        let sid_view = Rc::clone(&sid);
                                        let sid_exec = Rc::clone(&sid);
                                        let sid_start = Rc::clone(&sid);
                                        let sid_conf = Rc::clone(&sid);
                                        let sid_export = Rc::clone(&sid);
                                        let sname_start = Rc::clone(&sname);
                                        let is_renaming = renaming_id.read().as_ref().map(|s| s.as_str()) == Some(sid.as_str());
                                        let is_deleting = confirming_delete.read().as_ref().map(|s| s.as_str()) == Some(sid.as_str());
                                        let room = row.is_room;

                                        rsx! {
                                            div {
                                                class: if room { "stratum active" } else { "stratum" },
                                                style: if row.is_subagent {
                                                    "opacity: 0.55; padding-left: 28px;"
                                                } else {
                                                    ""
                                                },
    onclick: move |_| view_detail((*sid_view).clone()),
                                                div { class: "stratum-head",
                                                    span { class: "stratum-no", "{fmt_status(&row.summary.status)}" }
                                                    span { class: "stratum-time", "{fmt_ts(row.summary.updated_at)}" }
                                                    if row.is_subagent {
                                                        span {
                                                            class: "stratum-subagent-badge",
                                                            style: "font-size: 10px; background: var(--line, #333); color: var(--faint, #888); padding: 1px 6px; border-radius: 3px; margin-left: 6px;",
                                                            "{locale.t(keys::ARCHIVE_SUBAGENT_BADGE)}"
                                                        }
                                                    }
                                                }

                                                if is_renaming {
                                                    div { class: "stratum-title",
                                                        input {
                                                            class: "mem-search",
                                                            r#type: "text",
                                                            placeholder: "{locale.t(keys::ARCHIVE_RENAME_PLACEHOLDER)}",
                                                            value: "{rename_value}",
                                                            maxlength: "80",
                                                            oninput: move |e| rename_value.set(e.value()),
                                                            onkeydown: {
                                                                let locale_for_handler = locale.clone();
                                                                move |e| {
                                                                    if !e.is_composing() && e.key() == Key::Enter {
                                                                        let Some(sid) = renaming_id.take() else { return };
                                                                        let new_name = rename_value.take();
                                                                        if let Err(reason) = validate_rename(&new_name) {
                                                                            let msg = match reason {
                                                                                RenameError::Empty => format!("{}", locale_for_handler.t(keys::ARCHIVE_RENAME_PLACEHOLDER)),
                                                                                RenameError::TooLong => format!("标题不能超过 {} 个字符", MAX_SESSION_NAME_CHARS),
                                                                            };
                                                                            op_error.set(msg);
                                                                            renaming_id.set(Some(sid));
                                                                            return;
                                                                        }
                                                                        let name = new_name;
                                                                        let locale_async = locale_for_handler.clone();
                                                                        spawn(async move {
                                                                            match api::rename_session(&sid, &name).await {
                                                                                Ok(()) => {
                                                                                    if let Some(row) = all_sessions.write().iter_mut().find(|r| r.summary.id == sid) {
                                                                                        row.summary.name = name.clone();
                                                                                    }
                                                                                    op_error.set(String::new());
                                                                                }
                                                                                Err(e) => {
                                                                                    op_error.set(format!("{}{}", locale_async.t(keys::ARCHIVE_RENAME_FAIL), e));
                                                                                    renaming_id.set(Some(sid));
                                                                                }
                                                                            }
                                                                        });
                                                                    }
                                                                }
                                                            },
                                                        }
                                                        button {
                                                            class: "mem-btn mem-btn-clear",
                                                            style: "margin-left: 4px; padding: 2px 8px; font-size: 11px;",
                                                            onclick: move |_| cancel_rename(),
                                                            "取消"
                                                        }
                                                    }
                                                } else {
                                                    div { class: "stratum-title", "{sname}" }
                                                }

                                                // Actions row
                                                if !is_renaming {
                                                    div { class: "stratum-meta",
                                                        span { class: "who", "{fmt_status(&row.summary.status)}" }

                                                        if is_deleting {
                                                            div { style: "display: inline-flex; align-items: center; gap: 6px; margin-left: 8px;",
                                                                span { style: "font-size: 11px; color: var(--danger, #ef4444);", "{locale.t(keys::ARCHIVE_DELETE_CONFIRM)}" }
                                                                button {
                                                                    class: "mem-btn mem-btn-clear",
                                                                    style: "padding: 2px 8px; font-size: 11px;",
                                                                    onclick: cancel_delete,
                                                                    "取消"
                                                                }
                                                                button {
                                                                    class: "mem-btn",
                                                                    style: "padding: 2px 8px; font-size: 11px; background: var(--danger, #ef4444); color: #fff; border: none; border-radius: 3px;",
                                                                    onclick: {
                                                                        let locale_for_handler = locale.clone();
                                                                        move |_| {
                                                                            let id = (*sid_exec).clone();
                                                                            confirming_delete.set(None);
                                                                            let locale_async = locale_for_handler.clone();
                                                                            spawn(async move {
                                                                                match api::delete_session(&id).await {
                                                                                    Ok(()) => {
                                                                                        all_sessions.write().retain(|r| r.summary.id != id);
                                                                                        op_error.set(String::new());
                                                                                    }
                                                                                    Err(e) => {
                                                                                        op_error.set(format!("{}{}", locale_async.t(keys::ARCHIVE_DELETE_FAIL), e));
                                                                                        confirming_delete.set(Some(id));
                                                                                    }
                                                                                }
                                                                            });
                                                                        }
                                                                    },
    "确定删除"
                                                                }
                                                            }
                                                        } else {
                                                            button {
                                                                class: "mem-btn mem-btn-clear",
                                                                style: "padding: 2px 6px; font-size: 10px;",
                                                                onclick: move |_| start_rename((*sid_start).clone(), (*sname_start).clone()),
                                                                "{locale.t(keys::ARCHIVE_BTN_RENAME)}"
                                                            }
                                                            if room {
                                                                button {
                                                                    class: "mem-btn mem-btn-clear",
                                                                    style: "padding: 2px 6px; font-size: 10px; opacity: 0.4; cursor: not-allowed;",
                                                                    disabled: true,
                                                                    title: "{locale.t(keys::ARCHIVE_DELETE_FORBIDDEN)}",
                                                                    "{locale.t(keys::ARCHIVE_BTN_DELETE)}"
                                                                }
                                                            } else {
                                                                button {
                                                                    class: "mem-btn mem-btn-clear",
                                                                    style: "padding: 2px 6px; font-size: 10px; color: var(--danger, #ef4444);",
                                                                    onclick: move |_| confirm_delete((*sid_conf).clone()),
                                                                    "{locale.t(keys::ARCHIVE_BTN_DELETE)}"
                                                                }
                                                            }
                                                            button {
                                                                class: "mem-btn mem-btn-export",
                                                                style: "padding: 2px 6px; font-size: 10px;",
                                                                onclick: {
                                                                    let locale_for_handler = locale.clone();
                                                                    move |_| {
                                                                        let id = (*sid_export).clone();
                                                                        op_error.set(String::new());
                                                                        let locale_async = locale_for_handler.clone();
                                                                        spawn(async move {
                                                                            match api::get_messages(&id).await {
                                                                                Ok(msgs) => {
                                                                                    let md = format_session_export(&id, &msgs);
                                                                                    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
                                                                                    let filename = format!("session-{}-{}.md", &id[..8], ts);
                                                                                    let dir = dirs::config_dir()
                                                                                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                                                                                        .join("northhing")
                                                                                        .join("exports");
                                                                                    if let Err(e) = std::fs::create_dir_all(&dir) {
                                                                                        op_error.set(format!("{}{}", locale_async.t(keys::ARCHIVE_EXPORT_FAIL), e));
                                                                                        return;
                                                                                    }
                                                                                    let path = dir.join(&filename);
                                                                                    if let Err(e) = std::fs::write(&path, md) {
                                                                                        op_error.set(format!("{}{}", locale_async.t(keys::ARCHIVE_EXPORT_FAIL), e));
                                                                                    } else {
                                                                                        export_path.set(path.to_string_lossy().to_string());
                                                                                    }
                                                                                }
                                                                                Err(e) => {
                                                                                    op_error.set(format!("{} {}", locale_async.t(keys::ARCHIVE_EXPORT_FAIL), e));
                                                                                }
                                                                            }
                                                                        });
                                                                    }
                                                                },
                                                                "{locale.t(keys::ARCHIVE_BTN_EXPORT)}"
                                                            }
                                                            button {
                                                                class: "mem-btn",
                                                                style: "padding: 2px 6px; font-size: 10px;",
    onclick: move |_| view_detail((*sid_click).clone()),
                                                                "{locale.t(keys::ARCHIVE_BTN_DETAIL)}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Session detail panel
                        if !selected_ids.read().is_empty() {
                            div { class: "mem-list", style: "border-top: 1px solid var(--line); padding-top: 8px;",
                                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;",
                                    span { style: "font-size: 12px; font-weight: 500;", "消息详情（只读）" }
                                    button {
                                        class: "mem-btn mem-btn-clear",
                                        style: "padding: 2px 8px; font-size: 11px;",
                                        onclick: close_detail,
                                        "关闭"
                                    }
                                }

                                if msgs_loading() {
                                    div { class: "mem-loading", "加载消息..." }
                                }
                                if !msgs_error.read().is_empty() {
                                    div { class: "mem-error", "{msgs_error}" }
                                }
                                if session_messages.read().is_empty() && !msgs_loading() {
                                    div { class: "mem-empty", "暂无消息" }
                                }
                                for msg in session_messages.read().iter() {
                                    div { class: "mem-row",
                                        div { style: "font-size: 10px; color: var(--faint); margin-bottom: 2px;",
                                            span { style: "font-weight: 500; color: var(--text);", "{message_role_label(&msg.role)}" }
                                            span { style: "margin-left: 8px;", "{fmt_ts(msg.timestamp)}" }
                                        }
                                        div { class: "mem-text", style: "font-size: 12px;",
                                            "{message_content_text(msg)}"
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "abyss-foot",
                            div { class: "abyss-foot-note", "{locale.t(keys::ARCHIVE_FOOT_NOTE)}" }
                        }
                        div { class: "room-fog" }
                    }
                }
            }
        }
}
