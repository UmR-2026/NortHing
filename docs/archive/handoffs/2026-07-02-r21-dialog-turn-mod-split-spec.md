# R21 Spec: dialog_turn/mod.rs 1653 → facade ~700 + 4 sibling 扩展（parallel sub-rounds）

> **目标**: 把 `src/crates/assembly/core/src/agentic/coordination/dialog_turn/mod.rs` (1653 行) 拆开：单 impl block L187-L1644 中 51 个 public method 按 sub-domain 下沉到现有 sibling, mod.rs 缩到 ~700 行 facade
> **风险**: MEDIUM（god-object 二次拆分, R6 + R7 已拆过, mod.rs 长大但 sibling 结构稳定）
> **新流程** (2026-07-02 决策): 4 sub-rounds **并行** 跑 + producer self-report + Mavis 3-axis verify + 1 squash-merge / round
> **预计时长**: ~2-2.5h（spec 10min + 4 producer 并行 90min + Mavis r21e 30min + verify 20min + squash 5min）

---

## §0 前置状态（实测 baseline, 2026-07-02）

| 项 | 值 |
|---|---|
| `dialog_turn/mod.rs` | **1653 行** (canonical wc -l) |
| `dialog_turn/workspace.rs` | 398 行 |
| `dialog_turn/session.rs` | 253 行 |
| `dialog_turn/turn.rs` | 690 行 |
| `dialog_turn/turn_subhandlers.rs` | 806 行 (R7 拆出, 超 800 cap 6 行, R21 scope 外) |
| `dialog_turn/compaction.rs` | 255 行 |
| `dialog_turn/restore.rs` | **2 行**（空 placeholder, R6 拆时建但从未迁入代码） |
| `dialog_turn/thread_goal.rs` | 211 行 |
| **Total** | **4268 行** |
| R6 历史 spec | `docs/handoffs/2026-06-28-round6-dialog-turn-split-spec.md` (目标 ~2950, 实际 +1300 是 R7 turn_internal 709 → 4 sub-handlers) |
| R7 历史 commit | `79b496b refactor(turn-internal): extract start_dialog_turn_internal (709 lines) into 4 sub-handlers` |

**mod.rs 单 impl block `impl ConversationCoordinator`** 起始 L187, 装载 **51 个 public method**：
- 6 config setter + getter (L188-289)
- 8 session CRUD (L291-463) — 已 delegate 到 session.rs, body 1-3 行
- 1 ensure_assistant_bootstrap (L465-583)
- 4 start_dialog_turn_* (L584-698) — facade dispatch 到 turn.rs
- 12 thread_goal_* (L699-1041)
- 1 compact_session_manually (L1042-1234, **193 行单 method**)
- 4 cancel/delete (L1235-1425)
- 8 restore_* (L1426-1570) — **mod.rs 有实现, restore.rs 空**
- 9 misc (L1571-1644, list_sessions/get_messages/subscribe_internal/confirm_tool/reject_tool/cancel_tool)

**对话 turn subagent_orchestrator.rs 也有 `impl ConversationCoordinator`**（R6 §7 E3 历史, R21 不动）

---

## §1 R21 拆分方案（4 sub-rounds 并行 + Mavis r21e 后处理）

### §1.1 sub-round 总览

| ID | 名称 | mod.rs 改 line 段 | 目标 sibling | 预计行数变化 |
|---|---|---|---|---|
| **r21a** | restore-revival | L1426-1570 (8 method) | `restore.rs` 2 → ~250 | mod.rs -150 / restore.rs +250 |
| **r21b** | turn-control-extract | L1235-1425 (4 method) | `turn.rs` 690 → ~950 | mod.rs -200 / turn.rs +260 |
| **r21c** | session-ext-and-tool | L1571-1644 (9 method) | `session.rs` 253 → ~450 | mod.rs -100 / session.rs +200 |
| **r21d** | thread-goal-consolidate | L699-1041 (12 method) | `thread_goal.rs` 211 → ~700 | mod.rs -350 / thread_goal.rs +500 |
| **r21e** | mod-cleanup（Mavis） | L1-185 (顶层 fn + struct + use) | `helpers.rs` (新) + 各 sibling | mod.rs -300 / 新 sibling +300 |

### §1.2 mod.rs 改后预期

| 段 | 内容 | 预计行数 |
|---|---|---|
| L1-185 | use / 常量 / struct / 顶层 fn (r21e 拆到 helpers.rs) | 0 (迁空) |
| L187-289 | impl block: new + 6 config setter + thread_goal_runtime | ~110 行 |
| L291-583 | facade: 8 session CRUD + ensure_assistant_bootstrap | ~300 行 |
| L584-698 | facade: 4 start_dialog_turn_* | ~115 行 |
| L699-1041 | facade: 4 thread_goal_* (其他 8 delegate) | ~200 行 |
| L1042-1234 | facade: compact_session_manually (delegate 到 compaction.rs) | ~15 行 |
| L1235-1570 | **删除**（r21a + r21b 已迁） | 0 |
| L1571-1644 | **删除**（r21c 已迁） | 0 |
| L1645-1653 | pub mod 声明 | 9 行 |
| **Total mod.rs** | | **~700-750 行** (vs 1653, -57%) |

---

## §2 4 sub-rounds 详细 spec

### §2.1 r21a restore-revival

**目标**: 复活空 `restore.rs`，把 8 个 restore_* method 从 mod.rs 迁入。

**mod.rs 改 line 段**: L1426-1570（严格，不越界）

**目标 sibling 文件**: `restore.rs` (从 2 行 → ~250 行)

**迁入 method 清单**:
- `restore_session` (L1426)
- `restore_internal_session` (L1436)
- `restore_session_with_turns` (L1447)
- `restore_internal_session_with_turns` (L1457)
- `restore_session_view` (L1468)
- `restore_session_view_timed` (L1478)
- `restore_session_view_tail` (L1492)
- `restore_session_view_tail_timed` (L1503)
- `restore_internal_session_view` (L1519)
- `restore_internal_session_view_timed` (L1529)
- `restore_internal_session_view_tail` (L1543)
- `restore_internal_session_view_tail_timed` (L1554)

实际是 12 个（rg 漏了 4 个）。mod.rs L1426-1570 共 145 行。

**实施模式**:
- `restore.rs` 用 `impl ConversationCoordinator { pub(super) async fn method_name(&self, ...) { ... } }` pattern（参考 R20 manager_*.rs）
- mod.rs 留 facade: `pub async fn method_name(&self, ...) -> Result<...> { self.restore_method_name(...).await }`
- 字段访问：R6 已提升 `pub(crate)`, 当前直接可见

**producer self-report**:
- restore.rs 行数 (before 2 → after ~250)
- mod.rs L1426-1570 段删除确认（before 145 行 → after 12 行 facade delegate）
- 12 个 method 都迁了（不是 delegate 后空 body）
- pub(super) vs pub 选择理由
- BOM / CRLF = 0
- long line count (>120 char, ≤5 R18+ tolerance)
- `cargo check -p northhing-core --features product-full --lib --message-format=short` 0 errors

### §2.2 r21b turn-control-extract

**目标**: 把 turn 取消 / delete 4 method 迁入 `turn.rs`, turn.rs 拥有 turn 生命周期全权。

**mod.rs 改 line 段**: L1235-1425（严格）

**目标 sibling 文件**: `turn.rs` (从 690 → ~950 行)

**迁入 method 清单**:
- `cancel_dialog_turn` (L1235, ~100 行)
- `cancel_active_turn_for_session` (L1336, ~40 行)
- `delete_session` (L1375, ~15 行)
- `delete_hidden_subagent_sessions_for_parent_turns` (L1390, ~36 行)

**约束**:
- turn.rs 已有 start_dialog_turn_* facade delegate + turn_subhandlers.rs sub-handlers
- 4 个 method 涉 turn 取消 / session 删除, 写入 `impl ConversationCoordinator { pub(super) async fn }` block 在 turn.rs
- mod.rs 留 facade delegate（4 行 method body）
- turn.rs 加完后 **必须 ≤ 1000 行**（reviewer precedent: turn_subhandlers.rs 806 已超 800 但 R7 reviewer 例外接受 ≤1000）

**producer self-report**: 同 r21a + turn.rs 行数变化 + ≤1000 cap 验证

### §2.3 r21c session-ext-and-tool

**目标**: 把 session 周边 + tool control 9 method 迁入 `session.rs`。

**mod.rs 改 line 段**: L1571-1644（严格）

**目标 sibling 文件**: `session.rs` (从 253 → ~450 行)

**迁入 method 清单**:
- `list_sessions` (L1571)
- `resolve_session_workspace_path` (L1578)
- `get_messages` (L1588)
- `get_messages_paginated` (L1593)
- `subscribe_internal` (L1607)
- `unsubscribe_internal` (L1618)
- `confirm_tool` (L1623)
- `reject_tool` (L1634)
- `cancel_tool` (L1639)

**注意**: `confirm_tool` / `reject_tool` / `cancel_tool` 严格说是 tool control, 但只有 3 method, 单独 sibling 不划算, 暂合在 session.rs。如果 reviewer 不同意, R22 可拆 tool_control.rs。

**producer self-report**: 同 r21a + session.rs 行数变化

### §2.4 r21d thread-goal-consolidate

**目标**: 把 12 个 thread_goal_* method 中 8 个下沉到 `thread_goal.rs`, facade 留 4 个核心。

**mod.rs 改 line 段**: L699-1041（严格）

**目标 sibling 文件**: `thread_goal.rs` (从 211 → ~700 行)

**facade 保留 4 个核心**（import / basic getter, 业务入口）:
- `get_thread_goal` (L699, getter)
- `clear_thread_goal` (L709)
- `create_thread_goal` (L722)
- `pause_thread_goal_after_user_cancel` (L896, turn 取消触发)

**下沉 8 个**（业务实现 / 复杂 update / lifecycle）:
- `update_thread_goal_objective` (L740, ~46 行)
- `set_thread_goal_objective` (L786, ~42 行)
- `maybe_mark_thread_goal_usage_limited` (L828, ~36 行)
- `set_thread_goal_status` (L864, ~32 行)
- `update_thread_goal_status` (L936, ~14 行)
- `emit_thread_goal_updated` (L950, ~9 行)
- `activate_session_goal` (L959, ~32 行)
- `prepare_goal_continuation_after_turn` (L991, ~51 行)

**facade delegate 模式**:
```rust
// mod.rs L740
pub async fn update_thread_goal_objective(&self, session_id: &str, new_objective: String) -> NortHingResult<()> {
    self.update_thread_goal_objective_impl(session_id, new_objective).await
}
```

**注意**: mod.rs 已有同名 method, impl block 内叫 `update_thread_goal_objective`, thread_goal.rs 用不同名 `update_thread_goal_objective_impl` 或 `update_thread_goal_objective_inner`。Rust 允许同名 method 在不同 file 的 `impl ConversationCoordinator` block, 但要避免与 facade method 签名冲突。

**R20 manager_*.rs precedent**: R20 manager_session_lifecycle.rs 用 `pub(super) async fn method_name`, 不带 `_impl` 后缀, 因为是不同 impl block 同名 method。

**producer self-report**: 同 r21a + thread_goal.rs 行数变化 + 同名 method 不冲突验证（`cargo check` 0 errors）

### §2.5 r21e mod-cleanup (Mavis 后处理)

**目标**: 收尾 mod.rs L1-185 顶层段。

**Mavis 范围**:
- L82-85 常量 `MANUAL_COMPACTION_COMMAND` / `CONTEXT_COMPRESSION_TOOL_NAME` / `DEFAULT_SUBAGENT_MAX_CONCURRENCY` / `MAX_SUBAGENT_MAX_CONCURRENCY` → 各 sibling（compaction.rs / turn.rs 等）
- L87-99 `WrappedUserInputPayload` struct + `SkillAgentSnapshotPersistence` enum → turn_subhandlers.rs 或新 helpers
- L101-175 5 个顶层 fn → 各 sibling 或新 helpers.rs
  - `format_background_subagent_delivery_text` → turn_subhandlers.rs (背景 subagent 相关)
  - `format_background_subagent_display_text` → turn_subhandlers.rs
  - `build_subagent_session_relationship` → turn_subhandlers.rs
  - `fork_subagent_system_reminder` → turn_subhandlers.rs
  - `runtime_tool_restrictions_for_delegation_policy` → helpers.rs 或 turn_subhandlers.rs

**Mavis 时机**: 4 producer commit + verify PASS 后, Mavis 单人做 r21e, 不进 team plan

---

## §3 visibility 与 import 规则

### §3.1 字段 visibility

R6 已提升 8 字段 `pub(crate)`, 当前 `ConversationCoordinator` 字段全部 `pub(crate)`, sibling 直接可见。

**不要** 再加 `pub(super)` 给字段 — sibling 已能访问。

### §3.2 sibling method visibility

按 R20 manager_*.rs precedent:

```rust
// restore.rs / turn.rs / session.rs / thread_goal.rs
impl ConversationCoordinator {
    pub(super) async fn method_name(&self, ...) -> ... {
        // 直接访问 self.x.y() 字段
    }
}
```

**不要** 用 `pub fn` 在 sibling (避免跨 crate 暴露, R19 教训)。

### §3.3 facade delegate 模式

```rust
// mod.rs
pub async fn method_name(&self, ...) -> Result<...> {
    self.method_name(...).await  // delegate 到 sibling impl block 同名 method
}
```

### §3.4 use 导入

- sibling 内访问同 crate type: `use super::*;` 或精确 `use super::{ConversationCoordinator, ...}`
- 不要 `use super::super::*;`（按 R5/R6 教训）

---

## §4 producer 并行约束

### §4.1 file ownership（互不重叠）

| Producer | 写 | 读 |
|---|---|---|
| r21a | `restore.rs` 全权 | `mod.rs` L1426-1570 段（其他段只读） |
| r21b | `turn.rs` 全权 | `mod.rs` L1235-1425 段 |
| r21c | `session.rs` 全权 | `mod.rs` L1571-1644 段 |
| r21d | `thread_goal.rs` 全权 | `mod.rs` L699-1041 段 |

**mod.rs 不同 line 段同时被 4 producer 改, 但段不重叠**:
- r21a: L1426-1570
- r21b: L1235-1425
- r21c: L1571-1644
- r21d: L699-1041
- mod.rs 其他段: 4 producer 都只读

### §4.2 worktree 隔离

每个 producer 在独立 git worktree:
- `impl/r21a-restore-revival`
- `impl/r21b-turn-control-extract`
- `impl/r21c-session-ext-and-tool`
- `impl/r21d-thread-goal-consolidate`

### §4.3 Cargo.lock

- producer 不要 `cargo update`
- 只跑 `cargo check` (不改 lock)
- 4 worktree 后由 Mavis 在 main HEAD 一次性 `cargo check --workspace` 锁 Cargo.lock

### §4.4 timeout

- 每 producer `timeout_ms: 5400000` (90 min), engine cap 30 min, Mavis 监控 + extend-timeout 如需要

---

## §5 Mavis 3-axis verify (替代 10-axis)

| Axis | 命令 | PASS 标准 |
|---|---|---|
| 1. 编译过 | `cargo check --workspace --message-format=short` | 0 errors |
| 2. 跨 crate 测过 | `cargo check -p northhing-cli` + `cargo check -p northhing-desktop` + `cargo check -p northhing-server` | 0 errors (R19 教训: workspace check 漏跨 crate) |
| 3. 不退化 | `cargo test -p northhing-core --features product-full --lib` | 0 failed (baseline 同 HEAD 跑) |

**其他 7 axis (line cap / long line / BOM / visibility / pub(super) / cross-ref / spec drift) 由 producer self-report, Mavis 不再独立跑**。

---

## §6 squash-merge + stage-summary

### §6.1 squash 顺序

1. 4 producer commit + push worktree branch
2. Mavis 4 个 worktree 顺序 merge 到 main（保持 4 个独立 commit 便于回溯）, **不** 用 squash（squash 在用户口头确认 review 通过后）
3. 用户找 QClaw + Kimi review（如需要）
4. review 通过后, Mavis 用 `git merge --squash` + 1 squash-merge commit 到 main
5. Mavis 写 `docs/handoffs/2026-07-02-r21-stage-summary.md`

**注**: R21 不一定 squash, 沿用 R20 pattern (4 sequential merge + 1 squash at end)。Mavis 决定是 sequential merge 还是 squash, 看 review 反馈。

### §6.2 stage-summary 必填

- sub-round 列表 + commit hash
- 各 sub-round self-report 关键数字（line count delta / visibility / cross-crate）
- QClaw verdict (如有)
- Kimi verdict (如有)
- Mavis 3-axis verify 结果
- 合并 commit hash

---

## §7 Errata

### E1: restore.rs R6 拆时空 placeholder

**事实**: R6 spec §2.2 列出 restore.rs ~300 行, 实际只创建 2 行 placeholder 文件, method body 留在 mod.rs。

**Mitigation**: R21 r21a 复活 restore.rs, 把 12 个 restore_* method (R6 spec 说 15+, 实际 12) 迁入。

### E2: mod.rs L1042-1234 compact_session_manually 193 行单 method

**事实**: 1 个 method 占 193 行, 不在本 R21 scope 拆 method 内部。

**Mitigation**: r21e (Mavis) 后续可下沉 method body 到 compaction.rs, R21 只让 facade delegate。

### E3: turn_subhandlers.rs 806 行超 800 cap 6 行

**事实**: R7 已部分紧致, R21 scope 外。

**Mitigation**: R22 候选目标。

### E4: subagent_orchestrator.rs 也有 `impl ConversationCoordinator` (R6 §7 E3)

**事实**: R21 不动 subagent_orchestrator.rs。

**Mitigation**: R6 E3 已记录, R21 维持。

### E5: producer 并行改 mod.rs 不同 line 段 vs merge 冲突

**风险**: 4 producer 在 4 worktree 都改 mod.rs, merge 时可能冲突。

**Mitigation**:
- spec §4.1 严格 line 段 ownership
- merge 顺序: 先 r21d (L699-1041), 再 r21b (L1235-1425), 再 r21a (L1426-1570), 最后 r21c (L1571-1644)
- 4 段不重叠, 但 mod.rs 中 `impl ConversationCoordinator {` 关键字在 L187, r21a/r21b/r21c/r21d 都不改 L187 附近
- 如 merge 真冲突, Mavis 手工解（按 line 段所有权判, 不需 producer 重做）

### E6: producer commit 后用户 review 周期

**事实**: R20 模式是 user-driven review (QClaw 9.2/10 verbal + commit, Kimi 8.6/10 verbal 不 commit)。

**Mitigation**: R21 producer commit 后, Mavis 通知用户启动 review cycle。Review 通过前不 squash merge。

---

## §8 不在范围

- 不拆 `compact_session_manually` 193 行 method 内部 (R22 候选)
- 不拆 `turn_subhandlers.rs` 806 行超 cap (R22 候选)
- 不动 `subagent_orchestrator.rs` `impl ConversationCoordinator` (R6 E3)
- 不动 `coordinator.rs` / `ports.rs` / `scheduler.rs` / `a1_path.rs` / `state_manager.rs`
- 不动 `mod.rs` 中 51 个 public method 的签名
- 不做 cargo fmt 大范围扫尾 (pre-existing 156 行未提交 cargo fmt 改动是项目历史, R21 不碰)

---

## §9 时间预算（per-sub-round 90 min, 4 并行）

```
[0-10 min]   Mavis 写 spec → commit
[10 min]     Mavis 派 4 producer 并行
[10-100 min] 4 producer 同时跑各自 worktree
[100-120 min] producer commit + push worktree branch
[120-150 min] Mavis merge 4 worktree → main (sequential 4 commit, mod.rs 4 段)
[150-180 min] Mavis r21e mod.rs cleanup (顶层 fn/struct/常量)
[180-210 min] Mavis 3-axis verify
[210-240 min] Mavis 写 stage-summary + 等用户 review 信号
```

---

## §10 Owner

- **Owner**: Mavis (orchestrator)
- **Producer**: 4 个 sub-agent, `minimax/MiniMax-M2.7` (非 highspeed), 4500 calls / 5h 预算
- **Reviewer**: QClaw (user-driven, external) + Kimi (user-driven, external)
- **Final arbitration**: Mavis (after QClaw + Kimi verdicts)