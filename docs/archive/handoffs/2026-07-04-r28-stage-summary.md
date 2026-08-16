# R28 god-object split — stage summary (terminal/session/manager.rs 1457 → types.rs + session_manager.rs)

> R28 retry: `services/terminal/src/session/manager.rs` (1457 lines,
> 4 type defs + 3 helper fns + 1 struct + 1 large impl + 1 Drop + 1 cfg(test))
> split into types.rs + session_manager.rs, manager.rs deleted.

## Spec

- This run: no formal spec doc (Mavis take-over per user's "继续, 等一会再review" instruction).
- Stage summary (this file) documents the structure.

## Result

| File | Status | Lines | Notes |
|---|---|---|---|
| `manager.rs` | DELETED (moved to trash) | — | No external refs to `session::manager::*` path |
| `types.rs` | NEW | 77 | 4 type defs + impl Default + type alias + minimal imports |
| `session_manager.rs` | NEW | 1393 | imports + const + 3 helper fns + struct + impl + impl Drop + tests |
| `mod.rs` | MODIFIED | 321 | +`mod types; mod session_manager;` + `pub use session_manager::*; pub use types::*;` (wildcard) |

**Total**: 1457 → 1470 (+13 lines from new `use super::types::{...};` + mod.rs wildcards).

## Strategy applied (per `2026-07-02-r28-stage-summary.md` retry strategy)

1. ✅ `pub use types::*;` in mod.rs (replaced explicit list `pub use manager::{X, Y, Z};` with wildcard)
2. ✅ No `pub(super)` on `impl Drop for SessionManager` — `fn drop(&mut self) {}` stays default visibility
3. ✅ Added explicit `use super::types::{...};` at top of session_manager.rs (R28 lesson: don't rely on `use super::*;` chain through mod.rs)
4. ✅ Verified with `cargo test -p terminal-core` (not just `cargo check`) per R28 strategy #4

## Cross-sibling pattern

```rust
// session_manager.rs (top)
use super::types::{
    CommandCompletionReason, CommandExecuteResult, CommandStream, CommandStreamEvent,
    ExecuteOptions,
};
use super::{SessionSource, SessionStatus, TerminalSession};

// session_manager.rs tests (cfg(test) mod)
mod tests {
    use super::compute_stream_output_delta;          // file-level fn in session_manager.rs
    use super::super::types::CommandCompletionReason; // cross-sibling via mod.rs re-export
    // ...
}
```

## 3-axis verify (Mavis)

| Axis | Command | Result |
|---|---|---|
| 1 | `cargo check -p terminal-core` | ✅ 0 errors (5 warnings, all pre-existing in exec/mod.rs) |
| 2 | `cargo check -p northhing-cli` | ✅ 0 errors |
| 3 | `cargo check -p northhing` (desktop) | ✅ 0 errors |
| 4 | `cargo check --workspace` | ✅ 0 errors |
| 5 | `cargo test -p terminal-core` | ✅ 22 unit tests passed, 0 failed (4 new in `session::session_manager::tests` namespace) |
| 6 | `cargo test -p terminal-core --no-run` | ✅ compile clean |

## Cross-crate consumer preservation

External use sites (verified):

```bash
rg --type-add 'rs:*.rs' 'terminal::session::' --no-filename 2>$null
# → 1 hit: SessionSource (in remote_ssh/disabled.rs, was already in mod.rs inline)
# → 0 hits for SessionManager, CommandStreamEvent, ExecuteOptions, etc. (all via mod.rs re-export)
```

All consumers go through `mod.rs`'s `pub use` re-exports (wildcard now). The single external reference to `terminal::session::SessionSource` resolves correctly (it's defined inline in mod.rs and was never in manager.rs).

## Lessons applied (from prior R28 attempt + R27/R27b/R23/R22)

- **R28 retry strategy**: 4-step plan from `2026-07-02-r28-stage-summary.md` — applied verbatim
- **R27 lesson**: avoid `pub(super)` on inherent impl methods (R28 has no inherent impl methods, only Drop)
- **R23 lesson**: tests use explicit `super::super::types::CommandCompletionReason` rather than implicit chain
- **R22 lesson**: no `_impl` suffix needed (single impl SessionManager block, no split)
- **R18 lesson**: long-line tolerance — checked, no new long lines added

## Decisions taken (vs R28 retry strategy verbatim)

1. **Helpers stay in session_manager.rs** (not types.rs) — `compute_stream_output_delta` is used by tests; keeping it in same file as tests avoids cross-sibling test friction. The 2 integration-output helpers (`get_integration_output_snapshot`, `get_post_command_terminal_state`) are used by impl SessionManager and also stay with it.
2. **No facade `manager.rs`** — strategy option, but `pub use types::*;` in mod.rs (R28 step #1) makes facade redundant. Verified 0 external refs to `session::manager::*` path → safe to delete.
3. **Unused imports removed** — `Pin`, `Stream`, `Deserialize`, `Serialize` all moved to types.rs. Removed from session_manager.rs to avoid 3 unused-import warnings.

## Next session suggestion

- User does **end-of-day review** (QClaw + Kimi dual or single) for both R25 + R28
- Likely observations: line cap (1393 vs 800 cap ✓), cross-sibling import pattern correctness, test isolation
- R29 candidates (from 2026-07-03 night handoff):
  - `services/terminal/src/exec.rs` 83k
  - `services/terminal/src/shell/integration.rs` 29k
  - `services/terminal/src/pty/process.rs` 25k
  - Or wait for user to pick from candidates list

## Refs

- R28 retry stage summary (prior, deferred): `docs/handoffs/2026-07-02-r28-stage-summary.md`
- 2026-07-04 session addendum: `docs/handoffs/2026-07-04-session-addendum.md`
- 2026-07-03 night handoff: `docs/handoffs/2026-07-03-night-handoff.md`