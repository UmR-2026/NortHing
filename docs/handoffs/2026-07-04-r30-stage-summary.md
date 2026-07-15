# R30 god-object split — stage summary (exec/manager.rs 490 → facade + 4 sibling in subdir)

> R30 god-object split: `services/terminal/src/exec/manager.rs` (490 lines,
> 1 inherent 391-line `impl ExecProcessManager` + 1 `impl Drop for ExecProcess`
> + 1 75-line `impl ExecProcess`) split into facade + 4 sibling files
> in `manager/` subdirectory.

## Result

| File | Status | Lines | Notes |
|---|---|---|---|
| `exec/manager.rs` (parent) | REWRITE (facade) | 67 | `mod` declarations + thin `impl ExecProcessManager` with 6 public delegate methods |
| `exec/manager/command_exec.rs` | NEW | 67 | `exec_command_inner_impl` free fn (called by facade `exec_command` / `exec_command_streaming`) |
| `exec/manager/stdin.rs` | NEW | 138 | `write_stdin_inner_impl` + `send_stdin_impl` free fns |
| `exec/manager/control_session.rs` | NEW | 215 | `control_session_impl` + 5 session map helpers (store_session, remove_session, update_session_cursor, store_completed_session, take_completed_session) + `emit_lifecycle` re-export |
| `exec/manager/exec_process.rs` | NEW | 92 | `impl Drop for ExecProcess` + `impl ExecProcess` (mark_out_of_band_control, write_input_bytes, request_control, request_terminate, terminate, close_windows_pipe_job) |

**Total**: 490 → 579 (+89 from facade delegate boilerplate + 4 mod declarations). All siblings ≤ 215 (well under 800 cap).

## Strategy

R30 has no pre-existing retry strategy. Plan chosen: **sub-domain split with facade delegate pattern** (R22 precedent).

| Decision | Why |
|---|---|
| Facade `manager.rs` (thin) + 4 sibling subdir | Largest single impl ExecProcessManager was 391 lines; user profile "质量 > 紧凑度" preference for more files |
| Subdir `manager/` for siblings | Same pattern as R29 (`shell/integration/`); parent module name `manager` collides with flat sibling naming |
| Free functions (`_impl` suffix) + facade delegates | Rust E0592 forbids same-name inherent methods across multiple `impl X { ... }` blocks; free fns with `pub(super)` accessibility work cleanly without `_impl` suffix on inherent methods |
| `use self::sibling::...` in facade | `super::sibling::...` resolves to parent's sibling (wrong); `self::sibling::...` resolves to current module's child (correct) |

## Cross-sibling pattern

```rust
// facade manager.rs
mod command_exec;
mod control_session;
mod exec_process;
mod stdin;

use self::command_exec::exec_command_inner_impl;
use self::control_session::control_session_impl;
use self::stdin::{send_stdin_impl, write_stdin_inner_impl};

impl ExecProcessManager {
    pub async fn exec_command(&self, request: ExecCommandRequest)
        -> TerminalResult<ExecCommandResponse>
    {
        exec_command_inner_impl(self, request, None).await
    }
    // ... 5 more delegates
}

// sibling command_exec.rs
use super::control_session::{
    remove_session_impl, store_session_impl, update_session_cursor_impl,
};

pub(super) async fn exec_command_inner_impl(
    mgr: &ExecProcessManager,
    request: ExecCommandRequest,
    output_tx: Option<mpsc::Sender<String>>,
) -> TerminalResult<ExecCommandResponse> {
    // ... body uses `mgr.store_session(...)` style calls (delegated to control_session free fns)
}
```

## Visibility rules

- All struct types: stay in `types.rs` (already there)
- All impl methods: stay in their sub-domain sibling (no `pub(super)` on inherent methods)
- Free functions (`_impl`): `pub(super)` to allow sibling access
- `impl ExecProcess` methods: `pub(super)` (so manager siblings can call `process.write_input_bytes` etc.)
- All public API surface preserved via facade's thin `impl ExecProcessManager`

## 3-axis verify (Mavis)

| Axis | Command | Result |
|---|---|---|
| 1 | `cargo check -p terminal-core` | ✅ 0 errors (1 unused import warn in control_session.rs, 5 pre-existing in mod.rs) |
| 2 | `cargo check -p northhing-cli` | ✅ 0 errors |
| 3 | `cargo check -p northhing` (desktop) | ✅ 0 errors |
| 4 | `cargo check --workspace` | ✅ 0 errors |
| 5 | `cargo test -p terminal-core` | ✅ 22 unit tests passed, 0 failed (no new tests; behavior preservation verified by existing test pass) |

## Cross-crate consumer preservation

External use sites (verified):

```bash
rg --type-add 'rs:*.rs' 'ExecProcessManager' --no-filename 2>$null
# → All references go through `crate::exec::ExecProcessManager` (re-exported by mod.rs line 14)
# → Manager's public methods (exec_command, write_stdin, send_stdin, control_session, etc.) preserved
# → No external code changes needed
```

The facade's `impl ExecProcessManager` re-exposes the same 6 public methods with identical signatures, so all external call sites (`crate::exec::ExecProcessManager::exec_command(...)` etc.) work without modification.

## Lessons applied (from R22-R29 memory)

- **R22 lesson**: facade delegate pattern with `_impl` free functions (used here as `_impl` suffix, no inherent method `_impl` suffix needed)
- **R27b lesson**: `pub(super)` on private fields when splitting impl across siblings (types.rs already promoted fields to `pub(crate)` per R22e)
- **R29 lesson**: subdirectory pattern for siblings (`manager/` subdir, like `shell/integration/`)
- **R28 retry strategy #3**: explicit cross-sibling imports (not relying on super::* chain)
- **R18 long-line tolerance**: ≤5 new long lines per file — checked, no new long lines added
- **R23 lesson**: no method dropped from original — verified all 6 public methods + 5 private helpers preserved

## Decisions taken (vs R29 patterns)

1. **Sub-domain split with facade delegate** instead of R29's "types + struct + impl" pattern. R30's `impl ExecProcessManager` has 6 methods split into 4 sub-domains; R29's `impl ShellIntegration` was 1 contiguous block, so 1 sibling sufficed.
2. **Free functions over inherent impl blocks** because Rust E0592 forbids same-name inherent methods across 2 `impl X { ... }` blocks in same crate. Using `pub(super) fn name_impl(mgr: &ExecProcessManager, ...)` instead of `impl ExecProcessManager { pub(super) fn name_impl(...) }`.
3. **Helpers (store_session, remove_session, etc.) stay in `control_session.rs`** rather than a separate `helpers.rs` — they're used by control_session and other sub-domains; centralizing in the most-affected sibling reduces cross-file coupling.

## Next session suggestion

- User does **end-of-day review** for R25 + R28 + R29 + R30 (alltogether)
- R31 candidates (from 2026-07-03 night handoff):
  - `services/terminal/src/api.rs` 610 lines (14 DTOs, god DTO pattern similar to R25)
  - `services/terminal/src/exec/output.rs` 592 lines (1 large impl OutputState)
  - `services/terminal/src/shell/detection.rs` 417 lines (1 large impl ShellDetector)

## Refs

- 2026-07-04 session addendum: `docs/handoffs/2026-07-04-session-addendum.md`
- R29 retry stage summary: `docs/handoffs/2026-07-04-r29-stage-summary.md` (commit `ad0bdb9`)
- R28 retry stage summary: `docs/handoffs/2026-07-04-r28-stage-summary.md` (commit `49874c8`)
- R25 retry stage summary: `docs/handoffs/2026-07-04-r25-stage-summary.md` (commit `311b3e0`)
- 2026-07-03 night handoff: `docs/handoffs/2026-07-03-night-handoff.md`