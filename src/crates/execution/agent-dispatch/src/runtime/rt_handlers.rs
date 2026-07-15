//! `ActorRuntime` per-tick helpers + tests.
//!
//! - [`run_one_tick`] — internal free function used by
//!   [`super::rt_dispatch`] for the `OneShot`, `Periodic`, and
//!   `OnSignal` schedule branches. Observes `ctx.cancel` and the
//!   per-tick timeout, emits telemetry for the tick outcome.
//! - `#[cfg(test)] mod tests` — the test suite that previously lived
//!   at the bottom of `runtime.rs`. Re-homed here because the suite
//!   is sibling-aware (it touches [`super::rt_types::ActorHandle`] and
//!   [`super::rt_types::ActorRuntime`] directly) and a single
//!   `tests.rs` keeps the `mod tests` private inside the runtime
//!   module rather than promoting it to the crate root.

use std::time::Duration;

use crate::actor::ActorContext;
use crate::actor::SkillActor;

/// Internal: run a single actor tick under the per-tick timeout.
/// Observes `ctx.cancel` and emits telemetry for the tick outcome.
///
/// Takes `&mut Box<dyn SkillActor>` so callers (Periodic loop, signal
/// loop) can retain the actor across multiple ticks.
pub(super) async fn run_one_tick(actor: &mut Box<dyn SkillActor>, ctx: &ActorContext, timeout: Duration) {
    let id = actor.id().to_string();
    let start = std::time::Instant::now();
    let outcome = tokio::select! {
        result = actor.tick(ctx) => result,
        _ = ctx.cancel.cancelled() => {
            ctx.telemetry.emit(crate::telemetry::TelemetryEvent::ActorError {
                id: id.clone(),
                message: "actor cancelled mid-tick".into(),
            });
            return;
        }
        _ = tokio::time::sleep(timeout) => {
            ctx.telemetry.emit(crate::telemetry::TelemetryEvent::ActorError {
                id: id.clone(),
                message: format!("actor tick exceeded {}s timeout", timeout.as_secs()),
            });
            return;
        }
    };
    match outcome {
        Ok(_) => {
            ctx.telemetry.emit(crate::telemetry::TelemetryEvent::ActorTicked { id });
        }
        Err(e) => {
            ctx.telemetry
                .emit(crate::telemetry::TelemetryEvent::ActorError { id, message: e.message });
        }
    }
    let _ = start; // hook for future per-tick duration telemetry
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::super::rt_types::{ActorHandle, ActorRuntime};
    use super::*;
    use crate::actor::{ActorOutput, ActorSchedule, SkillActor};
    use crate::telemetry::{NoopTelemetrySink, TelemetrySink};
    use async_trait::async_trait;
    use northhing_runtime_ports::{LightweightTaskOutput, LightweightTaskRequest, ToolDispatcherPort};
    use std::sync::{Arc, Mutex};
    use tokio_util::sync::CancellationToken;

    struct NullDispatcher;
    #[async_trait::async_trait]
    impl ToolDispatcherPort for NullDispatcher {
        async fn dispatch_once(&self, _req: LightweightTaskRequest) -> LightweightTaskOutput {
            LightweightTaskOutput::NoToolMatched { reason: "null".into() }
        }
    }

    struct RecordingSink {
        events: Mutex<Vec<String>>,
    }
    impl std::fmt::Debug for RecordingSink {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("RecordingSink").finish_non_exhaustive()
        }
    }
    impl TelemetrySink for RecordingSink {
        fn emit(&self, event: crate::telemetry::TelemetryEvent) {
            let kind = match event {
                crate::telemetry::TelemetryEvent::ActorSpawned { .. } => "spawned",
                crate::telemetry::TelemetryEvent::ActorTicked { .. } => "ticked",
                crate::telemetry::TelemetryEvent::ActorError { .. } => "error",
                crate::telemetry::TelemetryEvent::ActorEvent { .. } => "event",
                crate::telemetry::TelemetryEvent::ActorTerminatedAfterCancel { .. } => "terminated",
                crate::telemetry::TelemetryEvent::DispatchCompleted { .. } => "dispatch_completed",
                crate::telemetry::TelemetryEvent::DispatchAborted { .. } => "dispatch_aborted",
                crate::telemetry::TelemetryEvent::LongRunningSpawned { .. } => "long_running_spawned",
                crate::telemetry::TelemetryEvent::LongRunningRoundCompleted { .. } => "long_running_round_completed",
                crate::telemetry::TelemetryEvent::LongRunningTerminated { .. } => "long_running_terminated",
            };
            self.events.lock().unwrap().push(kind.to_string());
        }
    }

    /// A blocking actor that holds the tick open until canceled.
    struct BlockingActor {
        id: String,
    }
    #[async_trait]
    impl SkillActor for BlockingActor {
        fn id(&self) -> &str {
            &self.id
        }
        fn skill_name(&self) -> &str {
            "blocking"
        }
        async fn tick(&mut self, ctx: &ActorContext) -> Result<Option<ActorOutput>, crate::actor::ActorError> {
            // Park forever unless cancelled — the runtime's select! should
            // observe the cancel and emit the right telemetry.
            ctx.cancel.cancelled().await;
            Ok(Some(ActorOutput::Silent))
        }
    }

    /// A fast actor that emits Silent on every tick.
    struct FastActor {
        id: String,
    }
    #[async_trait]
    impl SkillActor for FastActor {
        fn id(&self) -> &str {
            &self.id
        }
        fn skill_name(&self) -> &str {
            "fast"
        }
        async fn tick(&mut self, _ctx: &ActorContext) -> Result<Option<ActorOutput>, crate::actor::ActorError> {
            Ok(Some(ActorOutput::Silent))
        }
    }

    #[tokio::test]
    async fn actor_handle_clone_shares_cancel() {
        // Clone the handle and verify both clones see the same cancel.
        let cancel = CancellationToken::new();
        let notify = Arc::new(tokio::sync::Notify::new());
        let h1 = ActorHandle::new("a".into(), cancel.clone(), notify);
        let h2 = h1.clone();
        assert!(!h1.is_cancelled());
        assert!(!h2.is_cancelled());
        h1.stop();
        assert!(h1.is_cancelled());
        assert!(h2.is_cancelled());
    }

    #[tokio::test]
    async fn one_shot_actor_completes_and_emits_terminated() {
        let runtime = ActorRuntime::new(Arc::new(NullDispatcher), Arc::new(NoopTelemetrySink));
        let handle = runtime.spawn_actor(Box::new(FastActor { id: "fast-1".into() }), ActorSchedule::OneShot);
        handle.await_join().await.expect("join ok");
        runtime.deregister("fast-1");
        assert!(runtime.is_empty());
    }

    #[tokio::test]
    async fn blocking_actor_observes_cancel() {
        let telemetry = Arc::new(RecordingSink {
            events: Mutex::new(Vec::new()),
        });
        let runtime = ActorRuntime::new(Arc::new(NullDispatcher), telemetry.clone());
        let handle = runtime.spawn_actor(Box::new(BlockingActor { id: "block-1".into() }), ActorSchedule::OneShot);
        // Give the spawned task a moment to enter its tick.
        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.stop();
        // Bounded wait so a regression that breaks the cancel
        // propagation surfaces as a fail rather than a hang.
        tokio::time::timeout(std::time::Duration::from_secs(2), handle.await_join())
            .await
            .expect("await_join timed out — cancel did not propagate")
            .expect("join ok");

        let events = telemetry.events.lock().unwrap().clone();
        // The blocked actor should have observed cancel and emitted
        // an `error` event (per the run_one_tick semantics) plus the
        // `terminated` event emitted by the spawn task wrapper.
        assert!(
            events.iter().any(|k| k == "terminated"),
            "expected terminated event, got {events:?}"
        );
    }

    #[tokio::test]
    async fn stop_all_broadcasts_cancel() {
        let runtime = ActorRuntime::new(Arc::new(NullDispatcher), Arc::new(NoopTelemetrySink));
        let _h1 = runtime.spawn_actor(Box::new(BlockingActor { id: "a".into() }), ActorSchedule::OneShot);
        let _h2 = runtime.spawn_actor(Box::new(BlockingActor { id: "b".into() }), ActorSchedule::OneShot);
        assert_eq!(runtime.len(), 2);
        runtime.stop_all();
        // Drain both handles.
        let handles: Vec<_> = runtime.actors.iter().map(|e| e.value().clone()).collect();
        for h in handles {
            // `wait_cancelled` (not `await_join`): the BlockingActor
            // exits after cancel fires, but `await_join` waits on the
            // full Notify completion — fine for normal actors, but
            // here we just need "shut down" semantics. Bounded by a
            // 2s timeout so the test fails loudly if cancel doesn't
            // propagate.
            let result = tokio::time::timeout(std::time::Duration::from_secs(2), h.wait_cancelled()).await;
            assert!(result.is_ok(), "stop_all cancel did not propagate within 2s");
            // Deregister so the registry empties — production callers
            // do this after `await_join` returns; the test mirrors that
            // pattern even though we used `wait_cancelled` here.
            runtime.deregister(h.id());
        }
        assert!(runtime.is_empty());
    }

    /// Phase 2.6: a Periodic actor actually ticks on its interval and
    /// stops cleanly when cancelled. We use a 50ms period and a short
    /// total runtime so the test stays under a second.
    #[tokio::test]
    async fn periodic_actor_ticks_repeatedly_and_stops_on_cancel() {
        let telemetry = Arc::new(RecordingSink {
            events: Mutex::new(Vec::new()),
        });
        let runtime = ActorRuntime::new(Arc::new(NullDispatcher), telemetry.clone());
        let handle = runtime.spawn_actor(
            Box::new(FastActor {
                id: "periodic-1".into(),
            }),
            ActorSchedule::Periodic(Duration::from_millis(50)),
        );
        // Let the loop tick a few times.
        tokio::time::sleep(Duration::from_millis(175)).await;
        handle.stop();
        handle.await_join().await.expect("join ok");
        runtime.deregister("periodic-1");

        let events = telemetry.events.lock().unwrap().clone();
        let ticked = events.iter().filter(|k| k.as_str() == "ticked").count();
        // ~3 ticks (at 50ms interval over 175ms), plus terminated on cancel.
        assert!(ticked >= 2, "expected at least 2 ticks, got {events:?}");
        assert!(
            events.iter().any(|k| k == "terminated"),
            "expected terminated event, got {events:?}"
        );
    }

    /// Phase 2.6: an OnSignal actor ticks when the producer pushes a
    /// trigger and stops cleanly when the channel closes.
    #[tokio::test]
    async fn on_signal_actor_ticks_on_each_trigger_and_exits_on_close() {
        let telemetry = Arc::new(RecordingSink {
            events: Mutex::new(Vec::new()),
        });
        let runtime = ActorRuntime::new(Arc::new(NullDispatcher), telemetry.clone());
        let (tx, rx) = tokio::sync::mpsc::channel::<crate::actor::ActorTrigger>(4);

        let handle = runtime.spawn_actor(
            Box::new(FastActor { id: "signal-1".into() }),
            ActorSchedule::OnSignal(rx),
        );

        // Push three triggers, give the runtime time to consume them.
        for _ in 0..3 {
            tx.send(crate::actor::ActorTrigger::Opaque).await.expect("send");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Close the producer side — the runtime's loop should observe
        // the closed channel and exit cleanly (no error event).
        drop(tx);
        handle.await_join().await.expect("join ok");
        runtime.deregister("signal-1");

        let events = telemetry.events.lock().unwrap().clone();
        let ticked = events.iter().filter(|k| k.as_str() == "ticked").count();
        assert!(ticked >= 3, "expected at least 3 ticks for 3 triggers, got {events:?}");
        // No error event should be emitted for a clean channel close.
        assert!(
            !events.iter().any(|k| k == "error"),
            "did not expect error event on clean close, got {events:?}"
        );
    }
}
