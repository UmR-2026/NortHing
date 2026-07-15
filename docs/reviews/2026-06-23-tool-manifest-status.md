# Tool Manifest 拆分 — 现状调查 (2026-06-23)

> **Author:** Auto-investigation (per user request: "现状调查 + 优化拆分策略")
> **Status:** Awaiting user review
> **Related spec:** `docs/superpowers/specs/2026-06-17-v3-prompt-loader-design-v2.md` §6 (open question #4)

---

## TL;DR

**Tool Manifest 拆分基础设施 100% 就位，且已经激进分类**：

- ✅ 20 个 tool 已标记为 `Collapsed`（按需 GetToolSpec 加载）
- ✅ ~19 个常用 tool 保持 `Expanded`（每轮注入完整 schema）
- ✅ `ToolExposure` enum + `resolve_tool_manifest_policy` 已实现
- ✅ `GetToolSpecTool` 已实现，模型可按需调用

**真实剩余 token 优化空间**：**~5-8K tokens**（不是 v2 spec 估的 10-15K）
- 通过重新分类 `task_tool` (131KB impl)、`bash_tool` (71KB)、`grep_tool` (47KB) 中等大小但相对小众的 tool

---

## 1. 现状：分类完整性

### 1.1 已 Collapsed 的 20 个 tool

| Tool | 实现大小 | 评估 |
|------|---------|------|
| `computer_use_tool` | 大 | ✅ 合理（computer use 不常用） |
| `control_hub_tool` | 中 | ✅ 合理 |
| `create_plan_tool` | 中 | ✅ 合理（plan mode 专属） |
| `cron_tool` | 中 | ✅ 合理 |
| `generative_ui_tool` | 中 | ✅ 合理 |
| `get_file_diff_tool` | 中 | ⚠️ 可考虑展开（常用） |
| `git_tool` | 中 | ⚠️ 可考虑展开（常用） |
| `log_tool` | 中 | ✅ 合理 |
| `mcp_tools` (×4) | 大 | ✅ 合理（按 server 加载） |
| `playbook_tool` | 中 | ✅ 合理 |
| `review_platform_tool` | 中 | ✅ 合理 |
| `session_control_tool` | 中 | ✅ 合理 |
| `session_history_tool` | 小 | ✅ 合理 |
| `session_message_tool` | 中 | ✅ 合理 |
| `terminal_control_tool` | 小 | ✅ 合理 |
| `web/fetch.rs` | 小 | ⚠️ 可考虑展开（常用） |
| `web/search.rs` | 小 | ⚠️ 可考虑展开（常用） |

### 1.2 已 Expanded（每轮注入）的 ~19 个常用 tool

| Tool | 实现大小 | 每轮 tokens (估) |
|------|---------|------------------|
| `bash_tool` | 71KB | ~600-800 |
| `file_read_tool` | 15KB | ~150-250 |
| `file_write_tool` | 27KB | ~250-400 |
| `glob_tool` | 17KB | ~200-300 |
| `grep_tool` | 47KB | ~400-600 |
| `skill_tool` | 23KB | ~200-350 |
| `task_tool` | **131KB** ⚠️ | **~800-1200** |
| `ask_user_question_tool` | 12KB | ~100-200 |
| `todo_write_tool` | 6KB | ~50-100 |
| `delete_file_tool` | 13KB | ~150-200 |
| `get_time_tool` | 5KB | ~50-80 |
| `ls_tool` | 12KB | ~100-180 |
| `thread_goal_tools` | 8KB | ~80-120 |
| `file_edit_tool` | 19KB | ~200-300 |
| `code_review_tool` | 55KB | ~400-600 |
| 其他 (miniapp, exec_command, ...) | - | ~500-800 |

**已 expanded 估算总和**：**~5,000-8,500 tokens/turn**

---

## 2. 优化机会分析

### 机会 A: 把 `task_tool` 改为 Collapsed（节省 ~800-1200 tokens）

**理由**：
- `Task` 是元工具（让主 agent 调用子 agent），**大部分 turn 不会用**
- 实现大小 131KB（最大），schema 复杂（subagent_type, context_mode, run_in_background, etc.）
- 当主 agent 需要用 Task 时，会主动 GetToolSpec（这是 collapsed 的正常 workflow）

**风险**：
- 中：模型可能在第一次用 Task 时多花 1 个 turn 来调用 GetToolSpec
- 缓解：GetToolSpec 是 trivial tool call，开销很小

### 机会 B: 把 `code_review_tool` 改为 Collapsed（节省 ~400-600 tokens）

**理由**：
- `code_review` 是 review-specific，不常用
- 类似 `review_platform_tool` 已经是 Collapsed，应该一致

**风险**：低

### 机会 C: 把 `bash_tool` 部分参数延迟加载

**理由**：
- bash_tool 有 71KB schema（parameters 复杂）
- 但 bash 是最常用的工具，collapsed 会降低效率

**风险**：高（不建议改）

### 机会 D: 把 `get_file_diff_tool`、`git_tool` 改为 Expanded（增加 ~300-500 tokens）

**理由**：
- 这两个工具在日常开发中很常用
- 但当前是 Collapsed，每次都要 GetToolSpec

**风险**：低（但增加 token）

---

## 3. 推荐的优化

**只推荐机会 A**：
- 把 `task_tool` 改为 Collapsed
- 节省 ~800-1200 tokens/turn
- 风险可控（GetToolSpec 是 trivial tool call）
- 总 token 节省：~1K tokens/turn（保守估计）

**不推荐 B/C/D**：
- B 节省小且 review 工具一致性问题低
- C 高风险
- D 增加 token

---

## 4. 风险评估

| 项 | 风险 | 缓解 |
|----|------|------|
| 模型不会主动 GetToolSpec | 中 | GetToolSpec 在 collapsed listing 中有 brief reminder |
| 模型 GetToolSpec 后忘记使用 | 低 | `validate_collapsed_tool_usage` 会强制要求 |
| Task 调用增加 1 个 turn 延迟 | 低 | 在大部分 turn 不调用 Task，净收益为正 |

---

## 5. 实施计划（如用户批准）

### 工作量估算
- 1 行代码修改（`task_tool.rs` 中 `ToolExposure::Collapsed`）
- 1 个测试更新（`task_tool.rs` 测试断言 `default_exposure() == Collapsed`）
- 1 个手动验证（agent listing 中的 Task tool 变成 stub）

### 预计时间
- 15-30 min（含验证）

---

## 6. 下一步

用户决定：
- 选择 A：批准任务 A（task_tool → Collapsed），执行 LAEP
- 选择 B：批准任务 B（code_review → Collapsed），执行 LAEP
- 选择 A+B：两个都做
- 选择 拒绝：当前分类已足够，标记 v3-P1 完结，转向 R1
- 选择 进一步调研：需要看真实 prompt 中的 tool listing size 才能精确估计

---

**Last updated:** 2026-06-23
**Status:** Awaiting user decision