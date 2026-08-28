// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Task W9-2 (2026-08-29) — Memory browser module window (read-only).
//
// Standalone OS window for browsing/searching/exporting agent memory facts.

use dioxus::desktop::window;
use dioxus::prelude::*;
use std::rc::Rc;

use super::api;
use super::css;
use super::page_shell::use_page_shell;
use super::registry::ModuleAppProps;

#[cfg(target_os = "windows")]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;

#[derive(Clone)]
struct FactItem {
    id: String,
    text: String,
    scope: String,
    confidence: String,
    fact_type: String,
    created_at: u64,
    session_id: String,
    turn_id: String,
}

fn fmt_ts(ts: u64) -> String {
    let secs = ts / 1000;
    let nanos = ((ts % 1000) * 1_000_000) as u32;
    if let Some(dt) = chrono::DateTime::from_timestamp(secs as i64, nanos) {
        dt.format("%Y-%m-%d %H:%M").to_string()
    } else {
        format!("{}", ts)
    }
}

fn scope_label(s: &str) -> &str {
    match s {
        "global" => "全局",
        "workspace" => "工作区",
        _ => s,
    }
}

fn confidence_label(c: &str) -> &str {
    match c {
        "high" => "高",
        "med" => "中",
        "low" => "低",
        _ => c,
    }
}

fn fact_type_label(t: &str) -> &str {
    match t {
        "user" => "用户",
        "feedback" => "反馈",
        "project" => "项目",
        "reference" => "引用",
        _ => t,
    }
}

pub fn memory_app_root(props: ModuleAppProps) -> Element {
    let theme_dark = use_page_shell(&props);

    let class = if theme_dark() { "dark" } else { "light" };

    let mut facts = use_signal(|| Vec::<FactItem>::new());
    let mut search_query = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut error_msg = use_signal(String::new);
    let mut export_path = use_signal(String::new);

    let is_searching = use_memo(move || !search_query.read().trim().is_empty());

    // Load facts on mount + when search toggles off
    use_effect(move || {
        let query = search_query.read().clone();
        let mut facts = facts;
        let mut loading = loading;
        let mut error_msg = error_msg;

        if !query.trim().is_empty() {
            return;
        }

        loading.set(true);
        error_msg.set(String::new());

        spawn(async move {
            match api::list_facts(None).await {
                Ok(f) => {
                    let items = f
                        .into_iter()
                        .map(|d| FactItem {
                            id: d.id,
                            text: d.text,
                            scope: d.scope,
                            confidence: d.confidence,
                            fact_type: d.fact_type,
                            created_at: d.created_at,
                            session_id: d.session_id,
                            turn_id: d.turn_id,
                        })
                        .collect::<Vec<_>>();
                    facts.set(items);
                    loading.set(false);
                }
                Err(e) => {
                    error_msg.set(format!("加载失败: {}", e));
                    loading.set(false);
                }
            }
        });
    });

    let do_search = move |_| {
        let q = search_query.read().clone();
        let mut facts = facts;
        let mut loading = loading;
        let mut error_msg = error_msg;

        if q.trim().is_empty() {
            return;
        }

        loading.set(true);
        error_msg.set(String::new());

        let query = q.clone();
        spawn(async move {
            match api::search_facts(&query, None, Some(20)).await {
                Ok(f) => {
                    let items = f
                        .into_iter()
                        .map(|d| FactItem {
                            id: d.id,
                            text: d.text,
                            scope: d.scope,
                            confidence: d.confidence,
                            fact_type: d.fact_type,
                            created_at: d.created_at,
                            session_id: d.session_id,
                            turn_id: d.turn_id,
                        })
                        .collect::<Vec<_>>();
                    facts.set(items);
                    loading.set(false);
                }
                Err(e) => {
                    error_msg.set(format!("搜索失败: {}", e));
                    loading.set(false);
                }
            }
        });
    };

    let clear_search = move |_| {
        search_query.set(String::new());
        let mut facts = facts;
        let mut loading = loading;
        let mut error_msg = error_msg;
        loading.set(true);
        error_msg.set(String::new());
        spawn(async move {
            match api::list_facts(None).await {
                Ok(f) => {
                    let items = f
                        .into_iter()
                        .map(|d| FactItem {
                            id: d.id,
                            text: d.text,
                            scope: d.scope,
                            confidence: d.confidence,
                            fact_type: d.fact_type,
                            created_at: d.created_at,
                            session_id: d.session_id,
                            turn_id: d.turn_id,
                        })
                        .collect::<Vec<_>>();
                    facts.set(items);
                    loading.set(false);
                }
                Err(e) => {
                    error_msg.set(format!("加载失败: {}", e));
                    loading.set(false);
                }
            }
        });
    };

    let do_export = move |_| {
        let current: Vec<FactItem> = facts.read().clone();
        if current.is_empty() {
            return;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("northhing")
            .join("exports");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            error_msg.set(format!("创建导出目录失败: {}", e));
            return;
        }
        let path = dir.join(format!("memory-{}.jsonl", now));
        let mut lines = Vec::with_capacity(current.len());
        for item in &current {
            let json = serde_json::json!({
                "id": item.id,
                "text": item.text,
                "scope": item.scope,
                "confidence": item.confidence,
                "fact_type": item.fact_type,
                "created_at": item.created_at,
                "session_id": item.session_id,
                "turn_id": item.turn_id,
            });
            lines.push(serde_json::to_string(&json).unwrap_or_default());
        }
        let content = lines.join("\n");
        if let Err(e) = std::fs::write(&path, content) {
            error_msg.set(format!("导出失败: {}", e));
        } else {
            export_path.set(path.to_string_lossy().to_string());
        }
    };

    rsx! {
        body {
            "data-theme": "{class}",
            "data-window": "module",
            style { dangerous_inner_html: "{css::truth_css()}" }
            style { dangerous_inner_html: "{css::OVERLAY_CSS}" }
            meta { charset: "UTF-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }
            aside {
                id: "memory-panel",
                div { class: "station-head",
                    "记忆浏览器"
                    button {
                        class: "close-btn",
                        title: "关闭",
                        onmousedown: move |e| { e.stop_propagation(); },
                        onclick: move |_| {
                            #[cfg(target_os = "windows")]
                            {
                                let hwnd = window().hwnd();
                                super::windows::win::hide_and_close_hwnd(hwnd as isize);
                            }
                            window().close();
                        },
                        "✕"
                    }
                }

                // Toolbar: search + export
                div { class: "mem-toolbar",
                    input {
                        class: "mem-search",
                        r#type: "text",
                        placeholder: "搜索记忆...",
                        value: "{search_query}",
                        oninput: move |e| search_query.set(e.value()),
                        onkeydown: move |e| {
                            if !e.is_composing() && e.key() == Key::Enter {
                                do_search(());
                            }
                        }
                    }
                    if !search_query.read().is_empty() {
                        button {
                            class: "mem-btn mem-btn-clear",
                            onclick: clear_search,
                            "清除"
                        }
                    }
                    button {
                        class: "mem-btn mem-btn-export",
                        disabled: facts.read().is_empty(),
                        onclick: do_export,
                        "导出 JSONL"
                    }
                }

                // Export path notice
                if !export_path.read().is_empty() {
                    div { class: "mem-export-path",
                        "{export_path}"
                    }
                }

                // Error state
                if !error_msg.read().is_empty() {
                    div { class: "mem-error", "{error_msg}" }
                }

                // Loading state
                if loading() {
                    div { class: "mem-loading", "加载中..." }
                }

                // Fact list
                div { class: "mem-list",
                    if !is_searching() && facts.read().is_empty() && !loading() {
                        div { class: "mem-empty", "暂无记忆事实" }
                    } else if is_searching() && facts.read().is_empty() && !loading() {
                        div { class: "mem-empty", "未找到匹配的记忆" }
                    } else {
                        for item in facts.read().iter() {
                            div { class: "mem-row",
                                div { class: "mem-text", "{item.text}" }
                                div { class: "mem-meta",
                                    span { class: "mem-scope", "{scope_label(&item.scope)}" }
                                    span { class: "mem-conf", "{confidence_label(&item.confidence)}" }
                                    span { class: "mem-type", "{fact_type_label(&item.fact_type)}" }
                                    span { class: "mem-time", "{fmt_ts(item.created_at)}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
