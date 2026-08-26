# Task W2-1 Report — F5 进程组/kill_on_drop 模式根治

## 1. 实现内容

1. 在 `services-core/src/process_manager.rs` 新增 `pub fn create_tokio_command_for_spawn<S: AsRef<std::ffi::OsStr>>(program: S) -> TokioCommand`，组合 `create_tokio_command` + `kill_on_drop(true)` + `configure_process_group(&mut cmd)`，并附规范英文 doc comment 说明与 `create_tokio_command`（用于一次性 output()/status() 任务）的使用区别。
2. 在 `services-core/src/lib.rs` re-export `create_tokio_command_for_spawn`。
3. 对 F5 列出的所有候选点进行了逐一审计与分类，真实 `spawn()` 点统一迁移至 `create_tokio_command_for_spawn`，一次性 `output()` 点保持原样不动。
4. 将三处先前手工配置 `kill_on_drop(true)` + `configure_process_group` 的参考点（`lsp/process_spawn.rs`、`mcp/server/process.rs`、`flashgrep/client.rs`）统一收口至 `create_tokio_command_for_spawn`，消除了重复的手工配置代码。

---

## 2. 复用侦察节

全仓检索 `create_tokio_command`、`create_command` 及 `configure_process_group` 调用点：
- **`services-core/src/process_manager.rs` 内部**：`terminate_child_process_tree` 中的 `kill` / `taskkill` 调用均为一次性状态查询/退出命令（`.status().await`），保持 `create_tokio_command`。
- **`services-core/src/system/command.rs:280`**：`run_command` 为一次性命令执行器，通过 `.output().await` 消费结果，保持 `create_tokio_command`。
- **`services-integrations/src/git/utils.rs:201`**：`execute_git_command_raw` 为一次性 Git 命令执行器，通过 `.output().await` 消费结果，保持 `create_tokio_command`。
- **`assembly/core/src/service/workspace/workspace_info_impl.rs:380`**：`git status --porcelain` 查询通过 `.output().await` 消费，保持 `create_tokio_command`。
- **`assembly/core/src/agentic/tools/implementations/computer_use_actions/utilities.rs`**：行 227（`pbpaste`）、行 239（`powershell Get-Clipboard`）、行 274（`wl-paste/xclip/xsel`）均为一次性剪贴板读取，通过 `.output().await` 消费，保持 `create_tokio_command`；行 295 为管道式剪贴板写入子进程，通过 `.spawn()` 启动，迁移至 `create_tokio_command_for_spawn`。
- **`assembly/core/src/agentic/tools/implementations/computer_use_actions/system_actions/app_control.rs:425`**：`run_script` 工具通过 `.spawn()` 启动带超时取消的子进程，迁移至 `create_tokio_command_for_spawn`，移除手工 `.kill_on_drop(true)`。
- **`assembly/core/src/service/lsp/process_command.rs`**：行 125, 142, 178, 187, 211 均为 LSP 进程构建路径，供 `process_spawn.rs` 中的 `.spawn()` 消费，全部迁移至 `create_tokio_command_for_spawn`。
- **`assembly/core/src/service/lsp/process_spawn.rs`**：移除冗余的手工 `kill_on_drop(true)` 与 `configure_process_group` 调用及未使用的 import。
- **`services-integrations/src/mcp/server/process.rs:85`**：迁移至 `create_tokio_command_for_spawn`，移除手工两行。
- **`services-integrations/src/workspace_search/flashgrep/client.rs:423`**：迁移至 `create_tokio_command_for_spawn`，移除手工两行。
- **`interfaces/acp/src/client/requirements/req_session.rs:47`**：一次性命令版本检测，通过 `tokio::time::timeout(..., command.output())` 消费，保持 `create_tokio_command`。
- **`interfaces/acp/src/client/manager_process_lifecycle.rs:72, 104, 121`**：一次性 `kill` / `taskkill` 调用，保持 `create_tokio_command`。
- **`interfaces/acp/src/client/manager_transport.rs:131`**：ACP 客户端独立传输通道，自身管理生命周期与退出，不在 F5 清单内，保持不变。
- **`services/terminal/src/exec/output.rs:422`**：终端执行管道拥有专用的 `configure_pipe_process_group` 与 SIGINT 转发控制，保持自身变体不变。
- **结论**：全仓未见第三方重复构造器；除真实长命 spawn 站点外，所有短生命周期 output 任务均维持原样，确保信号路由无回退。

---

## 3. 站点分类表

| 文件与行号 | 类型 | 处置 | 理由 |
|---|---|---|---|
| `assembly/core/src/service/lsp/process_command.rs:125` | spawn | 迁移到 `create_tokio_command_for_spawn` | Windows batch 解析出的 node 命令，供 LSP 服务器长命子进程 spawn |
| `assembly/core/src/service/lsp/process_command.rs:142` | spawn | 迁移到 `create_tokio_command_for_spawn` | 独立可执行二进制 LSP 服务器长命子进程 spawn |
| `assembly/core/src/service/lsp/process_command.rs:178` | spawn | 迁移到 `create_tokio_command_for_spawn` | Windows Bash fallback LSP 服务器长命子进程 spawn |
| `assembly/core/src/service/lsp/process_command.rs:187` | spawn | 迁移到 `create_tokio_command_for_spawn` | Unix Bash fallback LSP 服务器长命子进程 spawn |
| `assembly/core/src/service/lsp/process_command.rs:211` | spawn | 迁移到 `create_tokio_command_for_spawn` | Node 运行时 LSP 服务器长命子进程 spawn |
| `services-core/src/system/command.rs:280` | output | 原样不动（保持 `create_tokio_command`） | 一次性 `run_command` 通过 `.output().await` 执行，非长命子进程 |
| `services-integrations/src/git/utils.rs:201` | output | 原样不动（保持 `create_tokio_command`） | 一次性 `execute_git_command_raw` 通过 `.output().await` 执行，保持进程组/信号语义 |
| `assembly/core/src/service/workspace/workspace_info_impl.rs:380` | output | 原样不动（保持 `create_tokio_command`） | 一次性 `git status --porcelain` 检查通过 `.output().await` 执行 |
| `assembly/core/src/agentic/tools/implementations/computer_use_actions/utilities.rs:227` | output | 原样不动（保持 `create_tokio_command`） | 一次性 macOS `pbpaste` 读取通过 `.output().await` 执行 |
| `assembly/core/src/agentic/tools/implementations/computer_use_actions/utilities.rs:239` | output | 原样不动（保持 `create_tokio_command`） | 一次性 Windows PowerShell `Get-Clipboard` 读取通过 `.output().await` 执行 |
| `assembly/core/src/agentic/tools/implementations/computer_use_actions/utilities.rs:274` | output | 原样不动（保持 `create_tokio_command`） | 一次性 Linux `wl-paste`/`xclip`/`xsel` 探测通过 `.output().await` 执行 |
| `assembly/core/src/agentic/tools/implementations/computer_use_actions/utilities.rs:295` | spawn | 迁移到 `create_tokio_command_for_spawn` | 管道式剪贴板写入通过 `.spawn()` 启动子进程并写入 stdin，防止 drop 时孤儿泄漏 |
| `assembly/core/src/agentic/tools/implementations/computer_use_actions/system_actions/app_control.rs:425` | spawn | 迁移到 `create_tokio_command_for_spawn` | `run_script` 启动带超时取消的外部脚本进程，移除原手工 `.kill_on_drop(true)` |
| `assembly/core/src/service/lsp/process_spawn.rs:50-51` | spawn (手工合规) | 迁移统一模式 | 移除手工两行（已由 `build_command` 统一提供），清理无用 import |
| `services-integrations/src/mcp/server/process.rs:85-92` | spawn (手工合规) | 迁移到 `create_tokio_command_for_spawn` | 替换为新构造器并移除手工两行 |
| `services-integrations/src/workspace_search/flashgrep/client.rs:423-431` | spawn (手工合规) | 迁移到 `create_tokio_command_for_spawn` | 替换为新构造器并移除手工两行 |

---

## 4. 编译错误分析与层级

- `northhing-core --lib lsp` 单测初次运行缺 `--features product-full` 报 E0433：机制层修复（Cargo feature 门控参数补齐 `--features product-full`，符合 brief 预期与先前轮次记录）。

---

## 5. 测试与验证输出原文

### 5.1 `cargo check --workspace`
```text
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP; cargo check --workspace
    Checking northhing-services-core v0.2.10 (E:\agent-project\northing\src\crates\services\services-core)
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 44s
```

### 5.2 LSP 聚焦测试 (`cargo test -p northhing-core --features product-full --lib lsp`)
```text
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --features product-full --lib lsp
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 03s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)

running 15 tests
test service::lsp::plugin_loader::tests::validated_plugin_id_error_kinds_are_precise ... ok
test service::lsp::plugin_loader::tests::validated_plugin_id_accepts_safe_ids ... ok
test service::lsp::plugin_loader::tests::validated_plugin_id_rejects_unsafe_ids ... ok
test service::lsp::plugin_loader::tests::uninstall_missing_plugin_errors ... ok
test service::lsp::manager::tests::uninstall_file_delete_failure_rolls_back_registration ... ok
test service::lsp::plugin_loader::tests::install_rejects_corrupt_archive_with_zero_fs_effect ... ok
test service::lsp::plugin_loader::tests::install_rejects_missing_manifest_with_zero_fs_effect ... ok
test service::lsp::plugin_loader::tests::uninstall_refuses_target_outside_plugins_dir_via_symlink ... ok
test service::lsp::plugin_loader::tests::install_extract_failure_in_staging_leaves_no_half_install ... ok
test service::lsp::manager::tests::uninstall_unregistered_plugin_keeps_unregister_error_and_skips_stop ... ok
test service::lsp::plugin_loader::tests::load_plugin_rejects_mismatched_manifest_id ... ok
test service::lsp::plugin_loader::tests::install_already_installed_fails_no_residue ... ok
test service::lsp::plugin_loader::tests::install_then_uninstall_roundtrip_no_residue ... ok
test service::lsp::plugin_loader::tests::install_rejects_invalid_id_with_zero_fs_effect ... ok
test service::lsp::manager::tests::uninstall_stops_servers_by_resolved_language_keys ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 1037 filtered out; finished in 1.09s
```

### 5.3 `northhing-services-core` 聚焦测试
```text
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-core
    Finished `test` profile [unoptimized + debuginfo] target(s) in 8.63s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_core-f64d1e29c2ef8d22.exe)

running 52 tests
...
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests\diagnostic_log_redaction.rs (target\debug\deps\diagnostic_log_redaction-1cdeb332fb787908.exe)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

     Running tests\json_store_contracts.rs (target\debug\deps\json_store_contracts-8efbd074796955c7.exe)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.39s

     Running tests\service_contracts.rs (target\debug\deps\service_contracts-9c554435af422588.exe)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests\session_contracts.rs (target\debug\deps\session_contracts-a3df5edb7b08e6a0.exe)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\session_layout_contracts.rs (target\debug\deps\session_layout_contracts-276a3f6eab799973.exe)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests\session_metadata_contracts.rs (target\debug\deps\session_metadata_contracts-9567447db940c7d1.exe)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\session_page_contracts.rs (target\debug\deps\session_page_contracts-22f927a682ca77ae.exe)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\session_usage_contracts.rs (target\debug\deps\session_usage_contracts-66ea006f2556da03.exe)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\token_usage_contracts.rs (target\debug\deps\token_usage_contracts-1333e389fc42813f.exe)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests northhing_services_core
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.79s
```

### 5.4 `northhing-services-integrations --features mcp` 聚焦测试
```text
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-integrations --features mcp
    Finished `test` profile [unoptimized + debuginfo] target(s) in 48.00s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_integrations-de98792c0c7fb295.exe)

running 10 tests
...
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests\config_and_server_lifecycle.rs (target\debug\deps\config_and_server_lifecycle-c40083878b6d6648.exe)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\context_enhancer_and_catalog.rs (target\debug\deps\context_enhancer_and_catalog-fc92ef7e23213feb.exe)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\dynamic_tools_and_runtime.rs (target\debug\deps\dynamic_tools_and_runtime-d1fe02dc22093fcd.exe)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\tool_names_and_protocol.rs (target\debug\deps\tool_names_and_protocol-46cd1d9bbe1a16d3.exe)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\workspace_search_contracts.rs (target\debug\deps\workspace_search_contracts-afc26fd5a11e243f.exe)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 5.5 边界检查与桌面检查
- `node scripts/check-core-boundaries.mjs`: `Core boundary check passed.`
- `cargo check -p northhing`: `Finished dev profile target(s) in 1m 00s` (0 errors)

---

## 6. 文件清单

- `src/crates/services/services-core/src/process_manager.rs`
- `src/crates/services/services-core/src/lib.rs`
- `src/crates/assembly/core/src/service/lsp/process_command.rs`
- `src/crates/assembly/core/src/service/lsp/process_spawn.rs`
- `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_actions/utilities.rs`
- `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_actions/system_actions/app_control.rs`
- `src/crates/services/services-integrations/src/mcp/server/process.rs`
- `src/crates/services/services-integrations/src/workspace_search/flashgrep/client.rs`
- `.superpowers/sdd/w2-1-f5-spawn-constructor-report.md`

---

## 7. 自审发现

- 代码改动完全符合 YAGNI 原则与 Ponytail 规范，无过度抽象。
- `services-core` 作为底层基础服务，没有向上依赖（不依赖 core/app/Tauri）。
- 保持 `create_tokio_command` 语义不变，防止 output 任务信号破坏。
- `spawn_child_process_tree_cleanup` 完全未修改（保留给 W2-2 任务）。
- 未修改 `.superpowers/sdd/progress.md`。

---

## 8. 疑虑与后续建议

- 无遗留疑虑。
