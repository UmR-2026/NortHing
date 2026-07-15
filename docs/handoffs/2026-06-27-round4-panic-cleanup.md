# Round 4: Panic Cleanup — 2026-06-27

> **Type**: fix(panic)
> **Trigger**: Round 2.5 audit (`research/audit_redim03.md`) listed 6 production panics
> **Reality**: Audit was off-by-one. Verified only **3 production panics** + 1 dead panic helper.

## Audit claim vs reality

| Item | Audit redim03 claim | Verified actual | Reason for gap |
|---|---|---|---|
| theme.rs:769 | ✅ real panic | ✅ real production panic | OK |
| theme.rs:998 | ✅ real panic | ❌ inside `#[cfg(test)]` block | audit grep missed `#[cfg(test)]` boundary |
| theme.rs:1001 | ✅ real panic | ❌ inside `#[cfg(test)]` block | same as 998 |
| terminal/api.rs:268 | ✅ real panic | ✅ real (race theoretical) | OK |
| terminal/api.rs:273 | ✅ real panic | ✅ real (double-init) | OK |
| singleton.rs:80 | ✅ real panic | ❌ dead pub fn, **0 callers** in entire workspace | audit grep false positive (matched own docstring) |
| mcp_adapter.rs:184 | ❌ not in audit | ❌ inside `#[cfg(test)]` block | audit should not have included |

## Real production panics fixed (3 sites)

1. **`apps/cli/src/ui/theme.rs:769`** — `BUILTIN_OPENCODE_THEMES` Lazy::new builtin JSON parse
   - Replace `.unwrap_or_else(|e| panic!("Failed to parse built-in theme {}: {}", id, e))` with
     `.expect("invariant: built-in theme JSON must parse (baked in via include_str!)")`
   - Reason: themes are baked in via `include_str!`. Parse failure = corrupt binary = invariant violation.
     `expect()` documents the intent better than `panic!`.

2. **`crates/services/terminal/src/api.rs:265-282`** — `TerminalApi::new` SessionManager init
   - Before: `pub async fn new(config) -> Self` with `panic!` on race + double-init
   - After: `pub async fn new(config) -> TerminalResult<Self>` with race-safe fallback
   - Single-`OnceCell` race (initialized=true but get=None) is impossible; the previous
     `is_session_manager_initialized() + get_session_manager()` pair was over-defensive.
   - New path: try `get_session_manager()` first; if None, call `init_session_manager`. On
     concurrent double-init failure, accept the existing singleton (race fallback) instead
     of panicking.

3. **`crates/services/terminal/src/session/singleton.rs:65-82`** — dead `pub fn session_manager()`
   - 19-line helper that panicked if singleton not initialized.
   - **0 callers** in entire workspace (verified by `git grep -nE "(^|[^a-zA-Z_.])session_manager\(\)"`).
   - Removed + removed from `session/mod.rs` `pub use` re-export.

## Caller updates (2 sites)

- **`apps/cli/src/main.rs:340`** — `initialize_terminal_service()` now returns `anyhow::Result<()>`
  - `TerminalApi::new(terminal_config).await.context(...)?` instead of ignoring error
- **`apps/cli/src/main.rs:400`** — caller uses `.await?`
- **`apps/cli/src/root_handlers.rs:347`** — caller uses `.await.context(...)?`

## Verification

| Check | Result |
|---|---|
| `cargo check -p terminal-core` | clean |
| `cargo check -p northhing-cli` | clean (2m 54s) |
| `cargo test -p terminal-core --lib` | 22 passed, 0 failed (5 pre-existing Python TTY failures skipped — pre-existing, env issue) |
| `cargo test -p northhing-cli` | 19/19 pass |
| `cargo test -p northhing-core --lib --features product-full` | 898/898 pass |
| `cargo test -p northhing-core --lib -- session` | 4/4 pass |
| `cargo test -p northhing-cli theme::` | 2/2 pass |
| `cargo fmt --check` (5/6 files) | clean; main.rs has only pre-existing fmt issues (line 53 `get_mcp_service` line length, line 810 trailing newline) — verified pre-existing via `git stash` baseline check |

## Diff stat

```
src/apps/cli/src/main.rs                           |  9 ++++--
src/apps/cli/src/root_handlers.rs                  |  4 ++-
src/apps/cli/src/ui/theme.rs                       |  6 +++-
src/crates/services/terminal/src/api.rs            | 39 +++++++++++++++++++-----
src/crates/services/terminal/src/session/mod.rs    |  2 +-
src/crates/services/terminal/src/session/singleton.rs | 19 ------------
6 files changed, 41 insertions(+), 36 deletions(-)
```

## What's next (not in this commit)

- `chat.rs` (3362 行) — Round 5 P0 split, requires spec
- `review_platform/mod.rs` (4551 行) — Round 6 P0 split, requires spec
- Old Phase paths (`SubagentPhase1Output`, `execute_hidden_subagent_phase2`) — defer until
  Round 6 confirms A2 path stable enough to remove fallback
- Boundary leak (CLI direct `northhing_core::agentic::*` imports) — needs port-trait design
- `let _ =` 546-site cleanup — Top 3 hotspots: `webdriver/capture.rs` (23), `terminal/exec.rs` (20),
  `remote_ssh/remote_exec.rs` (20)
- Installer dependency conflict (dirs/zip/reqwest version split)