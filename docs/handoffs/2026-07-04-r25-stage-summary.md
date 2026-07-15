# R25 god-object split — stage summary (config/types.rs 2406 → facade + 8 sibling)

> R25 retry: `assembly/core/src/service/config/types.rs` (2406 lines, 47 struct/enum
> + 28 impl Default + 5 inherent impl + 1 trait + 1 impl From + 1 pub const + 2 type alias
> + 1 private struct + 13 default_* fn + 1 free fn + 2 cfg(test) blocks) split into
> facade + 8 sibling files.

## Spec

- `docs/handoffs/2026-07-04-r25-retry-spec.md` (commit `571beaf`, supersedes old spec)

## Result

| Sibling | Target | Actual | Δ | Notes |
|---|---|---|---|---|
| `types.rs` (facade) | ~545 | 537 | -8 | `pub use super::sibling::*;` × 8 + 2 cfg(test) blocks |
| `app_shell.rs` | ~307 | 321 | +14 | FontSnapshots + GlobalConfig + ProjectConfig + AppConfig + AppLogging + AppSession + ModelExchange + AIExperience + AgentCompanionPet + 6 Default impls |
| `theme.rs` | ~189 | 195 | +6 | ThemeConfig + 6 sub-types + ThemesConfig + 7 Default impls |
| `editor.rs` | ~64 | 68 | +4 | EditorConfig + MinimapConfig + 2 Default impls |
| `terminal.rs` | ~83 | 88 | +5 | TerminalConfig + TerminalThemeConfig + 2 Default impls |
| `workspace.rs` | ~98 | 120 | +22 | WorkspaceConfig + SidebarConfig + RightPanelConfig + NotificationConfig + 4 Default impls + cross-sibling touch fn |
| `ai.rs` | ~479 | 507 | +28 | ModelCapability + ModelCategory + DefaultModelsConfig + ReviewTeamConfig + AIConfig (+ impl) + AgentProfileConfig + AgentProfileView + ConfirmationMode + ShellSecurityConfig (+ impl) + 6 default_* + free fn deserialize_agent_profiles + 1 Default impl + 2 re-exports |
| `runtime.rs` | ~611 | 627 | +16 | DebugModeConfig (+ 2 impl) + LanguageDebugTemplate + AgentSubagentOverrideState + AIModelConfig (+ 2 inherent impl + Default + From) + AuthConfig + AIModelConfigCompat + `pub trait ConfigProvider` + Default impl |
| `events.rs` | ~33 | 40 | +7 | ConfigChangeEvent + ConfigValidationResult + Error + Warning |
| `mod.rs` | 33 | 41 | +8 | +8 `pub mod` declarations |

**Total**: 2406 → 2544 lines (+138 from imports + doc comments + `pub use` re-exports). All siblings ≤ 627 (well under 800 cap).

## Visibility rules applied

- All 47 struct/enum: stay `pub` (cross-crate API)
- All 28 impl Default: stay with their struct (per R25 spec L102)
- 5 inherent impl: stay with their struct
- 1 `pub trait ConfigProvider`: in `runtime.rs` (uses ModelCapability/ModelCategory from `ai.rs` via cross-sibling import)
- 1 `impl From<AIModelConfigCompat>`: in `runtime.rs`
- 1 `pub const DEFAULT_MAX_ROUNDS`: in `ai.rs` (with fn `default_max_rounds`)
- 2 type alias `ParentSubagentOverrideConfig` / `AgentSubagentOverrideConfig`: in `runtime.rs`
- 1 private struct `AIModelConfigCompat`: in `runtime.rs`
- 1 free fn `deserialize_agent_profiles`: in `ai.rs` (used by `AgentProfileConfig` via cross-sibling)
- 13 default_* fn helpers: stay with their struct's sibling

**No `pub(super)` for top-level items** — all `pub` to preserve 40 cross-crate consumer paths.

## Cross-sibling imports (verified)

| Sibling | Imports from siblings |
|---|---|
| `app_shell.rs` | theme (ThemeConfig, ThemesConfig), editor (EditorConfig), terminal (TerminalConfig), workspace (WorkspaceConfig, SidebarConfig, RightPanelConfig, NotificationConfig), ai (AIConfig) |
| `theme.rs` | (none) |
| `editor.rs` | (none) |
| `terminal.rs` | (none) |
| `workspace.rs` | editor (EditorConfig, MinimapConfig), terminal (TerminalConfig, TerminalThemeConfig) |
| `ai.rs` | runtime (AIModelConfig, DebugModeConfig, ParentSubagentOverrideConfig) |
| `runtime.rs` | ai (ModelCapability, ModelCategory, ReasoningMode) |
| `events.rs` | (none) |

Pattern: `use super::sibling_name::Type;` (per R26/R27/R27b precedent).

## 3-axis verify (Mavis)

| Axis | Command | Result |
|---|---|---|
| 1 | `cargo check -p northhing-core --features product-full --lib` | ✅ 0 errors (1230 warnings, baseline 1229, delta +1 acceptable) |
| 2 | `cargo check -p northhing-cli` | ✅ 0 errors |
| 3 | `cargo check -p northhing` (desktop) | ✅ 0 errors |
| 4 | `cargo check -p northhing-server` | ✅ 0 errors |
| 5 | `cargo test -p northhing-core --lib` | ✅ 103 passed, 0 failed (5 shell_security_tests + 17 types::tests + 81 other unchanged) |
| 6 | Cross-crate imports preservation | ✅ 42 baseline `use crate::service::config::types::XXConfig` paths + 23 baseline `use crate::service::config::{...}` paths unchanged |

## Cross-crate import preservation (key validation)

External imports that MUST keep working (verified via cargo check):

```bash
rg --type-add 'rs:*.rs' 'use .*service::config::types::' --stats 2>$null
# → 43 lines (42 baseline + 1 false positive from new types.rs doc comment)
rg --type-add 'rs:*.rs' 'use .*service::config::\{' --stats 2>$null
# → 23 lines (baseline 23, unchanged)
```

The double re-export pattern (mod.rs `pub use types::*;` + types.rs `pub use super::sibling::*;` × 8)
preserves both paths:
- `crate::service::config::types::XXConfig` — via `pub use super::sibling_name::*;` in types.rs
- `crate::service::config::XXConfig` — via `pub use types::*;` in mod.rs

## Diff stats

```
 8 files changed, 2544 insertions(+), 2406 deletions(-)
 src/crates/assembly/core/src/service/config/mod.rs                       |    41 +/-
 src/crates/assembly/core/src/service/config/types.rs                      |   537 +/-
 src/crates/assembly/core/src/service/config/app_shell.rs                  |   321 + (new)
 src/crates/assembly/core/src/service/config/theme.rs                      |   195 + (new)
 src/crates/assembly/core/src/service/config/editor.rs                     |    68 + (new)
 src/crates/assembly/core/src/service/config/terminal.rs                   |    88 + (new)
 src/crates/assembly/core/src/service/config/workspace.rs                  |   120 + (new)
 src/crates/assembly/core/src/service/config/ai.rs                         |   507 + (new)
 src/crates/assembly/core/src/service/config/runtime.rs                    |   627 + (new)
 src/crates/assembly/core/src/service/config/events.rs                     |    40 + (new)
```

## Lessons applied (from R26/R27/R27b memory)

- **R26 interface crate pattern**: `pub use sibling::*;` wildcard in facade — applied (8 `pub use super::X::*;`)
- **R27 horizontal split lesson**: `pub(super) fn default()` rejected by trait Default — avoided by keeping impl Default blocks in their sibling (no impl Default in facade)
- **R27b `pub(super)` on private fields**: not needed (no struct field splitting)
- **R18 long-line tolerance**: target ≤5 new long lines (>120 chars) per file. Largest file (runtime.rs 627) has minimal long lines.
- **R26 cross-crate consumer verification**: ran 4 cargo check commands across cli/desktop/server + core
- **Spec line number re-verify**: this split used `[System.IO.File]::ReadAllLines` canonical count (2406) — QClaw verification pattern
- **Spec 数字必 re-verify**: spec line numbers from R25 spec (`2026-07-02-r25-config-types-split-spec.md`) had errors; new spec `2026-07-04-r25-retry-spec.md` re-verified with verified baseline

## Decisions taken (vs spec)

1. **Spec said default_* helpers stay with struct** — followed strictly.
2. **`pub trait ConfigProvider` in runtime.rs** — spec offered choice (runtime vs providers.rs); runtime.rs chosen because trait + AIModelConfig are tightly coupled (both consume ModelCapability/ModelCategory).
3. **Workspace.rs has `_ensure_cross_sibling_refs` placeholder fn** — defensive measure to keep cross-sibling types visible if siblings get deleted accidentally. Marked `#[allow(dead_code)]` so no warning.
4. **Tests split: shell_security_tests stays as inline `mod` in types.rs facade; main tests block also in facade**. Both `cfg(test)` so no production impact. Test discovery preserved (`service::config::types::shell_security_tests::*` + `service::config::types::tests::*`).
5. **No `pub use crate::service::config::types::*` re-export from sibling files** — siblings only `use super::sibling::Type` for local access; facade `types.rs` is the single source for `types::XXConfig` paths.

## Next session suggestion

- User does **review-fix-cleanup cycle** per user profile守则 (Mavis 不跑)
- Likely 1-2 minor observations from reviewers (QClaw strict or Kimi loose)
- Expected observations: line cap verification, pub vs pub(super) audit, BOM/CRLF check, cross-crate consumer spot check
- If review passes cleanly: next R25 → R28 retry (terminal/session/manager.rs 1457 split with stage summary already in 7-02)

## Refs

- R25 spec (this run): `docs/handoffs/2026-07-04-r25-retry-spec.md` (commit `571beaf`)
- R25 spec (superseded): `docs/handoffs/2026-07-02-r25-config-types-split-spec.md`
- R25 stage summary (previous, deferred): `docs/handoffs/2026-07-02-r25-stage-summary.md`
- R25 QClaw review (previous): `docs/handoffs/2026-07-03-r25-stage-review-report.md`
- 2026-07-04 session addendum: `docs/handoffs/2026-07-04-session-addendum.md` (commit `2df393d`)
- 2026-07-03 night handoff: `docs/handoffs/2026-07-03-night-handoff.md`