//! Phase A1: long-running multi-turn LLM skills.
//!
//! ## Invariants (carry-over from `SkillActor`, enforced by convention
//! not by the runtime)
//!
//! 1. `LongRunningSkill::tick` MUST NOT call any LLM directly. Every
//!    LLM call goes through `ctx.tool_dispatcher.dispatch_once(req)`.
//!    The trait surface has no `LlmClient` field — only
//!    `Arc<dyn ToolDispatcherPort>` via `ActorContext` — so there's
//!    no LLM to call from inside `tick` (SkillActor invariant #1).
//! 2. `tick` MUST observe `ctx.cancel` on every blocking call.
//!    The runtime observes cancel at three boundaries: inside `tick`
//!    (skill responsibility), inside `dispatch_once` (runtime's
//!    `tokio::select!`), and at the `max_rounds` cap (runtime fires
//!    `ctx.cancel` and the next `tick` sees it fired).
//! 3. `tick` returns `Continue` to drive the next round or `Done`
//!    to stop and return the final output.
//! 4. The runtime caps round count at `max_rounds` (default 16)
//!    to prevent runaway loops.
//!
//! ## Spec source
//!
//! `docs/superpowers/specs/2026-06-21-k2-3-long-running-skill-design.md` §3.3.

use async_trait::async_trait;

use northhing_runtime_ports::{LightweightTaskOutput, LightweightTaskRequest};

use crate::actor::{ActorContext, ActorError};

/// Maximum rounds a `LongRunningSkill` is allowed to drive before
/// the runtime forces exit with `ActorError`. Exposed as a
/// `pub const` (not `pub static`) so it's a true constant — callers
/// who need per-skill caps (Phase A2+) can wrap their own runtime.
///
/// Lives in this module (the trait module) rather than `runtime.rs`
/// because the cap is a property of the *skill protocol*, not of
/// the runtime — different runtimes (single-threaded, distributed)
/// would all enforce the same cap.
pub const DEFAULT_MAX_ROUNDS: u32 = 16;

/// A wrapper around `LightweightTaskRequest` that exists so future
/// phases can add long-running-only fields (intermediate scratchpad,
/// retry policy, round counter) without breaking the trait
/// signature. A1 carries no extra fields.
///
/// **Not** `PartialEq` / `Eq` because the inner
/// `LightweightTaskRequest` is not `PartialEq` (it carries non-Eq
/// types like `Option<CancellationToken>`).
#[derive(Debug, Clone)]
pub struct LongRunningRequest(pub LightweightTaskRequest);

/// What `LongRunningSkill::tick` returns to the runtime.
///
/// **Not** `PartialEq` / `Eq` — see `LongRunningRequest` for why.
#[derive(Debug, Clone)]
pub enum LongRunningTickOutput {
    /// Drive another round. The runtime will call
    /// `ctx.tool_dispatcher.dispatch_once(next_request.0)` and feed
    /// the result into the next `tick` call.
    Continue { next_request: LongRunningRequest },
    /// Stop the loop; `final_output` is the spawn's `Ok` result.
    Done { final_output: LightweightTaskOutput },
}

/// Phase A1 multi-turn LLM skill.
///
/// Parallel to `SkillActor` (per design spec option A — see
/// `.agents/reference/actor/NOTES.md`). All four `SkillActor`
/// invariants carry over; see the module-level docs.
#[async_trait]
pub trait LongRunningSkill: Send + Sync {
    /// Stable id used for telemetry correlation (mirrors `SkillActor::id`).
    fn id(&self) -> &str;
    /// Skill name (mirrors `SkillActor::skill_name`).
    fn skill_name(&self) -> &str;

    /// Drive one round of the multi-turn loop.
    ///
    /// `prior_output` is `None` on the first tick; on subsequent ticks
    /// it carries the result of the previous
    /// `ctx.tool_dispatcher.dispatch_once(...)` call.
    ///
    /// Return `Continue { next_request }` to drive another round;
    /// return `Done { final_output }` to stop the loop and return
    /// `final_output` as the spawn's result.
    async fn tick(
        &mut self,
        ctx: &ActorContext,
        prior_output: Option<LightweightTaskOutput>,
    ) -> Result<LongRunningTickOutput, ActorError>;
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::ActorContext;
    use crate::runtime::ActorRuntime;
    use crate::telemetry::{TelemetryEvent, TelemetrySink};
    use northhing_runtime_ports::ToolDispatcherPort;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// A `ToolDispatcherPort` impl that echoes `req.user_prompt`
    /// back as the output. Lets the tests verify how many dispatcher
    /// calls happened (by inspecting `dispatch_id`).
    struct EchoDispatcher {
        call_log: Arc<Mutex<Vec<String>>>,
    }
    #[async_trait::async_trait]
    impl ToolDispatcherPort for EchoDispatcher {
        async fn dispatch_once(&self, req: LightweightTaskRequest) -> LightweightTaskOutput {
            self.call_log.lock().unwrap().push(req.dispatch_id.clone());
            LightweightTaskOutput::ToolResult {
                tool_name: "echo".into(),
                output: req.user_prompt,
            }
        }
    }

    /// Minimal no-op dispatcher for tests that don't care about
    /// dispatch calls.
    struct NoopDispatcher;
    #[async_trait::async_trait]
    impl ToolDispatcherPort for NoopDispatcher {
        async fn dispatch_once(&self, _req: LightweightTaskRequest) -> LightweightTaskOutput {
            LightweightTaskOutput::NoToolMatched { reason: "noop".into() }
        }
    }

    /// A `TelemetrySink` that records only the 3 long-running
    /// events. Used by tests to assert on telemetry sequencing.
    #[derive(Default)]
    struct TestSink {
        events: Mutex<Vec<(String, String, String)>>,
    }
    impl std::fmt::Debug for TestSink {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TestSink").finish_non_exhaustive()
        }
    }
    impl TelemetrySink for TestSink {
        fn emit(&self, event: TelemetryEvent) {
            let (kind, id, extra) = match event {
                TelemetryEvent::LongRunningSpawned { id } => ("spawned".into(), id, String::new()),
                TelemetryEvent::LongRunningRoundCompleted { id, round } => ("round".into(), id, round.to_string()),
                TelemetryEvent::LongRunningTerminated { id, reason } => ("terminated".into(), id, reason),
                _ => return, // ignore other events in long-running tests
            };
            self.events.lock().unwrap().push((kind, id, extra));
        }
    }

    /// A skill that returns `Done` on its very first tick with a
    /// fixed final output. No dispatcher calls happen.
    struct DoneImmediately {
        id: String,
        final_output: LightweightTaskOutput,
    }
    #[async_trait]
    impl LongRunningSkill for DoneImmediately {
        fn id(&self) -> &str {
            &self.id
        }
        fn skill_name(&self) -> &str {
            "done_immediately"
        }
        async fn tick(
            &mut self,
            _ctx: &ActorContext,
            _prior: Option<LightweightTaskOutput>,
        ) -> Result<LongRunningTickOutput, ActorError> {
            Ok(LongRunningTickOutput::Done {
                final_output: self.final_output.clone(),
            })
        }
    }

    /// A skill that drives exactly `target_rounds` rounds before
    /// returning `Done`. Each round's dispatch_id is `"round-{n}"`.
    struct NthRoundDone {
        id: String,
        target_rounds: u32,
        round_count: u32,
    }
    #[async_trait]
    impl LongRunningSkill for NthRoundDone {
        fn id(&self) -> &str {
            &self.id
        }
        fn skill_name(&self) -> &str {
            "nth_round_done"
        }
        async fn tick(
            &mut self,
            _ctx: &ActorContext,
            _prior: Option<LightweightTaskOutput>,
        ) -> Result<LongRunningTickOutput, ActorError> {
            // Each tick that returns Continue triggers one more
            // round. We count those ticks and stop at target_rounds.
            if self.round_count >= self.target_rounds {
                Ok(LongRunningTickOutput::Done {
                    final_output: LightweightTaskOutput::ToolResult {
                        tool_name: "final".into(),
                        output: format!("completed after {} rounds", self.round_count),
                    },
                })
            } else {
                let round_n = self.round_count;
                self.round_count += 1;
                Ok(LongRunningTickOutput::Continue {
                    next_request: LongRunningRequest(LightweightTaskRequest {
                        dispatch_id: format!("round-{round_n}"),
                        user_prompt: format!("round {round_n}"),
                        prepended_context: vec![],
                        tool_allowlist: vec![],
                        timeout: Some(Duration::from_secs(5)),
                        cancel: None,
                        telemetry: None,
                    }),
                })
            }
        }
    }

    /// Skill that always returns `Continue` — drives forever until
    /// the runtime's `max_rounds` cap fires.
    struct AlwaysContinue {
        id: String,
        call_count: u32,
    }
    #[async_trait]
    impl LongRunningSkill for AlwaysContinue {
        fn id(&self) -> &str {
            &self.id
        }
        fn skill_name(&self) -> &str {
            "always_continue"
        }
        async fn tick(
            &mut self,
            _ctx: &ActorContext,
            _prior: Option<LightweightTaskOutput>,
        ) -> Result<LongRunningTickOutput, ActorError> {
            self.call_count += 1;
            Ok(LongRunningTickOutput::Continue {
                next_request: LongRunningRequest(LightweightTaskRequest {
                    dispatch_id: format!("ac-{}", self.call_count),
                    user_prompt: format!("ac {}", self.call_count),
                    prepended_context: vec![],
                    tool_allowlist: vec![],
                    timeout: Some(Duration::from_secs(5)),
                    cancel: None,
                    telemetry: None,
                }),
            })
        }
    }

    /// Skill that succeeds on first tick (drives 1 dispatch) then
    /// returns `Err` on the second tick. Used to verify the skill's
    /// `Err` propagates through the spawn's `Result` and that
    /// dispatch stops after the error.
    struct ErringSkill {
        id: String,
    }
    #[async_trait]
    impl LongRunningSkill for ErringSkill {
        fn id(&self) -> &str {
            &self.id
        }
        fn skill_name(&self) -> &str {
            "erring"
        }
        async fn tick(
            &mut self,
            _ctx: &ActorContext,
            prior: Option<LightweightTaskOutput>,
        ) -> Result<LongRunningTickOutput, ActorError> {
            if prior.is_none() {
                Ok(LongRunningTickOutput::Continue {
                    next_request: LongRunningRequest(LightweightTaskRequest {
                        dispatch_id: "e1".into(),
                        user_prompt: "e1".into(),
                        prepended_context: vec![],
                        tool_allowlist: vec![],
                        timeout: Some(Duration::from_secs(5)),
                        cancel: None,
                        telemetry: None,
                    }),
                })
            } else {
                Err(ActorError::new("skill reports error"))
            }
        }
    }

    fn make_runtime(dispatcher: Arc<dyn ToolDispatcherPort>, sink: Arc<dyn TelemetrySink>) -> ActorRuntime {
        ActorRuntime::new(dispatcher, sink)
    }

    fn req(prompt: &str) -> LongRunningRequest {
        LongRunningRequest(LightweightTaskRequest {
            dispatch_id: "initial".into(),
            user_prompt: prompt.into(),
            prepended_context: vec![],
            tool_allowlist: vec![],
            timeout: Some(Duration::from_secs(5)),
            cancel: None,
            telemetry: None,
        })
    }

    /// Test 1: skill that returns Done on first tick → 0 dispatcher
    /// calls, spawn resolves immediately with the skill's output,
    /// exactly 2 telemetry events (spawned + terminated with reason="done").
    #[tokio::test]
    async fn done_immediately_skips_dispatcher() {
        let sink: Arc<TestSink> = Arc::new(TestSink::default());
        let sink_dyn: Arc<dyn TelemetrySink> = sink.clone();
        let runtime = make_runtime(Arc::new(NoopDispatcher), sink_dyn);
        let final_output = LightweightTaskOutput::ToolResult {
            tool_name: "final".into(),
            output: "all done".into(),
        };
        let join = runtime.spawn_long_running(
            Box::new(DoneImmediately {
                id: "t1".into(),
                final_output: final_output.clone(),
            }),
            req("initial"),
        );
        let out = join.await.expect("join ok").expect("skill ok");
        assert_eq!(out, final_output);

        let events = sink.events.lock().unwrap().clone();
        assert_eq!(events.len(), 2, "expected spawned + terminated, got {events:?}");
        assert_eq!(events[0].0, "spawned");
        assert_eq!(events[1].0, "terminated");
        assert_eq!(events[1].2, "done");
    }

    /// Test 2: skill that drives 3 rounds → exactly 3 dispatcher
    /// calls + 3 round-completed events + 2 boundary events (spawned + terminated).
    #[tokio::test]
    async fn nth_round_done_drives_n_rounds() {
        let call_log = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink: Arc<TestSink> = Arc::new(TestSink::default());
        let sink_dyn: Arc<dyn TelemetrySink> = sink.clone();
        let runtime = make_runtime(
            Arc::new(EchoDispatcher {
                call_log: call_log.clone(),
            }),
            sink_dyn,
        );
        let join = runtime.spawn_long_running(
            Box::new(NthRoundDone {
                id: "t2".into(),
                target_rounds: 3,
                round_count: 0,
            }),
            req("initial"),
        );
        let out = join.await.expect("join ok").expect("skill ok");
        assert_eq!(
            out,
            LightweightTaskOutput::ToolResult {
                tool_name: "final".into(),
                output: "completed after 3 rounds".into(),
            }
        );

        let log = call_log.lock().unwrap().clone();
        assert_eq!(
            log,
            vec!["round-0".to_string(), "round-1".to_string(), "round-2".to_string()],
            "expected exactly 3 dispatches in order, got {log:?}"
        );

        let events = sink.events.lock().unwrap().clone();
        let rounds = events.iter().filter(|(k, _, _)| k == "round").count();
        assert_eq!(rounds, 3, "expected 3 round events, got {events:?}");
        // spawned (1) + 3 round + terminated (1) = 5 total
        assert_eq!(events.len(), 5);
    }

    /// Test 3: skill that drives > DEFAULT_MAX_ROUNDS → cap fires,
    /// spawn returns Err containing "max_rounds".
    #[tokio::test]
    async fn max_rounds_cap_returns_err() {
        let call_log = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink: Arc<TestSink> = Arc::new(TestSink::default());
        let sink_dyn: Arc<dyn TelemetrySink> = sink.clone();
        let runtime = make_runtime(
            Arc::new(EchoDispatcher {
                call_log: call_log.clone(),
            }),
            sink_dyn,
        );
        let join = runtime.spawn_long_running(
            Box::new(AlwaysContinue {
                id: "t3".into(),
                call_count: 0,
            }),
            req("initial"),
        );
        let err = tokio::time::timeout(Duration::from_secs(5), join)
            .await
            .expect("join timed out — max_rounds not enforced")
            .expect("join ok")
            .expect_err("expected cap error");
        assert!(
            err.message.contains("max_rounds"),
            "expected 'max_rounds' in error, got: {}",
            err.message
        );

        // The cap fires at DEFAULT_MAX_ROUNDS rounds. The runtime
        // emits a LongRunningRoundCompleted event AFTER each
        // dispatch but BEFORE incrementing the round counter, so
        // the events have round indices 0..(DEFAULT_MAX_ROUNDS-1).
        let events = sink.events.lock().unwrap().clone();
        let rounds = events.iter().filter(|(k, _, _)| k == "round").count();
        assert_eq!(
            rounds as u32, DEFAULT_MAX_ROUNDS,
            "expected exactly {DEFAULT_MAX_ROUNDS} round events before cap fires, got {rounds}"
        );
    }

    /// Test 4: skill's tick returns Err → spawn returns Err with
    /// the skill's message; no further dispatch calls happen.
    #[tokio::test]
    async fn skill_error_propagates_through_spawn() {
        let call_log = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink: Arc<TestSink> = Arc::new(TestSink::default());
        let sink_dyn: Arc<dyn TelemetrySink> = sink.clone();
        let runtime = make_runtime(
            Arc::new(EchoDispatcher {
                call_log: call_log.clone(),
            }),
            sink_dyn,
        );
        let join = runtime.spawn_long_running(Box::new(ErringSkill { id: "t4".into() }), req("initial"));
        let err = join.await.expect("join ok").expect_err("expected skill error");
        assert_eq!(err.message, "skill reports error");

        let log = call_log.lock().unwrap().clone();
        assert_eq!(log, vec!["e1".to_string()], "exactly 1 dispatch, got {log:?}");

        let events = sink.events.lock().unwrap().clone();
        let last = events.last().expect("at least one event");
        assert_eq!(last.0, "terminated");
        assert_eq!(
            last.2, "skill reports error",
            "termination reason must be the skill error"
        );
    }
}
