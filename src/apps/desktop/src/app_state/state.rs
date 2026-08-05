//! AppState struct + impl (R37a split from mod.rs)
//!
//! Owns the `AppState` god-struct and its `SessionMeta` companion. The
//! struct holds `Mutex`-protected fields that the Slint UI callbacks and
//! the async runtime share. Getters/setters stay co-located with the
//! struct so future maintainers see the field + its accessor in one
//! place. The `Default` impl just delegates to `new()`.

use parking_lot::Mutex;

use crate::app_state::slint_glue::SkillStateItem;

/// T4 (2026-08-05): process-global `AppState` so background
/// callback threads (e.g. `set-skill-filter`) can reach the
/// skills cache without re-plumbing `Arc<AppState>` through
/// every `register_X_callback(ui, app_state)` signature. Set
/// once in `create_ui` (the only call site that constructs
/// `AppState`); the filter callback is registered after
/// `create_ui`, so the lookup is non-racy in practice. The
/// type's other accessors (e.g. `set_skills_full`,
/// `skills_full_snapshot`, `set_skills_filter`,
/// `skills_filter`) go through this same handle.
static GLOBAL_APP_STATE: once_cell::sync::OnceCell<std::sync::Arc<AppState>> =
    once_cell::sync::OnceCell::new();

/// App-level state shared between Slint UI callbacks and the async core
pub struct AppState {
    /// Tracks whether the kernel facade core has been initialized
    pub core_ready: std::sync::OnceLock<()>,
    /// Currently active session ID (set by switch-session callback)
    current_session_id: Mutex<String>,
    /// Pagination cursor: message ID of the oldest loaded message (for load-more)
    load_more_cursor: Mutex<Option<String>>,
    /// Phase G.3: whether the sidebar tree shows subagent (depth >= 1)
    /// sessions. Default `true` so the tree view shows the full
    /// hierarchy on first launch. Flipped by the `toggle-show-subagents`
    /// callback from the sidebar checkbox.
    show_subagents: Mutex<bool>,
    // R37a: pub(super) accessor needed by callbacks_lifecycle::register_toggle_show_subagents_callback
    // and create_ui::create_ui (initial binding). Field stays private; access goes through the
    // `show_subagents_handle()` method below to keep visibility explicit.
    /// T4 (2026-08-05): full unfiltered skill list cached for the
    /// settings route's search filter. The settings panel's
    /// `skills-list` property holds the filtered view; the
    /// `set-skill-filter` callback applies a case-insensitive
    /// substring match on `name` or `description` against
    /// this cache and re-publishes. The cache is refreshed by
    /// `refresh_settings_lists` (the same path that populates
    /// `skills-list`), so toggles / list refreshes don't drop
    /// the user's search text. The drawer (Inspector) keeps
    /// its own model and is unaffected by the filter
    /// (brief §3.3: "抽屉模型不动").
    skills_full: Mutex<Vec<SkillStateItem>>,
    /// T4 (2026-08-05): current search filter text for the
    /// settings Skills module. The user types in
    /// `SettingsView`'s search input; the value is forwarded
    /// to Rust via the `set-skill-filter` callback. Empty
    /// string = show all.
    skills_filter: Mutex<String>,
    /// Phase I.3 (2026-06-20): the actor runtime, constructed at
    /// `create_ui` time when `USE_LIGHTWEIGHT_ACTOR = true`. The
    /// `OnceLock` stays empty when the flag is false (the default).
    /// Future Phase I.x work can use this to replace the heavy
    /// `ConversationCoordinator::execute_hidden_subagent_internal` path.
    actor_runtime: std::sync::OnceLock<std::sync::Arc<northhing_agent_dispatch::ActorRuntime>>,
    /// A7: tracks which session is currently streaming a response.
    /// Set when user sends a message, cleared when response completes.
    current_streaming_session: Mutex<Option<String>>,
    /// Tracks the active dialog turn id so the stop button can cancel it.
    /// Set from DialogTurnStarted, cleared on terminal turn events.
    active_turn_id: Mutex<Option<String>>,
    /// 2026-06-26 (Phase 5 Q6/Q7 wire-up): per-session metadata
    /// (provider_id + workspace_path) so `validate_session_integrity`
    /// can detect Q6 (provider deleted) and Q7 (workspace removed)
    /// for the live wire-up. Populated when a session is created
    /// (`on_new_session` callback). The runtime's `SessionSummary`
    /// doesn't currently expose these fields, so we maintain the
    /// mapping on the desktop side. When the core adds them to
    /// `SessionSummary`, this map can be removed.
    session_metadata: Mutex<std::collections::HashMap<String, SessionMeta>>,
}

/// 2026-06-26 (Phase 5): per-session metadata captured at session
/// creation time. Used by `validate_session_integrity` in the live
/// wire-up to detect Q6 (provider deleted) and Q7 (workspace removed).
#[derive(Debug, Clone)]
pub struct SessionMeta {
    /// Provider ID the session was created against. Empty string when
    /// the session predates this metadata tracking (legacy data).
    pub provider_id: String,
    /// Workspace path the session belongs to. Empty `PathBuf::new()`
    /// when the session was created in the default workspace.
    pub workspace_path: std::path::PathBuf,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            core_ready: std::sync::OnceLock::new(),
            current_session_id: Mutex::new(String::new()),
            load_more_cursor: Mutex::new(None),
            show_subagents: Mutex::new(true),
            actor_runtime: std::sync::OnceLock::new(),
            current_streaming_session: Mutex::new(None),
            active_turn_id: Mutex::new(None),
            session_metadata: Mutex::new(std::collections::HashMap::new()),
            skills_full: Mutex::new(Vec::new()),
            skills_filter: Mutex::new(String::new()),
        }
    }

    /// T4 (2026-08-05): install the process-global `AppState`
    /// handle. Called once from `create_ui` after `Arc::new`
    /// so background callback threads (`set-skill-filter`)
    /// can fetch it without an `Arc` parameter. Idempotent —
    /// the first setter wins.
    pub fn install_global(self: &std::sync::Arc<AppState>) {
        let _ = GLOBAL_APP_STATE.set(std::sync::Arc::clone(self));
    }

    /// T4 (2026-08-05): fetch the process-global `AppState`
    /// handle. Panics if `install_global` was never called
    /// (which is a programming error — every callback that
    /// uses the global is registered after `create_ui`).
    pub fn global() -> std::sync::Arc<AppState> {
        GLOBAL_APP_STATE
            .get()
            .expect("AppState::global() called before install_global()")
            .clone()
    }

    /// Phase I.3: install the actor runtime (called from
    /// `maybe_construct_actor_runtime` when the flag is on). Idempotent
    /// — the first setter wins, subsequent calls are ignored.
    pub fn set_actor_runtime(&self, runtime: std::sync::Arc<northhing_agent_dispatch::ActorRuntime>) {
        let _ = self.actor_runtime.set(runtime);
    }

    /// Phase I.3: get a reference to the actor runtime, if it was
    /// constructed. Returns `None` when `USE_LIGHTWEIGHT_ACTOR` is false.
    /// `#[allow(dead_code)]` because Phase I.3 only constructs the
    /// runtime — Phase I.x (the next plan phase) will replace the
    /// `ConversationCoordinator::execute_hidden_subagent_internal` call
    /// site with `state.actor_runtime().spawn_actor(...)` and use this
    /// getter for the first time.
    #[allow(dead_code)]
    pub fn actor_runtime(&self) -> Option<std::sync::Arc<northhing_agent_dispatch::ActorRuntime>> {
        self.actor_runtime.get().cloned()
    }

    /// Mark the kernel facade core as initialized
    pub fn set_core_ready(&self) {
        let _ = self.core_ready.set(());
    }

    /// Get the agentic system, or None if not yet initialized
    pub fn get_agentic_system(&self) -> Option<&()> {
        self.core_ready.get()
    }

    /// K.2.3 follow-up: get the `ConversationCoordinator` (if
    /// initialized). Used by `maybe_construct_actor_runtime` to
    /// forward the runtime into the coordinator's `ToolPipeline`.
    pub fn coordinator(
        &self,
    ) -> Option<std::sync::Arc<northhing_core::agentic::coordination::ConversationCoordinator>> {
        northhing_core::agentic::coordination::global_coordinator()
    }

    /// Get the current session ID
    pub fn get_current_session_id(&self) -> String {
        self.current_session_id.lock().clone()
    }

    /// Set the current session ID
    pub fn set_current_session_id(&self, id: String) {
        *self.current_session_id.lock() = id;
    }

    /// Set the load-more pagination cursor
    pub fn set_load_more_cursor(&self, cursor: Option<String>) {
        *self.load_more_cursor.lock() = cursor;
    }

    /// Get the load-more pagination cursor
    pub fn get_load_more_cursor(&self) -> Option<String> {
        self.load_more_cursor.lock().clone()
    }

    /// A7: set which session is currently streaming a response
    pub fn set_streaming_session(&self, session_id: Option<String>) {
        *self.current_streaming_session.lock() = session_id;
    }

    /// R37a: pub(super) accessor for the `show_subagents` field, needed by
    /// `callbacks_lifecycle::register_toggle_show_subagents_callback` and the
    /// initial binding in `create_ui::create_ui`. Returns the underlying
    /// `Mutex<bool>` (NOT owned, to avoid E0716 temporary-borrow issues when
    /// the caller wants to lock across statements).
    pub(super) fn show_subagents_handle(&self) -> &Mutex<bool> {
        &self.show_subagents
    }

    /// A7: get the session ID that is currently streaming, if any
    pub fn get_streaming_session(&self) -> Option<String> {
        self.current_streaming_session.lock().clone()
    }

    /// Set the active dialog turn id (set from DialogTurnStarted).
    pub fn set_active_turn_id(&self, turn_id: Option<String>) {
        *self.active_turn_id.lock() = turn_id;
    }

    /// Get the active dialog turn id, if any.
    pub fn get_active_turn_id(&self) -> Option<String> {
        self.active_turn_id.lock().clone()
    }

    /// 2026-07-27 (K4a R3, fix #2): atomic compare-and-clear of the
    /// `active_turn_id`. Returns `true` when the field was
    /// atomically set to `None` BECAUSE the current value
    /// matched `expected_turn_id`; `false` when the field held
    /// some other value (e.g. a fresh `enter_turn` already
    /// superseded this generation, or the field was already
    /// `None`).
    ///
    /// This is the missing primitive for the turn-generation
    /// guards in `streaming_lifecycle::clear_turn` and
    /// `streaming_lifecycle::enter_failed`. The pre-fix
    /// sequence
    ///     let cur = app_state.get_active_turn_id();
    ///     // <-- enter_turn may write here -->
    ///     if cur == expected { app_state.set_active_turn_id(None); }
    /// is racy: a concurrent `enter_turn` (e.g. the bridge
    /// firing `Started` for turn-2 while the submit-failure
    /// path tries to clear turn-1) can interleave between the
    /// get and the set, so we end up clearing a fresh
    /// generation's state.
    ///
    /// The lock is held across the compare AND the clear, so
    /// `enter_turn`'s subsequent write is serialized behind the
    /// call (and vice-versa). Returns `true` only when the
    /// observed value at the moment of the compare was
    /// `Some(expected_turn_id)` — exactly the contract callers
    /// need to decide whether to dispatch `set_is_streaming(false)`.
    pub fn compare_and_clear_active_turn_id(&self, expected_turn_id: &str) -> bool {
        let mut guard = self.active_turn_id.lock();
        match guard.as_ref() {
            Some(current) if current == expected_turn_id => {
                *guard = None;
                true
            }
            _ => false,
        }
    }

    /// 2026-06-26 (Phase 5): record session metadata when a session
    /// is created. Used by `validate_session_integrity` in the live
    /// wire-up to detect Q6/Q7. Called from `on_new_session` after
    /// `coordinator.create_session` returns the new session id.
    pub fn record_session_meta(&self, session_id: String, meta: SessionMeta) {
        self.session_metadata.lock().insert(session_id, meta);
    }

    /// 2026-06-26 (Phase 5): drop a session from the metadata map.
    /// Called from `on_delete_session` so stale entries don't trigger
    /// false-positive Q6/Q7 issues.
    pub fn forget_session_meta(&self, session_id: &str) {
        self.session_metadata.lock().remove(session_id);
    }

    /// 2026-06-26 (Phase 5): snapshot of all session metadata for
    /// use by `validate_session_integrity`. Returns cloned `Vec` so
    /// the caller can iterate without holding the lock.
    pub fn session_metadata_snapshot(&self) -> Vec<(String, SessionMeta)> {
        self.session_metadata
            .lock()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// T4 (2026-08-05): replace the cached full skill list (called
    /// by `refresh_settings_lists` after rebuilding from the kernel
    /// facade). The settings route's search filter reads this
    /// cache, not `skills-list` (which holds the filtered view).
    pub fn set_skills_full(&self, list: Vec<SkillStateItem>) {
        *self.skills_full.lock() = list;
    }

    /// T4 (2026-08-05): snapshot of the full skill list for the
    /// filter callback to operate on. Returns a clone so the caller
    /// can iterate without holding the lock.
    pub fn skills_full_snapshot(&self) -> Vec<SkillStateItem> {
        self.skills_full.lock().clone()
    }

    /// T4 (2026-08-05): update the settings Skills search filter
    /// text. The text is whatever the user typed into the
    /// `SettingsView` search input (Slint 1.16 has no
    /// `string.contains`, so the actual substring match runs here
    /// instead of in Slint).
    pub fn set_skills_filter(&self, filter: String) {
        *self.skills_filter.lock() = filter;
    }

    /// T4 (2026-08-05): current skills filter text. Used by
    /// `refresh_settings_lists` to apply the filter on every
    /// refresh (so the search box text and the published list
    /// stay in sync after the user toggles a skill or a new
    /// skill is discovered).
    pub fn skills_filter(&self) -> String {
        self.skills_filter.lock().clone()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
