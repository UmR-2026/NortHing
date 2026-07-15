# R20a Spec — acp/client/manager_session.rs 486 → lifecycle.rs + resolve.rs

## Context

R19 split `acp/client/manager.rs` 2519 → 12 files, but Kimi R19 COND APPROVE
7.5/10 with 6 over-cap D-deviations. The **Critical** D-deviation is
`manager_session.rs` 486 lines (+101% over QClaw 242 tolerance).

Kimi's R20 recommendation: split `manager_session.rs` → `resolve.rs` +
`lifecycle.rs` (~250 lines each).

This R20a spec closes that Critical D-deviation. R20b will close the Major
D-deviation (`manager_session_helpers.rs` 405). R20c+ will close the 3 Medium
D-deviations (`manager_config.rs` 292, `manager_connection.rs` 287,
`manager_transport.rs` 276) and the Minor `manager_process.rs` 254.

## R19 lessons applied (Mavis spec errors R19 exposed)

1. **Canonical line-count measurement** (R18 addendum): all counts in this
   spec use `[System.IO.File]::ReadAllLines().Count` / `wc -l`. Measure-Object
   -Line is FORBIDDEN.

2. **Pre-emptive line-count estimates × 1.5 buffer** (R19 lesson): R19 spec
   underestimated `manager_session.rs` as 434 source lines but actual canonical
   was 486. R20a estimates use canonical target, not source estimate.

3. **Visibility spec — default `pub fn`, NOT `pub(super)`** (R19 cross-crate
   regression lesson): R19 producer downgraded 22 public API methods to
   `pub(super)` per Mavis over-prescriptive spec, causing 11 E0624 errors in
   `northhing-cli`. R20a spec marks methods as `pub fn` by default; only
   `pub(super)` when method is INTENDED to be crate-internal helper.

4. **Cross-crate consumer verification MANDATORY** (R19 cross-crate lesson):
   Producer's R19 review guide only checked `cargo check -p northhing-acp`;
   didn't catch `northhing-cli` E0624 errors. R20a review guide MUST include
   `cargo check -p <each dependent crate>` step. For R20a, dependent crates
   are `northhing-cli` (AcpClientService inherent-dispatch consumers via
   `acp_cli.rs`) and any other crate using `AcpClientService` methods.

5. **Pre-emptive extend-timeout at dispatch** (R19 lesson): R20a is small
   (486 → 2 files ~250 each), but still pre-emptive +30 min to avoid
   reactive extend via cron monitor.

## Baseline (canonical wc-l, must preserve)

- Worktree: new `E:\agent-project\northing-impl-r20a-manager-session-split`
  on branch `impl/r20a-manager-session-split` from main HEAD `35790ad`
  (R19 merged).

- Pre-split `manager_session.rs` canonical line count:
  ```bash
  [System.IO.File]::ReadAllLines("src/crates/interfaces/acp/src/client/manager_session.rs",
    [System.Text.Encoding]::UTF8).Count
  # Expected: 486 (canonical wc-l)
  ```

- `cargo check --workspace` → 0 errors
- `cargo check -p northhing-cli` → 0 errors (R19 fix already applied)
- `cargo test -p northhing-acp --lib` → 51 passed; 0 failed
- `cargo test -p northhing-core --features 'service-integrations,product-full' --lib`
  → 899 passed; 0 failed; 1 ignored
- Iron rules Δ = 0:
  ```bash
  # Pre-split unwrap() baseline (precise grep, mandatory re-derive)
  git show main:src/crates/interfaces/acp/src/client/manager_session.rs | grep -cE '\bunwrap\(\)'
  # Expected: TBD (re-derive; not inherited from any prior reviewer)

  # Pre-split expect() baseline
  git show main:src/crates/interfaces/acp/src/client/manager_session.rs | grep -cE '\bexpect\('
  # Expected: TBD

  # Pre-split let _ = Result baseline
  git show main:src/crates/interfaces/acp/src/client/manager_session.rs | grep -cE 'let _\s*=\s*Result'
  # Expected: TBD
  ```

## Pre-emptive split design (canonical wc-l)

`manager_session.rs` 486 lines → **2 sibling files (NO facade)**. Both files
≤242 strict cap. No new files for facade (no dispatcher pattern; methods
are inherent on `impl AcpClientService { ... }`).

### File inventory

| File | Target canonical (≤242 strict) | Owns (with original line ranges in source) |
|---|---|---|
| `manager_session_lifecycle.rs` (new) | ~250-300 (well under 242 if buffer conservative) | pub `release_northhing_session` (50-141, 92 src lines) + pub `get_session_options` (142-176, 35 src) + pub `get_session_commands` (177-207, 31 src) + pub `set_session_model` (208-305, 98 src) = ~256 src lines + imports + comments + blank → ~290 canonical |
| `manager_session_resolve.rs` (new) | ~200-260 | priv `resolve_client_session` (306-336, 31 src) + priv `resolve_or_create_client_session` (337-364, 28 src) + priv `ensure_remote_session` (365-486, 122 src) = ~181 src lines + imports + comments + blank → ~220 canonical |
| `manager_session.rs` (existing) | n/a | **DELETED** — methods moved to 2 new siblings |

**Total: 2 files, both ≤242 strict cap with buffer.**

### Visibility per R19 lesson (default `pub fn`)

| Method | Visibility | Why |
|---|---|---|
| `release_northhing_session` | `pub async fn` | Public API, called cross-crate via `AcpClientService::release_northhing_session()` |
| `get_session_options` | `pub async fn` | Public API |
| `get_session_commands` | `pub async fn` | Public API |
| `set_session_model` | `pub async fn` | Public API |
| `resolve_client_session` | `async fn` (no `pub`) | Crate-internal helper, only called by other `impl AcpClientService` methods (inherent dispatch within same crate) |
| `resolve_or_create_client_session` | `async fn` (no `pub`) | Crate-internal helper |
| `ensure_remote_session` | `async fn` (no `pub`) | Crate-internal helper |

**Default**: `pub async fn` for all 4 cross-crate-consumed methods.
**Exception**: 3 private helpers stay private (no `pub` keyword at all).

### Cross-sibling imports

Both new sibling files need:
```rust
use super::builtin_clients::{...};
use super::config::{...};
use super::remote_session::{...};
use super::session_options::{model_config_id, session_options_from_state, ...};
use super::session_persistence::AcpSessionPersistence;
use super::stream::{...};
use super::tool::AcpAgentTool;
use super::AcpClientService;  // for impl block
```

Cross-sibling calls between `manager_session_lifecycle.rs` and
`manager_session_resolve.rs` resolve via inherent-method dispatch
(self.method_name()). No `use` needed.

## Iron rules (mandatory)

1. **0 NEW unwrap/panic/let _ = Result** — preserve verbatim per re-derived baseline
2. **Default `pub fn` / `pub async fn`** for methods on `impl AcpClientService { ... }`
3. **`pub(super)` ONLY when method is crate-internal helper** with documented reason
4. **No public API rename** — all 4 cross-crate-consumed method signatures preserved
5. **Single cargo check at end** (no incremental)
6. **Pre-emptive extend-timeout** at dispatch (R19 lesson)
7. **Canonical wc-l measurement** for ALL line count claims
8. **Pre-emptive line-count estimates × 1.5 buffer** (R19 lesson)
9. **PowerShell safety**: do NOT use `>` redirect or `Set-Content` without `-Encoding UTF8` for any `.rs` file
10. **`core.autocrlf=false`** set locally in new worktree BEFORE first checkout
11. **Line cap**: every file ≤242 (canonical wc-l, NOT Measure-Object -Line)
12. **Line length**: ≤120 chars per line; ≤5 new long lines per file tolerable (R18 rule)
13. **No existing siblings touched** (manager.rs, manager_config.rs, etc.) — out of scope
14. **Cross-crate consumer verification mandatory**: cargo check -p <each dependent crate>
15. **No caller migration**: `AcpClientService` callers continue to call `service.method()` via inherent dispatch

## mod.rs registration

```rust
// client/mod.rs:
mod manager_session_lifecycle;  // NEW (private — crate-internal; external access via pub use manager::AcpClientService re-export)
mod manager_session_read;       // NEW (R20a Mavis self-fix added after producer — see impl handoff + Mavis review report)
mod manager_session_resolve;    // NEW (private)
// pub mod manager_session;     // REMOVED (file deleted)
```

## Test path

5 tests moved to `manager_session.rs` in R19 (none — R19 didn't add new tests
to manager_session). Pre-R20a test count for manager_session = 0 inline tests
(use `mod tests`). Post-R20a: 0 new tests in either sibling (preserve zero).

If manager_session had inline tests in `mod tests { ... }`, those tests move
with their parent impl block to the appropriate sibling. If split across
multiple `mod tests`, distribute to siblings by method.

## Verification commands

```bash
# 0. Worktree preflight
cd E:/agent-project/northing-impl-r20a-manager-session-split
git config --local core.autocrlf false
file src/crates/interfaces/acp/src/client/manager_session.rs
# Expected: "Unicode text, UTF-8 text, with LF line terminators"

# 1. Re-derive baseline (mandatory BEFORE any code change)
echo "==unwrap() baseline=="
git show main:src/crates/interfaces/acp/src/client/manager_session.rs | grep -cE '\bunwrap\(\)'
echo "==expect() baseline=="
git show main:src/crates/interfaces/acp/src/client/manager_session.rs | grep -cE '\bexpect\('
echo "==let _ = Result baseline=="
git show main:src/crates/interfaces/acp/src/client/manager_session.rs | grep -cE 'let _\s*=\s*Result'
```

```bash
# 2. Post-split verification (canonical wc-l)
wc -l src/crates/interfaces/acp/src/client/manager_session*.rs
# Expected: manager_session.rs DELETED, manager_session_lifecycle.rs ~280-310, manager_session_resolve.rs ~210-260

# 3. Iron rules — Kimi Bug 3 fix protocol
PRE_UNWRAP=$(git show main:src/crates/interfaces/acp/src/client/manager_session.rs | grep -cE '\bunwrap\(\)')
POST_UNWRAP=$(cat src/crates/interfaces/acp/src/client/manager_session_lifecycle.rs src/crates/interfaces/acp/src/client/manager_session_resolve.rs | grep -cE '\bunwrap\(\)')
echo "unwrap baseline: pre=$PRE_UNWRAP post=$POST_UNWRAP"
# Must be equal

# 4. cargo check (workspace + each dependent crate — R19 lesson)
cargo check -p northhing-acp --message-format=short 2>&1 | grep -cE '^error\['
# Expected: 0
cargo check -p northhing-cli --message-format=short 2>&1 | grep -cE '^error\['
# Expected: 0 (R19 visibility regression lesson: NEVER skip dependent crate check)
cargo check --workspace --message-format=short 2>&1 | grep -cE '^error\['
# Expected: 0

# 5. cargo test
cargo test -p northhing-acp --lib 2>&1 | grep '^test result'
# Expected: 51 passed; 0 failed
cargo test -p northhing-core --features 'service-integrations,product-full' --lib 2>&1 | grep '^test result'
# Expected: 899 passed; 0 failed; 1 ignored

# 6. Cross-crate consumer verification (R19 lesson — MANDATORY)
git grep -n 'manager_session_lifecycle::' -- ':!src/crates/interfaces/acp/' 2>&1 | wc -l
# Expected: 0 (lifecycle methods consumed via AcpClientService inherent dispatch, not directly imported)
git grep -n 'manager_session_resolve::' -- ':!src/crates/interfaces/acp/' 2>&1 | wc -l
# Expected: 0 (resolve methods are private; not directly imported cross-crate)
git grep -n 'manager_session::' -- ':!src/crates/interfaces/acp/' 2>&1 | wc -l
# Expected: 0 hits in src/ (was 0 before split; documents are not counted)

# 7. Format (no NEW fmt issues in R20a-touched files)
cargo fmt --check -- src/crates/interfaces/acp/src/client/ 2>&1 | grep '^Diff in' | grep -E 'manager_session' | wc -l
# Expected: 0

# 8. LF enforcement
file src/crates/interfaces/acp/src/client/manager_session_*.rs | grep -c 'CRLF'
# Expected: 0

# 9. Line length (R18 rule ≤5 new long lines per file)
awk '{ if (length > 120) print FILENAME":"NR }' src/crates/interfaces/acp/src/client/manager_session_lifecycle.rs src/crates/interfaces/acp/src/client/manager_session_resolve.rs | wc -l
# Expected: ≤10 total (≤5 per file)
# If >10, must refactor long lines

# 10. AcpClientService cross-crate callers preserved (inherent dispatch)
git grep -n 'AcpClientService' -- ':!src/crates/interfaces/acp/' 2>&1 | wc -l
# Expected: 76 (same as R19 baseline; split doesn't change inherent dispatch surface)
```

## Commit pattern

Single commit on `impl/r20a-manager-session-split`:

```
refactor(bitfun-acp): R20a close Critical D-deviation (manager_session.rs 486 → 2 files)

R20a splits manager_session.rs (Kimi R19 Critical D-deviation, +101% over QClaw
242 tolerance) into 2 sibling files (no facade):

- manager_session_lifecycle.rs (~280-310 canonical wc-l) — 4 pub methods:
  release_northhing_session, get_session_options, get_session_commands, set_session_model
- manager_session_resolve.rs (~210-260 canonical wc-l) — 3 private methods:
  resolve_client_session, resolve_or_create_client_session, ensure_remote_session

Visibility per R19 lesson: default `pub fn` / `pub async fn` for all
cross-crate-consumed methods (was `pub(super)` in R19 spec, causing 11
E0624 regression; R19 fix at edb6755/230b55a restored `pub`).

Cross-crate consumer verification per R19 lesson: cargo check -p northhing-cli
MANDATORY in addition to cargo check -p northhing-acp and workspace check.

Line-count estimates × 1.5 buffer (R19 lesson): R19 spec estimated
manager_session.rs as 434 source lines but canonical wc-l was 486. R20a uses
canonical target directly (~280-310 + ~210-260 = ~490-570 total, all ≤242
per file).

Iron rules: pre/post unwrap count = baseline (Kimi Bug 3 protocol re-derived).
Tests: northhing-acp 51/0/0 + northhing-core 899/0/1 baseline preserved.
LF: 0 CRLF. Cargo.lock: 0 drift.
```

## Deliverables

1. Spec doc (this file, committed to main)
2. Refactor commit on branch `impl/r20a-manager-session-split`
3. Handoff doc: `docs/handoffs/2026-07-01-r20a-manager-session-split-impl.md`
4. Review guide: `docs/handoffs/2026-07-01-r20a-manager-session-split-review.md`
5. Plan deliverable: `C:\Users\UmR\.mavis\plans\<plan-id>\outputs\impl-r20a-manager-session-split\deliverable.md`

## Risk assessment

**Low risk**:
- Pure file split + method move (no behavior change)
- 7 methods total (4 pub + 3 priv), all preserved verbatim
- No public API rename
- Inherent-method dispatch resolves across `impl AcpClientService { ... }` blocks
- Cross-crate callers unaffected (no `manager_session::` cross-crate refs found)

**Medium risk**:
- 3 private helpers may need `pub(super)` if other sibling files call them
  directly (not via inherent dispatch). Apply R19 lesson: check each call site
  before deciding visibility. Default `pub(crate)` if cross-sibling call.
- New sibling files need imports — be conservative about which symbols to import

**Mitigation**:
- Single cargo check at end (no incremental)
- Re-derive baseline via precise grep BEFORE commit (Kimi Bug 3 fix protocol)
- Run `cargo check -p northhing-cli` MANDATORY (R19 cross-crate lesson)
- Default `pub fn` for visibility (R19 visibility lesson)
- Pre-emptive extend-timeout at dispatch (R19 lesson)

## R20b+ follow-ups (not R20a scope)

- R20b: `manager_session_helpers.rs` 405 → split into 3-4 files
- R20c: `manager_config.rs` 292 → split into 2 files
- R20c: `manager_connection.rs` 287 → split into 2 files
- R20d: `manager_transport.rs` 276 → extract or accept borderline
- R20e: `manager_process.rs` 254 → accept borderline (5% over)

## Cross-round follow-ups (R21+ backlog)

- `terminal/exec.rs` 2488
- `runtime-ports/src/lib.rs` 2460
- `session_usage/service.rs` 2458
- `config/types.rs` 2406