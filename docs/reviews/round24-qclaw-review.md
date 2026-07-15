# QClaw R24 Stage Review Report

**Reviewer**: QClaw (independent verification agent)  
**Date**: 2026-07-03  
**Stage**: R24 (Mavis take-over, single commit `7c13624`)  
**Subject**: `session_usage/service.rs` 2458 → facade + 5 sibling files  
**Document under review**: `docs/handoffs/2026-07-02-r24-stage-summary.md`  
**Verdict**: ✅ **APPROVE (9.0/10)**

---

## 1. Executive Summary

R24 splits `session_usage/service.rs` (2460 actual lines, 2458 claimed) into a 1228-line facade + 5 sibling files. This is a **free-fn god-impl** split (top-level `fn` functions, not `impl` block methods) — a different pattern from R23's `impl WorkspaceService` split. Mavis used direct Python extraction scripts instead of producer dispatch (lesson from R23: 4 producers all hit 30-min cap).

**Core result**: 0 errors (workspace + cli), **899/0/1 tests pass** (tests fully green — document incorrectly claims "30+ test errors remaining"), 0 new iron rules violations, 0 mojibake, 0 CRLF, 0 Cargo.lock drift, 0 cross-crate ref leakage, 3 pub fn signatures preserved, 0 R24 file warnings, 0 R24 fmt diffs.

---

## 2. Verification Matrix

### 2.1 Build & Test

| Check | Result | Document Claim | Match |
|---|---|---|---|
| `cargo check --workspace` | 0 errors | 0 errors | ✅ |
| `cargo check -p northhing-core --lib` | 0 errors | 0 errors | ✅ |
| `cargo check -p northhing-core --lib --tests` | **0 errors** | "30+ errors" | ❌ **Document wrong** |
| `cargo test -p northhing-core --lib` | **899 passed; 0 failed; 1 ignored** | "not run (test compile errors)" | ❌ **Document wrong** |
| `cargo check -p northhing-cli` | 0 errors | 0 errors | ✅ |

**Major finding**: Document claims 30+ test compile errors and tests not run. Actual: 0 test errors, 899/0/1 fully pass. The test code was fixed before commit `7c13624`.

### 2.2 File Line Counts

| File | Claim | Actual | Status |
|---|---|---|---|
| service.rs (post) | 1228 | 1228 | ✅ EXACT |
| entry.rs | 130 | 130 | ✅ EXACT |
| snapshot.rs | 181 | 181 | ✅ EXACT |
| breakdowns_core.rs | 434 | 434 | ✅ EXACT |
| breakdowns_extra.rs | 379 | 379 | ✅ EXACT |
| utilities.rs | 229 | 229 | ✅ EXACT |
| mod.rs | — | 29 | (not claimed) |
| service.rs (original) | 2458 | 2460 | ⚠️ +2 |
| **Total (post-R24)** | **2581** | **2581** | ✅ EXACT |

All 6 main files match exactly. Original service.rs 2460 vs claim 2458 (+2).

### 2.3 Iron Rules (Production Code)

| File | unwrap | expect | panic! | unreachable! |
|---|---|---|---|---|
| service.rs (production) | 0 | 0 | 0 | 0 |
| service.rs (test module L52+) | 5 | 19 | 0 | 0 |
| entry.rs | 0 | 0 | 0 | 0 |
| snapshot.rs | 0 | 0 | 0 | 0 |
| breakdowns_core.rs | 0 | 0 | 0 | 0 |
| breakdowns_extra.rs | 0 | 0 | 0 | 0 |
| utilities.rs | 0 | 0 | 0 | 0 |

**0 production iron rules violations.** All 5 unwrap + 19 expect are in `mod tests` (L52+). Original service.rs had same counts (5 unwrap, 19 expect) — **0 new violations**. ✅

### 2.4 Mojibake / CRLF / Long Lines

| File | Mojibake | CRLF | Long lines (>120) | R24-introduced |
|---|---|---|---|---|
| service.rs | 0 | 0 | 0 | 0 |
| entry.rs | 0 | 0 | 1 (L115, 123 chars) | 1 ⚠️ |
| snapshot.rs | 0 | 0 | 0 | 0 |
| breakdowns_core.rs | 0 | 0 | 0 | 0 |
| breakdowns_extra.rs | 0 | 0 | 0 | 0 |
| utilities.rs | 0 | 0 | 0 | 0 |

**Note on "mojibake"**: 8 non-ASCII characters found (6× U+2014 em-dash `—`, 2× U+2192 arrow `→`), all in English comments. These are legitimate Unicode punctuation, NOT mojibake encoding errors. 0 real mojibake (U+9225/U+2982/U+FFFD).

**1 new long line**: entry.rs L115 (123 chars) — `report.files = super::breakdowns_extra::build_file_breakdown(...)`. Original line was `report.files = build_file_breakdown(...)` (shorter, local call). The `super::breakdowns_extra::` prefix added by the split pushed it over 120. Within R18 tolerance.

Document claims "0 long lines added" — **inaccurate** (1 new long line).

### 2.5 Cargo.lock & cargo fmt

| Check | Result |
|---|---|
| Cargo.lock drift (89f4f5d..b732d64) | 0 lines ✅ |
| R24 file fmt diffs | 0 ✅ |
| R24 file cargo warnings | 0 ✅ |

### 2.6 Cross-Crate API Stability

| Check | Result |
|---|---|
| 3 pub fn signatures (generate_session_usage_report, build_session_usage_report_from_turns, build_session_usage_report_from_sources) | All preserved in facade ✅ |
| mod.rs `pub use service::{...}` re-export | 3 API + SessionUsageReportRequest ✅ |
| Sibling module names in external crates | 0 hits ✅ |
| Sibling fn names in external crates | 0 hits (3 `p95_duration_ms` hits are struct field, not fn) ✅ |
| External crate imports (cli) | Only via mod.rs re-export (generate_session_usage_report, render_usage_report_markdown, SessionUsageReportRequest) ✅ |

### 2.7 Method Migration & Visibility

| File | pub fn | pub(super) fn | fn (private) |
|---|---|---|---|
| service.rs (facade) | 3 | 0 | 0 |
| entry.rs | 2 | 0 | ~10 |
| snapshot.rs | 2 | 0 | ~8 |
| breakdowns_core.rs | 15 | 0 | ~5 |
| breakdowns_extra.rs | 13 | 0 | ~3 |
| utilities.rs | 13 | 0 | ~2 |
| **Total** | **48** | **0** | **~28** |

**R24 visibility deviation** (documented): All sibling functions are `pub fn` (not `pub(super) fn`). Document explains: "needed for test access via `use super::super::sibling::*;` glob imports; R23 `pub(super)` doesn't propagate through glob imports". This is a legitimate Rust limitation — `pub(super)` items are NOT accessible through glob imports from sibling modules.

mod.rs declares `pub mod` for all siblings (not `pub(super) mod`). This exposes siblings to external crates in theory, but 0 external refs confirmed.

### 2.8 Cross-Sibling Calls

| Sibling | Cross-sibling calls (`super::sibling::fn`) |
|---|---|
| entry.rs | 14 |
| snapshot.rs | 3 |
| breakdowns_core.rs | 17 |
| breakdowns_extra.rs | 20 |
| utilities.rs | 1 |
| **Total** | **55** |

All cross-sibling calls use explicit `super::sibling::fn_name(...)` pattern. ✅

### 2.9 `let _ =` & `#[allow(dead_code)]`

| Check | Count | Origin |
|---|---|---|
| `let _ =` | 0 | 0 new, 0 pre-existing ✅ |
| `#[allow(dead_code)]` | 0 | 0 new, 0 pre-existing ✅ |

### 2.10 Commit Structure

| Commit | Type | Parents | Note |
|---|---|---|---|
| 2edd6c7 | Spec doc | 1 | Pre-split spec ✅ |
| 7c13624 | Refactor | 1 | Mavis take-over, single direct commit ✅ |
| b732d64 | Handoff doc | 1 | Stage summary ✅ |

Working tree: clean (0 untracked/modified). ✅

---

## 3. Issues Found

### 3.1 Document Claims "30+ Test Errors Remaining" — Actually 0 (Moderate, Documentation)

**Issue**: Document §Tests claims "30+ errors remaining (type mismatches in test code after split)" and §Mavis 3-axis verify Axis 3 claims "30+ errors (test type mismatches)" and Axis 4 claims "not run (test compile errors)".  
**Actual**: `cargo check --tests` = 0 errors, `cargo test` = 899/0/1 fully pass.  
**Impact**: Document significantly understates R24 quality. The test code was fully fixed before commit. This is the most important documentation inaccuracy.  
**Root cause**: Likely the document was written mid-process (before test fixes), and not updated after Mavis fixed the tests.

### 3.2 1 New Long Line in entry.rs (Minor)

**Issue**: entry.rs L115 (123 chars) — `report.files = super::breakdowns_extra::build_file_breakdown(...)`.  
**Document claim**: "0 long lines added".  
**Impact**: Within R18 tolerance (≤5 per round). Trivial.  
**Recommendation**: Could be fixed by line-wrapping.

### 3.3 Original service.rs Line Count (Minor, Documentation)

**Issue**: Document claims 2458 lines; actual is 2460 (+2).  
**Impact**: None — consistent with R22/R23 pattern of small counting differences.

### 3.4 Visibility Deviation: `pub fn` Instead of `pub(super) fn` (Noted, Documented)

**Issue**: All ~45 sibling functions are `pub fn` instead of `pub(super) fn`. mod.rs declares `pub mod` for all siblings.  
**Document claim**: Acknowledged as "R24 deviation from R23 `pub(super)` — needed for test access via glob imports".  
**Impact**: 0 external cross-crate refs (confirmed). The deviation is a legitimate Rust limitation: `pub(super)` items are NOT accessible through `use super::super::sibling::*;` glob imports in test modules.  
**Assessment**: Acceptable design choice for free-fn god-impl splits. Not a bug.

### 3.5 "R19 Lesson Applied" — Skipped Producer Dispatch (Process, Noted)

**Issue**: Document describes skipping producer dispatch entirely (R23 4 producers all hit 30-min cap). Direct Python extraction used instead.  
**Impact**: Mavis take-over completed successfully in ~30 min (22:30→23:00). This is a valid process adaptation.  
**Note**: The R19 standing-rule about `extend-timeout` is moot since no producers were dispatched.

---

## 4. Document Accuracy Assessment

| Claim in document | Verified | Notes |
|---|---|---|
| 6 file line counts (1228/130/181/434/379/229) | ✅ Exact | All 6 match |
| service.rs original 2458 | ⚠️ Approximate | Actual 2460 (+2) |
| Total 2581 | ✅ Exact | |
| 0 errors (workspace) | ✅ Exact | |
| 0 errors (northhing-core lib) | ✅ Exact | |
| "30+ test errors remaining" | ❌ **Wrong** | Actual: 0 test errors |
| "cargo test not run (compile errors)" | ❌ **Wrong** | Actual: 899/0/1 pass |
| 0 errors (northhing-cli) | ✅ Exact | |
| 0 NEW unwrap/panic | ✅ Exact | 5 unwrap + 19 expect all pre-existing in test module |
| 0 BOM/CRLF | ✅ Exact | |
| 0 Cargo.lock drift | ✅ Exact | |
| 0 R24 file fmt diffs | ✅ Exact | |
| 3 pub fn preserved | ✅ Exact | |
| 0 cross-crate ref leakage | ✅ Exact | |
| "0 long lines added" | ⚠️ Inaccurate | 1 new long line (entry.rs L115, 123 chars) |
| Visibility deviation documented | ✅ Acknowledged | `pub fn` instead of `pub(super) fn`, legitimate Rust limitation |
| Cross-sibling call pattern | ✅ Exact | `super::sibling::fn_name()` with 55 calls |
| Mavis take-over, single commit | ✅ Exact | Commit 7c13624, 1 parent |
| `let _ =` 0 new | ✅ Exact | 0 total |
| R19 lesson: skipped producer dispatch | ✅ Acknowledged | Process adaptation |

**Document accuracy**: 15/18 claims fully verified. 3 claims have discrepancies (test errors being the most significant — document understates quality).

---

## 5. Verdict

| Dimension | Score | Notes |
|---|---|---|
| Structural integrity | 9/10 | Clean 6-file facade, 55 cross-sibling calls |
| Iron rules compliance | 10/10 | 0 production violations, all pre-existing |
| Build & test baseline | 10/10 | 899/0/1 fully pass (document understates) |
| Mojibake / encoding | 10/10 | 0 real mojibake |
| Cross-crate API stability | 10/10 | 3 pub fn preserved, 0 leakage |
| Cross-crate ref cleanliness | 10/10 | 0 external refs |
| Merge/commit integrity | 10/10 | Single clean commit |
| Cargo.lock stability | 10/10 | 0 drift |
| cargo fmt / warnings | 10/10 | 0 R24 diffs, 0 R24 warnings |
| Line count compliance | 10/10 | All ≤800 (max 434 in breakdowns_core.rs) |
| Visibility pattern | 8/10 | Documented deviation (pub fn, legitimate) |
| Documentation accuracy | 7/10 | 3/18 claims wrong (test status significantly understated) |
| `let _ =` discipline | 10/10 | 0 total |
| Long lines | 9/10 | 1 new (within tolerance) |

**Overall: 9.0/10 — APPROVE**

---

## 6. Pre-Merge Checklist

- [x] Build: 0 errors (workspace + cli)
- [x] Test: 899/0/1 (fully green — document claim of "30+ errors" is inaccurate)
- [x] Iron rules: 0 production violations
- [x] Mojibake: 0
- [x] CRLF: 0
- [x] Cargo.lock: 0 drift
- [x] cargo fmt: 0 R24 diffs
- [x] cargo warnings: 0 R24 warnings
- [x] Cross-crate API: 3 pub fn preserved
- [x] Cross-crate refs: 0 leakage
- [x] `let _ =`: 0
- [x] `#[allow(dead_code)]`: 0
- [x] Working tree: clean
- [ ] Fix entry.rs L115 long line (optional, trivial)
- [ ] Update document to reflect actual test status (optional)
- [ ] Squash-merge (pending user review signal)

---

*Generated by QClaw independent verification. All checks executed against merged source at commit b732d64.*
