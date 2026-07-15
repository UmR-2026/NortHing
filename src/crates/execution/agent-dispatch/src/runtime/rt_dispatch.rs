//! `ActorRuntime` spawn entry points.
//!
//! Three entry points live here, all of which attach a fresh
//! `CancellationToken` + `Notify` and dispatch into the per-tick helper
//! [`super::rt_handlers::run_one_tick`]:
//!
//! - [`ActorRuntime::spawn_actor`] — generic dispatcher-driven spawn
//!   honouring the requested [`crate::actor::ActorSchedule`] (OneShot /
//!   Periodic / OnSignal).
//! - [`ActorRuntime::spawn_one_shot`] — closure-based one-shot sugar
//!   for callers that don't want to implement [`crate::actor::SkillActor`]
//!   just to run a single tick.
//! - [`ActorRuntime::spawn_long_running`] — multi-turn dispatch loop
//!   for [`crate::long_running::LongRunningSkill`].
//!
//! Visibility: this file reads `pub(super)` fields from
//! [`super::rt_types::ActorRuntime`] (dispatcher, telemetry,
//! default_tick_timeout, handle) and inserts into the `actors` map.
//! The actual scheduling bodies (`tokio::select!` arms) remain inline
//! because splitting them further would push them below the
//! cognitive-load threshold without a runtime invariant to anchor on.

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::actor::{ActorContext, ActorError, ActorOutput, ActorSchedule, SkillActor};
use crate::long_running::{LongRunningRequest, LongRunningSkill, LongRunningTickOutput, DEFAULT_MAX_ROUNDS};
use crate::telemetry::TelemetryEvent;
use northhing_runtime_ports::LightweightTaskOutput;

use super::rt_handlers::run_one_tick;
use super::rt_types::{ActorHandle, ActorRuntime};

impl ActorRuntime {
    /// Spawn a new actor and start ticking it on the configured
    /// schedule. The runtime takes ownership of the boxed actor and
    /// returns an `ActorHandle` for later shutdown.
    ///
    /// **Phase 2 partial body**: only `OneShot` is fully implemented.
    /// `Periodic(Duration)` runs a single tick and exits — the
    /// scheduler loop lands in Phase 2.6 alongside the registration
    /// wiring (`SkillRuntime::register_async`).
    pub fn spawn_actor(&self, mut actor: Box<dyn SkillActor>, schedule: ActorSchedule) -> ActorHandle {
        let id = actor.id().to_string();
        let cancel = CancellationToken::new();
        let dispatcher = Arc::clone(&self.dispatcher);
        let telemetry = Arc::clone(&self.telemetry);
        let default_tick_timeout = self.default_tick_timeout;
        let handle = Arc::clone(&self.handle);

        let actor_id_for_task = id.clone();
        let cancel_for_task = cancel.clone();
        // Phase I.1 (2026-06-20): use `Notify` instead of `JoinHandle`
        // so clones of the returned `ActorHandle` (e.g. the registry
        // keeps one) can all await completion. The task fires
        // `notify_one()` on its way out below.
        let notify: Arc<tokio::sync::Notify> = Arc::new(tokio::sync::Notify::new());
        let notify_for_task = Arc::clone(&notify);
        handle.spawn(async move {
            // Construct the per-tick ActorContext once and reuse it
            // across ticks. The cancel token is the same one returned
            // to the caller, so cancelling from outside is observed
            // by `tokio::select!` below.
            let ctx = ActorContext {
                tool_dispatcher: Arc::clone(&dispatcher),
                cancel: cancel_for_task.clone(),
                telemetry: Arc::clone(&telemetry),
            };

            match schedule {
                ActorSchedule::OneShot => {
                    run_one_tick(&mut actor, &ctx, default_tick_timeout).await;
                }
                ActorSchedule::Periodic(period) => {
                    // Phase 2.6 (impl plan 2.3): real scheduler loop.
                    // Tick on the configured interval, observing cancel
                    // both inside each tick (via run_one_tick's select!)
                    // and between ticks (via tokio::select! over
                    // tokio::time::sleep vs cancel.cancelled()). When
                    // cancel fires we exit immediately and emit the
                    // terminated telemetry.
                    loop {
                        tokio::select! {
                            _ = cancel_for_task.cancelled() => {
                                telemetry.emit(
                                    TelemetryEvent::ActorError {
                                        id: actor_id_for_task.clone(),
                                        message: "periodic actor cancelled between ticks".into(),
                                    },
                                );
                                break;
                            }
                            _ = tokio::time::sleep(period) => {
                                run_one_tick(&mut actor, &ctx, default_tick_timeout).await;
                            }
                        }
                    }
                }
                ActorSchedule::OnSignal(mut receiver) => {
                    // Phase 2.6 (impl plan 2.3 + 3.x): real channel
                    // plumbing. Receive on the channel until cancel
                    // fires or the channel is closed by the producer;
                    // each received trigger runs one tick. The
                    // trigger payload is opaque (see `ActorTrigger`)
                    // — Phase 3 will type it once the subagent routing
                    // code lands.
                    loop {
                        tokio::select! {
                            biased;
                            _ = cancel_for_task.cancelled() => {
                                telemetry.emit(
                                    TelemetryEvent::ActorError {
                                        id: actor_id_for_task.clone(),
                                        message: "signal actor cancelled".into(),
                                    },
                                );
                                break;
                            }
                            maybe_trigger = receiver.recv() => {
                                match maybe_trigger {
                                    Some(_trigger) => {
                                        run_one_tick(&mut actor, &ctx, default_tick_timeout).await;
                                    }
                                    None => {
                                        // Channel closed by producer —
                                        // exit cleanly. Emit terminated
                                        // (no error) since this is a
                                        // normal shutdown path.
                                        telemetry.emit(
                                            TelemetryEvent::ActorTerminatedAfterCancel {
                                                id: actor_id_for_task.clone(),
                                            },
                                        );
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            telemetry.emit(TelemetryEvent::ActorTerminatedAfterCancel { id: actor_id_for_task });

            // Phase I.1: signal completion to every handle clone.
            // `notify_waiters()` wakes any currently-waiting
            // `notified()` future; future waits will be satisfied
            // by the default `Notified` permit.
            notify_for_task.notify_waiters();
        });

        let handle = ActorHandle::new(id.clone(), cancel, notify);
        self.actors.insert(id, handle.clone());
        handle
    }

    /// Phase I.x (2026-06-20, A3): one-shot closure-based actor
    /// spawn. Wraps a `FnMut(&ActorContext) -> Result<Option<ActorOutput>, ActorError>`
    /// in a fresh `SkillActor` impl and spawns it under `OneShot`
    /// schedule. Used by `app_state::on_send_message` to demonstrate
    /// the runtime path end-to-end without requiring callers to
    /// define a new `SkillActor` impl per use site.
    ///
    /// The closure is `FnMut` (not `FnOnce`) because the actor
    /// runtime may retry the tick on a future scheduling change; the
    /// current `OneShot` path only calls it once, so the
    /// `FnMut` bound is purely for future-proofing.
    pub fn spawn_one_shot<F>(&self, body: F) -> ActorHandle
    where
        F: FnMut(&ActorContext) -> Result<Option<ActorOutput>, ActorError> + Send + Sync + 'static,
    {
        // Unique id so the registry doesn't collide if the same
        // call site spawns multiple one-shots.
        let id = format!("one-shot-{}", uuid::Uuid::new_v4());
        struct ClosureActor<F> {
            id: String,
            body: F,
        }
        #[async_trait::async_trait]
        impl<F> SkillActor for ClosureActor<F>
        where
            F: FnMut(&ActorContext) -> Result<Option<ActorOutput>, ActorError> + Send + Sync + 'static,
        {
            fn id(&self) -> &str {
                &self.id
            }
            fn skill_name(&self) -> &str {
                "one_shot_closure"
            }
            async fn tick(&mut self, ctx: &ActorContext) -> Result<Option<ActorOutput>, ActorError> {
                (self.body)(ctx)
            }
        }
        self.spawn_actor(Box::new(ClosureActor { id: id.clone(), body }), ActorSchedule::OneShot)
    }

    /// Phase A1 (K.2.3): spawn a `LongRunningSkill` and drive its
    /// multi-turn loop until the skill returns `Done` or the
    /// cancel token fires.
    ///
    /// Unlike `spawn_actor` (which returns `ActorHandle` for "ticks
    /// forever"), this returns the bare `JoinHandle` because the
    /// task ends naturally on `Done` and the caller wants the
    /// `Result<LightweightTaskOutput, ActorError>` return value.
    ///
    /// Loop semantics (see `long_running` module docs for invariants):
    /// 1. Cap check BEFORE the tick so a runaway skill can't start
    ///    its N+1-th round.
    /// 2. Skill tick under `tokio::select!` with cancel observation.
    /// 3. On `Continue { next_request }`: dispatch under
    ///    `tokio::select!` with cancel observation. Emit
    ///    `LongRunningRoundCompleted`. Increment round counter.
    /// 4. On `Done { final_output }`: break with `Ok(final_output)`.
    /// 5. Emit `LongRunningTerminated` on exit (success or failure).
    pub fn spawn_long_running(
        &self,
        mut skill: Box<dyn LongRunningSkill>,
        _initial_request: LongRunningRequest,
    ) -> tokio::task::JoinHandle<Result<LightweightTaskOutput, ActorError>> {
        let id = skill.id().to_string();
        let cancel = CancellationToken::new();
        let dispatcher = Arc::clone(&self.dispatcher);
        let telemetry = Arc::clone(&self.telemetry);
        let max_rounds = DEFAULT_MAX_ROUNDS;
        let handle = Arc::clone(&self.handle);

        telemetry.emit(TelemetryEvent::LongRunningSpawned { id: id.clone() });

        handle.spawn(async move {
            let ctx = ActorContext {
                tool_dispatcher: dispatcher,
                cancel: cancel.clone(),
                telemetry: Arc::clone(&telemetry),
            };

            let mut prior: Option<LightweightTaskOutput> = None;
            let mut rounds: u32 = 0;
            let result: Result<LightweightTaskOutput, ActorError> = loop {
                // Cap check BEFORE the tick so a runaway skill can't
                // even start its N+1-th round.
                if rounds >= max_rounds {
                    cancel.cancel();
                    break Err(ActorError::new(format!(
                        "LongRunningSkill '{id}' exceeded max_rounds={max_rounds}"
                    )));
                }

                // Skill tick under cancel observation.
                let tick_outcome = tokio::select! {
                    biased;
                    _ = ctx.cancel.cancelled() => {
                        Err(ActorError::new(format!(
                            "LongRunningSkill '{id}' cancelled"
                        )))
                    }
                    out = skill.tick(&ctx, prior.take()) => out,
                };
                let tick_outcome = match tick_outcome {
                    Ok(o) => o,
                    Err(e) => break Err(e),
                };

                match tick_outcome {
                    LongRunningTickOutput::Continue { next_request } => {
                        // Dispatch one LLM call under cancel observation.
                        let req = next_request.0;
                        let dispatched = tokio::select! {
                            biased;
                            _ = ctx.cancel.cancelled() => {
                                break Err(ActorError::new(format!(
                                    "LongRunningSkill '{id}' cancelled during dispatch"
                                )));
                            }
                            out = ctx.tool_dispatcher.dispatch_once(req) => out,
                        };
                        telemetry.emit(TelemetryEvent::LongRunningRoundCompleted {
                            id: id.clone(),
                            round: rounds,
                        });
                        rounds += 1;
                        prior = Some(dispatched);
                    }
                    LongRunningTickOutput::Done { final_output } => {
                        break Ok(final_output);
                    }
                }
            };

            telemetry.emit(TelemetryEvent::LongRunningTerminated {
                id: id.clone(),
                reason: match &result {
                    Ok(_) => "done".to_string(),
                    Err(e) => e.message.clone(),
                },
            });
            result
        })
    }
}
