//! create_ui + run_event_loop + spawn_startup_session (R37a split from mod.rs)
//!
//! The 17 callback wirings are extracted to `callbacks_lifecycle.rs`
//! + `callbacks_settings.rs` as `register_X_callback(ui, app_state)`
//! functions; this file calls each in order after the initial UI setup
//! and background-thread spawns. The startup-session background thread
//! (P0-A) is moved here from the inline tail of the original create_ui.
//!
//! Bodies + comments are preserved verbatim from the original `mod.rs`
//! (R37a spec: preserve all comments + bodies).

use super::actor::maybe_construct_actor_runtime;
use super::callbacks_lifecycle::{
    register_clear_inline_error_callback, register_clear_input_error_callback, register_clear_session_error_callback,
    register_delete_session_callback, register_dismiss_banner_callback, register_load_more_messages_callback,
    register_new_session_callback, register_refresh_messages_callback, register_refresh_sessions_callback,
    register_send_message_callback, register_stop_streaming_callback, register_switch_session_callback,
    register_toggle_show_subagents_callback, register_toggle_skill_callback, register_toggle_theme_callback,
};
use super::callbacks_settings::{
    register_delete_provider_callback, register_remove_workspace_callback, register_upsert_provider_callback,
};
use super::error_banners::set_session_error;
use super::event_bridge;
use super::inspector::build_mcp_status_string;
use super::inspector_model_status::build_model_status_string;
use super::sessions::refresh_sessions_ui;
use super::skills::refresh_skills_ui;
use super::slint_glue::{AppWindow, MessageItem, SessionItem, SkillItem};
use super::state::AppState;
use anyhow::Result;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::sync::Arc;

/// Create the Slint UI instance with callbacks wired to app state.
///
/// Phase I.2 (2026-06-20): takes `Arc<AppState>` (was `&'static AppState`).
/// Each Slint callback now captures an `Arc::clone` instead of a raw
/// `*const AppState`, so the file compiles under `#![forbid(unsafe_code)]`.
/// `Arc::clone` is one atomic increment — negligible cost vs. the raw
/// pointer cast it replaces.
pub fn create_ui(app_state: Arc<AppState>) -> Result<AppWindow> {
    // Phase H (2026-06-20): record the boot event so manual tests can
    // confirm the app reached `create_ui` at all (vs. crashing earlier
    // in main). Fire-and-forget — never blocks startup. Spawn on a
    // dedicated thread because `log_event` is async and the caller
    // here is the synchronous `create_ui` entry point.
    std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
        match rt {
            Ok(rt) => {
                rt.block_on(async move {
                    northhing_core::infrastructure::debug_log::log_event(
                        northhing_core::infrastructure::debug_log::COMP_APP_LIFECYCLE,
                        crate::flags::DEFAULT_MODE_ID,
                        "app_state::create_ui:enter",
                        "desktop shell entering create_ui",
                        None,
                    )
                    .await;
                });
            }
            Err(e) => {
                eprintln!("Phase H: failed to build runtime for create_ui boot log: {e}");
            }
        }
    });

    let ui = AppWindow::new()?;

    // Set initial values
    ui.set_app_title(SharedString::from("northhing v0.1.0"));
    // Phase G.2 (replaces Phase C.5 placeholder): the Inspector's MCP
    // status reads from the live `McpCatalogReader` once at init. The
    // initial `"MCP: not configured"` placeholder is set first so the
    // Inspector renders something immediately; the background refresh
    // below replaces it with the live count once `MCPService` answers.
    ui.set_mcp_status(SharedString::from("MCP: not configured"));
    ui.set_model_status(SharedString::from("Model: Not configured"));
    ui.set_dark_mode(true);
    ui.set_current_session_id(SharedString::from(""));
    // Phase C.2: bind the sidebar tree-view flag from `flags.rs`.
    // `SESSION_TREE_VIEW = true` renders nested sessions; `false` keeps
    // the byte-identical flat list. The const lives in `flags.rs` rather
    // than `main.rs` because `main` is a binary sibling, not a lib module.
    ui.set_session_tree_view(crate::flags::SESSION_TREE_VIEW);

    // Phase 4 (spec §6.1 Q9=a): first-launch detect. If the app.json
    // doesn't exist OR only has P0-B legacy seeded providers (no real
    // providers, no workspaces), route the UI to the welcome flow.
    // The check runs in a background thread to keep `create_ui` non-
    // blocking — startup should not wait for disk I/O. Until the
    // background check finishes, the UI shows the main view (current
    // default); once the result lands, the route flips to "welcome"
    // if needed. This avoids a perceptible delay during boot.
    //
    // 2026-06-26 (manual test fix): the `set_current_route` call MUST
    // run on the Slint event loop thread. Calling Slint property setters
    // from a non-event-loop thread is silently dropped (Slint 1.16 posts
    // a debug warning and skips the update). Use `invoke_from_event_loop`
    // to dispatch the setter onto the UI thread.
    let ui_weak_first_run = ui.as_weak();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Phase 4: failed to build runtime for first-run check: {e}");
                return;
            }
        };
        rt.block_on(async move {
            match crate::app_state::settings::load_app_settings().await {
                Ok(settings) => {
                    let is_first = settings.is_first_run();
                    tracing::info!(
                        target: "app_state",
                        "Phase 4: first-run check complete: is_first_run={is_first}"
                    );
                    if is_first {
                        let ui_weak = ui_weak_first_run.clone();
                        if let Err(e) = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_current_route(slint::SharedString::from("welcome"));
                            }
                        }) {
                            tracing::warn!(
                                target: "app_state",
                                "Phase 4: failed to dispatch welcome route to UI thread: {e}"
                            );
                        }
                    }
                    // One-time startup sync: push any providers the user
                    // stored on disk into core's runtime config. This is
                    // the migration path for existing users whose keys
                    // live in app.json but whose core config is empty.
                    // Failure is non-fatal — we just warn and continue.
                    if !settings.providers.is_empty() {
                        if let Err(e) =
                            crate::app_state::settings::sync_providers_to_core(&settings).await
                        {
                            tracing::warn!(
                                target: "app_state",
                                "startup sync_providers_to_core failed: {e}"
                            );
                        }
                    }
                }
                Err(e) => {
                    // Settings load failure is non-fatal — show the main
                    // UI as before. The Rust side already logs eprintln!
                    // via the function; no need to duplicate.
                    tracing::warn!(
                        target: "app_state",
                        "Phase 4: first-run check skipped: settings load failed: {e}"
                    );
                }
            }
        });
    });

    // Phase I.3: construct an `ActorRuntime` (when the flag is on)
    // and register a heartbeat actor. The runtime is a no-op when the
    // flag is `false` (the default) — no behavior change for users.
    maybe_construct_actor_runtime(&app_state, &ui);
    // Phase G.3: bind the show-subagents toggle. Initial value comes from
    // `AppState::new` (default true). The user can flip it via the
    // sidebar checkbox; the callback updates both the Slint property
    // and the AppState field.
    ui.set_show_subagents(*app_state.show_subagents_handle().lock());

    // Phase C.3: refresh `model-status` from the live provider list. The
    // initial placeholder ("Model: Not configured") is replaced once the
    // global config service reports which providers have enabled models.
    // This is fire-and-forget — if it fails we keep the placeholder.
    //
    // 2026-06-26 (review follow-up): the `set_model_status` call MUST run
    // on the Slint event loop thread. Calling a Slint property setter from
    // a non-event-loop thread is silently dropped (Slint 1.16). Same root
    // cause as the first-run welcome-route fix in `748f628` — wrap with
    // `invoke_from_event_loop`. Reviewer caught this as a latent bug
    // matching the original Phase 4 pattern.
    let ui_weak_provider = ui.as_weak();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Phase C.3: failed to build runtime for model-status refresh: {e}");
                return;
            }
        };
        rt.block_on(async move {
            let Some(ui) = ui_weak_provider.upgrade() else {
                return;
            };
            let status = build_model_status_string().await;
            let ui_weak = ui.as_weak();
            if let Err(e) = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_model_status(SharedString::from(status));
                }
            }) {
                tracing::warn!(
                    target: "app_state",
                    "Phase C.3: failed to dispatch model-status to UI thread: {e}"
                );
            }
        });
    });

    // Phase G.2: refresh `mcp-status` from the live MCP catalog. Mirrors
    // the C.3 pattern — fire-and-forget thread, fail silently so the
    // placeholder persists if the catalog can't be reached.
    //
    // 2026-06-26 (review follow-up): wrap `set_mcp_status` in
    // `invoke_from_event_loop` to dispatch the Slint setter onto the UI
    // thread. Same root cause as Phase C.3 / first-run welcome-route
    // (`748f628`). Reviewer caught this as a latent bug.
    let ui_weak_mcp = ui.as_weak();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Phase G.2: failed to build runtime for mcp-status refresh: {e}");
                return;
            }
        };
        rt.block_on(async move {
            let Some(ui) = ui_weak_mcp.upgrade() else {
                return;
            };
            let status = build_mcp_status_string().await;
            let ui_weak = ui.as_weak();
            if let Err(e) = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_mcp_status(SharedString::from(status));
                }
            }) {
                tracing::warn!(
                    target: "app_state",
                    "Phase G.2: failed to dispatch mcp-status to UI thread: {e}"
                );
            }
        });
    });

    // Pre-build empty models for initial state
    ui.set_sessions(ModelRc::new(VecModel::from(Vec::<SessionItem>::new())));
    ui.set_messages(ModelRc::new(VecModel::from(Vec::<MessageItem>::new())));
    ui.set_skills(ModelRc::new(VecModel::from(Vec::<SkillItem>::new())));

    // Phase C.4: initial skills load. The Inspector renders the empty
    // placeholder until this completes; on failure the user can still
    // interact with whatever was already loaded.
    let ui_weak_skills_init = ui.as_weak();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Phase C.4: failed to build runtime for initial skills load: {e}");
                return;
            }
        };
        rt.block_on(async move {
            if let Some(ui) = ui_weak_skills_init.upgrade() {
                refresh_skills_ui(&ui).await;
            }
        });
    });

    // Register the desktop event bridge (core events → UI). The global
    // coordinator is initialized by `initialize_core_services` before
    // `run_slint_app` calls `create_ui`, so `global_coordinator()` is
    // expected to be available here.
    event_bridge::register_desktop_event_bridge(&ui, &app_state);

    // --- Register all 17 Slint callbacks ---
    // LifecyCle callbacks (chat/session/theme/subagents/skill/clears)
    register_send_message_callback(&ui, &app_state);
    register_new_session_callback(&ui, &app_state);
    register_switch_session_callback(&ui, &app_state);
    register_delete_session_callback(&ui, &app_state);
    register_toggle_theme_callback(&ui, &app_state);
    register_toggle_show_subagents_callback(&ui, &app_state);
    register_toggle_skill_callback(&ui, &app_state);
    register_load_more_messages_callback(&ui, &app_state);
    register_refresh_sessions_callback(&ui, &app_state);
    register_refresh_messages_callback(&ui, &app_state);
    register_clear_session_error_callback(&ui, &app_state);
    register_clear_input_error_callback(&ui, &app_state);
    register_dismiss_banner_callback(&ui, &app_state);
    register_clear_inline_error_callback(&ui, &app_state);
    register_stop_streaming_callback(&ui, &app_state);
    // Settings callbacks (Q6/Q7/upsert)
    register_delete_provider_callback(&ui, &app_state);
    register_remove_workspace_callback(&ui, &app_state);
    register_upsert_provider_callback(&ui, &app_state);

    // P0-A startup auto-create session (background thread)
    spawn_startup_session(&ui, &app_state);

    Ok(ui)
}

/// Run the Slint event loop
pub fn run_event_loop(ui: AppWindow) -> Result<()> {
    ui.show()?;
    slint::run_event_loop()?;
    Ok(())
}

// --- P0-A (2026-06-25): startup auto-create session ---
// Without this, the sidebar stays empty and `on_send_message` early-returns
// because `current_session_id == ""`. The agentic system is initialized by
// the worker thread before `run_slint_app()` calls us, so we wait briefly
// (poll up to ~5s) for `agentic_system` and the global coordinator to be
// available, then create a default session and update the UI.
//
// Failure modes are surfaced via the P0-C error banner (not stderr).
pub(super) fn spawn_startup_session(ui: &AppWindow, app_state: &Arc<AppState>) {
    let app_state_arc_startup = std::sync::Arc::clone(app_state);
    let ui_weak_startup = ui.as_weak();
    std::thread::spawn(move || {
        // Poll for agentic system + coordinator readiness. Worker thread
        // initializes these before calling run_slint_app, but spawn ordering
        // is not strictly guaranteed if anything between fails.
        let mut system_ready = false;
        for _ in 0..50 {
            if app_state_arc_startup.get_agentic_system().is_some() {
                system_ready = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        if !system_ready {
            tracing::warn!("P0-A: agentic system not ready after 5s, skipping startup session");
            if let Some(ui) = ui_weak_startup.upgrade() {
                set_session_error(&ui, "Startup: agentic system not ready. Try restarting the app.");
            }
            return;
        }

        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("P0-A: failed to build tokio runtime: {e}");
                return;
            }
        };
        rt.block_on(async move {
            let app_state = &*app_state_arc_startup;
            let Some(coordinator) = northhing_core::agentic::coordination::global_coordinator() else {
                tracing::warn!("P0-A: global coordinator not available");
                if let Some(ui) = ui_weak_startup.upgrade() {
                    set_session_error(
                        &ui,
                        "Startup: global coordinator not available. Try restarting the app.",
                    );
                }
                return;
            };

            let workspace = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string());

            let session_name = format!("Session {}", chrono::Local::now().format("%Y-%m-%d %H:%M"));

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
                    app_state.set_load_more_cursor(None);

                    tracing::info!(target: "app_state", "P0-A: created startup session {sid}");

                    // 2026-06-26 (review follow-up): both `set_current_session_id`
                    // and `refresh_sessions_ui` (which calls `set_sessions` and
                    // `set_current_session_id` internally) are Slint property
                    // setters. They MUST run on the event loop thread — calling
                    // from this background `rt.block_on` is silently dropped by
                    // Slint 1.16. Dispatch the whole UI-touching block via
                    // `invoke_from_event_loop`; the dispatched closure runs a
                    // fresh tokio runtime to await the `refresh_sessions_ui`
                    // future on the UI thread. Rust state mutations above
                    // (`app_state.set_current_session_id` /
                    // `set_load_more_cursor`) stay on the background thread.
                    //
                    // Reviewer flagged this as the third latent bug matching
                    // the Phase 4 first-run pattern (`748f628`).
                    if let Some(ui) = ui_weak_startup.upgrade() {
                        let ui_weak = ui.as_weak();
                        let sid_for_dispatch = sid.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            let Some(ui) = ui_weak.upgrade() else {
                                return;
                            };
                            ui.set_current_session_id(SharedString::from(sid_for_dispatch.clone()));
                            // refresh_sessions_ui awaits a tokio future. We
                            // can't `await` inside a sync closure, so spin up
                            // a fresh current-thread runtime to drive the
                            // future to completion on the UI thread.
                            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
                            if let Ok(rt) = rt {
                                let _ = rt.block_on(refresh_sessions_ui(&ui, &sid_for_dispatch));
                            }
                        });
                    }
                }
                Err(e) => {
                    tracing::error!("P0-A: failed to create startup session: {e}");
                    if let Some(ui) = ui_weak_startup.upgrade() {
                        set_session_error(&ui, format!("Failed to create startup session: {e}"));
                    }
                }
            }
        });
    });
}
