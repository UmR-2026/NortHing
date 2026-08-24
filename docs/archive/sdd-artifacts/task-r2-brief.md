# Task R-2：自暂停恢复路径（消除"记忆静默永久死亡"）

## 1. 背景：这是一个活 bug

`record_distill_outcome`（`src/agentic/src/scheduler.rs:88`）在「累计 20 轮且 0 命中」时把 `distill.paused` 置 `true`，作为自学习刹车。

**但全仓没有任何把它置回 `false` 的路径**（已实测）。后果：一旦某个用户的记忆蒸馏因连续 20 轮无可记内容而自暂停，**它永远不会再恢复**——记忆功能从此静默死亡，用户不会收到任何提示，也没有任何操作能救回来。

这与刚修完的 R-7 同一性质（静默、永久、不可恢复的功能损失），但影响面更大：R-7 只影响特定长回合，本 bug 影响该用户此后的**全部**回合。

用户已拍板：**两条恢复路径都要做**。

## 2. 范围

### 2.1 允许改动的文件

1. `src/agentic/src/scheduler.rs`（325 行，空间充足——**主要工作在这里**，纯逻辑零 IO）
2. `src/agentic/src/state.rs`（若需给 `DistillStats` 加字段）
3. `src/crates/assembly/core/src/agentic/growth_adapter.rs`（731 行，宿主接线）
4. `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs` —— ⚠️ **见 §2.2 硬约束**

### 2.2 🔴 硬约束：`turn_persist.rs` 当前 799 行，硬上限 800，**不得净增行数**

该文件只允许**修改现有调用的参数**，不允许新增语句/新增行。已确认可行路径：`growth_adapter::begin_distill_turn(db)`（调用点在 `turn_persist.rs:524`）目前只收 `db` 一个参数，而该处**已有 `user_input` 变量在作用域内**（同函数内 `:496` 处 `boost_turn_topics(db, user_input, now_ms)` 正在用它）。因此把 `user_input` 加进 `begin_distill_turn` 的签名即可，`turn_persist.rs` 净增 0 行。

如果你发现无法在不增行的前提下完成，**停下报 BLOCKED**，不要突破 800 行，也不要顺手拆分该文件（拆分是另一个已记账的任务）。

行数判定必须用 `(Get-Content -LiteralPath <file> -Encoding UTF8).Count`，**不要**用 `Measure-Object -Line`（上次因此误报 708 vs 实际 799）。

### 2.3 明确不在范围内（勿动）

- 园丁/dream 的触发时机改造（`dream.rs` 的 24h 门与 `dream_last_sweep_at` 键）—— 归 T12，本单**不要碰**。
- `facts.rs` 的关键词回落表（见 §4.2）—— 只读参考，**不要修改**。
- `search_facts` / `boost_keyword` / `decay_all_weights` / 任何 schema 或 SQL。
- 20 轮 0 命中这个**暂停阈值本身**不变。

## 3. 要实现的两条恢复路径

### 3.1 路径 A：探针节律（自动恢复，无需用户介入）

暂停后不能真的一次都不跑——否则永远拿不到"命中率回升"的证据，无法恢复。所以：

- 处于 `paused` 状态时，**每 20 轮放行一次蒸馏作为探针**（成本降到 1/20，而非归零）。
- 探针**命中**（产出了 facts）→ **完全解除暂停**。
- 探针未命中 → 保持暂停，等下一个探针窗口。

⚠️ **计数口径必须精确定义，避免 off-by-one 导致探针永不触发或每轮触发**。注意现状：`record_distill_outcome` 在暂停期间仍**无条件**给 `turns` +1（`scheduler.rs:89`，T4a 已钉死的语义，不要改）。请自行设计一个不会漂移的锚点（例如记录暂停发生时的 `turns` 值，用差值取模），并在文档注释里写清口径。

如需给 `DistillStats` 加字段：**必须 `#[serde(default)]`**，并加一条测试证明「旧的、不含该字段的持久化 blob 仍能反序列化成功」（成长状态是 serde blob，线上已有旧数据）。

### 3.2 路径 B：用户信号唤醒（立即恢复）

用户说出明确的记忆意图时，**立即解除暂停并让本轮蒸馏**——用户都开口要求记了，刹车没有任何理由继续踩着。

- 在 crate 里实现一个**纯函数**做检测（名字自拟，例如 `detect_memory_intent(user_input) -> bool`）。
- 短语表要求**高精度**：`记住` / `记得` / `以后` / `从现在起` / `别忘` / `总是` / `一直` / `优先` / `remember` / `always` / `never` / `from now on` / `prefer`。
- **不要**收入裸的 `别` 与 `不要`——它们过宽（`别的`、`不要紧`）且属于否定语义而非记忆意图。（教训来源：A5 否定检测的过宽短语问题。）
- 误报代价评估（写进注释）：误唤醒的后果仅是多跑一次蒸馏 + 解除暂停，代价低；**漏唤醒的后果是记忆继续死着**。故此处偏向宽松是正确取向——但仍不得收入上面点名排除的两个词。

### 3.3 解除暂停时必须重置窗口计数（**已授权的行为变更**）

无论经路径 A 还是 B 解除暂停，都要把命中率窗口重置为新窗口（`turns` 与 `hit_turns` 归零）。

理由：不重置会让刹车**再也无法重新挂上**——现有暂停条件是 `turns >= 20 && hit_turns == 0`，一旦 `hit_turns` 变成 ≥1 就永久不满足，记忆侧 LLM 成本失去上限保护。

📌 顺带记录（**本单不修**，只在报告里点明供编排者记账）：上述 `hit_turns == 0` 的写法意味着**该用户生命周期内只要命中过一次，刹车就永远不会挂上**。这是 T4a 从遗留代码逐字搬来的既有语义。请在报告里确认这一观察是否成立，作为后续 T13 的输入。

### 3.4 可观测性

`scheduler.rs:50` 已有 `AutoPauseEvent`（只在 false→true 转换时发一次）。请对称地为**解除暂停**加一个事件/返回信号，同样**只在 true→false 转换时发一次**，宿主侧记一条 `info!` 或 `warn!`（你判断哪个合适并说明理由）。原 `AutoPauseEvent` 的"只发一次"语义不得破坏。

## 4. 设计约束

### 4.1 crate 纯净性（硬规则）

`northhing-agentic-growth` 是第 6 层 Growth core：**纯逻辑、零 IO、禁依赖 rusqlite**。判定与状态转换逻辑全部放 crate；只有"读写 DB / 打日志"留在 `growth_adapter`。

### 4.2 短语表与 `facts.rs` 的关系（必须在报告里说明）

`facts.rs:245-248` 已有一张相似的双语短语表（`以后/记住/记得/不要/别/总是/一直/优先/别再` + `prefer/always/never/remember/from now on`），用途是**LLM 蒸馏失败时的关键词回落**（`distill_facts_from_user_message`）。

- crate 不能依赖 core，所以**不要**试图复用它，在 crate 内另立表。
- 但**必须**在你新表的文档注释里点明这层刻意重复，并写清两者用途差异（回落抽取 vs 唤醒判定）与故意的取舍差异（你排除了 `别`/`不要`）。
- **绝不修改** `facts.rs`——它在生产回落路径上，改动有行为风险。

### 4.3 通用纪律

- 非测试代码禁止 `unwrap`/`expect`/`panic!`；warn-only，失败不传播。
- 日志与注释 **English-only、无 emoji**（测试中文字面量允许）。
- 生产 `.rs` < 800 行。
- **禁止 `cargo fmt`**。

## 5. 测试要求

crate 侧（`scheduler.rs` 内，纯函数好写，请写足）：
1. 未暂停时行为不变（回归保护）。
2. 暂停后第 1..19 轮不放行；第 20 轮放行探针。
3. 探针命中 → 解除暂停 + 窗口归零。
4. 探针未命中 → 保持暂停，且下一个探针窗口仍会到来（连续两个窗口都跑到）。
5. 唤醒短语命中 → 立即解除暂停 + 窗口归零 + 本轮放行。
6. `别的` / `不要紧` **不触发**唤醒（负向，防过宽）。
7. 解除暂停事件只在 true→false 转换时发一次；已解除状态重复调用不再发。
8. 原 `AutoPauseEvent` 只发一次的语义未被破坏。
9. 若加了新字段：旧 blob（无该字段）反序列化成功。
10. 暂停期间 `turns` 仍无条件 +1（T4a 语义回归保护）。

宿主侧（`growth_adapter.rs`）：至少 2 条真实读写往返测试，证明"暂停的库经唤醒后确实恢复并持久化"。

## 6. 验证（全部执行，**完整原始 stdout+stderr** 贴进报告，不要摘录）

前置：`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

1. `cargo test -p northhing-agentic-growth`（当前 121 tests，报新总数）
2. `cargo check -p northhing-core --features product-full` —— warning **基线 19，不得新增**
3. `cargo test -p northhing-core --features product-full growth_adapter`（当前 25）
4. `cargo test -p northhing-core --features product-full turn_persist`（当前 12）
5. `cargo test -p northhing-core --features product-full memory_db`（当前 21）
6. `node scripts/check-core-boundaries.mjs` —— exit 0
7. 三个文件行数实测值（`turn_persist.rs` 必须 ≤ 799 且**未净增**、`growth_adapter.rs`、`scheduler.rs`，均 < 800）

## 7. 报告

写到 `E:\agent-project\northing\.superpowers\sdd\task-r2-report.md`：
- 状态：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
- 探针计数口径的精确定义 + 为什么不会漂移
- 是否给 `DistillStats` 加了字段；若加了，向后兼容如何保证
- §3.3 窗口重置的实现位置；以及对 §3.3 📌 那条"命中一次刹车永久失效"观察的确认或反驳（带 file:line）
- 短语表与 `facts.rs:245-248` 的关系说明
- `turn_persist.rs` 的净行数变化（必须 0 或负）
- §6 七条的完整原始输出
- 改动文件清单
- 疑虑

## 8. 工作目录与提交

- `E:\agent-project\northing\.worktrees\growth-core-0804`（分支 `feat/growth-core-0804`，当前 HEAD `6365cf5`）
- 一个 commit，`fix(growth): ` 前缀，正文说明"两条恢复路径 + 窗口重置为已授权行为变更"。
- 提交前 `git status --short`；**不要**提交 `.superpowers/` 下任何文件。

## 9. 纪律

- brief 是需求唯一来源。发现 brief 与代码矛盾、或验收标准自相矛盾 → **停下报 BLOCKED**（上一轮 T6a 的实现者正是这样抓出我的算术错误，做得对）。
- 不要自派子代理。
- 不要预判审查者。
