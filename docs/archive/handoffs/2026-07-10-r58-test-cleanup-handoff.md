# Phase B Handoff: Test Code Cleanup (R58 准备)

## 当前状态 (2026-07-10 00:30)

**Phase B lib 代码完成** (R47 → R57):
- assembly/core 源文件 >750 lines: 0 个
- `cargo check -p northhing-core --lib`: 0 错误
- main HEAD: 25 个 split commits + 6 个 Mavis take-over fix

**Test 代码问题** (`cargo check --tests`): 131 错误，分布在 20+ 文件

## 错误分布 (cargo check --tests)

| 错误类型 | 数量 | 含义 |
|----------|------|------|
| E0433 | 87 | cannot find type (类型找不到) |
| E0425 | 27 | cannot find function/value (函数/常量找不到) |
| E0422 | 5 | cannot find struct/variant |
| E0603 | 3 | private item (私有项) |
| E0432 | 2 | unresolved import |
| E0405 | 1 | cannot find trait |

### 缺失最多的类型 (按出现次数)

| 缺失类型 | 次数 | 来源 split | 修复方向 |
|----------|------|------------|----------|
| `Arc` | 29 | pre-existing | 加 `use std::sync::Arc;` |
| `CodeReviewTool` | 22 | R55a (code_review_tool) | tests.rs 加 `use super::CodeReviewTool;` 或改 `super::super::CodeReviewTool` |
| `DialogTurnKind` | 15 | dialog_turn split (R47) | tests.rs 加跨模块 import |
| `ToolExecutionOptions` | 4 | tool pipeline | 加 `use crate::...ToolExecutionOptions;` |
| `ToolWorkspaceKind` | 4 | tool pipeline | 同上 |
| `ToolTask`, `ToolCall`, `ToolExecutionContext` | 4 | tool pipeline | 同上 |
| `NortHingError`, `NortHingResult` | 3 | pre-existing | 加 `use crate::util::errors::*;` |

### 缺失最多的函数

| 缺失函数 | 次数 | 来源 | 修复方向 |
|----------|------|------|----------|
| `command_needs_light_checkpoint` | 5 | R55b (bash_tool) | bash_tool_impl.rs 加 `use super::execute_loop::command_needs_light_checkpoint;` 或 sibling re-export |
| `GET_TOOL_SPEC_TOOL_NAME` | 2 | tool framework | 找对模块路径 |
| `validate_tool_execution_admission` | 2 | tool framework | 同上 |
| `command_for_working_directory` | 2 | bash_tool | sibling re-export |
| `build_tool_call_truncation_recovery_notice` | 2 | tool framework | sibling re-export |
| `USER_STEERING_INTERRUPTED_MESSAGE` | 1 | constants | 加 `pub(crate) use` |
| `format_background_command_delivery_text` | 1 | R55b (bash_tool) | sibling re-export |

### E0603 私有项 (3 个)

- `escape_html` in `mcp/server/manager/auth/tests.rs:1` — 函数变 private
- `DEFAULT_SUBAGENT_MAX_CONCURRENCY` in `coordination/tests/turn_ports.rs:29`
- `MAX_SUBAGENT_MAX_CONCURRENCY` in `coordination/tests/turn_ports.rs:29`

## 关键错误文件清单 (按修复优先级)

### Tier 1 — 大量错误 (R57c + R55a 主战场)

**`code_review_tool/tests.rs`** (22 错误，CodeReviewTool 缺失)
- 来自 R55a split: tests.rs 引用 `super::CodeReviewTool` 但 CodeReviewTool 移到 sibling
- 修复: tests.rs 顶部加 `use super::super::CodeReviewTool;` 或更精确路径

**`prompt_builder/tests.rs`** (R57c)
- 多数已由 Mavis 修过 (ed99e791) — 删了 `mod tests {}` wrapper
- 但可能还残留 sibling reference 问题

**`code_review_tool/mod.rs`** (R55a)
- Mavis 修过 (bac458d1) — 删了重复 `mod tests;`
- 应已 OK

### Tier 2 — 多错误但分散

**`bash_tool/bash_tool_impl.rs`** (5 错误)
- 引用 `command_needs_light_checkpoint` (R55b moved to execute_loop)
- 修复: 加 `use super::execute::execute_loop::command_needs_light_checkpoint;`

**`session_usage/{service,format,persist,tracking}.rs`** (~20 错误)
- R57b split 后 sibling 引用问题
- 修复: 整理 use 路径

**`snapshot/manager.rs`** + `service/search/service.rs`
- 引用已搬走的类型

### Tier 3 — 单点

**`agentic/coordination/port_types.rs`** (R57a split)
**`agentic/coordination/tests/{session,subagent,turn}_ports.rs`** (R57a tests)
**`service/mcp/server/manager/auth/{auth_types.rs,tests.rs}`** 
**`agentic/tools/{git_tool,tool_context_runtime}/mod.rs`**

## 修复策略 (R58 计划)

### 选项 A: 局部 take-over (推荐)
Mavis 自己逐文件修，每个文件 1-2 行 fix。优点：精确、最小变更。缺点：费时（131 错误跨 20+ 文件）。

### 选项 B: 测试代码现代化
把 `mod tests { use super::*; }` 模式统一改为顶级测试 + 显式 import。优点：长期可维护。缺点：大量重写。

### 选项 C: 临时绕过
给 lib 加 `#[cfg(test)]` 守卫或禁用部分 test。**不推荐** — 治标不治本。

## R58 任务建议

如果继续 R58，2 个 5-way plan:

**R58a (lib code 修复)**:
1. `bash_tool_impl.rs` 加 `command_needs_light_checkpoint` 等 import (5 行)
2. `session_usage/service.rs` 和 sibling 整理 use 路径 (~10 行)
3. `code_review_tool/tests.rs` 加 `CodeReviewTool` import (1 行)
4. `git_tool/mod.rs` 和 `tool_context_runtime/mod.rs` 加缺失 import (5 行)
5. `coordination/port_types.rs` 整理 (~5 行)

**R58b (test files 修复)**:
1. `coordination/tests/{session,subagent,turn}_ports.rs` 加 `DEFAULT_SUBAGENT_MAX_CONCURRENCY` import 或改 `pub(crate)`
2. `mcp/server/manager/auth/tests.rs` 加 `escape_html` 路径或改 `pub(crate)`
3. 收集剩余的 `Arc` 缺失 → 批量加 `use std::sync::Arc;`
4. 收集剩余的 `NortHingError/NortHingResult` 缺失 → 批量加 import

预计 R58 完成后 `cargo check --tests` 应该 0 错误（除非有更深层设计问题）。

## 验证命令

```bash
# 1. lib 编译
cargo check -p northhing-core --features product-full --lib

# 2. test 编译 (主要目标)
cargo check -p northhing-core --features product-full --tests

# 3. workspace 全编译
cargo check --workspace

# 4. 错误文件清单 (生成 fix 优先级)
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cargo check -p northhing-core --features product-full --tests 2>&1 | Tee-Object test-errors.log
Select-String -Path test-errors.log -Pattern "^\s+-->\s+src.*\.rs" | ForEach-Object {
  $_.Line.Trim() -replace '^\s*-->\s+', '' -replace ':\d+:\d+\s*$', ''
} | Sort-Object -Unique
```

## 关键 commit 提示

- R57c: `2394f53b` main commit
- Mavis take-over: `ed99e791` (R57c tests fix), `bac458d1` (R55a mod tests fix)
- main HEAD: `bac458d1`
- branch tips: `git -C northing worktree list` 检查所有 `northing-impl-r5*` worktree

## 历史教训 (Mavis 笔记)

- **R55a/R55b/R55c/R55d/R55e/R57c** 的 take-over 模式: tests 文件被原样保留在 sibling 目录里，但 tests.rs 顶层就有 `mod tests {}` 包装，而 mod.rs 又有 `mod tests;` 声明 → 双层嵌套。
- 解决 pattern: 删除 tests.rs 里的 `mod tests {}` 包装 + 删除 mod.rs 里的重复 `#[cfg(test)]\nmod tests;` 声明。
- 还需要: 调整 tests.rs 里的 `use super::Type` 为 `use super::super::Type` (因为 Type 现在在 sibling 而不是 parent)

## 上下文恢复指令 (给下个 session)

如果下个 session 需要从这里接续：
1. `cd E:\agent-project\northing` (主 worktree, branch main)
2. `git log --oneline -10` — 看最新 commits
3. 读 `docs/handoffs/2026-07-10-r57-chain-completion.md` (本 handoff 的位置/名字待定)
4. 运行 `cargo check -p northhing-core --features product-full --tests 2>&1 | Tee-Object test-errors.log` 看当前错误状态
5. 按 Tier 1 → Tier 2 → Tier 3 顺序修复
