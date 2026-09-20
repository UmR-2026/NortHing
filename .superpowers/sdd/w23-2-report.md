# W23-2 实施报告 — 事件契约诚实化

## 1. 改动摘要

本任务按照 D3-② + ZCode C1（2026-09-09 用户拍板）要求完成事件契约诚实化，零行为变化：
1. **许愿注释改 TODO**（`src/crates/contracts/events/src/agentic.rs`）：
   - 将 `UserSteeringInjected`（:293）与 `SessionModelAutoMigrated`（:307-308）中的前端虚构承诺转为 `// TODO(surface): ... (当前无任何 surface 消费（2026-09-09 ZCode 审查）)`。
2. **死变体标注**（`src/crates/contracts/events/src/agentic.rs`）：
   - 对零发射零消费的 `ImageAnalysisStarted` 与 `ImageAnalysisCompleted` 添加 `// not yet emitted (reserved)` 注释标注，保留计划面信息。
3. **双 catch-all 显式化与去 `_ =>`**（`src/crates/assembly/core/src/kernel_facade/events.rs`）：
   - 外层 `AgenticEvent` match 移除 `_ => vec![]`，显式罗列全部 14 个未翻译变体，每臂附带英文 `debug!` 日志并返回 `vec![]`；
   - 内层 `ToolEventData` match 移除 `_ => vec![]`，显式罗列全部 9 个未翻译变体，每臂附带英文 `debug!` 日志并返回 `vec![]`；
   - 将 `use tracing::warn;` 更新为 `use tracing::{debug, warn};`。
4. **意图丢弃防退化测试**（`src/crates/assembly/core/src/kernel_facade/events.rs`）：
   - 在内联 `#[cfg(test)] mod tests` 中添加 `test_agentic_event_to_dtos_intentional_drops`，断言外层 14 个与内层 9 个变体全部返回 `vec![]`。

### 显式丢弃清单与 debug! 文案

#### 外层 AgenticEvent（共 14 个，无 catch-all 兜底）：
1. `AgenticEvent::SessionCreated`
2. `AgenticEvent::SessionStateChanged`
3. `AgenticEvent::SessionDeleted`
4. `AgenticEvent::SessionTitleGenerated`
5. `AgenticEvent::ImageAnalysisStarted`
6. `AgenticEvent::ImageAnalysisCompleted`
7. `AgenticEvent::SubagentSessionLinked`
8. `AgenticEvent::TokenUsageUpdated`
9. `AgenticEvent::ThreadGoalUpdated`
10. `AgenticEvent::ModelRoundStarted`
11. `AgenticEvent::ModelRoundCompleted`
12. `AgenticEvent::DeepReviewQueueStateChanged`
13. `AgenticEvent::UserSteeringInjected`
14. `AgenticEvent::SessionModelAutoMigrated`

#### 内层 ToolEventData（共 9 个，无 catch-all 兜底）：
1. `ToolEventData::EarlyDetected`
2. `ToolEventData::ParamsPartial`
3. `ToolEventData::Queued`
4. `ToolEventData::Waiting`
5. `ToolEventData::Progress`
6. `ToolEventData::Streaming`
7. `ToolEventData::StreamChunk`
8. `ToolEventData::Confirmed`
9. `ToolEventData::Rejected`

#### 三类语义 debug! 样例原文：
- **(a) 零发射零消费（reserved）**：
  `debug!("ImageAnalysisStarted intentionally dropped at facade (reserved, never emitted)");`
- **(b) 外部订阅者消费（consumed by external subscriber）**：
  `debug!("TokenUsageUpdated intentionally dropped at facade (consumed by external subscriber)");`
- **(c) Facade 意图丢弃（standard facade drop）**：
  `debug!("SessionCreated intentionally dropped at facade");`

### 编译错误复盘与分层修复说明
- `E0559`（`SessionCreated` / `SessionStateChanged` / `TokenUsageUpdated` / `ModelRoundStarted` / `ModelRoundCompleted` 字段不匹配）：修在机制层与契约层，测试构造体直接依照 `northhing_events::AgenticEvent` 真实字段对齐。
- `E0277` / `E0308`（`ThreadGoalUpdated.goal` 与 `ModelRoundCompleted.duration_ms` 类型不匹配）：修在机制层（m04），分别对齐为 `None` 与 `Some(100)`。

---

## 2. 验证证据

### 验证 1: 工作区编译检查 (`cargo check --workspace`)
命令：`<RUSTUP> run stable-x86_64-pc-windows-msvc cargo check --workspace`
Exit Code: 0
输出：
```
    Checking northhing-events v0.2.10 (E:\agent-project\northing\src\crates\contracts\events)
    Checking northhing-agent-stream v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-stream)
    Checking northhing-kernel-api v0.1.0 (E:\agent-project\northing\src\crates\contracts\kernel-api)
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Checking northhing-ai-adapters v0.2.10 (E:\agent-project\northing\src\crates\adapters\ai-adapters)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.88s
```

### 验证 2: 新测试验证 (`test_agentic_event_to_dtos_intentional_drops`)
命令：`<RUSTUP> run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full --lib test_agentic_event_to_dtos_intentional_drops`
Exit Code: 0
输出：
```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.34s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)

running 1 test
test kernel_facade::events::tests::test_agentic_event_to_dtos_intentional_drops ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1073 filtered out; finished in 0.00s
```

### 验证 3: 既有映射测试无回归
命令：`<RUSTUP> run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full --lib agentic_event_to_dtos`
Exit Code: 0
输出：
```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.58s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)

running 14 tests
test kernel_facade::events::tests::test_agentic_event_to_dtos_context_compression_banners ... ok
test kernel_facade::events::tests::test_agentic_event_to_dtos_intentional_drops ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_cancelled_summary_with_prefix_truncated_to_120 ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_completed_summary_and_detail ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_completed_truncation_at_120 ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_completed_result_fallback ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_dialog_turn_started_produces_state_and_phase ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_confirmation_needed_maps_to_awaiting_confirmation ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_failed_maps_to_completed_phase ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_started_summary_fallback ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_started_summary_from_command ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_thinking_chunk_produces_phase_only ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_tool_started_carries_tool_name ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_text_chunk_produces_text_and_phase ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1060 filtered out; finished in 0.00s
```
（基线 13 passed 全部通过，加上新增的 `test_agentic_event_to_dtos_intentional_drops` 共 14 passed，0 failed）。

### 验证 4: 代码规范与边界检查
- `pnpm run check:repo-hygiene`: exit 0 (passed)
- `node scripts/check-core-boundaries.mjs`: exit 0 (passed)
- `git show --stat HEAD`: 恰好 2 个允许文件（`src/crates/assembly/core/src/kernel_facade/events.rs` 与 `src/crates/contracts/events/src/agentic.rs`）。

---

## 3. 状态

DONE
