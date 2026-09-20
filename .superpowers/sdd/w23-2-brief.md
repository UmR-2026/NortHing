# W23-2 Brief — 事件契约诚实化（许愿注释 + 死变体标注 + catch-all 显式化）

## 任务标识

W23-2（防腐决策包 D3-② + ZCode C1 零成本部分；用户拍板 2026-09-09）。只动注释与事件映射臂的显式化，零行为变化（丢弃的仍丢弃，但从此显式留痕）。

## BASE

BASE = 编排者派发正文给出的 7 位 SHA（brief 定版时的 main HEAD 以派发正文为准；其后不得再有非本单允许文件集的 commit，否则 task-gate 起点顺延）。

## 背景与 BASE 证据（ZCode 实测 + 编排者抽查验真）

- **许愿注释**：`src/crates/contracts/events/src/agentic.rs:293`（UserSteeringInjected「The frontend renders this as a synthetic record」——前端不渲染）、`:307-308`（SessionModelAutoMigrated「The frontend should refresh its model selector…」——前端不做）。编排者已逐字验真。
- **死变体**：`ImageAnalysisStarted/Completed` 全仓仅契约定义与自家辅助函数（agentic.rs:100,108），零发射零消费。
- **隐形坟墓**：`src/crates/assembly/core/src/kernel_facade/events.rs` 外层 match（`agentic_event_to_dtos`，函数在 :80）尾部 `_ => vec![]`（:314）静默丢弃 **14 个** AgenticEvent 变体（编排者 2026-09-20 复算：enum 共 25 变体，显式臂 11 个 = TextChunk/ThinkingChunk/DialogTurnStarted/DialogTurnCompleted/DialogTurnCancelled/DialogTurnFailed/SystemError/ToolEvent/ContextCompressionStarted/ContextCompressionCompleted/ContextCompressionFailed；25−11=14）。落入的 14 个：SessionCreated / SessionStateChanged / SessionDeleted / SessionTitleGenerated / ImageAnalysisStarted / ImageAnalysisCompleted / SubagentSessionLinked / TokenUsageUpdated / ThreadGoalUpdated / ModelRoundStarted / ModelRoundCompleted / DeepReviewQueueStateChanged / UserSteeringInjected / SessionModelAutoMigrated。（ZCode 原清单 9 个漏了后 5 个中的 ImageAnalysis×2、SubagentSessionLinked、TokenUsageUpdated、ModelRoundStarted——**以本 brief 的 14 为准**。）丢弃语义分两类，debug! 文案须区分：**(a) 零发射零消费（reserved）**：ImageAnalysisStarted/ImageAnalysisCompleted；**(b) 有外部 subscriber 消费、只是不在 facade 翻译为 KernelEventDto**：SubagentSessionLinked / TokenUsageUpdated / ModelRoundStarted（外部订阅方如 token_usage subscriber、subagent_orchestrator、round_executor，实现时 grep 确认后照实写）；其余 9 个为 ZCode 原清单（无 facade 翻译即静默丢失）。内层 `ToolEventData` catch-all（:296）丢弃 **9 个**子变体：EarlyDetected / ParamsPartial / Queued / Waiting / Progress / Streaming / StreamChunk / Confirmed / Rejected（共 14 变体，显式臂 5 个 = Started/Completed/Failed/Cancelled/ConfirmationNeeded；Streaming 发射点 = `src/crates/assembly/core/src/agentic/tools/pipeline/state_manager.rs:185-186`，注意仓内有两个 state_manager.rs，是 tools/pipeline 下那个）。**两个 enum（AgenticEvent / ToolEventData）均无 `#[non_exhaustive]`**（编排者已核实），故下游显式罗列后新增变体必报编译错误——本单核心机制成立。events.rs 已有 `use tracing::warn;`（:8）。
- 用户拍板口径：只修 UserSteeringInjected 的 UX（那是 W26 的事）；**本单只做诚实化，不接任何事件到界面**。

## 允许文件集

- `src/crates/contracts/events/src/agentic.rs`（修改：仅注释）
- `src/crates/assembly/core/src/kernel_facade/events.rs`（修改：catch-all 显式化 + 测试）

report 写 `.superpowers/sdd/w23-2-report.md`，不进验收 diff。

## 功能要求

1. **许愿注释改 TODO**（agentic.rs）：:293 与 :307-308 两处的前端行为承诺改为 `// TODO(surface): …` 形态（保留原描述内容，前缀改为 TODO 并注明「当前无任何 surface 消费（2026-09-09 ZCode 审查）」）。
2. **死变体标注**（agentic.rs）：`ImageAnalysisStarted/Completed` 加 `// not yet emitted (reserved)` 标注（**标注不删除**——保留计划面信息）。
3. **catch-all 显式化**（kernel_facade/events.rs）：
   - 外层 `_ => vec![]`（:314）改为**显式罗列全部 14 个落入变体**（清单见背景段；以编译器 exhaustiveness 报错为引导收齐——每个具名臂返回 `vec![]`），每个臂上加**一行** `debug!`（事件名 + "intentionally dropped at facade"， reserved 类加 "(reserved, never emitted)"，外部订阅类加 "(consumed by external subscriber)"——实现时 grep 确认照实写）。**收齐后不得保留任何 `_ =>` 兜底臂**——本单核心目标：此后 AgenticEvent 新增变体 = 编译错误，强制有意识决定。
   - 内层 `ToolEventData` catch-all（:296）同法处理：显式罗列全部 9 个落入变体（EarlyDetected/ParamsPartial/Queued/Waiting/Progress/Streaming/StreamChunk/Confirmed/Rejected），每臂返回 `vec![]` + 一行 `debug!`；收齐后不留 `_ =>` 兜底。
   - tracing 风格：:8 的 `use tracing::warn;` 改为 `use tracing::{debug, warn};`，臂内写 `debug!(...)`（与既有 `warn!(...)` 风格统一，不写 `tracing::debug!` 全路径）。
4. **测试**：在 events.rs 内联 `#[cfg(test)] mod tests`（:358 已有）加一个测试，测试名钉死为 `test_agentic_event_to_dtos_intentional_drops`：对外层 14 个变体逐一构造最小实例断言 `agentic_event_to_dtos(&event)` 结果为 `vec![]`；对内层 9 个 ToolEventData 变体经 `AgenticEvent::ToolEvent { .. }` 包裹后同样断言 `vec![]`。防未来有人把具名臂删回 catch-all（W22-1 内联测试先例）。
5. 零行为变化：所有显式臂返回 `vec![]`，与现 catch-all 语义逐字等价；debug! 为日志不算行为变化（日志纪律：英文无 emoji）。

## Constraints

- commit 逐文件点名 `git add`（2 文件）；message 前缀 `refactor(events):` + `(W23-2)` 后缀，body 注明 D3-② + ZCode C1 + 用户拍板 2026-09-09。
- 禁改任何事件的发射/消费逻辑；禁动事件载荷字段；禁接 UX（W26）。**边界明示**：match 臂结构显式化（把 `_ =>` 改具名臂、加 debug!）允许，且臂内仍返回 `vec![]` 不算消费逻辑改动；不新增 DTO、不改任何翻译结果。
- report 贴验证输出 + exit code；结尾状态词。

## 验证（编排者已 BASE 预跑：命令形态全绿）

1. `cargo check --workspace`（`<RUSTUP> run stable-x86_64-pc-windows-msvc cargo check --workspace`）→ exit 0（BASE 实测绿；存量 warning 不影响，不要求清零）。
2. 新测试跑绿：`cargo test -p northhing-core --features product-full --lib test_agentic_event_to_dtos_intentional_drops`（**必须带 feature**，feature 门后模块，裸跑假绿/编译死——W22 教训；测试名已钉死）。
3. 既有映射测试不回归：`cargo test -p northhing-core --features product-full --lib agentic_event_to_dtos`（BASE 实测基线 = **13 passed; 0 failed; 1060 filtered out**，exit 0；实现后复跑须同绿或更绿，一个不许红）。
4. report 附：显式臂完整清单（外层 14 + 内层 9，逐一变体名）+ debug! 文案原文（三类语义各一条样例）。

## 禁区

- 禁动 contracts/events 的事件定义与载荷（只许动注释）。
- 禁动 compress_run.rs 等发射点。
- 禁动清单外文件。

## 报告

写 `.superpowers/sdd/w23-2-report.md`，三节：**改动摘要**（含显式丢弃清单：外层 14 + 内层 9）/ **验证**（4 项）/ **状态**。
