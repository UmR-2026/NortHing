# Task Report — W2-2: F9 `spawn_child_process_tree_cleanup` 模式收口

## 1. 实现内容

根据 Brief 要求，完成了 F9 `spawn_child_process_tree_cleanup` 模式收口收拢：

1. `src/crates/services/services-integrations/src/workspace_search/flashgrep/client.rs`:
   - 移除了 `AsyncDaemonClient` 的 `Drop` 实现中对 `process_manager::spawn_child_process_tree_cleanup` 的调用，替换为 `drop(self.take_child_for_drop())` 并说明直接子进程靠 `kill_on_drop(true)` 兜底。
   - 删除了已无地方引用的 `DROP_CLEANUP_TIMEOUT` 常量。
2. `src/crates/services/services-integrations/src/mcp/server/process.rs`:
   - 保留了 `MCPServerProcess` 的 `Drop` 实现中对 `process_manager::spawn_child_process_tree_cleanup` 的调用，并添加单行注释说明 Windows 下 node MCP 服务由 `cmd.exe /c` 包装产生孙进程，需要 tree-kill。
3. `src/crates/services/services-core/src/process_manager.rs`:
   - 为 `spawn_child_process_tree_cleanup` 函数追加 doc comment，说明其使用边界（仅适用于有孙进程的 shell-wrapped spawn；直产二进制带 `kill_on_drop(true)` 无需调用）。函数体与签名保持零修改。

## 2. 复用侦察（Re-use Reconnaissance）

1. **`take_child_for_drop` 调用点核查**：
   - 全仓搜索 `take_child_for_drop` 仅存在于 `flashgrep/client.rs` 的 `AsyncDaemonClient`（:648 定义，:669 使用）。
   - 修改后在 `AsyncDaemonClient::drop` 中通过 `drop(self.take_child_for_drop())` 继续复用该方法，无废弃方法。

2. **`spawn_child_process_tree_cleanup` 调用点核查**：
   - 全仓搜索 `spawn_child_process_tree_cleanup`（排除 docs 与 brief/report）：
     - 调整前两处调用：`flashgrep/client.rs:670` 与 `mcp/server/process.rs:400`。
     - 调整后仅剩 `mcp/server/process.rs:400` 一处调用。

## 3. 编译错误修复记录

- 本次修改零编译错误（0 errors）。

## 4. 测试与输出原文

### (1) `cargo check --workspace`

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP; cargo check --workspace
```

输出原文：
```text
    Checking northhing-services-core v0.2.10 (E:\agent-project\northing\src\crates\services\services-core)
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 57.39s
```

### (2) `cargo test -p northhing-services-integrations --features mcp`

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP; & "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-integrations --features mcp
```

输出原文：
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 9.56s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_integrations-de98792c0c7fb295.exe)

running 10 tests
test mcp::tool_name::tests::build_mcp_tool_name_normalizes_both_segments ... ok
test mcp::tool_name::tests::normalize_name_for_mcp_keeps_ascii_word_chars_and_hyphen ... ok
test mcp::tool_name::tests::normalize_name_for_mcp_replaces_spaces_and_symbols ... ok
test mcp::auth::tests::clear_fails_closed_on_corrupted_vault_without_touching_file ... ok
test mcp::auth::tests::clear_fails_closed_on_truncated_vault_without_touching_file ... ok
test mcp::auth::tests::store_fails_closed_on_corrupted_vault_without_touching_file ... ok
test mcp::auth::tests::store_fails_closed_on_truncated_vault_without_touching_file ... ok
test mcp::auth::tests::load_returns_error_on_corrupted_vault ... ok
test mcp::auth::tests::vault_store_is_atomic_and_keeps_bak_of_previous_content ... ok
test mcp::auth::tests::vault_clear_deletes_file_when_last_entry_is_cleared ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests\config_and_server_lifecycle.rs (target\debug\deps\config_and_server_lifecycle-c40083878b6d6648.exe)
running 18 tests
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\context_enhancer_and_catalog.rs (target\debug\deps\context_enhancer_and_catalog-fc92ef7e23213feb.exe)
running 3 tests
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\dynamic_tools_and_runtime.rs (target\debug\deps\dynamic_tools_and_runtime-d1fe02dc22093fcd.exe)
running 9 tests
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\request_builders_and_adapters.rs (target\debug\deps\request_builders_and_adapters-100949ed028fc0ef.exe)
running 4 tests
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\tool_names_and_protocol.rs (target\debug\deps\tool_names_and_protocol-46cd1d9bbe1a16d3.exe)
running 3 tests
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### (3) `cargo test -p northhing-services-integrations --features workspace-search workspace_search`

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP; & "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-integrations --features workspace-search workspace_search
```

输出原文：
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 8.02s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_integrations-92e2a76c3728f20e.exe)

running 5 tests
test workspace_search::service::tests::content_search_output_modes_use_current_flashgrep_protocol_modes ... ok
test workspace_search::service::tests::content_search_converts_legacy_line_matches ... ok
test workspace_search::flashgrep::rpc_client::tests::drains_remote_stdio_content_length_messages ... ok
test workspace_search::flashgrep::rpc_client::tests::drains_remote_stdio_initialize_response_with_legacy_search_modes ... ok
test workspace_search::service_session::tests::schedule_repo_release_for_test_releases_idle_session ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests\workspace_search_contracts.rs (target\debug\deps\workspace_search_contracts-78b8a0b3f1ace919.exe)

running 3 tests
test workspace_search::daemon_binary_contract_lists_current_platform_candidate ... ok
test workspace_search::daemon_missing_hint_preserves_env_override_guidance ... ok
test workspace_search::service_constructs_without_core_runtime_dependencies ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### (4) `cargo test -p northhing-services-core`

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP; & "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-core
```

输出原文：
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.94s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_core-f64d1e29c2ef8d22.exe)

running 52 tests
...
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

## 5. 文件清单

1. `src/crates/services/services-integrations/src/workspace_search/flashgrep/client.rs`
2. `src/crates/services/services-integrations/src/mcp/server/process.rs`
3. `src/crates/services/services-core/src/process_manager.rs`
4. `.superpowers/sdd/w2-2-f9-tree-cleanup-report.md` (本报告文件)

## 6. 自审发现

- flashgrep 在 spawn 时已经通过 `create_tokio_command_for_spawn` 设置了 `kill_on_drop(true)`，无需依赖异步 cleanup helper。
- `spawn_child_process_tree_cleanup` 仅为 MCP（特别是 Windows 下 cmd.exe /c 包装）保留，边界注释清晰。
- 零额外抽象，无任何破损或遗留未使用警告。

## 7. 疑虑

- 无。
