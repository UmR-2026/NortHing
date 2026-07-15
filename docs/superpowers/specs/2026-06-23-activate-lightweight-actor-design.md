# Activate USE_LIGHTWEIGHT_ACTOR — A2 → Production Path

> **Status:** Design Complete — Ready for Implementation
> **Date:** 2026-06-23
> **Scope:** K.2.3 Phase A2 activation — flip `USE_LIGHTWEIGHT_ACTOR = true` and verify the A2 long-running path replaces the legacy phase1/2/3 path

---

## 1. Motivation

K.2.3 Phase A1 (commit `d0ee0da`) and Phase A2 (commits `821137e` + review fixes) shipped the infrastructure for the long-running-skill path:

- **`LongRunningSkill` trait** in `agent-dispatch` (24/24 tests passing)
- **`ActorRuntime::spawn_long_running`** with cancel + telemetry + timeout
- **`run_a1_path()`** in `a1_path.rs` routing coordinator through `CoordinatorHiddenSubagentSkill`
- **A2 per-round stepping** via `init_turn` + `tick` + `finalize_turn` + `build_result`
- **Mapping** `LightweightTaskOutput ⇄ SubagentResult` (9 unit tests)
- **Wiring** `AppState → Coordinator → ToolPipeline → TaskTool` (commits `cf1ca9a`, `7d66704`)
- **Production init** in `maybe_construct_actor_runtime` (gated by `USE_LIGHTWEIGHT_ACTOR`)

But the `USE_LIGHTWEIGHT_ACTOR` const flag is still `false`. The gate at `coordinator.rs:4282-4292` is dead code.

This spec activates the A2 path end-to-end so every `Task` tool call in the desktop app routes through `CoordinatorHiddenSubagentSkill` instead of the legacy `execute_hidden_subagent_phase1/2/3`.

---

## 2. Goal

Flip `USE_LIGHTWEIGHT_ACTOR` from `false` to `true` so that:

1. At desktop app boot, `maybe_construct_actor_runtime` constructs an `ActorRuntime` with 1 heartbeat actor (existing Phase I.3 logic).
2. The runtime is forwarded to the coordinator's `ToolPipeline` via `set_actor_runtime`.
3. Every `TaskTool::call` invocation passes `context.actor_runtime()` (now `Some(...)`) into `coordinator.execute_subagent`.
4. The A1 gate at `coordinator.rs:4282` fires, routing through `a1_path::run_a1_path`.
5. `CoordinatorHiddenSubagentSkill::tick` runs phase1 + `init_turn` (first tick), then `engine.tick()` per round (subsequent ticks), then `finalize_turn` + `build_result` (Done).

End-to-end behavior should be **identical** to the legacy path for the user (same `SubagentResult`, same error semantics, same cancel/timeout), but the call site benefits from per-round telemetry, cancel granularity, and the actor runtime's structured observability.

---

## 3. Non-Goals

- **A3 RoundExecutor refactor** — split `execute_round` into per-token yield points. Out of scope; separate spec after this one lands.
- **IPC adapter implementation** — `USE_ACTOR_IPC` stays `false`. The runtime is in-process.
- **New `LightweightTaskOutput` variants** — already complete.
- **Touching CLI** — CLI doesn't construct `AppState` or `actor_runtime`; the flag flip is desktop-only.
- **Production telemetry dashboard** — telemetry events fire (`LongRunningRoundCompleted`, `ActorTicked`) but no UI surfaces them yet.

---

## 4. Architecture Overview

```
                     ┌──────────────────────────────┐
                     │  Desktop App Boot (Phase I.3) │
                     │  USE_LIGHTWEIGHT_ACTOR=true  │
                     └──────────────┬───────────────┘
                                    ▼
                     ┌──────────────────────────────┐
                     │  maybe_construct_actor_      │
                     │  runtime(state, ui)          │
                     │  → ActorRuntime + Heartbeat  │
                     │  → state.set_actor_runtime() │
                     │  → coord.set_actor_runtime() │
                     └──────────────┬───────────────┘
                                    ▼
                     ┌──────────────────────────────┐
                     │  User invokes Task tool      │
                     └──────────────┬───────────────┘
                                    ▼
                     ┌──────────────────────────────┐
                     │  TaskTool::call              │
                     │  context.actor_runtime() → Some │
                     └──────────────┬───────────────┘
                                    ▼
                     ┌──────────────────────────────┐
                     │  coordinator.execute_subagent│
                     └──────────────┬───────────────┘
                                    ▼
                     ┌──────────────────────────────┐
                     │  execute_hidden_subagent_    │
                     │  internal                    │
                     │  USE_LIGHTWEIGHT_ACTOR=true  │
                     │  → A1 GATE FIRES             │
                     └──────────────┬───────────────┘
                                    ▼
                     ┌──────────────────────────────┐
                     │  a1_path::run_a1_path        │
                     │  → spawn_long_running(skill) │
                     └──────────────┬───────────────┘
                                    ▼
                     ┌──────────────────────────────┐
                     │  CoordinatorHiddenSubagentSkill│
                     │  tick 1: phase1 + init_turn  │
                     │  tick N: engine.tick()       │
                     │  tick final: finalize + build │
                     └──────────────┬───────────────┘
                                    ▼
                     ┌──────────────────────────────┐
                     │  LightweightTaskOutput →     │
                     │  SubagentResult (mapped)     │
                     └──────────────────────────────┘
```

---

## 5. Design

### 5.1 Const Flag Flip

**File:** `src/crates/execution/agent-dispatch/src/flags.rs`

```diff
- pub const USE_LIGHTWEIGHT_ACTOR: bool = false;
+ pub const USE_LIGHTWEIGHT_ACTOR: bool = true;
```

**Side effects:**
- `all_flags_default_off_in_phase_1` test in `flags.rs:42-47` will FAIL — this is the expected signal that we have crossed out of "Phase 1 dark launch". The test must be updated (renamed + relaxed) to reflect that `USE_LIGHTWEIGHT_ACTOR` is now on.
- All other 3 flags stay `false` (they represent future IPC work).
- `a1_path.rs` doc header at line 42 updates from "default false" to "default true, A2 path".
- `coordinator.rs:4280` comment updates from "Default (flag false)" to "Default (flag true)".

### 5.2 Test Update

**File:** `src/crates/execution/agent-dispatch/src/flags.rs:32-47`

Rename `all_flags_default_off_in_phase_1` → `flags_phase_appropriate`. The test should assert:
- `USE_LIGHTWEIGHT_ACTOR == true` (activated as of this spec)
- `USE_ONESHOT_DISPATCHER == false` (still Phase 1)
- `USE_ACTOR_IPC == false` (still Phase 1)
- `USE_DISPATCHER_IPC == false` (still Phase 1)

### 5.3 Doc Updates

- `docs/PROJECT_STATE.md` — bump K.2.3 from "A2 complete, flag false" to "A2 activated, flag true"
- `.task/HANDOVER.md` — record the activation
- `docs/superpowers/specs/2026-06-18-lightweight-actor-design.md` — flag status table

### 5.4 Regression Test

Add a new integration test in `src/crates/execution/agent-runtime/tests/` (or `tests/northhing_agent_dispatch.rs`):

**Test name:** `a1_path_routes_through_long_running_skill`

**Setup:**
- Create an `ActorRuntime` with a `NoopTelemetrySink`
- Create a `CoordinatorHiddenSubagentSkill` directly (skip the spawn_long_running path)
- Call `tick()` with a mock `ActorContext`

**Assertion:**
- First tick returns `LongRunningTickOutput::Continue`
- Phase1 was invoked (mock coordinator records the call)
- `turn_state` and `execution_context` are populated

This test exercises the A2 path with a mocked coordinator. It catches regressions if the flag flip or wiring breaks.

### 5.5 A3 Investigation (Phase 2 of the larger plan)

After this spec lands, the next session investigates A3 value. The investigation produces a separate spec or a "no, skip A3" decision doc. Out of scope for this activation.

---

## 6. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Subagent behavior changes after flag flip | Medium | High — user-visible | Run full desktop regression before commit; compare `SubagentResult` for representative `Task` invocations |
| Heartbeat actor interferes with telemetry | Low | Low | Heartbeat is `OneShot`, runs once, exits; no interference |
| `all_flags_default_off_in_phase_1` test removal breaks intent | Low | Low | Rename to `flags_phase_appropriate` + comment why the change |
| Pre-existing test build errors in `coordinator.rs` boundary tests block verification | Medium | Medium | Per K.2.3 session log: 37 pre-existing errors verified independent of this change; document as known issue |
| A2 review fixes (P0-1 dialog_turn_id, P0-2 workspace, P1 FinishReason) not all honored under flag | Low | High | Verify with `cargo test -p northhing-core --lib coordination::` after flip |

---

## 7. Acceptance Criteria

- [ ] `USE_LIGHTWEIGHT_ACTOR = true` in `flags.rs`
- [ ] `flags_phase_appropriate` test passes (replaces `all_flags_default_off_in_phase_1`)
- [ ] `cargo build -p northhing` 0 errors
- [ ] `cargo build -p northhing-core` 0 errors
- [ ] `cargo test -p northhing-agent-dispatch --lib` 24/24 PASS
- [ ] `cargo test -p northhing-core --lib` (filtering out 37 pre-existing boundary test errors) passes for all other modules
- [ ] `bash scripts/regression-test-desktop.sh` 8/8 PASS
- [ ] New `a1_path_routes_through_long_running_skill` integration test PASS
- [ ] `.task/HANDOVER.md` updated with activation record
- [ ] `docs/PROJECT_STATE.md` updated
- [ ] `docs/superpowers/specs/2026-06-18-lightweight-actor-design.md` flag table updated
- [ ] Single commit per file (5-10 commits total); no behavior change beyond flag flip + test updates

---

## 8. Out of Scope (Next Steps After This Lands)

- **A3 RoundExecutor refactor** — separate spec, depends on this activation's telemetry data
- **Phase 2: ToolDispatcher activation** — `USE_ONESHOT_DISPATCHER` stays `false`
- **Phase 3: IPC adapters** — `USE_ACTOR_IPC` and `USE_DISPATCHER_IPC` stay `false`
- **Telemetry dashboard** — out of project scope

---

## 9. Plan Reference

This spec is implemented via the LAEP protocol:
- Spec: `docs/superpowers/specs/2026-06-23-activate-lightweight-actor-design.md` (this file)
- Plan: `docs/plans/2026-06-23-activate-lightweight-actor-impl.md` (separate document)
- Execution: 6 tasks via LAEP archive (see impl plan)
- Review: human reviewer + automated checklist