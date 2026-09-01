# Audit Task I4+I5: LSP/MCP 子进程孤儿治理 Report

## 1. 实现内容

按 Brief 的 5 条 Spec 进行了精准修改（零新增抽象，零新增 helper/配置）：

1. **LSP spawn 配置** (`src/crates/assembly/core/src/service/lsp/process_spawn.rs`)
   - 导入 `northhing_services_core::process_manager`
   - 在 Command 创建后、`spawn()` 前添加 `cmd.kill_on_drop(true)` 与 `process_manager::configure_process_group(&mut cmd)`

2. **LSP Drop 治理** (`src/crates/assembly/core/src/service/lsp/process.rs`)
   - 保留 debug 日志，在 `drop()` 中增加 `if let Ok(mut child) = self.child.try_write() { let _ = child.start_kill(); }`
   - 在 `LspServerProcess.child` 字段旁添加 ponytail 注释说明残余空转天花板

3. **MCP start 配置** (`src/crates/services/services-integrations/src/mcp/server/process.rs`)
   - 在 `create_tokio_command` 之后、`spawn()` 之前添加 `cmd.kill_on_drop(true)` 与 `process_manager::configure_process_group(&mut cmd)`

4. **MCP stop 路径** (`src/crates/services/services-integrations/src/mcp/server/process.rs`)
   - 将 `child.kill().await` 替换为 `process_manager::terminate_child_process_tree(&mut child, Duration::from_millis(750)).await`
   - 保持 warn! 日志的英文语义与变量（`self.name`, `self.id`, `e`）

5. **MCP Drop 治理** (`src/crates/services/services-integrations/src/mcp/server/process.rs`)
   - 将 `Drop` 中的 `child.start_kill()` 替换为 `process_manager::spawn_child_process_tree_cleanup(child, Duration::from_millis(750))`

---

## 2. 复用侦察

在实现前对 `process_manager` (`src/crates/services/services-core/src/process_manager.rs`) 与 `flashgrep` 参考范式 (`src/crates/services/services-integrations/src/workspace_search/flashgrep/client.rs`) 进行了侦察：

- **`configure_process_group(&mut TokioCommand)`**
  - Unix: 设置 `process_group(0)` 使子进程拥有独立 Process Group
  - Windows: no-op（Windows 依赖全局 Job Object 兜底 kill-on-close）
  - 复用点：`process_spawn.rs` (LSP spawn) 与 `process.rs` (MCP start)

- **`kill_on_drop(true)`**
  - Tokio `Command` 内置方法，确保 Tokio runtime 丢弃 child 对象时触发 kill
  - 复用点：`process_spawn.rs` (LSP spawn) 与 `process.rs` (MCP start)

- **`terminate_child_process_tree(&mut Child, Duration)`**
  - Unix: 先对 `-PID` (PGID) 发送 SIGTERM，超时后发送 SIGKILL
  - Windows: 调用 `taskkill /PID <pid> /T /F` 强制递归关闭整个进程树
  - 复用点：`process.rs` (MCP stop)

- **`spawn_child_process_tree_cleanup(Child, Duration)`**
  - 转移 owned `Child` 所有权到后台专用线程/runtime 中执行异步进程树清理，避免阻塞 Synchronous `Drop::drop`
  - 复用点：`process.rs` (MCP Drop)

本任务无任何新抽象，100% 复用 `process_manager` 现成 helpers。

---

## 3. 编译错误记录与分层解析

- 无任何代码编译错误（0 错误）。
  *(注：运行 `cargo test -p northhing-core --lib lsp` 时因缺少 `product-full` feature 触发条件编译缺失，加上 `--features product-full` 标志后编译与测试即全绿。无代码层面层级修补。)*

---

## 4. 测试与输出原文

### 1) `cargo check --workspace`
```powershell
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
warning: `northhing-cli` (bin "northhing-cli") generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli` to apply 1 suggestion)
warning: `northhing` (bin "northhing") generated 37 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 56.64s
```

### 2) `cargo test -p northhing-core --features product-full --lib lsp`
```powershell
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 38s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)

running 15 tests
test service::lsp::plugin_loader::tests::validated_plugin_id_error_kinds_are_precise ... ok
test service::lsp::plugin_loader::tests::validated_plugin_id_rejects_unsafe_ids ... ok
test service::lsp::plugin_loader::tests::validated_plugin_id_accepts_safe_ids ... ok
test service::lsp::plugin_loader::tests::uninstall_missing_plugin_errors ... ok
test service::lsp::manager::tests::uninstall_file_delete_failure_rolls_back_registration ... ok
test service::lsp::plugin_loader::tests::install_rejects_corrupt_archive_with_zero_fs_effect ... ok
test service::lsp::plugin_loader::tests::install_rejects_missing_manifest_with_zero_fs_effect ... ok
test service::lsp::plugin_loader::tests::uninstall_refuses_target_outside_plugins_dir_via_symlink ... ok
test service::lsp::plugin_loader::tests::install_extract_failure_in_staging_leaves_no_half_install ... ok
test service::lsp::plugin_loader::tests::install_then_uninstall_roundtrip_no_residue ... ok
test service::lsp::plugin_loader::tests::install_already_installed_fails_no_residue ... ok
test service::lsp::plugin_loader::tests::load_plugin_rejects_mismatched_manifest_id ... ok
test service::lsp::manager::tests::uninstall_unregistered_plugin_keeps_unregister_error_and_skips_stop ... ok
test service::lsp::plugin_loader::tests::install_rejects_invalid_id_with_zero_fs_effect ... ok
test service::lsp::manager::tests::uninstall_stops_servers_by_resolved_language_keys ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 1037 filtered out; finished in 1.09s
```

### 3) `cargo test -p northhing-services-integrations --features product-full --lib mcp`
```powershell
    Finished `test` profile [unoptimized + debuginfo] target(s) in 9.21s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_integrations-482ce0ac9a8d71b5.exe)

running 10 tests
test mcp::tool_name::tests::normalize_name_for_mcp_keeps_ascii_word_chars_and_hyphen ... ok
test mcp::tool_name::tests::build_mcp_tool_name_normalizes_both_segments ... ok
test mcp::tool_name::tests::normalize_name_for_mcp_replaces_spaces_and_symbols ... ok
test mcp::auth::tests::clear_fails_closed_on_corrupted_vault_without_touching_file ... ok
test mcp::auth::tests::clear_fails_closed_on_truncated_vault_without_touching_file ... ok
test mcp::auth::tests::store_fails_closed_on_corrupted_vault_without_touching_file ... ok
test mcp::auth::tests::store_fails_closed_on_truncated_vault_without_touching_file ... ok
test mcp::auth::tests::load_returns_error_on_corrupted_vault ... ok
test mcp::auth::tests::vault_store_is_atomic_and_keeps_bak_of_previous_content ... ok
test mcp::auth::tests::vault_clear_deletes_file_when_last_entry_is_cleared ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 37 filtered out; finished in 0.02s
```

---

## 5. 文件清单

- `src/crates/assembly/core/src/service/lsp/process_spawn.rs`
- `src/crates/assembly/core/src/service/lsp/process.rs`
- `src/crates/services/services-integrations/src/mcp/server/process.rs`

---

## 6. 自审发现

- 所有修改均符合项目编码规范与平台兼容要求（Windows 下 Job Object + Taskkill /T /F，Unix 下 process_group + kill -TERM/-KILL）。
- LSP 的 `Drop` 中使用了 `try_write()` 获取写锁。因后台任务并不持久持有 child 的 RwLock，正常 Drop 时能够顺畅拿到写锁并触发 `start_kill()`。
- MCP 的 `stop()` 使用了 750ms 的 graceful timeout 清理进程树，`Drop` 中使用专用线程后台清理进程树，避免阻塞主进程 Drop 流程。

---

## 7. 疑虑与 Ponytail 声明

- **残余天花板（Ponytail 声明）**：
  若 LSP / MCP 的孙进程继承了 stdout 管道且主进程退出后孙进程未关闭 stdout，LSP 的 `read_task` 在 stdout EOF 前会处于空转状态（每 30 秒超时重置计数）。
  根据编排者预检裁定，按 F1 (1)+(2) 实施后，主进程/直接子进程被回收，管道在子进程终止时关闭，常规情况下能够触发 EOF 并使 read task 正常 break 退出。对于孙进程硬挂载 stdout 管道的极端残余情况，不引入额外的 JoinHandle 复杂跟踪逻辑，仅在代码中标注 ponytail 注释。
