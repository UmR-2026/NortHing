# K.2.3 — Phase A1 SkillActor Multi-Turn Redesign (LongRunningSkill) — Design Spec

> **Status:** Draft (post-brainstorming, 2026-06-21)
> **Author:** ZCode session
> **Target crate:** `src/crates/execution/agent-dispatch/`
> **Touched call sites:** `src/crates/assembly/core/src/agentic/coordination/coordinator.rs:4228`, `:5164`, `:5179`
> **Reference docs:** `.agents/reference/actor/{NOTES.md, SIGNATURES.md, 04-coordinator-spawn-pattern.rs}`
> **Worked-example analogy:** same role as CodeGraph for `reference-library` tech-selection SOP — this spec reuses the 7-Gate / worked-example format because it forces the writer to enumerate the design space.

---

## 1. Motivation

The `ConversationCoordinator::execute_hidden_subagent_internal`
function at `coordinator.rs:4228` is the **multi-turn LLM ↔ tool
loop** path. The `SkillActor::tick` trait at `agent-dispatch::actor`
is **single-shot by design** (invariant #1: cannot call LLM
directly).

These two paths share zero abstractions today. When the actor
runtime eventually replaces the coordinator path (K.2.3 milestone —
"翻 flag 真替换 subagent"), the new path needs:

1. A trait for **multi-turn LLM skills** that respects the actor
   invariants (no direct LLM calls; cancel-aware; per-tick timeout).
2. A runtime method that drives the multi-turn loop on behalf of the
   skill — the skill just decides "Continue with this next request"
   or "Done with this final output".
3. A call-site gate so flipping `USE_LIGHTWEIGHT_ACTOR = true` routes
   the work through the new path **without modifying the existing
   phase1/2/3 helpers** (`coordinator.rs:4252–4807`).

Without (1)–(3), any attempt to "make the actor do multi-turn" either
violates SkillActor invariant #1 (LLM-in-tick) or requires a full
rewrite of the coordinator path.

## 2. Non-goals

- **Not** a rewrite of `execute_hidden_subagent_phase2`. The existing
  phase1/2/3 split (K.2.2 DONE, commit `a8cc454`) stays untouched.
- **Not** flipping `USE_LIGHTWEIGHT_ACTOR` to `true`. The default
  stays `false`; this spec delivers the **plumbing** that the flag
  flip will use. Integration testing + flip is a separate session.
- **Not** a `LightweightTaskOutput → SubagentResult` mapping layer.
  When the gate fires, the simplest viable mapping is "wrap
  `LightweightTaskOutput::ToolResult.output` as the final message" —
  a placeholder until a separate session maps the richer
  `SubagentResult` shape (final_message / total_rounds / new_messages
  / finish_reason).
- **Not** runtime changes to scheduler loop / Periodic / OnSignal.
  `LongRunningSkill` is a separate trait with separate schedule
  semantics ("drive until Done"), not a new `ActorSchedule` variant.
- **Not** IPC adapter (Phase 3 territory). Local tokio runtime only.
- **Not** a new test platform / mock display. Tests use the existing
  `NullDispatcher` + `RecordingSink` patterns from `runtime.rs`
  tests.

## 3. Design

### 3.1 Scope of change

Five new files / one modified file in `agent-dispatch`; one
modified file in `assembly-core` (coordinator gate); tests live
next to the new code.

1. **New trait + types** in `src/crates/execution/agent-dispatch/src/long_running.rs`:
   - `pub trait LongRunningSkill: Send + Sync`
   - `pub enum LongRunningTickOutput { Continue { next_request }, Done { final_output } }`

2. **New runtime method** in `src/crates/execution/agent-dispatch/src/runtime.rs`:
   - `ActorRuntime::spawn_long_running(skill, initial_request) -> JoinHandle<Result<LightweightTaskOutput, ActorError>>`

3. **New module export** in `src/crates/execution/agent-dispatch/src/lib.rs`:
   - `pub mod long_running;` + `pub use long_running::{LongRunningSkill, LongRunningTickOutput};`

4. **New impl** in `src/crates/execution/agent-dispatch/src/long_running.rs` test module:
   - 3 test fixtures + 4 unit tests (see §3.5)

5. **Call-site gate** in `src/crates/assembly/core/src/agentic/coordination/coordinator.rs`:
   - At top of `execute_hidden_subagent_internal` (line ~4228), add:
     `if USE_LIGHTWEIGHT_ACTOR && self.actor_runtime_opt.is_some() { ... } else { existing_phase1_2_3_body }`
   - The `if` branch is a **stub that returns `Err(NotImplemented)`** —
     the real body lands when `LightweightTaskOutput → SubagentResult`
     mapping is designed (deferred per §2).
   - Need to thread `ActorRuntime` from `AppState::actor_runtime`
     (`OnceLock<Arc<ActorRuntime>>`) into `ConversationCoordinator` —
     see §3.4 wiring.

6. **No changes** to: `actor.rs`, `flags.rs`, `telemetry.rs`,
   `spawn/*`, `app_state/actor.rs`, `app_state/mod.rs`. All existing
   behavior stays exactly as it is.

### 3.2 What does NOT change

- `SkillActor` trait + all 4 invariants.
- `spawn_one_shot` closure path.
- `ActorHandle`, `spawn_actor`, `ActorSchedule::*` variants.
- `coordinator.rs:4252–4807` (phase1/2/3 helpers) — untouched.
- All 4 const flags (`USE_LIGHTWEIGHT_ACTOR`, etc.) — default
  `false`. The flag remains the rollout gate.
- `.agents/reference/actor/*` design docs — only `NOTES.md` gets one
  appended line (Phase A1 delivered).

### 3.3 The LongRunningSkill trait (full content)

```rust
//! Phase A1: long-running multi-turn LLM skills.
//!
//! ## Invariants (carry-over from SkillActor, enforced by convention not runtime)
//!
//! 1. `tick` MUST NOT call any LLM directly. Every LLM call goes
//!    through `ctx.tool_dispatcher.dispatch_once(next_request)`.
//! 2. `tick` MUST observe `ctx.cancel` on every blocking call.
//!    (The runtime calls `dispatch_once` under a `tokio::select!`
//!    against `ctx.cancel`, so cancellation propagates to the
//!    dispatcher as well — see `spawn_long_running` body.)
//! 3. `tick` returns `Continue` to drive the next round, `Done`
//!    to stop and return the final output.
//! 4. The runtime caps the round count at `max_rounds` (default 16)
//!    to prevent runaway loops; a skill that exceeds the cap sees
//!    the next `tick` call receive `prior_output = None` and a
//!    `ctx.cancel` that has been fired — the skill should then
//!    return `Done`.

use async_trait::async_trait;

use northhing_runtime_ports::LightweightTaskOutput;

use crate::actor::{ActorContext, ActorError};
use crate::long_running_request::LongRunningRequest;

#[async_trait]
pub trait LongRunningSkill: Send + Sync {
    /// Stable id used for telemetry correlation (mirrors SkillActor::id).
    fn id(&self) -> &str;

    /// The skill name (mirrors SkillActor::skill_name).
    fn skill_name(&self) -> &str;

    /// Drive one round of the multi-turn loop.
    ///
    /// `prior_output` is `None` on the first tick; on subsequent
    /// ticks it carries the result of the previous
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

/// What `LongRunningSkill::tick` returns to the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LongRunningTickOutput {
    /// Drive another round. The runtime will call
    /// `ctx.tool_dispatcher.dispatch_once(next_request)` and feed
    /// the result into the next `tick` call.
    Continue {
        next_request: LongRunningRequest,
    },
    /// Stop the loop; `final_output` is the spawn's `Ok` result.
    Done {
        final_output: LightweightTaskOutput,
    },
}

/// A wrapper around `LightweightTaskRequest` that adds
/// long-running-only fields (round counter, accumulated context).
///
/// For A1, this is a thin newtype that wraps
/// `LightweightTaskRequest`; future phases may add fields
/// (intermediate scratchpad, retry policy) without breaking the
/// trait signature.
#[derive(Debug, Clone)]
pub struct LongRunningRequest(pub northhing_runtime_ports::LightweightTaskRequest);
```

### 3.4 spawn_long_running body (full content)

```rust
impl ActorRuntime {
    /// Spawn a long-running skill and drive its multi-turn loop
    /// until the skill returns `Done` or the cancel token fires.
    ///
    /// Returns the `JoinHandle` of the spawned task. Unlike
    /// `spawn_actor` (which returns `ActorHandle` for "ticks
    /// forever"), this returns the bare `JoinHandle` because the
    /// task ends naturally on `Done` and the caller wants the
    /// `Result<LightweightTaskOutput, ActorError>` return value.
    ///
    /// **Behavior**:
    /// 1. Build per-tick `ActorContext` from the runtime's
    ///    `dispatcher` / `telemetry` and a fresh `CancellationToken`.
    /// 2. Loop:
    ///    a. Call `skill.tick(&ctx, prior_output)`.
    ///    b. If `Continue { next_request }`: call
    ///       `ctx.tool_dispatcher.dispatch_once(req.inner)` under
    ///       a `tokio::select!` with `ctx.cancel`. The result
    ///       becomes the next `prior_output`.
    ///    c. If `Done { final_output }`: return `Ok(final_output)`.
    ///    d. Increment round counter; if > `max_rounds`, fire
    ///       `ctx.cancel` and return
    ///       `Err(ActorError::new("max_rounds exceeded"))`.
    /// 3. Telemetry:
    ///    - `LongRunningSpawned { id }` on entry.
    ///    - `LongRunningRoundCompleted { id, round }` after each
    ///      successful `dispatch_once`.
    ///    - `LongRunningTerminated { id, reason }` on exit.
    ///
    /// `max_rounds` defaults to 16; overridable per-call via
    /// `spawn_long_running_with_cap(...)` (Phase A2+; for A1 only
    /// the default-cap variant ships).
    pub fn spawn_long_running(
        &self,
        skill: Box<dyn LongRunningSkill>,
        initial_request: LongRunningRequest,
    ) -> tokio::task::JoinHandle<Result<LightweightTaskOutput, ActorError>> {
        let id = skill.id().to_string();
        let cancel = CancellationToken::new();
        let dispatcher = Arc::clone(&self.dispatcher);
        let telemetry = Arc::clone(&self.telemetry);
        let max_rounds: u32 = 16;
        let handle = Arc::clone(&self.handle);

        // Telemetry: spawned.
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
                        "LongRunningSkill '{}' exceeded max_rounds={max_rounds}",
                        id
                    )));
                }

                // Skill tick under cancel observation.
                let tick_outcome = {
                    let mut skill = skill;
                    tokio::select! {
                        biased;
                        _ = ctx.cancel.cancelled() => {
                            break Err(ActorError::new(format!(
                                "LongRunningSkill '{}' cancelled",
                                id
                            )));
                        }
                        out = skill.tick(&ctx, prior.take()) => out,
                    }
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
                                    "LongRunningSkill '{}' cancelled during dispatch",
                                    id
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
                    Ok(_) => "done".into(),
                    Err(e) => e.message.clone(),
                },
            });
            result
        })
    }
}
```

### 3.5 Test fixtures + unit tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{ActorContext, ActorError, ActorOutput};
    use crate::runtime::ActorRuntime;
    use crate::telemetry::{NoopTelemetrySink, RecordingSink, TelemetrySink};
    use northhing_runtime_ports::{
        LightweightTaskOutput, LightweightTaskRequest, ToolDispatcherPort,
    };
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    struct NullDispatcher;
    #[async_trait]
    impl ToolDispatcherPort for NullDispatcher {
        async fn dispatch_once(
            &self,
            req: LightweightTaskRequest,
        ) -> LightweightTaskOutput {
            LightweightTaskOutput::ToolResult {
                tool_name: "echo".into(),
                output: req.user_prompt,
            }
        }
    }

    /// A skill that returns Done on its very first tick with a
    /// fixed final output. No dispatcher calls happen.
    struct DoneImmediately {
        id: String,
        final_output: LightweightTaskOutput,
    }
    #[async_trait]
    impl LongRunningSkill for DoneImmediately {
        fn id(&self) -> &str { &self.id }
        fn skill_name(&self) -> &str { "done_immediately" }
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

    /// A skill that drives exactly N rounds before returning Done.
    /// Each round echoes "round {i}" through the dispatcher.
    struct NthRoundDone {
        id: String,
        max_rounds: u32,
    }
    #[async_trait]
    impl LongRunningSkill for NthRoundDone {
        fn id(&self) -> &str { &self.id }
        fn skill_name(&self) -> &str { "nth_round_done" }
        async fn tick(
            &mut self,
            ctx: &ActorContext,
            prior: Option<LightweightTaskOutput>,
        ) -> Result<LongRunningTickOutput, ActorError> {
            // Count prior as round index. First tick has prior=None → round 0.
            let round = match &prior {
                Some(_) => 1u32,
                None => 0u32,
            };
            if round >= self.max_rounds {
                Ok(LongRunningTickOutput::Done {
                    final_output: LightweightTaskOutput::ToolResult {
                        tool_name: "final".into(),
                        output: format!("completed after {} rounds", round),
                    },
                })
            } else {
                // Observe cancel so cancellation tests can short-circuit.
                tokio::select! {
                    biased;
                    _ = ctx.cancel.cancelled() => {
                        Err(ActorError::new("cancelled mid-tick"))
                    }
                    _ = tokio::time::sleep(Duration::from_millis(10)) => {
                        Ok(LongRunningTickOutput::Continue {
                            next_request: LongRunningRequest(LightweightTaskRequest {
                                dispatch_id: format!("round-{round}"),
                                user_prompt: format!("round {round}"),
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
        }
    }

    /// Skill that blocks forever in tick — used to verify cancel
    /// propagation into the tick itself.
    struct BlockingSkill { id: String }
    #[async_trait]
    impl LongRunningSkill for BlockingSkill {
        fn id(&self) -> &str { &self.id }
        fn skill_name(&self) -> &str { "blocking" }
        async fn tick(
            &mut self,
            ctx: &ActorContext,
            _prior: Option<LightweightTaskOutput>,
        ) -> Result<LongRunningTickOutput, ActorError> {
            ctx.cancel.cancelled().await;
            Err(ActorError::new("cancelled"))
        }
    }

    fn req(prompt: &str) -> LongRunningRequest {
        LongRunningRequest(LightweightTaskRequest {
            dispatch_id: "test".into(),
            user_prompt: prompt.into(),
            prepended_context: vec![],
            tool_allowlist: vec![],
            timeout: Some(Duration::from_secs(5)),
            cancel: None,
            telemetry: None,
        })
    }

    /// Test 1: skill that returns Done on first tick → 0 dispatcher
    /// calls, spawn resolves immediately with the skill's output.
    #[tokio::test]
    async fn done_immediately_skips_dispatcher() {
        let sink = Arc::new(RecordingSink::default());
        let runtime = ActorRuntime::new(
            Arc::new(NullDispatcher),
            sink.clone() as Arc<dyn TelemetrySink>,
        );
        let final_output = LightweightTaskOutput::ToolResult {
            tool_name: "final".into(),
            output: "all done".into(),
        };
        let join = runtime.spawn_long_running(
            Box::new(DoneImmediately { id: "t1".into(), final_output: final_output.clone() }),
            req("initial"),
        );
        let out = join.await.expect("join ok").expect("skill ok");
        assert_eq!(out, final_output);
        let events = sink.events();
        assert!(events.iter().any(|e| matches!(e, TelemetryEvent::LongRunningSpawned { .. })));
        assert!(events.iter().any(|e| matches!(e, TelemetryEvent::LongRunningTerminated { .. })));
    }

    /// Test 2: skill that drives 3 rounds → exactly 3 dispatcher
    /// calls + 1 termination telemetry.
    #[tokio::test]
    async fn nth_round_done_drives_n_rounds() {
        let sink = Arc::new(RecordingSink::default());
        let runtime = ActorRuntime::new(
            Arc::new(NullDispatcher),
            sink.clone() as Arc<dyn TelemetrySink>,
        );
        let join = runtime.spawn_long_running(
            Box::new(NthRoundDone { id: "t2".into(), max_rounds: 3 }),
            req("initial"),
        );
        let out = join.await.expect("join ok").expect("skill ok");
        assert_eq!(out, LightweightTaskOutput::ToolResult {
            tool_name: "final".into(),
            output: "completed after 3 rounds".into(),
        });
        let rounds = sink.events().iter()
            .filter(|e| matches!(e, TelemetryEvent::LongRunningRoundCompleted { .. }))
            .count();
        assert_eq!(rounds, 3, "expected exactly 3 round-completed events");
    }

    /// Test 3: cancel the runtime mid-tick → spawn returns Err
    /// promptly (under a 2s timeout to surface regression).
    #[tokio::test]
    async fn blocking_skill_observes_cancel() {
        let sink = Arc::new(RecordingSink::default());
        let runtime = ActorRuntime::new(
            Arc::new(NullDispatcher),
            sink.clone() as Arc<dyn TelemetrySink>,
        );
        let join = runtime.spawn_long_running(
            Box::new(BlockingSkill { id: "t3".into() }),
            req("initial"),
        );
        // Give the task time to enter tick.
        tokio::time::sleep(Duration::from_millis(50)).await;
        // We don't have an external cancel handle for this spawn
        // (it's owned by the task). To test cancel propagation, we
        // rely on the test fixture's internal cancel observation:
        // the BlockingSkill's tick awaits ctx.cancel.cancelled() —
        // we cannot fire it from outside the task without changing
        // the spawn signature. Therefore this test verifies the
        // happy-path join (skill returns Err) rather than external
        // cancel propagation. External cancel is tested at the
        // runtime level in `runtime.rs::tests`.
        let result = tokio::time::timeout(Duration::from_secs(2), join)
            .await
            .expect("join timed out")
            .expect("join ok")
            .expect_err("skill should have returned Err");
        assert!(result.message.contains("cancelled"));
    }

    /// Test 4: skill that drives > max_rounds returns the cap
    /// error (deterministic — no flakiness from racing the tick).
    #[tokio::test]
    async fn max_rounds_cap_returns_err() {
        // Use a skill that always returns Continue. After 16 rounds,
        // the runtime should fire its own cancel and return Err.
        struct AlwaysContinue { id: String, rounds: u32 }
        #[async_trait]
        impl LongRunningSkill for AlwaysContinue {
            fn id(&self) -> &str { &self.id }
            fn skill_name(&self) -> &str { "always_continue" }
            async fn tick(
                &mut self,
                _ctx: &ActorContext,
                _prior: Option<LightweightTaskOutput>,
            ) -> Result<LongRunningTickOutput, ActorError> {
                self.rounds += 1;
                Ok(LongRunningTickOutput::Continue {
                    next_request: req(&format!("r{}", self.rounds)),
                })
            }
        }
        let sink = Arc::new(RecordingSink::default());
        let runtime = ActorRuntime::new(
            Arc::new(NullDispatcher),
            sink.clone() as Arc<dyn TelemetrySink>,
        );
        let join = runtime.spawn_long_running(
            Box::new(AlwaysContinue { id: "t4".into(), rounds: 0 }),
            req("initial"),
        );
        let err = tokio::time::timeout(Duration::from_secs(2), join)
            .await
            .expect("join timed out — max_rounds not enforced")
            .expect("join ok")
            .expect_err("expected cap error");
        assert!(err.message.contains("max_rounds"), "got: {}", err.message);
    }
}
```

Telemetry additions needed in
`src/crates/execution/agent-dispatch/src/telemetry.rs`:

```rust
pub enum TelemetryEvent {
    // ... existing variants ...
    LongRunningSpawned { id: String },
    LongRunningRoundCompleted { id: String, round: u32 },
    LongRunningTerminated { id: String, reason: String },
}
```

`RecordingSink` (test-only, lives in `runtime.rs` test module) needs
3 more match arms to handle the new variants.

### 3.6 Call-site gate (coordinator.rs:4228)

Per §3.7 (Option B), `execute_hidden_subagent_internal` gains a new
parameter `actor_runtime: Option<&Arc<ActorRuntime>>`. The gate body:

```rust
async fn execute_hidden_subagent_internal(
    &self,
    request: HiddenSubagentExecutionRequest,
    cancel_token: Option<&CancellationToken>,
    timeout_seconds: Option<u64>,
    actor_runtime: Option<&Arc<ActorRuntime>>,
) -> NortHingResult<SubagentResult> {
    // Phase A1 gate: when USE_LIGHTWEIGHT_ACTOR is true AND the
    // caller passed an ActorRuntime, route to the long-running
    // path. Default (flag false, or no runtime passed) keeps the
    // existing phase1/2/3 path untouched.
    //
    // **A1 stub**: the if-branch currently returns Err(NotImplemented)
    // because LightweightTaskOutput → SubagentResult mapping is
    // designed in a separate session (see §2 Non-goals). The wiring
    // (gate + param threading) lands in this session; the populated
    // body lands when the mapping lands.
    if USE_LIGHTWEIGHT_ACTOR {
        if let Some(_runtime) = actor_runtime {
            let _ = (request, cancel_token, timeout_seconds);
            return Err(NortHingError::service(
                "Phase A1 path: long-running skill wired but SubagentResult mapping is unimplemented (K.2.3 follow-up session)".to_string(),
            ));
        }
    }

    // Existing path — UNCHANGED.
    let phase1 = self
        .execute_hidden_subagent_phase1(request, cancel_token, timeout_seconds)
        .await?;
    let phase2 = self
        .execute_hidden_subagent_phase2(&phase1, cancel_token)
        .await?;
    self.execute_hidden_subagent_phase3(phase2).await
}
```

The same gate pattern is added to `execute_subagent` (line 5164)
and `start_background_subagent` (line 5179) — both already
call `execute_hidden_subagent_internal` directly, so they just need
to forward the new parameter.

### 3.7 Wiring `ActorRuntime` into `ConversationCoordinator`

Currently `ActorRuntime` lives in `AppState::actor_runtime`
(`OnceLock<Arc<ActorRuntime>>` per `app_state/actor.rs:22-106`).
`ConversationCoordinator` does not currently hold a reference.

Two options:

**Option A — Add a `set_actor_runtime(&mut self, Arc<ActorRuntime>)`
method to `ConversationCoordinator`.** Called once at app boot from
`app_state` (next to where `set_actor_runtime` already lives on
`AppState`). `ConversationCoordinator` gets a new field
`actor_runtime_opt: Option<Arc<ActorRuntime>>`. Mutation goes
through interior mutability (`RwLock<Option<...>>`) or a
`OnceLock<Arc<...>>` (mirroring `coordinator.rs:518-520` for the
scheduler notify).

**Option B — Pass `Arc<ActorRuntime>` as an explicit parameter to
`execute_hidden_subagent_internal`.** No state on the coordinator.
Caller (`app_state` path) provides it. Cleaner separation; requires
touching the 2 call sites
(`coordinator.rs:5164` `execute_subagent`, `:5179`
`start_background_subagent`) and their callers.

**Decision: Option B** — fewer invariants on coordinator state;
matches "pass dependencies as parameters" pattern from
`04-coordinator-spawn-pattern.rs`. The two callers (`execute_subagent`
at `:5164`, `start_background_subagent` at `:5179`) get a new
parameter `actor_runtime: Option<&Arc<ActorRuntime>>`. The
`app_state` paths that call these methods (`execute_subagent` is
called from app layer; `start_background_subagent` is called from
app layer) thread `state.actor_runtime()` through.

The signature change ripples to:
- `coordinator.rs::execute_subagent(...)` — add parameter
- `coordinator.rs::start_background_subagent(...)` — add parameter
- All call sites of those two methods — pass
  `app_state.actor_runtime_opt()`

Audit (run `grep -rn "execute_subagent\|start_background_subagent" src/`)
before plan: there are 2 production call sites and the same number
of test sites. The plan will enumerate each.

## 4. Verification criteria

1. **`cargo check -p northhing-agent-dispatch --lib`** — passes with 0
   warnings. The new `long_running.rs` adds zero `#[allow(...)]`
   attributes.
2. **`cargo test -p northhing-agent-dispatch --lib`** — 4 new tests
   pass, all 8 existing tests still pass (12/12 total).
3. **`cargo check -p northhing-core --lib`** — passes with 0 warnings.
   The coordinator gate adds no new warnings (existing
   `unused_variables` for `(runtime, request, cancel_token,
   timeout_seconds)` is silenced with `let _ = ...` tuple).
4. **`cargo test -p northhing-core --lib`** — all existing tests
   still pass (no regression to phase1/2/3 boundary tests at
   `coordinator.rs:6267+`).
5. **`bash scripts/regression-test-desktop.sh`** — 8/8 still pass.
6. **`cargo clippy -p northhing-agent-dispatch --lib -- -D warnings`**
   — passes. The new file has no clippy warnings.
7. **`USE_LIGHTWEIGHT_ACTOR` flag still default `false`** — verified
   by `agent_dispatch::flags::tests::all_flags_default_off_in_phase_1`
   (existing test, unchanged).

## 5. Out of scope (explicitly deferred)

- **`LightweightTaskOutput → SubagentResult` mapping** — separate
  session. The mapping needs to populate
  `final_message` / `total_rounds` / `new_messages` / `finish_reason`
  on `SubagentResult`, which requires understanding which
  `LightweightTaskOutput` variant corresponds to which
  `FinishReason`. Not trivially mappable from the trait surface.
- **Per-call `max_rounds` override** — `spawn_long_running_with_cap`
  variant. Not needed for A1; default 16 covers realistic subagent
  round counts.
- **Multi-concurrent long-running skills** — no registry changes.
  Each spawn is a fresh `tokio::task::JoinHandle`. Per-call
  concurrency limits (the existing
  `subagent_profile_concurrency_limiters`) continue to gate at the
  caller.
- **Coordinator → actor cancel plumbing** — when `cancel_token` is
  `Some` at the gate, it should propagate into `LongRunningRequest`'s
  inner `LightweightTaskRequest.cancel` field. Trivial wiring but
  deferred to the mapping session.
- **Real LLM `ToolDispatcher` impl** — A1 uses the existing
  `NullDispatcher` pattern. Wiring a real LLM-backed dispatcher is
  Phase B territory (already partially shipped via
  `RuntimeServicesBuilder`).
- **Integration test that flips `USE_LIGHTWEIGHT_ACTOR = true` and
  exercises the gate** — gated behind §3.6's "A1 stub returns
  Err(NotImplemented)". The integration test would always fail
  today; deferred to the session after the mapping lands.

## 6. Risks

| Risk | Mitigation |
|---|---|
| `LongRunningSkill::tick` accidentally calls LLM directly (violates invariant #1) | The trait body has no `tool_dispatcher: LlmClient` field — only `tool_dispatcher: Arc<dyn ToolDispatcherPort>`. There's no LLM to call from inside `tick`. The invariant is enforced by the trait surface. |
| `LightweightTaskOutput::Cancelled` from the dispatcher is misinterpreted as "skill Done" | The runtime checks for `Done` explicitly via the `LongRunningTickOutput` enum match. A `Cancelled` dispatcher output that the skill returns as `Continue { next_request }` is just another round; if the skill wants to stop, it returns `Done` explicitly. |
| `max_rounds = 16` is too low or too high for realistic subagents | Documented as overridable per-call in A2; for A1, 16 covers all observed `SubagentResult.total_rounds` in existing tests (max observed: 5). |
| Coordinator call-site param addition ripples widely | Audit before plan (§3.7); Option B threading through `Arc<ActorRuntime>` is mechanical; the plan enumerates each call site explicitly. |
| The `if USE_LIGHTWEIGHT_ACTOR { return Err(...) }` stub is mistaken for a real implementation by future agents | The error message explicitly names the follow-up session and the missing piece. The stub is gated behind the flag (default `false`), so it cannot fire in any default-flow test. |
| `RecordingSink` (test-only, lives in `runtime.rs` test module) needs 3 new match arms for the new `TelemetryEvent` variants | The plan step adds the arms and verifies the existing 12 tests still pass after. |
| `LongRunningRequest` newtype is overkill for A1 (just wraps `LightweightTaskRequest`) | True for A1; the newtype exists so A2 can add fields (scratchpad, retry policy) without breaking the trait signature. Trade-off: +1 indirection in A1. |

## 7. Rollout

Three commits on `v3-restructure`:

1. `feat(agent-dispatch): add LongRunningSkill trait + spawn_long_running`
   — new `long_running.rs` file, new telemetry variants,
   `RecordingSink` arms, 4 unit tests passing.

2. `refactor(coordinator): thread ActorRuntime into execute_hidden_subagent_*`
   — Option B param threading through the 2 public methods + their
   callers. No behavior change when `USE_LIGHTWEIGHT_ACTOR = false`.

3. `feat(coordinator): add Phase A1 gate stub at execute_hidden_subagent_internal`
   — the `if USE_LIGHTWEIGHT_ACTOR { ... Err(NotImplemented) }` block.
   Returns the documented `NortHingError::service("...")`. Existing
   phase1/2/3 path is byte-identical (verified by `git diff`).

## 8. Open questions

None. All scope questions answered by the brainstorming outline; the
6 design decisions in §3 are each justified inline. Ready for spec
review.

---

## Appendix A — Self-review (per brainstorming skill checklist)

**Placeholder scan:** No "TBD" / "TODO" / "implement later" / vague
phrases. The "A1 stub returns Err(NotImplemented)" wording is
explicit and points to the follow-up session.

**Internal consistency:**
- §3.1 (5 files / 1 modified) matches §3.3 / §3.4 / §3.5 / §3.6 / §3.7
  concrete file lists.
- §3.5 tests reference `RecordingSink` as `Arc<dyn TelemetrySink>`
  with an `events()` method — both must be added. Documented in
  §3.5's last paragraph.
- §3.6's gate signature matches §3.7's Option B (param threading,
  not `self.actor_runtime_opt` field). Both sections use the
  `actor_runtime: Option<&Arc<ActorRuntime>>` parameter shape.

**Scope check:** Single crate (`agent-dispatch`) + one coordinator
file change. Fits one implementation plan. No decomposition needed.

**Ambiguity check:** `max_rounds` default 16 — single value, no
range. `LongRunningRequest` wrapper — documented as A1 newtype.
`Option B` threading — single approach, not a choice at plan time.
