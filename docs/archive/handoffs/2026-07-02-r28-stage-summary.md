# R28 attempt — `terminal/session/manager.rs` 1457 lines — DEFERRED

> R28 god-object split attempted on
> `services/terminal/src/session/manager.rs` (1457 lines, ~30 struct/enum +
> 5 impl + 1 free fn). **Split deferred** after horizontal-split attempt
> produced 105 errors due to use-block + Drop trait visibility + cross-sibling
> type visibility issues.

## Why deferred

Horizontal split (impl+struct same sibling for private field access) had
multiple compounding issues:

1. **Original use block** (29 lines) needs to be in BOTH siblings — but
   copy-paste creates duplicate use warnings; using `use super::*;` from
   mod.rs re-export can lose some items.
2. **Drop trait** `impl Drop for SessionManager` has `fn drop(&mut self)`
   which doesn't allow `pub(super)` visibility qualifier — needs to be
   stripped.
3. **Cross-sibling types** like `CommandStreamEvent`, `CommandExecuteResult`,
   `ExecuteOptions`, `CommandStream` defined in types.rs sibling — but
   `use super::*;` from session_manager.rs goes to mod.rs which has explicit
   `pub use types::{...}` list. Should work, but actual errors say "cannot
   find type" — module system visibility issue.
4. **Dangling `///` doc comment** at end of types.rs:146 (off-by-one in range).
5. **`#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]`**
   got applied to `pub struct SessionManager` accidentally (R28 script
   added derive that wasn't there) — `Copy` trait conflicts with `Arc` fields.

## Reverted

First extraction + 3 take-over attempts reverted to baseline. lib.rs
back to 1457 lines, terminal-core crate compiles clean.

## Lessons for R28 retry / R29+

- **Horizontal split is harder than sub-domain split** when the use block
  is large and includes crate-level imports (e.g., `use crate::config::*`).
- **Drop trait method** `fn drop(&mut self)` rejects `pub(super)` visibility
  — strip the qualifier for `impl Drop` blocks.
- **Module re-export pattern**: `use super::*;` from sibling goes to parent
  (mod.rs). If mod.rs has `pub use types::{X, Y, Z}` (explicit list), only
  those listed are visible. To get ALL items from types sibling, either
  use `pub use types::*;` in mod.rs or `use super::types::*;` directly in
  the consumer sibling.
- **Test mod** in the facade `manager.rs` needs to use explicit `use super::*;`
  to glob-import from mod.rs re-exports.

## Suggested R28 retry strategy

- Use `pub use types::*;` in mod.rs (replace explicit list with wildcard)
- Strip `pub(super)` from `impl Drop` blocks
- Add explicit `use super::types::{...}` in session_manager.rs (don't rely
  on `use super::*;` chain through mod.rs)
- Verify with `cargo test -p terminal-core` (not just `cargo check`)

## Refs

- R27 stage summary: `docs/handoffs/2026-07-02-r27-stage-summary.md`
- R26 stage summary: `docs/handoffs/2026-07-02-r26-stage-summary.md`
- R25 stage summary (deferred): `docs/handoffs/2026-07-02-r25-stage-summary.md`