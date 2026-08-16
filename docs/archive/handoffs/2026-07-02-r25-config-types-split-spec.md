# R25 god-object split spec — `service/config/types.rs` (2406 lines)

> Round 25 god-object split: `assembly/core/src/service/config/types.rs`
> (2406 lines, ~50 struct/enum + ~30 impl Default + ~15 free fn) split into
> facade + 5 sibling files.

## §1 Background

R24 session_usage/service.rs 完成 (commit `8c328ab` review-fix, QClaw 7.8/10 + Kimi 7.8/10 APPROVE).
R25 = config god-file 拆分。

**Pre-R25 baseline**:
- `service/config/types.rs`: 2406 lines (QClaw 抓的实际 2460 vs spec 2458 — R24 errata)
- `service/config/mod.rs`: mod.rs (待查)
- 50+ struct/enum + 30+ impl Default + 15+ free fn (deserialize helper, default_*)

**God-file pattern**: DTO-heavy god-file (类似 session_usage free-fn but more
struct/enum + impl Default). 没有 inherent method block.

## §2 目标 — service.rs 2406 → facade + 5 sibling

### §2.1 r25a config-theme

**目标 sibling**: `service/config/theme.rs` (新, ~360 行)

**迁入内容** (L1-358):
- FontPreferenceSnapshot, UiFontSizeSnapshot, FlowChatFontSnapshot
- GlobalConfig, ProjectConfig, AppConfig, AppLoggingConfig
- ModelExchangeTracingMode, ModelExchangeTracingConfig
- AppSessionConfig
- AiExperienceQuickAction, AIExperienceConfig
- AgentCompanionPetSelection
- SidebarConfig, RightPanelConfig, NotificationConfig
- ThemeConfig, ThemeColors, ThemeFonts, FontSizes, ThemeSpacing,
  ThemeBorderRadius, ThemeShadows
- ThemesConfig + impl Default for ThemesConfig

**facade 保留** (`service.rs` L1-358):
- 全部 struct/enum stay (cross-crate API)
- 全部 impl Default stay (orphan for type)

### §2.2 r25b config-editor

**目标 sibling**: `service/config/editor.rs` (新, ~70 行)

**迁入内容** (L360-426):
- EditorConfig, MinimapConfig
- (Terminal 留 r25c with AI)

Actually spec shows Editor at L360-426. Terminal at L388-426. Workspace at L429+.

Let me re-check actual line numbers — spec needs validation against real file.

**计划 (验证后)**: r25b = editor.rs (EditorConfig + MinimapConfig) + terminal.rs (TerminalConfig + TerminalThemeConfig) + workspace.rs (WorkspaceConfig). 三个 sibling 一起.

### §2.3 r25c config-ai

**目标 sibling**: `service/config/ai.rs` (新, ~530 行, largest)

**迁入内容** (L429-963 验证后):
- WorkspaceConfig
- ModelCapability, ModelCategory
- DefaultModelsConfig
- ReviewTeamConfig + impl Default + default_* helpers
- AIConfig (~165 lines, big)
- AgentProfileConfig, AgentProfileView
- ConfirmationMode, ShellSecurityConfig + impl Default

### §2.4 r25d config-runtime

**目标 sibling**: `service/config/runtime.rs` (新, ~460 行)

**迁入内容** (L966-1420 验证后):
- DebugModeConfig + impl Default + impl DebugModeConfig
- LanguageDebugTemplate + impl Default
- AgentSubagentOverrideState
- AIModelConfig + impl AIModelConfig
- AuthConfig
- AIModelConfigCompat + impl From

### §2.5 r25e config-events

**目标 sibling**: `service/config/events.rs` (新, ~100 行)

**迁入内容** (L1421-end):
- ConfigChangeEvent
- ConfigValidationResult
- ConfigValidationError
- ConfigValidationWarning
- (大部分 impl Default blocks 留在原 struct 的 sibling 中)

### §2.6 r25f service-facade-finalize

**Mavis 范围** (after 5 sub-rounds):
- service.rs: re-export only + `mod tests` (if any) + free fn deserialize_agent_profiles
- mod.rs: add `pub mod theme; pub mod editor; pub mod terminal; pub mod workspace;
  pub mod ai; pub mod runtime; pub mod events;`

## §3 visibility 规则

- 50+ struct/enum: stay `pub` (cross-crate API)
- 30+ impl Default block: stay in same file as struct
- 15+ free fn (default_* helpers, deserialize_*): stay in same file as
  their struct usage (or extract to private helper if cross-sibling)
- All type items stay in their dedicated sibling

## §4 mod.rs 调整

```rust
pub mod theme;
pub mod editor;
pub mod terminal;
pub mod workspace;
pub mod ai;
pub mod runtime;
pub mod events;
pub mod types; // facade for re-exports
```

## §5 producer self-report (每 sub-round)

- line cap (canonical wc-l, target ≤ 600 per sibling except ai.rs ≤ 600)
- long line count (≤5 per file R18+ tolerance)
- visibility (哪些 struct 是 pub 哪些是 pub(super), why)
- cross-crate consumer (50+ struct 全 pub)
- BOM / CRLF 检查 (0 必填)
- `cargo check -p northhing-core --features product-full --lib` 0 errors

## §6 Mavis 3-axis verify (after r25f)

| Axis | Command | Result |
|---|---|---|
| 1 | `cargo check --workspace` | 0 errors |
| 2 | `cargo check -p northhing-cli` | 0 errors |
| 3 | `cargo check -p northhing-desktop` | 0 errors |
| 4 | `cargo check -p northhing-server` | 0 errors |
| 5 | `cargo test -p northhing-core --lib` | 899 passed, 0 failed (no new test breakage) |

## §7 R19 lesson (apply at dispatch)

> **Pre-emptive `extend-timeout` at dispatch** for any split task >1000 lines.
> R25 计划: 5 producer sub-rounds, 2406 lines, ~360-530 lines/sub-round.
> Pre-emptive extend-timeout +60 min at dispatch.

## §8 ref

- R24 stage summary (review-fix): `docs/handoffs/2026-07-02-r24-stage-summary.md`
- R24 spec template: `docs/handoffs/2026-07-02-r24-session-usage-split-spec.md`
- AGENTS.md god-object split lessons: `northing-god-object-split.md` (memory topic)