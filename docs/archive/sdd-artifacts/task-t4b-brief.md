# Task T4b Brief — 宿主蒸馏状态收敛（3 个裸键 → 成长状态，行为等价）

> 需求唯一来源。本文件之外的信息不得作为需求依据。
> 工作目录（唯一）：`E:\agent-project\northing\.worktrees\growth-core-0804`，分支 `feat/growth-core-0804`，当前 HEAD `1c986a4`
> 报告：`E:\agent-project\northing\.superpowers\sdd\task-t4b-report.md`（在 worktree 之外，不进 commit）

## 1. 这个任务在做什么

`turn_persist.rs` 里的蒸馏调度状态目前是 3 个**裸字符串 KV 键**（`distiller_paused` / `distill_turns` / `distill_hit_turns`），判定逻辑内联在 IO 代码中间。前两个任务已经准备好替代品：

- 纯判定函数：`northhing_agentic_growth::scheduler`（`should_distill` / `record_distill_outcome`）
- 宿主状态适配：`crate::agentic::growth_adapter`（`load_growth_state` / `save_growth_state`）

本任务把宿主接到它们上面，**行为等价**，并让蒸馏状态从此只有一个真相来源（成长状态 blob）。

### 1.1 明确不做（编排者已裁定，不要自行扩大）

- **不动园丁（dream）**：`dream.rs:47-62` 的 24h 间隔门与 `dream_last_sweep_at` 键**原样保留**。理由：`GrowthState.garden.last_sweep_at_ms` 若此刻也开始写，就会与 `dream.rs` 自己读写的键形成**两个真相来源**。园丁的迁移属于它自身改造那一步。本任务里 `garden` 字段只被迁移读入、不被写出。
- **不做四合一入口**：`on_turn_finalized` 单一门面留待后续任务；本任务只替换状态读写，hook 调用点结构不变。
- **不动 episode 日志、facts 写入、`decay_all_weights`、`append_facts_dedup`、`run_dream_sweep`** 的调用顺序与参数。

## 2. 现状基准（必须逐行核对后再改）

文件：`src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs`，函数 `append_facts_entry`（签名 `:425-433`）。

要替换的区间是 **`:458-514`**，现状语义（逐条，全部必须保留）：

1. `:455-456` 已经打开了 DB（`let db = MemoryDb::open(&db_path)`，是 `Result`，不 unwrap）。
2. `:459-463` 暂停门：读 `distiller_paused`，只有等于字符串 `"true"` 才算暂停；**DB 打开失败或读失败 → 视为未暂停**（照常蒸馏）。
3. `:466-476` 暂停时 `candidates = Vec::new()`（跳过 LLM 蒸馏调用），否则调 `distill_facts_with_llm(...)`。
4. `:479-482` `now_ms` 取值方式：`SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)`。
5. `:484-508` 计数（**仅当 DB 可用**）：`distill_turns` **无条件 +1**（含已暂停回合）；`distill_hit_turns` 仅当 `!candidates.is_empty()` 时 +1；读取失败一律当 0；两者立即写回。
6. `:510-513` 自暂停刹车：`distill_turns >= 20 && distill_hit_turns == 0` → 写 `distiller_paused="true"` + `warn!("Distiller auto-paused: 0 hits in {} turns", distill_turns)`。
7. **顺序关键**：计数与刹车发生在 `:516-518` 的 `if candidates.is_empty() { return; }` **之前**。也就是说没产出候选的回合，计数照样落库。

## 3. 交付物（只允许改这 2 个文件）

1. **改** `src/crates/assembly/core/src/agentic/growth_adapter.rs`（加两个函数 + 测试）
2. **改** `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs`（只替换 §2 那段，其余不动）

**任何第 3 个文件都不许动**。特别点名禁改：`dream.rs`、`distiller.rs`、`memory_db.rs`、`facts.rs`、`judge_memory.rs`、`src/agentic/**`（crate 侧一行不改）、`Cargo.toml`。

### 3.1 `growth_adapter.rs` 新增两个函数

```rust
/// Distillation gate for one dialog turn.
///
/// Returns `(run_distill, state)`. On any storage failure the caller must fall
/// back to running distillation with no counting, mirroring the pre-migration
/// behaviour where an unreadable pause flag meant "not paused".
pub(crate) fn begin_distill_turn(db: &MemoryDb) -> (bool, GrowthState);

/// Records the outcome of one distillation attempt and persists growth state.
///
/// Must be called on every finalized turn, including turns that produced no
/// candidates and turns skipped because distillation is paused. Logs the
/// auto-pause warning exactly once, on the transition into the paused state.
pub(crate) fn finish_distill_turn(
    db: &MemoryDb,
    state: &mut GrowthState,
    produced_facts: bool,
    now_ms: u64,
);
```

实现要求：
- `begin_distill_turn` = `load_growth_state(db)` + `scheduler::should_distill(&state)`，返回二元组。
- `finish_distill_turn` = `scheduler::record_distill_outcome(state, produced_facts)`；返回 `Some(ev)` 时 `warn!("Distiller auto-paused: 0 hits in {} turns", ev.turns)`（**日志文本必须与现状 `turn_persist.rs:512` 完全一致**）；随后 `save_growth_state(db, state, now_ms)`。
- 不得在这两个函数里做任何 LLM / 文件 IO / 其它表写入。

### 3.2 `turn_persist.rs` 的改法

把 §2 的 `:458-514` 替换为等价结构（示意，具体形态你自己收拾干净，但语义必须一致）：

```rust
// Growth state: single source of truth for distillation scheduling.
let (run_distill, mut growth_state) = match &db {
    Ok(db) => growth_adapter::begin_distill_turn(db),
    Err(_) => (true, GrowthState::default()),   // storage unavailable -> behave as "not paused"
};

let candidates = if run_distill {
    distill_facts_with_llm(user_input, last_assistant_text.as_deref(), session_id, turn_id).await
} else {
    Vec::new()
};

let now_ms = /* 与 :479-482 完全相同的取值方式 */;

if let Ok(db) = &db {
    growth_adapter::finish_distill_turn(db, &mut growth_state, !candidates.is_empty(), now_ms);
}
```

硬性要求：
- **`finish_distill_turn` 必须在 `if candidates.is_empty() { return; }` 之前调用**（保住 §2.7 的顺序语义）。
- DB 不可用时：不计数、不落库、照常蒸馏（与现状一致）。
- `distill_facts_with_llm` 的四个实参、`last_assistant_text` 的取得方式（`:443` 附近的 `load_last_assistant_text`）**不得改动**。
- 删掉本函数内因此不再需要的 `get_judge_state` / `set_judge_state` 导入项（`:437` 的 `use` 语句按需精简）；**但不要删这两个函数本身**（`dream.rs` 还在用）。
- 3 个裸键从此**不再被写入**，但**不得删除库里的旧值**（审计轨迹只增；迁移已由适配层负责一次性读入）。这一点要在报告里确认。

## 4. 测试

### 4.1 `growth_adapter.rs` 内新增（inline `#[cfg(test)] mod tests`，沿用现有测试建库方式）

1. 未暂停库 → `begin_distill_turn` 返回 `(true, state)`
2. 状态已 `paused=true` → 返回 `(false, _)`
3. `finish_distill_turn(produced_facts=false)` 从 `turns=19, hit_turns=0` 起 → 落库后重新 `load_growth_state`，断言 `turns=20 && paused==true`（**跨进程内一次真实读写往返**，不是只改内存）
4. 同上再调一次 → `turns=21`，仍 `paused==true`（证明暂停期间继续计数）
5. `produced_facts=true` → `hit_turns` +1 且不触发暂停
6. **旧键不被改写**：先用 `set_judge_state` 写 `distill_turns="7"`，跑一轮 `begin`+`finish`，断言 `get_judge_state(db, "distill_turns")` 仍是 `"7"`（新状态写 blob，不回写旧键）
7. 迁移衔接：只有旧键的库 → `begin_distill_turn` 读到迁移后的计数（例如旧键 `distill_turns="19"`, `distill_hit_turns="0"` → 一次 `finish` 后 `paused==true`）

### 4.2 回归（不新增，只跑）

现有 core 测试中与本改动相关的子集必须仍然通过（见 §5 命令）。

## 5. 验证（全部实际执行，把命令与**原始输出**贴进报告）

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo check -p northhing-core --features product-full
cargo test -p northhing-core --features product-full growth_adapter
cargo test -p northhing-core --features product-full auto_memory
cargo test -p northhing-core --features product-full memory_db
cargo test -p northhing-agentic-growth
node scripts/check-core-boundaries.mjs
```

要求：
- `cargo check` **不得新增 warning**（报告里给出 warning 计数与上一轮 19 的对比）
- 上面每条 `cargo test` 的通过/失败数都要贴原始输出；**任何新增失败都必须停下**并在报告标 `BLOCKED`，不要自行"修"测试来迁就实现
- 不要跑 core 全量测试（耗时），也不要跑 `cargo check --workspace`（被上游 embed-resource 阻断）

## 6. 硬约束

- 只改 §3 那 2 个文件。
- **行为等价**：除"自暂停 warn 从每轮一条变成只在跃迁时一条"这一条已授权的日志噪音差异外，不得有任何可观察行为变化（落库字段、调用顺序、早退时机、LLM 调用次数、日志文本）。
- 成长路径 warn-only：新代码任何失败只 warn，绝不向上传播、绝不 `?`。
- 非测试代码禁止 `unwrap()` / `expect()` / `panic!`。
- 注释与日志 **English-only、无 emoji**。
- **禁止运行 `cargo fmt`**；手工 4 空格对齐。
- 不得新增依赖、不得改数据库 schema、不得新增表或列。
- `growth_adapter.rs` 改动后仍需 < 800 行。

## 7. 交付

1. 在 `feat/growth-core-0804` 上提交一个 commit：`refactor(growth): route distillation scheduling through growth state`
2. 报告写到 `E:\agent-project\northing\.superpowers\sdd\task-t4b-report.md`，包含：
   - 状态：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
   - **等价性对照表**：§2 的 7 条现状语义 → 改动后由哪段代码承担（带新行号）→ 你如何确认等价
   - 被替换区间的**改动前 / 改动后完整代码**（便于审查者逐行比对）
   - §5 六条命令的原始输出
   - 确认 3 个旧键仍在库中未被删除（贴测试证据）
   - `git log --oneline -1`、`git status --short`
   - 与本 brief 的任何偏离及原因
