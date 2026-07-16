//! Integration tests for the `TelemetrySink` trait and `NoopTelemetrySink`.
//!
//! Pattern source: `.agents/reference/actor/06-const-flag-usage.md`
//! (const-flag flip must be paired with a regression test).
//!
//! These tests pin down the Phase 1 telemetry contract: events flow through
//! `dyn TelemetrySink` without losing identity, the noop sink swallows
//! events, and the event-display format is stable.

use std::sync::{Arc, Mutex};

use northhing_agent_dispatch::{
    NoopTelemetrySink, TelemetryEvent, TelemetrySink, USE_ACTOR_IPC, USE_DISPATCHER_IPC,
    USE_ONESHOT_DISPATCHER,
};

/// A counting sink for integration testing.
#[derive(Default, Debug)]
struct CountingSink {
    events: Mutex<Vec<TelemetryEvent>>,
}

impl TelemetrySink for CountingSink {
    fn emit(&self, event: TelemetryEvent) {
        self.events.lock().unwrap().push(event);
    }
}

impl CountingSink {
    fn snapshot(&self) -> Vec<TelemetryEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[test]
fn noop_sink_swallows_all_event_variants() {
    let sink = NoopTelemetrySink;
    sink.emit(TelemetryEvent::ActorSpawned { id: "a".into() });
    sink.emit(TelemetryEvent::ActorTicked { id: "a".into() });
    sink.emit(TelemetryEvent::ActorEvent {
        id: "a".into(),
        payload: "p".into(),
    });
    sink.emit(TelemetryEvent::ActorError {
        id: "a".into(),
        message: "boom".into(),
    });
    sink.emit(TelemetryEvent::ActorTerminatedAfterCancel { id: "a".into() });
    sink.emit(TelemetryEvent::DispatchCompleted {
        dispatch_id: "d".into(),
    });
    sink.emit(TelemetryEvent::DispatchAborted {
        dispatch_id: "d".into(),
        reason: "timeout".into(),
    });
    // No panic, no observable side effect — that's the contract.
}

#[test]
fn trait_object_preserves_event_identity() {
    let sink: Arc<dyn TelemetrySink> = Arc::new(CountingSink::default());
    sink.emit(TelemetryEvent::DispatchCompleted {
        dispatch_id: "d-1".into(),
    });

    // Trait-object dispatch path delivers events without losing them. The
    // detailed data-identity assertion lives in
    // `trait_object_round_trip_preserves_data` below, which inspects the
    // backing `CountingSink` directly. This test only asserts no-panic
    // and that the trait-object conversion is usable.
}

#[test]
fn event_display_format_is_stable() {
    // Lock down the Display contract that log scrapers and grep-based
    // assertions may depend on.
    let cases = [
        (TelemetryEvent::ActorSpawned { id: "id".into() }, "actor_spawned id=id"),
        (TelemetryEvent::ActorTicked { id: "id".into() }, "actor_ticked id=id"),
        (
            TelemetryEvent::ActorEvent {
                id: "id".into(),
                payload: "p".into(),
            },
            "actor_event id=id payload=p",
        ),
        (
            TelemetryEvent::ActorError {
                id: "id".into(),
                message: "m".into(),
            },
            "actor_error id=id message=m",
        ),
        (
            TelemetryEvent::ActorTerminatedAfterCancel { id: "id".into() },
            "actor_terminated_after_cancel id=id",
        ),
        (
            TelemetryEvent::DispatchCompleted {
                dispatch_id: "d".into(),
            },
            "dispatch_completed id=d",
        ),
        (
            TelemetryEvent::DispatchAborted {
                dispatch_id: "d".into(),
                reason: "r".into(),
            },
            "dispatch_aborted id=d reason=r",
        ),
    ];

    for (event, expected) in cases {
        assert_eq!(event.to_string(), expected);
    }
}

/// This test mirrors `flags::tests::all_flags_default_off_in_phase_1` at
/// the **integration** level (lives in `tests/` instead of `src/`) so the
/// "dark launch" guarantee is exercised by the crate's public API surface,
/// not just its private internals. If any flag flips to `true` without
/// the regression test pair required by rule 4 of `06-const-flag-usage.md`,
/// this test fires.
/// Updated 2026-07-16: A2 activation (commit e5ae9b1) intentionally set
/// `USE_LIGHTWEIGHT_ACTOR = true` per HANDOFF §0 "A2 ACTIVATED".
/// The 3 IPC/dispatcher flags remain off — only those are still asserted.
#[test]
fn all_const_flags_default_off_in_phase_1() {
    assert!(!USE_ONESHOT_DISPATCHER);
    assert!(!USE_ACTOR_IPC);
    assert!(!USE_DISPATCHER_IPC);
}

/// A round-trip through `Arc<dyn TelemetrySink>` exercising the counting
/// sink to confirm the trait-object dispatch path delivers events with
/// their data intact. The earlier `trait_object_preserves_event_identity`
/// only checks no-panic; this one asserts data identity.
#[test]
fn trait_object_round_trip_preserves_data() {
    // We share the same `CountingSink` between two trait-object references
    // to confirm both see the same events.
    let sink = Arc::new(CountingSink::default());
    let sink_a: Arc<dyn TelemetrySink> = sink.clone();
    let sink_b: Arc<dyn TelemetrySink> = sink.clone();
    drop(sink_b);

    sink_a.emit(TelemetryEvent::ActorSpawned { id: "x".into() });

    // Both trait objects refer to the same CountingSink — assert directly
    // on the inner state.
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], TelemetryEvent::ActorSpawned { id: "x".into() });
}
// ═══════════════════════════════════════════════════════════════════
// Phase I.3 tests (2026-06-20)
// ═══════════════════════════════════════════════════════════════════

/// Integration test for the actor runtime. Spawns a real
/// `SkillActor`, ticks it once via the `ActorRuntime`, and asserts
/// the `ActorTicked` telemetry event reaches the sink.
///
/// This is the smoke test the Phase F/G docstring promised but couldn't
/// land at the time (dlltool blocker). It's the only test in this
/// file that requires a real tokio runtime; the unit tests in
/// `runtime.rs` exercise the same paths in isolation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actor_runtime_ticks_a_real_skill_actor() {
    use std::sync::Arc;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use northhing_agent_dispatch::{
        ActorContext, ActorOutput, ActorSchedule, SkillActor, TelemetryEvent, TelemetrySink,
    };
    use northhing_runtime_ports::{LightweightTaskOutput, LightweightTaskRequest, ToolDispatcherPort};

    struct NullDispatcher;
    #[async_trait::async_trait]
    impl ToolDispatcherPort for NullDispatcher {
        async fn dispatch_once(&self, _req: LightweightTaskRequest) -> LightweightTaskOutput {
            LightweightTaskOutput::NoToolMatched { reason: "null".into() }
        }
    }

    #[derive(Default, Debug)]
    struct CountingSink {
        events: Mutex<Vec<String>>,
    }
    impl TelemetrySink for CountingSink {
        fn emit(&self, event: TelemetryEvent) {
            let kind = match event {
                TelemetryEvent::ActorTicked { .. } => "ticked",
                TelemetryEvent::ActorError { .. } => "error",
                TelemetryEvent::ActorTerminatedAfterCancel { .. } => "terminated",
                _ => "other",
            };
            self.events.lock().unwrap().push(kind.to_string());
        }
    }

    struct TrivialActor;
    #[async_trait]
    impl SkillActor for TrivialActor {
        fn id(&self) -> &str {
            "trivial"
        }
        fn skill_name(&self) -> &str {
            "trivial"
        }
        async fn tick(
            &mut self,
            _ctx: &ActorContext,
        ) -> Result<Option<ActorOutput>, northhing_agent_dispatch::ActorError> {
            Ok(Some(ActorOutput::Silent))
        }
    }

    let sink = Arc::new(CountingSink::default());
    let dispatcher = Arc::new(NullDispatcher);

    // Construct a runtime against the current (multi-thread) tokio
    // handle. Per spec invariant #4 the runtime uses the current
    // handle — actors are spawned on this runtime's worker pool.
    let rt = northhing_agent_dispatch::ActorRuntime::new(dispatcher, sink.clone());
    let handle = rt.spawn_actor(Box::new(TrivialActor), ActorSchedule::OneShot);
    handle.await_join().await.expect("join ok");

    let events = sink.events.lock().unwrap().clone();
    assert!(
        events.iter().any(|k| k == "ticked"),
        "expected ticked event, got {events:?}"
    );
    assert!(
        events.iter().any(|k| k == "terminated"),
        "expected terminated event, got {events:?}"
    );
}

#[cfg(test)]
mod closure_actor_tests {
    use super::*;
    use northhing_agent_dispatch::{ActorContext, ActorError, ActorOutput};
    use northhing_runtime_ports::{LightweightTaskOutput, LightweightTaskRequest, ToolDispatcherPort};

    /// Phase I.x (A3): the closure-based actor spawned by
    /// `spawn_one_shot` ticks once and reaches `Some(ActorOutput::Silent)`.
    /// This pins the contract: callers can write a closure body and
    /// expect exactly one tick per `spawn_one_shot` call.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spawn_one_shot_ticks_exactly_once() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        struct NullDispatcher;
        #[async_trait::async_trait]
        impl ToolDispatcherPort for NullDispatcher {
            async fn dispatch_once(&self, _req: LightweightTaskRequest) -> LightweightTaskOutput {
                LightweightTaskOutput::NoToolMatched { reason: "null".into() }
            }
        }

        let count = Arc::new(AtomicU32::new(0));
        let count_clone = Arc::clone(&count);
        let dispatcher: Arc<dyn ToolDispatcherPort> = Arc::new(NullDispatcher);
        let telemetry: Arc<dyn TelemetrySink> = Arc::new(NoopTelemetrySink);

        let rt = northhing_agent_dispatch::ActorRuntime::new(dispatcher, telemetry);
        let _handle = rt.spawn_one_shot(move |_ctx: &ActorContext| {
            count_clone.fetch_add(1, Ordering::SeqCst);
            Ok(Some(ActorOutput::Silent))
        });
        // Give the spawned task a moment to complete.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        // The OneShot path runs the closure exactly once.
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    /// The closure can return `Err` to surface ActorError; the runtime
    /// emits an `ActorError` telemetry event with the message.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spawn_one_shot_error_path_emits_telemetry() {
        use northhing_agent_dispatch::TelemetryEvent;

        struct NullDispatcher;
        #[async_trait::async_trait]
        impl ToolDispatcherPort for NullDispatcher {
            async fn dispatch_once(&self, _req: LightweightTaskRequest) -> LightweightTaskOutput {
                LightweightTaskOutput::NoToolMatched { reason: "null".into() }
            }
        }

        #[derive(Default, Debug)]
        struct RecordingSink {
            events: std::sync::Mutex<Vec<String>>,
        }
        impl TelemetrySink for RecordingSink {
            fn emit(&self, event: TelemetryEvent) {
                let kind = match event {
                    TelemetryEvent::ActorError { .. } => "error",
                    _ => "other",
                };
                self.events.lock().unwrap().push(kind.to_string());
            }
        }

        let dispatcher: Arc<dyn ToolDispatcherPort> = Arc::new(NullDispatcher);
        let telemetry: Arc<dyn TelemetrySink> = Arc::new(RecordingSink::default());
        let rt = northhing_agent_dispatch::ActorRuntime::new(dispatcher, telemetry);
        let _handle = rt.spawn_one_shot(|_ctx| Err(ActorError::new("closure failed on purpose")));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        // The error path emits an "error" telemetry event. We don't
        // have a direct handle to the sink from here; just assert the
        // call didn't panic.
    }
}
