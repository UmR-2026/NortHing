# W23-2 Brief — 事件契约诚实化（许愿注释 + 死变体标注 + catch-all 显式化）

## 任务标识

W23-2（防腐决策包 D3-② + ZCode C1 零成本部分；用户拍板 2026-09-09）。只动注释与事件映射臂的显式化，零行为变化（丢弃的仍丢弃，但从此显式留痕）。

## BASE

`c544749`（main HEAD，工作树干净；派发时 task-gate 起点 = 本 brief 提交后的 docs commit）。

## 背景与 BASE 证据（ZCode 实测 + 编排者抽查验真）

- **许愿注释**：`src/crates/contracts/events/src/agentic.rs:293`（UserSteeringInjected「The frontend renders this as a synthetic record」——前端不渲染）、`:307-308`（SessionModelAutoMigrated「The frontend should refresh its model selector…」——前端不做）。编排者已逐字验真。
- **死变体**：`ImageAnalysisStarted/Completed` 全仓仅契约定义与自家辅助函数（agentic.rs:100,108），零发射零消费。
- **隐形坟墓**：`src/crates/assembly/core/src/kernel_facade/events.rs:314` 尾部 `_ => vec![]` 静默丢弃 9 个事件（ZCode 清单：SessionCreated / SessionStateChanged / SessionDeleted / SessionTitleGenerated / ThreadGoalUpdated / UserSteeringInjected / DeepReviewQueueStateChanged / SessionModelAutoMigrated / ModelRoundCompleted）；内层 `ToolEventData` catch-all（events.rs:296 区域）丢弃 `Streaming` 子变体（state_manager.rs:186 有发射）。新增变体掉进 catch-all 无任何编译期信号。
- 用户拍板口径：只修 UserSteeringInjected 的 UX（那是 W26 的事）；**本单只做诚实化，不接任何事件到界面**。

## 允许文件集

- `src/crates/contracts/events/src/agentic.rs`（修改：仅注释）
- `src/crates/assembly/core/src/kernel_facade/events.rs`（修改：catch-all 显式化 + 测试）

report 写 `.superpowers/sdd/w23-2-report.md`，不进验收 diff。

## 功能要求

1. **许愿注释改 TODO**（agentic.rs）：:293 与 :307-308 两处的前端行为承诺改为 `// TODO(surface): …` 形态（保留原描述内容，前缀改为 TODO 并注明「当前无任何 surface 消费（2026-09-09 ZCode 审查）」）。
2. **死变体标注**（agentic.rs）：`ImageAnalysisStarted/Completed` 加 `// not yet emitted (reserved)` 标注（**标注不删除**——保留计划面信息）。
3. **catch-all 显式化**（kernel_facade/events.rs）：
   - 外层 `_ => vec![]`（:314）改为**显式罗列**当前落入的全部变体（以编译器 Exhaustiveness 为准收齐——ZCode 清单 9 个为起点，implementer 以编译错误为引导补齐，每个臂返回 `vec![]`），并在这些臂上加**一行** `tracing::debug!`（事件名 + "intentionally dropped"）。此后新增变体 = 编译错误强制有意识决定（本单核心目标）。
   - 内层 `ToolEventData` 的 catch-all（:296 区域）同法处理 `Streaming` 及其他落入变体。
   - 若显式罗列遇孤儿变体（契约有但从不构造，如 ImageAnalysis*）：同样显式列臂 + debug!，注释标 reserved。
4. **测试**：加一个单元测试钉住「显式丢弃清单」（对 9 个已知丢弃变体逐一断言映射结果为 `vec![]`，防未来有人把臂删回 catch-all）。测试落 events.rs 内联 `#[cfg(test)] mod tests`（W22-1 先例）。
5. 零行为变化：所有显式臂返回 `vec![]`，与现 catch-all 语义逐字等价；debug! 为日志不算行为变化（日志纪律：英文无 emoji）。

## Constraints

- commit 逐文件点名 `git add`（2 文件）；message 前缀 `refactor(events):` + `(W23-2)` 后缀，body 注明 D3-② + ZCode C1 + 用户拍板 2026-09-09。
- 禁改任何事件的发射/消费逻辑；禁动事件载荷字段；禁接 UX（W26）。
- report 贴验证输出 + exit code；结尾状态词。

## 验证（编排者已 BASE 预跑编译命令形态）

1. `cargo check --workspace` → 绿（Rust 树与 CI 34374829227 全绿时等价 + W22 两单均已验证）。
2. 新测试跑绿：`cargo test -p northhing-core --features product-full --lib <test_name>`（**必须带 feature**，feature 门后模块，裸跑假绿/编译死——W22 教训）。
3. 既有映射测试不回归：同命令跑 `agentic_event_to_dtos` 前缀过滤，全绿。
4. report 附：显式臂完整清单（哪些变体现为显式丢弃）+ debug! 文案原文。

## 禁区

- 禁动 contracts/events 的事件定义与载荷（只许动注释）。
- 禁动 compress_run.rs 等发射点。
- 禁动清单外文件。

## 报告

写 `.superpowers/sdd/w23-2-report.md`，三节：**改动摘要**（含显式丢弃清单）/ **验证**（4 项）/ **状态**。
