# W15-1i Implementation Report — 桌面启动挂死修复（core IO 超时降级 + F1 挪出 UI 执行器）

## 改动摘要

1. **core 读/写 IO 超时包裹 (`services-core/src/json_store.rs`)**：
   - 提取 `io_timeout<F, T>(path, op, timeout, future)` 统一包装宏观/微观文件 IO 操作，默认超时 5 秒（`JSON_FILE_IO_TIMEOUT`），超时映射为 `std::io::Error(ErrorKind::TimedOut)`。
   - `read_optional` 内 `fs::metadata` 与 `fs::read_to_string` 分别由 `io_timeout` 包裹，超时分别抛出 `JsonFileStoreError::ReadMetadata` 与 `JsonFileStoreError::Read`，`source.kind() == TimedOut`。
   - `write_bytes_atomic` 内 `fs::create_dir_all`、`fs::write` (tmp)、`replace_file_from_temp`（内含 rename/remove/replace）、`fs::remove_file` (tmp) 及 `fs::write` (fallback) 全部由 `io_timeout` 包裹。超时映射为 `ErrorKind::TimedOut`，直接无缝接入既有 `is_retryable_write_error`（已内含 `TimedOut` 匹配）以及 `PermissionDenied` 降级直写。
   - 提供 `read_optional_timeout` 与 `write_bytes_atomic_timeout` 供自定义超时及自动化测试驱动。

2. **自动化超时回归测试 (`services-core/tests/json_store_contracts.rs`)**：
   - 新增 4 个确定性测试：
     - `json_store_io_timeout_read_with_pending_future_triggers_timeout`：针对 `read_to_string` 注入 `pending()` future，验证 10ms 超时触发及 `ErrorKind::TimedOut` 语义与错误文案。
     - `json_store_io_timeout_metadata_with_pending_future_triggers_timeout`：针对 `metadata` 注入 `pending()` future，验证超时。
     - `json_store_io_timeout_write_with_pending_future_triggers_timeout`：针对写路径 `write_temp` 注入 `pending()` future，验证超时。
     - `json_store_io_timeout_replace_with_pending_future_triggers_timeout`：针对写替换路径 `rename_replace` 注入 `pending()` future，验证超时。
     - `json_store_timed_out_is_retryable`：验证 `is_retryable_write_error` 确认 `TimedOut` 为可重试错误。
   - 既有 5 个测试 + 新增 5 个测试全部通过（共 10 个测试用例通过）。

3. **桌面 F1 移出 UI 执行器 (`desktop/src/ui_dioxus/app.rs`)**：
   - `app.rs:67-91` 中 F1 `use_future` 获取 `turn_runtime()` 句柄；若不可用输出 warn 日志返回。
   - 将 `api::ensure_room_session()` + `api::get_messages(&sid)` 派发至 `rt.spawn`（worker 多线程 runtime）。
   - 结果（纯 DTO `(Result<SessionId, KernelError>, Option<Result<Vec<SessionMessageDto>, KernelError>>)`）通过 `tokio::sync::oneshot` 回灌至 UI 执行器。
   - UI 线程仅通过 `rx.await` 等待通道消息（实验 D/E 已证 channel park 在 UI 线程安全），所有 Signal 写入（`session_id_signal.set`、`entries.set`）完全保留在 UI 线程侧执行。

4. **注释勘误 (`desktop/src/ui_dioxus/entry.rs`)**：
   - 将 `entry.rs:179-196` 处过时的 r3p4 注释修正，说明 sleeping use_future 已被实验 D/E 平反无害，真正的 poison 是在 dioxus 0.8.0-alpha.1 混合事件循环下返回 Pending 却被以 ~42k 次/秒高频反复轮询的 busy-wake future 冻结 tokio 时间驱动并饿死 tao 消息泵。

---

## Spec 逐条自核

| Spec 条目 | 核对情况 | 证据 |
|---|---|---|
| 1. `json_store.rs` 的 `read_optional` 内 `fs::metadata` + `fs::read_to_string` 包 timeout（5s 授权值）并转显式错误 | 满足 | `json_store.rs` 引入 `io_timeout(path, op, timeout, ...)`，超时转 `std::io::Error(ErrorKind::TimedOut)` 封装进 `JsonFileStoreError::ReadMetadata` / `Read` |
| 2. `write_bytes_atomic` 内单文件 fs 操作包 timeout 并接入既有重试/降级 | 满足 | `write_temp`、`replace_file_from_temp`、`remove_temp`、`fallback_overwrite` 均包 `io_timeout`；`TimedOut` 被 `is_retryable_write_error` 匹配触发最多 5 次重试与回退覆盖 |
| 3. 新增自动化测试真实执行超时路径（家规④），既有测试保持绿 | 满足 | `json_store_contracts.rs` 新增 5 个针对读、写、替换超时及重试的自动化测试，10/10 全部通过，耗时 0.03s |
| 4. `app.rs` F1 内核链改在 `turn_runtime()` worker rt 上执行，结果经 oneshot 回灌，Signal 留在 UI 侧，现有语义完整保留 | 满足 | `turn_runtime().spawn` 执行内核链，oneshot 回灌；`session_id_signal` 与 `entries` 在 UI 侧赋值；缓存命中、settings 错误警告、get_messages 失败等路径无缝保留 |
| 5. `entry.rs:179-188` 过时雷注释按 trace 报告新证据修正 | 满足 | 注释已更新为 busy-wake future + 冻结 timer 事实，风格保留一致 |
| 6. 运行验证：desktop debug 构建后运行 60–90s，窗口 `Responding=True` 且主线程 CPU 不钉死 | 满足 | 构建完成；实测持续运行 70s，`Responding = True`，70 秒总处理器时间仅 1.35s（CPU 单核占用 < 2%，彻底消除原 100% 满转卡死）；截图已落盘 |

---

## 复用侦察节

- **检索符号**：
  - `tokio::time::timeout`：检索全仓 `src/`，发现 30+ 处散装调用（如 `stream_handler`、`acp`、`terminal` 等），低层 `services-core` 内此前无通用 IO 超时助手。
  - `tokio::sync::oneshot`：检索全仓，发现 `acp`、`cdp_client`、`lsp/process_protocol` 均使用标准 `oneshot::channel` 跨执行器回灌结果。
  - `turn_runtime()`：检索 `desktop`，定位至 `src/apps/desktop/src/app_state/turn_runtime.rs`，`main.rs:77` 在 worker 线程启动时已将 MultiThread 运行时句柄写入。
  - `api_events.rs:101-109`：查阅 handle 获取与通道桥接先例。
- **复用项**：
  - 直接复用 `crate::app_state::turn_runtime::turn_runtime()` 获取多线程 worker 运行时 Handle。
  - 直接复用 `tokio::sync::oneshot` 作为 worker 任务向 UI 线程传递 DTO 的异步通道。
  - 直接复用 `json_store.rs` 现成的 `is_retryable_write_error` 中的 `ErrorKind::TimedOut` 分支。
- **新写等价物及理由**：
  - 在 `json_store.rs` 内新增私有/模块辅助函数 `io_timeout<F, T>(path, op, timeout, future)`（15 行）：`services-core` 属于低层独立服务 crate，禁止反向依赖上层 crate；通过短小精悍的内联 helper 消除 6 处文件系统调用中冗余的 match/map 模板代码。

---

## 编译与告警处理（机制层 / 设计层）

1. `warning: associated function replace_file_from_temp is never used`：
   - 修复层级：**设计层**
   - 处理方式：统一将 `replace_file_from_temp(target_path, tmp_path, timeout: Duration)` 增加 `timeout` 参数，消除冗余的 `_timeout` 后缀孪生函数与 dead_code 警告。

---

## 验证命令与输出原文

### 1. `cargo test -p northhing-services-core json_store`

```
warning: unused imports: `PathBuf` and `Path`
 --> src\crates\services\services-core\tests\session_layout_contracts.rs:3:17
  |
3 | use std::path::{Path, PathBuf};
  |                 ^^^^  ^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: `northhing-services-core` (test "session_layout_contracts") generated 1 warning (run `cargo fix --test "session_layout_contracts" -p northhing-services-core` to apply 1 suggestion)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.55s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_core-f64d1e29c2ef8d22.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 52 filtered out; finished in 0.00s

     Running tests\diagnostic_log_redaction.rs (target\debug\deps\diagnostic_log_redaction-1cdeb332fb787908.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\json_store_contracts.rs (target\debug\deps\json_store_contracts-8efbd074796955c7.exe)

running 10 tests
test json_store_timed_out_is_retryable ... ok
test json_store_reports_no_parent_directory ... ok
test json_store_returns_none_for_missing_file ... ok
test json_store_write_bytes_atomic_round_trips_raw_bytes ... ok
test json_store_creates_parent_dirs_and_round_trips_payload ... ok
test json_store_write_bytes_atomic_overwrites_and_cleans_up_temp_files ... ok
test json_store_io_timeout_replace_with_pending_future_triggers_timeout ... ok
test json_store_io_timeout_write_with_pending_future_triggers_timeout ... ok
test json_store_io_timeout_metadata_with_pending_future_triggers_timeout ... ok
test json_store_io_timeout_read_with_pending_future_triggers_timeout ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests\service_contracts.rs (target\debug\deps\service_contracts-9c554435af422588.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s

     Running tests\session_contracts.rs (target\debug\deps\session_contracts-a3df5edb7b08e6a0.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\session_layout_contracts.rs (target\debug\deps\session_layout_contracts-276a3f6eab799973.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

     Running tests\session_metadata_contracts.rs (target\debug\deps\session_metadata_contracts-9567447db940c7d1.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

     Running tests\session_page_contracts.rs (target\debug\deps\session_page_contracts-22f927a682ca77ae.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s

     Running tests\session_usage_contracts.rs (target\debug\deps\session_usage_contracts-66ea006f2556da03.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\token_usage_contracts.rs (target\debug\deps\token_usage_contracts-1333e389fc42813f.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s
```

### 2. `cargo check --workspace`

```
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 48.66s
```

---

## 运行验证数值与截图证据

- **构建命令**：`C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo build -p northhing`
- **运行命令**：`Start-Process -FilePath "E:\agent-project\NortHing\target\debug\northhing.exe"`（PID 44968）
- **采样数据**：
  - `t = 0s`: `Responding = True`, `TotalProcessorTime = 00:00:00.4062500`, `WorkingSet64 = 61284352`
  - `t = 30s`: `Responding = True`, `TotalProcessorTime = 00:00:00.4531250`, `WorkingSet64 = 61251584`
  - `t = 70s`: `Responding = True`, `TotalProcessorTime = 00:00:01.3593750`, `WorkingSet64 = 61292544`
- **对比证据**：
  - 修复前：启动 1 秒内主线程立即 100% 满转卡死（4.2 万次/秒轮询），4/4 必挂，`Responding = False`，CPU 持续满负荷。
  - 修复后：运行 70 秒 `Responding = True`，总 CPU 消耗仅 1.35 秒（平均 CPU 占用 < 2%），主线程完全摆脱满转风暴。
- **窗口截图**：
  - 截图命令：`C:\WINDOWS\TEMP\opencode\win-shot.ps1 -OutFile "E:\agent-project\NortHing\screenshots\w15-1i-desktop-70s.png" -ProcessName "northhing"`
  - 截图输出：`saved E:\agent-project\NortHing\screenshots\w15-1i-desktop-70s.png 894x828 title=northhing - consult room (dioxus)`
  - 视觉验证确认：窗口正常呈现，标题与主题卡片正常加载，无白屏崩溃或无响应标识。

---

## 遗留问题与后续单说明

- **点击输入框卡死**：用户反馈“现在是点击输入后才会卡死”。根据 brief §4 明确界外声明（“`send_action` / `stop_turn` / `api_events.rs` / 其它 UI 执行器内核调用点为同型问题，刻意留作后续单”），`send_action`（`app.rs:274` 附近）目前仍直接在 UI 执行器内同步 await 内核调用，需在下一单依本单 F1 相同方案迁移至 `turn_runtime`。本单范围（启动期 F1 + core json_store 超时）已完整闭环并验证通过。

---

---

## I1 修复记录

### 改动摘要
- **核查消费方**：通过 `rg` 全仓核查确认 `read_optional_timeout` 与 `write_bytes_atomic_timeout` 零外部调用方，测试套件 `tests/json_store_contracts.rs` 均直调 `io_timeout` 或基础方法，未触及超时后缀变体。
- **删除无 owner 抽象**：删除 `JsonFileStore::read_optional_timeout` 与 `JsonFileStore::write_bytes_atomic_timeout` 两个公开方法。
- **内联默认超时**：`read_optional` 与 `write_bytes_atomic` 直接使用 `Self::DEFAULT_TIMEOUT`（5s 授权常量）驱动底层 `io_timeout`，消除冗余间接层与 API 表面膨胀。
- **提交记录**：`d1d31b8 refactor(services-core): drop unused timeout variants (W15-1i review I1)`

### 验证 1：`cargo test -p northhing-services-core json_store`

```text
warning: unused imports: `PathBuf` and `Path`
 --> src\crates\services\services-core\tests\session_layout_contracts.rs:3:17
  |
3 | use std::path::{Path, PathBuf};
  |                 ^^^^  ^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: `northhing-services-core` (test "session_layout_contracts") generated 1 warning (run `cargo fix --test "session_layout_contracts" -p northhing-services-core` to apply 1 suggestion)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7.91s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_core-f64d1e29c2ef8d22.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 52 filtered out; finished in 0.00s

     Running tests\diagnostic_log_redaction.rs (target\debug\deps\diagnostic_log_redaction-1cdeb332fb787908.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\json_store_contracts.rs (target\debug\deps\json_store_contracts-8efbd074796955c7.exe)

running 10 tests
test json_store_timed_out_is_retryable ... ok
test json_store_reports_no_parent_directory ... ok
test json_store_returns_none_for_missing_file ... ok
test json_store_write_bytes_atomic_overwrites_and_cleans_up_temp_files ... ok
test json_store_write_bytes_atomic_round_trips_raw_bytes ... ok
test json_store_creates_parent_dirs_and_round_trips_payload ... ok
test json_store_io_timeout_replace_with_pending_future_triggers_timeout ... ok
test json_store_io_timeout_metadata_with_pending_future_triggers_timeout ... ok
test json_store_io_timeout_read_with_pending_future_triggers_timeout ... ok
test json_store_io_timeout_write_with_pending_future_triggers_timeout ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests\service_contracts.rs (target\debug\deps\service_contracts-9c554435af422588.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s

     Running tests\session_contracts.rs (target\debug\deps\session_contracts-a3df5edb7b08e6a0.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\session_layout_contracts.rs (target\debug\deps\session_layout_contracts-276a3f6eab799973.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

     Running tests\session_metadata_contracts.rs (target\debug\deps\session_metadata_contracts-9567447db940c7d1.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

     Running tests\session_page_contracts.rs (target\debug\deps\session_page_contracts-22f927a682ca77ae.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s

     Running tests\session_usage_contracts.rs (target\debug\deps\session_usage_contracts-66ea006f2556da03.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\token_usage_contracts.rs (target\debug\deps\token_usage_contracts-1333e389fc42813f.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s
```

### 验证 2：`cargo check --workspace`

```text
    Checking northhing-services-core v0.2.10 (E:\agent-project\northing\src\crates\services\services-core)
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 10s
```

status: DONE
