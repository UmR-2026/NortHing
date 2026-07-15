# R20e Decision: Accept `manager_process.rs` 254 lines as-is (5% over cap)

> **Decision:** ACCEPT — no new split needed. `manager_process.rs` 254 canonical lines (+5% over QClaw 242 tolerance) stays as-is, formalizing the QClaw R20a P3 "accept borderline" recommendation.

**Authority:** QClaw R20a P3 explicit recommendation, with Mavis verification.

**Date:** 2026-07-02

---

## 1. Background

QClaw R20a review (8.8/10 APPROVE) recommended for R20e:

> P3 R20e: `manager_process.rs` 254 → accept borderline (5% over). Acceptable as-is.

Kimi R20a review independently confirmed.

R20e is the lightest of the 5 sub-stages in the R20 round. At 5% over the 242 cap, the file is well within the "borderline acceptable" range. No precedent match needed; this is a direct accept per QClaw's explicit recommendation.

---

## 2. File analysis

**`manager_process.rs` (254 canonical wc-l, 4 methods on `AcpClientService`):**

| Method | Span | Visibility |
|---|---|---|
| `new` | L116-130 | `pub fn` (constructor) |
| `connection` | L131-200 | `pub async fn` (sibling caller) |
| `renders_remote_client_command_from_config` | L201-223 | `pub fn` (sibling caller) |
| `resolves_remote_client_config_from_global_config` | L224-end | `pub fn` (sibling caller) |

**Sub-domain coherence:** "Process lifecycle" — the 4 methods form a coherent "spawn process for client" sub-domain. Method 1 (`new`) is the constructor, methods 2-3 are runtime operations, method 4 is configuration resolution. No clean 2-way split divides these methods (any split would force a constructor/runtime split, which doesn't match the sub-domain semantics).

**Caller map (verified):**
- 3 in-crate sibling callers: `manager_config.rs`, `manager_install.rs`, `manager_session.rs` — all call via inherent dispatch on `AcpClientService` (no `use super::manager_process::` from siblings)
- 0 cross-crate callers (verified by `git grep 'manager_process::' -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/'`: 0 hits)

**Iron rules (Kimi Bug 3 protocol re-derive):**
- `git show main:src/crates/interfaces/acp/src/client/manager_process.rs | grep -cE '\bunwrap\(\)'`: 0
- `git show main:src/crates/interfaces/acp/src/client/manager_process.rs | grep -cE '\bexpect\('`: 0
- `git show main:src/crates/interfaces/acp/src/client/manager_process.rs | grep -cE 'panic!|unreachable!'`: 0
- `git show main:src/crates/interfaces/acp/src/client/manager_process.rs | grep -cE 'let _\s*=\s*Result'`: 0

All iron rules PRE=0. No new code in this decision (no POST change).

---

## 3. Decision

**`manager_process.rs` is accepted as-is. No new split, no new sub-domain file, no new worktree.**

This decision:

1. **Closes the QClaw R20a P3 D-deviation recommendation** for R20e (already at "accept borderline")
2. **Is a documentation-only decision** — no code change. The file remains at 254 canonical lines on main
3. **Does NOT require a new worktree or commit** — the decision is recorded here in this spec doc and the R20 stage review prep doc

The accept is unconditional (no precedent match needed — 5% over is well within the "borderline acceptable" range that QClaw acknowledged).

---

## 4. What if the file grows above 280 lines in the future?

The R20e accept is unconditional at 254 lines. If the file grows above 280 canonical wc-l (a ~10% over-cap, the "easy fix" threshold), Mavis would recommend:

- Re-evaluate per QClaw R20a "accept borderline" recommendation
- If 280-300: still accept, but flag for R20e+1 review
- If 300+: split into `manager_process_construct.rs` (new + connection) + `manager_process_resolve.rs` (renders + resolves) — 2-way split matching the constructor/runtime boundary

For now, the file is at 254 lines, well within the unconditional accept range.

---

## 5. Mavis verification (this decision's author)

Mavis verified the R20e accept by:

1. Re-deriving iron rules baseline: 0/0/0/0/0 (Kimi Bug 3 protocol)
2. Cross-crate grep: 0 hits
3. In-crate `use super::manager_process::` from siblings: 0
4. Method signature inspection: 4 methods, all `pub`
5. QClaw R20a explicit "accept borderline" recommendation confirmed

All conditions match. Accept is justified.

---

## 6. References

- QClaw R20a P3 recommendation: `docs/handoffs/2026-07-01-r20a-manager-session-split-review-report.md` Section 10 (R20b+ recommendations, R20e row)
- Kimi R20a confirmation: same row
- R20 stage review prep: `docs/handoffs/2026-07-02-r20a-r20b-r20c-stage-review.md` (R20e covered)

---

*Decision authored by Mavis on 2026-07-02. R20e accepted. No new commit, no new worktree, no code change. The accept is recorded in this spec doc and propagated to the R20 stage review prep doc.*
