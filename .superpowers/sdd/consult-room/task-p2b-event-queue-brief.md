# Task P2b — B4 Event queue 满队静默丢失 → Result 化 + Critical 旁路

来源：`.superpowers/sdd/consult-room/prescription-v3-20260825.md` §B4。独立任务，无依赖。

## 现状 bug（已核实）

`src/crates/assembly/core/src/agentic/events/queue.rs:76` 的 inherent `EventQueue::enqueue`：
签名 `-> NortHingResult<String>`；**满队时 warn 一行后 `return Ok(event_id)`**（L85-88）——事件被丢弃但调用方以为成功，纯静默丢失。

## 改动

### ① inherent enqueue Result 化（queue.rs）

1. 新错误类型（就放在 queue.rs）：
   ```rust
   #[derive(Debug, Error)]
   #[error("event queue full: max_queue_size={max_queue_size}, dropped event_id={event_id}")]
   pub struct EventQueueFull { pub event_id: String, pub max_queue_size: usize }
   pub type EventId = String;
   ```
   （core 已依赖 thiserror；若 import 习惯不同按模块现状对齐。）
2. 签名改：`pub async fn enqueue(&self, event: AgenticEvent, priority: Option<EventPriority>) -> Result<EventId, EventQueueFull>`
   ——函数体无其它 Err 源（envelope 构造不可失败），丢掉 `NortHingResult` 外壳零损失。
3. 满队判定一行改动：`if queue.len() >= self.config.max_queue_size && priority != EventPriority::Critical` → 满**且非 Critical** 时 `return Err(EventQueueFull { .. })`（替换现在的 warn+Ok）。Critical 旁路时照常 push（队列可瞬时超上限——处方明文接受）。原 warn! 行删除（Err 本身即信号，trait impl 侧统一记日志）。

### ② StreamEventSink impl（queue.rs L226-228）

trait 签名**不动**（`agent-stream/src/types.rs:62-64` 返回 `()`，该 crate 刻意不依赖 core 错误类型）。impl 内改为：

```rust
if let Err(e) = EventQueue::enqueue(self, event, priority).await {
    tracing::error!("Agentic event dropped: {e}");
}
```

### ③ 优先级类型（无需改动，仅确认）

`contracts/events/agentic.rs:7` 的 `AgenticEventPriority`（含 `Critical = 0`）与 core `events/types.rs:12` 的 `use ... as EventPriority` 是**同一类型别名**，单文件同步，无双轨。你只需在 report 里引用这两行确认。

### ④ 调用点普查（只查不改控制流）

`rg -n "\.enqueue\(" src/crates/assembly/core/src/agentic/ src/crates/execution/`
已知生产点：`stream_processor.rs:192` / `turn_persist.rs:118,198` / `sub_handle_out.rs`（另有测试 harness `stream_test_harness.rs:22` 实现 trait，签名未变不受影响）。
要求：report 逐点列出每个调用点对新 `Err` 的实际行为（多数是 `let _` fire-and-forget——保持不动）；**本批不改任何调用点的控制流、不加日志**（范围外），但若发现某调用点会把 Err 当 panic/unwrap 路径，立即上报 BLOCKED 而不是顺手修。

### ⑤ 单测（queue.rs tests mod 或同文件既有测试区）

至少 1 个：`EventQueueConfig { max_queue_size: 2, batch_size: 10 }` 填满后
- Normal 优先级 enqueue → `Err(EventQueueFull)`；
- Critical 优先级 enqueue → `Ok(_)` 且 `len()` 变 3（超上限证明旁路生效）。
事件构造用最廉价的 `AgenticEvent` 变体即可（参考同文件/邻近既有测试怎么造事件；`default_priority()` 存在于事件上）。

## 禁区

- 不动 trait `StreamEventSink` 定义文件（types.rs）。
- 不动 dequeue_batch / clear_session / broadcast / stats 逻辑。
- 不改任何生产调用点（见④）。
- 不引入新的并发原语、不加 select!/取消路径（家规 4 不触发）。
- 广播语义不变：满队 Err 路径不进 broadcast_tx（与现状 drop 行为一致）。

## 复用侦察（必填进 report）

- 全仓是否已有名为 `EventQueueFull` / `EventId` 的类型（应无；有则报告并停）。
- core 内是否已有可复用的"容量满"错误变体（如 `NortHingError::Capacity` 之类；处方点名要独立类型以便调用方 match，若发现语义完全等价的既有类型，报告并等编排者裁定，不要自行二选一）。

## 验证（report 必贴命令+尾部输出）

```
cargo check --workspace          # 共享 core 触碰，家规最小集
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --lib events::queue
```

第二条是本机已打通的 MSVC 测试通道（PATH 上 GNU cargo 会 `-lshlwapi` 链接失败，勿用裸 `cargo test`）。若 feature gate 报错则加 `--features product-full` 重试；仍不通则贴输出并标明，由编排者取证。

## Report

写 `.superpowers/sdd/reports/task-p2b-event-queue-report.md`：改动清单（file:line）、调用点普查表、复用侦察结论、验证输出尾部、偏离及理由。
