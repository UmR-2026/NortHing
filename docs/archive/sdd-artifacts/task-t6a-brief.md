# Task T6a Brief — 话题权重上升通道接线（修 🔴 权重只降不升）

> 需求唯一来源。本文件之外的信息不得作为需求依据。
> 工作目录（唯一）：`E:\agent-project\northing\.worktrees\growth-core-0804`，分支 `feat/growth-core-0804`，当前 HEAD `985bbb9`
> 报告：`E:\agent-project\northing\.superpowers\sdd\task-t6a-report.md`（在 worktree 之外，不进 commit）

## 1. 要修的缺陷（现状事实，已核实）

`keyword_weights` 表（`memory_db.rs:90-96`）本该给检索排序提供"这个话题当前有多热"的信号，但：

- `boost_keyword`（`memory_db.rs:584`）**生产零调用方**（只有测试引用）→ 权重永远不会上升
- `decay_all_weights(0.99, 0.1)`（唯一生产调用点 `turn_persist.rs:590`）每回合把全表乘 0.99
- 结果：表在生产里**始终是空的**，`search_facts` 里 `keyword_weight` 恒等于 fold 初值 → 排序信号完全惰性（`memory_db.rs:531-545`）

本任务把上升通道接上，让这个信号真正工作。

## 2. 编排者已裁定的设计（不要自行改动）

### 2.1 权重语义：1.0 = 从未提及的基线

关键事实：`get_keyword_weight`（`memory_db.rs:639-653`）对**不存在**的关键词返回 `1.0`；`search_facts` 的权重 fold 初值也是 `1.0`（`memory_db.rs:531-540`）。

于是现状的衰减底线 `0.1` 有个语义陷阱：一个被提过一次、随后长期不再提及的话题，权重会衰减到 1.0 以下，**排得比从来没被提过的话题还低**。这是错的——提过总比没提过更相关。

**裁定**：把每回合衰减的底线从 `0.1` 抬到 `1.0`。权重区间变成 `[1.0, 5.0]`：

- `1.0` = 从未提及 / 已完全冷却（与"不存在"等价）
- 每次提及 `+1.0`，上限 `5.0`（沿用 `boost_keyword` 现有实现，**不改该函数**）
- 每回合 `×0.99`，缓慢回落到 1.0 但**永不低于**基线

即：`turn_persist.rs:590` 的 `db.decay_all_weights(0.99, 0.1)` 改成 `db.decay_all_weights(0.99, 1.0)`。因为生产里该表一直是空的，这个参数改动在现网**不可观测**。

### 2.2 话题来源：用户输入抽取的话题（复用已审过的纯函数）

用 `northhing_agentic_growth::topics::extract::extract_topics(user_input)`（已通过审查，最多 3 个话题，含 CJK 处理）。

每个话题调一次 `boost_keyword(topic, &related, now_ms)`，其中 `related` = **同一回合抽到的其它话题**（写入现有 `related_keywords` 字段，形成共现关系图，供后续竞争组认定使用）。

**不要**用 LLM、不要引新的抽词逻辑、不要改 `boost_keyword` 与 `search_facts`。

### 2.3 已授权的行为变更（两条，必须写进报告）

1. **衰减底线 `0.1` → `1.0`**（理由见 §2.1）。
2. **boost 与 decay 的执行时机统一**：现状 `decay_all_weights` 位于 `:590`，在 `if candidates.is_empty() { return; }`（`:516-518`）**之后**，也就是只在"产出了候选事实"的回合才衰减。boost 必须每回合都发生（用户每句话都在表达话题热度），因此**把 boost 与 decay 成对放在同一处、每个完成的回合都执行一次**（即移到那个早退之前）。
   - 理由：若 boost 每回合发生而 decay 只在部分回合发生，权重会单调膨胀到上限、信号再次失效。二者必须同频。
   - 现网不可观测：该表当前为空，且 `decay_all_weights` 对空表是 no-op。
   - 这一条必须在报告里显式列为"已授权的行为变更"，并说明搬动后的确切位置。

除这两条外，**不得有任何其它可观察行为变化**。特别不许改：`search_facts` 的 SQL 与打分公式、`boost_keyword` / `decay_all_weights` / `get_keyword_weight` 的实现、facts 写入与去重、episode 日志、`run_dream_sweep`、蒸馏调用与计数（上一个任务刚做完，不要碰）。

## 3. 交付物（只允许改这 2 个文件）

1. **改** `src/crates/assembly/core/src/agentic/growth_adapter.rs`（加一个函数 + 测试）
2. **改** `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs`（接线 + 搬动 decay + 改一个参数）

**任何第 3 个文件都不许动**。特别点名禁改：`memory_db.rs`、`facts.rs`、`dream.rs`、`distiller.rs`、`judge_memory.rs`、`src/agentic/**`（crate 侧一行不改）、`Cargo.toml`、任何 SQL/schema。

### 3.1 `growth_adapter.rs` 新增

```rust
/// Boosts the topics mentioned in this turn and applies the paired per-turn
/// decay, keeping topic weights inside the [DECAY_FLOOR, 5.0] band.
///
/// Warn-only: every failure is logged and swallowed.
pub(crate) fn boost_turn_topics(db: &MemoryDb, user_input: &str, now_ms: u64);
```

以及两个常量（放在文件顶部，带文档注释说明含义）：

```rust
/// Per-turn multiplicative decay applied to all topic weights.
pub(crate) const TOPIC_DECAY_FACTOR: f64 = 0.99;
/// Decay floor: equals the implicit weight of a never-mentioned topic, so a
/// cooled-down topic never ranks below one that was never mentioned at all.
pub(crate) const TOPIC_DECAY_FLOOR: f64 = 1.0;
```

实现顺序（**先 boost 再 decay**，并在注释里说明为什么：先记本回合热度、再统一冷却，避免本回合刚提到的话题被同一回合的衰减抹掉一部分）：

1. `let topics = extract_topics(user_input);`
2. 对每个 topic：`related` = 同回合其它话题（`Vec<String>`）；调 `db.boost_keyword(topic, &related, now_ms)`，`Err` → `warn!`（English，含 topic 与错误）
3. `db.decay_all_weights(TOPIC_DECAY_FACTOR, TOPIC_DECAY_FLOOR)`，`Err` → `warn!`；返回值可丢弃
4. `topics` 为空时：**仍然执行 decay**（回合照常冷却），不要早退

### 3.2 `turn_persist.rs` 改动

- 在 `append_facts_entry` 里，把现有 `:590` 的 `let _ = db.decay_all_weights(0.99, 0.1);` **删掉**（它被 §3.1 的函数接管）。
- 在 `if candidates.is_empty() { return; }`（`:516-518`）**之前**，插入一次 `growth_adapter::boost_turn_topics(db, user_input, now_ms)` 调用（DB 可用时才调，沿用现有 `if let Ok(db) = &db` 形态；可与上一个任务的 `finish_distill_turn` 调用相邻，但**不要**把两者合并成一个函数）。
- `now_ms` 复用现有变量，不要再取一次时间。
- 不要动 `:551-592` 那个块里的迁移守卫、`insert_fact` 循环、`append_facts_dedup`、`run_dream_sweep`。

## 4. 测试（`growth_adapter.rs` 内 inline，沿用现有建库方式）

1. 单话题首次提及：`boost_turn_topics(db, "以后依赖安装都用 pnpm", now)` → `get_keyword_weight(db, "pnpm")` 落在 `(1.0, 2.0]`（因为 boost 后紧跟一次 ×0.99）；断言 > 1.0
2. 多次提及递增：连调 3 次 → 权重严格大于只调 1 次的结果
3. 上限：连调 10 次 → 权重 ≤ 5.0（不得越界）
4. **底线**：先 boost 一次，然后连续 500 次"空输入"调用（只触发 decay）→ 权重 **≥ 1.0**（钉死 §2.1 的裁定；不得跌破基线）
5. **从未提及的关键词**：`get_keyword_weight(db, "never-mentioned")` == 1.0（证明基线一致、排序单调）
6. 共现关系：一句话含 2 个以上话题 → 每行的 `related_keywords` 含同回合的其它话题（可用 `boost_keyword` 之外的既有读取路径断言；若无现成读取 API，可断言权重行数与话题数一致并在报告说明该限制）
7. 空输入 / 纯停用词输入 → 不新增任何权重行，但 decay 仍被执行（不 panic）
8. CJK 输入 → 至少产生一行（证明中文话题不会被漏掉）
9. `boost_turn_topics` 全程 warn-only：DB 正常时不 panic；不要求构造 DB 故障

## 5. 验证（全部实际执行，把命令与**原始输出**贴进报告）

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo check -p northhing-core --features product-full
cargo test -p northhing-core --features product-full growth_adapter
cargo test -p northhing-core --features product-full memory_db
cargo test -p northhing-core --features product-full auto_memory
cargo test -p northhing-agentic-growth
node scripts/check-core-boundaries.mjs
```

要求：
- `cargo check` warning 数与基线 **19** 一致（无新增）
- **`memory_db` 测试全过**（其中已有针对 `boost_keyword` / `decay_all_weights` 的测试，若因底线改动而失败：**停下标 BLOCKED**，贴原始输出——那些测试直接调 `decay_all_weights` 传自己的参数，理论上不受影响，若确实失败说明我的裁定有误，需要我重新决策，你不要自行改测试)
- 任何新增失败都必须停下标 `BLOCKED`，不许改测试迁就实现
- 不要跑 core 全量测试，不要跑 `cargo check --workspace`（被上游 embed-resource 阻断）

## 6. 硬约束

- 只改 §3 那 2 个文件；未改任何 SQL / schema / 表 / 列 / 依赖。
- warn-only：新代码任何失败只 warn，无 `?` 传播；非测试代码禁 `unwrap()` / `expect()` / `panic!`。
- 注释与日志 **English-only、无 emoji**。
- **禁止运行 `cargo fmt`**；手工 4 空格对齐。
- `growth_adapter.rs` 仍需 < 800 行（当前已用掉一部分预算，若逼近上限在报告里说明，不要擅自拆文件）。
- 不许"顺手"改检索打分、不许引入双层打分（`topics::score` 的接线是后续任务，本任务**只管权重升降**）。

## 7. 交付

1. 在 `feat/growth-core-0804` 上提交一个 commit：`fix(growth): wire topic weight boosting so retrieval weights can rise`
2. 报告写到 `E:\agent-project\northing\.superpowers\sdd\task-t6a-report.md`，包含：
   - 状态：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
   - 两条已授权行为变更的落地说明（底线 0.1→1.0 的位置；boost/decay 成对搬到哪一行）
   - 改动前后的关键代码片段（`:516-518` 附近与原 `:590`）
   - §5 六条命令的原始输出（含 warning 计数对比）
   - 测试 4（底线不跌破）与测试 5（未提及基线 1.0）的具体数值证据
   - 若测试 6 因缺少读取 API 而降级，说明降级方式
   - `git log --oneline -1`、`git status --short`
   - 与本 brief 的任何偏离及原因
