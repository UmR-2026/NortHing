# Task P2b — B4 Event queue 满队静默丢失 → Result 化 + Critical 旁路 Report

## 改动清单

1. `src/crates/assembly/core/src/agentic/events/queue.rs`:
   - L14-22: 新增 `EventQueueFull` 错误类型（`#[derive(Debug, Error, PartialEq, Eq)]`）与 `EventId` 类型别名。
   - L83-97: `EventQueue::enqueue` 签名改为 `pub async fn enqueue(&self, event: AgenticEvent, priority: Option<EventPriority>) -> Result<EventId, EventQueueFull>`；
     满队判定逻辑改为 `if queue.len() >= self.config.max_queue_size && priority != EventPriority::Critical`，满足时返回 `Err(EventQueueFull { event_id, max_queue_size })`；
     删除了原满队时的 `warn!` 日志与 `Ok(event_id)` 返回，Critical 优先级旁路直接入队。
   - L231-236: `StreamEventSink for EventQueue` 实现修改，在 `EventQueue::enqueue` 返回 `Err` 时以 `tracing::error!("Agentic event dropped: {e}")` 记录日志。
   - L239-284: 新增 `mod tests` 包含 `test_enqueue_queue_full_and_critical_bypass` 单测。

## 调用点普查表

| 调用点位置 | 调用的 enqueue 形式 | 期望/实际行为说明 | 是否有 panic/unwrap 危险 |
|---|---|---|---|
| `stream_processor.rs:192` | `self.event_sink.enqueue(...)` (trait) | 调用 `StreamEventSink::enqueue` (返回 `()`)，在 `queue.rs` impl 内部捕获 `Err` 并以 `tracing::error!` 记录 | 否 |
| `turn_persist.rs:113` | `event_queue.enqueue(..., EventPriority::Critical)` (inherent) | 匹配 `if let Err(error)` 记录日志，Critical 优先级入队返回 `Ok` 不会触发 Err | 否 |
| `turn_persist.rs:190` | `event_queue.enqueue(..., EventPriority::Critical)` (inherent) | 匹配 `if let Err(error)` 记录日志，Critical 优先级入队返回 `Ok` 不会触发 Err | 否 |
| `turn_main_loop.rs:161` | `self.event_queue.enqueue(...)` (inherent) | `let _ = ...` 忽略 Result 返回值 | 否 |
| `rexec_state.rs:77` | `self.event_queue.enqueue(...)` (inherent) | `let _ = ...` 忽略 Result 返回值 | 否 |
| `so_handlers.rs:268,290,304` | `self.event_queue.enqueue(...)` (inherent) | `let _ = ...` 忽略 Result 返回值 | 否 |
| `state_manager.rs:106` | `self.event_queue.enqueue(...)` (inherent) | `let _ = ...` 忽略 Result 返回值 | 否 |
| `pipeline/state_manager.rs:266` | `self.event_queue.enqueue(...)` (inherent) | `let _ = ...` 忽略 Result 返回值 | 否 |
| `sub_handle_out.rs:189,366` | `event_queue.enqueue(...)` (inherent) | `let _ = ...` 忽略 Result 返回值 | 否 |
| `stream_test_harness.rs:21` | `RecordingEventSink::enqueue(...)` (trait) | 实现 `StreamEventSink` trait，签名未变不受影响 | 否 |

结论：生产调用点均未发现 panic/unwrap 风险点，保持控制流不动符合 brief 要求。

## 复用侦察结论

1. 全仓类型重名核实：已使用 grep 搜索全仓 `*.rs` 文件，此前全仓无 `EventQueueFull` 或 `EventId` 类型定义。
2. `core` 内部既有错误复用核实：`NortHingError` 中包含 `Service`, `Agent`, `Tool`, `AIClient` 等通用变体，无任何"队列满/容量超限"的专用变体。处方与 brief 要求的独立 `EventQueueFull` 错误类型设计无既有等价类型冲突。

## 优先级类型声明确认

- `src/crates/contracts/events/src/agentic.rs:7` 定义了 `pub enum AgenticEventPriority { Critical = 0, ... }`。
- `src/crates/assembly/core/src/agentic/events/types.rs:12` 导出 `pub use northhing_events::{ ... AgenticEventPriority as EventPriority, ... };`。
两处为单一源类型别名与导出，确认一致无双轨。

## 验证命令与输出尾部

### 1. `cargo check --workspace`

```text
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check --workspace
...
    Checking northhing-runtime-services v0.2.10 (E:\agent-project\northing\src\crates\execution\runtime-services)
    Checking northhing-kernel-api v0.1.0 (E:\agent-project\northing\src\crates\contracts\kernel-api)
    Checking northhing-agent-tools v0.2.10 (E:\agent-project\northing\src\crates\execution\tool-contracts)
    Checking northhing-agent-dispatch v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-dispatch)
    Checking northhing-product-capabilities v0.2.10 (E:\agent-project\northing\src\crates\assembly\product-capabilities)
    Checking rmcp v1.8.0
    Checking northhing-ai-adapters v0.2.10 (E:\agent-project\northing\src\crates\adapters\ai-adapters)
    Checking northhing-agent-runtime v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-runtime)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
```

### 2. 单元测试（MSVC 通道）

```text
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --features product-full --lib events::queue::tests::test_enqueue_queue_full_and_critical_bypass
...
   Compiling northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 52s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)

running 1 test
test agentic::events::queue::tests::test_enqueue_queue_full_and_critical_bypass ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1049 filtered out; finished in 0.00s
```

## 偏离及理由

无偏离。完全遵循 `task-p2b-event-queue-brief.md` 的规范实现。
