# Northing god-object split — handoff to next session (2026-07-03)

> Tonight's session (R21-R28 + R27b) 收工。Next session 接手前先读这 doc。

## Current state

- **branch**: main
- **HEAD**: `c106dd2` (R27b refactor)
- **working tree**: clean
- **5 successful splits** (R23 R24 R26 R27 R27b) + 2 deferred (R25 R28)

## Commit chain (latest 12)

```
c106dd2 R27b refactor (manager_impl.rs 1234 -> 3 sibling, QClaw blocker fix)
3df7bc4 R27 BOM strip (Kimi R27 P2 fix)
42c7cd0 QClaw R27 review report (7.5/10 CONDITIONAL)
c496c27 R28 stage summary (deferred)
5dec785 R27 refactor (manager.rs 1505 -> 2 sibling)
672d03e R26 refactor (runtime-ports/lib.rs 2460 -> 4 sibling)
587605c R26 stage summary (replaced)
90e1e14 R26 spec
45139e9 R25 review report (QClaw)
658600f R25 stage summary (deferred)
043f415 R24 review correction
8c328ab R24 review-fix errata
```

## Tonight's splits - summary

| Round | Target | Before | After | Verdict | Commit |
|---|---|---|---|---|---|
| R23 | workspace/service.rs | 2339 | 1029 (-56%) | QClaw 8.5/10 + Kimi 8.3/10 APPROVE | multiple (R23 chain) |
| R24 | session_usage/service.rs | 2458 | 1228 (-50%) | QClaw 7.8/10 + Kimi 7.8/10 APPROVE | `8c328ab` review-fix |
| R25 | config/types.rs | 2406 | (deferred) | DTO god-file 跨引用太多 | spec only |
| R26 | runtime-ports/lib.rs | 2460 | 863 facade + 4 sibling | 0 errors + 43 + 102 tests | `672d03e` |
| R27 | workspace/manager.rs | 1505 | 7 facade + 2 sibling | Kimi 9.2/10 + QClaw 7.5/10 | `5dec785` + BOM fix `3df7bc4` |
| R27b | workspace/manager_impl.rs | 1234 | 10 facade + 3 sibling | QClaw blocker fixed, Kimi not yet | `c106dd2` |
| R28 | terminal/session/manager.rs | 1457 | (deferred) | Drop trait + cross-sibling visibility | stage summary only |

## Sibling files created (cumulative)

**R23** (workspace/):
- factory.rs (25) + lifecycle.rs (344) + identity_watch.rs (264) + accessors.rs (205) + update.rs (357)
- Note: admin.rs 821 lines is OUTSIDE R23 scope (R23 split was into different siblings, admin stayed)

**R24** (session_usage/):
- entry + snapshot + breakdowns_core + breakdowns_extra + utilities

**R26** (runtime-ports/src/):
- port_core.rs (97) + session_workspace.rs (588) + remote.rs (151) + agent.rs (800) — interface crate, all `pub`

**R27** (workspace/):
- types.rs (300) — impl WorkspaceIdentity

**R27b** (workspace/):
- workspace_info_impl.rs (487) + manager_lifecycle.rs (439) + manager_accessors.rs (363)

## Deferred: R25 + R28 retry strategy

**R25 (config/types.rs 2406 lines)** — DTO god-file, 30+ struct 互 reference, 28 scattered impl Default, 60+ external import sites
- User 提的 retry strategy: `pub use` re-exports in types.rs facade 解决 60 外部 imports; 水平拆分（按类型类别）vs 垂直拆分（按子域）; `default_*` helpers 跟随 struct
- Spec: `docs/handoffs/2026-07-02-r25-config-types-split-spec.md`

**R28 (terminal/session/manager.rs 1457 lines)** — horizontal split failed
- Issue: `pub(super) fn drop()` rejected (impl Drop trait); cross-sibling types not visible (mod.rs explicit list); use block needs to be in both siblings
- Retry strategy (documented in R28 stage summary):
  1. Use `pub use types::*;` in mod.rs (replace explicit list with wildcard)
  2. Strip `pub(super)` from `impl Drop` blocks
  3. Add explicit `use super::types::{...}` in session_manager.rs
- Spec: `docs/handoffs/2026-07-02-r28-terminal-session-manager-split-spec.md`

## Pre-existing noise (do NOT touch)

- 156 uncommitted `cargo fmt` 改动 (pre-existing 整个 workspace 大范围格式化扫尾) — user 守则 "不要碰"
- 7 untracked review/spec handoff 文档 — 跟当前工作无关
- 22 unused import warnings in admin/lifecycle/service/update/accessors (R23 split 残留) — user 守则 "不要碰" pre-existing

## Mavis lessons for next session (in memory)

`C:\Users\UmR\.mavis\agents\mavis\memory\MEMORY.md` 已存:
- 拆分守则 (god-object split)
- Sub-domain split 4 类隐式错误 (E0432/E0624/E0616/E0308)
- R26 interface crate pattern (lib.rs split into pub sibling)
- R27 horizontal split lesson
- R27b `pub(super)` on private fields when splitting impl across siblings
- f-string `{{` Python escape bug
- `#[serde(...)]` attribute scope (need `use serde;` or preceding `#[derive(Serialize, Deserialize)]`)
- Range off-by-one (Python `lines[start:end+1]` is INCLUSIVE end)
- `pub(super) fn default()` rejected by trait Default implicit visibility
- Drop trait method `fn drop(&mut self)` rejects `pub(super)` qualifier
- 800 line cap守则 (QClaw 抓)
- QClaw vs Kimi pattern: QClaw = strict scoring + 数字 verify, Kimi = APPROVE looser + 概念 drift
- QClaw 引用的 file line count / unwrap count 必 re-verify (R18 unwrap_or 误算, R23 service.rs 2347 vs 2339 误)

## Next session suggestion

按 user 节奏 (跨夜 work), 接手时:
1. 读这 handoff doc 先 (本文件)
2. 读 MEMORY.md 的 god-object split 段
3. 选 1 个: R25 retry / R28 retry / R29 ports.rs / R30 framework.rs / R30 scheduler.rs + insights
4. R29+ 候选: ports.rs (1739) + framework.rs (2189) + service.rs 多 + manager.rs 多

Mavis 跑 R21+ parallel sub-rounds flow (team plan) 或 Mavis take-over mode (单文件), 视复杂度。

## Refs (handoff doc paths)

- `docs/handoffs/2026-07-02-r23-stage-summary.md`
- `docs/handoffs/2026-07-02-r24-stage-summary.md`
- `docs/handoffs/2026-07-02-r25-stage-summary.md` (deferred)
- `docs/handoffs/2026-07-02-r25-config-types-split-spec.md`
- `docs/handoffs/2026-07-02-r26-stage-summary.md`
- `docs/handoffs/2026-07-02-r26-runtime-ports-split-spec.md`
- `docs/handoffs/2026-07-02-r27-stage-summary.md`
- `docs/handoffs/2026-07-02-r27b-stage-summary.md`
- `docs/handoffs/2026-07-02-r28-stage-summary.md` (deferred)
- `docs/handoffs/2026-07-02-r28-terminal-session-manager-split-spec.md` (mentioned)
- `docs/reviews/round24-qclaw-review.md` (R24 review, committed)
- `docs/reviews/round25-qclaw-review.md` (R25 review, committed)
- `docs/reviews/round27-qclaw-review.md` (R27 review, committed)
- `memory/MEMORY.md` (Mavis long-term memory)
- `memory/northing-god-object-split.md` (topic file, R3b-R20 detailed lessons)
- `memory/codegraph-workflow.md` (CodeGraph usage)
