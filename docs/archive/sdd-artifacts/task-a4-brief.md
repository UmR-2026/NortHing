# Task A4 Brief — topics/competition.rs（竞争组归一化与自然失效）

> 需求唯一来源。本文件之外的信息不得作为需求依据。
> 工作目录（唯一）：`E:\agent-project\northing\.worktrees\growth-a4`，分支 `feat/growth-a4`，基线 `7e96126`
> 报告：`E:\agent-project\northing\.superpowers\sdd\task-a4-report.md`（在 worktree 之外，不进 commit）

## 0. 你只能改一个文件

- `src/agentic/src/topics/competition.rs`

**其它任何文件一行都不能改**（`topics/mod.rs` 已预声明该模块）。有 4 个并行任务在同一 crate 的其它文件上作业，越界就会撞车。

## 1. 背景与设计裁定（本任务的核心机制）

用户拍板：**不做"硬作废"**（不许管家把旧记忆标记为失效）。取而代之的是**竞争组内的自然失效**：

> 同一个竞争组（例如"包管理器偏好"）内的话题共享 100% 的权重份额。当用户表达新偏好时，新话题的份额上升，**组内其它话题的份额被自动挤压下降**。份额低到一定程度，这条记忆就检索不到了——它"自然失效"了，但**数据仍在、可回滚、有审计**。

由此得出三条必须由测试证明的不变量：

1. **涨必有跌**：给组内某话题 boost 之后，组内份额和仍为 1.0，且至少一个其它成员的份额严格下降（组内成员数 ≥ 2 时）。
2. **可复活**：被挤压到"压制"状态的话题，再次被 boost 可以回到活跃态——不存在不可逆的死亡。
3. **无硬作废**：本文件**不得**提供任何标记 superseded / retired / deleted 的函数。管家（judge-mom）只能改权重。

## 2. 规格

### 2.1 常量

```rust
/// Normalized share below which a topic stops surfacing in retrieval.
pub const SUPPRESSION_SHARE_THRESHOLD: f64 = 0.15;
/// Raw (un-normalized) topic weight below which a topic stops surfacing.
pub const SUPPRESSION_RAW_THRESHOLD: f64 = 0.20;
/// Maximum share a single boost event may add before renormalization.
pub const MAX_BOOST_PER_EVENT: f64 = 0.15;
/// Tolerance for share-sum assertions.
pub const SHARE_SUM_EPSILON: f64 = 1e-9;
```

**注意**：压制判定是"归一化份额"与"原始权重"**两个条件同时成立**才算压制。理由：组内只有 2 个成员时份额天然偏高；而原始权重反映全局热度。二者分别来自不同数据源，所以判定函数不要试图自己从份额推原始权重。

### 2.2 类型

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct GroupMember {
    pub topic: String,
    pub share: f64,   // normalized 0.0..=1.0, group sums to 1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suppression { Active, Suppressed }

#[derive(Debug, Clone, PartialEq)]
pub enum HealthIssue {
    ShareSumDrift { sum: f64 },
    ShareOutOfRange { topic: String, share: f64 },
    DuplicateTopic { topic: String },
    EmptyGroup,
}
```

### 2.3 函数

```rust
/// Normalizes raw weights into shares summing to 1.0.
/// Rules: NaN/negative -> 0.0; if every weight is 0 -> equal split;
/// empty input -> empty output.
pub fn normalize(raw: &[(String, f64)]) -> Vec<GroupMember>;

/// Applies a boost to one member and renormalizes the group.
/// - delta is clamped to 0.0..=MAX_BOOST_PER_EVENT (negative -> 0.0, i.e. no-op)
/// - if the topic is not in the group it is inserted with share = delta before renormalizing
/// - the result always sums to 1.0 (within SHARE_SUM_EPSILON)
pub fn apply_boost(members: &[GroupMember], topic: &str, delta: f64) -> Vec<GroupMember>;

/// Suppression is share AND raw weight both below their thresholds.
pub fn suppression_state(share: f64, raw_weight: f64) -> Suppression;

/// Minimum number of MAX_BOOST_PER_EVENT boosts needed for `topic` to leave
/// the suppressed share band, or None if it is already above the band.
/// Pure arithmetic simulation using apply_boost; cap the loop at 100 iterations
/// and return None if it does not converge (must never loop forever).
pub fn boosts_to_revive(members: &[GroupMember], topic: &str) -> Option<u32>;

/// Structural validation of a group. Returns all issues found (empty = healthy).
pub fn health_check(members: &[GroupMember]) -> Vec<HealthIssue>;
```

实现细节：

- `normalize`：先 sanitize（NaN/负 → 0.0，上界不设限因为是原始权重），求和；和为 0 → 每个成员 `1.0 / n`；否则 `w / sum`。
- `apply_boost`：把目标成员的 share 加上 clamped delta（其它成员不动），然后整组重新归一化（除以新的和）。这个顺序天然产生"涨必有跌"。
- 输入含重复 topic：`apply_boost` 只 boost **第一个**匹配项；`health_check` 负责报 `DuplicateTopic`。不要在 `apply_boost` 里去重（保持函数单一职责）。
- **不得 panic**：非测试代码里禁止 `unwrap()` / `expect()`；除零必须显式判零。
- **不得提供** `retire` / `supersede` / `deactivate` 这类函数（见 §1 不变量 3）。

## 3. 测试（每条都要有）

**核心不变量**：

1. **涨必有跌**：3 成员均分（各 1/3），对成员 A boost 0.15 → 断言 A 份额上升、B 与 C 份额**都严格下降**、总和 == 1.0（`SHARE_SUM_EPSILON` 内）
2. **总和守恒**：连续 10 次随机化但确定性的 boost（用固定数列，不要引随机库）后总和始终 == 1.0
3. **boost 上限**：`apply_boost(..., 5.0)` 的效果与 `apply_boost(..., MAX_BOOST_PER_EVENT)` 完全一致；负 delta 是 no-op（组不变）
4. **可复活**：构造一个被挤压到 share < 0.15 的成员，断言 `boosts_to_revive` 返回 `Some(n)`，且实际连续 boost n 次后 `share >= SUPPRESSION_SHARE_THRESHOLD`
5. **不可逆性不存在**：断言不存在任何输入使某成员 share 变成 0 后再也无法上升（对 share=0 的成员 boost 一次，断言 share > 0）

**边界**：

6. 空组：`normalize(&[])` → 空；`apply_boost(&[], "x", 0.1)` → 单成员组，share == 1.0；`health_check(&[])` → `vec![EmptyGroup]`
7. 单成员组：share 恒为 1.0，永不因份额被压制（`suppression_state(1.0, 0.01)` 应为 `Active`，因为份额条件不成立）
8. 全零权重 → 均分（3 个成员各 1/3，误差 1e-9 内）
9. NaN / 负权重 → 当 0 处理，不 panic，不污染其它成员
10. boost 组内不存在的话题 → 该话题被插入，总和仍为 1.0
11. 重复 topic 输入：`apply_boost` 只改第一个；`health_check` 报 `DuplicateTopic`

**压制判定**：

12. `suppression_state(0.10, 0.10)` → `Suppressed`
13. `suppression_state(0.10, 0.50)` → `Active`（原始权重高，说明全局仍热）
14. `suppression_state(0.50, 0.10)` → `Active`（份额高）
15. 阈值边界：`share == 0.15` 与 `raw == 0.20` 恰好等于阈值时 → `Active`（阈值是"严格小于"才压制，必须在测试里钉死）

**health_check**：

16. 和漂移 0.9 → `ShareSumDrift`
17. share = 1.5 / -0.1 → `ShareOutOfRange`
18. 健康组 → 空 Vec

**收敛保护**：

19. `boosts_to_revive` 对已在阈值以上的成员 → `None`
20. 构造一个极端组（例如 200 个成员，目标 share 极小），断言函数**返回**（不死循环），无论是 `Some` 还是 `None`

## 4. 硬约束

- 只改第 0 节那一个文件。
- 零新依赖：只用标准库。不得改 `Cargo.toml`。禁止引入随机数库（测试用固定数列）。
- 无 IO、无时钟、无随机：纯函数模块。
- **不得 panic**：非测试代码禁止 `unwrap()` / `expect()`；`f64` 排序若需要用 `total_cmp`。
- 浮点断言一律用 `(a - b).abs() < eps` 形式，禁止 `assert_eq!` 直接比 `f64`（除 0.0 与 1.0 这类精确值可用 epsilon 断言表达）。
- 注释与文档 **English-only、无 emoji**。测试函数名英文。
- **禁止运行 `cargo fmt`**（本仓两次污染前科）。手工对齐：4 空格缩进。
- 文件 < 800 行（预计 300-450 行含测试）。
- 不要实现别的模块（extract / score / ports / negation 归其它任务）。
- 不要写"谁跟谁构成竞争组"的判定逻辑（那是 LLM 提议 + 三次一致证据的流程，归后续任务）。本文件只管**给定一个组之后的数学**。

## 5. 验证（必须实际执行并把命令与原始输出贴进报告）

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-agentic-growth
cargo check -p northhing-agentic-growth
```

不要跑 `cargo check --workspace`（被上游 embed-resource 阻断，与本任务无关）。

## 6. 交付

1. 在本 worktree 内提交一个 commit：`feat(growth): add competition group normalization and natural suppression`
   提交前 `git status --short` 确认只有那一个文件。
2. 报告写到 `E:\agent-project\northing\.superpowers\sdd\task-a4-report.md`，包含：
   - 状态：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
   - 文件行数
   - §5 命令原始输出（含测试名与通过数）
   - 三条核心不变量（涨必有跌 / 可复活 / 无硬作废函数）分别由哪个测试证明
   - `git log --oneline -1`、`git status --short`
   - 与本 brief 的任何偏离及原因
