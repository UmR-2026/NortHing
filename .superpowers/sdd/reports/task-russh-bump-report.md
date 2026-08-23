# Report: russh 0.45 → 0.62.7 大版本迁移（RUSTSEC-2026-0089 修复）

## 1. 改动清单

- `Cargo.toml`: 升级 `russh` 至 `0.62.7`，升级 `russh-sftp` 至 `2.4.0`，完全移除 `russh-keys = "0.45"`。
- `src/crates/services/services-integrations/Cargo.toml`: 移除 `russh-keys` 依赖及 `remote-ssh-concrete` feature 中的 `russh-keys` 条目。
- `src/crates/services/services-integrations/src/remote_ssh/manager.rs`: `PublicKey` 引用从 `russh_keys::key::PublicKey` 迁移至 `russh::keys::PublicKey`。
- `src/crates/services/services-integrations/src/remote_ssh/manager_handler.rs`: 
  - 移除 `#[async_trait]`，适配 russh 0.62 原生 `async fn in trait` 签名；
  - `PublicKey` 迁移至 `russh::keys::PublicKey`；
  - `check_server_key` 中指纹计算适配 `server_public_key.fingerprint(Default::default()).to_string()`。
- `src/crates/services/services-integrations/src/remote_ssh/mgr_lifecycle_handlers.rs`:
  - 私钥类型从 `russh_keys::key::KeyPair` 迁移至 `russh::keys::PrivateKey`；
  - 密钥解析函数迁移至 `russh::keys::decode_secret_key`；
  - `build_session_client_config()` 中 `preferred.key` 迁移至 `russh::keys::Algorithm` 变体，严格保留原 6 种算法及其优先级；
  - `perform_session_auth` 适配 `AuthResult` 返回类型及 `PrivateKeyWithHashAlg` 包装（集成 `best_supported_rsa_hash` 协商）。
- `Cargo.lock`: 依据上述依赖更新锁定版本。

## 2. 复用侦察（§3）

- **核查符号**：
  - `PublicKey`：`russh::keys::PublicKey`（re-export 自 `ssh_key::PublicKey`）完全覆盖。
  - 私钥类型：`russh::keys::PrivateKey`（re-export 自 `ssh_key::PrivateKey`）完全覆盖原有 `russh_keys::key::KeyPair`。
  - 私钥解码：`russh::keys::decode_secret_key` 原生提供。
  - 算法常量：`russh::keys::Algorithm` 提供 `Ed25519`, `Ecdsa { curve }`, `Rsa { hash }` 变体，配合 `russh::keys::EcdsaCurve` 与 `russh::keys::HashAlg` 覆盖原有所有常量。
- **复用决策**：直接复用 `russh::keys` 模块，完全删除 `russh-keys` 依赖。
- **判断依据**：russh 0.60+ 已经将 `russh-keys` 完全吸收合并入 `russh::keys`，并基于上游 `ssh-key` crate 统一管理公私钥及算法模型，无任何遗漏特性，不需要额外引入或保留 `russh-keys`。

## 3. 兼容缺口

无。原有 6 种 Host Key 算法（`Ed25519`, `ECDSA_SHA2_NISTP256`, `ECDSA_SHA2_NISTP521`, `RSA_SHA2_256`, `RSA_SHA2_512`, `SSH_RSA`）及全部 KEX 算法（`CURVE25519`, `CURVE25519_PRE_RFC_8731`, `DH_G16_SHA512`, `DH_G14_SHA256`, `DH_G14_SHA1`, `DH_G1_SHA1`, `EXTENSION_SUPPORT_AS_CLIENT`, `EXTENSION_OPENSSH_STRICT_KEX_AS_CLIENT`）与优先级均完整保留并成功适配。

## 4. 遇到的编译错误及修复层

- `E0195` (manager_handler.rs:129, 206): 机制层 — russh 0.62.7 的 `Handler` 采用 Rust 原生 `async fn in trait` / `impl Future` 签名，移除 `#[async_trait]` 宏以匹配 trait 方法的生命周期约束。
- `E0308` (mgr_lifecycle_handlers.rs:237): 机制层 — russh 0.62.7 的 `authenticate_password` 返回 `Result<AuthResult, Error>`，通过 `result.success()` 取出认证结果布尔值。
- `E0308` (mgr_lifecycle_handlers.rs:250): 设计层 — russh 0.62.7 将私钥模型由 `russh_keys::key::KeyPair` 迁移为 `russh::keys::PrivateKey`，认证接口升级为 `PrivateKeyWithHashAlg` 以显式支持 RSA 签名哈希协商（结合 `handle.best_supported_rsa_hash()`）。
- `E0308` (mgr_lifecycle_handlers.rs:254, 258): 机制层 — `authenticate_publickey` 返回 `Result<AuthResult, Error>`，通过 `auth_result.success()` 模式匹配处理认证结果。
- `E0271` (mgr_lifecycle_handlers.rs:181): 机制层 — `Preferred::key` 类型从 `Cow<[russh_keys::key::Name]>` 变更为 `Cow<[ssh_key::Algorithm]>`，改用 `russh::keys::Algorithm` 枚举表达算法偏好列表。

## 5. 验证命令及输出原文

### 1. Crate 编译门 (`cargo check -p northhing-services-integrations 2>&1`)

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.63s
```

### 2. 桌面编译门 (`rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing 2>&1`)

```text
warning: private item shadows public glob re-export
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
note: the name `prompt_cache` in the type namespace is supposed to be publicly re-exported here
  --> src\crates\assembly\core\src\agentic\session\mod.rs:34:9
   |
34 | pub use facade::*;
   |         ^^^^^^^^^
note: but the private item here shadows it
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: `#[warn(hidden_glob_reexports)]` on by default

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_loop.rs:295:9
    |
295 |     let mut command_started_after_ms: Option<u64> = None;
    |         ----^^^^^^^^^^^^^^^^^^^^^^^^
    |         |
    |         help: remove this `mut`
    |
    = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_input.rs:191:9
    |
191 |     let mut timeout_seconds = match input.get("timeout_seconds") {
    |         ----^^^^^^^^^^^^^^^
    |         |
    |         help: remove this `mut`

warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:63:13
   |
63 |         let mut turn_id = ctx.final_turn_id.clone();
   |             ----^^^^^^^
   |             |
   |             help: remove this `mut`

warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:35:13
   |
35 |         let mut extra_user_message_metadata = ctx.extra_user_message_metadata.clone();
   |             ----^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |             |
   |             help: remove this `mut`

warning: unused variable: `port`
   --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser.rs:137:13
    |
137 |         let port = params
    |             ^^^^ help: if this is intentional, prefix it with an underscore: `_port`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `actions`
  --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser_telemetry.rs:26:13
   |
26 |         let actions = BrowserActions::new(session.client.as_ref());
   |             ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_actions`

warning: unused variable: `deep_review_subagent_role`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:80:5
   |
80 |     deep_review_subagent_role: Option<crate::agentic::deep_review_policy::DeepReviewSubagentRole>,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_deep_review_subagent_role`

warning: unused variable: `is_retry`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:84:5
   |
84 |     is_retry: bool,
   |     ^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_is_retry`

warning: unused variable: `suppress_session_title_generation`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs:34:13
   |
34 |         let suppress_session_title_generation = ctx.suppress_session_title_generation;
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_suppress_session_title_generation`

warning: unused variable: `turn_index`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:39:13
   |
39 |         let turn_index = ctx.turn_index;
   |             ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_turn_index`

warning: unused variable: `workspace_turn_status`
   --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:372:17
    |
372 |             let workspace_turn_status = tokio::select! {
    |                 ^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_workspace_turn_status`

warning: unused variable: `active_counter`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:67:13
   |
67 |         let active_counter = Arc::new(AtomicUsize::new(0));
   |             ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_active_counter`

warning: unused variable: `ws`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:236:36
    |
236 |         let mut stmt = if let Some(ws) = workspace_key {
    |                                    ^^ help: if this is intentional, prefix it with an underscore: `_ws`

warning: unused variable: `last_mentioned_at`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:291:80
    |
291 |             let (id, text, scope, confidence, session_id, turn_id, created_at, last_mentioned_at, fact_type) =
    |                                                                                ^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_last_mentioned_at`

warning: unused variable: `at_ms`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:743:85
    |
743 |     pub(crate) fn supersede_fact(&self, fact_id: &str, superseded_by: Option<&str>, at_ms: u64) -> NortHingResult<()> {
    |                                                                                     ^^^^^ help: if this is intentional, prefix it with an underscore: `_at_ms`

warning: unused variable: `ws`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db\dream.rs:17:36
   |
17 |         let mut stmt = if let Some(ws) = workspace_key {
   |                                    ^^ help: if this is intentional, prefix it with an underscore: `_ws`

warning: unused variable: `params`
   --> src\crates\assembly\core\src\service\mcp\server\manager\interaction.rs:104:9
    |
104 |         params: Option<Value>,
    |         ^^^^^^ help: if this is intentional, prefix it with an underscore: `_params`

warning: `northhing-core` (lib) generated 18 warnings (run `cargo fix --lib -p northhing-core` to apply 17 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.28s
```

### 3. MSVC 实跑 remote_ssh 测试 (`rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-services-integrations --all-features remote_ssh 2>&1`)

```text
warning: unused import: `serde_json::json`
 --> src\crates\services\services-integrations\tests\dynamic_tools_and_runtime.rs:7:5
  |
7 | use serde_json::json;
  |     ^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `serde_json::json`
 --> src\crates\services\services-integrations\tests\request_builders_and_adapters.rs:7:5
  |
7 | use serde_json::json;
  |     ^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `serde_json::json`
 --> src\crates\services\services-integrations\tests\tool_names_and_protocol.rs:7:5
  |
7 | use serde_json::json;
  |     ^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `PathBuf`
 --> src\crates\services\services-integrations\tests\function_agent_contracts.rs:6:23
  |
6 | use std::path::{Path, PathBuf};
  |                       ^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `serde_json::json`
 --> src\crates\services\services-integrations\tests\context_enhancer_and_catalog.rs:7:5
  |
7 | use serde_json::json;
  |     ^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: `northhing-services-integrations` (test "dynamic_tools_and_runtime") generated 1 warning (run `cargo fix --test "dynamic_tools_and_runtime" -p northhing-services-integrations` to apply 1 suggestion)
warning: `northhing-services-integrations` (test "request_builders_and_adapters") generated 1 warning (run `cargo fix --test "request_builders_and_adapters" -p northhing-services-integrations` to apply 1 suggestion)
warning: `northhing-services-integrations` (test "tool_names_and_protocol") generated 1 warning (run `cargo fix --test "tool_names_and_protocol" -p northhing-services-integrations` to apply 1 suggestion)
warning: `northhing-services-integrations` (test "function_agent_contracts") generated 1 warning (run `cargo fix --test "function_agent_contracts" -p northhing-services-integrations` to apply 1 suggestion)
warning: `northhing-services-integrations` (test "context_enhancer_and_catalog") generated 1 warning (run `cargo fix --test "context_enhancer_and_catalog" -p northhing-services-integrations` to apply 1 suggestion)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.49s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_integrations-762801d1916f277d.exe)

running 29 tests
test remote_ssh::manager_tests::tests::mkdir_all_prefixes_collapse_redundant_separators ... ok
test remote_ssh::manager_tests::tests::mkdir_all_prefixes_expand_absolute_posix_path ... ok
test remote_ssh::remote_exec::output::tests::remote_exec_session_ids_match_local_test_baseline ... ok
test remote_ssh::remote_exec::output::tests::head_tail_text_keeps_full_output_when_unbounded ... ok
test remote_ssh::workspace_search::service_helpers::tests::remote_search_cache_keys_normalize_workspace_root ... ok
test remote_ssh::workspace_search::tests::preserves_supported_linux_flashgrep_bundle_order ... ok
test remote_ssh::workspace_search::tests::remote_scan_fallback_retry_policy_preserves_current_contract ... ok
test remote_ssh::workspace_search::tests::remote_workspace_search_bundle_rejects_unsupported_linux_arch ... ok
test remote_ssh::workspace_search::tests::remote_workspace_search_paths_preserve_current_contract ... ok
test remote_ssh::workspace_search::tests::remote_workspace_search_scope_preserves_current_contract ... ok
test remote_ssh::workspace_search::tests::remote_workspace_search_mode_preserves_current_contract ... ok
test remote_ssh::workspace_search::tests::remote_workspace_search_probe_parsers_preserve_current_contract ... ok
test remote_ssh::password_vault::tests::migrate_fails_closed_on_corrupted_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::remove_fails_closed_on_truncated_vault_without_touching_file ... ok
test remote_ssh::manager_tests::tests::rejects_saving_password_connection_without_password ... ok
test remote_ssh::password_vault::tests::remove_fails_closed_on_corrupted_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::migrate_fails_closed_on_truncated_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::store_fails_closed_on_corrupted_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::store_fails_closed_on_truncated_vault_without_touching_file ... ok
test remote_ssh::manager_tests::tests::prunes_password_connection_without_vault_entry ... ok
test remote_ssh::manager_tests::tests::prunes_remote_workspaces_without_saved_connection ... ok
test remote_ssh::password_vault::tests::load_returns_error_on_corrupted_vault ... ok
test remote_ssh::manager_tests::tests::restores_connection_config_from_saved_password_profile ... ok
test remote_ssh::password_vault::tests::vault_remove_deletes_file_when_last_entry_is_removed ... ok
test remote_ssh::password_vault::tests::migrate_entry_moves_password_to_new_connection_id ... ok
test remote_ssh::password_vault::tests::vault_store_is_atomic_and_keeps_bak_of_previous_content ... ok
test remote_ssh::workspace_search::service_helpers::tests::remote_search_rejects_non_linux_before_stdio_open ... ok
test remote_ssh::workspace_search::service_helpers::tests::remote_search_context_ignores_stale_cache_before_resolving_connection ... ok
test remote_ssh::workspace_search::service_helpers::tests::remote_search_open_guard_is_removed_when_stdio_spawn_fails ... ok

test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 18 filtered out; finished in 0.38s

     Running tests\announcement_contracts.rs (target\debug\deps\announcement_contracts-1eaad2d7ec740da5.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s

     Running tests\config_and_server_lifecycle.rs (target\debug\deps\config_and_server_lifecycle-098c1c8c330acada.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 18 filtered out; finished in 0.00s

     Running tests\context_enhancer_and_catalog.rs (target\debug\deps\context_enhancer_and_catalog-53175886cd22b9aa.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\dynamic_tools_and_runtime.rs (target\debug\deps\dynamic_tools_and_runtime-1fa2759a71f320ef.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.00s

     Running tests\file_watch_contracts.rs (target\debug\deps\file_watch_contracts-49ed354474f96a88.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s

     Running tests\function_agent_contracts.rs (target\debug\deps\function_agent_contracts-364243c235b31d4c.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\git_contracts.rs (target\debug\deps\git_contracts-9f91bb2f545fc77c.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s

     Running tests\remote_ssh_contracts.rs (target\debug\deps\remote_ssh_contracts-5f2f3f39e3e1d90d.exe)

running 1 test
test remote_ssh_legacy_agent_auth_maps_to_default_private_key ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s

     Running tests\request_builders_and_adapters.rs (target\debug\deps\request_builders_and_adapters-8954274577489e4d.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s

     Running tests\tool_names_and_protocol.rs (target\debug\deps\tool_names_and_protocol-9235b132755b8b85.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\workspace_search_contracts.rs (target\debug\deps\workspace_search_contracts-5b8a30e015c9c19b.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
```

### 4. Audit 复验 (`cargo audit 2>&1 | Select-String -Pattern "russh" -Context 3,6`)

```text
(grep 为空，即 russh / russh-sftp / russh-keys 无任何漏洞条目)
```

## 6. 状态

DONE
