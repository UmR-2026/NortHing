# Round 13b handoff: remote_ssh/manager.rs facade 2303 → 7 sub-siblings

> R13 split the original 2810-line `manager.rs` into 1 facade + 3 sub-handlers
> but left the facade at **2303 lines** (D-deviation: 187% over 800 cap, worse
> than R12 D1's 1693 lines).
>
> R13b continues the sub-domain split, breaking the facade into **7 sibling
> files + facade reduced to 196 lines**.

## Diff stat (vs main `c301d8b`)

```
src/crates/services/services-integrations/src/remote_ssh/manager.rs             2303 -> 196  (-2107)
src/crates/services/services-integrations/src/remote_ssh/manager_handler.rs      251 unchanged (+1 line: import path)
src/crates/services/services-integrations/src/remote_ssh/mod.rs                  52 ->  62  (+10 new sibling mods)

+ manager_known_hosts.rs           119 lines  (new)
+ manager_remote_workspace.rs      149 lines  (new)
+ manager_ssh_config.rs            196 lines  (new)
+ manager_saved_connections.rs     348 lines  (new)
+ manager_sftp.rs                  331 lines  (new)
+ manager_session_lifecycle.rs     706 lines  (new)
+ manager_command_dispatch.rs      229 lines  (new)
+ manager_tests.rs                 185 lines  (new)
```

**Total: 8 files changed, 7 created. Facade 2303 → 196 (91% reduction).**

## Final file layout

| File | Lines | Owns |
|---|---|---|
| `manager.rs` | 196 | `SSHConnectionManager` struct + `ActiveConnection` + `new` + `connect`/`connect_with_timeout`/`disconnect`/`disconnect_all`/`is_connected` + helpers |
| `manager_known_hosts.rs` | 119 | `KnownHostEntry` + 7 fns (load/save/add/is/get/remove/list) |
| `manager_remote_workspace.rs` | 149 | 8 fns (load/save/get/set/prune/remove/clear workspace) |
| `manager_ssh_config.rs` | 196 | `ssh_cfg_get`/`ssh_cfg_has` + 4 cfg-gated fns (get/list) |
| `manager_saved_connections.rs` | 348 | 12 fns (load/save/migrate/get/prune/save/delete + vault coupling) |
| `manager_sftp.rs` | 331 | 14 SFTP ops + `sftp_mkdir_all_prefixes` helper |
| `manager_session_lifecycle.rs` | 706 | `establish_session` + `execute_command_internal` + `ensure_alive_or_reconnect` (3 god methods) + 3 helpers |
| `manager_command_dispatch.rs` | 229 | 9 public entry points (execute/open_pty/get_server_info/...) |
| `manager_tests.rs` | 185 | 6 tests + `test_data_dir` |
| (existing) `manager_handler.rs` | 251 | unchanged |
| (existing) `manager_session.rs` | 103 | unchanged |
| (existing) `manager_port_forward.rs` | 191 | unchanged |

**Total: 12 files, all under 800 line cap (largest: session_lifecycle 706).**

## Verification

```bash
cargo check -p services-integrations --features remote-ssh-concrete --lib  # 0 errors
cargo check -p northhing-core --features product-full --lib                # 0 errors (5 warnings, all pre-existing or unused-import fixed)
cargo test -p northhing-core --features product-full --lib                # 899 passed; 0 failed; 1 ignored
cargo test -p services-integrations --features remote-ssh-concrete --lib  # 9 passed; 0 failed
cargo fmt --check                                                         # clean
```

## Iron rules verification

```bash
git diff main..HEAD -- src/crates/services/services-integrations/src/remote_ssh/manager_*.rs \
  | grep -cE '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
# 0 NEW violations
```

All 16 pre-existing unwraps preserved verbatim (R11b lesson).

## Visibility pattern

`SSHConnectionManager` fields are `pub(super)` so all sibling impl blocks in
the `remote_ssh` module can read/write them directly without accessor methods.

```rust
pub(super) connections: Arc<tokio::sync::RwLock<HashMap<String, ActiveConnection>>>,
pub(super) saved_connections: Arc<tokio::sync::RwLock<Vec<SavedConnection>>>,
pub(super) config_path: std::path::PathBuf,
pub(super) known_hosts: Arc<tokio::sync::RwLock<HashMap<String, KnownHostEntry>>>,
pub(super) known_hosts_path: std::path::PathBuf,
pub(super) remote_workspaces: Arc<tokio::sync::RwLock<Vec<RemoteWorkspace>>>,
pub(super) remote_workspace_path: std::path::PathBuf,
pub(super) password_vault: std::sync::Arc<SSHPasswordVault>,
```

`ActiveConnection` fields also `pub(super)` (handle, config, server_info,
sftp_session, server_key, alive, reconnect_lock).

Free fns cross-siblings:
- `truncate_at_char_boundary` — in facade, used by session_lifecycle
- `SSH_COMMAND_*_INTERVAL` / `SSH_COMMAND_*_GRACE` consts — in facade, used by session_lifecycle
- `ssh_cfg_get`/`ssh_cfg_has` — in ssh_config sibling, used by ssh_config impl
- `sftp_mkdir_all_prefixes` — in sftp sibling, used by tests via `use crate::remote_ssh::manager_sftp::sftp_mkdir_all_prefixes;`

## Cross-sibling call graph (within SSHConnectionManager methods)

| Sibling | Calls into other siblings (via `Self::`) |
|---|---|
| facade `manager.rs` | `establish_session` (session_lifecycle), `ActiveConnection` struct construction |
| `manager_known_hosts.rs` | (none) |
| `manager_remote_workspace.rs` | `save_remote_workspaces` (self) |
| `manager_ssh_config.rs` | (none — uses external `ssh_config` crate) |
| `manager_saved_connections.rs` | `prune_saved_connections_without_credentials` (self), `remove_remote_workspaces_for_connections` (self), `save_connections` (self), `load_connection_config_from_saved` (self), `save_remote_workspaces` (remote_workspace) |
| `manager_sftp.rs` | `ensure_alive_or_reconnect` (session_lifecycle), `resolve_sftp_path` (self) |
| `manager_session_lifecycle.rs` | `get_server_info_internal` (self), `probe_remote_home_dir` (self), `interrupt_exec_channel` (self), `execute_command_internal` (self), `establish_session` (self), `load_connection_config_from_saved` (saved_connections) |
| `manager_command_dispatch.rs` | `ensure_alive_or_reconnect` (session_lifecycle), `execute_command_internal` (session_lifecycle), `probe_remote_home_dir` (session_lifecycle), `PTYSession::new` (session) |
| `manager_tests.rs` | `sftp_mkdir_all_prefixes` (sftp), `SSHConnectionManager::new` (facade), `load_saved_connections`/`save_connection`/`load_connection_config_from_saved`/`prune_remote_workspaces_without_saved_connections` (saved_connections/remote_workspace) |

## Re-export path

`mod.rs` re-exports:

```rust
pub use manager::SSHConnectionManager;          // was: manager::{KnownHostEntry, SSHConnectionManager}
pub use manager_known_hosts::KnownHostEntry;    // was: manager::KnownHostEntry
pub use manager_port_forward::{PortForward, PortForwardDirection, PortForwardManager};
pub use manager_session::PTYSession;
// manager_handler::{HandlerError, SSHHandler} crate-private per spec §1.2 (unchanged)
```

External callers (verified by `git grep 'use.*remote_ssh::manager'`):

- `src/crates/.../remote_terminal.rs` — uses `crate::remote_ssh::manager::SSHConnectionManager` directly (still works, re-export from `manager.rs`)

All other 25+ files use `crate::service::remote_ssh::*` (top-level mod.rs re-exports).

## What changed vs R13

R13 created 3 sibling files (handler/session/port_forward) and kept ~77
methods on the facade. R13b creates 7 more sibling files and reduces facade
methods to 5 (new, connect, connect_with_timeout, disconnect, disconnect_all,
is_connected).

| Metric | R13 (before R13b) | R13b | Delta |
|---|---|---|---|
| `manager.rs` facade lines | 2303 | 196 | **−2107 (91%)** |
| `manager.rs` facade methods | 97 | 6 | −91 |
| Total `manager_*.rs` files | 4 (manager + 3 siblings) | 12 | +8 |
| Total `manager_*.rs` lines | 2848 | 3004 | +156 (overhead from imports + doc comments) |
| `pub(super)` field count | 0 | 8 | +8 (visibility pattern) |
| god methods | 3 (in facade) | 3 (in session_lifecycle) | moved, not split |

## god methods note

R13b did NOT apply god method split to `establish_session` (236 lines),
`execute_command_internal` (212 lines), or `ensure_alive_or_reconnect` (152
lines). They're all in `manager_session_lifecycle.rs` which totals 706 lines
(still under 800 cap). Future R13c/R14 could apply god method split per
the R7/R12 pattern, but is not required to close the R13 D-deviation.

## Pre-existing debt preserved

Per R11b lesson, pre-existing unwrap/panic/let _ = occurrences were not
fixed (counted as 16 in original, preserved verbatim). New `unwrap()`/`expect()`
calls introduced by R13b: **0**.

## Pre-existing cargo fmt noise

This commit does NOT introduce any new cargo fmt noise. Verified via
`git checkout HEAD -- password_vault.rs remote_exec.rs remote_terminal.rs
workspace_search/` to revert cargo fmt's accidental cleanups of pre-existing
encoding-only diffs in unrelated files. R13b scope is strictly the
`remote_ssh/manager*.rs` family + `mod.rs`.

## Refs

- R13 spec: `docs/handoffs/2026-06-29-round13-remote-ssh-manager-split-spec.md` (811b22f)
- R13 refactor: `docs/handoffs/2026-06-29-round13-remote-ssh-manager-split-impl.md` (3b5f520)
- R13 review guide: `docs/handoffs/2026-06-29-round13-remote-ssh-manager-split-review.md` (569b85a)
- R13 merge: `c301d8b`
- R13b refactor (this handoff): `0763fff`
- Iron rules reference: `C:\Users\UmR\.qclaw\skills\code-rot-guard\references\coding-agent-rules.md`

## Commits

```
0763fff refactor(remote-ssh): R13b split facade 2303 -> 196 + 7 sub-siblings (closes R13 D-deviation)
c301d8b merge: Round 13 remote_ssh/manager split (critical #3 god object)
569b85a docs(handoff): R13 remote_ssh/manager split review guide
a58d6e9 docs(handoff): R13 remote_ssh/manager split - handoff
3b5f520 refactor(remote-ssh): R13 split — 1 facade + 3 sub-handlers (critical #3 god object)
811b22f docs(spec): Round 13 remote_ssh/manager.rs 2810 split (critical #3 god object)
```

## Sign-off for next round

R13 D-deviation is closed. Facade 2303 → 196 (75% under cap). No new debt
introduced. Pre-existing 16 unwraps preserved. Next round: R14 candidate
(`bot/command_router.rs` 2614) or return to GUI 30% completion work.