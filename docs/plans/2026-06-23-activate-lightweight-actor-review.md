# A2 Activation — Review Guide (Human Reviewer)

> **Audience:** Human reviewer (YOU)
> **Status:** Awaiting human review of the 7 LAEP tasks
> **Spec:** `docs/superpowers/specs/2026-06-23-activate-lightweight-actor-design.md`
> **Implementation plan:** `docs/plans/2026-06-23-activate-lightweight-actor-impl.md`

---

## 0. How to Use This Guide

This is a **human-reviewer-focused** document. The execution plan (separate file) is what the coding agent follows. Your job is to:

1. **Verify each task's change against this guide's checklist**
2. **Run the acceptance commands** listed per task
3. **Approve or reject** each task's commit before the next task starts

Each task has its own section below.

---

## 1. Review Strategy

### High-impact items (read carefully)
- **T1 — flag flip**: the single line change. Verify no collateral damage (no other flag touched, no logic change).
- **T5 — integration test**: if this test cannot be implemented cleanly (mock setup too brittle), it should be `#[ignore]`-d with documented reason.
- **T7 — final verification**: this is the canary. If it fails, the whole plan is at risk.

### Low-risk items (skim)
- **T2, T3, T4**: test rename + comment updates. Low risk, easy to revert.
- **T6**: doc updates. No code risk.

### Pre-existing issues to NOT block on
- 37 pre-existing test build errors in `coordinator.rs` K.2.2 boundary tests (verified independent via `git stash`).
- `cargo clippy -D warnings` errors in unrelated packages (deep_review, etc.).

---

## 2. Per-Task Checklists

### T1: flip-const-flag

**Files to inspect:** `src/crates/execution/agent-dispatch/src/flags.rs`

- [ ] ONLY line 15 changed: `USE_LIGHTWEIGHT_ACTOR` from `false` to `true`
- [ ] No other flag (lines 20, 25, 30) was touched
- [ ] Comment on lines 11-14 is still accurate (talks about "Flip to true only after Phase 2 passes integration" — that's now true)

**Acceptance commands:**
```bash
git diff src/crates/execution/agent-dispatch/src/flags.rs
# Expect: only line 15 changed, single -/+
cargo check -p northhing-agent-dispatch 2>&1 | tail -5
# Expect: 0 errors
```

**Block on:**
- Flag flip is not just for `USE_LIGHTWEIGHT_ACTOR` (others also flipped accidentally)
- Cargo build fails for reasons unrelated to the flag

---

### T2: phase-flag-test-update

**Files to inspect:** `src/crates/execution/agent-dispatch/src/flags.rs` (test module, bottom)

- [ ] Test renamed from `all_flags_default_off_in_phase_1` to `flags_phase_appropriate`
- [ ] New assertion `assert!(USE_LIGHTWEIGHT_ACTOR)` (positive)
- [ ] Other 3 flags still asserted as `!flag` (negative)
- [ ] Doc comment references the activation spec

**Acceptance commands:**
```bash
cargo test -p northhing-agent-dispatch --lib flags 2>&1 | tail -5
# Expect: 1 passed; 0 failed
cargo test -p northhing-agent-dispatch --lib 2>&1 | tail -5
# Expect: 24 passed; 0 failed (or same count as before T1)
```

**Block on:**
- Other agent-dispatch tests regressed (count < 24)
- Test renamed but not updated (still has `!USE_LIGHTWEIGHT_ACTOR`)

---

### T3: a1-path-doc-update

**Files to inspect:** `src/crates/assembly/core/src/agentic/coordination/a1_path.rs`

- [ ] Only lines 41-43 (doc comment) changed
- [ ] References the new spec file
- [ ] Date 2026-06-23 is correct
- [ ] Code (struct definition, `tick`, mapping functions) untouched

**Acceptance commands:**
```bash
git diff src/crates/assembly/core/src/agentic/coordination/a1_path.rs
# Expect: only doc comment block, +/- a few lines
cargo check -p northhing-core 2>&1 | tail -5
# Expect: 0 errors (excluding the 37 pre-existing)
```

**Block on:**
- Any code change
- Spec reference path wrong

---

### T4: coordinator-comment-update

**Files to inspect:** `src/crates/assembly/core/src/agentic/coordination/coordinator.rs`

- [ ] Only lines 4278-4282 (comment block) changed
- [ ] Logic at line 4282 (`if USE_LIGHTWEIGHT_ACTOR {`) is UNTOUCHED
- [ ] Gate body (lines 4283-4292) is UNTOUCHED

**Acceptance commands:**
```bash
git diff src/crates/assembly/core/src/agentic/coordination/coordinator.rs
# Expect: only comment block diff
cargo check -p northhing-core 2>&1 | tail -5
# Expect: 0 errors
```

**Block on:**
- Any code change (gate logic, surrounding functions)

---

### T5: a1-path-integration-test

**Files to inspect:** `src/crates/assembly/core/tests/a1_path_integration.rs` (NEW)

- [ ] Test file compiles: `cargo test --no-run -p northhing-core --test a1_path_integration`
- [ ] Test passes when run: `cargo test -p northhing-core --test a1_path_integration`
  - OR test is `#[ignore]`-d with documented reason
- [ ] No production code modified

**Acceptance commands:**
```bash
cargo test -p northhing-core --test a1_path_integration 2>&1 | tail -10
# Expect: 1 passed OR 1 ignored (with reason)
```

**Block on:**
- Test modifies production code (must be `tests/` directory)
- Test cannot be implemented AND not marked `#[ignore]`

---

### T6: handover-and-state-update

**Files to inspect:** `.task/HANDOVER.md`, `docs/PROJECT_STATE.md`

- [ ] HANDOVER has new "A2 Activation Complete (2026-06-23)" section
- [ ] HANDOVER mentions all 6 tasks completed
- [ ] HANDOVER notes other 3 flags still false
- [ ] PROJECT_STATE K.2.3 section says "已激活" not "仍 false"
- [ ] PROJECT_STATE lists other 3 flags as still off

**Acceptance commands:**
```bash
grep -n "USE_LIGHTWEIGHT_ACTOR" .task/HANDOVER.md docs/PROJECT_STATE.md
# Expect: matches showing activation status
git diff docs/PROJECT_STATE.md | head -30
# Expect: small targeted change under K.2.3 section
```

**Block on:**
- HANDOVER and PROJECT_STATE contradict each other
- Activation date is wrong
- Doc claims behavior change beyond flag flip

---

### T7: final-verification

**Files to inspect:** None (verification only)

- [ ] `cargo test --workspace --lib` returns expected counts (not regressed)
- [ ] `bash scripts/regression-test-desktop.sh` returns 8/8 PASS
- [ ] Plan compliance checker reports 100% for the 6 tasks

**Acceptance commands:**
```bash
cd E:/agent-project/northhing
cargo test --workspace --lib 2>&1 | tail -10
bash scripts/regression-test-desktop.sh 2>&1 | tail -10
cargo run -p plan-compliance-checker -- docs/plans/2026-06-23-activate-lightweight-actor-impl.md --format json
```

**Block on:**
- Any package fails to compile (other than pre-existing)
- Regression script reports <8/8 PASS
- Plan compliance checker reports non-compliance for any task

---

## 3. Cross-Cutting Review Notes

### Const-flag discipline
Per `agents/reference/actor/06-const-flag-usage.md`:
- Flag flips must be paired with regression test
- `PROJECT_STATE.md` must be updated
- Rollback is one-line

Verify T1 + T2 + T6 all satisfy this triad.

### Pre-existing issues
- 37 test build errors in `coordinator.rs` boundary tests — known, NOT introduced by this plan
- Clippy errors in unrelated packages (deep_review, etc.) — known, NOT introduced
- These should NOT block T7 if T7 reports the same count

### Out-of-scope verifications
- DO NOT verify A3 RoundExecutor refactor (separate spec)
- DO NOT verify IPC adapter implementations (USE_ACTOR_IPC still false)
- DO NOT verify production telemetry dashboard (not in scope)

---

## 4. Decision Tree

After all 7 tasks:

| If T7 reports... | Then... |
|------------------|---------|
| 8/8 regression PASS, all tests pass | ✅ APPROVE — proceed to A3 investigation |
| Regression <8/8 but known pre-existing | ⚠️ CONDITIONAL APPROVE — note pre-existing + accept |
| New test failure introduced | ❌ REJECT — find the regression, fix, re-run T7 |
| Compilation error in any non-pre-existing file | ❌ REJECT — fix before approval |

---

## 5. Post-Approval Actions

After all 7 tasks approved:
1. Commit the `.task/HANDOVER.md` changes (or include in T6)
2. Update `docs/PROJECT_STATE.md` "Next session suggestions" section to point to A3 investigation
3. Begin A3 RoundExecutor investigation (separate spec, separate plan)

---

**Last updated:** 2026-06-23
**Reviewer:** Human reviewer (you)
**Decision:** Pending