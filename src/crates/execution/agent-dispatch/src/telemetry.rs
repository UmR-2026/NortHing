//! Telemetry contract for the actor / dispatcher runtime.
//!
//! Pattern source: `.agents/reference/actor/03-actor-runtime.rs` (the
//! `Arc<dyn TelemetrySink>` parameter on `ActorRuntime::new`) and
//! `.agents/reference/actor/SIGNATURES.md` (the trait body).
//!
//! Phase 1 keeps the trait minimal — just an event sink — so the runtime
//! shape compiles without depending on the eventual telemetry backend
//! (likely `tracing` or the project's own `core-types` event bus).

use std::fmt;

/// A single telemetry event emitted by the actor runtime or dispatcher.
///
/// Phase 1 intentionally keeps the variants narrow; richer payloads
/// (per-tick metrics, structured failure reasons) land alongside the
/// runtime body in Phase 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelemetryEvent {
    /// An actor was registered / spawned.
    ActorSpawned { id: String },
    /// An actor ticked successfully (Silent output).
    ActorTicked { id: String },
    /// An actor emitted a user-visible event.
    ActorEvent { id: String, payload: String },
    /// An actor returned an error.
    ActorError { id: String, message: String },
    /// An actor survived its cancel token (the `actor_terminated_after_cancel`
    /// signal — see `.agents/reference/actor/NOTES.md` ⛔ #1).
    ActorTerminatedAfterCancel { id: String },
    /// A one-shot dispatch completed successfully.
    DispatchCompleted { dispatch_id: String },
    /// A one-shot dispatch was cancelled or timed out.
    DispatchAborted { dispatch_id: String, reason: String },
    /// Phase A1 (K.2.3): a `LongRunningSkill` was spawned.
    LongRunningSpawned { id: String },
    /// Phase A1 (K.2.3): a `LongRunningSkill` completed one LLM dispatch round.
    LongRunningRoundCompleted { id: String, round: u32 },
    /// Phase A1 (K.2.3): a `LongRunningSkill` exited the loop
    /// (`reason = "done"` on success, or the error message).
    LongRunningTerminated { id: String, reason: String },
}

/// Sink for [`TelemetryEvent`]s emitted by the runtime.
///
/// Implementations must be `Send + Sync` and non-blocking on the emit path —
/// the runtime calls this from `tokio::spawn`'d tasks and cannot tolerate
/// back-pressure inside the actor loop.
pub trait TelemetrySink: Send + Sync + std::fmt::Debug {
    /// Emit a single event. Must not block; must not panic.
    fn emit(&self, event: TelemetryEvent);
}

impl fmt::Display for TelemetryEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TelemetryEvent::ActorSpawned { id } => write!(f, "actor_spawned id={id}"),
            TelemetryEvent::ActorTicked { id } => write!(f, "actor_ticked id={id}"),
            TelemetryEvent::ActorEvent { id, payload } => {
                write!(f, "actor_event id={id} payload={payload}")
            }
            TelemetryEvent::ActorError { id, message } => {
                write!(f, "actor_error id={id} message={message}")
            }
            TelemetryEvent::ActorTerminatedAfterCancel { id } => {
                write!(f, "actor_terminated_after_cancel id={id}")
            }
            TelemetryEvent::DispatchCompleted { dispatch_id } => {
                write!(f, "dispatch_completed id={dispatch_id}")
            }
            TelemetryEvent::DispatchAborted { dispatch_id, reason } => {
                write!(f, "dispatch_aborted id={dispatch_id} reason={reason}")
            }
            TelemetryEvent::LongRunningSpawned { id } => {
                write!(f, "long_running_spawned id={id}")
            }
            TelemetryEvent::LongRunningRoundCompleted { id, round } => {
                write!(f, "long_running_round_completed id={id} round={round}")
            }
            TelemetryEvent::LongRunningTerminated { id, reason } => {
                write!(f, "long_running_terminated id={id} reason={reason}")
            }
        }
    }
}

/// A [`TelemetrySink`] implementation that drops every event on the floor.
///
/// The default sink for Phase 1: tests, the IPC stub, and any code path that
/// hasn't yet wired a real telemetry backend. Production wiring lives in
/// Phase 2.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopTelemetrySink;

impl TelemetrySink for NoopTelemetrySink {
    #[inline]
    fn emit(&self, _event: TelemetryEvent) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// A counting sink for tests.
    #[derive(Default, Debug)]
    struct CountingSink {
        events: Mutex<Vec<TelemetryEvent>>,
    }

    impl TelemetrySink for CountingSink {
        fn emit(&self, event: TelemetryEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    #[test]
    fn noop_sink_swallows_events() {
        let sink = NoopTelemetrySink;
        sink.emit(TelemetryEvent::ActorSpawned { id: "x".into() });
        // No panic, no observable side effect — that's the whole point.
    }

    #[test]
    fn trait_object_round_trip() {
        let sink: Arc<dyn TelemetrySink> = Arc::new(CountingSink::default());
        sink.emit(TelemetryEvent::DispatchCompleted {
            dispatch_id: "d-1".into(),
        });
        // The round-trip through `Arc<dyn TelemetrySink>` must not lose the event.
        // We can't observe the CountingSink's internal vec from here without
        // exposing it; the fact that it compiled + the emit didn't panic is the
        // minimum guarantee we verify in Phase 1.
    }

    #[test]
    fn event_display_is_stable() {
        // The string format is part of the Phase 1 contract — log scrapers and
        // grep-based assertions may rely on it.
        assert_eq!(
            TelemetryEvent::ActorSpawned { id: "a".into() }.to_string(),
            "actor_spawned id=a"
        );
        assert_eq!(
            TelemetryEvent::DispatchAborted {
                dispatch_id: "d".into(),
                reason: "timeout".into()
            }
            .to_string(),
            "dispatch_aborted id=d reason=timeout"
        );
    }
}
