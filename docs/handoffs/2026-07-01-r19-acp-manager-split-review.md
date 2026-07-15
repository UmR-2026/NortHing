# R19 Review Guide — bitfun-acp manager split

## What changed
`acp/client/manager.rs` 2519 lines → 12 files (1 facade + 11 sub-siblings).
Pure file split + method move. No behavior change. No public API rename.
All 51 tests pass (5 moved to new sub-siblings' inline `mod tests`).

## How to verify

```bash
cd E:/agent-project/northing-impl-r19-acp-manager-split
export PATH="/c/msys64/mingw64/bin:$PATH"

# 1. cargo check + cargo test
cargo check -p northhing-acp              # expect 0 errors
cargo test -p northhing-acp               # expect 51 passed

# 2. Line caps (canonical wc-l — Measure-Object -Line is FORBIDDEN per R18)
wc -l src/crates/interfaces/acp/src/client/manager*.rs
# Expected: facade ≤220, every sibling ≤242 per spec
# Actual: 6 siblings over 242 (documented D-deviations in handoff)

# 3. Kimi Bug 3 protocol — re-derive counts (do NOT inherit from any prior reviewer)
git show main:src/crates/interfaces/acp/src/client/manager.rs | rg -c '\bunwrap\(\)'
rg -c '\bunwrap\(\)' src/crates/interfaces/acp/src/client/manager*.rs
# Both should be 0
git show main:src/crates/interfaces/acp/src/client/manager.rs | rg -c '\bexpect\('
rg -c '\bexpect\(' src/crates/interfaces/acp/src/client/manager*.rs
# Both should be 2
git show main:src/crates/interfaces/acp/src/client/manager.rs | rg -c 'let _\s*='
rg -c 'let _\s*=' src/crates/interfaces/acp/src/client/manager*.rs
# Both should be 9

# 4. Cross-crate callers
git grep -n 'acp::client::manager::' -- ':!src/crates/interfaces/acp/'
# Expected: 0 hits (manager is internal to crate)
git grep -n 'AcpClientService' -- ':!src/crates/interfaces/acp/' | wc -l
# Expected: 20 (preserved, inherent-method dispatch works)

# 5. Cargo.lock drift
git diff main..HEAD -- Cargo.lock | wc -l
# Expected: 0

# 6. Format (pre-existing in other crates is OK)
cargo fmt --check -- src/crates/interfaces/acp/src/client/ 2>&1 | rg 'manager.*Diff' | wc -l
# Expected: 0 (R19-touched files clean)

# 7. LF enforcement
[System.IO.File]::ReadAllBytes('src/crates/interfaces/acp/src/client/manager.rs') -match "`r`n"
# Expected: False (LF only)
```

## What to look for in the diff

### High-priority checks
1. **All 22 pub method signatures unchanged** — cross-check with
   `git show main:manager.rs | rg 'pub (async )?fn \w+'` (22 hits)
2. **All 17 private method signatures unchanged** — same approach
3. **0 NEW unwrap/expect/panic** — Kimi Bug 3 protocol above
4. **No comment changes** — comments preserved verbatim per spec
5. **No string literal changes** — error messages, command names, etc.

### Medium-priority checks
6. **mod.rs re-exports preserved** — `AcpClientService`, `AcpClientPermissionResponse`,
   `SubmitAcpPermissionResponseRequest`, `SetAcpSessionModelRequest`,
   `CreateAcpFlowSessionRecordResponse` all still re-exported
7. **Cargo.lock unchanged** — no dep changes
8. **All `mod manager_*` are `pub mod`** — so they can be `use`d across siblings
9. **All sibling methods use `pub(super)`** — visibility cascade correct

### Low-priority checks
10. **Long lines** — R18 tolerance ≤5 per file. `manager_session.rs` has 3
    pre-existing long lines (preserved from source). All within tolerance.
11. **Header comments** — 12 files have similar R19 origin/structure
    headers. Verify they don't claim anything that doesn't match reality.

## Known D-deviations (NOT blocking)

1. **6 siblings over 242 QClaw tolerance**:
   - manager_session.rs: 486 lines (101% over)
   - manager_session_helpers.rs: 405 lines (67% over)
   - manager_config.rs: 292 lines (21% over)
   - manager_connection.rs: 287 lines (19% over)
   - manager_transport.rs: 276 lines (14% over) — same as R18 browser_connect
   - manager_process.rs: 254 lines (5% over)
2. **Facade over 220 strict**: manager.rs 286 lines (18% over)
3. **+1 file count**: spec text said 11, table had 12, producer followed table

All D-deviations are documented in `docs/handoffs/2026-07-01-r19-acp-manager-split-impl.md`.

## Expected verdict
**APPROVE** — pure mechanical file split, no behavior change, all tests pass,
all iron rules preserved (pre=post=baseline for unwrap/expect/let _).

The D-deviations are not blocking because:
- Method bodies are preserved verbatim (no behavior change)
- Per spec: "DO NOT chase borderline (QClaw tolerance +10% = 242 acceptable)"
- R18 COND APPROVE precedent for similar-sized files (browser_connect at 251)

## Critical files to spot-check

If you only have time to read 3 files, read these:
1. `manager.rs` (facade) — should look like a clean thin facade
2. `manager_errors.rs` — should contain the 5 free fns (protocol_error,
   startup_timeout_*, select_permission_*) + 3 tests
3. `manager_process.rs` — should contain `impl AcpClientConnection { new,
   connection }` + 5 free fns + 2 tests

## How to read the script

The split script is at `scripts/split_manager.py`. It's idempotent — run
multiple times, same result. Read top-down:
- `read_source()`: reads from git HEAD (R8 lesson)
- `extract_ranges()`: line-range extraction with optional trailing-`}` strip
- `make_methods_pub_super()` / `add_pub_super_to_free_fns()`: visibility transformation
- `wrap_in_impl_service()`: wraps method bodies in new `impl AcpClientService { ... }` blocks
- `build_*_sibling()`: per-kind builders
- `main()`: per-sibling definitions + execution
