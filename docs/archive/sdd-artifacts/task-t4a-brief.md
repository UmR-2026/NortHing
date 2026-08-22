# Task T4a Brief — scheduler.rs（对话回合调度判定纯函数）

> 需求唯一来源。本文件之外的信息不得作为需求依据。
> 工作目录（唯一）：`E:\agent-project\northing\.worktrees\growth-core-0804`，分支 `feat/growth-core-0804`，当前 HEAD `a150339`
> 报告：`E:\agent-project\northing\.superpowers\sdd\task-t4a-report.md`（在 worktree 之外，不进 commit）

## 0. 你只能改一个文件

- `src/agentic/src/scheduler.rs`（现为 1 行空壳）

**其它任何文件一行都不能改**。特别点名禁改：`src/agentic/src/state.rs`、`ports.rs`、`lib.rs`（模块已声明）、以及 `src/crates/**` 下任何宿主代码。

## 1. 背景

宿主 `turn_persist.rs` 现在有 4 处散落的成长 hook，其中的**调度判定逻辑内联在 IO 代码里**，无法单测。本任务把这些判定**逐字搬成纯函数**，为后续宿主收敛（单一入口 `on_turn_finalized`）做准备。

本任务**只写纯逻辑**，不接线任何宿主代码。

## 2. 现状语义（必须逐字保留，这是"行为等价"的基准）

### 2.1 蒸馏暂停门（`turn_persist.rs:458-463`）

```rust
let distiller_paused = db.as_ref().ok()
    .and_then(|db| get_judge_state(db, "distiller_paused").ok().flatten())
    .as_deref() == Some("true");
```
暂停时 `candidates = Vec::new()`（跳过蒸馏，`:466-468`）。

### 2.2 计数与自学习刹车（`turn_persist.rs:484-514`）

- `distill_turns` **每轮都 +1**（**注意：即使处于暂停状态也照样 +1**）
- `distill_hit_turns`：本轮产出了候选事实才 +1，否则保持原值
- 自暂停：`if distill_turns >= 20 && distill_hit_turns == 0` → 写 `distiller_paused = "true"` + `warn!("Distiller auto-paused: 0 hits in {} turns", distill_turns)`
- 读取失败一律当 0（`.ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(0)`）

### 2.3 园丁（dream）间隔门（`dream.rs:47-62`）

```rust
let last_sweep = match get_judge_state(&db, "dream_last_sweep_at") {
    Ok(Some(v)) => v.parse::<u64>().unwrap_or(0),  // 解析失败 -> 0
    _ => 0,                                        // 缺失或读失败 -> 0
};
if now_ms.saturating_sub(last_sweep) < DREAM_SWEEP_INTERVAL_MS { return; }
```
其中 `DREAM_SWEEP_INTERVAL_MS = 24 * 60 * 60 * 1000`（`dream.rs:20`）。即：**`now - last >= 24h` 才跑**，用 `saturating_sub`（时钟回拨时不跑）。

## 3. 规格

### 3.1 常量

```rust
/// Turn count at which zero-hit distillation auto-pauses itself.
pub const DISTILL_AUTO_PAUSE_TURNS: u64 = 20;
/// Minimum interval between garden (stale-memory) sweeps.
pub const GARDEN_SWEEP_INTERVAL_MS: u64 = 24 * 60 * 60 * 1000;
```

### 3.2 类型

```rust
/// Emitted exactly once, on the transition into the auto-paused state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoPauseEvent { pub turns: u64 }

/// What the host should do when finalizing one dialog turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnDecision {
    pub run_distill: bool,
    pub run_garden_sweep: bool,
}
```

### 3.3 函数

```rust
/// Distillation runs unless the self-learning brake has paused it.
pub fn should_distill(state: &GrowthState) -> bool;

/// Garden sweep gate. Mirrors the host's arithmetic exactly: uses
/// saturating_sub so a backwards clock never triggers a sweep.
pub fn should_run_garden_sweep(last_sweep_at_ms: u64, now_ms: u64) -> bool;

/// Combined per-turn decision.
pub fn decide_turn(state: &GrowthState, now_ms: u64) -> TurnDecision;

/// Updates distillation counters after a turn and applies the self-learning
/// brake. Must be called on every finalized turn, including turns where
/// distillation was skipped because it is paused (this mirrors the current
/// host behaviour, which increments `distill_turns` unconditionally).
///
/// Returns Some(AutoPauseEvent) only on the transition into the paused state.
pub fn record_distill_outcome(state: &mut GrowthState, produced_facts: bool) -> Option<AutoPauseEvent>;

/// Records that a garden sweep ran at `now_ms`.
pub fn record_garden_sweep(state: &mut GrowthState, now_ms: u64);
```

实现要求：

- `should_distill` = `!state.distill.paused`
- `should_run_garden_sweep` = `now_ms.saturating_sub(last_sweep_at_ms) >= GARDEN_SWEEP_INTERVAL_MS`
- `decide_turn` 用上面两者，园丁那项读 `state.garden.last_sweep_at_ms`
- `record_distill_outcome`：
  1. `state.distill.turns = state.distill.turns.saturating_add(1)`
  2. `produced_facts` 为真 → `state.distill.hit_turns = state.distill.hit_turns.saturating_add(1)`
  3. 刹车：`turns >= DISTILL_AUTO_PAUSE_TURNS && hit_turns == 0` 时把 `paused` 置 true
  4. **只在 false → true 的那一次**返回 `Some(AutoPauseEvent { turns })`；已经是 paused 时返回 `None`
- `record_garden_sweep` = `state.garden.last_sweep_at_ms = now_ms`

### 3.4 唯一允许的行为偏离（必须写进模块文档注释）

现状宿主代码在已暂停后**每一轮都会重复**写 `distiller_paused="true"` 并重复打一条 `warn!`（因为 `turns` 持续增长而 `hit_turns` 恒为 0，条件一直成立）。本模块改为**只在状态跃迁时返回事件**，从而让宿主只打一次日志。

- 持久化结果等价：`paused` 最终值相同，成长状态每轮都会被整体写回。
- 差异仅在**日志噪音**：从"每轮一条 warn"变成"一条 warn"。
- 这条偏离必须在模块 `//!` 注释里显式记录，并由测试钉死（见 §4.7）。

除此之外**不许有任何其它语义变更**（不要"顺手"改阈值、不要加冷启动豁免、不要引入命中率百分比）。

## 4. 测试（每条都要有，表驱动优先）

1. `should_distill`：`paused=false` → true；`paused=true` → false
2. 未到阈值：`turns=18, hit_turns=0` 调一次 → `turns=19`、仍未暂停、返回 `None`
3. 恰好触发：`turns=19, hit_turns=0` 调一次（`produced_facts=false`）→ `turns=20`、`paused=true`、返回 `Some(AutoPauseEvent { turns: 20 })`
4. 有命中不暂停：`turns=19, hit_turns=1` 调一次 → `turns=20`、`paused=false`、`None`
5. `produced_facts=true` 时 `hit_turns` 才 +1；为 false 时保持不变
6. **暂停期间仍然计数**：`paused=true, turns=30, hit_turns=0` 调一次 → `turns=31`（证明 §2.2 那条"即使暂停也 +1"的语义被保留）
7. **事件只发一次**（§3.4 偏离的钉死测试）：从 `turns=19, hit_turns=0` 连调 3 次 → 第 1 次 `Some`，第 2、3 次 `None`，且 `turns` 变成 22
8. `saturating_add` 不 panic：`turns=u64::MAX` 调一次 → 仍为 `u64::MAX`
9. 园丁间隔：`now-last` 恰好 == 24h → true；== 24h-1ms → false；`last=0, now=0` → false；`last=0, now=GARDEN_SWEEP_INTERVAL_MS` → true
10. 时钟回拨：`last=now+10_000` → false（`saturating_sub` 归零，不跑）
11. `decide_turn` 组合矩阵：{paused, 未paused} × {到期, 未到期} 四种组合的 `TurnDecision` 逐字段断言
12. `record_garden_sweep` 后 `should_run_garden_sweep(state.garden.last_sweep_at_ms, same_now)` → false（刚跑完不会立刻再跑）

## 5. 硬约束

- 只改 `src/agentic/src/scheduler.rs`。
- 零新依赖：只用标准库 + `crate::state::GrowthState`（`use crate::state::GrowthState;`）。不得改 `Cargo.toml`。
- 纯函数：无 IO、**不得自己取当前时间**（`now_ms` 一律由参数传入）、无随机、无全局状态。
- 非测试代码不得 panic：无 `unwrap()` / `expect()`；算术一律 `saturating_*`。
- 注释与文档 **English-only、无 emoji**；测试函数名英文。
- **禁止运行 `cargo fmt`**（本仓两次污染前科）；手工 4 空格对齐。
- 文件 < 800 行（预计 200-350 行含测试）。
- 不要实现别的模块（distill / garden / review / route / executor 归后续任务）；不要在本文件里写任何宿主接线代码。

## 6. 验证（实际执行，把命令与原始输出贴进报告）

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-agentic-growth
cargo check -p northhing-agentic-growth
```

期望：全部通过；无 warning。不要跑 `cargo check --workspace`（被上游 embed-resource 阻断）。不要跑 core 的测试（本任务不碰宿主）。

## 7. 交付

1. 在 `feat/growth-core-0804` 上提交一个 commit：`feat(growth): add pure turn scheduling decisions`
   提交前 `git status --short` 确认只有那一个文件。
2. 报告写到 `E:\agent-project\northing\.superpowers\sdd\task-t4a-report.md`，包含：
   - 状态：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
   - 文件行数
   - §6 命令原始输出（含测试名与通过数）
   - §2 三条现状语义各自对应到你的哪个函数/哪行，以及 §3.4 那条偏离的测试证据
   - `git log --oneline -1`、`git status --short`
   - 与本 brief 的任何偏离及原因
