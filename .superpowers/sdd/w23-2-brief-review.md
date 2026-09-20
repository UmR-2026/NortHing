# W23-2 Brief Review（minimax-m3，2026-09-20）

## 判决: REVISE

## Findings

- [Critical] **外层 catch-all 变体清单缺漏（brief 列 9/14，实际 14/14）** — `src/crates/assembly/core/src/kernel_facade/events.rs:314` 外层 `_ => vec![]` 实际落入 14 个 AgenticEvent 变体；brief 背景段只列 9 个且测试要求只对 9 个逐一断言。漏算的 5 个：ImageAnalysisStarted、ImageAnalysisCompleted、SubagentSessionLinked、TokenUsageUpdated、ModelRoundStarted。核对方式：枚举 agentic.rs 全部 25 变体，扣减 events.rs 显式臂 = 14，不是 brief 的 9。
  - 修正处方：① 背景段 9 清单改完整 14 清单，并按语义分三类（reserved 零发射 / 外部 subscriber 消费 / 无 facade 翻译即静默丢失）；② 功能要求 3 改为"以编译错误引导收齐全部 14 个显式臂"，不能钉死为 9；③ 功能要求 4 测试断言清单从 9 改为 14。
- [Important] **内层 ToolEventData catch-all 变体清单缺漏** — events.rs:296 内层 `_ => vec![]` 实际落入 9 个 ToolEventData 子变体（EarlyDetected/ParamsPartial/Queued/Waiting/Progress/Streaming/StreamChunk/Confirmed/Rejected），brief 只提 Streaming。处方：背景段钉死 9 清单 + 测试覆盖 9 个。
- [Important] **允许文件集 vs 改动范围自洽性未声明** — 功能要求 3 要改 match 臂结构，与 Constraints"禁改消费逻辑"的边界未明文。处方：Constraints 补"臂结构显式化允许；臂内仍返回 vec![] 不算消费逻辑改动；不增 DTO、不改翻译结果"。
- [Important] **测试构造可行性 vs 测试名未钉死** — 内联 mod 可行（use super::* 覆盖），但测试名由 implementer 自定有误取宽名风险。处方：钉死推荐名 `test_agentic_event_to_dtos_intentional_drops`。
- [Minor] **tracing::debug! 与既有 `use tracing::warn` 风格混用** — 处方：:8 改为 `use tracing::{debug, warn};`，臂内写 `debug!(...)`。

## Cannot verify from diff

无（brief 复审阶段无 diff；全部基于 BASE 实际源码静态核对）。

## 范围变动

无越界：brief 限 2 文件（agentic.rs + events.rs）合理；但清单数字错误必须先修 brief 再派发。

## 增量复审（rev-1，447f538）
## 判决: PASS
- finding 1 (Critical, 外层清单 9→14): 已修复 — 背景段已改为 14 清单 + 三类语义；功能要求 3 改为"收齐全部 14、不留 `_ =>` 兜底"；功能要求 4 测试覆盖 14。
- finding 2 (Important, 内层 9 清单): 已修复 — 背景段钉死 9 个 ToolEventData 变体；功能要求 4 经 ToolEvent 包裹覆盖 9。
- finding 3 (Important, match 臂边界): 已修复 — Constraints 补边界明示（臂结构显式化允许；臂内 vec![] 不算消费逻辑改动；不增 DTO、不改翻译结果）。
- finding 4 (Important, 测试名钉死): 已修复 — 钉死 test_agentic_event_to_dtos_intentional_drops；验证段 2 完整命令。
- finding 5 (Minor, tracing 风格): 已修复 — :8 改 use tracing::{debug, warn};，臂内 debug!(...)。
## 新发现问题
- 无（diff 仅 brief 自身 9 增 9 删；外部代码未动；测试构造可行性经既有 use super::* + 既有测试确认）。
