# QClaw R20 Stage Review Report

**Reviewer**: QClaw (independent verification agent)  
**Date**: 2026-07-02  
**Stage**: R20a + R20b + R20c + R20d + R20e (full stage)  
**Subject**: `acp/client/manager.rs` (2519 lines) → split across 5 sub-rounds  
**Verdict**: ✅ **APPROVE (9.2/10)**

---

## 1. Executive Summary

R20 is a five-sub-round split of `acp/client/manager.rs` (2519 lines), the largest God file in the `northhing-acp` crate. The split produces 10 new sibling files across 3 worktrees, plus 2 accept-as-is files (R20d manager_transport.rs 276 lines, R20e manager_process.rs 254 lines).

**Core result**: 0 errors, 51/51 tests pass, 0 iron rules violations in production code, 0 mojibake in source files, 0 cross-crate ref drift. All file line counts within 800-line cap. Workspace-wide `cargo check` passes.

---

## 2. Verification Matrix

### 2.1 Build & Test

| Check | R20a | R20b | R20c | Result |
|---|---|---|---|---|
| `cargo check -p northhing-acp` | 0 errors | 0 errors | 0 errors | ✅ |
| `cargo check -p northhing-cli` | 0 errors | 0 errors | 0 errors | ✅ |
| `cargo check --workspace` | 0 errors | — | 0 errors | ✅ |
| `cargo test -p northhing-acp --lib` | 51 pass | 51 pass | 51 pass | ✅ |
| `cargo test -p northhing-core --lib` | — | — | 899 pass / 0 fail / 1 ignored | ✅ |

### 2.2 File Line Counts

| File | Lines | Cap | Delta | Status |
|---|---|---|---|---|
| **R20a** | | | | |
| `manager_session_lifecycle.rs` | 226 | 800 | — | ✅ |
| `manager_session_read.rs` | 101 | 800 | — | ✅ |
| `manager_session_resolve.rs` | 231 | 800 | — | ✅ |
| **R20b** | | | | |
| `manager_session_helpers_identity.rs` | 75 | 800 | — | ✅ |
| `manager_session_helpers_session_response.rs` | 204 | 800 | — | ✅ |
| `manager_session_helpers_session_state.rs` | 175 | 800 | — | ✅ |
| **R20c** | | | | |
| `manager_config_loading.rs` | 93 | 800 | — | ✅ |
| `manager_config_requirements.rs` | 237 | 800 | — | ✅ |
| `manager_connection_start.rs` | 227 | 800 | — | ✅ |
| `manager_connection_stop.rs` | 69 | 800 | — | ✅ |
| **R20d** (accept-as-is) | | | | |
| `manager_transport.rs` | 276 | 800 | — | ✅ |
| **R20e** (accept-as-is) | | | | |
| `manager_process.rs` | 254 | 800 | — | ✅ |

All files well within 800-line cap. Largest file: 276 lines (R20d). **Zero files exceed cap.**

### 2.3 Iron Rules (Production Code)

| File | unwrap | expect | panic! | unreachable! | let _ = |
|---|---|---|---|---|---|
| R20a (3 files) | 0 | 0 | 0 | 0 | 0 |
| R20b (3 files) | 0 | 0 | 0 | 0 | 0 |
| R20c (4 files) | 0 | 0 | 0 | 0 | 0 |
| R20d | 0 | 0 | 0 | 0 | 0 |
| R20e | 0 | 0 | 0 | 0 | 0 |

**All iron rules: 0 violations in production code.** R20e has 2 `expect()` calls in `#[cfg(test)]` module — not production code, not a violation.

### 2.4 Cross-Crate Refs

| Module pattern | Cross-crate hits | Status |
|---|---|---|
| `manager_session_*` (R20a) | 0 | ✅ |
| `manager_session_helpers_*` (R20b) | 0 | ✅ |
| `manager_config_*`, `manager_connection_*` (R20c) | 0 | ✅ |
| `manager_transport::` (R20d) | 0 (in-crate `super::` refs only) | ✅ |
| `manager_process::` (R20e) | 0 (in-crate `super::` refs only) | ✅ |

### 2.5 Mojibake & CRLF

| Check | R20a | R20b | R20c | R20d | R20e |
|---|---|---|---|---|---|
| Source file mojibake | 0 | 0 | 0 | 0 | 0 |
| CRLF line endings | 0 | 0 | 0 | 0 | 0 |
| Long lines (>120) | 0 | 0 | 0 | 0 | 0 |

### 2.6 Cargo.lock Drift

| Worktree | Cargo.lock diff vs main |
|---|---|
| R20a | 0 lines |
| R20b | 0 lines |
| R20c | 0 lines |

### 2.7 mod.rs Registration

All three worktrees correctly register new sub-modules and remove old parent module declarations:
- R20a: `mod manager_session` → replaced by `manager_session_lifecycle`, `manager_session_read`, `manager_session_resolve`
- R20b: `mod manager_session_helpers` → replaced by 3 `manager_session_helpers_*` modules
- R20c: `mod manager_config` + `mod manager_connection` → replaced by 4 `manager_config_*` + `manager_connection_*` modules

### 2.8 Method Count & Visibility

| Round | Total fn | pub | pub(super) | plain |
|---|---|---|---|---|
| R20a | 7 | 4 | 2 | 1 |
| R20b | 16 | 11 | 0 | 5 |
| R20c | 14 | 14 | 0 | 0 |
| R20d | 6 | 6 | 0 | 0 |
| R20e | 7 (4 impl + 3 standalone) | 7 | 0 | 0 |
| **Total** | **50** | **42** | **2** | **6** |

**Note**: Mavis documentation claims "47 methods" using R20e=4 (impl-block methods only). Actual production fn count is 50 (R20e has 7 including 3 standalone `pub fn` helpers). This is a documentation counting convention difference, not a code issue.

### 2.9 Visibility Fix (get_session)

All three worktrees contain a byte-identical visibility fix commit:
- `pub(crate) fn get_session` → `pub fn get_session`
- Same file: `session_manager_lifecycle.rs:201`
- Same diff in all 3 worktrees

### 2.10 cargo fmt

| Worktree | New-file fmt diffs | Pre-existing diffs | Total |
|---|---|---|---|
| R20a | 2 (mod.rs import sort) | 14 | 16 |
| R20b | 0 | 14 | 14 |
| R20c | 0 | 14 | 14 |
| Main baseline | — | 15 | 15 |

R20a introduces 2 fmt diffs in `mod.rs` (import ordering from module rename). R20b and R20c introduce **zero** new fmt diffs.

---

## 3. Issues Found

### 3.1 Documentation Counting Discrepancy (Minor, Non-Blocking)

**Issue**: Mavis documentation claims "47 methods" but actual production fn count is 50.  
**Root cause**: R20e `manager_process.rs` has 7 production fns (4 in `impl AcpClientConnection` block + 3 standalone `pub fn`), but documentation counts only the 4 impl-block methods.  
**Impact**: None — R20e is accept-as-is (no code change). The 3 standalone fns (`resolve_config_for_client`, `ensure_remote_client_supported`, `render_remote_client_command`, `current_unix_timestamp_ms`) are module-level helpers that were always there.  
**Recommendation**: Update documentation to clarify "4 impl methods + 3 standalone = 7 total" or use consistent counting.

### 3.2 R20a mod.rs fmt drift (Minor, Non-Blocking)

**Issue**: R20a `mod.rs` has 2 `cargo fmt` diffs (import ordering).  
**Root cause**: Module rename from `manager_session` to 3 sub-modules changes import sort order.  
**Impact**: None — code compiles and passes all tests.  
**Recommendation**: Run `cargo fmt` before merge.

### 3.3 R20e expect() in Test Code (Non-Issue, Confirmed Safe)

**Issue**: R20e `manager_process.rs` has 2 `expect()` calls.  
**Finding**: Both at L215 and L244, inside `#[cfg(test)]` module (starts L195).  
**Verdict**: Not a violation — iron rules apply to production code only.

---

## 4. Cross-Crate Ref Investigation (Cleared)

Initial scan found `manager_transport::` and `manager_process::` refs in 4 files. Investigation confirms all are **in-crate sibling callers** (`use super::manager_process::...`) within `src/crates/interfaces/acp/src/client/`, not cross-crate refs. Zero cross-crate refs confirmed.

---

## 5. Verdict

| Dimension | Score |
|---|---|
| Structural integrity | 10/10 |
| Iron rules compliance | 10/10 |
| Build & test baseline | 10/10 |
| Mojibake / encoding | 10/10 |
| Cross-crate ref cleanliness | 10/10 |
| mod.rs registration | 10/10 |
| Method count & visibility | 9/10 (doc counting discrepancy) |
| cargo fmt | 9/10 (R20a 2 new diffs) |
| Cargo.lock stability | 10/10 |
| Documentation accuracy | 8/10 (method count, R20e description) |

**Overall: 9.2/10 — APPROVE**

---

## 6. Pre-Merge Checklist

- [ ] Run `cargo fmt` on R20a worktree (fix 2 mod.rs import sort diffs)
- [ ] Update Mavis documentation: R20e method count 4 → 7 (or clarify "4 impl + 3 standalone")
- [ ] Verify `command_router_tests.rs:323` mojibake (`鈥`→`…`) is fixed (R15残留, not R20)

---

*Generated by QClaw independent verification. All checks executed against worktree source files, not documentation claims.*
