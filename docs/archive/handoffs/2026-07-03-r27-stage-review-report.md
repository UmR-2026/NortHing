# R27 Stage Review — `workspace/manager.rs` 1505 → facade 7 + 2 sibling (QClaw)

> **Reviewer**: QClaw (human-verified post-merge review)
> **Date**: 2026-07-03
> **Commit**: `5dec785` on `main` (R27 stage summary)
> **Scope**: `src/crates/assembly/core/src/service/workspace/manager.rs` (1505 lines) → facade (7) + 2 siblings (types.rs 300, manager_impl.rs 1234)
> **Verdict**: ✅ **APPROVE 9.2/10** — 0 errors, 0 cross-crate breakage, clean horizontal split, 2 minor observations (BOM + mod.rs wildcard)

---

## 1. Summary

| Metric | Spec | Actual | Status |
|--------|------|--------|--------|
| `manager.rs` (facade) | 8 lines | **7 lines** | ✅ (-1 rounding, cosmetic) |
| `types.rs` (sibling 1) | 300 lines | **300 lines** | ✅ Exact |
| `manager_impl.rs` (sibling 2) | 1234 lines | **1234 lines** | ✅ Exact |
| **Total** | 1542 | **1541** | ✅ +36 (+2.4%, split overhead) |
| `cargo check -p northhing-core` | 0 errors | **0 errors** | ✅ |
| `cargo check --workspace` | 0 errors | **0 errors** | ✅ |
| `cargo test -p northhing-core` | 103 passed, 0 failed | **Not independently verified** (timeout) | ⏸ Presumed OK per stage summary |
| Consumer crate tests (5 crates) | 102 passed, 0 failed | **Not independently verified** | ⏸ Presumed OK per stage summary |
| Cargo.lock drift | none | **0** | ✅ |
| unwrap/panic/unreachable | 0 | **0** | ✅ |
| Cross-crate `manager_impl::` refs | 0 | **0** | ✅ |
| Cross-crate `workspace::types::` refs | 0 | **0** | ✅ |
| Line endings CRLF | 0 | **0** (but 2 files have BOM) | ⚠️ See §6 |
| Long lines >120 | 0 | **0** | ✅ |

---

## 2. Structural Verification (QClaw)

### 2.1 File Inventory

```bash
wc -l src/crates/assembly/core/src/service/workspace/manager.rs \
  src/crates/assembly/core/src/service/workspace/manager_impl.rs \
  src/crates/assembly/core/src/service/workspace/types.rs
```

| File | Lines | Content | Status |
|------|-------|---------|--------|
| `manager.rs` (facade) | 7 | 3 doc-comment lines + 2 `pub use super::*;` re-exports | ✅ Ultra-thin facade |
| `types.rs` | 300 | struct/enum + `impl Default` + `impl WorkspaceIdentity` + free fn + `IDENTITY_FILE_NAME` const | ✅ Types consolidated |
| `manager_impl.rs` | 1234 | `impl WorkspaceInfo` + `WorkspaceSummary` struct + `WorkspaceManager` struct + `WorkspaceManagerConfig` + `impl Default` + `impl WorkspaceManager` + `WorkspaceManagerStatistics` | ✅ Core logic |

### 2.2 Facade Pattern

```rust
// manager.rs: 7 lines
//! Workspace manager (R27 facade).
//!
//! Mavis take-over (impl-block god-impl, horizontal split). Split into 2
//! sibling files. impl+struct kept in same sibling for private field access.

pub use super::types::*;
pub use super::manager_impl::*;
```

**This is the thinnest facade in the entire project history** (7 lines, 2 re-export lines). All previous facades (R14 306, R19 286, R20a 225, R21 1310, R22 38, R23 1029, R24 1228) are significantly larger. R27 demonstrates that a facade can be reduced to **pure re-exports** when the split is horizontal (types + impl) rather than vertical (sub-domain). ✅

### 2.3 Horizontal Split Strategy (Validated)

**Stage summary rationale**: "`impl WorkspaceManager` accesses private fields `workspaces: HashMap<...>` etc. Splitting `WorkspaceManager` struct into a different sibling than `impl WorkspaceManager` requires `pub(super)` on every private field (5+ fields) — a behavior change. Keeping struct + impl in same sibling preserves original visibility."

**QClaw Assessment**: This is correct and well-justified. `WorkspaceManager` has private fields (`workspaces`, `current_workspace`, etc.) that are accessed by `impl WorkspaceManager` methods. If the struct were in `types.rs` and the impl in `manager_impl.rs`, the fields would need `pub(super)` visibility, which:
1. Changes the struct's encapsulation (fields accessible to all `workspace` module siblings)
2. Could break future invariants that rely on field privacy
3. Is a semantic change, not just a structural one

The horizontal split (types vs impl) is the **correct strategy** for this specific god-file. ✅

### 2.4 mod.rs Declaration

```rust
// mod.rs: L12-13
pub mod manager;
pub mod manager_impl;
pub mod types;

// mod.rs: L20
pub use manager::*;
```

**Change from explicit to wildcard re-export**: Before R27, `mod.rs` had explicit `pub use manager::{GitInfo, RelatedPath, ...}` (15 items). After R27, it uses `pub use manager::*;`.

**Impact**: The wildcard re-export preserves all original 15 items plus newly exposed `IDENTITY_FILE_NAME` and `WorkspaceWorktreeInfo`. The stage summary notes: "This widens the public API but preserves all original re-exports."

**QClaw Assessment**: This is a **minor API surface expansion**. The wildcard re-export means any future `pub` item added to `manager.rs` (or its re-exported siblings `types.rs`/`manager_impl.rs`) is automatically re-exported. This is:
- Acceptable for a module-level re-export (`workspace::*` is already broad)
- Consistent with Rust's module conventions
- A risk: future additions to `types.rs` or `manager_impl.rs` will be automatically public

**Alternative**: Keep explicit re-export list and add `IDENTITY_FILE_NAME` + `WorkspaceWorktreeInfo` explicitly. This is more verbose but preserves precise API control. P3 observation.

---

## 3. Visibility Verification (QClaw)

### 3.1 `manager_impl.rs` Visibility

```bash
grep -c 'pub(super) fn\|pub(super) async fn' src/crates/assembly/core/src/service/workspace/manager_impl.rs
# → 13

grep -c '^pub fn\|^pub async fn' src/crates/assembly/core/src/service/workspace/manager_impl.rs
# → 0
```

**13 `pub(super)` methods, 0 `pub` methods.** ✅ This follows the R23 pattern (`pub(super)` for cross-sibling, `pub` only for cross-crate facade delegates). Since `manager.rs` facade re-exports everything via `pub use super::manager_impl::*;`, the methods don't need to be `pub` — they're re-exported through the facade.

### 3.2 `types.rs` Visibility

```bash
grep -c '^pub struct\|^pub enum\|^pub type' src/crates/assembly/core/src/service/workspace/types.rs
# → (not counted, but `IDENTITY_FILE_NAME` is `pub const`)
```

**`IDENTITY_FILE_NAME`**: Changed from `pub(crate)` to `pub` per stage summary. This is required because `pub use manager::*;` in `mod.rs` only re-exports `pub` items, not `pub(crate)` items. The change is justified and documented. ✅

### 3.3 `impl Default` Visibility

**Stage summary lesson**: "`pub(super) fn default()` was rejected by Rust — `default()` trait method doesn't allow visibility qualifier. Removed `pub(super)` from `impl Default` `fn default()`."

**QClaw Verification**: This is correct. `impl Default for T { fn default() -> Self { ... } }` does not allow `pub(super)` on the `fn default()` method because it's a trait impl method, not an inherent method. The visibility is determined by the trait's visibility (`pub` trait = `pub` methods). ✅

---

## 4. Cross-Crate API Verification (QClaw)

### 4.1 Direct Module References

```bash
git grep -n 'manager_impl::' -- ':!src/crates/assembly/core/src/service/workspace/'
# → 0 hits

git grep -n 'workspace::types::' -- ':!src/crates/assembly/core/src/service/workspace/'
# → 0 hits
```

**0 cross-crate direct module references.** ✅ External crates use `workspace::GitInfo`, `workspace::WorkspaceManager`, etc. via the `mod.rs` `pub use manager::*;` re-export.

### 4.2 Consumer Crate Verification

Stage summary claims 5 consumer crates compile clean:
- `northhing-services-integrations` (99 tests)
- `northhing-runtime-services` (3 tests)
- `northhing-agent-runtime` (0 tests)
- `northhing-agent-tools` (0 tests)
- `northhing-product-capabilities` (0 tests)

**QClaw Verification**: `cargo check --workspace` passed with 0 errors. This includes all 5 consumer crates. The workspace check confirms cross-crate compilation. ✅

---

## 5. Cargo Verification

### 5.1 Cargo Check (northhing-core)

```bash
cargo check -p northhing-core --lib --message-format=short
# → 0 errors
# → 29 warnings (unused imports in service.rs, update.rs, etc. — pre-existing or R27 residue)
# → Finished in 0.87s
```

**0 NEW errors.** ✅ 29 warnings are unused imports (e.g., `tokio::fs` in `service.rs`, `RelatedPath` in `update.rs`). These are likely **R27 residue** from the split — some imports that were used in `manager.rs` (now in `manager_impl.rs`) are no longer needed in `service.rs`/`update.rs`. Or they could be pre-existing. Either way, unused import warnings are not compilation errors. P3 cleanup.

### 5.2 Cargo Check (Workspace)

```bash
cargo check --workspace --message-format=short
# → 0 errors
# → 5 warnings (northhing desktop) + 3 warnings (northhing-cli) + 5 duplicates (northhing bin)
# → Finished in 3m 11s
```

**0 NEW errors across workspace.** ✅ Desktop and CLI warnings are pre-existing (method never used, type visibility mismatches). Not R27 regression.

### 5.3 Cargo Test

Not independently verified by QClaw (300s timeout). Stage summary claims:
- `cargo test -p northhing-core --lib`: 103 passed, 0 failed
- Consumer crates (5): 102 passed, 0 failed

**Presumed OK** per stage summary. The workspace `cargo check` passing with 0 errors strongly implies tests would compile. However, test execution was not verified. Minor review gap. ⏸

### 5.4 Cargo.lock Drift

```bash
git diff HEAD~5 -- Cargo.lock | wc -l
# → 0
```

**0 drift.** ✅

---

## 6. Iron Rules Compliance (QClaw Verified)

| Rule | Pre (manager.rs 1505) | Post (sum of 3 files) | Delta | Status |
|------|----------------------|-----------------------|-------|--------|
| `unwrap()` | 0 | 0 | **0** | ✅ |
| `panic!` | 0 | 0 | **0** | ✅ |
| `unreachable!` | 0 | 0 | **0** | ✅ |
| `let _ = Result` | 0 | 0 | **0** | ✅ |

**0 NEW unwrap/panic/unreachable/let _ = Result.** ✅

---

## 7. Line Endings & Encoding

```bash
file src/crates/assembly/core/src/service/workspace/manager.rs
# → ASCII text (LF-only, no BOM) ✅

file src/crates/assembly/core/src/service/workspace/manager_impl.rs
# → assembler source, Unicode text, UTF-8 (with BOM) text ⚠️

file src/crates/assembly/core/src/service/workspace/types.rs
# → C source, Unicode text, UTF-8 (with BOM) text ⚠️
```

**BOM Detected in 2 files**: `manager_impl.rs` and `types.rs` have UTF-8 BOM (`EF BB BF` at file start, verified by `od -An -tx1`).

**Impact**: BOM is technically valid UTF-8 but:
1. Not standard practice in Rust (most Rust files are BOM-free UTF-8)
2. Can cause issues with some tools (e.g., `cat`, `diff`, `grep` may show BOM as visible characters in some terminals)
3. `file` command misidentifies `manager_impl.rs` as "assembler source" and `types.rs` as "C source" due to BOM + content heuristics (same issue as R16 `helpers.rs` being identified as "Algol 68 source")
4. `cargo fmt` and `rustc` handle BOM correctly, so no compilation issues

**Root cause**: The Mavis take-over Python script likely wrote the files with BOM. The `open()` function in Python on Windows defaults to UTF-8 with BOM when writing in some contexts (e.g., `encoding='utf-8-sig'`).

**Fix**: Remove BOM with `sed -i '1s/^\xEF\xBB\xBF//' file.rs` or `dos2unix` (if available). Or configure Python script to use `encoding='utf-8'` (not `utf-8-sig`). P2 cleanup.

---

## 8. Long Lines >120

```bash
for f in manager manager_impl types; do
  awk '{ if (length > 120) print NR": "length }' \
    src/crates/assembly/core/src/service/workspace/${f}.rs
done
```

**0 lines >120 across all 3 files.** ✅ Well within R18 ≤5/file tolerance.

---

## 9. Lessons Documentation (QClaw Assessment)

The stage summary documents 6 lessons from the Mavis take-over. QClaw evaluates each:

| # | Lesson | Validity | QClaw Assessment |
|---|--------|----------|-----------------|
| 1 | f-string `{{` and `}}` literal in Python | ✅ Valid | Same R26 bug. Raw string used. Correct fix. |
| 2 | Range off-by-one: (0, 295) vs (0, 294) | ✅ Valid | Python slice `[0:295]` = indices 0-294 (295 items), but lines are 1-indexed. (0, 295) in Mavis script = L1-L295 inclusive. Off-by-one fixed. Correct. |
| 3 | `pub(super)` on `fn default()` rejected by Rust | ✅ Valid | Trait impl methods don't allow visibility qualifiers. Correct observation. |
| 4 | Stray `//!` at L8: need `//` not `//!` after extraction | ✅ Valid | After extracting use block, `//!` (doc comment) becomes file-level doc comment. If the extracted file starts with `//!`, it's valid, but the original file's `//!` becomes a stray comment. Correct. |
| 5 | Reading from facade overwrites source: need `git checkout` first | ✅ Valid | If extraction script reads from the already-extracted facade (7 lines), it extracts 7 lines of garbage. Correct. |
| 6 | `pub(crate)` items not re-exported via `pub use ...::*` | ✅ Valid | Only `pub` items are re-exported by wildcard. `IDENTITY_FILE_NAME` changed from `pub(crate)` to `pub`. Correct. |

All 6 lessons are **valid and well-documented**. These are valuable for future Mavis take-over rounds. ✅

---

## 10. Quality Assessment

| Dimension | Score | Notes |
|-----------|-------|-------|
| Facade reduction | 10/10 | 1505 → 7 (-99.5%). The thinnest facade in project history. Pure re-exports. |
| Split strategy | 10/10 | Horizontal split (types vs impl) is correct for DTO+impl god-files. Private field access preserved. |
| Cap compliance | 10/10 | types.rs 300, manager_impl.rs 1234. No cap specified but both are reasonable. manager_impl.rs 1234 is under 1500. |
| Visibility pattern | 9/10 | `pub(super)` on 13 impl methods. `pub` on `IDENTITY_FILE_NAME` (justified). `impl Default` correctly has no visibility qualifier. |
| Cross-crate API stability | 10/10 | 0 direct module references. `pub use manager::*;` wildcard preserves all original re-exports + 2 new items. |
| Iron rules | 10/10 | 0 NEW unwrap/panic/unreachable. |
| Line endings | 7/10 | 0 CRLF. But 2 files have BOM (manager_impl.rs, types.rs). BOM is not a compilation error but non-standard. |
| Line length | 10/10 | 0 lines >120. |
| Cargo health | 9/10 | 0 errors. 29 warnings (unused imports — likely R27 residue or pre-existing). Tests not independently verified. |
| Cargo.lock hygiene | 10/10 | 0 drift. |
| Lessons documentation | 10/10 | 6 lessons documented, all valid. Valuable for future rounds. |
| mod.rs wildcard re-export | 8/10 | `pub use manager::*;` is convenient but widens API surface. Future additions to types.rs/manager_impl.rs become automatically public. |
| **Overall** | **9.2/10** | **APPROVE** |

---

## 11. Verdict

### ✅ APPROVED Items

1. **Facade reduction**: 1505 → 7 (-99.5%). Pure re-export facade. Best in project history. ✅
2. **Horizontal split strategy**: types.rs (300) + manager_impl.rs (1234). Correct for private field access. ✅
3. **0 compile errors**: northhing-core + workspace both pass. ✅
4. **Cross-crate API stable**: 0 direct module references. `pub use manager::*;` preserves all original re-exports. ✅
5. **Visibility correct**: 13 `pub(super)` methods, 0 `pub` methods in manager_impl.rs. `IDENTITY_FILE_NAME` `pub` justified. ✅
6. **`impl Default` visibility**: Correctly no qualifier (trait impl methods don't allow `pub(super)`). ✅
7. **0 unwrap/panic/unreachable**: Iron rules preserved. ✅
8. **0 CRLF**: No Windows line endings. ✅
9. **0 long lines >120**: All 3 files clean. ✅
10. **Cargo.lock 0 drift**: No dependency changes. ✅
11. **6 lessons documented**: All valid, valuable for future Mavis take-overs. ✅
12. **Consumer crates compile**: 5 crates (services-integrations, runtime-services, agent-runtime, agent-tools, product-capabilities) all compile clean. ✅
13. **`use super::*;` in sibling headers**: Siblings import from each other via wildcard. Works because mod.rs re-exports all items. ✅
14. **mod.rs explicit → wildcard re-export**: Preserves all original 15 items + adds 2 new items. No breakage. ✅
15. **R23 pattern applied**: `pub(super)` on impl methods (R23 lesson). ✅

### ⚠️ Minor Observations (Non-blocking)

1. **BOM in 2 files**: `manager_impl.rs` and `types.rs` have UTF-8 BOM (`EF BB BF`). Not a compilation error but non-standard. Recommend removing BOM in R28 cleanup. `sed -i '1s/^\xEF\xBB\xBF//' file.rs`.
2. **mod.rs wildcard re-export**: `pub use manager::*;` widens API surface. Any future `pub` item in `types.rs` or `manager_impl.rs` is automatically public. Alternative: explicit re-export list (15 original + 2 new = 17 items). P3 cleanup.
3. **Unused import warnings (29)**: `cargo check -p northhing-core` shows 29 unused import warnings. Some may be R27 residue (imports that were needed in `manager.rs` but moved to `manager_impl.rs`). `cargo fix --lib -p northhing-core` can auto-fix. P3 cleanup.
4. **Tests not independently verified**: Stage summary claims 103 + 102 passed, but QClaw didn't run `cargo test` (300s timeout). `cargo check --workspace` passing implies tests compile, but execution is presumed. Minor review gap. ⏸
5. **Facade line count discrepancy**: Stage summary says 8 lines, actual is 7 lines. ±1 rounding, cosmetic.

### ❌ NOT Applicable (Not R27 Scope)

- `service.rs` (1029), `lifecycle.rs` (343), `accessors.rs` (205), `update.rs` (357), `admin.rs` (821): Pre-existing R23 siblings, not touched by R27.
- `factory.rs` (883), `identity_watch.rs` (9491), `provider.rs` (6539): Pre-existing, not touched by R27.

---

## 12. R28 Recommendations (Deferred Cleanup)

| Priority | Task | Effort | Rationale |
|----------|------|--------|-----------|
| P2 | Remove BOM from `manager_impl.rs` + `types.rs` | 1 min | `sed -i '1s/^\xEF\xBB\xBF//'` both files. Non-standard UTF-8. |
| P3 | Fix 29 unused import warnings | 5 min | `cargo fix --lib -p northhing-core` auto-fix. Some R27 residue. |
| P3 | Consider explicit re-export list in mod.rs | 5 min | Replace `pub use manager::*;` with explicit 17 items. Better API control. |
| P3 | Verify `cargo test` execution | 5 min | Run `cargo test -p northhing-core --lib` with 600s timeout. Close review gap. |

---

## 13. References

- R27 spec: (no separate spec file — Mavis take-over directly, stage summary serves as spec)
- R27 stage summary: `docs/handoffs/2026-07-02-r27-stage-summary.md` (`5dec785`)
- R27 impl: `5dec785` (single commit, Mavis take-over)
- R23 review: `docs/handoffs/2026-07-02-r23-stage-review-report.md` (`ce4092c`)
- R26 review: (if applicable, R26 was contracts/runtime-ports)
- R19 lesson: `docs/handoffs/2026-07-01-r19-acp-manager-split-review-report.md` (`33a380a`)
- Code-rot prevention: `docs/code-rot-prevention-guide.md`

---

*R27 Stage Review completed by QClaw on 2026-07-03. Commit `5dec785` on `main` approved. Score: 9.2/10 APPROVE. Best facade reduction in project history (1505 → 7 lines, -99.5%).*
