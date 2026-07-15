# Round 13b review guide: remote_ssh/manager.rs facade 2303 → 196 + 7 sub-siblings

> Reviewer (QClaw / Kimi): please review commit `0763fff` (refactor) + `0f2c4c1` (handoff)
> on branch `impl/round13b-manager-facade-split`. Handoff doc:
> `docs/handoffs/2026-06-29-round13b-remote-ssh-manager-facade-split-impl.md`.
>
> This is a **mechanical follow-up** to Round 13 that closes the R13 D-deviation
> flagged in the R13 review guide (facade 2303 → 196). No spec file was written
> per the R12b precedent (when scope is mechanical closure of a known D-deviation).

## What to review

| File | Lines | Note |
|---|---|---|
| `src/crates/services/services-integrations/src/remote_ssh/manager.rs` (reduced 2303 → **196**) | 196 | facade: SSHConnectionManager + ActiveConnection + new + connect lifecycle + 5 fns |
| `manager_known_hosts.rs` (new) | 119 | KnownHostEntry + 7 CRUD fns |
| `manager_remote_workspace.rs` (new) | 149 | 8 workspace persistence fns |
| `manager_ssh_config.rs` (new) | 196 | ssh_cfg_get/has + 4 cfg-gated fns |
| `manager_saved_connections.rs` (new) | 348 | 12 saved profile + vault fns |
| `manager_sftp.rs` (new) | 331 | 14 SFTP ops + sftp_mkdir_all_prefixes helper |
| `manager_session_lifecycle.rs` (new) | 706 | establish_session + execute_command_internal + ensure_alive_or_reconnect (3 god methods preserved) + 3 helpers |
| `manager_command_dispatch.rs` (new) | 229 | 9 public entry points (execute/open_pty/get_server_info/...) |
| `manager_tests.rs` (new) | 185 | 6 #[cfg(test)] unit tests + test_data_dir |
| `manager_handler.rs` (+1 line: import path update) | 252 | SSHHandler now imports KnownHostEntry from manager_known_hosts |
| `mod.rs` | 62 | +7 new pub mod declarations + re-exports updated |

## Critical observations (please verify)

### 1. R13 D-deviation **CLOSED** ✅

R13 review flagged facade at **2303 lines** (187% over 800 cap, "worse than R12 D1 at 169%"). R13b brings facade to **196 lines** (75% UNDER cap). The closure is mechanical (no god method split, no behavior change) and is the primary success criterion for this round.

### 2. Visibility pattern — `pub(super)` on struct fields

`SSHConnectionManager` fields are now `pub(super)` instead of private, so all 7 sibling `impl SSHConnectionManager { ... }` blocks can read/write them directly without accessor methods.

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

`ActiveConnection` fields also `pub(super)`.

**Verify**: No field is `pub` (no leakage outside `remote_ssh` module). No field is fully private (siblings wouldn't compile). All access is mediated via the `pub(super)` boundary.

### 3. Multiple `impl SSHConnectionManager` blocks

7 separate files each contain `impl SSHConnectionManager { ... }` blocks. Rust allows this within the same crate (methods don't conflict because each is defined once per file). Verify by:

```bash
grep -c "^impl SSHConnectionManager" src/crates/services/services-integrations/src/remote_ssh/manager*.rs
# Expect: 9 (1 per file: manager.rs + 7 new siblings + manager_handler.rs)
```

### 4. Cross-sibling `Self::` calls (no cyclic deps)

`manager_session_lifecycle::establish_session` is called by:
- facade `connect_with_timeout` (via `Self::establish_session`)
- `manager_saved_connections::ensure_alive_or_reconnect` does NOT exist there; it's in `manager_session_lifecycle`
- `manager_command_dispatch::execute_command_with_options` (via `ensure_alive_or_reconnect` → `establish_session`)
- `manager_sftp::get_sftp` (via `ensure_alive_or_reconnect`)

Verify: No two siblings call each other in a cycle (no `A → B → A` patterns). All edges go through the facade struct + free fns.

### 5. god methods preserved (not split)

R13b did NOT apply god method split to:
- `establish_session` (236 lines)
- `execute_command_internal` (212 lines)
- `ensure_alive_or_reconnect` (152 lines)

These all sit in `manager_session_lifecycle.rs` (706 lines total, under 800 cap). Future R13c could apply R7/R12 god method split, but is **not required** to close R13 D-deviation.

**Question for reviewer**: should R13c be required like R12b was, or accepted as is?

### 6. Iron rules verification

```bash
git diff main..HEAD -- src/crates/services/services-integrations/src/remote_ssh/manager_*.rs \
  | grep -cE '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
# Expect: 0 NEW violations

git diff main..HEAD -- src/crates/services/services-integrations/src/remote_ssh/manager_*.rs \
  | grep -cE '^\+.*let _ = .*Result'
# Expect: 0 NEW let _ = Result
```

All 16 pre-existing unwraps preserved verbatim (R11b lesson — pre-existing debt is not "fixed" by refactors).

### 7. Cargo verification

```bash
cargo check -p services-integrations --features remote-ssh-concrete --lib  # 0 errors
cargo check -p northhing-core --features product-full --lib                # 0 errors (3 pre-existing mcprmcp warnings + 0 NEW)
cargo test -p northhing-core --features product-full --lib                # 899 passed; 0 failed; 1 ignored
cargo test -p services-integrations --features remote-ssh-concrete --lib  # 9 passed; 0 failed (6 manager_tests + 3 pre-existing)
cargo fmt --check                                                         # clean
```

All verified pre-merge.

### 8. External API unchanged

Cross-crate callers (verified `git grep 'use.*remote_ssh::manager'`):
- `src/crates/.../remote_terminal.rs` — uses `crate::remote_ssh::manager::SSHConnectionManager` directly (still works; re-export from `manager.rs`).

`mod.rs` re-exports preserve backward compatibility:
```rust
pub use manager::SSHConnectionManager;          // unchanged source path
pub use manager_known_hosts::KnownHostEntry;    // was manager::KnownHostEntry (R13)
```

**25+ other files** use `crate::service::remote_ssh::*` (top-level mod.rs re-exports) — unaffected.

### 9. Pre-existing cargo fmt noise NOT introduced

R13b commit does NOT introduce any new cargo fmt noise. Earlier R13b work accidentally triggered `cargo fmt` cleanups of pre-existing encoding diffs in `password_vault.rs`, `remote_exec.rs`, `remote_terminal.rs`, `workspace_search/mod.rs`, `workspace_search/service.rs` — those were reverted via `git checkout HEAD -- ...` and excluded from the R13b commit.

**Verify**: `git show 0763fff --stat | grep -E 'password_vault|remote_exec|remote_terminal|workspace_search'` → should be empty.

### 10. Test path

6 tests moved from `mod tests` in facade to `manager_tests::tests`:
- `prunes_password_connection_without_vault_entry`
- `rejects_saving_password_connection_without_password`
- `restores_connection_config_from_saved_password_profile`
- `prunes_remote_workspaces_without_saved_connection`
- `mkdir_all_prefixes_expand_absolute_posix_path`
- `mkdir_all_prefixes_collapse_redundant_separators`

`test_data_dir` helper moved with them. `sftp_mkdir_all_prefixes` import updated to `crate::remote_ssh::manager_sftp::sftp_mkdir_all_prefixes`.

**Question for reviewer**: any concern about moving tests from facade to dedicated file (test behavior unchanged, just relocated)?

## Refs

- R13 spec: `811b22f`
- R13 refactor: `3b5f520`
- R13 handoff: `a58d6e9`
- R13 review guide: `docs/handoffs/2026-06-29-round13-remote-ssh-manager-split-review.md` (569b85a)
- R13 review report (pending QClaw review)
- **R13b refactor: `0763fff`** ← this round
- **R13b handoff: `0f2c4c1`** ← this round
- **R13b merge: `e52f598`** ← this round
- Iron rules reference: `C:\Users\UmR\.qclaw\skills\code-rot-guard\references\coding-agent-rules.md`

## Questions for reviewer

1. **APPROVE / REJECT** decision with score (1-10)
2. List of any **minor observations** (non-blocking)
3. **R13c necessity**: facade 196 lines is now 75% UNDER cap. Should R13c (god method split on `establish_session` / `execute_command_internal` / `ensure_alive_or_reconnect`) be required, or accepted as is?
4. **Visibility pattern**: `pub(super)` on 8 struct fields — does this match project convention or should it be accessor methods?
5. **Cross-sibling `Self::` calls**: any concern about call graph complexity (5 sibling-to-sibling edges via facade methods)?
6. **Test relocation**: moving 6 tests from facade `mod tests` to `manager_tests.rs` — acceptable?
7. **Re-export path**: `KnownHostEntry` now sourced from `manager_known_hosts` instead of `manager` — does this break any external caller (verified no, but please confirm)?

## Sign-off request

Please provide:
1. **APPROVE / REJECT** decision with score (1-10)
2. List of any **minor observations** (non-blocking)
3. Confirmation of R13c necessity decision (required or accepted)
4. Any structural concerns about the 7-sibling + facade layout
5. Recommendation for R14 scope (`bot/command_router.rs` 2614 vs R13c god method split vs other)

Reply format: standard project review report ending in
`*-review-report.md` (will be committed by reviewer per established pattern).