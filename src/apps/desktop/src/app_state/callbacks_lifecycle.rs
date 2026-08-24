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

use super::error_banners::{set_banner_message, set_inline_error, set_input_error, set_session_error};
use super::log::log_debug_event;
use super::sessions::{build_messages_model, refresh_messages_ui, refresh_sessions_ui};
use super::skills::refresh_skills_ui;
use super::slint_glue::AppWindow;
use super::state::{AppState, SessionMeta};
use super::streaming_lifecycle::{
    enter_turn, reset_after_submit_failure, streaming_dispatcher, StreamingStateDispatcher,
};
use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::session::{KernelSessionApi, MessageContentDto, MessageRoleDto, SessionConfigDto};
use northhing_kernel_api::turn::{
    DialogSubmitOutcomeKindDto, KernelTurnApi, SubmissionPolicyDto, TriggerSourceDto, TurnInputDto,
};
use northhing_kernel_api::KernelAgentsApi;
use northhing_kernel_api::KernelSettingsApi;
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
            northhing_debug_log::COMP_MODE_ROUTING,
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

        let app_state = &*app_state_arc_send;
        let Some(_system) = app_state.get_agentic_system() else {
            // 2026-07-18 (D2j): UI thread — pass weak directly; helper upgrades on UI thread.
            set_session_error(ui_weak.clone(), "Agentic system not initialized. Please restart.");
            return;
        };

        let session_id = app_state.get_current_session_id();
        if session_id.is_empty() {
            // 2026-07-18 (D2j): UI thread — pass weak directly; helper upgrades on UI thread.
            set_input_error(
                ui_weak.clone(),
                "No session selected. Please create or select a session first.",
            );
            return;
        };

        // 2026-07-27 (K4a R3, Bug A — fix #1 + fix #2): mark this
        // session as streaming and pre-flight the turn into the
        // helper BEFORE we touch the turn runtime. `enter_turn`
        // sets `active_turn_id` and dispatches `is-streaming=true`
        // in one shot — that way the stop button is up before the
        // broadcast router's DialogTurnStarted event has a chance
        // to lag. The event bridge's `Started` handler routes
        // through the same helper, so the turn-generation guard
        // keeps a duplicate `enter_turn` call from clobbering an
        // already-fresh generation.
        //
        // The dispatcher is installed by
        // `register_desktop_event_bridge` (process-global
        // `OnceLock`); if the submit path runs before the bridge
        // is registered (shouldn't happen in production, but
        // could in unit tests), we fall back to direct AppState
        // mutation — the bridge will pick up the state on its
        // first `Started` event.
        let dispatcher: Arc<dyn StreamingStateDispatcher> = match streaming_dispatcher() {
            Some(d) => d,
            None => {
                tracing::warn!(
                    target: "app_state",
                    "send_message: no streaming_dispatcher installed; \
                     is-streaming will not be set until the bridge's first Started event"
                );
                return;
            }
        };

        // A7: mark this session as streaming so the UI shows the
        // indicator.
        app_state.set_streaming_session(Some(session_id.clone()));

        // We pre-flight a `started` turn with a synthetic id; the
        // event bridge's `Started` event replaces it with the
        // real turn id via the same `enter_turn` helper.
        //
        // 2026-07-27 (K4a R3, fix #3): the returned
        // `optimistic_turn_id` is captured and used as the
        // generation token for the failure path below. If a
        // fresh `enter_turn` (e.g. the user sent a second
        // message, or the bridge's `Started` for a queued
        // turn advanced the generation) lands before our
        // submit_turn returns, the failure path's
        // `reset_after_submit_failure(&token)` no-ops
        // because the active_turn_id no longer matches.
        let optimistic_turn_id = format!("submit-pending-{}", uuid::Uuid::new_v4());
        let generation_token = enter_turn(&*dispatcher, app_state, &session_id, &optimistic_turn_id);

        let ui_clone = ui_weak.clone();
        let sid = session_id.clone();
        let app_state_for_spawn = Arc::clone(&app_state_arc_send);
        let app_state = &*app_state_arc_send;
        let dispatcher_for_failure = dispatcher.clone();
        let generation_token_for_failure = generation_token.clone();
        let Some(turn_rt) = super::turn_runtime::turn_runtime() else {
            // 2026-07-19 (W4): turn runtime missing — cannot dispatch without
            // aborting the turn. Surface the error instead of hanging silently.
            set_session_error(ui_clone.clone(), "Turn runtime not initialized. Please restart.");
            // 2026-07-27 (K4a R3, Bug A — fix #1 + fix #3):
            // tear down the optimistic streaming state we
            // set above so the stop button doesn't stay
            // visible after a failed submit. We pass the
            // generation token so the reset is a no-op when
            // a fresh `enter_turn` already superseded the
            // pre-flight.
            reset_after_submit_failure(&*dispatcher_for_failure, app_state, &generation_token_for_failure);
            return;
        };
        // Move the generation token into the spawn closure so
        // the post-submit failure path can pass it to
        // `reset_after_submit_failure`. We don't use
        // `generation_token_for_failure` here because the spawn
        // closure owns its own copy (and we already
        // `Clone`d before the move).
        let generation_token_for_spawn = generation_token;
        turn_rt.spawn(async move {
            let app_state = &*app_state_for_spawn;
            let facade = kernel_facade();

            let workspace = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string());

            // 2026-07-18 (W3a-4): route submissions through the dialog
            // scheduler via the facade's submit_turn so in-turn messages are
            // enqueued (per-session depth cap 20) instead of rejected by the
            // Processing guard. The facade's outcome handler auto-dispatches
            // the next queued turn when the active turn ends.
            let input = TurnInputDto {
                session_id: sid.clone(),
                text: text_str,
                mode: crate::flags::DEFAULT_MODE_ID.to_string(),
                policy: SubmissionPolicyDto {
                    allow_subagent: false,
                    max_turns: None,
                },
                source: TriggerSourceDto::User,
                workspace_path: Some(workspace),
            };
            let submit_ok = match facade.submit_turn(input).await {
                Ok(outcome) if outcome.accepted => {
                    match outcome.outcome_kind {
                        Some(DialogSubmitOutcomeKindDto::Queued) => {
                            // 2026-07-18 (W3a-4): message queued behind the
                            // active turn. Show a banner; keep the streaming
                            // indicator — the current turn is still running.
                            // Turn handoff is收敛 by the event bridge when
                            // the active turn ends and the queued turn starts.
                            // 2026-07-18 (D2j): background thread — pass weak directly.
                            set_banner_message(ui_clone.clone(), "已排队，将在当前回复完成后发送", "");
                            Ok(())
                        }
                        // Started (or unknown) — turn started immediately;
                        // `enter_turn` already pre-flighted the UI; the
                        // event bridge's `Started` handler will replace
                        // the synthetic id with the real one.
                        _ => Ok(()),
                    }
                }
                Ok(outcome) => Err(format!("Failed to send message: {}", outcome.error.unwrap_or_default())),
                Err(e) => Err(format!("Failed to send message: {e}")),
            };

            if let Err(e) = submit_ok {
                // 2026-07-18 (D2j): background thread — pass weak directly; helper upgrades on UI thread.
                set_session_error(ui_clone.clone(), e);
                // 2026-07-27 (K4a R3, Bug A — fix #1 + fix #3):
                // the pre-fix comment claimed the event bridge
                // owned the streaming lifecycle, so submit
                // failure would naturally tear the UI back down.
                // In practice that is wrong when the bridge
                // never saw a `Started` for this generation
                // (e.g. submit_turn returned Err before
                // scheduling any turn) — `is-streaming` would
                // stay stuck at `true` and the stop button
                // would never come back down. The fix routes
                // through the same helper the event bridge's
                // terminal handlers use so AppState and the
                // Slint root are always toggled together.
                //
                // fix #3: the generation token is passed
                // through. If a fresh `enter_turn` (the
                // bridge's `Started` for a queued turn, or
                // a second submit pre-flight) advanced the
                // generation while this submit_turn was in
                // flight, the reset no-ops and the fresh
                // turn's stop button stays visible. The pre-fix
                // unconditional reset would have wiped it.
                let dispatcher_for_failure = streaming_dispatcher().unwrap_or_else(|| dispatcher_for_failure.clone());
                reset_after_submit_failure(&*dispatcher_for_failure, app_state, &generation_token_for_spawn);
                return;
            }

            // Refresh messages after response completes.
            // 2026-07-18 (D2j-fix): background thread fetches messages,
            // then dispatches model build + set onto the UI thread via
            // invoke_from_event_loop. ModelRc is !Send, so messages are
            // moved into the closure and the model is built inside (on
            // the UI thread). No nested block_on inside the invoke
            // closure (would panic: "Cannot start a runtime from within
            // a runtime").
            let sid_clone = sid.clone();
            let ui_weak2 = ui_clone.clone();
            let facade_for_msgs = kernel_facade();
            match facade_for_msgs.get_messages(&sid_clone).await {
                Ok(msgs) => {
                    let ui_weak_for_msgs = ui_weak2.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak_for_msgs.upgrade() {
                            let model = build_messages_model(&msgs, None);
                            ui.set_messages(model);
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        target: "app_state",
                        "send_message: failed to refresh messages: {e}"
                    );
                }
            }
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
            northhing_debug_log::COMP_SESSION_LIFECYCLE,
            "app_state::on_new_session:enter",
            crate::flags::DEFAULT_MODE_ID,
            "user clicked + (new session)",
            None,
        );
        let app_state = &*app_state_arc2;
        let Some(_system) = app_state.get_agentic_system() else {
            // 2026-07-18 (D2j): UI thread — pass weak directly; helper upgrades on UI thread.
            set_session_error(ui_weak3.clone(), "Agentic system not initialized. Please restart.");
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
                let facade = kernel_facade();

                let workspace = std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string());

                let session_name = format!("Session {}", chrono::Local::now().format("%Y-%m-%d %H:%M"));

                // 2026-06-26 (Phase 5): keep a clone of the workspace
                // path so we can record session metadata for the
                // Q6/Q7 integrity check. The `config` below takes
                // ownership of `workspace` (a `String$).
                let workspace_path_for_meta = std::path::PathBuf::from(&workspace);

                let config = SessionConfigDto {
                    workspace_path: Some(workspace),
                    agent_type: crate::flags::DEFAULT_MODE_ID.to_string(),
                    model_name: String::new(),
                    name: Some(session_name),
                };

                match facade.create_session(config).await {
                    Ok(sid) => {
                        app_state.set_current_session_id(sid.clone());
                        app_state.set_load_more_cursor(None); // Reset pagination for new session

                        // Record session metadata for session integrity validation.
                        let cfg = facade.get_global_config().await.ok();
                        let provider_id = cfg.and_then(|c| c.default_provider_id).unwrap_or_default();
                        app_state.record_session_meta(
                            sid.clone(),
                            SessionMeta {
                                provider_id,
                                workspace_path: workspace_path_for_meta,
                            },
                        );

                        // 2026-07-18 (D2j): background thread — set_current_session_id
                        // must run on UI thread; refresh functions take weak and
                        // dispatch internally.
                        let ui_weak_for_set = ui_clone.clone();
                        let sid_for_set = sid.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak_for_set.upgrade() {
                                ui.set_current_session_id(SharedString::from(sid_for_set));
                            }
                        });
                        // Refresh sessions and messages — pass weak directly.
                        refresh_sessions_ui(ui_clone.clone(), &sid).await;
                        refresh_messages_ui(ui_clone.clone(), &sid, None).await;
                    }
                    Err(e) => {
                        // 2026-07-18 (D2j): background thread — pass weak directly; helper upgrades on UI thread.
                        set_session_error(ui_clone.clone(), format!("Failed to create session: {e}"));
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
            northhing_debug_log::COMP_SESSION_LIFECYCLE,
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

        // 2026-07-18 (D2j): UI thread — keep upgrade; background thread passes weak.
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
                    // 2026-07-18 (D2j): background thread — pass weak directly; function upgrades on UI thread.
                    refresh_messages_ui(ui_weak_msg.clone(), &sid_clone, None).await;
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
            northhing_debug_log::COMP_SESSION_LIFECYCLE,
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
                let facade = kernel_facade();

                match facade.delete_session(&sid_str).await {
                    Ok(_) => {
                        // If we deleted the current session, clear it
                        if current_sid == sid_str {
                            app_state.set_current_session_id(String::new());
                        }
                        // 2026-06-26 (Phase 5): drop the session's
                        // metadata so the integrity check doesn't
                        // report stale issues for it.
                        app_state.forget_session_meta(&sid_str);

                        // 2026-07-18 (D2j): background thread — clear current-session
                        // Slint properties via event loop when the deleted session
                        // was the active one; refresh_sessions_ui takes weak.
                        let was_current = current_sid == sid_str;
                        let ui_weak_clear = ui_clone.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak_clear.upgrade() {
                                if was_current {
                                    ui.set_current_session_id(SharedString::from(""));
                                    ui.set_current_session_name(SharedString::from(""));
                                }
                            }
                        });
                        refresh_sessions_ui(ui_clone.clone(), "").await;
                    }
                    Err(e) => {
                        // 2026-07-18 (D2j): background thread — pass weak directly; helper upgrades on UI thread.
                        set_session_error(ui_clone.clone(), format!("Failed to delete session: {e}"));
                    }
                }
            });
        });
    });
}

pub(super) fn register_toggle_theme_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    // --- toggle-theme callback ---
    let ui_weak6 = ui.as_weak();
    ui.on_toggle_theme(move || {
        if let Some(ui) = ui_weak6.upgrade() {
            let current = ui.get_dark_mode();
            let new_dark = !current;
            ui.set_dark_mode(new_dark);
            super::block_registry::set_blocks_dark_mode(new_dark);
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
            northhing_debug_log::COMP_SKILL_PANEL,
            "app_state::on_toggle_skill:enter",
            crate::flags::DEFAULT_MODE_ID,
            "user toggled skill",
            Some([
                ("skill", skill_name_str.clone()),
                ("mode", crate::flags::DEFAULT_MODE_ID.to_string()),
                ("", String::new()),
                ("", String::new()),
            ]),
        );
        let app_state = &*app_state_arc7;
        let Some(_system) = app_state.get_agentic_system() else {
            // 2026-07-18 (D2j): UI thread — pass weak directly; helper upgrades on UI thread.
            set_session_error(ui_weak7.clone(), "Agentic system not initialized. Please restart.");
            return;
        };
        let ui_clone = ui_weak7.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for toggle-skill callback");
            rt.block_on(async move {
                let facade = kernel_facade();
                let skill = match facade.get_skill(&skill_name_str).await {
                    Ok(s) => s,
                    Err(_) => {
                        // Phase I.6: structured log instead of eprintln.
                        log_debug_event(
                            northhing_debug_log::COMP_SKILL_PANEL,
                            "app_state::on_toggle_skill:not_found",
                            crate::flags::DEFAULT_MODE_ID,
                            "skill not found",
                            Some([
                                ("skill", skill_name_str.clone()),
                                ("", String::new()),
                                ("", String::new()),
                                ("", String::new()),
                            ]),
                        );
                        return;
                    }
                };

                let default_enabled = facade
                    .resolve_skill_default_enabled(&skill.id, crate::flags::DEFAULT_MODE_ID)
                    .await
                    .unwrap_or(false);
                let new_enabled = !default_enabled;

                let scope = northhing_kernel_api::agents::SkillScopeDto {
                    scope_type: "user".to_string(),
                    workspace_path: None,
                    mode_id: Some(crate::flags::DEFAULT_MODE_ID.to_string()),
                };
                if let Err(e) = facade.set_skill_enabled(&skill_name_str, scope, new_enabled).await {
                    // Phase I.6: structured log instead of eprintln.
                    log_debug_event(
                        northhing_debug_log::COMP_SKILL_PANEL,
                        "app_state::on_toggle_skill:error",
                        crate::flags::DEFAULT_MODE_ID,
                        "set_user_mode_skill_state failed",
                        Some([
                            ("skill", skill_name_str.clone()),
                            ("error", format!("{e}")),
                            ("", String::new()),
                            ("", String::new()),
                        ]),
                    );
                    return;
                }

                // 2026-07-18 (D2j-fix): background thread — both refresh
                // functions take weak and dispatch their own UI sets
                // internally. No invoke_from_event_loop wrapper needed.
                refresh_sessions_ui(ui_clone.clone(), "").await;
                // Phase C.4: also refresh the Inspector skills model so
                // the `●` badge reflects the new enabled state. Without
                // this the toggle would persist but the UI wouldn't
                // re-render until the next manual reload.
                refresh_skills_ui(ui_clone.clone()).await;

                // Phase I.6: structured log of the result so manual
                // tests can grep the toggle outcome. `new_enabled`
                // already reflects the post-toggle state.
                log_debug_event(
                    northhing_debug_log::COMP_SKILL_PANEL,
                    "app_state::on_toggle_skill:result",
                    crate::flags::DEFAULT_MODE_ID,
                    "skill toggle persisted",
                    Some([
                        ("skill", skill_name_str.clone()),
                        (
                            "new_state",
                            if new_enabled { "enabled" } else { "disabled" }.to_string(),
                        ),
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

                let facade = kernel_facade();

                // K4a-T23 (缺口4 adjudication): facade has no paginated API, so
                // fetch the full message Vec and do client-side pagination here.
                // UI behavior is unchanged — the model is still built from the
                // full list, and the cursor tracks the last message of the current
                // page for the next "load more" click.
                match facade.get_messages(&sid).await {
                    Ok(all_msgs) => {
                        // Client-side pagination: find the cursor position and
                        // advance by `limit` messages to compute the new cursor.
                        let new_cursor = match &cursor {
                            Some(cursor_id) => all_msgs.iter().position(|m| &m.id == cursor_id).and_then(|idx| {
                                let end = (idx + 1 + limit).min(all_msgs.len());
                                if idx + 1 < end {
                                    Some(all_msgs[end - 1].id.clone())
                                } else {
                                    None
                                }
                            }),
                            None => {
                                let end = limit.min(all_msgs.len());
                                if end > 0 {
                                    Some(all_msgs[end - 1].id.clone())
                                } else {
                                    None
                                }
                            }
                        };
                        app_state.set_load_more_cursor(new_cursor);

                        // 2026-07-18 (D2j): background thread — fetch messages
                        // (Send-safe Vec), then dispatch model build + set onto
                        // UI thread via invoke_from_event_loop. ModelRc is !Send
                        // so it must be constructed inside the closure.
                        let ui_weak_for_msgs = ui_clone.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak_for_msgs.upgrade() {
                                let model = build_messages_model(&all_msgs, None);
                                ui.set_messages(model);
                            }
                        });
                    }
                    Err(e) => {
                        // 2026-07-18 (D2j): background thread — pass weak directly; helper upgrades on UI thread.
                        set_session_error(ui_clone.clone(), format!("Failed to load more messages: {e}"));
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

        let ui_clone_for_refresh = ui_clone.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build runtime for refresh-sessions");
            let current_session = current_session;
            rt.block_on(async move {
                // 2026-07-18 (D2j): background thread — pass weak directly; function upgrades on UI thread.
                refresh_sessions_ui(ui_clone_for_refresh.clone(), &current_session).await;
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

        let ui_clone_for_refresh = ui_clone.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for refresh-messages");
            rt.block_on(async move {
                let app_state = &*app_state_for_spawn;
                app_state.set_load_more_cursor(None); // Reset pagination on full refresh
                                                      // 2026-07-18 (D2j): background thread — pass weak directly; function upgrades on UI thread.
                refresh_messages_ui(
                    ui_clone_for_refresh.clone(),
                    &sid,
                    app_state.get_streaming_session().as_deref(),
                )
                .await;
            });
        });
    });
}

pub(super) fn register_clear_session_error_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    // --- P0-C: clear-error callbacks (banner × button) ---
    let ui_weak_clear_sess = ui.as_weak();
    ui.on_clear_session_error(move || {
        if let Some(ui) = ui_weak_clear_sess.upgrade() {
            ui.set_session_error(SharedString::from(String::new()));
        }
    });
}

pub(super) fn register_clear_input_error_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    let ui_weak_clear_in = ui.as_weak();
    ui.on_clear_input_error(move || {
        if let Some(ui) = ui_weak_clear_in.upgrade() {
            ui.set_input_error(SharedString::from(String::new()));
        }
    });
}

pub(super) fn register_dismiss_banner_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
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

pub(super) fn register_clear_inline_error_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
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
            // 2026-07-18 (D2j): UI thread — pass weak directly; helper upgrades on UI thread.
            set_inline_error(ui_weak_stop.clone(), "当前没有正在运行的回复");
            return;
        };

        let ui_clone = ui_weak_stop.clone();
        let _sid = session_id.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for stop-streaming");
            rt.block_on(async move {
                let facade = kernel_facade();
                if let Err(e) = facade.stop_turn(&turn_id).await {
                    // 2026-07-18 (D2j): background thread — pass weak directly; helper upgrades on UI thread.
                    set_session_error(ui_clone.clone(), format!("停止失败: {e}"));
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
pub(super) fn register_export_markdown_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- export-markdown callback (C7=b) ---
    let app_state_arc = std::sync::Arc::clone(app_state);
    let ui_weak = ui.as_weak();
    ui.on_export_markdown(move || {
        let app_state = &*app_state_arc;
        let session_id = app_state.get_current_session_id();
        if session_id.is_empty() {
            set_session_error(ui_weak.clone(), "没有选中的会话，无法导出 Markdown");
            return;
        }
        let ui_clone = ui_weak.clone();
        let sid = session_id.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!(target: "app_state", "export-markdown: failed to build runtime: {e}");
                    // Use the same set_session_error pattern as other failure paths in this function.
                    set_session_error(ui_clone.clone(), format!("导出失败: {e}"));
                    return;
                }
            };
            rt.block_on(async move {
                let facade = kernel_facade();
                let messages = match facade.get_messages(&sid).await {
                    Ok(msgs) => msgs,
                    Err(e) => {
                        set_session_error(ui_clone.clone(), format!("导出失败: {e}"));
                        return;
                    }
                };
                // Build Markdown content
                let mut md = String::new();
                md.push_str(&format!("# Session Export ({})\n\n", &sid));
                for msg in &messages {
                    let role_label = match msg.role {
                        MessageRoleDto::User => "User",
                        MessageRoleDto::Assistant => "Assistant",
                        MessageRoleDto::System => "System",
                        MessageRoleDto::Tool => "Tool",
                    };
                    md.push_str(&format!("## {}\n\n", role_label));
                    let text = match &msg.content {
                        MessageContentDto::Text(t) => t.clone(),
                        MessageContentDto::Multimodal { text, .. } => text.clone(),
                        MessageContentDto::Mixed { text, .. } => text.clone(),
                        MessageContentDto::ToolResult { result, .. } => result.to_string(),
                    };
                    md.push_str(&text);
                    md.push_str("\n\n");
                }
                // Write to file in current working directory
                let filename = format!("export-{}.md", &sid);
                let path = std::path::PathBuf::from(&filename);
                match std::fs::write(&path, &md) {
                    Ok(_) => {
                        let abs = std::fs::canonicalize(&path)
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or(filename);
                        tracing::info!(target: "app_state", "export-markdown: wrote {}", abs);
                        // Show success banner on UI thread
                        let ui_weak_ok = ui_clone.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak_ok.upgrade() {
                                ui.set_banner_message(SharedString::from(format!("已导出到 {}", abs)));
                                ui.set_banner_detail(SharedString::from(String::new()));
                            }
                        });
                    }
                    Err(e) => {
                        set_session_error(ui_clone.clone(), format!("写入文件失败: {e}"));
                    }
                }
            });
        });
    });
}

pub(super) fn register_open_session_settings_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    // --- open-session-settings callback (Q4=c) ---
    let ui_weak = ui.as_weak();
    ui.on_open_session_settings(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_current_route(SharedString::from("settings"));
        }
    });
}

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
                let facade = kernel_facade();
                // K4a-T23 (缺口6 adjudication): facade rename_session returns (),
                // so read back the normalized name via get_session_metadata to update the UI.
                match facade.rename_session(&sid, &name).await {
                    Ok(()) => {
                        // Read back the normalized name from the facade.
                        let normalized = facade
                            .get_session_metadata(&sid)
                            .await
                            .ok()
                            .map(|m| m.session_name)
                            .unwrap_or_else(|| name.clone());
                        // 2026-07-18 (D2j-fix): background thread — dispatch
                        // only the sync setter via invoke_from_event_loop;
                        // then drive refresh_sessions_ui directly (it handles
                        // its own UI dispatch). No nested block_on inside the
                        // invoke closure (would panic).
                        let ui_weak2 = ui_weak.clone();
                        let sid_for_dispatch = sid.clone();
                        let normalized_for_dispatch = normalized.clone();
                        let app_state_for_refresh = Arc::clone(&app_state_for_spawn);
                        let sid_clone = sid_for_dispatch.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak2.upgrade() {
                                ui.set_current_session_id(SharedString::from(sid_clone));
                            }
                        });
                        let current_now = app_state_for_refresh.get_current_session_id();
                        refresh_sessions_ui(ui_weak.clone(), &current_now).await;
                        if sid_for_dispatch == current_now {
                            let ui_weak_for_name = ui_weak.clone();
                            let name = normalized_for_dispatch.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ui) = ui_weak_for_name.upgrade() {
                                    ui.set_current_session_name(SharedString::from(name));
                                }
                            });
                        }
                    }
                    Err(e) => {
                        // 2026-07-18 (D2j): background thread — pass weak directly; helper upgrades on UI thread.
                        set_session_error(ui_weak.clone(), format!("Failed to rename session: {e}"));
                    }
                }
            });
        });
    });
}
