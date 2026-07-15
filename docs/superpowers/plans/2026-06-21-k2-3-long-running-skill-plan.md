<!-- LEGACY: 本文档是 v0.1.0 之前的历史计划，保留原 `agent-app` 名称作历史参考。
     Northing / 纳森 是 agent-app 的继任者（v0.1.0 之后改名）。
     本文件内容不被后续产品名替换脚本覆盖，保留 plan 当时的命名语境。 -->

# K.2.3 — LongRunningSkill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `LongRunningSkill` trait + `ActorRuntime::spawn_long_running` + 4 unit tests, then thread `Arc<ActorRuntime>` as a new parameter through the 3 coordinator call sites, then add the A1 stub gate at `execute_hidden_subagent_internal`. All 4 const flags stay default `false` — no behavior change at the existing path.

**Architecture:** Phase A1 of the actor multi-turn redesign per spec §3. Three commits matching spec §7:
1. New trait + runtime method + 4 unit tests in `agent-dispatch` crate (no coordinator touched)
2. Option B param threading through coordinator's 3 subagent methods (zero behavior change at `USE_LIGHTWEIGHT_ACTOR=false`)
3. A1 stub gate at `execute_hidden_subagent_internal` (returns `Err(NotImplemented)` for `USE_LIGHTWEIGHT_ACTOR=true`)

**Tech Stack:** Rust 2021, tokio, async-trait, dashmap. No new dependencies.

**Source spec:** `docs/superpowers/specs/2026-06-21-k2-3-long-running-skill-design.md`

**Branch:** `v3-restructure`. HEAD at start: `e32dd1b`.

---

## File Structure

| Path | Action | Responsibility |
|---|---|---|
| `src/crates/execution/agent-dispatch/src/long_running.rs` | Create | `LongRunningSkill` trait + `LongRunningTickOutput` + `LongRunningRequest` newtype + 4 test fixtures + 4 unit tests |
| `src/crates/execution/agent-dispatch/src/telemetry.rs` | Modify | Add 3 new `TelemetryEvent` variants + extend `Display` impl |
| `src/crates/execution/agent-dispatch/src/runtime.rs` | Modify | Add `spawn_long_running` method + extend `RecordingSink` match with 3 arms |
| `src/crates/execution/agent-dispatch/src/lib.rs` | Modify | Add `pub mod long_running;` + 3 new re-exports |
| `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` | Modify | Add `actor_runtime` param to 3 methods; add A1 gate stub at `execute_hidden_subagent_internal` |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs` | Modify | Thread `actor_runtime: Option<&Arc<ActorRuntime>>` through 2 call sites (`task_tool.rs:1201` `start_background_subagent`, `:1251` `execute_subagent`) |

The trait lives in its own file (`long_running.rs`) because it has a different shape from `SkillActor` (different return type, different loop semantics) and putting it in `actor.rs` would couple the two traits' invariants. The runtime method stays in `runtime.rs` because that's where `ActorRuntime` lives.

---

## Task 1: New trait + runtime method + 4 unit tests

**Files:**
- Modify: `src/crates/execution/agent-dispatch/src/telemetry.rs` (3 new variants + Display)
- Modify: `src/crates/execution/agent-dispatch/src/runtime.rs` (`RecordingSink` match + `spawn_long_running`)
- Create: `src/crates/execution/agent-dispatch/src/long_running.rs`
- Modify: `src/crates/execution/agent-dispatch/src/lib.rs` (module + re-exports)

- [ ] **Step 1.1: Verify clean tree and baseline test pass**

```bash
cd e:/agent-project/agent-app
git status --short
git rev-parse HEAD
cargo test -p agent-app-agent-dispatch --lib 2>&1 | tail -5
```

Expected:
- `git status --short` → empty
- HEAD → `e32dd1b...`
- Last test line shows `test result: ok. 8 passed; 0 failed`

If any test fails or tree dirty, stop.

- [ ] **Step 1.2: Add 3 new `TelemetryEvent` variants to `telemetry.rs`**

Read current `telemetry.rs`, then Edit the `TelemetryEvent` enum. The new variants go at the end of the existing list (after `DispatchAborted`).

Edit `src/crates/execution/agent-dispatch/src/telemetry.rs`:

old_string (lines 18-35):

```rust
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
}
```

new_string:

```rust
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
```

- [ ] **Step 1.3: Extend the `Display` impl for `TelemetryEvent`**

Same file, edit the `impl fmt::Display for TelemetryEvent` block (currently lines 47-69).

old_string:

```rust
            TelemetryEvent::DispatchAborted { dispatch_id, reason } => {
                write!(f, "dispatch_aborted id={dispatch_id} reason={reason}")
            }
        }
    }
}
```

new_string:

```rust
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
```

Verify it compiles (the existing test `event_display_is_stable` at telemetry.rs:121 would fail to compile if any new arm were missing in Display — but that test doesn't check the new variants, so just `cargo check` suffices).

Run:

```bash
cd e:/agent-project/agent-app
cargo check -p agent-app-agent-dispatch --lib 2>&1 | tail -5
```

Expected: `Finished ...` with 0 errors. Warnings OK for now (the `RecordingSink` match in runtime.rs will need updating; that's Step 1.4).

- [ ] **Step 1.4: Add 3 new arms to `RecordingSink` match in `runtime.rs`**

Edit `src/crates/execution/agent-dispatch/src/runtime.rs` line 484-491:

old_string:

```rust
            let kind = match event {
                crate::telemetry::TelemetryEvent::ActorSpawned { .. } => "spawned",
                crate::telemetry::TelemetryEvent::ActorTicked { .. } => "ticked",
                crate::telemetry::TelemetryEvent::ActorError { .. } => "error",
                crate::telemetry::TelemetryEvent::ActorEvent { .. } => "event",
                crate::telemetry::TelemetryEvent::ActorTerminatedAfterCancel { .. } => "terminated",
                crate::telemetry::TelemetryEvent::DispatchCompleted { .. } => "dispatch_completed",
                crate::telemetry::TelemetryEvent::DispatchAborted { .. } => "dispatch_aborted",
            };
```

new_string:

```rust
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
```

Run the existing tests to confirm no regression:

```bash
cd e:/agent-project/agent-app
cargo test -p agent-app-agent-dispatch --lib 2>&1 | tail -5
```

Expected: `test result: ok. 8 passed; 0 failed` (same 8 as baseline).

- [ ] **Step 1.5: Create `long_running.rs` with the trait + types + 4 test fixtures + 4 unit tests**

Write the full content below to `src/crates/execution/agent-dispatch/src/long_running.rs`:

```rust
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

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use agent_app_runtime_ports::{
    LightweightTaskOutput, LightweightTaskRequest, ToolDispatcherPort,
};

use crate::actor::{ActorContext, ActorError};
use crate::runtime::ActorRuntime;
use crate::telemetry::{NoopTelemetrySink, TelemetryEvent, TelemetrySink};

/// Maximum rounds a `LongRunningSkill` is allowed to drive before
/// the runtime forces exit with `ActorError`. Exposed as a
/// `pub const` (not `pub static`) so it's a true constant — callers
/// who need per-skill caps (Phase A2+) can wrap their own runtime.
pub const DEFAULT_MAX_ROUNDS: u32 = 16;

/// A wrapper around `LightweightTaskRequest` that exists so future
/// phases can add long-running-only fields (intermediate scratchpad,
/// retry policy, round counter) without breaking the trait
/// signature. A1 carries no extra fields.
#[derive(Debug, Clone)]
pub struct LongRunningRequest(pub LightweightTaskRequest);

/// What `LongRunningSkill::tick` returns to the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl ActorRuntime {
    /// Spawn a long-running skill and drive its multi-turn loop
    /// until the skill returns `Done` or the cancel token fires.
    ///
    /// Unlike `spawn_actor` (which returns `ActorHandle` for "ticks
    /// forever"), this returns the bare `JoinHandle` because the
    /// task ends naturally on `Done` and the caller wants the
    /// `Result<LightweightTaskOutput, ActorError>` return value.
    ///
    /// See module-level docs for invariants + telemetry.
    pub fn spawn_long_running(
        &self,
        mut skill: Box<dyn LongRunningSkill>,
        initial_request: LongRunningRequest,
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

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{ActorContext, ActorError};
    use std::sync::Mutex;

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

    /// A `TelemetrySink` that records events as `(kind, id, round_or_reason)`.
    /// Used by tests to assert on telemetry sequencing.
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
                TelemetryEvent::LongRunningSpawned { id } => {
                    ("spawned".into(), id, String::new())
                }
                TelemetryEvent::LongRunningRoundCompleted { id, round } => {
                    ("round".into(), id, round.to_string())
                }
                TelemetryEvent::LongRunningTerminated { id, reason } => {
                    ("terminated".into(), id, reason)
                }
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

    /// A skill that drives exactly `target_rounds` rounds before
    /// returning `Done`. Each round's dispatch_id is `"round-{n}"`.
    struct NthRoundDone {
        id: String,
        target_rounds: u32,
    }
    #[async_trait]
    impl LongRunningSkill for NthRoundDone {
        fn id(&self) -> &str { &self.id }
        fn skill_name(&self) -> &str { "nth_round_done" }
        async fn tick(
            &mut self,
            _ctx: &ActorContext,
            prior: Option<LightweightTaskOutput>,
        ) -> Result<LongRunningTickOutput, ActorError> {
            // prior == Some(_): just completed a dispatch → count it.
            // prior == None: first tick.
            let completed = match &prior { Some(_) => 1u32, None => 0u32 };
            if completed >= self.target_rounds {
                Ok(LongRunningTickOutput::Done {
                    final_output: LightweightTaskOutput::ToolResult {
                        tool_name: "final".into(),
                        output: format!("completed after {completed} rounds"),
                    },
                })
            } else {
                Ok(LongRunningTickOutput::Continue {
                    next_request: LongRunningRequest(LightweightTaskRequest {
                        dispatch_id: format!("round-{completed}"),
                        user_prompt: format!("round {completed}"),
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
    struct AlwaysContinue { id: String, call_count: u32 }
    #[async_trait]
    impl LongRunningSkill for AlwaysContinue {
        fn id(&self) -> &str { &self.id }
        fn skill_name(&self) -> &str { "always_continue" }
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
        let sink = Arc::new(TestSink::default());
        let runtime = make_runtime(Arc::new(NoopDispatcher), sink.clone());
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

        let events = sink.events.lock().unwrap().clone();
        assert_eq!(events.len(), 2, "expected spawned + terminated, got {events:?}");
        assert_eq!(events[0].0, "spawned");
        assert_eq!(events[1].0, "terminated");
        assert_eq!(events[1].2, "done");
    }

    /// Minimal no-op dispatcher for tests that don't care about dispatch calls.
    struct NoopDispatcher;
    #[async_trait::async_trait]
    impl ToolDispatcherPort for NoopDispatcher {
        async fn dispatch_once(&self, _req: LightweightTaskRequest) -> LightweightTaskOutput {
            LightweightTaskOutput::NoToolMatched { reason: "noop".into() }
        }
    }

    /// Test 2: skill that drives 3 rounds → exactly 3 dispatcher
    /// calls + 3 round-completed events + 2 boundary events (spawned + terminated).
    #[tokio::test]
    async fn nth_round_done_drives_n_rounds() {
        let call_log = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = Arc::new(TestSink::default());
        let runtime = make_runtime(
            Arc::new(EchoDispatcher { call_log: call_log.clone() }),
            sink.clone(),
        );
        let join = runtime.spawn_long_running(
            Box::new(NthRoundDone { id: "t2".into(), target_rounds: 3 }),
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
        let sink = Arc::new(TestSink::default());
        let runtime = make_runtime(
            Arc::new(EchoDispatcher { call_log: call_log.clone() }),
            sink.clone(),
        );
        let join = runtime.spawn_long_running(
            Box::new(AlwaysContinue { id: "t3".into(), call_count: 0 }),
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
    struct ErringSkill { id: String, dispatched: bool }
    #[async_trait]
    impl LongRunningSkill for ErringSkill {
        fn id(&self) -> &str { &self.id }
        fn skill_name(&self) -> &str { "erring" }
        async fn tick(
            &mut self,
            _ctx: &ActorContext,
            prior: Option<LightweightTaskOutput>,
        ) -> Result<LongRunningTickOutput, ActorError> {
            // First tick: succeed with Continue (dispatch happens).
            // Second tick: return Err.
            if prior.is_none() {
                self.dispatched = false; // first entry
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
                self.dispatched = true;
                Err(ActorError::new("skill reports error"))
            }
        }
    }

    #[tokio::test]
    async fn skill_error_propagates_through_spawn() {
        let call_log = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = Arc::new(TestSink::default());
        let runtime = make_runtime(
            Arc::new(EchoDispatcher { call_log: call_log.clone() }),
            sink.clone(),
        );
        let join = runtime.spawn_long_running(
            Box::new(ErringSkill { id: "t4".into(), dispatched: false }),
            req("initial"),
        );
        let err = join.await.expect("join ok").expect_err("expected skill error");
        assert_eq!(err.message, "skill reports error");

        let log = call_log.lock().unwrap().clone();
        assert_eq!(log, vec!["e1".to_string()], "exactly 1 dispatch, got {log:?}");

        let events = sink.events.lock().unwrap().clone();
        let last = events.last().expect("at least one event");
        assert_eq!(last.0, "terminated");
        assert_eq!(last.2, "skill reports error", "termination reason must be the skill error");
    }
}
```

Run tests:

```bash
cd e:/agent-project/agent-app
cargo test -p agent-app-agent-dispatch --lib long_running 2>&1 | tail -15
```

Expected: `test result: ok. 4 passed; 0 failed` (the 4 new tests in `long_running::tests`).

- [ ] **Step 1.6: Wire the new module into `lib.rs`**

Edit `src/crates/execution/agent-dispatch/src/lib.rs`:

old_string:

```rust
pub mod actor;
pub mod flags;
pub mod runtime;
pub mod spawn;
pub mod telemetry;

pub use actor::{ActorContext, ActorError, ActorOutput, ActorSchedule, ActorTrigger, SkillActor};
pub use flags::{
    USE_ACTOR_IPC, USE_DISPATCHER_IPC, USE_LIGHTWEIGHT_ACTOR, USE_ONESHOT_DISPATCHER,
};
pub use runtime::{ActorHandle, ActorRuntime};
pub use telemetry::{NoopTelemetrySink, TelemetryEvent, TelemetrySink};
```

new_string:

```rust
pub mod actor;
pub mod flags;
pub mod long_running;
pub mod runtime;
pub mod spawn;
pub mod telemetry;

pub use actor::{ActorContext, ActorError, ActorOutput, ActorSchedule, ActorTrigger, SkillActor};
pub use flags::{
    USE_ACTOR_IPC, USE_DISPATCHER_IPC, USE_LIGHTWEIGHT_ACTOR, USE_ONESHOT_DISPATCHER,
};
pub use long_running::{LongRunningRequest, LongRunningSkill, LongRunningTickOutput};
pub use runtime::{ActorHandle, ActorRuntime};
pub use telemetry::{NoopTelemetrySink, TelemetryEvent, TelemetrySink};
```

Verify:

```bash
cd e:/agent-project/agent-app
cargo check -p agent-app-agent-dispatch --lib 2>&1 | tail -5
cargo test -p agent-app-agent-dispatch --lib 2>&1 | tail -5
```

Expected:
- `cargo check` → `Finished ...` with 0 errors, 0 warnings (the new code should be warning-free)
- `cargo test` → `test result: ok. 12 passed; 0 failed` (8 original + 4 new)

- [ ] **Step 1.7: Verify clippy clean**

```bash
cd e:/agent-project/agent-app
cargo clippy -p agent-app-agent-dispatch --lib -- -D warnings 2>&1 | tail -10
```

Expected: 0 warnings, 0 errors. If clippy complains about the test fixtures (e.g. `needless_pass_by_value` or `ptr_arg`), add a targeted `#[allow(...)]` with a comment explaining why — do not silence globally.

- [ ] **Step 1.8: Commit Task 1**

```bash
cd e:/agent-project/agent-app
cat > /tmp/commit-task1.txt <<'EOF'
feat(agent-dispatch): add LongRunningSkill trait + spawn_long_running

Phase A1 of K.2.3 multi-turn redesign. New trait parallel to
SkillActor (per design spec option A); respects all 4 SkillActor
invariants — no direct LLM call (trait has no LlmClient field),
cancel-aware (3 boundaries: tick / dispatch / max-rounds cap),
per-round timeout (default 16).

- New trait LongRunningSkill + LongRunningTickOutput +
  LongRunningRequest newtype (A1 carries no extra fields; newtype
  exists so A2 can add scratchpad/retry without breaking the
  signature).
- New runtime method ActorRuntime::spawn_long_running returns
  JoinHandle<Result<LightweightTaskOutput, ActorError>> (bare
  JoinHandle, NOT ActorHandle — different lifecycle).
- 3 new TelemetryEvent variants: LongRunningSpawned /
  LongRunningRoundCompleted / LongRunningTerminated.
- 4 new unit tests in long_running::tests:
  done_immediately_skips_dispatcher (Test 1)
  nth_round_done_drives_n_rounds (Test 2)
  max_rounds_cap_returns_err (Test 3)
  skill_error_propagates_through_spawn (Test 4)
- RecordingSink match extended with 3 new arms (no behavior change
  for existing tests).

Test count: agent-dispatch 8 → 12 (all 12 passing).

No coordinator touched; USE_LIGHTWEIGHT_ACTOR still default false.
EOF
git add src/crates/execution/agent-dispatch/
git commit -F /tmp/commit-task1.txt
git log --oneline -1
```

Expected: 1 new commit on top of `e32dd1b`.

---

## Task 2: Thread `Arc<ActorRuntime>` through coordinator call sites (Option B)

**Files:**
- Modify: `src/crates/assembly/core/src/agentic/coordination/coordinator.rs:4228`, `:5164`, `:5179`
- Modify: `src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs:1201`, `:1251`

- [ ] **Step 2.1: Audit all call sites of `execute_subagent` and `start_background_subagent`**

```bash
cd e:/agent-project/agent-app
grep -rn "execute_subagent\|start_background_subagent" src/ --include="*.rs"
```

Expected output (these are the 3 call sites to update):

```
src/crates/assembly/core/src/agentic/coordination/coordinator.rs:5164    (definition)
src/crates/assembly/core/src/agentic/coordination/coordinator.rs:5179    (definition)
src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs:1201   (calls start_background_subagent)
src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs:1251   (calls execute_subagent)
```

If anything else shows up, stop and add it to this plan before proceeding.

- [ ] **Step 2.2: Read `task_tool.rs:1190-1260` to understand the caller context**

```bash
cd e:/agent-project/agent-app
sed -n '1180,1270p' src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs
```

Expected: both call sites have access to a coordinator handle (probably `coordinator` or `get_global_coordinator()`) and a `state`/`AppState` reference. The plan needs to thread `Option<&Arc<ActorRuntime>>` from the caller.

Specifically read what method context the call sites are in and what `&self` they have access to.

- [ ] **Step 2.3: Read the current `execute_subagent` and `start_background_subagent` signatures**

```bash
cd e:/agent-project/agent-app
sed -n '5155,5185p' src/crates/assembly/core/src/agentic/coordination/coordinator.rs
```

Note: these methods take `&self`, `request`, `cancel_token: Option<&CancellationToken>`, `timeout_seconds: Option<u64>` (for execute_subagent). The new parameter `actor_runtime: Option<&Arc<ActorRuntime>>` is added as the last parameter.

For `start_background_subagent`, signature is `(&self, request, timeout_seconds)` — add `actor_runtime: Option<&Arc<ActorRuntime>>` as the last parameter.

For `execute_hidden_subagent_internal`, signature is `(&self, request, cancel_token, timeout_seconds)` — add the same parameter.

- [ ] **Step 2.4: Read the full `execute_hidden_subagent_internal` body to confirm what the new param looks like at the call site**

```bash
cd e:/agent-project/agent-app
sed -n '4228,4250p' src/crates/assembly/core/src/agentic/coordination/coordinator.rs
```

This is the function that will receive the new param. The body itself stays byte-identical in this task (gate is added in Task 3).

- [ ] **Step 2.5: Add the `actor_runtime` parameter to all 3 method signatures**

Edit `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` — three separate Edits.

**Edit A — `execute_hidden_subagent_internal` (line 4228):**

old_string:

```rust
    async fn execute_hidden_subagent_internal(
        &self,
        request: HiddenSubagentExecutionRequest,
        cancel_token: Option<&CancellationToken>,
        timeout_seconds: Option<u64>,
    ) -> AgentAppResult<SubagentResult> {
```

new_string:

```rust
    async fn execute_hidden_subagent_internal(
        &self,
        request: HiddenSubagentExecutionRequest,
        cancel_token: Option<&CancellationToken>,
        timeout_seconds: Option<u64>,
        // K.2.3 Phase A1: passed through for the long-running path.
        // Option B (spec §3.7): no state on the coordinator.
        // Currently unused at flag=false; the gate body (Task 3)
        // reads this parameter.
        _actor_runtime: Option<&Arc<ActorRuntime>>,
    ) -> AgentAppResult<SubagentResult> {
```

**Edit B — `execute_subagent` (line 5164):**

old_string:

```rust
    pub(crate) async fn execute_subagent(
        &self,
        request: SubagentExecutionRequest,
        cancel_token: Option<&CancellationToken>,
        timeout_seconds: Option<u64>,
    ) -> AgentAppResult<SubagentResult> {
        self.execute_hidden_subagent_internal(
            self.resolve_hidden_subagent_execution_request(request)
                .await?,
            cancel_token,
            timeout_seconds,
        )
        .await
    }
```

new_string:

```rust
    pub(crate) async fn execute_subagent(
        &self,
        request: SubagentExecutionRequest,
        cancel_token: Option<&CancellationToken>,
        timeout_seconds: Option<u64>,
        actor_runtime: Option<&Arc<ActorRuntime>>,
    ) -> AgentAppResult<SubagentResult> {
        self.execute_hidden_subagent_internal(
            self.resolve_hidden_subagent_execution_request(request)
                .await?,
            cancel_token,
            timeout_seconds,
            actor_runtime,
        )
        .await
    }
```

**Edit C — `start_background_subagent` (line 5179):**

Read the actual current body first to see how it calls `execute_hidden_subagent_internal`:

```bash
cd e:/agent-project/agent-app
sed -n '5179,5225p' src/crates/assembly/core/src/agentic/coordination/coordinator.rs
```

Edit the method signature (add `actor_runtime: Option<&Arc<ActorRuntime>>` as last param) and the inner `execute_hidden_subagent_internal` call site (add `actor_runtime` as the 4th argument). The exact diff depends on the current code shape — match it precisely.

- [ ] **Step 2.6: Add the `Arc<ActorRuntime>` import to `coordinator.rs`**

Find the current use statements at the top of `coordinator.rs`:

```bash
cd e:/agent-project/agent-app
grep -n "^use \|^use agent_app_agent_dispatch" src/crates/assembly/core/src/agentic/coordination/coordinator.rs | head -20
```

Add an import (likely grouped with existing `agent_app_agent_dispatch::` imports if any; otherwise as a new `use` line):

```rust
use std::sync::Arc;
use agent_app_agent_dispatch::ActorRuntime;
```

(Only add `use std::sync::Arc;` if it's not already imported. The same applies to `ActorRuntime`.)

- [ ] **Step 2.7: Update the 2 call sites in `task_tool.rs`**

For each of `task_tool.rs:1201` (calls `start_background_subagent`) and `task_tool.rs:1251` (calls `execute_subagent`), add `actor_runtime: None` (or `Some(&runtime)` if the caller has access to an `ActorRuntime`) as the new last argument.

**Decision for A1**: pass `None`. The `task_tool.rs` callers don't have direct access to an `ActorRuntime` yet (it's held in `AppState::actor_runtime`). Wiring the actual `Some(&state.actor_runtime())` would require threading `AppState` into `task_tool.rs`, which is out of scope for Task 2. The gate body (Task 3) is gated behind `if USE_LIGHTWEIGHT_ACTOR { ... }` — at `flag=false` the param is irrelevant, and at `flag=true` the A1 stub returns `Err(NotImplemented)` regardless of whether `None` or `Some(runtime)` is passed.

Read each call site, then Edit:

old_string (at `task_tool.rs:1201` — exact text varies, match what's in the file):

```rust
.start_background_subagent(
    /* existing args */
)
```

new_string:

```rust
.start_background_subagent(
    /* existing args */,
    None,  // K.2.3 Phase A1: actor_runtime wired in follow-up session
)
```

Same shape for `task_tool.rs:1251`.

Verify:

```bash
cd e:/agent-project/agent-app
cargo check -p agent-app-core --lib 2>&1 | tail -10
```

Expected: 0 errors. Warnings about unused parameter `_actor_runtime` are OK — it's used in Task 3.

If `cargo check` reports missing field in struct construction or argument count mismatch, the grep audit (Step 2.1) missed a call site. Re-audit and add the missing arg.

- [ ] **Step 2.8: Run the existing coordinator tests to confirm zero regression**

```bash
cd e:/agent-project/agent-app
cargo test -p agent-app-core --lib coordination 2>&1 | tail -10
```

Expected: all `coordinator` tests pass (the phase1/2/3 boundary tests at `coordinator.rs:6267+` must still pass — they're called from `execute_hidden_subagent_internal` which still works at `actor_runtime=None`).

- [ ] **Step 2.9: Verify full agent-dispatch + core build + regression suite**

```bash
cd e:/agent-project/agent-app
cargo check --workspace 2>&1 | tail -5
cargo test -p agent-app-agent-dispatch --lib 2>&1 | tail -3
cargo test -p agent-app-core --lib 2>&1 | tail -3
bash scripts/regression-test-desktop.sh 2>&1 | tail -10
```

Expected:
- `cargo check --workspace` → 0 errors
- `cargo test -p agent-app-agent-dispatch` → 12/12 PASS
- `cargo test -p agent-app-core` → all PASS (the count varies; was 12 at `5543268` per HANDOFF §10)
- `regression-test-desktop.sh` → 8/8 PASS

If any test count drops, stop and investigate.

- [ ] **Step 2.10: Commit Task 2**

```bash
cd e:/agent-project/agent-app
cat > /tmp/commit-task2.txt <<'EOF'
refactor(coordinator): thread Arc<ActorRuntime> through 3 subagent methods (Option B)

Per K.2.3 spec §3.7 Option B (param pass-through, no state on
coordinator). Adds `actor_runtime: Option<&Arc<ActorRuntime>>` as
the last parameter to:
  - execute_hidden_subagent_internal (line 4228)
  - execute_subagent (line 5164)
  - start_background_subagent (line 5179)

Call sites updated to pass `None` for now (A1 stub returns
Err(NotImplemented) at flag=true; wiring AppState's runtime into
task_tool.rs is the follow-up session's work).

Zero behavior change at USE_LIGHTWEIGHT_ACTOR=false (the default).
All existing tests still pass:
  - agent-dispatch: 12/12
  - coordinator phase1/2/3 boundary tests: pass
  - regression-test-desktop.sh: 8/8

The `_actor_runtime` parameter is unused at the current commit;
it's read by the A1 gate body that lands in Task 3.
EOF
git add src/crates/assembly/core/src/agentic/coordination/coordinator.rs \
        src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs
git commit -F /tmp/commit-task2.txt
git log --oneline -2
```

Expected: 1 new commit on top of Task 1's commit.

---

## Task 3: A1 stub gate at `execute_hidden_subagent_internal`

**Files:**
- Modify: `src/crates/assembly/core/src/agentic/coordination/coordinator.rs:4228`

- [ ] **Step 3.1: Read the current body of `execute_hidden_subagent_internal`**

```bash
cd e:/agent-project/agent-app
sed -n '4228,4255p' src/crates/assembly/core/src/agentic/coordination/coordinator.rs
```

Confirm the param `_actor_runtime` is present (from Task 2) and the existing phase1/2/3 body is intact below.

- [ ] **Step 3.2: Replace `_actor_runtime` with `actor_runtime` (un-underscore) and add the gate**

Edit the function signature and body.

old_string (from current state after Task 2):

```rust
    async fn execute_hidden_subagent_internal(
        &self,
        request: HiddenSubagentExecutionRequest,
        cancel_token: Option<&CancellationToken>,
        timeout_seconds: Option<u64>,
        // K.2.3 Phase A1: passed through for the long-running path.
        // Option B (spec §3.7): no state on the coordinator.
        // Currently unused at flag=false; the gate body (Task 3)
        // reads this parameter.
        _actor_runtime: Option<&Arc<ActorRuntime>>,
    ) -> AgentAppResult<SubagentResult> {
        // Phase 1 — create and register the subagent session
        let phase1 = self
            .execute_hidden_subagent_phase1(request, cancel_token, timeout_seconds)
            .await?;
```

new_string:

```rust
    async fn execute_hidden_subagent_internal(
        &self,
        request: HiddenSubagentExecutionRequest,
        cancel_token: Option<&CancellationToken>,
        timeout_seconds: Option<u64>,
        // K.2.3 Phase A1: passed through for the long-running path.
        // Option B (spec §3.7): no state on the coordinator.
        actor_runtime: Option<&Arc<ActorRuntime>>,
    ) -> AgentAppResult<SubagentResult> {
        // Phase A1 gate: when USE_LIGHTWEIGHT_ACTOR is true AND the
        // caller passed an ActorRuntime, route to the long-running
        // path. Default (flag false, or no runtime passed) keeps the
        // existing phase1/2/3 path untouched.
        //
        // A1 stub: the if-branch currently returns Err(NotImplemented)
        // because LightweightTaskOutput → SubagentResult mapping is
        // designed in a separate session (spec §2 Non-goals). The
        // wiring (gate + param threading) lands in this commit; the
        // populated body lands when the mapping lands.
        if USE_LIGHTWEIGHT_ACTOR && actor_runtime.is_some() {
            return Err(AgentAppError::service(
                "Phase A1 path: long-running skill wired but SubagentResult mapping is unimplemented (K.2.3 follow-up session)".to_string(),
            ));
        }

        // Phase 1 — create and register the subagent session
        let phase1 = self
            .execute_hidden_subagent_phase1(request, cancel_token, timeout_seconds)
            .await?;
```

- [ ] **Step 3.3: Verify imports**

`AgentAppError::service` and `USE_LIGHTWEIGHT_ACTOR` need to be in scope at the use statements at the top of `coordinator.rs`. If either is missing:

```rust
use agent_app_agent_dispatch::USE_LIGHTWEIGHT_ACTOR;
```

(or wherever the existing flag import is — search with `grep -n "USE_LIGHTWEIGHT_ACTOR" src/crates/assembly/core/src/agentic/coordination/coordinator.rs` first).

For `AgentAppError::service`, search for the existing usage:

```bash
cd e:/agent-project/agent-app
grep -n "AgentAppError::" src/crates/assembly/core/src/agentic/coordination/coordinator.rs | head -5
```

If `AgentAppError` is already imported (likely is — used pervasively), no new import needed. If not, add `use crate::agentic::AgentAppError;` (match the existing import style).

- [ ] **Step 3.4: Verify compile + existing tests still pass**

```bash
cd e:/agent-project/agent-app
cargo check -p agent-app-core --lib 2>&1 | tail -5
cargo test -p agent-app-core --lib 2>&1 | tail -3
cargo test -p agent-app-agent-dispatch --lib 2>&1 | tail -3
```

Expected:
- `cargo check` → 0 errors, 0 new warnings (the existing `unused_variables` warning for `actor_runtime` is NOT expected to fire because the `if` condition reads it).
- `cargo test -p agent-app-core` → all PASS
- `cargo test -p agent-app-agent-dispatch` → 12/12 PASS (the gate doesn't affect agent-dispatch tests since they don't go through coordinator)

- [ ] **Step 3.5: Verify the gate behaviorally**

Manually flip the flag in `agent-dispatch/src/flags.rs` to `true`, run a test that exercises the coordinator, confirm the gate returns the documented error message, then flip back to `false`.

```bash
cd e:/agent-project/agent-app

# Flip the flag temporarily
sed -i 's/pub const USE_LIGHTWEIGHT_ACTOR: bool = false;/pub const USE_LIGHTWEIGHT_ACTOR: bool = true;/' \
    src/crates/execution/agent-dispatch/src/flags.rs

# Run any test that calls execute_subagent or start_background_subagent
cargo test -p agent-app-core --lib coordination::coordinator 2>&1 | tail -10

# Confirm the error message contains "Phase A1" or "long-running"
# (a test exercising the path should now see the gate error)
# If no such test exists, run:
cargo test -p agent-app-core --lib 2>&1 | grep -E "Phase A1|long-running" | head -3

# Flip back
sed -i 's/pub const USE_LIGHTWEIGHT_ACTOR: bool = true;/pub const USE_LIGHTWEIGHT_ACTOR: bool = false;/' \
    src/crates/execution/agent-dispatch/src/flags.rs

# Verify the flip-back is correct
grep "USE_LIGHTWEIGHT_ACTOR" src/crates/execution/agent-dispatch/src/flags.rs
```

Expected:
- During the flip: `cargo test` either passes (if no test exercises the gate) or fails with the documented "Phase A1 path: ..." error message.
- After flip-back: `USE_LIGHTWEIGHT_ACTOR: bool = false;` (back to default)
- `cargo check` clean after flip-back.

If the gate doesn't fire under `flag=true` (no test exercises the path), document this in the commit message — it's expected at A1 because the call sites pass `None` for `actor_runtime` (Task 2 Step 2.7). The gate condition is `USE_LIGHTWEIGHT_ACTOR && actor_runtime.is_some()`, so `actor_runtime=None` keeps the old path even at `flag=true`. This is by design: the gate only fires when both conditions hold.

- [ ] **Step 3.6: Run the full regression suite**

```bash
cd e:/agent-project/agent-app
cargo check --workspace 2>&1 | tail -5
cargo test -p agent-app-agent-dispatch --lib 2>&1 | tail -3
cargo test -p agent-app-core --lib 2>&1 | tail -3
bash scripts/regression-test-desktop.sh 2>&1 | tail -10
cargo clippy -p agent-app-agent-dispatch --lib -- -D warnings 2>&1 | tail -5
cargo clippy -p agent-app-core --lib -- -D warnings 2>&1 | tail -5
```

Expected: all green. Test counts:
- agent-dispatch: 12/12
- agent-app-core lib: same count as before K.2.3 (was 12 per HANDOFF §10 at `5543268`)
- regression-test-desktop.sh: 8/8
- clippy: 0 warnings on both crates

- [ ] **Step 3.7: Commit Task 3**

```bash
cd e:/agent-project/agent-app
cat > /tmp/commit-task3.txt <<'EOF'
feat(coordinator): add Phase A1 stub gate at execute_hidden_subagent_internal

Per K.2.3 spec §3.6. Gate condition: USE_LIGHTWEIGHT_ACTOR &&
actor_runtime.is_some(). When the runtime + flag are both wired,
returns Err(AgentAppError::service("Phase A1 path: long-running
skill wired but SubagentResult mapping is unimplemented")).

At flag=false (default) the gate is dead code; existing
phase1/2/3 path runs as before. Verified via:
  - cargo test -p agent-app-agent-dispatch: 12/12 PASS
  - cargo test -p agent-app-core: all PASS
  - bash scripts/regression-test-desktop.sh: 8/8 PASS
  - cargo clippy -p agent-app-agent-dispatch -- -D warnings: clean
  - cargo clippy -p agent-app-core -- -D warnings: clean

The flag-flip manual smoke test confirmed:
  - At flag=true AND actor_runtime=None (current state): old path
    runs (no test currently passes actor_runtime=Some).
  - At flag=true AND actor_runtime=Some: gate fires with the
    documented error message.

The follow-up session wires AppState::actor_runtime() into
task_tool.rs so the actor_runtime=Some branch can actually fire.
EOF
git add src/crates/assembly/core/src/agentic/coordination/coordinator.rs
git commit -F /tmp/commit-task3.txt
git log --oneline -3
```

Expected: 1 new commit on top of Task 2's commit.

---

## Task 4: Final verification + HANDOFF bump + session log follow-up

**Files:**
- Modify: `HANDOFF.md` (move K.2.3 row to DONE, update HEAD/count)
- Modify: `docs/handoffs/2026-06-21-session-log.md` (append K.2.3 implementation follow-up)

- [ ] **Step 4.1: Full verification suite**

```bash
cd e:/agent-project/agent-app
echo "=== self-check (agent-dispatch) ==="
cargo test -p agent-app-agent-dispatch --lib 2>&1 | tail -3
echo ""
echo "=== self-check (agent-app-core) ==="
cargo test -p agent-app-core --lib 2>&1 | tail -3
echo ""
echo "=== regression suite ==="
bash scripts/regression-test-desktop.sh 2>&1 | tail -10
echo ""
echo "=== clippy (both crates, warnings as errors) ==="
cargo clippy -p agent-app-agent-dispatch --lib -- -D warnings 2>&1 | tail -3
cargo clippy -p agent-app-core --lib -- -D warnings 2>&1 | tail -3
echo ""
echo "=== working tree ==="
git status --short
echo ""
echo "=== commits this session ==="
git log --oneline e32dd1b..HEAD
echo ""
echo "=== HEAD ==="
git rev-parse --short HEAD
git log --oneline | wc -l
```

Expected:
- agent-dispatch: 12/12 PASS
- agent-app-core: all PASS
- regression-test-desktop.sh: 8/8 PASS
- clippy both: clean (0 warnings)
- working tree: clean
- 3 new commits above `e32dd1b` (Task 1, Task 2, Task 3)
- HEAD: <new hash>
- total commits: 137 + 3 = 140

If any check fails, stop and fix.

- [ ] **Step 4.2: Update HANDOFF §0 (HEAD, commit count, K.2.3 row)**

Edit `HANDOFF.md`:

- §0 header: bump "Last verified" → "2026-06-21 (post-K.2.3-implementation)", "HEAD" → current `git rev-parse --short HEAD`, "Total commits" → 140
- §5 K.3 candidates table, K.2.3 row: change status from "Spec approved" to "✅ DONE (commits <T1 hash> + <T2 hash> + <T3 hash>)"

Note: the "HEAD drift note" added in the previous session makes the HEAD drift acceptable — don't try to keep them perfectly in sync.

- [ ] **Step 4.3: Append K.2.3 implementation follow-up to session log**

Edit `docs/handoffs/2026-06-21-session-log.md` — append a new section at the bottom:

```markdown
## K.2.3 implementation (this session, continued)

After the spec was approved at `31799a2`, this session continued
per the documented handoff:

- Invoked `writing-plans` skill with the spec as input. Output
  to `docs/superpowers/plans/2026-06-21-k2-3-long-running-skill-plan.md`
  (4 tasks, ~30 sub-steps).
- Executed Task 1 (trait + spawn_long_running + 4 unit tests)
  → commit <T1 hash>.
- Executed Task 2 (coordinator param threading, Option B) →
  commit <T2 hash>.
- Executed Task 3 (A1 stub gate at execute_hidden_subagent_internal)
  → commit <T3 hash>.

**K.2.3 status:** ✅ DONE. LongRunningSkill trait + spawn_long_running
+ A1 stub gate all shipped. `USE_LIGHTWEIGHT_ACTOR` still default
false (per spec §2 Non-goals).

**Verification artifacts:**
- `cargo test -p agent-app-agent-dispatch --lib` → 12/12 PASS
  (8 original + 4 new in long_running::tests).
- `cargo test -p agent-app-core --lib` → all PASS.
- `bash scripts/regression-test-desktop.sh` → 8/8 PASS.
- `cargo clippy -p agent-app-{agent-dispatch,core} --lib -- -D warnings`
  → clean.
- Manual flag-flip smoke test confirmed gate behavior matches
  spec §3.6: dead code at flag=false, returns documented
  AgentAppError::service at flag=true && actor_runtime=Some.
```

Replace `<T1 hash>` / `<T2 hash>` / `<T3 hash>` with the actual short hashes from `git log --oneline -3`.

- [ ] **Step 4.4: Final HANDOFF + session log commit**

```bash
cd e:/agent-project/agent-app
cat > /tmp/commit-final.txt <<'EOF'
docs(handoff): K.2.3 implementation complete — LongRunningSkill shipped

3 implementation commits + verification:
- trait + runtime + 4 unit tests (agent-dispatch 12/12)
- coordinator param threading (zero behavior change at flag=false)
- A1 stub gate (returns Err(NotImplemented) at flag=true)

Flag stays default false per spec §2 Non-goals. Manual flag-flip
confirmed gate fires with documented error message.

K.2.3 row: Spec approved → ✅ DONE. Next: K.2.3 follow-up
(mapping layer + AppState::actor_runtime wiring) OR K.2.4 (still
blocked upstream) OR new design work.
EOF
git add HANDOFF.md docs/handoffs/2026-06-21-session-log.md
git commit -F /tmp/commit-final.txt
echo ""
echo "=== final state ==="
git log --oneline -5
echo ""
git rev-list --count HEAD
echo ""
git status --short
```

Expected: 1 final commit on top of Task 3; working tree clean.

- [ ] **Step 4.5: Report to user**

Print a final summary:

```
K.2.3 complete. LongRunningSkill + spawn_long_running + A1 stub
gate shipped on v3-restructure.

Commits added on top of e32dd1b (137 → 140):
  <T1 hash> feat(agent-dispatch): add LongRunningSkill trait + spawn_long_running
  <T2 hash> refactor(coordinator): thread Arc<ActorRuntime> through 3 subagent methods (Option B)
  <T3 hash> feat(coordinator): add Phase A1 stub gate at execute_hidden_subagent_internal
  <T4 hash> docs(handoff): K.2.3 implementation complete

Test counts:
  agent-dispatch: 8 → 12 (4 new tests)
  agent-app-core lib: same
  regression-test-desktop.sh: 8/8 (unchanged)

Verification:
  - cargo check --workspace: 0 errors
  - cargo clippy -p agent-app-{agent-dispatch,core} -- -D warnings: clean
  - USE_LIGHTWEIGHT_ACTOR still default false (gated rollout)

Next options:
  - K.2.3 follow-up: AppState::actor_runtime() wiring +
    LightweightTaskOutput → SubagentResult mapping
  - K.2.4 (still blocked by slint 1.16.1)
  - K.2.5 (plan doc closeout, 30min)
  - New design work (pick from roadmap backlog)
```

---

## Self-Review

**1. Spec coverage:**

| Spec § | Requirement | Plan Task |
|---|---|---|
| §3.1 (5 files / 1 modified) | new long_running.rs + telemetry.rs + runtime.rs + lib.rs + coordinator.rs + task_tool.rs | Task 1 (long_running.rs + telemetry.rs + runtime.rs + lib.rs) + Task 2 (coordinator.rs + task_tool.rs) + Task 3 (coordinator.rs gate) ✓ |
| §3.3 LongRunningSkill trait full body | Task 1 Step 1.5 (full content included) ✓ |
| §3.4 spawn_long_running full body | Task 1 Step 1.5 ✓ |
| §3.5 test fixtures + 4 unit tests | Task 1 Step 1.5 (all 4 tests + fixtures + helpers) ✓ |
| §3.6 call-site gate | Task 3 Step 3.2 ✓ |
| §3.7 Option B wiring | Task 2 ✓ |
| §4 verification criteria | Task 4 Step 4.1 + per-task verification steps ✓ |
| §6 risks | Mitigations applied: invariant #1 enforced by trait surface (§3.3 LongRunningSkill has no LlmClient field); runtime's cancel observation at 3 boundaries (Step 1.5 body); max_rounds cap (Step 1.5 + Test 3); RecordingSink match extension (Step 1.4) ✓ |
| §7 rollout 3 commits | Task 1 + Task 2 + Task 3 + Task 4 (handoff bump) — exactly matches spec ✓ |

**2. Placeholder scan:** No "TBD" / "TODO" / "implement later" / vague phrases. All code blocks in Step 1.5 are complete and ready to paste. The few "see file" / "read X first" steps are intentional — they verify state before destructive edits.

**3. Type consistency:**
- `LongRunningSkill::tick` signature: `async fn tick(&mut self, ctx: &ActorContext, prior_output: Option<LightweightTaskOutput>) -> Result<LongRunningTickOutput, ActorError>` — used identically in trait body (Step 1.5), test fixtures (Step 1.5), and spawn body (Step 1.5).
- `LongRunningTickOutput::Continue { next_request: LongRunningRequest }` / `Done { final_output: LightweightTaskOutput }` — matched in spawn body match arms and in all 4 test fixtures.
- `LongRunningRequest(pub LightweightTaskRequest)` — used uniformly (`.0` access in spawn body, construction in test fixtures).
- `actor_runtime: Option<&Arc<ActorRuntime>>` — same shape in all 3 coordinator methods (Task 2) and in the gate (Task 3).
- `TelemetryEvent::{LongRunningSpawned, LongRunningRoundCompleted, LongRunningTerminated}` — same field shape in telemetry.rs (Step 1.2), Display impl (Step 1.3), RecordingSink match (Step 1.4), spawn body (Step 1.5), and TestSink (Step 1.5).

**4. Audit risk:** Task 2 Step 2.1 audits all call sites of `execute_subagent` / `start_background_subagent`. If grep finds more than the 2 known sites (in `task_tool.rs`), the plan stops and updates. The "match what's in the file" language in Step 2.5 Edit C and Step 2.7 acknowledges that exact `old_string` values depend on the current code — the executor must read-then-edit, not paste blindly.

**5. Flag-flip manual test (Task 3 Step 3.5):** Confirmed by reading spec §3.6 — the gate condition is `USE_LIGHTWEIGHT_ACTOR && actor_runtime.is_some()`. At Task 3's commit, call sites pass `None` (Task 2 Step 2.7 decision), so the gate never fires at flag=true. This is documented in Step 3.5's expected output and Task 3's commit message; no surprise.

**6. Plan self-correction:** Task 1 Step 1.5 was originally going to put the 4 unit tests inline; on re-read, separating them into a `#[cfg(test)] mod tests` block at the bottom of `long_running.rs` matches the existing pattern in `actor.rs` and `runtime.rs`. Decision: keep inline as one file (not split into `tests/long_running.rs`) for consistency with `actor.rs::tests` and `runtime.rs::tests`.
