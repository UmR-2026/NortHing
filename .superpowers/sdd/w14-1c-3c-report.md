# W14-1c-3c Report — REMOTE_STDIO clear seam（services-integrations）

## 1. 修改清单

- `src/crates/services/services-integrations/src/remote_ssh/workspace_search/service.rs`
  - 新增 `#[cfg(test)] pub async fn clear_remote_stdio_for_test()` 测试专用 seam，清空 `REMOTE_STDIO_SESSIONS` 与 `REMOTE_STDIO_OPEN_GUARDS`。
- `src/crates/services/services-integrations/src/remote_ssh/workspace_search/service_helpers.rs`
  - 导入 `clear_remote_stdio_for_test`；在 3 个 async 测试（`remote_search_rejects_non_linux_before_stdio_open`、`remote_search_context_ignores_stale_cache_before_resolving_connection`、`remote_search_open_guard_is_removed_when_stdio_spawn_fails`）开头调用 `clear_remote_stdio_for_test().await;`。

## 2. 编译错误与分层记录

- 遇到的编译错误：无（0 个 E0xxx 错误）。

## 3. 验证输出原文

### 3.1 Cargo Check (`cargo check -p northhing-services-integrations --features product-full`)

```text
    Checking northhing-runtime-ports v0.2.10 (E:\agent-project\northing\src\crates\contracts\runtime-ports)
    Checking terminal-core v0.2.10 (E:\agent-project\northing\src\crates\services\terminal)
    Checking northhing-services-core v0.2.10 (E:\agent-project\northing\src\crates\services\services-core)
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.96s
```

### 3.2 并行测试 (`cargo test -p northhing-services-integrations --all-features remote_search`)

```text
     Running unittests src\lib.rs (target\debug\deps\northhing_services_integrations-482ce0ac9a8d71b5.exe)

running 4 tests
test remote_ssh::workspace_search::service_helpers::tests::remote_search_cache_keys_normalize_workspace_root ... ok
test remote_ssh::workspace_search::service_helpers::tests::remote_search_rejects_non_linux_before_stdio_open ... ok
test remote_ssh::workspace_search::service_helpers::tests::remote_search_open_guard_is_removed_when_stdio_spawn_fails ... ok
test remote_ssh::workspace_search::service_helpers::tests::remote_search_context_ignores_stale_cache_before_resolving_connection ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 43 filtered out; finished in 0.29s
```

### 3.3 串行测试 (`cargo test -p northhing-services-integrations --all-features remote_search -- --test-threads=1`)

```text
     Running unittests src\lib.rs (target\debug\deps\northhing_services_integrations-482ce0ac9a8d71b5.exe)

running 4 tests
test remote_ssh::workspace_search::service_helpers::tests::remote_search_cache_keys_normalize_workspace_root ... ok
test remote_ssh::workspace_search::service_helpers::tests::remote_search_context_ignores_stale_cache_before_resolving_connection ... ok
test remote_ssh::workspace_search::service_helpers::tests::remote_search_open_guard_is_removed_when_stdio_spawn_fails ... ok
test remote_ssh::workspace_search::service_helpers::tests::remote_search_rejects_non_linux_before_stdio_open ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 43 filtered out; finished in 0.29s
```

## 4. 复用侦察

- 直接复用 `service.rs` 既有 `REMOTE_STDIO_SESSIONS` 与 `REMOTE_STDIO_OPEN_GUARDS` 全局 map，将其清理逻辑封装为 `#[cfg(test)] pub async fn clear_remote_stdio_for_test()`，避免模块外或单元测试内直接暴露/操作未封装的静态可变状态锁。

## 5. 偏离说明

- `service_helpers.rs:125` 的同步测试 `remote_search_cache_keys_normalize_workspace_root` 仅测试 `test_remote_stdio_session_key` 与 `test_remote_search_context_key` 的纯字符串/路径规范化逻辑，不触碰任何静态 map 全局状态，依 Brief 说明保持不动。

DONE
