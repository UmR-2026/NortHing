# Round 7 Spec — `start_dialog_turn_internal` 709 → 4 sub-handlers

> **Status**: Draft (Mavis 2026-06-28 凌晨准备，给新 session 用)
> **Trigger**: QClaw Round 6 review observation #1 (COND APPROVE 必做项)
> **Target**: `dialog_turn/turn.rs:537-1246` `start_dialog_turn_internal` (709 行)
> **Goal**: turn.rs 1352 → ≤ 1000 行

---

## 1. 当前状态

### 1.1 文件 / 方法 / 行数

| 指标 | 值 | 出处 |
|---|---|---|
| `dialog_turn/turn.rs` 总行数 | 1352 (`ReadAllLines`) / 1258 (analyzer 排除空行) | QClaw review §3.1 |
| `start_dialog_turn_internal` 行数 | **709** (line 537-1246) | QClaw review §3.4 |
| turn.rs 总方法数 | 13 (含 start_dialog_turn_internal + 12 helpers) | split-analyzer |
| start_dialog_turn_internal 共享字段 | 30+ 局部变量 + 13 个 `self.xxx` 字段 | 代码观察 |
| 关键共享字段 | `session_manager`, `event_queue`, `event_router`, `execution_engine`, `tool_pipeline`, `thread_goal_runtime`, `subagent_concurrency_limiter`, `subscribers`, `next_subscriber_id`, `active_turns_per_session`, `round_injection_source`, `active_subagent_executions`, `scheduler_notify_tx` | coordinator.rs:541 |
| §7 E1 cap | 1000 行 | spec §7 E1 |
| 当前超出 | 352 行 (35%) | QClaw review §1 |

### 1.2 turn.rs 现有 13 方法

```
L89-97     ensure_user_message_metadata_object           ( 9 lines)
L98-117    assistant_bootstrap_kickoff_query           (19 lines)
L106-118   is_chinese_locale                            (12 lines)
L119-130   assistant_bootstrap_system_reminder         (11 lines)
L131-248   finalize_turn_in_workspace                   (117 lines)
L249-326   persist_completed_dialog_turn               (77 lines)
L327-403   persist_cancelled_dialog_turn               (76 lines)
L404-498   persist_failed_dialog_turn                  (94 lines)
L499-535   finalize_persisted_turn_in_workspace_if_needed (31 lines)
L537-1246  start_dialog_turn_internal                   (709 lines)  ← 目标
L1247-1262 wait_session_drained                        (17 lines)
L1263-1294 cancel_active_subagents_for_parent_turn     (31 lines)
L1295-1351 stop_active_subagent_execution              (56 lines)
```

### 1.3 Round 6 经验 (避免重复)

| 错误类 | Round 6 hit | Round 7 防御 |
|---|---|---|
| Import 路径 super::super vs super | 4 处 | 新 sibling file 默认用 `use super::super::*` (parent), `use super::*` 仅 siblings |
| Sibling 方法 private | 16 处 | bulk promote via Python script (`promote-sibling-visibility.py`) |
| Struct 字段 private | 1 处 (WrappedUserInputPayload) | 跨 sub-handler 共享 `TurnContext` struct → 字段 `pub(crate)` |
| Cargo.lock drift | 2 E0308 (rmcp 1.7.0→1.8.0) | Plan YAML preflight step: 在 main HEAD 跑 baseline cargo check |
| cargo check stop-at-first-error | 32+ dialog_turn errors 被掩盖 | worker 必须 **在所有上游 crate 通过编译后** 才报"0 NEW errors" |
| M3 模型慢 | 39 min 沉默 | Plan YAML 强制 `model: minimax/MiniMax-M2.7-highspeed` |
| Plan engine abort fail | 50001 internal error | 用 `mavis team plan cancel` 不依赖 `mavis session abort` |

---

## 2. 拆分方案

### 2.1 sub-handler 切分 (per spec §7 E1 Alternative)

`start_dialog_turn_internal` 拆为 4 个 private async fn:

| sub-handler | 职责 | 估计行数 | 依赖字段 |
|---|---|---|---|
| `prepare_turn` | 初始化 TurnContext, 解析 workspace, 加载 session state, 调度前置检查 (abort token, subagent concurrency, thread_goal pre-state) | ~150 行 | session_manager, event_queue, thread_goal_runtime, subagent_concurrency_limiter, active_turns_per_session |
| `dispatch_turn` | 调用 ExecutionEngine::tick() 主循环，处理 model round, tool calls, side questions, remote injection | ~400 行 | execution_engine, tool_pipeline, round_injection_source, event_router |
| `finalize_turn` | 处理 TurnOutcome, 持久化 completed/cancelled/failed, 更新 thread_goal status, 事件路由 emit | ~120 行 | session_manager, event_queue, thread_goal_runtime, scheduler_notify_tx |
| `cleanup_turn` | 释放 subagent concurrency, 清理 active_turns_per_session, 处理 abort token, 路由 final event | ~50 行 | subagent_concurrency_limiter, active_turns_per_session, event_router, subscribers, next_subscriber_id |

**total**: 4 sub-handlers ~720 行 (vs 原 709 行集中)

### 2.2 TurnContext struct (跨 sub-handler 共享状态)

```rust
// 新增到 dialog_turn/turn.rs (or 新 sibling turn_subhandlers.rs)
pub(crate) struct TurnContext<'a> {
    pub coordinator: &'a ConversationCoordinator,
    pub session_id: String,
    pub turn_id: String,
    pub turn_kind: DialogTurnKind,
    pub trigger_source: DialogTriggerSource,
    pub workspace_path: PathBuf,
    pub session: Session,
    pub turn_start: Instant,
    // 局部 mutable state 在 sub-handler 间流转
    pub abort_token: CancellationToken,
    pub cancellation_observed: bool,
    // ... 根据实际代码补
}
```

`TurnContext` 字段 `pub(crate)` 让 sub-handler 之间通过 `&mut TurnContext` 流转。

### 2.3 文件结构

**Option A** (推荐, sub-domain split 第 7 层):
- 新增 `dialog_turn/turn_subhandlers.rs` 包含 4 sub-handlers + TurnContext struct
- `turn.rs` 保留 12 个 helpers + 1 个 wrapper `start_dialog_turn_internal` (now ~30 行)

```
dialog_turn/
├── mod.rs                    1652 行 (facade)
├── workspace.rs              398 行
├── session.rs                253 行
├── turn.rs                   ~640 行 (12 helpers + 1 wrapper)
├── turn_subhandlers.rs       ~750 行 (4 sub-handlers + TurnContext)
├── compaction.rs             255 行
├── thread_goal.rs            211 行
└── restore.rs                  2 行
```

预计 turn.rs: 1352 - 709 + 30 = ~673 行 ✅ ≤ 1000

**Option B** (保守, in-place split):
- `turn.rs` 保留所有内容，4 sub-handlers 仍在同一文件
- 估计 turn.rs = 1352 行不变 (sub-handler 还在 turn.rs) → **不达标，弃用**

### 2.4 sub-handler 物理位置

sub-handlers 都是 `impl ConversationCoordinator { async fn prepare_turn(...) }` 等。新 sibling file `turn_subhandlers.rs` 包含独立的 `impl ConversationCoordinator` block (Round 5/6 已验证 Rust 允许多 impl block)。

### 2.5 public API 保留

`start_dialog_turn_internal` 签名不变 (`pub(crate) async fn start_dialog_turn_internal(...)`)。它是 `start_dialog_turn` (facade public) 的内部实现, 不暴露给 crate 外。

---

## 3. 字段可见性

### 3.1 ConversationCoordinator 字段 (Round 6 D1 已确认全部 `pub`)

无需变更。Coordinater struct 字段 (coordinator.rs:541) 全是 `pub`。

### 3.2 TurnContext 字段

所有 `pub(crate)` (跨 sibling file 访问需要)。TurnContext 定义在 turn_subhandlers.rs，但 turn.rs main wrapper 也用，所以字段必须 visible to dialog_turn/。

### 3.3 sub-handler 方法

`async fn prepare_turn` 等都是 `pub(super)` (visible to mod.rs facade)。Round 6 已用 bulk promote script 验证 pattern。

---

## 4. Step-by-step implementation

```
Step 1: 读 start_dialog_turn_internal 709 行, 标识 4 个 sub-handler 边界
Step 2: 创建 dialog_turn/turn_subhandlers.rs skeleton (空文件 + impl ConversationCoordinator block)
Step 3: 提取 TurnContext struct + 字段 (从 start_dialog_turn_internal 的局部变量 group)
Step 4: 提取 prepare_turn (原 start_dialog_turn_internal 前 ~150 行)
Step 5: 提取 dispatch_turn (原 start_dialog_turn_internal 中 ~400 行)
Step 6: 提取 finalize_turn (原 start_dialog_turn_internal 后 ~120 行)
Step 7: 提取 cleanup_turn (原 start_dialog_turn_internal 末尾 ~50 行)
Step 8: 改写 turn.rs start_dialog_turn_internal 为 30 行 wrapper (call 4 sub-handlers)
Step 9: cargo check -p northhing-core --features product-full --lib (0 errors expected)
Step 10: cargo test -p northhing-core --features product-full --lib (899 pass expected)
Step 11: cargo fmt fixups
Step 12: split-analyzer after.json + custom verifier
Step 13: handoff doc
```

### 4.1 Worker preflight (NEW, per Round 6 lesson)

```bash
# 在 worktree 创建后, worker 开始前:
git checkout main
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cargo check -p northhing-core --features product-full --lib --message-format=short 2>&1 | Tee-Object baseline-main-cargo-check.log
cargo test -p northhing-core --features product-full --lib 2>&1 | Tee-Object baseline-main-cargo-test.log
git checkout -

# 记录 baseline: 0 errors, 899/0/1 passed
```

Worker 完成后必须证明:
- worktree cargo check / cargo test = baseline (0 NEW errors, 899 pass)
- worktree Cargo.lock rmcp 版本 = baseline main Cargo.lock rmcp 版本 (no drift)
- 0 cargo fmt diffs (除本 Round 7 引入)
- split-analyzer turn.rs 行数 = 期望值

### 4.2 强制 model

Plan YAML 必须 `model: minimax/MiniMax-M2.7-highspeed` (用户守则)。M3 模型不可接受。

---

## 5. Gate 标准

| Gate | 标准 | 验证 |
|---|---|---|
| Files ≤ 1000 行 | turn.rs ≤ 1000 | `[System.IO.File]::ReadAllLines().Count` |
| Files ≤ 800 行 | turn_subhandlers.rs ≤ 800 | 同上 |
| Method 不丢失 | start_dialog_turn_internal + 4 sub-handlers = 5 methods 替代 1 | split-analyzer after vs before |
| Public API 不变 | `start_dialog_turn` (facade pub) 签名不变 | git diff coordinator impl block |
| cargo check 0 errors | baseline 比较 | cargo check exit code 0 |
| cargo test 899/0/1 | baseline 比较 | cargo test 计数 |
| 0 cargo fmt diffs | turn.rs + turn_subhandlers.rs + 其他 touched files | cargo fmt --check |
| iron rules | 0 new unwrap/panic/let _ = | grep |

---

## 6. Errata (spec vs impl 偏差预登记)

| # | 预期偏差 | 备注 |
|---|---|---|
| E1 | TurnContext struct 字段数可能 ±5 个 | 看实际 start_dialog_turn_internal 局部变量决定 |
| E2 | sub-handler 行数估计 ±30 行 | depends on TurnContext field count |
| E3 | sub-handler 可能需要 4-5 个 (而非严格 4) | 如果 dispatch 拆 prepare + main loop 2 step |
| E4 | cargo test 数可能从 899 → 更高 | worker 可能补 unit tests for new sub-handlers (不要求) |

---

## 7. Spec Deviations 容忍范围 (前轮 pattern)

类似 Round 5 D3 + Round 6 D6:
- Spec 估算偏差 (实际 sub-handler 数 vs spec 4) → ACCEPT
- Atomic single commit for Steps 1-13 → ACCEPT (per Round 5/6 pattern, 35min timeout)
- Python script 提取方法 → ACCEPT (per Round 5/6 D7, git diff 可验证)
- bulk promote visibility script → ACCEPT (Round 6 D8.2 模式)

---

## 8. Plan YAML 草稿

```yaml
version: 1
plan:
  name: "round7-turn-internal-split-2026-06-28"
  max_concurrency: 1
  max_consecutive_failures: 2
  max_cycles: 1
  auto_accept: false
  auto_reject_retries: 1
  verifier_config:
    default_verifiers:
      - verifier
    audit_sample_rate: 0
    strict_mode: false
tasks:
  - id: impl-turn-internal-split
    title: "Round 7: start_dialog_turn_internal 709 → 4 sub-handlers (turn.rs ≤ 1000)"
    assigned_to: coder
    role: produce
    verified_by: verifier
    depends_on: []
    gates: []
    max_retries: 0
    timeout_ms: 14400000
    hang_alert_after_ms: 7200000
    prompt: |
      [见 Round 7 spec §4 + iron rules]

      ## Round 6 lessons (mandatory)
      1. Model: 必须 minimax/MiniMax-M2.7-highspeed (用户在守则, 不要 M3)
      2. Plan YAML preflight step: 在 main HEAD 跑 baseline cargo check + cargo test, 记录 pre-existing errors baseline
      3. Worker 报 "0 NEW errors" 必须 verify 在 baseline commit 上重现确认
      4. 不要相信 "pre-existing" 标注, 必须 source-of-truth 重现
      5. cargo check stop-at-first-error: 你的 cargo check PASS 必须确认所有 upstream crate 都先 compile
      6. Cargo.lock drift: 如果 baseline rmcp != worktree rmcp, 必须先 fix upstream drift 再继续

      ## Iron rules
      - 禁止 unwrap()/panic!/unreachable!/let _ = in production
      - 禁止 copy (sub-handler 必须 move not duplicate)
      - 文件 ≤ 1000 行 (turn.rs), ≤ 800 行 (turn_subhandlers.rs)
      - 字段可见性: TurnContext 字段 pub(crate), sub-handler 方法 pub(super)
      - public API 不变 (start_dialog_turn facade 签名)

      ## Implementation (13 steps per spec §4)
      [详细 step 内容]

      ## Critical: bulk promote script
      用 Round 6 的 promote-sibling-visibility.py 处理 sub-handler pub(super) 提升

      ## Verification at end
      cargo check -p northhing-core --features product-full --lib → 0 errors (与 baseline 对比)
      cargo test -p northhing-core --features product-full --lib → 899 pass (与 baseline 对比)
      cargo fmt --check → clean on turn.rs + turn_subhandlers.rs

      ## Handoff
      docs/handoffs/2026-06-28-round7-turn-internal-split-impl.md

    verify_prompt: |
      Verify Round 7 turn_internalsplit impl per spec §5.

      ## 4-axis review
      ### Axis 1: actual tests
      cargo check -p northhing-core --features product-full --lib → 0 errors (vs baseline 0)
      cargo test -p northhing-core --features product-full --lib → 899/0/1 (vs baseline 899/0/1)

      ### Axis 2: split-analyzer after.json
      turn.rs ≤ 1000 行
      turn_subhandlers.rs ≤ 800 行
      方法: start_dialog_turn_internal (turn.rs wrapper ~30 行) + 4 sub-handlers (turn_subhandlers.rs) = 5 总方法替代 1

      ### Axis 3: 0 fmt diffs + 0 iron rule violations
      cargo fmt --check → clean
      grep unwrap/panic/let _ = in production → 0

      ### Axis 4: public API + struct fields visibility
      ConversationCoordinator::start_dialog_turn facade 签名不变
      TurnContext 字段 pub(crate)
      sub-handler 方法 pub(super)

      ## PASS conditions
      - All 4 axes pass
      - TurnContext struct 字段完备 (≥ 10 个关键字段)
      - sub-handler 物理位置在 turn_subhandlers.rs (不在 turn.rs)

      Any fail → FAIL with specific detail.
```

---

## 9. 参考资料

- **Round 6 spec**: `docs/handoffs/2026-06-28-round6-dialog-turn-split-spec.md`
- **Round 6 handoff (Mavis take-over)**: `docs/handoffs/2026-06-28-round6-dialog-turn-split-impl.md`
- **QClaw review**: `docs/handoffs/2026-06-28-round6-dialog-turn-split-review-report.md`
- **Round 5 spec (类似 sub-domain split template)**: `docs/handoffs/2026-06-28-round5-chat-rs-split-spec.md`
- **Round 5 handoff (Mavis 4-axis review pattern)**: `docs/handoffs/2026-06-28-round5-chat-rs-split-impl.md`
- **QClaw code-rot-guard skill**: `C:\Users\UmR\.qclaw\skills\code-rot-guard\`
- **Round 6 Mavis take-over script**: `C:\Users\UmR\.qclaw\workspace\.rot\promote-sibling-visibility.py`

---

## 10. Round 7 Handoff 给新 session

新 session 启动时:

1. **读本 spec** (`docs/handoffs/2026-06-28-round7-turn-internal-split-spec.md`)
2. **读 Round 6 review** 验证 COND APPROVE 的 follow-up 必要 (`docs/handoffs/2026-06-28-round6-dialog-turn-split-review-report.md`)
3. **复用 Round 6 plan YAML template** (`C:\Users\UmR\.mavis\scratchpads\mvs_4cfd3e045ea44bf1942ff29fa9970579\round6-dialog-turn-plan.yaml`)
4. **新 worktree**: `git worktree add ../northing-impl-round7 -b impl/round7-turn-internal-split`
5. **Dispatch coder sub-agent** with mandatory preflight step + model M2.7-highspeed
6. **Mavis 4-axis review** after worker reports done
7. **Handoff → user** → user 转 QClaw/Kimi external review
8. **Merge to main** after external APPROVE

**关键: 重复 Round 6 take-over 5 lessons 不要相信 worker "0 NEW errors" 报**.