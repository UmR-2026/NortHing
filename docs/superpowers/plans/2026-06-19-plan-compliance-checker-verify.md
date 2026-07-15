<!-- LEGACY: 本文档是 v0.1.0 之前的历史计划，保留原 `agent-app` 名称作历史参考。
 Northing / 纳森 是 agent-app 的继任者（v0.1.0 之后改名）。
 本文件内容不被后续产品名替换脚本覆盖，保留 plan 当时的命名语境。 -->

# Plan Compliance Checker — Verification & Acceptance Protocol

> **For reviewers and executors:** This document defines when the
> `plan-compliance-checker` plan at `docs/superpowers/plans/2026-06-19-plan-compliance-checker-impl.md`
> is considered "done". It complements the per-task `Run: ... Expected: ...`
> steps in the plan with **per-phase** and **final-acceptance** gates.
>
> The plan is NOT done when the last task is committed.
> The plan IS done when all gates in this document pass.

---

## Why this exists

The implementation plan has per-task verification (every step has a `Run:` command and an `Expected:` outcome). But weak models can:
- Pass per-task steps but skip the cross-cutting work (the actor plan path correction in Task 4.4)
- Commit broken code by misreading "PASS" as "EXIT 0 only"
- Forget to run the tool against its first-use case (the actor plan bug)
- Mark plan done without producing the deliverable (the tool binary itself working on real input)

This protocol is the **acceptance gate** that catches those failures.

---

## Per-phase gates

After every phase commit, the executor must run the per-phase gate before proceeding to the next phase.

### Phase 1 gate (skeleton)

**Pass criteria:**
- [ ] `cargo build -p plan-compliance-checker` exits 0
- [ ] `cargo test -p plan-compliance-checker` shows — 7 tests passing (2 CLI + 3 plan struct + 2 path resolver)
- [ ] `cargo run -p plan-compliance-checker -- --help` prints usage including `--task`, `--skip-slow`, `--start-sha`, `--format`
- [ ] Running `cargo run -p plan-compliance-checker` with no arguments exits non-zero (clap rejects missing plan path)

**Fail behavior:** Fix before Phase 2. Do not proceed with failing tests.

### Phase 2 gate (parser)

**Pass criteria:**
- [ ] `cargo test -p plan-compliance-checker` shows — 11 tests passing (Phase 1's 7 + 4 parser)
- [ ] The parser correctly extracts: task ID from `### Task N.M:` heading, task title, files in `Create:` / `Modify:` blocks, step indices, `Run:` verify commands, `Expected:` outcomes
- [ ] The parser does NOT panic on the actor plan (`docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md`) — even if the check is partial

**Fail behavior:** Fix before Phase 3.

### Phase 3 gate (checker + report)

**Pass criteria:**
- [ ] `cargo test -p plan-compliance-checker` shows — 15 tests passing (Phase 2's 11 + 4 checker)
- [ ] The checker produces human-readable output for at least 3 fixture plans: one passes, one fails (path mismatch), one is pending (no commits)
- [ ] The checker exits 0 when all tasks pass or are pending; exits 1 when any task fails
- [ ] `--format json` produces valid JSON with the schema `{plan, plan_start_sha, tasks[], summary}`
- [ ] The checker correctly identifies the actor plan's path-mismatch bug (the `crates/agent-dispatch/...` should suggest `src/crates/execution/agent-dispatch/...`)

**Fail behavior:** Fix before Phase 4.

### Phase 4 gate (fixtures + actor plan correction)

**Pass criteria:**
- [ ] All 4 fixture plans in `tools/plan-compliance-checker/tests/fixtures/` parse cleanly
- [ ] `cargo run -p plan-compliance-checker -- tools/plan-compliance-checker/tests/fixtures/good-plan.md` exits 0
- [ ] `cargo run -p plan-compliance-checker -- tools/plan-compliance-checker/tests/fixtures/path-mismatch-plan.md` exits 1 with path-consistency warning
- [ ] `cargo run -p plan-compliance-checker -- tools/plan-compliance-checker/tests/fixtures/missing-file-plan.md` exits 1
- [ ] The actor plan path correction step (Task 4.4) was applied: `crates/agent-dispatch/` no longer appears in `docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md`
- [ ] Re-running the checker against the actor plan shows fewer or zero path-consistency warnings (compared to the pre-correction run)

**Fail behavior:** Fix before final acceptance.

---

## Final acceptance criteria

The plan is **DONE** only when ALL of the following are true:

### Code quality

- [ ] All 19 plan tasks committed on `v3-restructure` branch
- [ ] `cargo build --workspace --all-features` exits 0
- [ ] `cargo test --workspace --all-features` exits 0
- [ ] `cargo test -p plan-compliance-checker` shows — 22 tests passing (4 phases × ~5 tests each + 4 fixture tests)
- [ ] No new compiler warnings introduced by the plan (warnings count — pre-plan baseline)
- [ ] No `unsafe` blocks in `tools/plan-compliance-checker/src/` (excluding `#[allow(unsafe_code)]` if any)

### Deliverable: working binary

- [ ] `cargo run -p plan-compliance-checker -- <any-plan>.md` does NOT panic on real agent-app-v3 plan markdown
- [ ] `cargo run -p plan-compliance-checker -- docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md` runs to completion
- [ ] Output matches the schema in `docs/superpowers/specs/2026-06-19-plan-compliance-checker-design.md` §Output Format (human + JSON)
- [ ] Exit code follows the rule: 0 if all pass/pending, 1 if any fail, 2 if parse error

### Documentation

- [ ] `tools/plan-compliance-checker/README.md` exists and explains usage
- [ ] `docs/notes/plan-compliance-checker.md` exists and explains how to extend
- [ ] `docs/superpowers/specs/2026-06-19-plan-compliance-checker-design.md` matches implementation (no drift)
- [ ] `docs/PROJECT_STATE.md` "🔧 Plan Compliance Checker" section is up to date with all 4 phases marked complete
- [ ] `docs/HANDOFF_NEXT_SESSION.md` references the working tool

### Rollback readiness

- [ ] Every behavioral change in the plan is shipped behind a const flag (the checker itself doesn't have behavior flags because it has no prior behavior; the actor plan correction is itself a doc change and is trivially reversible)
- [ ] `git revert <last-phase-tag>` would restore the pre-plan state without breaking the rest of the workspace

### First-use case verification

- [ ] The actor plan's path-mismatch bug was detected by the checker (Task 4.4 Step 1 captured the pre-correction output)
- [ ] The actor plan was corrected using `sed` or equivalent (Task 4.4 Step 2)
- [ ] The post-correction run shows fewer path-consistency warnings (Task 4.4 Step 3)

### Cross-crate consistency

- [ ] `cargo check --workspace` still succeeds (the new crate does not break sibling crates)
- [ ] The 821+ v3 tests still pass (the new crate does not run them but does not interfere)

---

## Failure modes the protocol catches

| Failure | Where caught |
|---|---|
| Per-task passes but cross-phase verification skipped | Per-phase gate at the end of each phase |
| Output format drifts from spec | Final acceptance "Deliverable" section |
| First-use case (actor plan bug) not exercised | Final acceptance "First-use case verification" section |
| README or maintainer's note forgotten | Final acceptance "Documentation" section |
| Tag not applied | Final acceptance requires `git tag v0.1.0-checker` |
| Workspace regressed | Final acceptance "Cross-crate consistency" section |

---

## Sign-off

The plan is considered DONE when the executor (human or model) signs off here:

```
I have completed all 19 tasks in docs/superpowers/plans/2026-06-19-plan-compliance-checker-impl.md
and verified all per-phase gates and final acceptance criteria above.

Tool works on real input (the actor plan).
Tag v0.1.0-checker applied.
Ready to ship.

Date: __________
Executor: __________
```

This sign-off block is the **only** authoritative completion signal. Without it, the plan is **not done** even if all commits are pushed.
