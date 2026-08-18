# Task A3 Brief — topics/score.rs（双层权重检索打分）

> 需求唯一来源。本文件之外的信息不得作为需求依据。
> 工作目录（唯一）：`E:\agent-project\northing\.worktrees\growth-a3`，分支 `feat/growth-a3`，基线 `7e96126`
> 报告：`E:\agent-project\northing\.superpowers\sdd\task-a3-report.md`（在 worktree 之外，不进 commit）

## 0. 你只能改一个文件

- `src/agentic/src/topics/score.rs`

**其它任何文件一行都不能改**（`topics/mod.rs` 已预声明该模块）。有 4 个并行任务在同一 crate 的其它文件上作业，越界就会撞车。

## 2. 背景与设计裁定（这是本任务的灵魂，必须严格实现）

记忆重要度是**双层**的：

- **话题权重（主导）**：这条记忆所属话题当前有多热。范围 0.0..=1.0。
- **条目分数（次要）**：这条记忆本身写得多值钱。范围 0.0..=1.0。

用户拍板的关系是：**话题权重 > 条目重要度**。落成公式：

```
score = topic_weight * (ENTRY_FLOOR + ENTRY_SPAN * entry_score)
其中 ENTRY_FLOOR = 0.6, ENTRY_SPAN = 0.4
```

这个形状的含义（**必须写进模块文档注释**）：

- 条目分数只是一个 0.6..=1.0 的乘数，最多造成 **1.667 倍**的排序摆动；
- 话题权重是主因子，比值可以任意大（0.9 : 0.1 = 9 倍）；
- 因此：当两条记忆的话题权重之比 ≥ 1.667 时，条目分数**无论如何都无法翻盘**——这就是"话题权重主导"的精确数学表述。

## 3. 规格

### 3.1 常量

```rust
pub const ENTRY_FLOOR: f64 = 0.6;
pub const ENTRY_SPAN: f64 = 0.4;
/// Ratio of topic weights beyond which entry score can never flip the order.
pub const TOPIC_DOMINANCE_RATIO: f64 = 1.0 / ENTRY_FLOOR; // 1.666...
/// Items scoring below this are dropped from retrieval results.
pub const RETRIEVAL_FLOOR: f64 = 0.02;
```

### 3.2 类型

```rust
#[derive(Debug, Clone)]
pub struct ScoredCandidate {
    pub id: String,
    pub topic_weight: f64,   // best topic weight among the item's topics
    pub entry_score: f64,
}
```

### 3.3 函数

```rust
/// Sanitizes a raw weight/score into 0.0..=1.0. NaN and negative -> 0.0, >1 -> 1.0.
pub fn sanitize_unit(value: f64) -> f64;

/// Best (max) topic weight among an item's topics. Empty -> 0.0.
/// NaN entries are sanitized, not propagated.
pub fn best_topic_weight(weights: &[f64]) -> f64;

/// score = tw * (ENTRY_FLOOR + ENTRY_SPAN * es), both inputs sanitized.
pub fn retrieval_score(topic_weight: f64, entry_score: f64) -> f64;

/// Ranks candidates by descending score.
/// Tie-break: higher topic_weight first, then lexicographically smaller id
/// (so the order is total and stable regardless of input order).
/// Candidates scoring below RETRIEVAL_FLOOR are dropped.
/// Returns (id, score) pairs.
pub fn rank_candidates(candidates: &[ScoredCandidate]) -> Vec<(String, f64)>;

/// True when a difference in topic weight is large enough that entry score
/// cannot change the relative order (ratio >= TOPIC_DOMINANCE_RATIO).
pub fn topic_weight_dominates(higher: f64, lower: f64) -> bool;
```

实现细节：

- 所有输入一律先 `sanitize_unit`，函数内部不得出现 NaN 传播。
- `rank_candidates` 排序必须**确定性**：不要用 `sort_by` + `partial_cmp().unwrap()`（NaN 会 panic）。先算分再排，`f64` 比较用 `total_cmp` 或先转成有序键。
- `topic_weight_dominates(higher, lower)`：`lower <= 0.0 && higher > 0.0` → true；`lower <= 0.0 && higher <= 0.0` → false；否则 `higher / lower >= TOPIC_DOMINANCE_RATIO`。

## 4. 测试（每条都要有）

**主导性（本任务最重要的断言，D5 的机器化表达）**：

1. `topic_weight=0.9, entry_score=0.0` 的条目 **必须**排在 `topic_weight=0.5, entry_score=1.0` 前面（0.54 > 0.5）
2. 反向证明摆动有上限：`tw=0.55, es=0.0`（0.33）**排在** `tw=0.5, es=1.0`（0.5）**后面** —— 说明话题权重差距不足 1.667 倍时条目分数可以翻盘，这是设计允许的
3. 属性测试（手写循环即可，不引依赖）：对 `tw_high` 从 0.1 到 1.0 步长 0.1、`tw_low = tw_high / TOPIC_DOMINANCE_RATIO - 0.001`，断言 `retrieval_score(tw_high, 0.0) > retrieval_score(tw_low, 1.0)`

**数值健壮性**：

4. `sanitize_unit`：`NaN` → 0.0、`-1.0` → 0.0、`2.0` → 1.0、`0.5` → 0.5、`f64::INFINITY` → 1.0、`f64::NEG_INFINITY` → 0.0
5. `best_topic_weight`：空 slice → 0.0；含 NaN → 忽略 NaN 取最大；全 NaN → 0.0
6. `retrieval_score(0.0, 1.0)` == 0.0（没有话题权重就没有分）
7. `retrieval_score(1.0, 1.0)` == 1.0（上界精确，允许 1e-12 误差）
8. `retrieval_score(1.0, 0.0)` == 0.6

**排序**：

9. 三个候选乱序输入 → 输出按分数降序
10. 同分（相同 tw、相同 es，不同 id）→ 按 id 字典序；**打乱输入顺序两次，输出一致**
11. 同分不同 tw（例如 tw=0.6/es=0.5 与 tw=0.5/es=... 构造成同分）→ tw 高者在前
12. 低于 `RETRIEVAL_FLOOR` 的候选被丢弃（构造 tw=0.01, es=0.0 → 0.006 < 0.02）
13. 空输入 → 空输出
14. 含 NaN 的候选**不 panic**，被当 0 处理并因低于 floor 被丢弃

## 5. 硬约束

- 只改第 0 节那一个文件。
- 零新依赖：只用标准库（`use std::cmp::Ordering` 之类 OK）。不得改 `Cargo.toml`。
- 无 IO、无时钟、无随机：纯函数模块。
- **不得 panic**：禁止 `unwrap()` / `expect()` / 索引越界写法出现在非测试代码里。
- 注释与文档 **English-only、无 emoji**。测试函数名英文。
- **禁止运行 `cargo fmt`**（本仓两次污染前科）。手工对齐：4 空格缩进。
- 文件 < 800 行（预计 200-350 行含测试）。
- 不要实现别的模块（extract / competition / ports / negation 归其它任务）。
- 不要引入"衰减"逻辑（decay 归后续任务），本文件只管**打分与排序**。

## 6. 验证（必须实际执行并把命令与原始输出贴进报告）

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-agentic-growth
cargo check -p northhing-agentic-growth
```

不要跑 `cargo check --workspace`（被上游 embed-resource 阻断，与本任务无关）。

## 7. 交付

1. 在本 worktree 内提交一个 commit：`feat(growth): add two-layer retrieval scoring with topic dominance`
   提交前 `git status --short` 确认只有那一个文件。
2. 报告写到 `E:\agent-project\northing\.superpowers\sdd\task-a3-report.md`，包含：
   - 状态：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
   - 文件行数
   - §6 命令原始输出（含测试名与通过数）
   - 对"话题权重主导"三条测试（§4.1-4.3）的具体数值证明
   - `git log --oneline -1`、`git status --short`
   - 与本 brief 的任何偏离及原因
