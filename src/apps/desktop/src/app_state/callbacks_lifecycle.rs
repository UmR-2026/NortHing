//! Lifecycle Slint callback wirings (R37a split from mod.rs)
//!
//! Each `register_X_callback` function takes a `&AppWindow` +
//! `&Arc<AppState>` and wires the matching `ui.on_X(...)` closure.
//! Bodies + comments are preserved verbatim from the original
//! `mod.rs` (R37a spec: preserve all comments + bodies).
//!
//! Note: the setup line `Arc::clone(&app_state)` is rewritten to
//! `Arc::clone(app_state)` to match the `&Arc<AppState>` parameter;
//! semantics are identical (clone the Arc, no behavior change).

use super::callbacks_settings::{load_app_settings_quiet, save_app_settings_quiet};
use super::error_banners::{set_banner_message, set_inline_error, set_input_error, set_session_error};
use super::log::log_debug_event;
use super::sessions::{build_messages_model, refresh_messages_ui, refresh_sessions_ui};
use super::skills::refresh_skills_ui;
use super::slint_glue::AppWindow;
use super::state::{AppState, SessionMeta};
use slint::{ComponentHandle, SharedString};
use std::sync::Arc;

pub(super) fn register_send_message_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- send-message callback ---
    let app_state_arc_send = std::sync::Arc::clone(&app_state);
    let ui_weak = ui.as_weak();
    ui.on_send_message(move |text| {
        let text_str = text.to_string();
        if text_str.trim().is_empty() {
            return;
        }
        // Phase H (mode_routing): record the user's submission so
        // manual tests can grep "what was sent" + which mode is in
        // effect when the dispatch lands. We truncate the message to
        // 80 chars in the data field to keep log lines scannable.
        let truncated: String = text_str.chars().take(80).collect();
        log_debug_event(
            northhing_core::infrastructure::debug_log::COMP_MODE_ROUTING,
            "app_state::on_send_message:enter",
            crate::flags::DEFAULT_MODE_ID,
            "user submitted text",
            Some([
                ("len", text_str.chars().count().to_string()),
                ("preview", truncated),
                ("mode", crate::flags::DEFAULT_MODE_ID.to_string()),
                ("", String::new()),
            ]),
        );

        // Phase I.x (2026-06-20, A3): minimal landing for the actor
        // runtime. When `USE_LIGHTWEIGHT_ACTOR = true` AND the runtime
        // was constructed at app boot, spawn a one-shot `DispatchActor`
        // that records the event end-to-end. The `ActorTicked`
        // telemetry reaches the same telemetry sink the heartbeat
        // actor uses, proving the runtime is reachable from the
        // production on_send_message path.
        //
        // This is a *demonstration* wiring, not a replacement: the
        // existing `coordinator.start_dialog_turn` path still runs
        // (the actor doesn't suppress it). A3 is the smallest landing
        // that proves the wiring works end-to-end; A1/A2 (multi-turn
        // redesign or a parallel LongRunningSubagent path) are out
        // of MVP scope.
        if northhing_agent_dispatch::USE_LIGHTWEIGHT_ACTOR {
            if let Some(runtime) = app_state_arc_send.actor_runtime() {
                let msg_id = format!("dispatch-{}", text_str.len());
                // Recompute the preview rather than cloning — `truncated`
                // was already moved into the on_send_message:enter
                // log line above. Cheap: text is already capped at 80
                // chars at the user-input boundary.
                let preview: String = text_str.chars().take(80).collect();
                let mode = crate::flags::DEFAULT_MODE_ID.to_string();
                runtime.spawn_one_shot(move |ctx| {
                    // The skill actor body is a no-op beyond the
                    // structured log + telemetry emit; the point is
                    // to prove the runtime path runs in production.
                    log_debug_event(
                        northhing_core::infrastructure::debug_log::COMP_ACTOR_RUNTIME,
                        "actor::dispatch:tick",
                        &mode,
                        "one-shot dispatch actor ticked",
                        Some([
                            ("actor", msg_id.clone()),
                            ("preview", preview.clone()),
                            ("", String::new()),
                            ("", String::new()),
                        ]),
                    );
                    ctx.telemetry
                        .emit(northhing_agent_dispatch::TelemetryEvent::ActorTicked { id: msg_id.clone() });
                    Ok(Some(northhing_agent_dispatch::ActorOutput::Silent))
                });
            }
        }

        let app_state = &*app_state_arc_send;
        let Some(_system) = app_state.get_agentic_system() else {
            if let Some(ui) = ui_weak.upgrade() {
                set_session_error(&ui, "Agentic system not initialized. Please restart.");
            }
            return;
        };

        let session_id = app_state.get_current_session_id();
        if session_id.is_empty() {
            if let Some(ui) = ui_weak.upgrade() {
                set_input_error(&ui, "No session selected. Please create or select a session first.");
            }
            return;
        };

        // A7: mark this session as streaming so the UI shows the indicator
        app_state.set_streaming_session(Some(session_id.clone()));

        let ui_clone = ui_weak.clone();
        let sid = session_id.clone();
        let app_state_for_spawn = Arc::clone(&app_state_arc_send);
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for UI callback");
            rt.block_on(async move {
                let app_state = &*app_state_for_spawn;
                let Some(coordinator) = northhing_core::agentic::coordination::global_coordinator() else {
                    if let Some(ui) = ui_clone.upgrade() {
                        set_session_error(&ui, "Global coordinator not available.");
                    }
                    app_state.set_streaming_session(None);
                    return;
                };

                let workspace = std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string());

                let result = coordinator
                    .start_dialog_turn(
                        sid.clone(),
                        text_str,
                        None,
                        None,
                        crate::flags::DEFAULT_MODE_ID.to_string(),
                        Some(workspace),
                        northhing_core::agentic::coordination::DialogSubmissionPolicy::for_source(
                            northhing_core::agentic::coordination::DialogTriggerSource::DesktopApi,
                        ),
                        None,
                    )
                    .await;

                if let Err(e) = result {
                    if let Some(ui) = ui_clone.upgrade() {
                        set_session_error(&ui, format!("Failed to send message: {e}"));
                    }
                    app_state.set_streaming_session(None);
                    return;
                }

                // turn 已 spawn，流式状态由 event bridge 管理
                // (DialogTurnStarted sets streaming; terminal events clear it)

                // Refresh messages after response completes
                if let Some(ui) = ui_clone.upgrade() {
                    let sid_clone = sid.clone();
                    let ui_weak2 = ui.as_weak();
                    // 2026-06-26 (review follow-up #2): dispatch the
                    // entire refresh onto the event loop thread. The
                    // original code reached `ui.set_messages(...)` from
                    // a non-UI thread (a `std::thread::spawn` →
                    // `rt2.block_on` → `ui_weak2.upgrade()` chain),
                    // which Slint 1.16 silently drops. `ModelRc` is
                    // `!Send` (it's `Rc`-backed), so the model cannot
                    // be moved across thread boundaries. The fix is to
                    // fetch the data and build the model INSIDE the
                    // dispatched closure, on the UI thread, by spinning
                    // up a fresh current-thread runtime there. The
                    // outer `std::thread::spawn` is no longer needed
                    // and is removed. The model fetch is a single
                    // session read (fast), so the brief UI freeze is
                    // acceptable. Same root cause as `bff005a` /
                    // `748f628` / `5b7deeb`. Cleanup review's
                    // observation #1.
                    let _ = slint::invoke_from_event_loop(move || {
                        let Some(ui) = ui_weak2.upgrade() else {
                            return;
                        };
                        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
                        if let Ok(rt) = rt {
                            let _ = rt.block_on(async {
                                if let Some(c) = northhing_core::agentic::coordination::global_coordinator() {
                                    if let Ok(msgs) = c.get_messages(&sid_clone).await {
                                        ui.set_messages(build_messages_model(&msgs, None));
                                    }
                                }
                            });
                        }
                    });
                }
            });
        });
    });
}

pub(super) fn register_new_session_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- new-session callback ---
    let app_state_arc2 = std::sync::Arc::clone(&app_state);
    let ui_weak3 = ui.as_weak();
    ui.on_new_session(move || {
        // Phase H: log the entry so manual tests can confirm the
        // callback fired. The session id is filled in below by
        // coordinator.create_session — this log line only carries the
        // timestamp + mode so we can correlate with later events.
        log_debug_event(
            northhing_core::infrastructure::debug_log::COMP_SESSION_LIFECYCLE,
            "app_state::on_new_session:enter",
            crate::flags::DEFAULT_MODE_ID,
            "user clicked + (new session)",
            None,
        );
        let app_state = &*app_state_arc2;
        let Some(_system) = app_state.get_agentic_system() else {
            if let Some(ui) = ui_weak3.upgrade() {
                set_session_error(&ui, "Agentic system not initialized. Please restart.");
            }
            return;
        };

        let ui_clone = ui_weak3.clone();
        // Phase I.2 (2026-06-20): move a clone of the Arc into the
        // spawn closure so the inner `async move` block can borrow
        // `app_state` with `'static` lifetime (which `std::thread::spawn`
        // requires). Without this, the rebind `app_state` above is
        // bound to the outer Slint closure's `'1` lifetime.
        let app_state_for_spawn = Arc::clone(&app_state_arc2);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for UI callback");
            rt.block_on(async move {
                let app_state = &*app_state_for_spawn;
                let Some(coordinator) = northhing_core::agentic::coordination::global_coordinator() else {
                    if let Some(ui) = ui_clone.upgrade() {
                        set_session_error(&ui, "Global coordinator not available.");
                    }
                    return;
                };

                let workspace = std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string());

                let session_name = format!("Session {}", chrono::Local::now().format("%Y-%m-%d %H:%M"));

                // 2026-06-26 (Phase 5): keep a clone of the workspace
                // path so we can record session metadata for the
                // Q6/Q7 integrity check. The `config` below takes
                // ownership of `workspace` (a `String`).
                let workspace_path_for_meta = std::path::PathBuf::from(&workspace);

                let config = northhing_core::agentic::core::SessionConfig {
                    workspace_path: Some(workspace),
                    ..Default::default()
                };

                match coordinator
                    .create_session(session_name, crate::flags::DEFAULT_MODE_ID.to_string(), config)
                    .await
                {
                    Ok(session) => {
                        let sid = session.session_id.clone();
                        app_state.set_current_session_id(sid.clone());
                        app_state.set_load_more_cursor(None); // Reset pagination for new session

                        // 2026-06-26 (Phase 5): record session metadata
                        // so `validate_session_integrity` can detect
                        // Q6/Q7 issues in the live wire-up. Provider id
                        // comes from the current default_model; empty
                        // when no default is set (will still report
                        // Q7 issues but not Q6).
                        let provider_id = match load_app_settings_quiet().await {
                            Ok(s) => s.resolve_default_model().map(|m| m.provider_id).unwrap_or_default(),
                            Err(_) => String::new(),
                        };
                        app_state.record_session_meta(
                            sid.clone(),
                            SessionMeta {
                                provider_id,
                                workspace_path: workspace_path_for_meta,
                            },
                        );

                        if let Some(ui) = ui_clone.upgrade() {
                            ui.set_current_session_id(SharedString::from(sid.clone()));
                            // Refresh sessions and messages
                            refresh_sessions_ui(&ui, &sid).await;
                            refresh_messages_ui(&ui, &sid, None).await;
                        }
                    }
                    Err(e) => {
                        if let Some(ui) = ui_clone.upgrade() {
                            set_session_error(&ui, format!("Failed to create session: {e}"));
                        }
                    }
                }
            });
        });
    });
}

pub(super) fn register_switch_session_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- switch-session callback ---
    let app_state_arc4 = std::sync::Arc::clone(&app_state);
    let ui_weak4 = ui.as_weak();
    ui.on_switch_session(move |session_id| {
        let sid_str = session_id.to_string();
        log_debug_event(
            northhing_core::infrastructure::debug_log::COMP_SESSION_LIFECYCLE,
            "app_state::on_switch_session:enter",
            crate::flags::DEFAULT_MODE_ID,
            "user clicked sidebar session",
            Some([
                ("session_id", sid_str.clone()),
                ("", String::new()),
                ("", String::new()),
                ("", String::new()),
            ]),
        );

        let app_state = &*app_state_arc4;
        app_state.set_current_session_id(sid_str.clone());
        app_state.set_load_more_cursor(None); // Reset pagination on session switch

        if let Some(ui) = ui_weak4.upgrade() {
            ui.set_current_session_id(SharedString::from(sid_str.clone()));
            // Refresh messages for the switched session
            let ui_weak_msg = ui.as_weak();
            let sid_clone = sid_str;
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build tokio runtime");
                rt.block_on(async move {
                    if let Some(ui) = ui_weak_msg.upgrade() {
                        refresh_messages_ui(&ui, &sid_clone, None).await;
                    }
                });
            });
        }
    });
}

pub(super) fn register_delete_session_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- delete-session callback ---
    let app_state_arc5 = std::sync::Arc::clone(&app_state);
    let ui_weak5 = ui.as_weak();
    ui.on_delete_session(move |session_id| {
        let sid_str = session_id.to_string();
        log_debug_event(
            northhing_core::infrastructure::debug_log::COMP_SESSION_LIFECYCLE,
            "app_state::on_delete_session:enter",
            crate::flags::DEFAULT_MODE_ID,
            "user deleted session",
            Some([
                ("session_id", sid_str.clone()),
                ("", String::new()),
                ("", String::new()),
                ("", String::new()),
            ]),
        );
        let app_state = &*app_state_arc5;
        let Some(_system) = app_state.get_agentic_system() else {
            return;
        };

        let sid_str = session_id.to_string();
        let ui_clone = ui_weak5.clone();
        let current_sid = app_state.get_current_session_id();
        // Phase I.2: see note in on_new_session — Arc clone into spawn.
        let app_state_for_spawn = Arc::clone(&app_state_arc5);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for UI callback");
            rt.block_on(async move {
                let app_state = &*app_state_for_spawn;
                let Some(coordinator) = northhing_core::agentic::coordination::global_coordinator() else {
                    return;
                };

                let workspace = std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string());

                match coordinator
                    .delete_session(std::path::Path::new(&workspace), &sid_str)
                    .await
                {
                    Ok(_) => {
                        // If we deleted the current session, clear it
                        if current_sid == sid_str {
                            app_state.set_current_session_id(String::new());
                        }
                        // 2026-06-26 (Phase 5): drop the session's
                        // metadata so the integrity check doesn't
                        // report stale issues for it.
                        app_state.forget_session_meta(&sid_str);

                        if let Some(ui) = ui_clone.upgrade() {
                            // 2026-07-18 (D2b fix): clear current-session
                            // Slint properties via event loop when the
                            // deleted session was the active one.
                            let was_current = current_sid == sid_str;
                            let ui_weak_clear = ui.as_weak();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ui) = ui_weak_clear.upgrade() {
                                    if was_current {
                                        ui.set_current_session_id(SharedString::from(""));
                                        ui.set_current_session_name(SharedString::from(""));
                                    }
                                }
                            });
                            refresh_sessions_ui(&ui, "").await;
                        }
                    }
                    Err(e) => {
                        if let Some(ui) = ui_clone.upgrade() {
                            set_session_error(&ui, format!("Failed to delete session: {e}"));
                        }
                    }
                }
            });
        });
    });
}

pub(super) fn register_toggle_theme_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- toggle-theme callback ---
    let ui_weak6 = ui.as_weak();
    ui.on_toggle_theme(move || {
        if let Some(ui) = ui_weak6.upgrade() {
            let current = ui.get_dark_mode();
            ui.set_dark_mode(!current);
        }
    });
}

pub(super) fn register_toggle_show_subagents_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- toggle-show-subagents callback (Phase G.3) ---
    // Flips the AppState's `show_subagents` flag and updates the Slint
    // property so the sidebar re-renders. No async work — the tree
    // visibility is computed by the Slint `for` filter inline.
    let app_state_arc_show = std::sync::Arc::clone(&app_state);
    let ui_weak_show = ui.as_weak();
    ui.on_toggle_show_subagents(move || {
        if let Some(ui) = ui_weak_show.upgrade() {
            // SAFETY: AppState outlives the UI in this app — the runtime
            // owns both, and `app_state` is dropped only after the UI
            // loop exits. This matches the convention used by every
            // other `ui.on_*` callback in this file (see `on_toggle_theme`
            // and `on_toggle_skill` above).
            let state = &*app_state_arc_show;
            let mut flag = state.show_subagents_handle().lock();
            *flag = !*flag;
            ui.set_show_subagents(*flag);
        }
    });
}

pub(super) fn register_toggle_skill_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- toggle-skill callback ---
    let app_state_arc7 = std::sync::Arc::clone(&app_state);
    let ui_weak7 = ui.as_weak();
    ui.on_toggle_skill(move |skill_name| {
        let skill_name_str = skill_name.to_string();
        log_debug_event(
            northhing_core::infrastructure::debug_log::COMP_SKILL_PANEL,
            "app_state::on_toggle_skill:enter",
            crate::flags::DEFAULT_MODE_ID,
            "user toggled skill",
            Some([("skill", skill_name_str.clone()), ("mode", crate::flags::DEFAULT_MODE_ID.to_string()), ("", String::new()), ("", String::new())]),
        );
        let app_state = &*app_state_arc7;
        let Some(_system) = app_state.get_agentic_system() else {
            if let Some(ui) = ui_weak7.upgrade() {
                set_session_error(&ui, "Agentic system not initialized. Please restart.");
            }
            return;
        };
        let ui_clone = ui_weak7.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for toggle-skill callback");
            rt.block_on(async move {
                let skill = match northhing_core::agentic::tools::implementations::skills::skill_registry()
                    .get_all_skills()
                    .await
                    .into_iter()
                    .find(|s| s.key == skill_name_str)
                {
                    Some(s) => s,
                    None => {
                        // Phase I.6: structured log instead of eprintln.
                        log_debug_event(
                            northhing_core::infrastructure::debug_log::COMP_SKILL_PANEL,
                            "app_state::on_toggle_skill:not_found",
                            crate::flags::DEFAULT_MODE_ID,
                            "skill not found",
                            Some([("skill", skill_name_str.clone()), ("", String::new()), ("", String::new()), ("", String::new())]),
                        );
                        return;
                    }
                };

                let default_enabled =
                    northhing_core::agentic::tools::implementations::skills::resolver::resolve_skill_default_enabled_for_mode(
                        &skill,
                        crate::flags::DEFAULT_MODE_ID,
                    );
                let new_enabled = !default_enabled;

                if let Err(e) = northhing_core::agentic::tools::implementations::skills::mode_overrides::set_user_mode_skill_state(
                    crate::flags::DEFAULT_MODE_ID,
                    &skill_name_str,
                    new_enabled,
                    default_enabled,
                )
                .await
                {
                    // Phase I.6: structured log instead of eprintln.
                    log_debug_event(
                        northhing_core::infrastructure::debug_log::COMP_SKILL_PANEL,
                        "app_state::on_toggle_skill:error",
                        crate::flags::DEFAULT_MODE_ID,
                        "set_user_mode_skill_state failed",
                        Some([("skill", skill_name_str.clone()), ("error", format!("{e}")), ("", String::new()), ("", String::new())]),
                    );
                    return;
                }

                // Refresh the session list to reflect the change
                if let Some(ui) = ui_clone.upgrade() {
                    refresh_sessions_ui(&ui, "").await;
                    // Phase C.4: also refresh the Inspector skills model so
                    // the `●` badge reflects the new enabled state. Without
                    // this the toggle would persist but the UI wouldn't
                    // re-render until the next manual reload.
                    refresh_skills_ui(&ui).await;
                }

                // Phase I.6: structured log of the result so manual
                // tests can grep the toggle outcome. `new_enabled`
                // already reflects the post-toggle state.
                log_debug_event(
                    northhing_core::infrastructure::debug_log::COMP_SKILL_PANEL,
                    "app_state::on_toggle_skill:result",
                    crate::flags::DEFAULT_MODE_ID,
                    "skill toggle persisted",
                    Some([
                        ("skill", skill_name_str.clone()),
                        ("new_state", if new_enabled { "enabled" } else { "disabled" }.to_string()),
                        ("mode", crate::flags::DEFAULT_MODE_ID.to_string()),
                        ("", String::new()),
                    ]),
                );
            });
        });
    });
}

pub(super) fn register_load_more_messages_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- load-more-messages callback ---
    let app_state_arc8 = std::sync::Arc::clone(&app_state);
    let ui_weak8 = ui.as_weak();
    ui.on_load_more_messages(move || {
        let app_state = &*app_state_arc8;
        let session_id = app_state.get_current_session_id();
        if session_id.is_empty() {
            return;
        }
        let ui_clone = ui_weak8.clone();
        let sid = session_id.clone();
        // Phase I.2: see note in on_new_session — Arc clone into spawn.
        let app_state_for_spawn = Arc::clone(&app_state_arc8);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for load-more-messages");
            rt.block_on(async move {
                let app_state = &*app_state_for_spawn;
                let cursor = app_state.get_load_more_cursor();
                let limit = 50usize;

                let Some(coordinator) = northhing_core::agentic::coordination::global_coordinator() else {
                    return;
                };

                let result = coordinator.get_messages_paginated(&sid, limit, cursor.as_deref()).await;

                match result {
                    Ok((messages, _has_more)) => {
                        // Update cursor from the oldest message in this batch
                        let cursor_id = messages.last().map(|m| m.id.clone());
                        app_state.set_load_more_cursor(cursor_id);

                        // Reload full message list to get proper ordering
                        if let Ok(all_msgs) = coordinator.get_messages(&sid).await {
                            if let Some(ui) = ui_clone.upgrade() {
                                let model = build_messages_model(&all_msgs, None);
                                ui.set_messages(model);
                            }
                        }
                    }
                    Err(e) => {
                        if let Some(ui) = ui_clone.upgrade() {
                            set_session_error(&ui, format!("Failed to load more messages: {e}"));
                        }
                    }
                }
            });
        });
    });
}

pub(super) fn register_refresh_sessions_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- refresh-sessions callback ---
    let app_state_arc9 = std::sync::Arc::clone(&app_state);
    let ui_weak9 = ui.as_weak();
    ui.on_refresh_sessions(move || {
        let app_state = &*app_state_arc9;
        let Some(_system) = app_state.get_agentic_system() else {
            return;
        };
        let ui_clone = ui_weak9.clone();
        let current_session = app_state.get_current_session_id();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build runtime for refresh-sessions");
            let current_session = current_session;
            rt.block_on(async move {
                if let Some(ui) = ui_clone.upgrade() {
                    refresh_sessions_ui(&ui, &current_session).await;
                }
            });
        });
    });
}

pub(super) fn register_refresh_messages_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- refresh-messages callback ---
    let app_state_arc10 = std::sync::Arc::clone(&app_state);
    let ui_weak10 = ui.as_weak();
    ui.on_refresh_messages(move || {
        let app_state = &*app_state_arc10;
        let session_id = app_state.get_current_session_id();
        if session_id.is_empty() {
            return;
        }
        let ui_clone = ui_weak10.clone();
        let sid = session_id.clone();
        // Phase I.2: see note in on_new_session — Arc clone into spawn.
        let app_state_for_spawn = Arc::clone(&app_state_arc10);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for refresh-messages");
            rt.block_on(async move {
                let app_state = &*app_state_for_spawn;
                app_state.set_load_more_cursor(None); // Reset pagination on full refresh
                if let Some(ui) = ui_clone.upgrade() {
                    refresh_messages_ui(&ui, &sid, app_state.get_streaming_session().as_deref()).await;
                }
            });
        });
    });
}

pub(super) fn register_clear_session_error_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- P0-C: clear-error callbacks (banner × button) ---
    let ui_weak_clear_sess = ui.as_weak();
    ui.on_clear_session_error(move || {
        if let Some(ui) = ui_weak_clear_sess.upgrade() {
            ui.set_session_error(SharedString::from(String::new()));
        }
    });
}

pub(super) fn register_clear_input_error_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    let ui_weak_clear_in = ui.as_weak();
    ui.on_clear_input_error(move || {
        if let Some(ui) = ui_weak_clear_in.upgrade() {
            ui.set_input_error(SharedString::from(String::new()));
        }
    });
}

pub(super) fn register_dismiss_banner_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- 2026-06-26 (Phase 5): banner / inline-error clear callbacks ---
    // Q8=c dual channel: banner auto-dismisses after 5s (handled by
    // `schedule_error_clear`); inline error stays until the user clicks
    // ×. Both routes call the matching `set_*("")` to clear.
    let ui_weak_dismiss_banner = ui.as_weak();
    ui.on_dismiss_banner(move || {
        if let Some(ui) = ui_weak_dismiss_banner.upgrade() {
            ui.set_banner_message(SharedString::from(String::new()));
            ui.set_banner_detail(SharedString::from(String::new()));
        }
    });
}

pub(super) fn register_clear_inline_error_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    let ui_weak_clear_inline = ui.as_weak();
    ui.on_clear_inline_error(move || {
        if let Some(ui) = ui_weak_clear_inline.upgrade() {
            ui.set_chat_inline_error(SharedString::from(String::new()));
        }
    });
}

pub(super) fn register_stop_streaming_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- stop-streaming callback (C6=c) ---
    let app_state_arc_stop = std::sync::Arc::clone(app_state);
    let ui_weak_stop = ui.as_weak();
    ui.on_stop_streaming(move || {
        let app_state = &*app_state_arc_stop;
        let session_id = app_state.get_current_session_id();
        let active_turn = app_state.get_active_turn_id();

        let Some(turn_id) = active_turn else {
            if let Some(ui) = ui_weak_stop.upgrade() {
                set_inline_error(&ui, "当前没有正在运行的回复");
            }
            return;
        };

        let ui_clone = ui_weak_stop.clone();
        let sid = session_id.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for stop-streaming");
            rt.block_on(async move {
                let Some(coordinator) = northhing_core::agentic::coordination::global_coordinator()
                else {
                    if let Some(ui) = ui_clone.upgrade() {
                        set_session_error(&ui, "Global coordinator not available.");
                    }
                    return;
                };
                if let Err(e) = coordinator.cancel_dialog_turn(&sid, &turn_id).await {
                    if let Some(ui) = ui_clone.upgrade() {
                        set_session_error(&ui, format!("停止失败: {e}"));
                    }
                }
                // On success, DialogTurnCancelled event cleans up the UI.
            });
        });
    });
}

// 2026-07-18 (D2b): rename-session callback. Spawns a thread, calls
// coordinator.update_session_title, then refreshes the sessions UI and
// updates the current-session-name if the renamed session is the active one.
//
// 2026-07-18 (D2b fix): the current-session id is re-read inside the
// event-loop closure (not captured before the spawn) so that a user who
// switches sessions during the async rename does not get their state
// overwritten by a stale value.
pub(super) fn register_rename_session_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    let app_state_arc = std::sync::Arc::clone(app_state);
    let ui_weak = ui.as_weak();
    ui.on_rename_session(move |session_id, new_name| {
        let sid = session_id.to_string();
        let name = new_name.to_string();
        // 2026-07-18 (D2b fix): clone the Arc into the spawn so the
        // outer closure remains FnMut-callable across clicks.
        let app_state_for_spawn = Arc::clone(&app_state_arc);
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!(
                        target: "app_state",
                        "rename-session: failed to build runtime: {e}"
                    );
                    return;
                }
            };
            rt.block_on(async move {
                let Some(coordinator) = northhing_core::agentic::coordination::global_coordinator() else {
                    if let Some(ui) = ui_weak.upgrade() {
                        set_session_error(&ui, "Global coordinator not available.");
                    }
                    return;
                };
                match coordinator.update_session_title(&sid, &name).await {
                    Ok(normalized) => {
                        if let Some(ui) = ui_weak.upgrade() {
                            let ui_weak2 = ui.as_weak();
                            let sid_for_dispatch = sid.clone();
                            let normalized_for_dispatch = normalized.clone();
                            // 2026-07-18 (D2b fix): move the Arc into the
                            // event-loop closure so it outlives the block_on
                            // scope; re-read current session id at dispatch.
                            let app_state_in_closure = Arc::clone(&app_state_for_spawn);
                            let _ = slint::invoke_from_event_loop(move || {
                                let Some(ui) = ui_weak2.upgrade() else {
                                    return;
                                };
                                // refresh_sessions_ui is async — drive it with a
                                // fresh current-thread runtime on the UI thread.
                                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
                                if let Ok(rt) = rt {
                                    let current_now = app_state_in_closure.get_current_session_id();
                                    let _ = rt.block_on(refresh_sessions_ui(&ui, &current_now));
                                    if sid_for_dispatch == current_now {
                                        ui.set_current_session_name(SharedString::from(normalized_for_dispatch));
                                    }
                                }
                            });
                        }
                    }
                    Err(e) => {
                        if let Some(ui) = ui_weak.upgrade() {
                            set_session_error(&ui, format!("Failed to rename session: {e}"));
                        }
                    }
                }
            });
        });
    });
}
