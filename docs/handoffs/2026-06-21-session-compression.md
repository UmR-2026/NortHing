# Session Handoff — 2026-06-21

> **Compress target**: This file is the single source of truth for resuming work.
> **Last session**: K.2.5 closeout + K.2.2 phase3 split + K.2.3 follow-up deferred items + review fix
> **HEAD**: `b743eaa`
> **Branch**: `v3-restructure`

---

## 1. 本次完成的工作

| 任务 | 状态 | Commit | 说明 |
|---|---|---|---|
| K.2.5 Plan doc closeout | ✅ | `e4e0f2e` | TL;DR 更新，风险登记更新，验证命令 |
| K.2.2 phase3 split | ✅ | `6624161` | `execute_hidden_subagent_phase3` 拆分为 `persist_subagent_result` + `cleanup_subagent_and_return` |
| Review fix — restore backend test | ✅ | `b743eaa` | 恢复 `backend_error_maps_to_partial_timeout` 测试 + `structured_output.is_none()` 断言 |
| 67 compile errors fix | ✅ | `fa134d9` | 删除陈旧边界测试 |
| Clippy warnings | ✅ | `018e185` | `cargo clippy --fix` + 手动修复 |
| SubagentResult JSON parsing | ✅ | `e4e0f2e` | 新增 `structured_output: Option<serde_json::Value>` |

---

## 2. 代码关键位置（下次唤起直接跳转）

```rust
// A1 gate — coordinator.rs:4247-4285
// 当 USE_LIGHTWEIGHT_ACTOR=true 且 actor_runtime 存在时路由到 a1_path

// Phase3 helpers — coordinator.rs:4903-4985
async fn persist_subagent_result(...)      // 4942-4985
async fn cleanup_subagent_and_return(...)  // 4903-4940

// A1 mapping — a1_path.rs:140-180
pub(crate) fn map_lightweight_to_subagent_result(...)

// LongRunningSkill trait — agent-dispatch/src/long_running.rs
// ActorRuntime::spawn_long_running — agent-dispatch/src/runtime.rs:426-509
```

---

## 3. 待决策事项（用户必须选择）

**当前所有 4 个 const flag 仍为 `false`**，A1 路径未实际启用。

| 选项 | 工作量 | 价值 | 说明 |
|---|---|---|---|
| **A. 实现真正的 `CoordinatorHiddenSubagentSkill`** | 2-3 天 | 高 | 替换 `A1StubSkill`，包装 phase1/2/3 为 `LongRunningSkill` |
| **B. Remake R1 — Shell-exec sandbox** | 2 天 | **最高** | 安全审计，S-1 + P3-2 |
| **C. v3 Phase 1 — Prompt loader** | 1-2 天 | 高 | 最大 token 节省 (~40-65K → ~5K) |
| **D. K.2.4 — Mock display test** | 2-3h | 低 | 被 slint 1.16.1 阻塞 |

**推荐顺序**: B → C → A → D

---

## 4. 验证命令（唤起后先执行）

```bash
cd /e/agent-project/northhing

# 1. 状态确认
git status
git log --oneline -5

# 2. 快速健康检查
cargo check -p northhing --lib 2>&1 | tail -5
cargo test -p northhing-agent-dispatch --lib 2>&1 | tail -5
cargo test -p northhing --lib 2>&1 | tail -5

# 3. 全量回归
bash scripts/regression-test-desktop.sh
```

**Expected**: 8/8 regression, 24/24 agent-dispatch, 12/12 desktop, clean tree, HEAD `6624161`.

---

## 5. 已知风险

| ID | 描述 | 严重度 |
|---|---|---|
| R-NEW-2 | `SubagentResult.structured_output` 尚无消费者 | 低 |
| R-BLOCK-1 | slint 1.16.1 无 `backend-testing` | 中（阻塞 K.2.4） |
| R-ARCH-2 | `GetToolSpecTool` 已弃用但仍使用 | 中（接受） |

---

## 6. 唤起钩子

下次会话开始时，执行以下操作即可恢复上下文：

1. 读取本文件 `docs/handoffs/2026-06-21-session-compression.md`
2. 执行 §4 验证命令确认状态
3. 询问用户选择 §3 中的选项（A/B/C/D）
4. 根据选择进入 Plan Mode 或直接开始实现

无需回顾历史 session 内容 — 本文件已包含所有必要状态。
