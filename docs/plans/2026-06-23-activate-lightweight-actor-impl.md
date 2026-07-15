# A2 Activation — Implementation Plan

> **Status:** Implementation Plan — Ready for LAEP Execution
> **Date:** 2026-06-23
> **Spec:** `docs/superpowers/specs/2026-06-23-activate-lightweight-actor-design.md`
> **Execution Mode:** LAEP (Lightweight Agent Execution Protocol)
> **Audience:** Coding agent (this doc) + Human reviewer (separate review-guide per task)

---

## 0. Plan Overview

This plan activates `USE_LIGHTWEIGHT_ACTOR = true` end-to-end. Six LAEP tasks, each scoped to a single concern with its own review-guide and verification report. Each task produces a single commit.

### Task Map

| ID | Task | Files | Est. Time | Commit Type |
|----|------|-------|-----------|-------------|
| T1 | Flip const flag | `flags.rs` | 5min | feat |
| T2 | Update phase-flag test | `flags.rs` | 10min | test |
| T3 | Update a1_path.rs doc header | `a1_path.rs` | 5min | docs |
| T4 | Update coordinator.rs comment | `coordinator.rs` | 5min | docs |
| T5 | Add integration test | new file in `tests/` | 30min | test |
| T6 | Run full regression + write HANDOVER + update PROJECT_STATE | `.task/HANDOVER.md`, `docs/PROJECT_STATE.md` | 1h | docs |
| T7 | Final verification (Plan Compliance Checker) | n/a | 10min | chore |

Total estimated time: ~2h.

---

## 1. LAEP Task Definitions

### Task 1: flip-const-flag

**Goal:** Flip `USE_LIGHTWEIGHT_ACTOR` from `false` to `true` in the canonical flag definition.

**Scope:** ONE file, ONE line change.

**Files modified:**
- `src/crates/execution/agent-dispatch/src/flags.rs`

**Change:**
```diff
- pub const USE_LIGHTWEIGHT_ACTOR: bool = false;
+ pub const USE_LIGHTWEIGHT_ACTOR: bool = true;
```

**Acceptance:**
- File compiles standalone: `cargo check -p northhing-agent-dispatch`
- The `all_flags_default_off_in_phase_1` test FAILS (expected — we fix in T2)
- All other workspace packages still compile (no syntax break)

**Tests:** none new; existing tests should fail in T2 catch-up step.

**Review focus:**
- Verify only ONE line changed
- Verify the comment above the const still describes the activated meaning (line 11-14)
- Verify no other const flag was touched

---

### Task 2: phase-flag-test-update

**Goal:** Rename and update the `all_flags_default_off_in_phase_1` test to reflect the new phase.

**Scope:** ONE file, ONE test rename + assertion update.

**Files modified:**
- `src/crates/execution/agent-dispatch/src/flags.rs`

**Change:**
```diff
- /// All four flags must default to `false` — this is the project's
- /// "dark launch" guarantee for Phase 1 of the actor rollout.
- /// If this test ever fails after a flag flip, the flip was deliberate
- /// and should be paired with a regression test (see `06-const-flag-usage.md`
- /// rule 4).
- #[test]
- fn all_flags_default_off_in_phase_1() {
-     assert!(!USE_LIGHTWEIGHT_ACTOR);
-     assert!(!USE_ONESHOT_DISPATCHER);
-     assert!(!USE_ACTOR_IPC);
-     assert!(!USE_DISPATCHER_IPC);
- }
+ /// Flags reflect the current phase of the rollout.
+ /// As of 2026-06-23 (spec `2026-06-23-activate-lightweight-actor-design`),
+ /// `USE_LIGHTWEIGHT_ACTOR` is activated. The other three flags represent
+ /// future work (one-shot dispatcher, IPC adapters) and remain off.
+ #[test]
+ fn flags_phase_appropriate() {
+     assert!(USE_LIGHTWEIGHT_ACTOR);
+     assert!(!USE_ONESHOT_DISPATCHER);
+     assert!(!USE_ACTOR_IPC);
+     assert!(!USE_DISPATCHER_IPC);
+ }
```

**Acceptance:**
- `cargo test -p northhing-agent-dispatch --lib flags` 1/1 PASS
- All other agent-dispatch tests still 23/23 PASS

**Tests:** the modified test itself.

**Review focus:**
- Verify the new test name reflects phase state, not "default off" semantics
- Verify the three other flags are still asserted as off
- Verify the doc comment references the activation spec

---

### Task 3: a1-path-doc-update

**Goal:** Update the module doc header in `a1_path.rs` to reflect flag activation.

**Scope:** ONE file, doc-only change.

**Files modified:**
- `src/crates/assembly/core/src/agentic/coordination/a1_path.rs`

**Change:** lines 41-46:
```diff
 //! `coordinator.rs::execute_hidden_subagent_internal` calls
 //! `run_a1_path()` when:
-//!   - `USE_LIGHTWEIGHT_ACTOR = true` (const flag, default false)
+//!   - `USE_LIGHTWEIGHT_ACTOR = true` (const flag, ACTIVATED 2026-06-23 per
+//!     `docs/superpowers/specs/2026-06-23-activate-lightweight-actor-design.md`)
 //!   - caller passed a non-None `actor_runtime: Option<&Arc<ActorRuntime>>`
```

**Acceptance:**
- `cargo check -p northhing-core` 0 errors
- No code changes, doc-only

**Tests:** none.

**Review focus:**
- Verify only the doc comment changed, not the code
- Verify the spec reference is correct
- Verify other comment blocks (line 47 "Both conditions must hold...") still make sense

---

### Task 4: coordinator-comment-update

**Goal:** Update the gate comment in `coordinator.rs` to reflect the new default.

**Scope:** ONE file, doc-only change.

**Files modified:**
- `src/crates/assembly/core/src/agentic/coordination/coordinator.rs`

**Change:** lines 4278-4282:
```diff
         // Phase A1 gate: when USE_LIGHTWEIGHT_ACTOR is true AND the
         // caller passed an ActorRuntime, route to the long-running
-        // path. Default (flag false, or no runtime passed) keeps the
-        // existing phase1/2/3 path untouched.
+        // path. Default (flag TRUE as of 2026-06-23, or no runtime passed)
+        // keeps the existing phase1/2/3 path untouched for callers that
+        // don't yet wire the runtime.
```

**Acceptance:**
- `cargo check -p northhing-core` 0 errors

**Tests:** none.

**Review focus:**
- Verify only the comment block changed
- Verify the gate logic at line 4282 (`if USE_LIGHTWEIGHT_ACTOR {`) is untouched

---

### Task 5: a1-path-integration-test

**Goal:** Add an integration test that exercises `CoordinatorHiddenSubagentSkill::tick` with a mocked coordinator.

**Scope:** ONE new test file + test fixture.

**Files created:**
- `src/crates/assembly/core/tests/a1_path_integration.rs`

**Skeleton:**
```rust
//! K.2.3 A2 path integration test.
//!
//! Verifies that `CoordinatorHiddenSubagentSkill::tick` correctly:
//! 1. Calls `execute_hidden_subagent_phase1` on the first tick
//! 2. Initializes turn state + execution context
//! 3. Returns `LongRunningTickOutput::Continue` after phase1
//! 4. Returns `LongRunningTickOutput::Continue` after `engine.tick()`

use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn a1_path_routes_through_long_running_skill() {
    // TODO: detailed mock setup
    // Implementation strategy: see `.task/archive/activate-lightweight-actor/review-guide.md`
    // for the full mock setup (mocked Coordinator + global coordinator injection).
    // If mock setup proves too brittle, mark with #[ignore] and document reason.
    todo!("Implement in coding phase")
}
```

**Acceptance:**
- Test compiles: `cargo test -p northhing-core --test a1_path_integration --no-run`
- Test passes: `cargo test -p northhing-core --test a1_path_integration`
- Test is skipped (with `#[ignore]`) if it cannot mock the global coordinator cleanly

**Tests:** the new test.

**Review focus:**
- Test MUST NOT modify any production code (it's an integration test, pure)
- Test SHOULD be skippable if mock setup proves brittle
- If skipped, document the reason in `.task/archive/activate-lightweight-actor/review-guide.md`

---

### Task 6: handover-and-state-update

**Goal:** Update `.task/HANDOVER.md` and `docs/PROJECT_STATE.md` to record the activation.

**Scope:** TWO doc files.

**Files modified:**
- `.task/HANDOVER.md`
- `docs/PROJECT_STATE.md`

**Changes:**

`.task/HANDOVER.md` — append a new section:
```markdown
## A2 Activation Complete (2026-06-23)

Per spec `docs/superpowers/specs/2026-06-23-activate-lightweight-actor-design.md`:
- `USE_LIGHTWEIGHT_ACTOR` flipped from `false` to `true`
- `all_flags_default_off_in_phase_1` test renamed to `flags_phase_appropriate`
- `a1_path.rs` + `coordinator.rs` comments updated
- Integration test `a1_path_routes_through_long_running_skill` added

End-to-end routing verified: TaskTool calls now flow through `CoordinatorHiddenSubagentSkill` instead of legacy `execute_hidden_subagent_phase1/2/3`.

Other 3 const flags remain `false` (USE_ONESHOT_DISPATCHER, USE_ACTOR_IPC, USE_DISPATCHER_IPC).
```

`docs/PROJECT_STATE.md` — under K.2.3 section, change:
```diff
 - **USE_LIGHTWEIGHT_ACTOR** 仍 `false`（默认），所有现有路径无行为变更
 + **USE_LIGHTWEIGHT_ACTOR** 已激活 (`true`)，从 2026-06-23 起 Task 工具调用走 A2 long-running path
 + 其他 3 个 const flag (`USE_ONESHOT_DISPATCHER`, `USE_ACTOR_IPC`, `USE_DISPATCHER_IPC`) 仍 `false`
```

**Acceptance:**
- Files compile-free (docs)
- `git diff` shows only intended sections changed

**Tests:** none.

**Review focus:**
- HANDOVER and PROJECT_STATE say the same thing
- Activation date is correct (2026-06-23)
- Other 3 flags correctly noted as still off

---

### Task 7: final-verification

**Goal:** Run the plan-compliance-checker on the impl plan and confirm all tasks complete.

**Scope:** No code changes.

**Commands:**
```bash
cd E:/agent-project/northhing
cargo test --workspace --lib 2>&1 | tail -20
bash scripts/regression-test-desktop.sh
cargo run -p plan-compliance-checker -- docs/plans/2026-06-23-activate-lightweight-actor-impl.md --format json
```

**Acceptance:**
- All 19 packages compile
- All non-pre-existing test failures are absent
- Plan compliance checker reports 100% compliance for the 6 tasks

**Tests:** none new.

**Review focus:**
- Verify regression script returns 8/8 PASS
- Verify no test count regressed (count agent-runtime tests, agent-dispatch tests, core tests before vs after)

---

## 2. LAEP Archive Setup

Before T1 starts, the coding agent creates the archive directory:

```
.task/archive/activate-lightweight-actor/
├── change-log.json
├── review-guide.md
└── verification-report.json
```

Each task's `change-log.json` records:
- Task ID
- Commit hash
- Files modified
- Acceptance result

`review-guide.md` is a human-reviewer-focused checklist (separate from this execution-focused plan).

`verification-report.json` records the final state of the activation.

---

## 3. Dependency Graph

```
T1 → T2 → T5
  ↓    ↓
  T3 → T4 → T6 → T7
```

T1 and T2 are sequential (test update follows flag flip).
T3, T4 are parallel doc updates (no dependency on T1/T2).
T5 depends on T2 (test must pass after phase-flag update).
T6 depends on T5 + T1 + T2 (HANDOVER records all changes).
T7 depends on all (final verification).

---

## 4. Rollback Plan

If T7 fails or any task produces unexpected regressions:

1. Revert T1: flip `USE_LIGHTWEIGHT_ACTOR = false`
2. Revert T2: rename test back to `all_flags_default_off_in_phase_1` + assertions
3. Revert T3, T4: doc comments
4. Revert T5: delete the integration test file
5. Revert T6: revert HANDOVER + PROJECT_STATE changes
6. Each task's commit is atomic and revertable independently

The const-flag pattern guarantees single-line rollback.

---

## 5. Success Definition

This plan is successful when:
- `USE_LIGHTWEIGHT_ACTOR = true` is committed
- All 6 LAEP task review-guides pass human review
- Regression script returns 8/8 PASS
- No behavior change for end users (subagents still produce the same `SubagentResult`)
- A3 RoundExecutor investigation can begin (separate spec)

---

**Last updated:** 2026-06-23
**Plan owner:** Coding agent (LAEP)
**Reviewer:** Human reviewer (per-task review-guide.md)