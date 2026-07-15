# R1 Shell-exec Sandbox — Implementation Plan

> **Status:** Implementation Plan — Ready for LAEP Execution
> **Date:** 2026-06-23
> **Spec:** `docs/superpowers/specs/2026-06-23-r1-shell-exec-sandbox-design.md`

---

## 0. Plan Overview

3-phase implementation. Each phase has 3-5 LAEP tasks. Each task produces 1 commit.

### Phase Map

| Phase | Tasks | Files | Time |
|-------|-------|-------|------|
| **Phase 1** | T1.1, T1.2, T1.3 | audit doc + related | 1-2d |
| **Phase 2** | T2.1-T2.5 | guard function + 8 paths | 1-2d |
| **Phase 3** | T3.1-T3.4 | config + audit log | 0.5-1d |

**Total**: ~3-5 days

---

## Phase 1 — Audit Pass

### T1.1: Write audit template + scaffold
- Create `docs/security/` directory
- Write `docs/security/r1-shell-exec-audit.md` with empty sections for each path

### T1.2: Audit each path (LLM-friendly)
For each of the 9 paths:
- Read source code
- Document: access control, test coverage, risk level, fix recommendation
- Commit per-path audit entries

### T1.3: Audit summary + priorities
- Aggregate findings
- Rank by priority (P0/P1/P2)
- Output final audit doc

---

## Phase 2 — Denylist Extension + Guard Function

### T2.1: Extract `guard_command_execution()` skeleton
- Add `GuardOutcome` enum to `shell_safety.rs`
- Add `guard_command_execution()` stub (async signature, returns Ok(Allowed) for now)
- Add unit tests for stub

### T2.2: Implement denylist branch in guard
- Add sync denylist check (calls `check_command_denied`)
- Add audit log call (Phase 3 dependency)
- Unit tests for DeniedByDenylist path

### T2.3: Implement confirmation branch in guard
- Read context's `skip_tool_confirmation`
- Async confirmation gate via existing `request_user_confirmation`
- Handle all 4 outcomes: Confirmed, Rejected, Timeout, ChannelClosed
- Unit tests for each outcome

### T2.4: Wire guard into 8 un-audited paths
For each path:
- Locate `Command::new()` call
- Add `guard_command_execution()` before
- Add unit test (mock or real)

### T2.5: Phase 2 verification
- Run all existing tests
- Verify no regression in BashTool denylist behavior

---

## Phase 3 — Mode-based Confirmation + Audit Log

### T3.1: Add `ConfirmationMode` enum + `ShellSecurityConfig`
- Add to `service/config/types.rs`
- Update `AIConfig` to include `ShellSecurityConfig`
- Add default mode mapping (8 modes → Permissive)

### T3.2: Wire mode_overrides into round_executor
- Update `round_executor.rs` to read `ShellSecurityConfig.mode_overrides`
- Replace `skip_tool_confirmation` lookup with mode-aware lookup
- Add tests for mode override

### T3.3: Implement audit log writer
- New module `service/audit_log.rs`
- NDJSON format
- File rotation (10MB or 7 days)
- Wire into `guard_command_execution()`

### T3.4: Phase 3 verification + final regression
- Run full workspace test
- Verify `.northhing/audit.log` is created
- Update HANDOVER + PROJECT_STATE

---

## Dependency Graph

```
T1.1 → T1.2 → T1.3 (Phase 1: sequential)
T1.3 → T2.1 → T2.2 → T2.3 (Phase 2: sequential, audit informs priority)
T2.3 → T2.4 → T2.5 (T2.4 can parallelize across 8 paths)
T2.5 → T3.1 → T3.2 → T3.3 → T3.4 (Phase 3: sequential)
```

---

## Rollback Strategy

| Phase | Rollback |
|-------|----------|
| Phase 1 | Doc-only, no rollback needed |
| Phase 2 | `guard_command_execution()` can be made no-op via feature flag |
| Phase 3 | `ShellSecurityConfig` defaults to current behavior; no-op if config missing |

---

## Acceptance Definition

This plan is successful when:
- Phase 1 audit doc exists with all 9 paths
- Phase 2 `guard_command_execution()` is called in all 8+ paths
- Phase 3 `ConfirmationMode` + audit log work end-to-end
- All existing tests pass
- `cargo test --workspace --lib` returns 0 failed

---

**Last updated:** 2026-06-23
**Plan owner:** Coding agent (LAEP)
**Reviewer:** Human reviewer (per-phase review-guide.md)