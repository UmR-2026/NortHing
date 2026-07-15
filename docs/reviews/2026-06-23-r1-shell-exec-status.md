# R1 Shell-exec Sandbox — 现状调查 (2026-06-23)

> **Author:** Auto-investigation
> **Status:** Awaiting user decision
> **Related:** handoff 中 R1 = "Shell-exec sandbox + confirm audit"

---

## TL;DR

**R1 基础设施大部分已实现**，但**有两个关键缺口**：

| 项 | 状态 | 来源 |
|----|------|------|
| S-1 Shell denylist (11 patterns) | ✅ | `shell_safety.rs`, commit `e2cb1bd` |
| `BashTool::needs_permissions() = true` | ✅ | `bash_tool.rs:354` |
| `round_executor` reads `needs_permissions` → `confirm_before_run` | ✅ | `round_executor.rs:750-770` |
| `ToolConfirmationPlan/Outcome` framework | ✅ | `tool_confirmation.rs` |
| `skip_tool_confirmation` config | ⚠️ **默认 `true`** | `config/types.rs:1598` |
| 审计日志（allow/deny/confirm history） | ❌ 缺失 | - |
| 其他 shell-exec 路径 (computer_use_actions, browser_launcher, ngrok, lsp/process, mcp/connection, miniapp runtime) | ❌ 未审计 | - |

---

## 1. 已实现的 S-1 部分

### 1.1 Shell denylist (commit `e2cb1bd`)

`src/crates/assembly/core/src/agentic/tools/implementations/shell_safety.rs`:
- 11 条 regex pattern 覆盖：
  - `rm -rf /` (含 `--no-preserve-root`)
  - `rm -rf ~` / `rm -rf ../`
  - `mkfs` on `/dev/`
  - `dd` of=`/dev/[sh]d`
  - `> /dev/[sh]d` 重定向
  - `shutdown`/`reboot`/`halt`/`poweroff`
  - fork bomb `:(){:|:&};:`
  - `curl|wget` pipe to shell
  - `fdisk|parted|gdisk` on `/dev/`

**调用点**：`bash_tool.rs:382-396` 在 `validate_input` 中检查 denylist（在 confirmation 之前 fail-fast）。

### 1.2 Confirmation framework

`src/crates/execution/agent-runtime/src/tool_confirmation.rs`:
- `ToolConfirmationRequestFacts { confirm_before_run, tool_needs_permission, ... }`
- `ToolConfirmationPlan::{ Skip, Await { timeout_at, timeout_secs } }`
- `ToolConfirmationOutcome::{ Confirmed, Rejected, ChannelClosed, Timeout }`
- `resolve_tool_confirmation_plan()` → 根据 `confirm_before_run && tool_needs_permission` 决定 `Skip` 还是 `Await`

**调用点**：
- `round_executor.rs:719-770` 读 config，决定 `needs_confirmation`
- `tool_pipeline.rs:668-670` 把 facts 传给 `resolve_tool_confirmation_plan`

### 1.3 `needs_permissions()` impl

- `BashTool::needs_permissions() = true` (`bash_tool.rs:354`)
- 其他 ~25 个 tool 都实现了 `needs_permissions` (默认是 `false`)
- `round_executor.rs:755-762` 在循环里检查每个 tool_call

---

## 2. R1 关键缺口

### 2.1 ⚠️ `skip_tool_confirmation` 默认是 `true`

**File**: `src/crates/assembly/core/src/service/config/types.rs:1598`
```rust
skip_tool_confirmation: true,  // default!
```

**含义**：默认情况下 confirmation **完全跳过**，即使 `BashTool::needs_permissions() = true`。

**用户影响**：
- 默认配置下，shell 命令（包括潜在危险命令如 `rm -rf build/`）直接执行，无用户确认
- Denylist 仍然 fail-fast，但 confirmation 完全 disabled

**修复选项**：
- A. 改默认 `skip_tool_confirmation: false`（**单行改动，但行为变化大**）
- B. 保留默认但加 deprecation warning + docs 说明
- C. 按 agent 类型分流（不同 agent 不同 skip policy）

### 2.2 ❌ 审计日志缺失

**缺口**：denylist 检查 + confirmation 决策**没有持久化审计日志**。
- 命令被允许：日志？
- 命令被 denylist 拒绝：日志？
- 命令需要 confirmation 但被 skip：日志？
- 命令需要 confirmation 且 user 拒绝：日志？

**修复方向**：写入 `.northhing/audit.log` 或通过 `tracing` 层。

### 2.3 ❌ 其他 shell-exec 路径未审计

**所有 `Command::new()` 调用点**（除 bash_tool.rs 已加 denylist 外）：
- `computer_use_actions.rs` - mouse/keyboard commands
- `browser_launcher.rs` - launch browser
- `ngrok.rs` - tunnel management
- `lsp/process.rs` - LSP server
- `mcp/server/connection.rs` - MCP server connections
- `miniapp/runtime.rs` - miniapp JS worker
- `process_manager.rs` - generic process mgmt
- `glob_search.rs` - search shell-out
- `port_adapters.rs` - function agent

**风险**：这些路径可能：
- 不走 denylist 检查
- 不走 confirmation
- 直接 LLM 触发执行

**审计范围**：每个文件 + 行为 + 风险等级。

---

## 3. R1 推荐工作拆分

### 子任务 1: Audit Pass (Phase 1) — 1-2 天

对每个 shell-exec 路径：
1. 标记当前是否有 denylist 检查
2. 标记当前是否有 confirmation
3. 标记是否 LLM 可直接触发
4. 评估风险（高/中/低）
5. 输出 audit report

### 子任务 2: Denylist 扩展（Phase 2）— 0.5-1 天

基于 audit 结果，给缺失的路径加 denylist 检查（统一通过 `shell_safety::check_command_denied`）。

### 子任务 3: Confirmation 默认值决策 — 半天

- 选项 A：`skip_tool_confirmation: false` 默认（行为变化大，需用户回归测试）
- 选项 B：保留默认 + docs + deprecation warning
- 选项 C：按 agent type 分流（需要新配置 schema）

### 子任务 4: 审计日志（Phase 4）— 0.5-1 天

- 添加 `tracing` span 包住 shell-exec 路径
- 命令被允许/拒绝/confirm 时记录 event
- 写到 `.northhing/audit.log`

---

## 4. 风险评估

| 项 | 当前风险 | 修复后风险 |
|----|---------|----------|
| Catastrophic commands (rm -rf /) | 已 denylist | 已 denylist |
| Catastrophic via other paths (browser_launcher etc.) | 高 | 低 |
| Confirmation disabled by default | 中 | 低 |
| No audit trail for forensic | 中 | 低 |
| Confirmation UX 完整性 | 未知（需验证） | 已知 |

---

## 5. 推荐方案

**Phase 1: Audit Pass（必做）**
- 不改任何代码
- 输出 markdown audit report (`docs/security/r1-shell-exec-audit.md`)
- 给每个路径打分 + 修复建议
- 1-2 天

**Phase 2: Denylist 扩展（按 audit 结果）**
- 在每个缺失的路径加 denylist 检查
- 复用 `shell_safety::check_command_denied`
- 1-2 天

**Phase 3: 默认值 + 审计日志（最后）**
- 决定 `skip_tool_confirmation` 默认值
- 加 audit log
- 0.5-1 天

**总估计**：3-5 天

---

## 6. 用户决策点

请选择：

- **A**：完整 R1（Phase 1 + 2 + 3）— 3-5 天
- **B**：只做 Phase 1 Audit Pass — 1-2 天，先看 audit 报告再决定
- **C**：只做 Phase 2 denylist 扩展（不做 audit） — 1-2 天
- **D**：标 R1 为大部分完成，转向其他任务（确认 R1 已 70% 完成）

---

**Last updated:** 2026-06-23
**Status:** Awaiting user decision