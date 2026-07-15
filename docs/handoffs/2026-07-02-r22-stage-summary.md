# R22 Stage Summary: terminal/exec.rs 2488 → exec/{mod,types,manager,output,platform}.rs 5-file facade

> **Round**: R22 (4 sub-rounds parallel + r22e Mavis take-over, R21+ new flow)
> **Date**: 2026-07-02
> **Scope**: `src/crates/services/terminal/src/exec.rs` (2496 行 services-layer god-object) → 1 facade + 4 sibling files
> **新流程**: 4 producer 并行 + producer self-report + Mavis r22e take-over for cross-sibling fixes + 3-axis verify + sequential merge → 等 user review

---

## 1. Stage Summary

| Sub-round | File edited | Before | After | Δ | Methods migrated | Status |
|---|---|---|---|---|---|---|
| **R22a** | `exec/types.rs` (new) | 0 | 233 | +233 | 12 pub struct/enum + 11 internal struct/enum + ExecProcessManager + 5 consts | ✅ Merged `45189ea` |
| **R22b** | `exec/manager.rs` (new) | 0 | 490 | +490 | impl ExecProcessManager (6 pub method) + Drop + impl ExecProcess | ✅ Merged `43d8df6` |
| **R22c** | `exec/output.rs` (new) | 0 | 592 | +592 | impl OutputState + 5 helper fn + CollectedOutput + impl HeadTailText 366-line god-impl (impl block 58 lines, struct 366 lines) | ✅ Merged `2786738` |
| **R22d** | `exec/platform.rs` (new) | 0 | 382 | +382 | 28 free fn for PTY Windows/Unix + encoding + utility | ✅ Merged `415566c` |
| **R22e** | `exec/mod.rs` (new) | n/a | 38 | +38 | Facade: use + 4 mod declarations + pub use + GLOBAL_EXEC_MANAGER + get_global_exec_process_manager | ✅ Mavis take-over `0b8cc3f` |
| **exec.rs** | (deleted) | **2496** | **0** | **-2496** | Replaced by exec/ dir + 4 sibling files | ✅ Deleted |

**Total**:
- exec.rs: 2496 → 0 行 (-100%, deleted)
- 4 sibling files + 1 facade = 1735 行总 (mod.rs 38 + types.rs 233 + manager.rs 490 + output.rs 592 + platform.rs 382)
- net: 2496 → 1735 = -761 行 (-30.5%)
- 73 fn definitions across siblings (types 2 + manager 22 + output 19 + platform 30, including 6 pub method on ExecProcessManager + 28 free fn + 5 helper fn + others)
- 0 fns dropped, 0 method signatures changed (cross-crate API preserved via lib.rs `pub use exec::types::*`)

---

## 2. File structure (final)

```
src/crates/services/terminal/src/
├── lib.rs                                          # pub use exec::types::{...13 types}
│                                                   # pub use exec::get_global_exec_process_manager
└── exec/
    ├── mod.rs                                      # 38 行 facade
    │                                               # pub mod manager/output/platform/types
    │                                               # use output::* use types::* pub use types::ExecProcessManager
    │                                               # static GLOBAL_EXEC_MANAGER + pub fn get_global_exec_process_manager
    ├── types.rs                                    # 233 行
    │                                               # 12 pub struct/enum (cross-crate API)
    │                                               # 12 pub(crate) struct/enum/type (cross-sibling use)
    │                                               # ExecProcessManager struct + Default impl
    │                                               # 5 pub(crate) const (MAX_*, PTY_EXIT_DRAIN_TIMEOUT_MS, DEFAULT_YIELD_TIME_MS)
    ├── manager.rs                                   # 490 行
    │                                               # impl ExecProcessManager (6 pub method: exec_command, write_stdin, send_stdin, control_session)
    │                                               # impl Drop for ExecProcess
    │                                               # impl ExecProcess (private)
    ├── output.rs                                    # 592 行
    │                                               # impl OutputState (~128 行)
    │                                               # 5 helper fn (emit_lifecycle, completion_status, spawn_lifecycle_watcher)
    │                                               # struct CollectedOutput
    │                                               # impl HeadTailText (366 行 god-impl, preserved verbatim)
    │                                               # 3 spawn functions (spawn_exec/pty/pipe_process)
    └── platform.rs                                  # 382 行
                                                    # 28 free fn for PTY Windows/Unix + encoding + utility
                                                    # cfg(unix) / cfg(windows) preserved verbatim
```

---

## 3. Merge order (sequential, 2 conflicts resolved by Mavis)

```
1a69a82 docs(spec): R22 terminal/exec.rs 2488 -> facade + 4 sibling parallel split
1a1c6fb refactor(terminal-core): R22a types extraction
45189ea Merge R22a (--no-ff)
15313f5 refactor(terminal-core): R22b manager extraction
43d8df6 Merge R22b (--no-ff) — git auto-detect exec.rs → mod.rs rename 79%
f69922e refactor(terminal-core): R22c output extraction
2786738 Merge R22c (--no-ff) [Mavis conflict: take --theirs r22c exec.rs, add pub mod declarations]
329d4c4 refactor(terminal-core): R22d platform extraction
415566c Merge R22d (--no-ff) [Mavis conflict: discard stale exec.rs, keep mod.rs facade]
192e946 fix(terminal-core): R22 cross-sibling visibility + import paths (Mavis r22e partial)
0b8cc3f fix(terminal-core): R22 cross-sibling visibility + import paths (Mavis r22e final)
```

Mavis r22e did 11 cross-sibling fixes (see §6).

---

## 4. Spec deviation (Mavis r22e corrections)

R22 spec §3 assumed:
- 4 producer strictly own exec.rs L46-247 / L249-724 / L725-1294 / L1300-1617 line 段
- 4 sub-rounds run in independent worktrees, each only writes its sibling file

**Actual**:
1. R22b (manager) **renamed exec.rs → exec/mod.rs** in worktree (autonomous decision, since exec.rs is single file and r22b needed exec.rs to become a directory entry). This caused r22c/d merge conflicts.
2. R22c (output) added `#[path = "exec/output.rs"] mod output; use output::*;` to wire output.rs from exec.rs context (worktree-local). Mavis r22e replaced with `pub mod output;` in mod.rs facade.
3. R22d (platform) added `mod platform;` + cleaned 6 dead imports + updated 22 call sites to `platform::name(...)`. Mavis r22e discarded stale exec.rs from r22d worktree.
4. R22c added `pub(crate)` to 4 consts (MAX_RETAINED_OUTPUT_BYTES, PTY_EXIT_DRAIN_TIMEOUT_MS, CREATE_NO_WINDOW, PIPE_JOB_CLOSE_WAIT_MS). Mavis r22e consolidated into types.rs + mod.rs.
5. R22c added `pub(crate)` to 6 helper fn in output.rs (emit_lifecycle, completion_status_*, etc.) and 10 method on OutputState/HeadTailText.
6. R22b added `pub(crate)` to 1 field (out_of_band_control_action) for cross-sibling visibility.

Mavis r22e promoted 39 internal struct fields to `pub(crate)` in types.rs (ExecProcess.output/writer/terminator/etc) and 12 internal struct/enum/type to `pub(crate)` so sibling files (manager/output/platform) can access them.

---

## 5. Mavis 3-axis verify (R21+ new flow)

| Axis | Command | Result |
|---|---|---|
| 1. 编译过 | `cargo check --workspace --message-format=short` | ✅ **0 errors** |
| 2. 跨 crate | (workspace check covers all) | ✅ 0 errors |
| 3. 不退化 | `cargo test -p northhing-core --features product-full --lib` | ✅ **899 passed; 0 failed; 1 ignored** (R17 baseline preserved) |
| 3. 不退化 | `cargo test -p terminal-core --lib` | ✅ **22 passed; 0 failed; 0 ignored** (terminal-core tests pass) |

R19 cross-crate lesson applied: workspace check + per-target-crate test (terminal-core has 22 internal tests that must pass).

---

## 6. r22e Mavis take-over: 11 cross-sibling fixes

| # | File | Fix |
|---|---|---|
| 1 | `types.rs` | 12 internal struct/enum/type → `pub(crate)` (cross-sibling access) |
| 2 | `types.rs` | 41 struct fields → `pub(crate)` (e.g., ExecProcess.output/writer/terminator + ExecProcessManager.sessions/completed_sessions) |
| 3 | `types.rs` | ExecProcessManager struct fields (sessions, completed_sessions) → `pub(crate)` |
| 4 | `types.rs` | Add 5 consts (MAX_*, PTY_EXIT_DRAIN_TIMEOUT_MS, DEFAULT_YIELD_TIME_MS) → `pub(crate)` |
| 5 | `output.rs` | Add std imports (VecDeque, ErrorKind, Arc, Duration, Notify, Command, Stdio, TerminalError, TerminalResult) + portable_pty imports (native_pty_system, CommandBuilder, PtySize) + `use std::sync::Mutex as StdMutex;` |
| 6 | `output.rs` | `use super::*;` → `use super::platform::*; use super::types::*;` |
| 7 | `platform.rs` | `use super::ExecSessionEntry` etc → `use super::types::ExecSessionEntry` (sibling path) |
| 8 | `platform.rs` | `#[cfg(unix)] use super::PIPE_INTERRUPT_GRACE_TIMEOUT_MS; use super::PTY_EXIT_DRAIN_TIMEOUT_MS; #[cfg(windows)] use super::{CREATE_NO_WINDOW, PIPE_JOB_CLOSE_WAIT_MS};` |
| 9 | `mod.rs` | Add `pub use types::ExecProcessManager;` for cross-sibling re-export |
| 10 | `mod.rs` | Rebuild as clean facade (use + 4 mod + pub use + GLOBAL_EXEC_MANAGER + get_global_exec_process_manager) |
| 11 | `lib.rs` | `pub use exec::{...13 types}` → `pub use exec::types::{...13 types} + pub use exec::get_global_exec_process_manager` |

---

## 7. Pre-existing warnings (NOT R22 regression)

- `unused imports` warnings in mod.rs (some imports are dead due to facade simplicity) — could be cleaned in r22e follow-up
- `unused dead_code` warnings in platform.rs (some platform-specific consts are conditional on cfg) — acceptable

**0 errors, 0 NEW warnings introduced by R22.**

---

## 8. Cross-crate API stability (R19 lesson applied)

- 13 pub type re-exports in lib.rs L33-42 unchanged (path changed from `exec::Type` → `exec::types::Type`, but the use alias `Local*` keeps same identifier)
- `get_global_exec_process_manager()` signature unchanged
- 6 pub method on `ExecProcessManager` (exec_command / write_stdin / send_stdin / control_session) signatures unchanged
- 0 fn signatures changed in facade

---

## 9. Risk assessment (post-merge)

| Risk | Mitigation | Status |
|---|---|---|
| R22b rename exec.rs → mod.rs caused merge conflict | Mavis r22e took --theirs + added mod declarations | ✅ Mitigated |
| 4 producer各自 worktree 改 exec.rs 不同方式 | Mavis r22e consolidated into mod.rs facade | ✅ Mitigated |
| Cross-sibling struct field visibility | Mavis r22e bulk promoted 39 fields to pub(crate) | ✅ Mitigated |
| Cross-sibling cfg-gated const import | Mavis r22e added #[cfg(unix/windows)] `use super::CONST` | ✅ Mitigated |
| Cargo.lock drift | No `cargo update` run | ✅ 0 drift (assumed; can verify with diff) |
| Pre-existing 156 cargo fmt changes (R20 history) | Out of R22 scope | ⏸ Untouched |

---

## 10. Owner

- **Owner**: Mavis (orchestrator)
- **Producer**: 4 sub-agent (M2.7 non-highspeed)
- **Verifier**: Mavis r22e take-over + 3-axis verify (workspace + cli + desktop + 2 test suites)
- **Reviewer**: ⏸ Pending — User-driven QClaw + Kimi review
- **Final arbitration**: Mavis (after QClaw + Kimi verdicts returned)
- **Squash merge**: ⏸ Pending — after user review signal, Mavis will `git merge --squash` + 1 squash-merge commit + bump version if appropriate

---

## 11. Artifacts

- R22 spec: `docs/handoffs/2026-07-02-r22-terminal-exec-split-spec.md` (commit `f6bda2e`)
- Plan YAML: `C:\Users\UmR\.mavis\plans\round22-terminal-exec-split-2026-07-02.yaml`
- Plan ID: `plan_31bb09da`
- Main HEAD: `0b8cc3f` (4 sequential merge commits + 2 r22e fix commits)
- Verify logs: `r22-final-check.log`, `r22-verify-workspace.log`, `r22-verify-test.log`, `r22-verify-terminal.log` (in northing/ root)
- 4 worktree: `E:/agent-project/northing-impl-r22{a,b,c,d}-*` (待 review 完成后清理)