# Review — Task P2b: Event queue 满队静默丢失 → Result 化 + Critical 旁路

**Base:** `2c54f33` · **Head:** `df47924` · **Scope:** 1 file, 74 insertions / 5 deletions (`queue.rs`)

## Spec Compliance

✅ **Spec compliant**

逐条对照 brief §①–⑤ + 禁区：

| § | Brief 要求 | 证据 (file:line) | 判决 |
| --- | --- | --- | --- |
| ① | `EventQueueFull { event_id, max_queue_size }` + `Debug`/`Error` 派生 + `EventId` alias | `queue.rs:14-21`（`#[derive(Debug, Error, PartialEq, Eq)]`、字段 `event_id: String` + `max_queue_size: usize`、`pub type EventId = String;`） | ✅ |
| ① | 签名改 `Result<EventId, EventQueueFull>`，去掉 `NortHingResult` 外壳 | `queue.rs:86-90` | ✅ |
| ① | 满队判定含 `priority != EventPriority::Critical` 且返回 `Err`；原 `warn!` 行删除 | `queue.rs:99-104`（`if queue.len() >= self.config.max_queue_size && priority != EventPriority::Critical { return Err(EventQueueFull { event_id, max_queue_size: self.config.max_queue_size }); }`）；enqueue 函数体内已无 `warn!` 调用 | ✅ |
| ① | Critical 旁路时照常 push + broadcast + stats + notify | `queue.rs:105` push（与判 size 在同一锁内）→ `:109` `broadcast_tx.send` → `:113-116` stats 更新 → `:119` `notify.notify_one()` —— Critical 走的是 `&&` 短路让 `if` 不进 `return`，后续逻辑无差异 | ✅ |
| ② | trait impl 改为 `if let Err(e) = … { tracing::error!(…); }` | `queue.rs:241-247` | ✅ |
| ③ | 优先级类型别名一致性（仅确认、不改） | report 已引 `src/crates/contracts/events/src/agentic.rs:7` 与 `src/crates/assembly/core/src/agentic/events/types.rs:12`；本任务未触碰这两处 | ✅ |
| ④ | 调用点普查表存在于 report，覆盖已知生产点 | report §调用点普查表 11 行；实测当前代码路径全部命中（详见下文"行为变化面"） | ✅ |
| ⑤ | 单测：Normal→Err、Critical→Ok 且 len 超上限 | `queue.rs:254-297` `test_enqueue_queue_full_and_critical_bypass`：填满后 Normal 返 `Err(EventQueueFull { max_queue_size: 2, .. })` + `len()==2`；Critical 返 `Ok(_)` + `len()==3`（超上限） | ✅ |
| 禁-1 | 不动 trait `StreamEventSink` 定义文件 | diff 仅 `queue.rs`；未触碰 `src/crates/execution/agent-stream/src/types.rs:62-64` | ✅ |
| 禁-2 | 不动 dequeue_batch / clear_session / broadcast / stats 逻辑 | `queue.rs:127-217`（dequeue_batch / clear_session / stats）全部保持原状；broadcast 发送模式 `let _ = self.broadcast_tx.send(envelope);` 行为不变 | ✅ |
| 禁-3 | 不改任何生产调用点 | `rg "\.enqueue\("` 实测：所有 11 处生产调用点（stream_processor:192, turn_main_loop:161, rexec_state:77, sub_handle_out:189,366, state_manager:106, pipeline/state_manager:266, so_handlers:268,290,304, turn_persist:113,190）控制流均未改 | ✅ |
| 禁-4 | 不引入新并发原语 / `select!` / 取消路径 | diff 只增 `thiserror::Error` 一个 use + 两个结构项 + 测试 mod | ✅ |
| 禁-5 | 满队 Err 路径不进 broadcast_tx | `queue.rs:99-104` 在锁内早返，跳过 `:109` broadcast 与 `:113` stats 更新 —— 与原 drop 行为一致 | ✅ |
| 复用-1 | 全仓无同名 `EventQueueFull` / `EventId` | 实测 grep 全仓 `*.rs`，仅 `queue.rs` 命中 | ✅ |
| 复用-2 | core 内无等价"队列满"错误变体 | `src/crates/assembly/core/src/util/errors.rs:13-…` 的 `NortHingError` 含 `Service/Agent/Tool/AIClient/Session/Workspace/Validation/Io/Serialization/Http/Other/Semaphore/MCPError/ProcessError/NotFound…`，无 `Capacity`/`QueueFull`，独立类型合理 | ✅ |

## Strengths

- **diff 极小且聚焦**：74 / 5 全在 `queue.rs`，无散弹改动；trait 定义、生产调用点、`dequeue_batch` / `clear_session` / `stats()` 均零变动。
- **single-lock 守门保持**：满队判 size + push 仍在 `let queue_len = { let mut queue = self.queue.lock().await; … }` 的同一 `Mutex` 临界区内（`queue.rs:97-107`），Critical 旁路未引入新的锁窗口或 TOCTOU。
- **broadcast 语义严格保持**：`return Err` 早于 `broadcast_tx.send`，与原 `return Ok` 路径下 envelope 不进入广播的 drop 行为一一对应。
- **测试覆盖对处方要求 100%**：Normal→Err 断言 `max_queue_size: 2`；Critical→Ok 断言 `len()==3`（超 max=2 的 bypass 证据），直接覆盖 brief §⑤ 两段。
- **call site 普查比 brief 更彻底**：report 不仅列出 brief 已知的 4 处（stream_processor:192 / turn_persist:118,198 / sub_handle_out.rs），还补全了 turn_main_loop:161 / rexec_state:77 / state_manager:106 / pipeline/state_manager:266 / so_handlers:268,290,304 共 7 处额外固有调用点。
- **复用侦察实事求是**：确认全仓无重名，且在 `NortHingError` 14 个变体里逐一确认无 `Capacity` / `QueueFull` 等价变体，符合 brief 处方要求独立类型。

## Issues

### Critical

无。

### Important

无。

### Minor

- **M-1：brief 锚点行号与现行代码漂移。** brief §④ 给的 `turn_persist.rs:118,198` 在当前代码中是 `:113` / `:190`（实测）。report §调用点普查表已使用现行行号且完整列点，不构成实现缺陷，但建议策划侧把 brief 的 anchor 列表改成"grep 实时获取"或下一次 anchor 来源更新，避免评审者再次对账。
- **M-2：`cargo check --workspace` 输出尾部被截断。** report 贴的 cargo check tail 只有 `Checking …` 行，缺 `Finished` 总结行；MSVC 测试 tail 完整且 `1 passed` 即证明编译通过（否则 test 无法跑），所以不构成证据缺失，但报告粘贴习惯上应包含 `Finished …` / `error: …` 收尾行。属于过程纪律、不影响结论。
- **M-3：`EventQueueFull` 派生比 brief 多加了 `PartialEq, Eq`。** brief §① 仅写 `#[derive(Debug, Error)]`；diff 实现为 `Debug, Error, PartialEq, Eq`。加成让测试 `matches!(res3, Err(EventQueueFull { max_queue_size: 2, .. }))` 更干净（实测也是这么用的）。派生是加法不是替换、不改语义，建议默认允许；如果严格遵循 brief 写法，把 `PartialEq, Eq` 删掉会让测试回退到 `assert!(matches!(res3, Err(EventQueueFull { .. }))`，功能不受影响。建议保留现状并把 brief 下次补一句"可加 PartialEq, Eq 以便测试 match"。
- **M-4：`queue.rs:12` `use tracing::{debug, trace, warn};` 中 `warn` 仍被引用**（`dequeue_batch` 的 slow-delivery 路径仍在用，见 `queue.rs:153`），不属于 unused import；这点报告没提，本评审验证确认无 lint 风险，仅作澄清记录。

### Cannot verify from diff

- **C-1：report 声称的 `cargo check --workspace` 是否真实运行过、输出是否完整。** 仅看到 8 行 `Checking …`，缺总结行；无法从 diff 本身证伪，但同任务的 MSVC `cargo test` 通过已是更强证据（test 跑通必须先 check 成功）。建议编排者 trust 不再重跑。
- **C-2：报告外的下游模块是否真有 crate 依赖 `northhing-agent-stream::StreamEventSink` 之外的路径调用 `EventQueue::enqueue`。** grep 已扫全仓 `src/crates/**`，无遗漏；但如果有跨 workspace 的外部 binary 直接调 inherent 方法，本评审看不到。

## 双判决证据细节

### 行为变化面（Ok → Err）

11 处生产调用点的真实形态（实测当前代码，验证报告未漏报、无 `unwrap` / `expect` / `?` 传播路径）：

| 调用点 | 形式 | 风险 |
| --- | --- | --- |
| `stream_processor.rs:192` | `let _ = self.event_sink.enqueue(...)`（trait 方法返回 `()`） | 无 |
| `turn_main_loop.rs:161` | `let _ = self.event_queue.enqueue(...)` | 无 |
| `rexec_state.rs:77` | `let _ = self.event_queue.enqueue(...)` | 无 |
| `so_handlers.rs:268` | `let _ = self.event_queue.enqueue(...)` | 无 |
| `so_handlers.rs:290` | `let _ = self.event_queue.enqueue(...)` | 无 |
| `so_handlers.rs:304` | `let _ = self.event_queue.enqueue(...)` | 无 |
| `state_manager.rs:106` | `let _ = self.event_queue.enqueue(...)` | 无 |
| `pipeline/state_manager.rs:266` | `let _ = self.event_queue.enqueue(...)` | 无 |
| `sub_handle_out.rs:189` | `let _ = eq.enqueue(...)`（链式） | 无 |
| `sub_handle_out.rs:366` | `let _ = event_queue.enqueue(...)` | 无 |
| `turn_persist.rs:113` | `if let Err(error) = event_queue.enqueue(...)`（本就 match Err） | 无（变化兼容：原 match `NortHingResult::Err`、现 match `EventQueueFull`，均静默） |
| `turn_persist.rs:190` | `if let Err(queue_error) = event_queue.enqueue(...)`（本就 match Err） | 无（同上） |

判定：行为变化面零回归风险——所有"硬吞"或"已 match Err"的调用点都不被新 Err 类型破坏。

### 并发正确性专项

- **single-lock 守门保持**：判 size + push 仍在 `let queue_len = { let mut queue = self.queue.lock().await; … }` 同一临界区（`queue.rs:97-107`）。
- **Critical 旁路不引入新窗口**：Critical 路径仅短路 `if`，未跨 `await`、未再 `lock()`、未 `select!`，与原 Normal happy-path 完全同构。
- **stats 旁路读数**：`stats.pending_events = queue_len`（`queue.rs:115`）在 Critical 旁路时会被赋值为 `max_queue_size + 1` 之类大于 max 的值。实测 grep 全仓无 `event_queue.stats()` 或 `.pending_events` 的下游消费方（`QueueStats` 仅在 `queue.rs` 内自用，`pub async fn stats()` 公开但无人调），因此 `pending > max` 不会触发任何下游断言或监控误报。
- **broadcast 与 stats 在 Err 路径上一致跳过**：原 drop 行为是"既不入队也不广播也不增计数"，新 Err 行为是同样的"早返三件套"，行为保形。

### 复用核查（补强验证）

- 全仓 `*.rs` grep `EventQueueFull` / `pub type EventId`：仅 `queue.rs:16` 与 `:21` 命中，无别名或前置定义。
- `NortHingError` 14 个变体逐一确认无 `Capacity` / `QueueFull` / `Backpressure` 等价语义。
- `northhing-agent-stream::StreamEventSink` trait 方法签名返回 `()`（`src/crates/execution/agent-stream/src/types.rs:62-64`），与 `EventQueue` 的 inherent `enqueue` 返回 `Result` 互不耦合——agent-stream 不需要任何改动，impl 内部吸收 Err，与 brief §② 设计一致。

## Assessment

**Task quality:** Approved

**Reasoning:** diff 严格落到 brief §①–⑤ 的每一条要求且无禁区触发；single-lock 守门不变、broadcast 语义不变、所有生产调用点零回归；测试覆盖了 Normal 满队→Err 与 Critical 超上限 bypass 两个核心断言；唯一可挑剔的是 brief anchor 行号漂移（M-1，策划侧问题）与 cargo check tail 截断（M-2，过程纪律），均不阻塞合并。

### Spec Compliance
- ✅ Spec compliant
- ⚠️ Cannot verify from diff：报告外是否还有跨 workspace 外部 binary 直接调 `EventQueue::enqueue`（C-2）；cargo check 完整尾部（C-1，已用 test 通过作为间接证据）
