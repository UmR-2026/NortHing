//! AppState struct + impl (R37a split from mod.rs)
//!
//! Owns the `AppState` god-struct and its `SessionMeta` companion. The
//! struct holds `Mutex`-protected fields that the Slint UI callbacks and
//! the async runtime share. Getters/setters stay co-located with the
//! struct so future maintainers see the field + its accessor in one
//! place. The `Default` impl just delegates to `new()`.

use parking_lot::Mutex;

/// App-level state shared between Slint UI callbacks and the async core
pub struct AppState {
    /// Handle to the agentic system (available after init)
    pub agentic_system: std::sync::OnceLock<std::sync::Arc<northhing_core::agentic::system::AgenticSystem>>,
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
            agentic_system: std::sync::OnceLock::new(),
            current_session_id: Mutex::new(String::new()),
            load_more_cursor: Mutex::new(None),
            show_subagents: Mutex::new(true),
            actor_runtime: std::sync::OnceLock::new(),
            current_streaming_session: Mutex::new(None),
            active_turn_id: Mutex::new(None),
            session_metadata: Mutex::new(std::collections::HashMap::new()),
        }
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

    /// Set the agentic system reference after initialization
    pub fn set_agentic_system(&self, system: std::sync::Arc<northhing_core::agentic::system::AgenticSystem>) {
        let _ = self.agentic_system.set(system);
    }

    /// Get the agentic system, or None if not yet initialized
    pub fn get_agentic_system(&self) -> Option<&std::sync::Arc<northhing_core::agentic::system::AgenticSystem>> {
        self.agentic_system.get()
    }

    /// K.2.3 follow-up: get the `ConversationCoordinator` (if
    /// initialized). Used by `maybe_construct_actor_runtime` to
    /// forward the runtime into the coordinator's `ToolPipeline`.
    pub fn coordinator(
        &self,
    ) -> Option<std::sync::Arc<northhing_core::agentic::coordination::ConversationCoordinator>> {
        self.agentic_system.get().map(|s| s.coordinator.clone())
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
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
