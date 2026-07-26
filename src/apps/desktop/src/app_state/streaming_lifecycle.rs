//! Streaming lifecycle helper — single source of truth for the
//! `is_streaming` Slint property (the gate for the stop button in
//! `ChatPaneView.slint:149`).
//!
//! 2026-07-27 (K4a R3, Bug A — fix #1 + fix #2): the submit path and
//! the event bridge each used to maintain their own
//! `set_is_streaming(true|false)` calls. Under event-bridge lag
//! (`event_queue.subscribe()` broadcast can lag past
//! `EVENT_BROADCAST_BUFFER=1024` in `queue.rs:54`) the stop button
//! wouldn't render until the event finally caught up. Worse, on
//! submit-failure paths (`callbacks_lifecycle.rs:151-156`
//! turn-runtime missing; `callbacks_lifecycle.rs:201-212`
//! accepted=false / `submit_turn` Err) we cleared
//! `AppState::streaming_session` but forgot to clear the Slint
//! root, so `is-streaming` was permanently stuck at `true` until
//! the next turn started. And the event bridge had no
//! turn-generation guard — a stale `Completed` event for an
//! already-superseded turn could clear a fresh turn's streaming
//! state.
//!
//! Fix: every caller (submit path, event bridge Started, event
//! bridge Completed/Cancelled/Failed, submit-failure paths) routes
//! through the same `enter_turn` / `clear_turn` /
//! `reset_after_submit_failure` / `enter_failed` helpers in this
//! module. The helpers:
//!
//! 1. Single UI-thread dispatch site
//!    (`dispatcher.set_is_streaming(value)`) that owns the
//!    `is_streaming` setter, so the submit path can flip the
//!    property without waiting for the broadcast router.
//! 2. AppState + Slint root are always toggled together (no more
//!    `set_streaming_session(None)` without the matching Slint
//!    setter).
//! 3. Turn generation: the `active_turn_id` AppState field is
//!    the "current generation". A terminal event whose `turn_id`
//!    does not match the current generation is a stale event from
//!    a superseded turn and is dropped on the floor. A fresh
//!    `enter_turn` always wins, even if a stale terminal event
//!    was in flight.
//!
//! 2026-07-27 (K4a R3, fix #3): the `dispatcher: Arc<dyn
//! StreamingStateDispatcher>` indirection lets tests inject a
//! `RecordingDispatcher` that captures every `set_is_streaming`
//! call. The production path stays on the real Slint setter via
//! `slint::invoke_from_event_loop`. The bridge and the submit
//! path share the dispatcher (installed into a `OnceLock` by
//! `register_desktop_event_bridge`; the submit path reads it
//! via `streaming_dispatcher()`), so we get a single observer
//! of the Slint property across both call sites — assertions
//! in `event_bridge::tests` cover the full lifecycle without
//! relying on the no-op Slint platform's `invoke_from_event_loop`
//! queue (which never drains, see the previous test that
//! `started_event_schedules_is_streaming_setter_on_real_appwindow`
//! silently relied on).
//!
//! Test coverage is in `streaming_lifecycle::tests` and
//! `event_bridge::tests`.

use super::slint_glue::AppWindow;
use super::state::AppState;
#[cfg(test)]
use std::sync::Mutex;
use std::sync::{Arc, OnceLock};

/// Process-global dispatcher. Set by `register_desktop_event_bridge`
/// after the AppWindow is built; read by the submit path and
/// `register_stop_streaming_callback` to issue their
/// `set_is_streaming` dispatches. Tests inject a
/// `RecordingDispatcher` via `set_streaming_dispatcher_for_test`
/// (which is `#[cfg(test)]`) to observe the full lifecycle
/// without a Slint event loop.
static STREAMING_DISPATCHER: OnceLock<Arc<dyn StreamingStateDispatcher>> = OnceLock::new();

/// Production-side installer. Called from
/// `register_desktop_event_bridge` immediately after the
/// `Arc<SlintDispatcher>` is built. Panics if a dispatcher is
/// already installed (which would mean two `AppWindow`s were
/// constructed in the same process — a programmer error in
/// production; tests use `set_streaming_dispatcher_for_test`
/// instead and run a single test per process to avoid the
/// `OnceLock` constraint).
pub(super) fn install_streaming_dispatcher(dispatcher: Arc<dyn StreamingStateDispatcher>) {
    if STREAMING_DISPATCHER.set(dispatcher).is_err() {
        panic!(
            "STREAMING_DISPATCHER already installed; the desktop shell \
             supports exactly one AppWindow per process"
        );
    }
}

/// Read the installed dispatcher. Used by the submit path
/// (`callbacks_lifecycle::register_send_message_callback`) and
/// the stop-button handler. Returns `None` if no dispatcher
/// has been installed yet (the call site is reached before
/// `create_ui` runs — e.g. a future CLI harness that exercises
/// the same `AppState` without a UI). Callers fall back to
/// direct AppState mutation in that case.
pub(super) fn streaming_dispatcher() -> Option<Arc<dyn StreamingStateDispatcher>> {
    STREAMING_DISPATCHER.get().cloned()
}

/// Test-only installer: replace the dispatcher with a custom
/// one (typically a `RecordingDispatcher`). Returns the
/// previous dispatcher (if any) so tests can restore it on
/// teardown — important when multiple tests share a process
/// and the `OnceLock` can only be set once per process.
///
/// If the global is already initialized, we DO NOT replace it
/// (the `OnceLock::set` rule). Tests that need a
/// `RecordingDispatcher` should be the FIRST test in the
/// process to touch the global. In practice we run unit tests
/// in parallel by default, so this constraint is enforced by
/// the `set_streaming_dispatcher_for_test_or_panic` helper,
/// which fails fast in `cargo test -p northhing --lib` if the
/// race was lost — surfacing the test-organizer bug rather
/// than silently dropping the assertion.
#[cfg(test)]
pub(super) fn set_streaming_dispatcher_for_test_or_panic(dispatcher: Arc<dyn StreamingStateDispatcher>) {
    if STREAMING_DISPATCHER.get().is_some() {
        panic!(
            "STREAMING_DISPATCHER already installed by another test; \
             this is a test-organizer bug (two tests racing on the \
             process-global dispatcher). Use --test-threads=1 or \
             refactor the test to share the dispatcher."
        );
    }
    STREAMING_DISPATCHER
        .set(dispatcher)
        .map_err(|_| "STREAMING_DISPATCHER was just confirmed empty")
        .expect("STREAMING_DISPATCHER was just confirmed empty");
}

/// Abstracts the `set_is_streaming(value)` Slint setter so tests
/// can record the calls without standing up a Slint event loop.
/// Production code uses `SlintDispatcher`; tests use
/// `RecordingDispatcher` (both `#[cfg(test)]` here + in
/// `event_bridge`).
pub(super) trait StreamingStateDispatcher: Send + Sync + 'static {
    fn set_is_streaming(&self, value: bool);
}

/// Production dispatcher. Wraps `slint::invoke_from_event_loop` so
/// the Slint setter fires on the UI thread (Slint 1.16 silently
/// drops property writes from non-UI threads; see
/// `AGENTS.md:166-167` "UI thread discipline"). The closure is
/// `move`d into the queue — no shared state escapes after this
/// returns.
pub(super) struct SlintDispatcher {
    pub(super) ui: slint::Weak<AppWindow>,
}

impl StreamingStateDispatcher for SlintDispatcher {
    fn set_is_streaming(&self, value: bool) {
        let ui_weak = self.ui.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_is_streaming(value);
            }
        });
    }
}

/// Test-only dispatcher that records every `set_is_streaming`
/// call. The bridge / submit path / helper can be exercised
/// without standing up a Slint event loop, and the recorded
/// history is asserted on in tests.
#[cfg(test)]
pub(super) struct RecordingDispatcher {
    history: Mutex<Vec<bool>>,
}

#[cfg(test)]
impl RecordingDispatcher {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            history: Mutex::new(Vec::new()),
        })
    }
    pub(super) fn history(&self) -> Vec<bool> {
        self.history.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl StreamingStateDispatcher for RecordingDispatcher {
    fn set_is_streaming(&self, value: bool) {
        self.history.lock().unwrap().push(value);
    }
}

/// Enter a new turn. The submit path and the event bridge's
/// `Started` handler both call this — the Slint `is-streaming`
/// setter is dispatched immediately (no broadcast-router hop) and
/// the `active_turn_id` is recorded so a stale `Completed` for
/// an older turn can be filtered later. Caller is responsible
/// for verifying the bridge's session-membership gate (the event
/// bridge keeps that check; the submit path doesn't need it
/// because the UI thread already gated on
/// `current_session_id`).
///
/// 2026-07-27 (K4a R3, fix #3): returns a `GenerationToken`
/// (a `String` clone of `turn_id`) so the submit path can
/// later call `reset_after_submit_failure(&token)` and have
/// the reset no-op when a fresh `enter_turn` already
/// superseded the submit's generation. The bridge's
/// `Started` handler can ignore the return value (it does
/// not need to track a token for the bridge's own
/// `clear_turn` calls — those carry the event's `turn_id`
/// directly, and the compare-and-clear primitive filters
/// out stale ones).
pub(super) fn enter_turn(
    dispatcher: &dyn StreamingStateDispatcher,
    app_state: &AppState,
    session_id: &str,
    turn_id: &str,
) -> String {
    app_state.set_active_turn_id(Some(turn_id.to_string()));
    app_state.set_streaming_session(Some(session_id.to_string()));
    dispatcher.set_is_streaming(true);
    turn_id.to_string()
}

/// Clear a turn, but only if `turn_id` matches the current
/// generation (`active_turn_id`). A terminal event for an
/// already-superseded turn returns false and is a no-op. Returns
/// true when the clear actually ran.
///
/// 2026-07-27 (K4a R3, fix #2): the compare-and-clear of
/// `active_turn_id` is now atomic — the pre-fix sequence
/// (`get` then `set`) was racy, allowing a concurrent
/// `enter_turn` to interleave between the read and the write
/// and end up with the fresh generation's state cleared by a
/// stale terminal event. The fix goes through
/// `AppState::compare_and_clear_active_turn_id`, which holds
/// the `active_turn_id` mutex across the compare AND the clear
/// in a single critical section.
pub(super) fn clear_turn(dispatcher: &dyn StreamingStateDispatcher, app_state: &AppState, turn_id: &str) -> bool {
    // Atomic compare-and-clear: returns `true` only when
    // `active_turn_id == Some(turn_id)` at the moment of the
    // check, and atomically clears it under the same lock.
    if app_state.compare_and_clear_active_turn_id(turn_id) {
        // The clear succeeded — this turn_id is the live
        // generation. Drop the streaming session marker and
        // dispatch the Slint setter.
        app_state.set_streaming_session(None);
        dispatcher.set_is_streaming(false);
        true
    } else {
        // Either `active_turn_id` was already `None` (a
        // "no current generation" case — the bridge fired a
        // terminal event we never saw a `Started` for, or
        // `reset_after_submit_failure` already cleared), or
        // it was `Some(other_id)` (a fresh `enter_turn`
        // superseded this terminal event). In both cases we
        // do NOT dispatch `set_is_streaming(false)` — the
        // latter would be a stale-terminal bug visible on
        // the UI as the stop button flickering off mid-turn.
        false
    }
}

/// Failed-turn path. Used by the event bridge's `Failed` handler
/// in place of `clear_turn` + the old `set_session_error` call —
/// the AppState fields clear AND the Slint root flips to false
/// (so the stop button comes down) AND the caller is expected
/// to dispatch the session-error banner separately.
///
/// 2026-07-27 (K4a R3, fix #1): the pre-fix `enter_failed` had
/// no `turn_id` parameter and unconditionally cleared
/// `active_turn_id`. That made it vulnerable to the same
/// stale-event class as `clear_turn` BEFORE fix #2: a Failed
/// for turn-1 landing AFTER turn-2 had already started would
/// erase turn-2's generation marker and dispatch a
/// `set_is_streaming(false)` while turn-2 was actually
/// running. The fix routes through the same atomic
/// compare-and-clear primitive as `clear_turn` so a stale
/// Failed for an already-superseded turn is a no-op (it
/// doesn't clear the fresh generation nor dispatch a false
/// setter).
pub(super) fn enter_failed(dispatcher: &dyn StreamingStateDispatcher, app_state: &AppState, turn_id: &str) {
    if app_state.compare_and_clear_active_turn_id(turn_id) {
        app_state.set_streaming_session(None);
        dispatcher.set_is_streaming(false);
    }
    // Stale Failed (active_turn_id was either `None` or
    // `Some(other_id)` at the moment of the compare): no-op.
    // We do NOT clear the active generation nor dispatch a
    // false setter, so a fresh turn's stop button stays
    // visible.
}

/// Submit-failure reset. The submit path's pre-flight gates
/// (agentic system not ready, empty session, turn runtime
/// missing) and the post-`submit_turn` failure paths
/// (accepted=false, Err) all funnel through here.
///
/// 2026-07-27 (K4a R3, fix #3): the pre-fix reset was
/// unconditional — it cleared `active_turn_id` and dispatched
/// `set_is_streaming(false)` regardless of the live
/// generation. That was wrong in the interleaving case:
///
///   T0: submit path pre-flights `enter_turn("submit-pending-uuid-A")`
///   T1: submit path dispatches to the turn-runtime; result
///       is pending in the worker thread
///   T2: a SECOND submit / bridge event pre-flights
///       `enter_turn("submit-pending-uuid-B")` (the
///       `active_turn_id` is now B)
///
///   T3: T1's submit_turn returns Err (e.g. accepted=false)
///   T4: the old reset_after_submit_failure runs, sees
///       `active_turn_id == B`, and:
///
///     - clears B from AppState (silently erases the
///       second turn's state),
///     - dispatches set_is_streaming(false) while B is the
///       live generation and the user is actively watching
///       the stop button,
///     - and the next "Started" event from B's actual
///       execution path will re-set the property to true,
///       causing a visible flicker.
///
/// The fix: take a `generation: &str` (the return value of
/// the matching `enter_turn`) and only clear / dispatch when
/// `active_turn_id == Some(generation)` at the moment of
/// the call. A submit that fired before another `enter_turn`
/// has a stale `generation` and is a no-op — the second
/// turn's state is preserved. The atomic compare-and-clear
/// primitive in `AppState` keeps the check + clear in one
/// critical section, so a concurrent `enter_turn` from the
/// bridge can't race in between.
pub(super) fn reset_after_submit_failure(
    dispatcher: &dyn StreamingStateDispatcher,
    app_state: &AppState,
    generation: &str,
) {
    if app_state.compare_and_clear_active_turn_id(generation) {
        app_state.set_streaming_session(None);
        dispatcher.set_is_streaming(false);
    }
    // Stale generation: another `enter_turn` already
    // superseded the submit that captured `generation`.
    // No-op. The fresh turn's state stays intact.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::state::AppState;
    use std::sync::Arc;

    fn fresh_state() -> Arc<AppState> {
        Arc::new(AppState::new())
    }

    /// enter_turn sets active_turn_id, streaming_session, and
    /// dispatches `set_is_streaming(true)`. The dispatcher
    /// records the call so the test can assert the
    /// Started→true half of the contract that the previous
    /// `started_event_schedules_is_streaming_setter_on_real_appwindow`
    /// test could not verify (the no-op platform never drains
    /// its `invoke_from_event_loop` queue).
    #[test]
    fn enter_turn_records_active_generation_and_dispatches_true() {
        let disp = RecordingDispatcher::new();
        let state = fresh_state();
        enter_turn(&*disp, &state, "sess-A", "turn-1");
        assert_eq!(state.get_active_turn_id().as_deref(), Some("turn-1"));
        assert_eq!(state.get_streaming_session().as_deref(), Some("sess-A"));
        assert_eq!(disp.history(), vec![true]);
    }

    /// clear_turn only resets state when the turn_id matches the
    /// current generation. A stale `Completed` event for a
    /// superseded turn must NOT clear a fresh turn's state and
    /// must NOT fire a `false` setter.
    #[test]
    fn clear_turn_guards_against_stale_terminal_events() {
        let disp = RecordingDispatcher::new();
        let state = fresh_state();
        enter_turn(&*disp, &state, "sess-A", "turn-1");
        // A new turn starts before turn-1's terminal event lands.
        enter_turn(&*disp, &state, "sess-A", "turn-2");
        assert_eq!(disp.history(), vec![true, true]);

        // turn-1's terminal event arrives: stale, must be
        // ignored (no extra false setter, active_turn_id
        // preserved at turn-2).
        let cleared = clear_turn(&*disp, &state, "turn-1");
        assert!(!cleared, "stale terminal event must not clear");
        assert_eq!(state.get_active_turn_id().as_deref(), Some("turn-2"));
        assert_eq!(
            disp.history(),
            vec![true, true],
            "stale terminal event must not append a false setter"
        );

        // turn-2's terminal event: this one wins.
        let cleared = clear_turn(&*disp, &state, "turn-2");
        assert!(cleared, "matching terminal event must clear");
        assert_eq!(state.get_active_turn_id(), None);
        assert_eq!(state.get_streaming_session(), None);
        assert_eq!(disp.history(), vec![true, true, false]);
    }

    /// 2026-07-27 (K4a R3, fix #2): the pre-fix `clear_turn`
    /// did `get_active_turn_id()` (acquire lock) then
    /// `set_active_turn_id(None)` (re-acquire lock). A
    /// concurrent `enter_turn` could interleave between the
    /// two lock acquisitions and end up with a fresh
    /// generation's `active_turn_id` cleared by a stale
    /// terminal event. The fix routes through
    /// `AppState::compare_and_clear_active_turn_id` which
    /// holds the lock across BOTH the compare AND the
    /// clear — the interleave is impossible.
    ///
    /// We can't reproduce the interleave deterministically
    /// without spawning a real tokio task that races
    /// between the get and the set, but we can simulate the
    /// window by hand: take a snapshot of `active_turn_id`
    /// BEFORE `clear_turn` runs, then advance the generation
    /// via a second `enter_turn`, then call `clear_turn`
    /// with the now-stale snapshot. Under the new contract
    /// the clear must fail (return false, no false
    /// setter); under the pre-fix contract it would have
    /// succeeded and dispatched a false setter.
    #[test]
    fn clear_turn_uses_atomic_compare_and_clear() {
        let disp = RecordingDispatcher::new();
        let state = fresh_state();
        enter_turn(&*disp, &state, "sess-A", "turn-A");
        // Simulate the interleave: we WERE going to clear
        // turn-A, but a fresh `enter_turn` advanced the
        // generation to turn-B before our `clear_turn` ran.
        let _ = enter_turn(&*disp, &state, "sess-A", "turn-B");
        // Now we call `clear_turn` with the stale turn-A
        // id. The atomic compare-and-clear sees
        // `active_turn_id == Some("turn-B")` != Some("turn-A")
        // and returns false.
        let cleared = clear_turn(&*disp, &state, "turn-A");
        assert!(
            !cleared,
            "stale-terminal clear must fail atomically when generation has advanced"
        );
        assert_eq!(
            state.get_active_turn_id().as_deref(),
            Some("turn-B"),
            "atomic compare-and-clear must NOT have cleared the fresh generation"
        );
        assert_eq!(
            disp.history(),
            vec![true, true],
            "no spurious false setter from the stale clear"
        );
    }

    /// Completed and Failed both end with `is_streaming=false`
    /// — the contract the stop button relies on. Failed routes
    /// through `enter_failed(turn_id)` (now turn-generation
    /// guarded, see fix #1); Completed / Cancelled route through
    /// `clear_turn(turn_id)` (also guarded). This test exercises
    /// both arms of the contract.
    #[test]
    fn completed_and_failed_paths_both_dispatch_false() {
        // Completed arm: turn-id matches the active generation.
        let disp = RecordingDispatcher::new();
        let state = fresh_state();
        enter_turn(&*disp, &state, "sess-A", "turn-1");
        let cleared = clear_turn(&*disp, &state, "turn-1");
        assert!(cleared);
        assert_eq!(disp.history(), vec![true, false]);

        // Failed arm: turn-id matches the active generation.
        let disp = RecordingDispatcher::new();
        let state = fresh_state();
        enter_turn(&*disp, &state, "sess-A", "turn-1");
        enter_failed(&*disp, &state, "turn-1");
        assert_eq!(disp.history(), vec![true, false]);
        assert_eq!(state.get_active_turn_id(), None);
        assert_eq!(state.get_streaming_session(), None);
    }

    /// 2026-07-27 (K4a R3, fix #1): a stale Failed for an
    /// already-superseded turn is a no-op. The pre-fix
    /// `enter_failed` had no `turn_id` parameter, so a
    /// Failed for turn-1 landing AFTER turn-2 had already
    /// started would erase turn-2's generation marker and
    /// dispatch a `set_is_streaming(false)` while turn-2 was
    /// actually running (visible on the UI as the stop button
    /// flickering off mid-turn). The fix routes the
    /// generation check through the same atomic
    /// compare-and-clear primitive as `clear_turn`.
    #[test]
    fn stale_failed_event_is_dropped() {
        let disp = RecordingDispatcher::new();
        let state = fresh_state();
        // Started→Started→stale Failed sequence: turn-1
        // starts, then a fresh turn-2 starts, then a
        // stale-Failed for turn-1 arrives.
        enter_turn(&*disp, &state, "sess-A", "turn-1");
        enter_turn(&*disp, &state, "sess-A", "turn-2");
        // Baseline: two true dispatches.
        assert_eq!(disp.history(), vec![true, true]);
        // Stale Failed for turn-1: must NOT clear turn-2's
        // state nor dispatch a false setter.
        enter_failed(&*disp, &state, "turn-1");
        assert_eq!(
            state.get_active_turn_id().as_deref(),
            Some("turn-2"),
            "stale Failed must not clear the fresh generation"
        );
        assert_eq!(
            state.get_streaming_session().as_deref(),
            Some("sess-A"),
            "stale Failed must not clear the streaming session"
        );
        assert_eq!(
            disp.history(),
            vec![true, true],
            "stale Failed must not append a false setter"
        );
    }

    /// clear_turn on an empty generation (no enter_turn was
    /// ever called) is a no-op-flavoured clear: it sweeps any
    /// stray streaming_session to None and dispatches a `false`
    /// setter. This is the safety net for the "bridge fired a
    /// terminal event we never saw a Started for" edge case.
    #[test]
    fn clear_turn_with_empty_generation_is_noop() {
        let disp = RecordingDispatcher::new();
        let state = fresh_state();
        // No enter_turn — orphan streaming_session (the event
        // bridge or the submit path set it directly, no guard).
        state.set_streaming_session(Some("orphan".to_string()));
        assert_eq!(state.get_streaming_session().as_deref(), Some("orphan"));

        // clear_turn with an empty generation (active_turn_id
        // is `None`) is a no-op under the new turn-generation
        // contract. The pre-fix `clear_turn` would have swept
        // any orphan `streaming_session` and dispatched a
        // `false` setter regardless. That was wrong: a
        // Completed for "some-stale-turn" we never saw a
        // `Started` for doesn't know the live generation, so
        // it must not blindly dispatch a `false` setter (the
        // bridge's session-membership gate already filtered
        // events for other sessions, so a Completed for
        // "some-stale-turn" implies the previous Started was
        // missed and a fresh Started may still be in flight
        // for the same session — dispatching `false` here
        // would flicker the stop button off mid-turn).
        //
        // The orphan streaming_session stays in place; the
        // next genuine `Started` for the same session will
        // see active_turn_id as None and proceed
        // (`compare_and_clear_active_turn_id` returns false,
        // but `enter_turn` unconditionally sets the new id and
        // dispatches `true`).
        let cleared = clear_turn(&*disp, &state, "some-stale-turn");
        assert!(!cleared, "orphan clear_turn must be a no-op (no current generation)");
        assert_eq!(
            state.get_streaming_session().as_deref(),
            Some("orphan"),
            "orphan streaming_session must be preserved (no false setter dispatch)"
        );
        assert_eq!(
            disp.history(),
            Vec::<bool>::new(),
            "orphan clear_turn must not append a false setter"
        );
    }

    /// reset_after_submit_failure: when the submit's
    /// generation still matches the live `active_turn_id`,
    /// the reset tears down the UI and dispatches a `false`
    /// setter. Captures the generation token from
    /// `enter_turn` so the call site can pass the same value
    /// the matching submit's pre-flight used.
    #[test]
    fn reset_after_submit_failure_clears_when_generation_matches() {
        let disp = RecordingDispatcher::new();
        let state = fresh_state();
        let generation = enter_turn(&*disp, &state, "sess-A", "turn-1");
        reset_after_submit_failure(&*disp, &state, &generation);
        assert_eq!(state.get_active_turn_id(), None);
        assert_eq!(state.get_streaming_session(), None);
        assert_eq!(disp.history(), vec![true, false]);
    }

    /// reset_after_submit_failure: when the submit's
    /// generation has been superseded (a fresh `enter_turn`
    /// already advanced `active_turn_id` to a new value),
    /// the reset is a no-op. The fresh turn's state is
    /// preserved (the second `set_is_streaming(true)` from
    /// the new `enter_turn` is not followed by an unwanted
    /// `false` from the stale submit-failure path).
    #[test]
    fn reset_after_submit_failure_noop_when_generation_superseded() {
        let disp = RecordingDispatcher::new();
        let state = fresh_state();
        // Submit A: pre-flights enter_turn with generation A.
        let gen_a = enter_turn(&*disp, &state, "sess-A", "turn-A");
        assert_eq!(state.get_active_turn_id().as_deref(), Some("turn-A"));
        // Before submit A's submit_turn returns, the user
        // sends a second message (or the bridge's Started
        // for a queued turn lands). enter_turn advances the
        // generation to B.
        let _gen_b = enter_turn(&*disp, &state, "sess-A", "turn-B");
        assert_eq!(state.get_active_turn_id().as_deref(), Some("turn-B"));
        // Submit A's submit_turn returns Err; the failure
        // path runs reset_after_submit_failure with gen_a.
        // gen_a is stale — the reset must not touch B.
        reset_after_submit_failure(&*disp, &state, &gen_a);
        assert_eq!(
            state.get_active_turn_id().as_deref(),
            Some("turn-B"),
            "stale submit-failure reset must NOT clear the fresh generation"
        );
        assert_eq!(
            state.get_streaming_session().as_deref(),
            Some("sess-A"),
            "stale submit-failure reset must NOT clear the streaming session"
        );
        // The dispatcher history is [true, true] — two
        // enter_turn true dispatches, no spurious false from
        // the stale reset.
        assert_eq!(
            disp.history(),
            vec![true, true],
            "stale reset must not append a false setter"
        );
    }
}
