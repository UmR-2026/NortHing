# W3-3: F6 — FileWatchService 增量 watch/unwatch 实施报告

## 1. 实施概览 (Implementation Summary)

- **`src/crates/services/services-integrations/src/file_watch/service.rs:57-112`**:
  - `watch_path`: 插入 `watched_paths` 后，检查 `self.watcher` 锁槽位。若现存 `RecommendedWatcher`，直接调用增量 `watcher.watch(&path_buf, mode)`（mode 由配置 `watch_recursively` 确定），仅在 watcher 为 `None` 时才调用 `create_watcher()` 创建新 watcher 并启动后台任务。
  - `unwatch_path`: 从 `watched_paths` 中移除指定路径。若 `watched_paths` 变为空，置 `self.watcher = None`（丢弃 watcher 自动关闭 channel 发送端，后台 `spawn_blocking` 任务在 channel Disconnected 时安全退出）；若 `watched_paths` 仍有其它路径且本次成功移除了已注册路径，直接调用现存 watcher 的增量 `watcher.unwatch(&path_buf)`。
- **`src/crates/services/services-integrations/Cargo.toml:50`**:
  - 在 `[dev-dependencies]` 增加 `anyhow = { workspace = true }`，供测试中的 `TestEmitter` 实现 `EventEmitter` trait（不污染生产依赖与 feature 隔离）。
- **`src/crates/services/services-integrations/tests/file_watch_contracts.rs:9-20, 56-203`**:
  - 增加 `TestEmitter` 模拟事件接收器。
  - 增加 `file_watch_incremental_watch_and_unwatch_delivers_events` 测试（严格满足 Spec 4）：watch 路径 A → 增量 watch 路径 B → unwatch 路径 A → 触发文件系统变更 → 验证路径 B 事件正常发射、路径 A 事件被忽略 → unwatch 路径 B。
  - 增加 `file_watch_unwatch_unknown_path_is_noop` 测试：验证 unwatch 未跟踪路径为安全 no-op。

---

## 2. 复用侦察 (Reconnaissance & Reuse)

- 复用 `northhing_test_support::TestTempDir` 提供的 RAII 临时目录机制，确保测试目录在测试结束时自动清理。
- 复用 `northhing_events::EventEmitter` trait 及其标准签名与 `"file-system-changed"` 事件结构。
- 复用既有 `tests/file_watch_contracts.rs` 文件，保持测试落点集中。

---

## 3. 编译错误与分层排查 (Compile Errors & Resolution Layer)

- `E0433: cannot find module or crate anyhow in this scope`（机制/依赖声明层）：
  - 错误原因：`tests/file_watch_contracts.rs` 中 `TestEmitter` 实现 `EventEmitter::emit` 需要返回 `anyhow::Result<()>`，而 `anyhow` 在 `services-integrations` 中此前仅由 `mcp`/`remote-ssh-concrete` 特性条件依赖引入。
  - 修复层级：机制层（在 `Cargo.toml` 的 `[dev-dependencies]` 显式引入工作区已有 `anyhow` 依赖，零生产影响）。

---

## 4. 验证命令与完整输出 (Verification Commands & Output)

### 4.1 聚焦文件监听契约测试
```powershell
$env:PATH = "C:\msys64\mingw64\bin;C:\msys64\usr\bin;$env:PATH"; $env:TMP = "C:/msys64/tmp"; $env:TEMP = "C:/msys64/tmp"; cargo test -p northhing-services-integrations --test file_watch_contracts --features file-watch
```
输出：
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.56s
     Running tests\file_watch_contracts.rs (target\debug\deps\file_watch_contracts-9acbcdd1ba153993.exe)

running 4 tests
test file_watch_event_kind_serializes_snake_case ... ok
test file_watch_preserves_missing_path_error ... ok
test file_watch_unwatch_unknown_path_is_noop ... ok
test file_watch_incremental_watch_and_unwatch_delivers_events ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
```

### 4.2 services-integrations 全量测试
```powershell
$env:PATH = "C:\msys64\mingw64\bin;C:\msys64\usr\bin;$env:PATH"; $env:TMP = "C:/msys64/tmp"; $env:TEMP = "C:/msys64/tmp"; cargo test -p northhing-services-integrations --features product-full
```
输出：
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 10s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_integrations-d8dc3001f9f06974.exe)

running 47 tests
... (47 unit tests passed) ...
test result: ok. 47 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s

     Running tests\announcement_contracts.rs
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests\config_and_server_lifecycle.rs
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests\context_enhancer_and_catalog.rs
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\dynamic_tools_and_runtime.rs
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\file_watch_contracts.rs
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s

     Running tests\function_agent_contracts.rs
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.56s

     Running tests\git_contracts.rs
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

     Running tests\remote_ssh_contracts.rs
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\request_builders_and_adapters.rs
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\tool_names_and_protocol.rs
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\workspace_search_contracts.rs
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### 4.3 全工作区编译检查 (Workspace Check)
```powershell
$env:PATH = "C:\msys64\mingw64\bin;C:\msys64\usr\bin;$env:PATH"; $env:TMP = "C:/msys64/tmp"; $env:TEMP = "C:/msys64/tmp"; cargo check --workspace
```
输出：
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 52.18s
```

### 4.4 桌面包编译检查 (Desktop Compile Gate)
```powershell
$env:PATH = "C:\msys64\mingw64\bin;C:\msys64\usr\bin;$env:PATH"; $env:TMP = "C:/msys64/tmp"; $env:TEMP = "C:/msys64/tmp"; cargo check -p northhing
```
输出：
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 50.94s
```

---

## 5. 自审清单 (Self-Review Findings)

- **Spec 合规性**:
  - Spec 1 (`watch_path` 增量 watch + 错误格式同级 + 首条调用 `create_watcher`): 完全满足。
  - Spec 2 (`unwatch_path` 增量 unwatch + 清空置 None 终结后台任务): 完全满足。
  - Spec 3 (消除双任务窗口，存活任务数 ≤ 1): 完全满足。
  - Spec 4 (新增端到端增量监听与取消监听测试): 完全满足并通过。
  - Spec 5 (事件过滤/防抖/转换语义零改动，不动 `identity_watch.rs`): 完全满足。
- **并发与锁安全**:
  - `watched_paths` 与 `watcher` 锁独立获取并在进入下一阶段前释放，不存在嵌套死锁。
- **代码精炼度 (Ponytail YAGNI)**:
  - 仅用最精简的几行直接调用 `notify::Watcher::watch` 与 `unwatch`，无冗余封装与中间层。

---

## 6. 遗留疑虑 (Concerns)

- 无。
