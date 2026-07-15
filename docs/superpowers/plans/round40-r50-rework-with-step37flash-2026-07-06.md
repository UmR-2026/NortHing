# R40-R50 Rework Plan — 选项 A 实施文档

> **Date**: 2026-07-06
> **Author**: Mavis (主会话)
> **Status**: 等待 user apikey → dispatch
> **Base**: `main @ 4841f3bd` (R39 完成, 干净 baseline)
> **Backup tag**: `backup/r40-r49-before-cleanup` = `f05d3b57` (R40-R49 半成品)

## 0. 整体策略

1. **Reset**: 新建 `fix/r40-r50-rework` branch from main (不动 backup branch, 留作 reference)
2. **派模型**: 混合 M2.7-highspeed (高难度) + step3.7flash (简单-中等, user 试能力)
3. **每 round**: N sub-rounds **并行** + 1 squash-merge
4. **3-axis verify**: cargo check + cargo test + cargo fmt
5. **Review**: 写 stage summary + 派 QClaw + Kimi

## 1. Model classification (关键)

| 难度 | 行数范围 | 复杂度特征 | 模型 | 备注 |
|---|---|---|---|---|
| 🔴 HARD | 1500+ | 大 struct + 多 impl + 跨 crate 风险 | **M2.7-highspeed** | step3.7flash 太慢/能力不够 |
| 🟡 MED-HIGH | 1300-1500 | 中型 struct + 多个 sub-domain | **M2.7-highspeed** | 1-2 个 sub-domain 可试 step3.7flash |
| 🟢 MED | 1000-1300 | 单一 struct + 清晰 sub-domain | **mixed** | 50/50 M2.7 + step3.7flash |
| 🟢 LOW | 800-1000 | 简单 impl 块 + 少 cross-ref | **step3.7flash** | 试 step3.7flash 能力 |
| ⚪ TEST | tests/*.rs | test 重排 | **step3.7flash** | 低风险, 适合试能力 |
| ⚪ CLEANUP | R50 阶段 | fmt + 长 line + BOM | **Mavis 自做** | 简单但量大, 不值得派 subagent |

## 2. R40 (6-way parallel, 90-120 min) — Critical Zone

| Sub | File | Lines | 模型 | Crate | 难度 | 备注 |
|---|---|---:|---|---|---|---|
| R40a | `agentic/tools/computer_use_host.rs` | 1811 | **M2.7-highspeed** | northhing-core | 🔴 HARD | R40 难度最高, 必须 M2.7 |
| R40b | `agentic/coordination/subagent_orchestrator.rs` | 1773 | **M2.7-highspeed** | northhing-core | 🔴 HARD | |
| R40c | `agentic/coordination/ports.rs` | 1739 | **M2.7-highspeed** | northhing-core | 🔴 HARD | interface crate pattern (R26) |
| R40d | `agentic/insights/service.rs` | 1681 | **M2.7-highspeed** | northhing-core | 🔴 HARD | |
| R40e | `service_agent_runtime.rs` | 1643 | **M2.7-highspeed** | northhing-core | 🔴 HARD | |
| R40f | `service/remote_connect/bot/feishu.rs` | 1638 | **M2.7-highspeed** | northhing-core | 🔴 HARD | 配对 R39a weixin.rs |

## 3. R41 (7-way, 90 min) — Big Zone P1

| Sub | File | Lines | 模型 | 难度 |
|---|---|---:|---|---|
| R41a | `agentic/tools/implementations/bash_tool.rs` | 1630 | **M2.7-highspeed** | 🟡 |
| R41b | `agentic/tools/pipeline/tool_pipeline.rs` | 1628 | **M2.7-highspeed** | 🟡 |
| R41c | `agentic/tools/implementations/git_tool.rs` | 1590 | **M2.7-highspeed** | 🟡 |
| R41d | `agentic/coordination/scheduler.rs` | 1526 | **M2.7-highspeed** | 🟡 |
| R41e | `agentic/tools/tool_context_runtime.rs` | 1447 | **step3.7flash** | 🟢 MED | 试 step3.7flash |
| R41f | `agentic/tools/implementations/code_review_tool.rs` | 1422 | **step3.7flash** | 🟢 MED | 试 step3.7flash |
| R41g | `service/lsp/workspace_manager.rs` | 1415 | **M2.7-highspeed** | 🟡 |

## 4. R42 (6-way, 1 SKIP) — Big Zone P2

| Sub | File | Lines | 模型 | 难度 |
|---|---|---:|---|---|
| R42a | `agentic/insights/html.rs` | 1373 | **M2.7-highspeed** | 🟡 |
| R42b | `service/remote_connect/mod.rs` (mod.rs) | 1380 | **SKIP** | — | R48 阶段处理 mod.rs 拆分 |
| R42c | `service/snapshot/snapshot_core.rs` | 1309 | **M2.7-highspeed** | 🟡 |
| R42d | `agentic/tools/implementations/cron_tool.rs` | 1286 | **step3.7flash** | 🟢 MED |
| R42e | `agentic/tools/implementations/review_platform_tool.rs` | 1261 | **step3.7flash** | 🟢 MED |
| R42f | `agentic/session/session_persistence.rs` | 1257 | **step3.7flash** | 🟢 MED |

## 5. R43 (7-way, 1 SKIP) — Mid Zone P1

| Sub | File | Lines | 模型 | 难度 |
|---|---|---:|---|---|
| R43a | `services-integrations/src/git/service.rs` | 1255 | **M2.7-highspeed** | 🟡 |
| R43b | `agentic/tools/browser_control/actions.rs` | 1253 | **M2.7-highspeed** | 🟡 |
| R43c | `service/session_usage/service.rs` (R24 partial) | 1228 | **step3.7flash** | 🟢 MED |
| R43d | `agentic/coordination/dialog_turn/mod.rs` (mod.rs) | 1219 | **SKIP** | — | R48 阶段处理 |
| R43e | `services-integrations/src/remote_ssh/remote_exec.rs` | 1195 | **step3.7flash** | 🟢 MED |
| R43f | `service/workspace_runtime/service.rs` | 1169 | **step3.7flash** | 🟢 MED |
| R43g | `agentic/agents/prompt_builder/prompt_builder_impl.rs` | 1168 | **step3.7flash** | 🟢 MED |

## 6. R44 (7-way, 75-90 min) — Mid Zone P2

| Sub | File | Lines | 模型 | 难度 |
|---|---|---:|---|---|
| R44a | `agentic/tools/implementations/exec_command/command.rs` | 1157 | **M2.7-highspeed** | 🟡 |
| R44b | `miniapp/manager.rs` | 1129 | **step3.7flash** | 🟢 MED |
| R44c | `apps/cli/src/ui/model_config_form.rs` | 1125 | **step3.7flash** | 🟢 MED |
| R44d | `agentic/tools/implementations/grep_tool.rs` | 1111 | **step3.7flash** | 🟢 MED |
| R44e | `execution/tool-execution/src/search/grep_search.rs` | 943 | **step3.7flash** | 🟢 LOW |
| R44f | `service/lsp/process.rs` | 1087 | **step3.7flash** | 🟢 MED |
| R44g | `agentic/tools/implementations/skills/registry.rs` | 1050 | **step3.7flash** | 🟢 MED |

## 7. R45 (6-way, 60-75 min) — Small + Cap Top

| Sub | File | Lines | 模型 | 难度 | 备注 |
|---|---|---:|---|---|---|
| R45a | `apps/cli/src/ui/theme.rs` | 1046 | **step3.7flash** | 🟢 MED | |
| R45b | `service/workspace/service.rs` (R23 partial) | 1029 | **step3.7flash** | 🟢 MED | 必读 R23 commit |
| R45c | `services-integrations/src/remote_ssh/workspace_search/service.rs` (R39c partial) | 1008 | **step3.7flash** | 🟢 LOW | 必读 R39c commit |
| R45d | `agentic/session/session_manager_metadata_tests.rs` (test) | 1010 | **step3.7flash** | ⚪ TEST | test file 拆分 |
| R45e | `apps/cli/src/ui/chat/render.rs` | 983 | **step3.7flash** | 🟢 LOW | |
| R45f | `agentic/execution/round_subhandlers.rs` | 972 | **step3.7flash** | 🟢 LOW | R8b 续 |

## 8. R46 (7-way, 60-75 min) — Cap Zone P1

| Sub | File | Lines | 模型 | 难度 | 备注 |
|---|---|---:|---|---|---|
| R46a | `agentic/tools/implementations/computer_use_actions/desktop_ax_actions.rs` | 970 | **step3.7flash** | 🟢 LOW | R37h/R38b 续 |
| R46b | `apps/cli/src/ui/startup/selectors.rs` | 958 | **step3.7flash** | 🟢 LOW | R39i 续 |
| R46c | `apps/desktop/src/app_state/settings.rs` | 925 | **step3.7flash** | 🟢 LOW | R37a 续 |
| R46d | `adapters/ai-adapters/src/stream/types/openai.rs` | 910 | **step3.7flash** | 🟢 LOW | |
| R46e | `execution/agent-runtime/src/deep_review/task_execution.rs` | 905 | **step3.7flash** | 🟢 LOW | R37c 续 |
| R46f | `services-integrations/src/workspace_search/service.rs` (R39c partial) | 884 | **step3.7flash** | 🟢 LOW | R39c 续 |
| R46g | `execution/agent-runtime/src/scheduler.rs` | 877 | **step3.7flash** | 🟢 LOW | |

## 9. R47 (7-way, 60-75 min) — Cap Zone P2

| Sub | File | Lines | 模型 | 难度 | 备注 |
|---|---|---:|---|---|---|
| R47a | `execution/agent-runtime/src/prompt_cache.rs` | 873 | **step3.7flash** | 🟢 LOW | |
| R47b | `services-integrations/src/remote_ssh/manager_session_lifecycle.rs` | 856 | **step3.7flash** | 🟢 LOW | R20a 续 |
| R47c | `service/snapshot/manager.rs` | 854 | **step3.7flash** | 🟢 LOW | R33/R36 续 |
| R47d | `execution/agent-runtime/src/deep_review/budget.rs` | 853 | **step3.7flash** | 🟢 LOW | |
| R47e | `service/mcp/server/manager/auth.rs` | 848 | **step3.7flash** | 🟢 LOW | |
| R47f | `apps/cli/src/ui/question.rs` | 838 | **step3.7flash** | 🟢 LOW | |
| R47g | `agentic/tools/registry.rs` | 835 | **step3.7flash** | 🟢 LOW | |

## 10. R48 (7-way, 60-75 min) — Cap Zone P3 + mod.rs 拆分

| Sub | File | Lines | 模型 | 难度 | 备注 |
|---|---|---:|---|---|---|
| R48a | `apps/cli/src/modes/chat/input.rs` | 835 | **step3.7flash** | 🟢 LOW | |
| R48b | `execution/agent-dispatch/src/runtime.rs` | 815 | **step3.7flash** | 🟢 LOW | |
| R48c | `agentic/tools/browser_control/browser_launcher.rs` | 815 | **step3.7flash** | 🟢 LOW | |
| R48d | `apps/cli/src/main.rs` | 813 | **M2.7-highspeed** | 🟡 MED | ⚠️ 入口文件, 改动风险高 |
| R48e | `agentic/coordination/dialog_turn/turn_subhandlers.rs` | 806 | **M2.7-highspeed** | 🟡 MED | R6/R21/R22 续 |
| R48f | `apps/cli/src/ui/tool_cards/block_render.rs` (R38a partial) | 806 | **M2.7-highspeed** | 🟡 MED | R38a 续 |
| R48g | `agentic/execution/round_executor.rs` | 804 | **M2.7-highspeed** | 🟡 MED | |

## 11. R49 (5-way) — mod.rs 拆分 + weixin_bot_media 续拆

| Sub | File | Lines | 模型 | 难度 | 备注 |
|---|---|---:|---|---|---|
| R49a | `service/remote_connect/bot/weixin_bot_media.rs` (R39a partial) | 803 | **step3.7flash** | 🟢 LOW | R39a 续 |
| R49b | `agentic/session/session_manager_lifecycle_tests.rs` (test) | 931 | **step3.7flash** | ⚪ TEST | test file 拆分 |
| R49c | `service/remote_connect/mod.rs` (mod.rs 1380) | 1380 | **M2.7-highspeed** | 🔴 HARD | ⚠️ mod.rs 重构, Mavis take-over 备份 |
| R49d | `agentic/coordination/dialog_turn/mod.rs` (mod.rs 1219) | 1219 | **M2.7-highspeed** | 🔴 HARD | ⚠️ mod.rs 重构 |
| R49e | (无独立 task) | - | - | - | R50 cleanup 统一处理 test files |

## 12. R50 (1-way) — Final cleanup (Mavis 自做)

| Sub | Task | 备注 |
|---|---|---|
| R50a | (全 workspace) | fmt + long line + BOM + cross-crate consumer 复查 + `cargo test --workspace` + `docs/handoffs/2026-07-06-r40-r50-retrospective.md` + 1 squash-merge |

## 13. Model 分配总览

| Model | Tasks | 占比 |
|---|---:|---:|
| **M2.7-highspeed** | 25 | 36% |
| **step3.7flash** | 42 | 60% |
| **SKIP** | 4 | — |
| **Mavis 自做** | 1 (R50 cleanup) | — |

**step3.7flash 主要在 R44-R49** (6 round, 41 task) — 这是测能力的最佳 zone (low risk + clear pattern)。

## 14. 关键铁律 (R40-R49 教训)

**禁止**:
- ❌ **任何 `_lost_methods.rs` placeholder** — 必须按 sub-domain 拆到正确 sibling
- ❌ **`part1.rs`/`part2.rs`/`part3.rs` 机械命名** — 必须 `bash_block.rs`/`init.rs`/`dispatch.rs` 等 sub-domain 命名
- ❌ **mod.rs 留 impl fn** — mod.rs 只放 `mod sibling;` + `pub use super::*;` + 必要的 struct 定义
- ❌ **cargo update** — 禁
- ❌ **`Out-File`/`Set-Content` 写 .rs** — 用 Edit/Write tool, 禁 PowerShell
- ❌ **`git commit --amend`** — R39c 0-byte 教训, 用 `git reset HEAD~1` + new commit
- ❌ **WIP commits in production** — 必须 fix 完再 commit
- ❌ **`Measure-Object -Line`** — 用 `[System.IO.File]::ReadAllLines().Count`

**必做**:
- ✅ **`git show HEAD:<file>` 看原 schema** — 必先看再拆
- ✅ **`git grep 'use crate::path'` 找 cross-crate consumer**
- ✅ **`cargo check -p <crate>` + `cargo check -p <consumer_crate>`** — 必跑
- ✅ **`cargo fmt -p <crate> -- --check`** — 必跑
- ✅ **每个 sibling ≤ 800 行, mod.rs ≤ 600 行**
- ✅ **wildcard re-export**: `pub use super::*;` in mod.rs
- ✅ **fields `pub(super)`**, methods 默认 private, 跨 sibling `pub(super)`
- ✅ **Encoding 验证**: commit 前 `py -c "raw=open(f,'rb').read(); assert raw[:3]!=b'\\xef\\xbb\\xbf', f; assert b'\\r\\n' not in raw, f"`

## 15. Dispatch 流程 (每 round)

```bash
# 1. 新建 branch
git checkout main  # 4841f3bd
git checkout -b fix/r40-r50-rework
git config --local core.autocrlf false  # 防止 CRLF 污染

# 2. 派 N 个 subagent 并行 (Mavis team plan)
mavis team plan run --plan-yaml plans/r40-critical-zone-2026-07-06.yaml
# pre-emptive extend at dispatch:
mavis team plan extend-timeout <plan-id> r40a-* --minutes 60
# (M2.7 tasks > 1500 lines)

# 3. 监控
mavis team plan status <plan-id>
# Watchdog: 每 30 min check 一次
# Auto-pause (2 cycles 0 pass) → take over

# 4. Take-over 信号
# Subagent 30 min 无 commit → Mavis take-over (R23/R39a/R39c pattern)

# 5. 全部 commit 后
git checkout fix/r40-r50-rework
# squash per round: 由于 PowerShell 不能 interactive rebase
# 用 git reset --soft main + 重 commit per round
# 或者保留 sequential commits (不 squash) — 由 user 决定

# 6. Mavis 3-axis verify
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cargo check --workspace --message-format=short | Tee-Object -FilePath target/round40-check.log
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path; cargo test --workspace --lib 2>&1 | Tee-Object -FilePath target/round40-test.log
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path; cargo fmt --workspace --check 2>&1 | Tee-Object -FilePath target/round40-fmt.log

# 7. 写 stage summary
# docs/handoffs/2026-07-06-r40-stage-summary.md
# docs/handoffs/2026-07-06-r41-stage-summary.md
# ...

# 8. 派 review
mavis team plan run --plan-yaml plans/r40-review-dispatch.yaml  # QClaw + Kimi
```

## 16. Reset branch 工具 (Mavis 准备好, 等 apikey 跑)

```bash
# 1. 确认 backup tag 在
git tag -l "backup/r40-r49-before-cleanup"

# 2. 不动 backup branch (impl/r40a-core-computer-use-host-split 保留)
git branch -a  # 确认 impl branch 还在

# 3. 新建 rework branch from main
git checkout main
git checkout -b fix/r40-r50-rework
git config --local core.autocrlf false  # 关键: 防止 CRLF 污染
```

## 17. Cron self-reminder (async-audit)

每 round 派 plan 后, 必设 cron 监控:

```bash
mavis cron self "r40-monitor" --every 30min --prompt "检查 R40 plan 状态, 如有 take-over 信号立刻处理"
```

## 18. Refs

- R21+ flow: `~/.mavis/agents/mavis/memory/MEMORY.md` §R21+ new flow
- northing-god-object-split.md 教训: `~/.mavis/agents/mavis/memory/northing-god-object-split.md`
- R40-R49 review report: 见上轮 review (REJECT, 6 BLOCKER)
- 详细 spec (R40-R50 v1): `docs/superpowers/plans/round40-50-detailed-plan-2026-07-06.md`
- 完整 plan YAML 模板: 上轮 `docs/superpowers/plans/round40-50-detailed-plan-2026-07-06.md` §每个 round 的执行 checklist

---

*Generated by Mavis 2026-07-06 15:45 (Asia/Shanghai). Awaiting user apikey for step3.7flash dispatch.*
