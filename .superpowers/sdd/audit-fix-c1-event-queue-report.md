# Task Report — Audit C1: EventQueue 容量闸与广播投递解耦

## 1. 实现内容

1. **配置项** (`EventQueueConfig`)：
   - 增加 `pub heap_enabled: bool` 字段，并在 `Default` impl 中设为 `true`。
   - 添加 doc comment 说明：`false` 为 broadcast-only 模式，专供无堆消费者的宿主（桌面）使用，该模式永不返回 `EventQueueFull`。
2. **broadcast-only 语义** (`heap_enabled == false`)：
   - `enqueue` 方法在 `heap_enabled == false` 时跳过容量闸检查与 Priority BinaryHeap push。
   - 照常调用 `self.broadcast_tx.send(envelope)` 进行广播。
   - 递增 `stats.total_enqueued += 1`，`stats.pending_events` 保持 0。
   - 不触发 `self.notify.notify_one()`。
   - 队列长度 `queue.len().await` 恒为 0。
   - 恒定返回 `Ok(event_id)`。
3. **默认模式保值** (`heap_enabled == true`)：
   - `heap_enabled == true` 时维持完全相同的逻辑：容量闸在 broadcast 前、满队非 Critical 返回 `Err(EventQueueFull)`、Critical 旁路 push。
   - 原现有测试 `test_enqueue_queue_full_and_critical_bypass` 保持绿。
4. **接线**：
   - `system.rs` 中新增 `pub async fn init_agentic_system_with_queue_config(config: events::EventQueueConfig) -> Result<AgenticSystem>`。
   - `init_agentic_system()` 改为通过 `EventQueueConfig::default()` 委托新函数，保持原函数签名与行为零改动。
   - `kernel_facade/lifecycle.rs`（桌面宿主入口）改为调用 `init_agentic_system_with_queue_config(EventQueueConfig { heap_enabled: false, ..Default::default() })`。
   - CLI / server / w4_repro 等宿主保持 Default 不变。
5. **回归测试**：
   - 在 `queue.rs` tests 模块增加回归测试 `test_broadcast_only_mode_unbounded_delivery`。

## 2. 复用侦察

- 侦察符号：`EventQueue` / `EventQueueConfig` / `init_agentic_system`。
- 复用内容：
  - 复用了现有 `EventQueueConfig` 配置结构体，通过新增 `heap_enabled` 布尔标志解耦堆存储与广播，无需重构或拆分 `EventQueue` 结构。
  - 复用了 `init_agentic_system()` 的系统组装流水线，通过提取 `init_agentic_system_with_queue_config` 暴露参数化构造接口，不破坏任何已有的宿主（CLI/server/测试）调用契约。

## 3. 编译错误修在哪一层

- `E0560`: struct `EventQueueConfig` has no field named `heap_enabled`（TDD RED 阶段故意引发）—— 设计层：在 `EventQueueConfig` 定义中新增 `pub heap_enabled: bool` 字段。

## 4. 测试与输出原文

### 命令 1：`cargo check --workspace`

```
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check --workspace
```

输出原文尾部：
```text
warning: `northhing-core` (lib) generated 18 warnings (run `cargo fix --lib -p northhing-core` to apply 17 suggestions)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
warning: `northhing-cli` (bin "northhing-cli") generated 1 warning
warning: `northhing` (bin "northhing") generated 36 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 07s
```

### 命令 2：`events::queue` 单元测试与回归测试

```
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --all-features --lib events::queue
```

输出原文：
```text
   Compiling northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
warning: `northhing-core` (lib test) generated 18 warnings (run `cargo fix --lib -p northhing-core --tests` to apply 17 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 43.43s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-74d05f6aaf9ca71e.exe)

running 2 tests
test agentic::events::queue::tests::test_broadcast_only_mode_unbounded_delivery ... ok
test agentic::events::queue::tests::test_enqueue_queue_full_and_critical_bypass ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1049 filtered out; finished in 0.00s
```

### 命令 3：桌面编译门禁查验 (`cargo check -p northhing`)

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 11s
```

## 5. 文件清单

- `src/crates/assembly/core/src/agentic/events/queue.rs`
- `src/crates/assembly/core/src/agentic/system.rs`
- `src/crates/assembly/core/src/kernel_facade/lifecycle.rs`

## 6. 自审发现

- Spec 的 5 条要求全部满足。
- CLI、server、w4_repro 及现有测试调用路径行为均保持默认 `heap_enabled: true` 不变。
- 桌面启动入口通过 `kernel_facade/lifecycle.rs` 显式开启 `heap_enabled: false`，彻底解决桌面事件满 10k 丢包与 UI 假死隐患。
- 代码格式与注释符合规范，日志均为标准英文且无 emoji。

## 7. 疑虑

- 无。
