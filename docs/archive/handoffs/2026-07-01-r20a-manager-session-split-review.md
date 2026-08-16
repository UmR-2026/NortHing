# R20a Review Guide — bitfun-acp manager_session split

## What to review

R20a split `acp/client/manager_session.rs` (486 canonical lines, Kimi R19
Critical D-deviation +101% over QClaw 242 tolerance) into 2 sibling files
(NO facade). All method bodies moved verbatim from `main`. No behavior change.

Read these in order:
1. **Spec**: `docs/handoffs/2026-07-01-r20a-manager-session-split-spec.md`
2. **Impl handoff**: `docs/handoffs/2026-07-01-r20a-manager-session-split-impl.md`
3. **Source diff**: `git show HEAD` (the single R20a commit) — should show:
   - `D src/crates/interfaces/acp/src/client/manager_session.rs` (deletion)
   - `+ src/crates/interfaces/acp/src/client/manager_session_lifecycle.rs` (new, 291 lines)
   - `+ src/crates/interfaces/acp/src/client/manager_session_resolve.rs` (new, 223 lines)
   - `M src/crates/interfaces/acp/src/client/mod.rs` (alphabetical re-sort, removed old `mod manager_session;`, added 2 new mods)

## Reviewer verification (10 minutes)

```bash
# 0. Worktree state
cd E:/agent-project/northing-impl-r20a-manager-session-split
git log --oneline -3   # Should show R20a commit on top of f579c71 (R20a spec)
git status             # Clean working tree after commit

# 1. File inventory (canonical wc-l)
wc -l src/crates/interfaces/acp/src/client/manager_session_*.rs
# Expected:
#   405 src/crates/interfaces/acp/src/client/manager_session_helpers.rs  (untouched, R19 file)
#   291 src/crates/interfaces/acp/src/client/manager_session_lifecycle.rs  (NEW)
#   223 src/crates/interfaces/acp/src/client/manager_session_resolve.rs     (NEW)

# 2. Method count preservation (cross-check by grep)
grep -cE 'fn \w+|async fn \w+' src/crates/interfaces/acp/src/client/manager_session_lifecycle.rs src/crates/interfaces/acp/src/client/manager_session_resolve.rs
# Expected: 4 (lifecycle) + 3 (resolve) = 7 total methods, matching R19 baseline

# 3. Visibility check
echo "==pub async fn=="
grep -cE '^\s+pub async fn' src/crates/interfaces/acp/src/client/manager_session_lifecycle.rs src/crates/interfaces/acp/src/client/manager_session_resolve.rs
# Expected: lifecycle 4, resolve 0

echo "==pub(super) async fn=="
grep -cE '^\s+pub\(super\) async fn' src/crates/interfaces/acp/src/client/manager_session_lifecycle.rs src/crates/interfaces/acp/src/client/manager_session_resolve.rs
# Expected: lifecycle 0, resolve 2 (resolve_or_create_client_session, ensure_remote_session)

echo "==async fn (no pub)=="
grep -cE '^\s+async fn' src/crates/interfaces/acp/src/client/manager_session_lifecycle.rs src/crates/interfaces/acp/src/client/manager_session_resolve.rs
# Expected: lifecycle 0, resolve 1 (resolve_client_session)

# 4. Iron rules — Kimi Bug 3 fix protocol
echo "==Pre-split unwrap/expect baseline=="
git show main:src/crates/interfaces/acp/src/client/manager_session.rs > "$env:TEMP\ms_baseline.txt"
(Get-Content "$env:TEMP\ms_baseline.txt" | Select-String -Pattern 'unwrap').Count
(Get-Content "$env:TEMP\ms_baseline.txt" | Select-String -Pattern 'expect').Count

echo "==Post-split unwrap/expect=="
(Get-Content src/crates/interfaces/acp/src/client/manager_session_lifecycle.rs, src/crates/interfaces/acp/src/client/manager_session_resolve.rs | Select-String -Pattern 'unwrap').Count
(Get-Content src/crates/interfaces/acp/src/client/manager_session_lifecycle.rs, src/crates/interfaces/acp/src/client/manager_session_resolve.rs | Select-String -Pattern 'expect').Count
# Both should equal: pre=0, post=0

# 5. Cargo checks (cross-crate consumer verification — R19 MANDATORY)
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cargo check -p northhing-acp --message-format=short 2>&1 | Select-String -Pattern '^error\[' | Measure-Object | Select-Object -ExpandProperty Count
# Expected: 0
cargo check -p northhing-cli --message-format=short 2>&1 | Select-String -Pattern '^error\[' | Measure-Object | Select-Object -ExpandProperty Count
# Expected: 0  ← R19 lesson: this is the regressed crate
cargo check --workspace --message-format=short 2>&1 | Select-String -Pattern '^error\[' | Measure-Object | Select-Object -ExpandProperty Count
# Expected: 0

# 6. Tests pass
cargo test -p northhing-acp --lib 2>&1 | Select-String -Pattern '^test result' | Select-Object -First 1
# Expected: "test result: ok. 51 passed; 0 failed; 0 ignored; ..."
cargo test -p northhing-core --features 'service-integrations,product-full' --lib 2>&1 | Select-String -Pattern '^test result' | Select-Object -First 1
# Expected: "test result: ok. 899 passed; 0 failed; 1 ignored; ..."

# 7. Format (no NEW diff in R20a-touched files)
cargo fmt --check -- src/crates/interfaces/acp/src/client/ 2>&1 | Select-String -Pattern '^Diff in' | Where-Object { $_.Line -match 'src.crates.interfaces.acp.src.client.manager_session_(lifecycle|resolve).rs' } | Measure-Object | Select-Object -ExpandProperty Count
# Expected: 0

# 8. LF enforcement (no CRLF in new files)
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
file src/crates/interfaces/acp/src/client/manager_session_lifecycle.rs src/crates/interfaces/acp/src/client/manager_session_resolve.rs
# Expected: "ASCII text" (Windows file command simplified output; or check Format-Hex for 0x0A only)

# 9. Line length (R18 rule ≤5 new long lines per file)
Select-String -Path src/crates/interfaces/acp/src/client/manager_session_lifecycle.rs, src/crates/interfaces/acp/src/client/manager_session_resolve.rs -Pattern '^.{121,}$' | Group-Object Path | ForEach-Object { @{ File = (Split-Path $_.Name -Leaf); Count = $_.Count } }
# Expected: lifecycle 0; resolve 5 (3 NEW in headers/imports + 2 inherited verbatim from R19 baseline)

# 10. Cross-crate consumer check (R19 lesson)
git grep -nE 'acp::client::manager_session_(lifecycle|resolve)' -- ':!docs/'
# Expected: 0 hits (methods consumed via AcpClientService inherent dispatch, not direct module imports)

git grep -n 'AcpClientService' -- ':!src/crates/interfaces/acp/' 2>&1 | Tee-Object -FilePath "$env:TEMP\refs.txt" | Out-Null
(Get-Content "$env:TEMP\refs.txt" | Measure-Object -Line).Lines
# Expected: ~88 (preserved from pre-split, no Δ from R20a)
```

## 10-axis verification (consolidated)

| # | Axis | Expected | Actual (per impl handoff) | Reviewer verdict |
|---|---|---|---|---|
| 1 | Line cap violations | ≤242 strict; spec D-deviation accepted for >242 | lifecycle 291 (D-dev, accepted); resolve 223 (within); manager_session DELETED | ☐ |
| 2 | Method count preserved | 7 (4 lifecycle + 3 resolve) | 7 ✓ | ☐ |
| 3 | Visibility | 4 pub + 2 pub(super) + 1 private (no pub) | matches ✓ | ☐ |
| 4 | Cargo.lock drift | 0 | 0 ✓ | ☐ |
| 5 | Tests | acp 51/0/0 + core 899/0/1 | matches ✓ | ☐ |
| 6 | Iron rules | 0 NEW unwrap/expect/panic/unreachable/let _ = Result | 0/0/0/0/0 ✓ | ☐ |
| 7 | Format | 0 NEW fmt diff | 0 ✓ | ☐ |
| 8 | LF | LF only | LF only ✓ | ☐ |
| 9 | Line length | ≤5 new long lines per file | lifecycle 0 NEW; resolve 3 NEW + 2 inherited (within ≤5) | ☐ |
| 10 | Cross-crate | 0 NEW module refs; AcpClientService count preserved | matches ✓ | ☐ |

## Reviewer focus areas

### 1. Spec deviation #1 (visibility for 2 resolve methods)

**Spec §Pre-emptive split design** says: "3 private helpers stay private (no `pub` keyword at all)."

**Impl deviation**: 2 of the 3 helpers (`resolve_or_create_client_session`, `ensure_remote_session`) are `pub(super) async fn` instead of `async fn` (no `pub`).

**Reviewer check**:
- Verify the claim "inherent dispatch within same crate" in spec §Visibility table is technically wrong.
- Confirm `self.method()` call from `manager_session_lifecycle.rs` to a method declared in `manager_session_resolve.rs` requires at minimum `pub(super)` visibility.
- Verify that spec §Risk assessment authorizes this: "3 private helpers may need `pub(super)` if other sibling files call them directly (not via inherent dispatch). Apply R19 lesson: check each call site before deciding visibility. Default `pub(crate)` if cross-sibling call."

**Acceptance criteria**: Reviewer agrees that `pub(super)` is the minimum visibility for cross-sibling inherent dispatch, OR proposes an alternative (e.g., extract helpers into a 3rd private module that both siblings depend on — but this would increase complexity).

### 2. Spec deviation #2 (lifecycle.rs 291 lines, over 242 cap)

**Spec §Pre-emptive split design** says: "Both files ≤242 strict cap with conservative buffer."

**Impl reality**: `manager_session_lifecycle.rs` = 291 canonical wc-l.

**Reviewer check**:
- Confirm 4 method bodies total 252 source lines (per spec source estimate).
- Confirm verbatim move (no body refactoring) is required by spec.
- Verify this matches spec's own estimated range "~280-310" in source-line estimate column.
- Decide: accept as minor D-deviation per spec's "DO NOT chase borderline" R19 precedent, OR request refactoring (extract `set_session_model` helper to reduce ~98 lines).

**Acceptance criteria**: Reviewer agrees that 291 is the minimum achievable with verbatim move, OR proposes specific body refactoring that preserves behavior.

### 3. Cross-crate consumer verification (R19 lesson — MANDATORY)

R19 broke `northhing-cli` with 11 E0624 errors by over-prescribing `pub(super)` visibility. R20a spec mandates `cargo check -p northhing-cli` to catch similar regressions.

**Reviewer check**: Re-run `cargo check -p northhing-cli` independently and confirm 0 errors.

### 4. Inherent-method dispatch across sibling modules

**Reviewer check**: Confirm that `self.resolve_or_create_client_session(...)` and `self.ensure_remote_session(...)` calls from `manager_session_lifecycle.rs` resolve correctly via Rust's inherent-method lookup across `impl AcpClientService { ... }` blocks in different files.

The way Rust resolves inherent methods:
- All `impl Foo { ... }` blocks in the same crate contribute to `Foo`'s method table.
- When `service.method()` is called, Rust looks up `method` in `Foo`'s method table (regardless of which file/block declared it).
- Visibility is checked separately: the caller (lifecycle.rs) must be able to see the method declaration. Methods declared in `mod manager_session_resolve` with `pub(super)` are visible to siblings of `manager_session_resolve` (i.e., `manager_session_lifecycle`, since both are children of `mod client`).

**Acceptance criteria**: Build passes (confirms visibility is correct), tests pass (confirms runtime dispatch works).

### 5. Method body preservation (no behavior change)

**Reviewer check**: Diff each method body line-by-line vs original `git show main:src/.../manager_session.rs`. Confirm:
- 0 logic changes.
- 0 comment changes (verbatim per R17 lesson).
- 0 formatting changes (rustfmt applied AFTER move; pre-fmt bodies were verbatim).
- All imports preserved (or correctly consolidated; `lifecycle.rs` has fewer imports than original since resolve-specific ones moved out).

### 6. Iron rules — Kimi Bug 3 fix protocol

**Reviewer check**: Independently re-run precise grep on `git show main:src/.../manager_session.rs` for unwrap/expect/panic/unreachable/let _ = Result. Confirm pre-split baseline = 0 for all 5 patterns. Then sum post-split counts from `lifecycle.rs + resolve.rs`. Confirm post-split = 0 for all 5 patterns.

**Acceptance criteria**: pre == post == 0 for all 5 iron-rule patterns.

## Reviewer decision matrix

| Decision | Trigger | Action |
|---|---|---|
| APPROVE | All 10 axes green; both spec deviations accepted | Merge to main via `pnpm`-style merge |
| COND APPROVE | All 10 axes green; one or more spec deviations noted but acceptable | Document deviation + merge |
| REJECT | Any axis red OR spec deviations unacceptable | Fix and re-review |

## Cross-round review links

- R19 review report (QClaw): `docs/handoffs/2026-07-01-r19-acp-manager-split-review-report.md`
- R19 review guide: `docs/handoffs/2026-07-01-r19-acp-manager-split-review.md`
- R19 spec: `docs/handoffs/2026-07-01-r19-acp-manager-split-spec.md`
- R19 impl: `docs/handoffs/2026-07-01-r19-acp-manager-split-impl.md`
- R20a spec: `docs/handoffs/2026-07-01-r20a-manager-session-split-spec.md`
- R20a impl (this handoff): `docs/handoffs/2026-07-01-r20a-manager-session-split-impl.md`
- R20a review guide: `docs/handoffs/2026-07-01-r20a-manager-session-split-review.md` (this file)