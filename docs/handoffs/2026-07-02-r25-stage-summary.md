# R25 attempt — `service/config/types.rs` 2404 lines — DEFERRED

> R25 god-object split attempted on `assembly/core/src/service/config/types.rs`
> (2404 lines, ~47 struct/enum + 28 impl Default + 1 free fn + 1 trait +
> 2 cfg(test) blocks). **Split deferred** after first attempt produced
> 232 errors due to high cross-referencing.

## Why deferred

config/types.rs has unusual cross-reference density:
- 30+ struct fields reference other types (e.g., `AppConfig` references
  `ProjectConfig`, `ThemeConfig`, `EditorConfig`, `TerminalConfig`, etc.)
- 28 impl Default blocks scattered throughout (not contiguous with
  their struct)
- 1 free fn `deserialize_agent_profiles` used by `AgentProfileConfig`
- 1 trait `ConfigProvider` with 10+ impl blocks in `providers.rs`
- External crates import via `use crate::service::config::types::XXConfig;`
  (~30 import sites across the codebase)

First attempt strategy: 5 sibling sub-domain split (theme/editor/ai/runtime/events).
Extracted 47 struct/enum + 28 impl Default successfully. Failed at:
- Cross-sibling type references (e.g., `theme.rs` ProjectConfig needs
  `EditorConfig`, `TerminalConfig`, `WorkspaceConfig`, `AIConfig`)
- Free fn `deserialize_agent_profiles` not assigned to any sibling
- `ConfigProvider` trait not in any sibling (lives in providers.rs)
- External imports `use crate::service::config::types::XXConfig` need
  re-exports in service.rs

## Reverted

First extraction produced 232 errors. Reverted to baseline (commit
R24 final state). Spec kept in `docs/handoffs/2026-07-02-r25-config-types-split-spec.md`
for future retry with different strategy (e.g., single-file split with
section markers, or move cross-references to types.rs re-exports).

## Lessons for R26+

- **DTO god-files with cross-references** are harder than free-fn
  god-files (R24) or impl-block god-files (R22, R23). Need different
  split strategy (e.g., horizontal split by type category not vertical
  by sub-domain).
- **R25 deferred, not failed** — the 5 sibling files WERE created with
  correct content, but service.rs truncation + cross-sibling imports
  weren't completed in single Mavis take-over pass.
- Future R25 retry should: (1) add `pub use` re-exports in service.rs
  for ALL sibling items, (2) move `deserialize_agent_profiles` to
  appropriate sibling, (3) keep `ConfigProvider` trait in providers.rs,
  (4) add `super::sibling::Type` cross-imports in each sibling file.

## Next: R26 = `contracts/runtime-ports/lib.rs` (2460 lines)

Different layer (contracts vs service). Has trait + struct/enum mix
but with potentially cleaner sub-domain boundaries (port groups).
Skip to R26 since R25 needs different strategy.