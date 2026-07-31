//! Desktop event bridge — subscribes to kernel events via the facade and drives the Slint UI.
//!
//! Bridges the kernel facade `subscribe_events` API to the desktop UI: streams text
//! chunks into the message list, toggles the streaming flag on turn
//! start/cancel/complete/fail, surfaces turn-failure errors, and tracks the
//! active turn id so the stop button can cancel it.
//!
//! 2026-07-27 (K4a R3, Bug A — fix #1 + fix #2 + fix #3): all
//! `set_is_streaming` and `AppState` active-turn mutations are
//! routed through `super::streaming_lifecycle::{enter_turn,
//! clear_turn, enter_failed}` so the submit path, the event
//! bridge, and the submit-failure reset share one code path.
//! The helper enforces the turn-generation guard — a stale
//! `Completed` for an already-superseded turn can no longer
//! clear a fresh turn's streaming state (see
//! `streaming_lifecycle::tests
//! ::clear_turn_guards_against_stale_terminal_events`).
//!
//! The bridge now also takes a `StreamingStateDispatcher`
//! indirection (a trait) so tests can swap in a
//! `RecordingDispatcher` and assert the
//! Started→true/Completed→false/Failed→false contract without
//! standing up a Slint event loop (the no-op test platform
//! never drains its `invoke_from_event_loop` queue — see
//! `streaming_lifecycle::tests` for the recording-based
//! assertions).

use super::error_banners::set_session_error;
use super::sessions::build_messages_model;
use super::slint_glue::{AppWindow, MessageItem};
use super::state::AppState;
use super::streaming_lifecycle::{clear_turn, enter_failed, enter_turn, SlintDispatcher, StreamingStateDispatcher};
use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::events::{KernelEventDto, KernelEventsApi, SubscriptionId};
use northhing_kernel_api::session::KernelSessionApi;
use northhing_kernel_api::turn::TurnStateKind;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::sync::{Arc, Mutex};

pub struct DesktopEventBridge {
    /// The UI weak. Held in addition to the dispatcher so the
    /// `Failed` handler can still dispatch the session-error
    /// banner via `set_session_error` (which has its own
    /// `invoke_from_event_loop` and doesn't need the dispatcher
    /// abstraction). Tests that want to drive the bridge
    /// without standing up Slint can pass a default `Weak`.
    ui: slint::Weak<AppWindow>,
    /// Slint `is-streaming` setter, indirection for testability.
    dispatcher: Arc<dyn StreamingStateDispatcher>,
    app_state: Arc<AppState>,
    draft: Mutex<String>,
    last_flush: Mutex<std::time::Instant>,
    subscription_id: Mutex<Option<SubscriptionId>>,
    /// Tracks the number of non-streaming (baseline) messages set during
    /// the first flush of the current streaming turn. When `Some(n)`, the
    /// model has `n + 1` rows (n baseline + 1 streaming item) and subsequent
    /// flushes can update only the last row via `set_row_data(n, item)` (O(1))
    /// instead of rebuilding the entire model (O(n)).
    /// Reset to `None` on turn start and turn end.
    streaming_base_count: Arc<Mutex<Option<usize>>>,
}

impl DesktopEventBridge {
    /// Production constructor: wraps the AppWindow's Weak in a
    /// `SlintDispatcher` (production implementation that
    /// dispatches via `slint::invoke_from_event_loop`).
    fn new(ui: slint::Weak<AppWindow>, app_state: Arc<AppState>) -> Self {
        let dispatcher: Arc<dyn StreamingStateDispatcher> = Arc::new(SlintDispatcher { ui: ui.clone() });
        Self::with_dispatcher(ui, dispatcher, app_state)
    }

    /// Test constructor: lets callers inject a custom
    /// dispatcher (e.g. `RecordingDispatcher` from
    /// `streaming_lifecycle::tests`). Production code uses
    /// [`Self::new`].
    fn with_dispatcher(
        ui: slint::Weak<AppWindow>,
        dispatcher: Arc<dyn StreamingStateDispatcher>,
        app_state: Arc<AppState>,
    ) -> Self {
        Self {
            ui,
            dispatcher,
            app_state,
            draft: Mutex::new(String::new()),
            last_flush: Mutex::new(std::time::Instant::now()),
            subscription_id: Mutex::new(None),
            streaming_base_count: Arc::new(Mutex::new(None)),
        }
    }

    /// Handle a kernel event (called from the facade subscription callback on the
    /// worker runtime). Performs sync state mutations + UI dispatch inline; spawns
    /// async tasks for the message-fetching terminal states.
    fn handle_event(&self, event: &KernelEventDto) {
        let current_session = self.app_state.get_current_session_id();

        match event {
            KernelEventDto::TurnState {
                session_id,
                turn_id,
                state,
                error,
                ..
            } => {
                if session_id != &current_session {
                    return;
                }
                match state {
                    TurnStateKind::Started => {
                        if let Ok(mut d) = self.draft.lock() {
                            d.clear();
                        }
                        // Reset incremental streaming state for the new turn.
                        if let Ok(mut m) = self.streaming_base_count.lock() {
                            m.take();
                        }
                        enter_turn(&*self.dispatcher, &self.app_state, session_id, turn_id);
                    }
                    TurnStateKind::Completed | TurnStateKind::Cancelled => {
                        if let Ok(mut d) = self.draft.lock() {
                            d.clear();
                        }
                        // Reset incremental streaming state; spawn_refresh_messages
                        // will do a full rebuild as consistency correction.
                        if let Ok(mut m) = self.streaming_base_count.lock() {
                            m.take();
                        }
                        clear_turn(&*self.dispatcher, &self.app_state, turn_id);
                        self.spawn_refresh_messages(session_id.clone());
                    }
                    TurnStateKind::Failed => {
                        if let Ok(mut d) = self.draft.lock() {
                            d.clear();
                        }
                        // Reset incremental streaming state.
                        if let Ok(mut m) = self.streaming_base_count.lock() {
                            m.take();
                        }
                        // 2026-07-27 (K4a R3, fix #1): pass
                        // the event's turn_id so a stale
                        // Failed for an already-superseded
                        // turn is a no-op (it doesn't clear
                        // the fresh generation nor dispatch
                        // a false setter).
                        enter_failed(&*self.dispatcher, &self.app_state, turn_id);
                        let msg = format!("LLM 调用失败: {}", error.as_deref().unwrap_or("unknown error"));
                        let ui = self.ui.clone();
                        let msg_clone = msg.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui.upgrade() {
                                set_session_error(ui.as_weak(), msg_clone);
                            }
                        });
                        self.spawn_refresh_messages(session_id.clone());
                    }
                }
            }
            KernelEventDto::TextChunk { session_id, text } => {
                if session_id != &current_session {
                    return;
                }
                {
                    if let Ok(mut d) = self.draft.lock() {
                        d.push_str(text);
                    }
                }
                let should_flush = {
                    let Ok(mut last) = self.last_flush.lock() else {
                        return;
                    };
                    let now = std::time::Instant::now();
                    if now.duration_since(*last).as_millis() >= 120 {
                        *last = now;
                        true
                    } else {
                        false
                    }
                };
                if should_flush {
                    let draft = self.draft.lock().map(|d| d.clone()).unwrap_or_default();
                    self.spawn_flush_draft(session_id.clone(), draft);
                }
            }
            // Other variants (ToolCall, TurnPhase, Banner, Error) are ignored by the
            // desktop bridge — the original EventSubscriber impl ignored them too.
            _ => {}
        }
    }

    /// Spawn an async task to refresh messages from the facade (terminal states).
    fn spawn_refresh_messages(&self, session_id: String) {
        let ui = self.ui.clone();
        tokio::spawn(async move {
            let facade = kernel_facade();
            if let Ok(msgs) = facade.get_messages(&session_id).await {
                let ui_weak = ui.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        let model = build_messages_model(&msgs, None);
                        ui.set_messages(model);
                    }
                });
            }
        });
    }

    /// Spawn an async task to flush the draft to the UI (streaming chunk).
    ///
    /// Incremental update mode:
    /// - First call in a turn (`streaming_base_count` is None): full fetch + build
    ///   + `set_messages`, then store the baseline count for subsequent updates.
    /// - Subsequent calls (`streaming_base_count` is `Some(n)`): only update the
    ///   streaming item at index `n` via `set_row_data` (O(1)).
    /// - Turn end (Completed/Cancelled/Failed): full refresh via `spawn_refresh_messages`.
    fn spawn_flush_draft(&self, session_id: String, draft: String) {
        let ui = self.ui.clone();
        let base_count = self.streaming_base_count.lock().ok().and_then(|g| *g);

        if let Some(base) = base_count {
            // Incremental path: update only the streaming item (last row) — o(1).
            // Clone the guard so the closure can verify the turn hasn't ended
            // (and state hasn't been reset / rebuilt by spawn_refresh_messages)
            // before applying a stale set_row_data.
            let streaming_base_count = self.streaming_base_count.clone();
            let _ = slint::invoke_from_event_loop(move || {
                // Guard: skip if turn ended and state was reset.
                let current = streaming_base_count.lock().ok().and_then(|g| *g);
                if current != Some(base) {
                    return;
                }
                if let Some(ui) = ui.upgrade() {
                    let model = ui.get_messages();
                    let item = slint_streaming_item(draft);
                    // Defensive bounds check: the model may have been
                    // rebuilt between the guard check and here.
                    if (base as usize) < model.row_count() {
                        model.set_row_data(base, item);
                    }
                }
            });
        } else {
            // First flush in this turn: full fetch + build.
            let streaming_base_count = self.streaming_base_count.clone();
            tokio::spawn(async move {
                let facade = kernel_facade();
                match facade.get_messages(&session_id).await {
                    Ok(msgs) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui.upgrade() {
                                let mut items = super::sessions::build_messages_items(&msgs, None);
                                let base = items.len();
                                items.push(slint_streaming_item(draft));
                                ui.set_messages(ModelRc::new(VecModel::from(items)));
                                if let Ok(mut sm) = streaming_base_count.lock() {
                                    *sm = Some(base);
                                }
                            }
                        });
                    }
                    Err(_) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui.upgrade() {
                                let items = vec![slint_streaming_item(draft)];
                                ui.set_messages(ModelRc::new(VecModel::from(items)));
                                if let Ok(mut sm) = streaming_base_count.lock() {
                                    *sm = Some(0);
                                }
                            }
                        });
                    }
                }
            });
        }
    }
}

impl Drop for DesktopEventBridge {
    fn drop(&mut self) {
        let id = self.subscription_id.lock().ok().and_then(|mut g| g.take());
        let Some(id) = id else {
            return;
        };
        let facade = kernel_facade();
        let unsub = async move {
            let _ = facade.unsubscribe_events(id).await;
        };
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(unsub);
        } else if let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() {
            rt.block_on(unsub);
        }
    }
}

/// Build the synthetic streaming assistant message item.
fn slint_streaming_item(content: String) -> MessageItem {
    MessageItem {
        id: SharedString::from("__streaming__"),
        role: SharedString::from("assistant"),
        content: SharedString::from(content),
        timestamp: SharedString::from(""),
        is_streaming: true,
        tool_calls_count: 0,
        tool_calls_summary: SharedString::from(""),
        tool_calls_json: SharedString::from(""),
        think_content: SharedString::from(""),
        tool_names: ModelRc::new(VecModel::from(Vec::<SharedString>::new())),
    }
}

/// Construct the bridge and subscribe to kernel events via the facade.
///
/// No-ops with a warning log if the facade isn't ready yet.
pub(super) fn register_desktop_event_bridge(ui: &AppWindow, app_state: &Arc<AppState>) {
    // 2026-07-27 (K4a R3, fix #3): install the Slint dispatcher into
    // the process-global slot so the submit path (and any other
    // non-bridge caller) share the same `set_is_streaming` path
    // we test against. Production always installs a SlintDispatcher
    // that wraps the AppWindow's Weak; tests bypass this by
    // calling `set_streaming_dispatcher_for_test_or_panic`
    // BEFORE `register_desktop_event_bridge` is ever reached.
    let dispatcher: Arc<dyn StreamingStateDispatcher> = Arc::new(SlintDispatcher { ui: ui.as_weak() });
    super::streaming_lifecycle::install_streaming_dispatcher(dispatcher);

    let bridge = Arc::new(DesktopEventBridge::new(ui.as_weak(), Arc::clone(app_state)));

    // subscribe_events is async; bridge it without nesting a tokio runtime.
    // The actual subscription work is synchronous (just registers a subscriber),
    // so we use block_in_place when a runtime is already running (e.g. tests),
    // or a throwaway current-thread runtime otherwise.
    let subscription_id = if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| {
            handle.block_on(async {
                let bridge_for_callback = Arc::clone(&bridge);
                let callback = Box::new(move |event: KernelEventDto| {
                    bridge_for_callback.handle_event(&event);
                });
                kernel_facade().subscribe_events(callback).await
            })
        })
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build runtime for event bridge subscription")
            .block_on(async {
                let bridge_for_callback = Arc::clone(&bridge);
                let callback = Box::new(move |event: KernelEventDto| {
                    bridge_for_callback.handle_event(&event);
                });
                kernel_facade().subscribe_events(callback).await
            })
    };

    match subscription_id {
        Ok(id) => {
            *bridge.subscription_id.lock().unwrap() = Some(id);
        }
        Err(e) => {
            tracing::warn!(
                target: "app_state",
                "failed to subscribe to kernel events: {e}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::streaming_lifecycle::RecordingDispatcher;
    use northhing_kernel_api::events::SubscriptionId;
    use std::sync::Arc;

    /// Per-test dispatcher. Each bridge test allocates a
    /// fresh `RecordingDispatcher` so the test is hermetic:
    /// the `set_is_streaming` history captured by the
    /// assertion contains only the entries produced by THIS
    /// test's `handle_event` calls, not entries from
    /// parallel-running bridge tests in the same process
    /// (cargo test runs tests in parallel by default). The
    /// global `STREAMING_DISPATCHER` slot is NOT installed;
    /// `register_desktop_event_bridge` is what would install
    /// the production dispatcher — these tests bypass that
    /// path by using `DesktopEventBridge::with_dispatcher`
    /// directly.
    fn fresh_dispatcher() -> Arc<RecordingDispatcher> {
        RecordingDispatcher::new()
    }

    fn fresh_bridge(dispatcher: Arc<RecordingDispatcher>, session: &str) -> (DesktopEventBridge, Arc<AppState>) {
        let app_state = Arc::new(AppState::new());
        app_state.set_current_session_id(session.to_string());
        let bridge = DesktopEventBridge::with_dispatcher(slint::Weak::default(), dispatcher, Arc::clone(&app_state));
        (bridge, app_state)
    }

    /// started_event_tracks_active_turn_for_stop_path:
    /// the AppState side of the contract (active_turn_id,
    /// streaming_session) is updated, and the dispatcher
    /// records `set_is_streaming(true)`. This is the foundation
    /// of the Started→true half of the lifecycle assertion the
    /// pre-fix `started_event_schedules_is_streaming_setter_on_real_appwindow`
    /// test could not verify.
    #[test]
    fn started_event_tracks_active_turn_for_stop_path() {
        let dispatcher = fresh_dispatcher();
        let baseline = dispatcher.history().len();
        let (bridge, app_state) = fresh_bridge(dispatcher.clone(), "session-1");
        bridge.handle_event(&KernelEventDto::TurnState {
            session_id: "session-1".to_string(),
            turn_id: "turn-1".to_string(),
            state: TurnStateKind::Started,
            duration_ms: None,
            error: None,
            error_kind: None,
        });

        assert_eq!(app_state.get_active_turn_id().as_deref(), Some("turn-1"));
        assert_eq!(app_state.get_streaming_session().as_deref(), Some("session-1"));
        assert_eq!(
            &dispatcher.history()[baseline..],
            &[true],
            "Started must dispatch set_is_streaming(true) on the bridge dispatcher"
        );
    }

    /// 2026-07-27 (K4a R3, fix #3): the Started→true, then
    /// Completed→false lifecycle is observable end-to-end on the
    /// shared dispatcher. The previous
    /// `started_event_schedules_is_streaming_setter_on_real_appwindow`
    /// test relied on `slint::invoke_from_event_loop` from a
    /// no-op platform (which never drains the queue); the
    /// shared dispatcher is process-deterministic and the
    /// assertion holds.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn started_then_completed_dispatches_true_then_false() {
        let dispatcher = fresh_dispatcher();
        let baseline = dispatcher.history().len();
        let (bridge, app_state) = fresh_bridge(dispatcher.clone(), "session-1");
        bridge.handle_event(&KernelEventDto::TurnState {
            session_id: "session-1".to_string(),
            turn_id: "turn-1".to_string(),
            state: TurnStateKind::Started,
            duration_ms: None,
            error: None,
            error_kind: None,
        });
        bridge.handle_event(&KernelEventDto::TurnState {
            session_id: "session-1".to_string(),
            turn_id: "turn-1".to_string(),
            state: TurnStateKind::Completed,
            duration_ms: Some(123),
            error: None,
            error_kind: None,
        });

        assert_eq!(app_state.get_active_turn_id(), None);
        assert_eq!(app_state.get_streaming_session(), None);
        assert_eq!(
            &dispatcher.history()[baseline..],
            &[true, false],
            "Completed must dispatch set_is_streaming(false) on the bridge dispatcher"
        );
    }

    /// 2026-07-27 (K4a R3, fix #1 + fix #3): the Failed path
    /// dispatches `set_is_streaming(false)` so the stop button
    /// comes back down on a failed turn. This is the contract
    /// the pre-fix code missed (the bridge used to leave
    /// `is-streaming=true` stuck after a failure — see
    /// `streaming_lifecycle::tests::completed_and_failed_paths_both_dispatch_false`).
    ///
    /// The Failed handler also dispatches the session-error
    /// banner via `set_session_error`, which posts through
    /// `slint::invoke_from_event_loop` — that requires a tokio
    /// runtime context, hence the `tokio::test(flavor =
    /// "multi_thread")` annotation. The Started/Completed
    /// tests above don't need it because they only touch the
    /// dispatcher + AppState.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn failed_event_dispatches_false_on_bridge() {
        let dispatcher = fresh_dispatcher();
        let baseline = dispatcher.history().len();
        let (bridge, _app_state) = fresh_bridge(dispatcher.clone(), "session-1");
        bridge.handle_event(&KernelEventDto::TurnState {
            session_id: "session-1".to_string(),
            turn_id: "turn-1".to_string(),
            state: TurnStateKind::Started,
            duration_ms: None,
            error: None,
            error_kind: None,
        });
        bridge.handle_event(&KernelEventDto::TurnState {
            session_id: "session-1".to_string(),
            turn_id: "turn-1".to_string(),
            state: TurnStateKind::Failed,
            duration_ms: None,
            error: Some("network".to_string()),
            error_kind: None,
        });

        assert_eq!(
            &dispatcher.history()[baseline..],
            &[true, false],
            "Failed must dispatch set_is_streaming(false) on the bridge dispatcher"
        );
    }

    /// 2026-07-27 (K4a R3, Bug A — fix #2): the turn-generation
    /// guard at the bridge boundary rejects a stale terminal
    /// event for an already-superseded turn (the bridge calls
    /// `clear_turn(turn_id=stale)`, which returns false because
    /// the active generation has advanced to a new turn id).
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn stale_terminal_event_is_dropped_at_bridge() {
        let dispatcher = fresh_dispatcher();
        let baseline = dispatcher.history().len();
        let (bridge, app_state) = fresh_bridge(dispatcher.clone(), "session-1");
        bridge.handle_event(&KernelEventDto::TurnState {
            session_id: "session-1".to_string(),
            turn_id: "turn-1".to_string(),
            state: TurnStateKind::Started,
            duration_ms: None,
            error: None,
            error_kind: None,
        });
        bridge.handle_event(&KernelEventDto::TurnState {
            session_id: "session-1".to_string(),
            turn_id: "turn-2".to_string(),
            state: TurnStateKind::Started,
            duration_ms: None,
            error: None,
            error_kind: None,
        });
        // turn-1's terminal event arrives: stale, must be ignored.
        bridge.handle_event(&KernelEventDto::TurnState {
            session_id: "session-1".to_string(),
            turn_id: "turn-1".to_string(),
            state: TurnStateKind::Completed,
            duration_ms: Some(123),
            error: None,
            error_kind: None,
        });

        assert_eq!(app_state.get_active_turn_id().as_deref(), Some("turn-2"));
        assert_eq!(
            &dispatcher.history()[baseline..],
            &[true, true],
            "stale terminal event must not append a false setter"
        );
    }

    /// 2026-07-27 (K4a R3, Bug A): out-of-session events must NOT
    /// mutate the AppState nor dispatch a setter. The bridge's
    /// `if session_id != &current_session { return; }` gate is
    /// the first line of defense against the agentic router
    /// fan-out (token-usage, thread-goal-tokens, cron, etc.). A
    /// regression here would let a background event from
    /// another session flip is-streaming=true on the active
    /// session's UI.
    #[test]
    fn started_event_for_other_session_is_dropped() {
        let dispatcher = fresh_dispatcher();
        let baseline = dispatcher.history().len();
        let (bridge, app_state) = fresh_bridge(dispatcher.clone(), "active-session");

        bridge.handle_event(&KernelEventDto::TurnState {
            session_id: "background-session".to_string(),
            turn_id: "other-turn".to_string(),
            state: TurnStateKind::Started,
            duration_ms: None,
            error: None,
            error_kind: None,
        });

        assert_eq!(app_state.get_active_turn_id(), None);
        assert_eq!(app_state.get_streaming_session(), None);
        assert_eq!(
            dispatcher.history()[baseline..],
            Vec::<bool>::new(),
            "out-of-session event must not touch the dispatcher"
        );
    }

    /// Regression test: Drop must take the subscription_id
    /// exactly once. If the id is already taken (e.g. a prior
    /// cleanup), Drop must not panic and must not attempt a
    /// second unsubscribe.
    #[test]
    fn drop_takes_subscription_id_idempotently() {
        let dispatcher = fresh_dispatcher();
        let bridge = DesktopEventBridge::with_dispatcher(slint::Weak::default(), dispatcher, Arc::new(AppState::new()));
        // Pre-arm a subscription_id so Drop has something to take.
        *bridge.subscription_id.lock().unwrap() = Some("999".to_string());

        // Simulate a first cleanup that takes the id.
        let first = bridge.subscription_id.lock().unwrap().take();
        assert!(first.is_some());

        // Drop the bridge — Drop impl will try to take again,
        // get None, and return early. This must not panic.
        drop(bridge);
    }
}
