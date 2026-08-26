# Audit I7 SSE Log Collector Ring Buffer Report

## 1. 实现内容
- **`types.rs`**: `SseLogConfig` 移除 Default derive，新增 `pub const SSE_LOG_DEFAULT_MAX_OUTPUT: usize = 2000;`，手写 `impl Default for SseLogConfig` 返回 `max_output: Some(2000)`。更新 doc 注释说明兼作缓冲容量与错误输出上限。
- **`sse_log_collector.rs`**:
  - 缓冲结构由 `Vec<String>` 改为 `VecDeque<String>`，新增私有 `evicted: usize` 计数器。
  - `push` 方法在 `Some(max)` 下控制 `buffer.len()` 不超过 `max`（满时 `pop_front()` 并加一 `evicted`）。
  - `flush_on_error` 方法重构：在 `evicted > 0` 时格式化 `SSE history (showing last {len} of {total} events):`，正确显示真实接收总量；遍历当前 buffer 输出 index 0..len。
  - 模块内添加 3 个单元测试 `default_config_max_output_is_2000`、`bounded_collector_evicts_oldest_on_overflow`、`unbounded_collector_keeps_all_entries`。
- **`stream_processor.rs`**: 删除 line 420 的 `// No limit for now` 注释。

## 2. 复用侦察
- 全仓库 grep 结果显示 `SseLogCollector` 和 `SseLogConfig` 仅在 `src/crates/execution/agent-stream` 内使用：
  - 构造点：`types.rs`（定义/Default）、`stream_processor.rs:419-420`（`SseLogCollector::new(SseLogConfig::default())`）；
  - 调用点：`stream_processor.rs:444`（`c.lock().await.flush_on_error(...)`）；
  - 无全仓库第三处构造/调用点。
- 环形缓冲直接使用 Rust 标准库 `std::collections::VecDeque`，未自写数据结构。

## 3. 编译错误修复记录
- 本次改动零编译错误（0 compilation errors）。

## 4. 验证与输出原文

### cargo check --workspace
```
Checking northhing-agent-stream v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-stream)
Checking northhing-ai-adapters v0.2.10 (E:\agent-project\northing\src\crates\adapters\ai-adapters)
Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
```

### cargo test -p northhing-agent-stream --lib
```
running 51 tests
test sse_log_collector::tests::default_config_max_output_is_2000 ... ok
test tests::derives_watchdog_timeout_from_stream_idle_timeout ... ok
test sse_log_collector::tests::bounded_collector_evicts_oldest_on_overflow ... ok
test sse_log_collector::tests::unbounded_collector_keeps_all_entries ... ok

test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

## 5. 修改文件清单
- `src/crates/execution/agent-stream/src/types.rs`
- `src/crates/execution/agent-stream/src/sse_log_collector.rs`
- `src/crates/execution/agent-stream/src/stream_processor.rs`

## 6. 自审发现
- 无非预期改动，API 签名与字段全兼容。

## 7. 疑虑
- 无。
