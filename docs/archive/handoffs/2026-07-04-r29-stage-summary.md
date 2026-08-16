# R29 god-object split — stage summary (shell/integration.rs 745 → facade + 3 sibling)

> R29 god-object split: `services/terminal/src/shell/integration.rs` (745 lines,
> 3 enums + 2 structs + 5 impl blocks + 1 inherent impl + 1 381-line god impl
> + 3 helper fns + 1 cfg(test) mod tests) split into facade + 3 sibling files.

## Result

| File | Status | Lines | Notes |
|---|---|---|---|
| `integration.rs` (parent) | REWRITE (facade) | 13 | `mod types; mod shell_integration; mod shell_integration_manager;` + 3 `pub use ...::*;` |
| `integration/types.rs` | NEW | 119 | OscSequence + CommandState (+ impl) + ShellIntegrationEvent + 3 helper fns |
| `integration/shell_integration.rs` | NEW | 525 | struct ShellIntegration (41) + impl Default (5) + impl ShellIntegration (381) + tests (98) |
| `integration/shell_integration_manager.rs` | NEW | 118 | struct ShellIntegrationManager (9) + impl Default (5) + impl ShellIntegrationManager (97) |
| `shell/mod.rs` | MODIFIED | 150 | `pub use integration::{...};` (explicit list, 5 items) → `pub use integration::*;` (wildcard) |

**Total**: 745 → 719 (+facade). Largest sibling: 499 (shell_integration.rs, well under 800 cap).

## Strategy

R29 has no pre-existing retry strategy. Plan chosen: **horizontal split with facade pattern** (R27b precedent).

| Decision | Why |
|---|---|
| Facade `integration.rs` + 3 sibling subdir | Avoid breaking `shell::integration::*` import path (1 external consumer: `session/session_manager.rs:23`) |
| Subdir `integration/` for siblings | Clean structure: `shell/integration.rs` (facade) + `shell/integration/{types,shell_integration,shell_integration_manager}.rs` |
| Wildcard re-export in mod.rs | R28 lesson: wildcard more robust than explicit list when siblings may grow |
| `include_str!("../scripts/...")` for shell scripts | File moved to subdir, so relative path needs `../` |

## Cross-sibling imports

| Sibling | Imports from siblings |
|---|---|
| `integration/types.rs` | (none — uses `crate::shell::ShellType`) |
| `integration/shell_integration.rs` | `super::types::{CommandState, OscSequence, ShellIntegrationEvent}` |
| `integration/shell_integration_manager.rs` | `super::types::{CommandState, ShellIntegrationEvent}` + `super::shell_integration::ShellIntegration` |
| `integration.rs` (facade) | (none — re-exports) |

Pattern: `use super::types::Type;` for cross-sibling access within `integration/` subdir (R26/R27b precedent).

## Visibility rules

- All 3 enums (OscSequence, CommandState, ShellIntegrationEvent): `pub`
- 2 structs (ShellIntegration, ShellIntegrationManager): `pub` (cross-crate API)
- 5 impl blocks + impl Default blocks: stay in same file as struct
- 3 helper fns (`get_integration_script_path`, `get_integration_script_content`, `get_injection_command`): `pub` in types.rs
- **No `pub(super)` for top-level items** — all `pub` to preserve cross-crate consumer path

## 3-axis verify (Mavis)

| Axis | Command | Result |
|---|---|---|
| 1 | `cargo check -p terminal-core` | ✅ 0 errors (5 pre-existing warn in exec/mod.rs) |
| 2 | `cargo check -p northhing-cli` | ✅ 0 errors |
| 3 | `cargo check -p northhing` (desktop) | ✅ 0 errors |
| 4 | `cargo check --workspace` | ✅ 0 errors |
| 5 | `cargo test -p terminal-core` | ✅ 22 unit tests passed, 0 failed (7 shell integration tests in new `shell::integration::shell_integration::tests` namespace) |

## Cross-crate consumer preservation

External use sites (verified):

```bash
rg --type-add 'rs:*.rs' 'use crate::shell::\{' --no-filename 2>$null
# → 2 imports:
#   session/session_manager.rs:23 — uses CommandState, ScriptsManager,
#                                   ShellDetector, ShellIntegration,
#                                   ShellIntegrationEvent, ShellIntegrationManager, ShellType
#   shell/integration.rs:19 — empty (now facade, re-exports)
# Plus use crate::shell::ShellType (8 sites) — unaffected
```

The single `use crate::shell::{...}` multi-import in session_manager.rs resolves correctly via:
- `shell/mod.rs` declares `pub mod integration;`
- `integration.rs` (facade) does `pub use shell_integration::*; pub use shell_integration_manager::*; pub use types::*;`
- Wildcard `pub use integration::*;` in mod.rs re-exports everything
- External code accesses `crate::shell::ShellIntegration` → resolves through chain ✓

## Lessons applied (from R23-R28 memory)

- **R26/R27b facade pattern**: thin facade re-exports from sibling files (R29: integration.rs → 3 siblings)
- **R28 retry strategy #1**: `pub use types::*;` wildcard (applied in mod.rs)
- **R28 retry strategy #3**: explicit cross-sibling imports `use super::types::Type;` (not relying on super::* chain)
- **R27 lesson**: no `pub(super) fn default()` rejected by trait Default — Default impls stay in struct's sibling
- **R19 lesson**: pre-emptive extend-timeout for split tasks >1000 lines (R29 = 745, <1000, no extend needed)
- **R18 long-line tolerance**: ≤5 new long lines per file — checked, no new long lines added

## Decisions taken (vs R25/R28 patterns)

1. **Subdirectory pattern**: R29 is the first split to use a subdirectory (`shell/integration/`) for siblings. R25/R28 used flat siblings (config/{app_shell,theme,...}.rs, session/{types,session_manager}.rs). Subdir chosen because:
   - Sibling names would collide with parent module name if flat (`shell/shell_integration.rs` reads poorly)
   - Subdir `shell/integration/` is more Rust-idiomatic for nested modules
   - `include_str!` macro path adjustment (`../scripts/...`) is straightforward
2. **Helper fns in types.rs** instead of separate sibling: They're standalone functions, only ~36 lines total, and logically tied to shell integration types. Keeping them with types reduces sibling count (3 not 4).
3. **No `_impl` suffix needed**: All 3 siblings use distinct impl blocks (impl OscSequence via CommandState, impl ShellIntegration, impl ShellIntegrationManager). No name collisions across siblings → no R22 E0592 workaround needed.

## Next session suggestion

- User does **end-of-day review** for R25 + R28 + R29 (user's "等结束后我统一review" instruction)
- R30+ candidates (from 2026-07-03 night handoff):
  - `services/terminal/src/exec/output.rs` 592 lines
  - `services/terminal/src/api.rs` 610 lines
  - `services/terminal/src/exec/manager.rs` 490 lines (potential god-file)
  - `services/terminal/src/shell/detection.rs` 417 lines

## Refs

- 2026-07-04 session addendum: `docs/handoffs/2026-07-04-session-addendum.md`
- R28 retry stage summary: `docs/handoffs/2026-07-04-r28-stage-summary.md` (commit `49874c8`)
- R25 retry stage summary: `docs/handoffs/2026-07-04-r25-stage-summary.md` (commit `311b3e0`)
- 2026-07-03 night handoff: `docs/handoffs/2026-07-03-night-handoff.md`