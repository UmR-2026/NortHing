# Round 13 Handoff: remote_ssh/manager.rs 2810 → 1 facade + 3 sub-handlers

> **Target**: critical #3 god object (`remote_ssh/manager.rs` 2810 lines)
> **Pattern**: R11b/R12b sub-domain split (facade + 3 sub-handlers)
> **Spec**: `docs/handoffs/2026-06-29-round13-remote-ssh-manager-split-spec.md`
> **Commit**: `3b5f520` on branch `impl/round13-remote-ssh-manager-split`
> **Worktree**: `E:\agent-project\northing-impl-round13`

## Summary

Split `remote_ssh/manager.rs` (2810 lines, 97 unique fn signatures across the original
plus the 6 SSHHandler/HandlerError methods, 6 PTYSession methods, and 10 PortForwardManager
methods) into 1 facade + 3 sibling sub-handlers per spec §2.1.

| File | Before | After | Cap | Δ |
|---|---|---|---|---|
| `mod.rs` | 43 | 52 | 200 | +9 (+3 pub mod + 1 cargo-private comment) |
| `manager.rs` (facade) | 2810 | 2303 | 800 | **-507** (D-deviation: still > 800) |
| `manager_handler.rs` (NEW) | — | 251 | 800 | +251 |
| `manager_session.rs` (NEW) | — | 103 | 800 | +103 |
| `manager_port_forward.rs` (NEW) | — | 191 | 800 | +191 |
| **TOTAL** | 2853 | 2900 | — | +47 (boilerplate per sibling) |

Sibling sub-handlers each stay well under the 800-line cap. The facade at 2303 lines
is a known D-deviation that needs R13c (further sub-domain splits: known_hosts /
saved_connections / remote_workspace / ssh_config / SFTP).

## Per-file struct/fn mapping

| Struct / Enum / Impl | Owner sibling | Lines (new file) |
|---|---|---|
| `KnownHostEntry` | facade (`manager.rs`) | (unchanged) |
| `ActiveConnection` | facade (`manager.rs`) | (unchanged, private) |
| `SSHConnectionManager` + `impl` (77 fns) | facade (`manager.rs`) | (unchanged) |
| helpers (`truncate_at_char_boundary`, `ssh_cfg_get`, `ssh_cfg_has`, `sftp_mkdir_all_prefixes`) | facade | (unchanged) |
| `#[cfg(test)] mod tests` (6 tests) | facade | (unchanged) |
| `SSHHandler` (struct + impl) | `manager_handler.rs` | 251 |
| `HandlerError` (struct + Display + Error + 2 From impls) | `manager_handler.rs` | 251 |
| `impl Handler for SSHHandler` (Russh trait) | `manager_handler.rs` | 251 |
| `PTYSession` (struct + 2 impl blocks) | `manager_session.rs` | 103 |
| `PortForward` | `manager_port_forward.rs` | 191 |
| `PortForwardDirection` (enum) | `manager_port_forward.rs` | 191 |
| `PortForwardManager` (struct + impl) | `manager_port_forward.rs` | 191 |
| `impl Default for PortForwardManager` | `manager_port_forward.rs` | 191 |

## Atomic commit

Single atomic commit `3b5f520` per R5/R6/R7/R8/R10a D6 precedent:

```
refactor(remote-ssh): R13 split — 1 facade + 3 sub-handlers (critical #3 god object)

manager.rs 2810 → facade (2303) + 3 sibling sub-handlers (251/103/191 lines).

Split pattern (per spec §1.2):
- manager_handler.rs (251): SSHHandler + HandlerError + 4 impls (pub(crate))
- manager_session.rs (103): PTYSession + 2 impl blocks
- manager_port_forward.rs (191): PortForward + Direction + PortForwardManager + Default

mod.rs: +3 pub mod + re-exports for PTYSession/PortForward/PortForwardDirection/PortForwardManager
(from new siblings). SSHConnectionManager + KnownHostEntry stay re-exported from facade.

Verification:
- BASELINE_ERRORS=0, BASELINE_TESTS=899/0/1, BASELINE_SVC_TESTS=9/0
- 0 fns dropped (97 unique fn signatures preserved across 4 files)
- 0 NEW unwrap/panic/unreachable (16 pre-existing preserved verbatim)
- cargo check services-integrations + northhing-core: 0 errors
- cargo test services-integrations + northhing-core: matches baseline
- cargo fmt --check services-integrations: clean
```

5 files changed, 563 insertions(+), 516 deletions(-).

## Preflight baseline (main, 811b22f)

```
$ cargo check -p northhing-services-integrations --features remote-ssh-concrete --lib
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 18.90s

BASELINE_ERRORS=0

$ cargo test -p northhing-core --features product-full --lib
test result: ok. 899 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.17s

BASELINE_TESTS="899 passed; 0 failed; 1 ignored"

$ cargo test -p northhing-services-integrations --features remote-ssh-concrete --lib
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

BASELINE_SVC_TESTS="9 passed; 0 failed"
```

## Post-split verification (worktree, HEAD 3b5f520)

### 0 fns dropped (97 → 97)

```bash
$ py -c "import re; ..."
original manager.rs unique fns: 97
worktree 4-file unique fns: 97
dropped: 0
added: 0
```

Verified by counting unique fn names (loose regex: `^\s*(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)`)
in original `manager.rs` (from git main) vs 4 worktree files
(`manager.rs` + `manager_handler.rs` + `manager_session.rs` + `manager_port_forward.rs`).

The spec's "102 fns" count was loose; the precise count with this regex is 97, all preserved.
(HandlerError / Display / Error / From impls are trait/blanket impls, not bare fn declarations.)

### Cargo tests match baseline

```
$ cargo test -p northhing-services-integrations --features remote-ssh-concrete --lib
test remote_ssh::manager::tests::mkdir_all_prefixes_collapse_redundant_separators ... ok
test remote_ssh::manager::tests::mkdir_all_prefixes_expand_absolute_posix_path ... ok
test remote_ssh::remote_exec::tests::head_tail_text_keeps_full_output_when_unbounded ... ok
test remote_ssh::remote_exec::tests::remote_exec_session_ids_match_local_test_baseline ... ok
test remote_ssh::manager::tests::rejects_saving_password_connection_without_password ... ok
test remote_ssh::manager::tests::prunes_remote_workspaces_without_saved_connection ... ok
test remote_ssh::manager::tests::prunes_password_connection_without_vault_entry ... ok
test remote_ssh::password_vault::tests::migrate_entry_moves_password_to_new_connection_id ... ok
test remote_ssh::manager::tests::restores_connection_config_from_saved_password_profile ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

$ cargo test -p northhing-core --features product-full --lib
test result: ok. 899 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.16s
```

### Cargo check upstream crates

```
$ cargo check -p northhing-services-integrations --features remote-ssh-concrete --lib
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.40s

$ cargo check -p northhing-core --features product-full --lib
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 49s
```

### Iron rules — 0 NEW violations

Pre-existing unwrap/panic/expect/unreachable count:
- Original `manager.rs` (main): 16 occurrences
- 4 split files in worktree: 16 occurrences (preserved verbatim)

```
$ git diff main..HEAD -- src/crates/services/services-integrations/src/remote_ssh/ \
  | Select-String '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
0  (no NEW violations)
```

### Cargo fmt

```
$ cargo fmt --package northhing-services-integrations --check
(no output — clean)
```

## External API unchanged

`mod.rs` re-export signatures preserved per spec §2.3:

| Item | Re-exported from | Status |
|---|---|---|
| `KnownHostEntry` | facade `manager.rs` | unchanged |
| `SSHConnectionManager` | facade `manager.rs` | unchanged |
| `PTYSession` | new `manager_session.rs` | path moved; mod.rs re-exports |
| `PortForward` | new `manager_port_forward.rs` | path moved; mod.rs re-exports |
| `PortForwardDirection` | new `manager_port_forward.rs` | path moved; mod.rs re-exports |
| `PortForwardManager` | new `manager_port_forward.rs` | path moved; mod.rs re-exports |
| `HandlerError` | `manager_handler.rs` | NOT re-exported (crate-private per spec §1.2) |
| `SSHHandler` | `manager_handler.rs` | NOT re-exported (crate-private per spec §1.2) |

Cross-crate callers (`git grep 'use crate::remote_ssh::manager'`): 1 file
(`remote_terminal.rs`) uses `crate::remote_ssh::manager::SSHConnectionManager` directly.
This path still resolves (facade re-exports `SSHConnectionManager`).

All other 25+ files use `crate::service::remote_ssh::*` (top-level mod.rs re-exports),
which still resolve to the new sibling modules.

## Cross-sibling visibility (no cyclic deps)

| Edge | Visibility | Direction |
|---|---|---|
| facade → `manager_handler::SSHHandler` | `pub(crate)` in handler | facade reads handler |
| facade → `manager_session::PTYSession` | `pub` (re-exported via mod.rs) | facade constructs PTYSession::new |
| `manager_handler` → facade `KnownHostEntry` | `pub` in facade | handler stores `KnownHostEntry` |
| `manager_port_forward` → facade `SSHConnectionManager` | `pub` in facade | port forward holds optional SSH manager |

No cyclic deps: facade does not depend on `manager_port_forward`, handler only depends
on facade for `KnownHostEntry`, port_forward only depends on facade for the manager.

PTYSession construction moved behind a `pub(crate) fn new(channel, connection_id)`
constructor in `manager_session.rs` so the facade can build one without exposing the
private fields.

## Critical caveat: SSHHandler / HandlerError re-export skipped

Spec §2.3 listed `pub use manager_handler::{HandlerError, SSHHandler};` in the proposed
`mod.rs`, but spec §1.2 also said `SSHHandler` is "private" (crate-local). The
crate-private interpretation wins: Rust does not allow `pub use` of `pub(crate)` items
outside the crate. Resolution: omit the `pub use` for these two items and add an
inline comment in `mod.rs` documenting why. Cross-crate callers do not need them
(verified via `git grep SSHHandler` returns 0 cross-crate matches).

## Known D-deviation (NOT R13 scope)

`manager.rs` (facade) is 2303 lines, far above the 800-line cap (spec §2.1 target ~700).
This needs R13c — further sub-domain splits. Candidates (in priority order):

| Sub-domain | Approx lines | Target file |
|---|---|---|
| `known_hosts_*` methods | ~120 | `manager_known_hosts.rs` |
| `saved_connections_*` methods | ~250 | `manager_saved_connections.rs` |
| `remote_workspace_*` methods | ~150 | `manager_remote_workspace.rs` |
| `ssh_config_*` methods | ~250 | `manager_ssh_config.rs` |
| SFTP operations | ~360 | `manager_sftp.rs` |
| `establish_session` + `execute_command_internal` god methods | ~500 | split into helpers (no new file) |

Extracting all 5 sub-domains could bring the facade to ~1170 (still > 800). To reach 800,
additionally split `establish_session` / `execute_command_internal` into smaller helpers
(API-preserving internal restructuring).

R13c scope recommended: extract `manager_saved_connations.rs` + `manager_remote_workspace.rs`
+ `manager_sftp.rs` (3 sibling sub-handlers, matches R13's pattern). This brings facade
to ~1500. Further reduction in R13d if needed.

## Pre-existing vs NEW violations distinction (spec §9)

Pre-existing unwrap/panic count in original `manager.rs`: 16 (all in production paths).
Pre-existing unwrap/panic count across 4 worktree files: 16.

NO `unwrap()`, `expect()`, `panic!`, or `unreachable!` was added in the R13 split.
NO pre-existing unwrap/panic was rewritten as `?` or `ok_or()`.

Pre-existing unwraps live in:
- `manager_handler.rs` (8 in `check_server_key` callback paths)
- `manager.rs` (8 in `execute_command_internal`, `connect`/`establish_session`,
  `is_connected`, `get_server_info_internal` paths)

All preserved verbatim.

## Step-by-step commits

R13 used a single atomic commit per spec D6 precedent (R5/R6/R7/R8/R10a all used
single commits). Step-by-step sub-steps were executed sequentially but committed
together at the end after all verification passed.

| Step | Action | Files touched | Cargo check status |
|---|---|---|---|
| 1 | Create `manager_handler.rs` (move SSHHandler + HandlerError + 4 impls) | +1 new file, manager.rs -237 lines, mod.rs +1 pub mod | 0 errors |
| 2 | Create `manager_session.rs` (move PTYSession) | +1 new file, manager.rs -85 lines, mod.rs +1 pub mod | 0 errors (1 broken: PTYSession field privacy → added `pub(crate) fn new`) |
| 3 | Create `manager_port_forward.rs` (move PortForward + Direction + Manager + Default) | +1 new file, manager.rs -185 lines, mod.rs +1 pub mod | 0 errors (1 broken: re-export of pub(crate) → omit from mod.rs) |
| 4 | Reduce manager.rs to facade | (already reduced) | 0 errors |
| 5 | Update mod.rs re-exports per spec §2.3 | mod.rs -5 lines, +4 cfg-gated blocks | 0 errors |
| 6 | cargo build --workspace + cross-crate caller grep | (no file changes) | n/a |
| 7 | cargo test services-integrations + northhing-core | (no file changes) | 9 + 899 tests pass |
| 8 | cargo fmt + commit + handoff | + cargo fmt applied to 4 files | 0 errors, fmt clean |

## Per-step line counts (R12a lesson: report per step)

| Step | manager.rs | manager_handler.rs | manager_session.rs | manager_port_forward.rs | mod.rs |
|---|---|---|---|---|---|
| Start (main) | 2810 | — | — | — | 43 |
| After Step 1 | 2573 | 250 | — | — | 44 |
| After Step 2 | 2488 | 250 | 102 | — | 45 |
| After Step 3 | 2303 | 250 | 102 | 190 | 52 |
| After cargo fmt | 2303 | 251 | 103 | 191 | 52 |
| **Final (committed)** | **2303** | **251** | **103** | **191** | **52** |

All new sibling files stay well under the 800-line cap. The facade's 2303 lines is the
documented D-deviation (see above).

## Round 5-R12b lessons applied

- R5 sub-domain split: ✓
- R6 cargo check stop-at-first-error: ✓ (3 parallel checks: services-integrations +
  northhing-core + northhing-tools-execution)
- R6 Cargo.lock drift check: ✓ (Cargo.lock is gitignored, no drift possible since
  no dependency changes; main and worktree share the same workspace)
- R7 turn_internal 4 sub-handlers god method split pattern: applied (1 facade + 3 siblings)
- R8 atomic single commit per D6 precedent: ✓
- R9 cargo test 899/0/1 baseline match: ✓
- R9b test attribute preservation: ✓ (all 6 `#[test]` / `#[tokio::test]` preserved in
  facade's `mod tests` block — none moved)
- R10a mod.rs `pub mod` declarations: ✓ (3 new `pub mod` declarations added with cfg gates)
- R10a 1130 unused imports 教训: ✓ (precise use blocks per sibling, removed unused
  `russh_keys::PublicKeyBase64` from manager_handler.rs)
- R11 mod.rs sub-facade + pub use re-export pattern: ✓
- R11a struct owner mapping: ✓ (spec §1.2 mapping applied)
- R11b Mavis take-over post-error pattern: ✓ (1 error encountered, fixed within R13)
- R11b sub_facade re-export preserves caller paths: ✓ (only 1 cross-crate direct caller;
  all others use mod.rs re-exports which still work)
- R12 god method split (R7 pattern applied to call_impl): deferred to R13c (see D-deviation)
- R12 per-step line count reporting: ✓ (see table above)
- R12b thin facade re-export pattern: applied (facade re-exports KnownHostEntry +
  SSHConnectionManager only; siblings re-export their own types)
- **R12b Set-Content encoding trap avoided**: ✓ (used Write tool, UTF-8 native)

## Cross-crate caller verification

```
$ git -C northing-impl-round13 grep 'use.*remote_ssh::manager' -- 'src/'
src/crates/services/services-integrations/src/remote_ssh/remote_terminal.rs:9:use crate::remote_ssh::manager::SSHConnectionManager;
```

Only 1 cross-crate caller (remote_terminal.rs uses facade `SSHConnectionManager`).
All other 25+ files use `crate::service::remote_ssh::*` which resolves through mod.rs.

`assembly/core/src/service/remote_ssh/manager.rs` (3-line compatibility facade):
```rust
//! Compatibility facade for Remote SSH connection management.

pub use northhing_services_integrations::remote_ssh::manager::*;
```
Still works because `manager.rs` re-exports `KnownHostEntry` + `SSHConnectionManager`.

## Known unknowns (for future review)

1. **Facade 2303 lines**: requires R13c split. Candidates listed above.
2. **R10a pre-existing 1130 unused imports**: not addressed in R13 (out of scope).
3. **`remote_exec.rs` 1195 > 1000 cap**: spec §4 documents this as known D-deviation
   (NOT R13 scope, separate round R13b planned).
4. **Port forwarding is registration-only stub**: pre-existing implementation gap,
   tracked separately, not addressed in R13.

---

*Implementation committed at 3b5f520 on branch `impl/round13-remote-ssh-manager-split`.
Ready for review.*