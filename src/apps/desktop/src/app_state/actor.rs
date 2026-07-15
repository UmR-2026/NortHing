//! actor module — see mod.rs for the wiring entry point.

use super::log::log_debug_event;
use super::slint_glue::AppWindow;
use super::*;

/// Phase I.3 (2026-06-20): construct an `ActorRuntime` at app boot
/// when `USE_LIGHTWEIGHT_ACTOR` is true, and register a heartbeat
/// actor that ticks once per minute.
///
/// This is the **first** real call site for the actor runtime — it
/// doesn't replace any existing production path yet. The heartbeat
/// actor's `tick` is a no-op; the point is to prove the runtime is
/// live, that the `SkillActor` impl compiles, and that the
/// `TelemetrySink` wired into `AppState` receives an `ActorTicked`
/// event per tick. Manual tests can `grep '"component":"actor_runtime"'`
/// in `.northhing/debug.log` to confirm.
///
/// The runtime is owned by `AppState` (`actor_runtime: OnceLock<...>`).
/// Future Phase I.x work can replace `ConversationCoordinator::execute_hidden_subagent_internal`
/// with `state.actor_runtime().spawn_actor(...)`.
pub(super) fn maybe_construct_actor_runtime(app_state: &AppState, ui: &AppWindow) {
    use async_trait::async_trait;
    use northhing_agent_dispatch::{
        ActorContext, ActorOutput, ActorSchedule, ActorTrigger, NoopTelemetrySink, SkillActor, TelemetryEvent,
        TelemetrySink, USE_LIGHTWEIGHT_ACTOR,
    };
    use northhing_runtime_ports::{LightweightTaskOutput, LightweightTaskRequest, ToolDispatcherPort};
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;

    if !USE_LIGHTWEIGHT_ACTOR {
        return;
    }

    // Phase I.3 (2026-06-20): trivial `ToolDispatcher` stub. Phase I.x
    // will wire a real coordinator-backed dispatcher.
    struct NullDispatcher;
    #[async_trait]
    impl ToolDispatcherPort for NullDispatcher {
        async fn dispatch_once(&self, _req: LightweightTaskRequest) -> LightweightTaskOutput {
            LightweightTaskOutput::NoToolMatched {
                reason: "phase-i3-stub".into(),
            }
        }
    }

    /// Phase I.3 heartbeat actor. Demonstrates the full path:
    /// `SkillActor` impl → `ActorRuntime::spawn_actor` →
    /// `ActorTicked` telemetry → debug log.
    struct HeartbeatActor {
        id: String,
    }
    #[async_trait]
    impl SkillActor for HeartbeatActor {
        fn id(&self) -> &str {
            &self.id
        }
        fn skill_name(&self) -> &str {
            "heartbeat"
        }
        async fn tick(
            &mut self,
            ctx: &ActorContext,
        ) -> Result<Option<ActorOutput>, northhing_agent_dispatch::ActorError> {
            // Phase H (actor_runtime component): record the heartbeat
            // so manual tests can confirm the runtime is alive.
            log_debug_event(
                northhing_core::infrastructure::debug_log::COMP_ACTOR_RUNTIME,
                "actor::heartbeat:tick",
                crate::flags::DEFAULT_MODE_ID,
                "heartbeat actor tick",
                Some([
                    ("actor_id", self.id.clone()),
                    ("", String::new()),
                    ("", String::new()),
                    ("", String::new()),
                ]),
            );
            ctx.telemetry.emit(TelemetryEvent::ActorTicked { id: self.id.clone() });
            Ok(Some(ActorOutput::Silent))
        }
    }

    let dispatcher: Arc<dyn ToolDispatcherPort> = Arc::new(NullDispatcher);
    let telemetry: Arc<dyn TelemetrySink> = Arc::new(NoopTelemetrySink);

    let runtime = northhing_agent_dispatch::ActorRuntime::new(dispatcher, telemetry);
    let handle = runtime.spawn_actor(
        Box::new(HeartbeatActor { id: "heartbeat".into() }),
        ActorSchedule::OneShot,
    );
    // Detach the JoinHandle — the actor will run to completion on the
    // current tokio runtime and exit. A future Phase I.x can register
    // a Periodic actor instead.
    drop(handle);

    let runtime_arc = Arc::new(runtime);
    app_state.set_actor_runtime(runtime_arc.clone());
    // K.2.3 follow-up: forward the runtime to the coordinator's
    // ToolPipeline so TaskTool sees it via ToolUseContext.
    if let Some(coordinator) = app_state.coordinator() {
        coordinator.set_actor_runtime(runtime_arc);
    }
    let _ = ui; // reserved for future Phase I.x Periodic actor wiring

    // Phase H: log the activation so manual tests can grep for it.
    log_debug_event(
        northhing_core::infrastructure::debug_log::COMP_ACTOR_RUNTIME,
        "actor::runtime:activated",
        crate::flags::DEFAULT_MODE_ID,
        "ActorRuntime constructed at app boot",
        Some([
            ("flag", "USE_LIGHTWEIGHT_ACTOR".into()),
            ("value", "true".into()),
            ("actors", "1".into()),
            ("", String::new()),
        ]),
    );
    let _ = (CancellationToken::new(), ActorTrigger::Opaque); // silence unused
}
