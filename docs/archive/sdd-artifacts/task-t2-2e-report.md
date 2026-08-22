# Task Report T2-2e — remote 栈子批 C3：services-integrations remote_connect 整删

## Status
DONE

## 逐文件操作清单

1. **整删源码目录** `src/crates/services/services-integrations/src/remote_connect/`（14 个文件，4,081 行生产代码全部删除）：
   - `device.rs`
   - `encryption.rs`
   - `mod.rs`
   - `pairing.rs`
   - `qr_generator.rs`
   - `relay_client.rs`
   - `remote_cancel_handlers.rs`
   - `remote_dialog_handlers.rs`
   - `remote_file_io.rs`
   - `remote_request_builders.rs`
   - `remote_session_handlers.rs`
   - `remote_session_response_builders.rs`
   - `remote_session_state.rs`
   - `remote_workspace_resolver.rs`

2. **模块注册移除** `src/crates/services/services-integrations/src/lib.rs`：
   - 移除 `#[cfg(feature = "remote-connect")] pub mod remote_connect;`

3. **依赖与 Feature 清理** `src/crates/services/services-integrations/Cargo.toml`：
   - 移除 10 个孤立依赖：`hostname`, `image`, `mac_address`, `qrcode`, `rustls`, `rustls-native-certs`, `schannel`, `tokio-tungstenite`, `urlencoding`, `x25519-dalek`
   - 移除 `remote-connect = [...]` feature 块（22 行）
   - 从 `product-full` feature 列表中移除 `"remote-connect"`
   - 保留共享依赖与 `remote-ssh` / `remote-ssh-concrete` 等所有其它 feature

4. **测试文件清理** `src/crates/services/services-integrations/tests/`：
   - 整文件删除 7 个纯 `remote_connect` 测试文件：
     - `tests/pairing_qr_relay.rs`（58 行，2 个测试）
     - `tests/command_runtime.rs`（119 行，2 个测试）
     - `tests/dialog_cancel_contracts.rs`（290 行，8 个测试）
     - `tests/file_transfer.rs`（186 行，7 个测试）
     - `tests/model_catalog_tracker_poll.rs`（449 行，10 个测试）
     - `tests/session_wire_and_responses.rs`（424 行，9 个测试）
     - `tests/submission_images.rs`（189 行，9 个测试）
   - 手术修改 `tests/common/mod.rs`：
     - 移除 `pub use northhing_services_integrations::remote_connect::{...}` 整块 re-export（32 行）
     - 移除文件内所有 remote 专属辅助结构与实现（`TestImageContext`, `remote_history_contract_turn`, `RecordingDialogHost`, `RecordingCancelHost`, `remote_state`, `RecordingCommandHost`, `make_temp_remote_workspace`, `RecordingFileHost`, `sample_remote_model_catalog`, `RecordingTrackerHost`）
     - 保留 MCP 共享测试 helper 与类型 re-export（117 行）
   - `tests/remote_ssh_contracts.rs`：100% 原样保留（7 个测试函数全绿）

5. **Boundary 规则同步** `scripts/core-boundaries/`：
   - `rules/feature-rules.mjs`：更新 `services-integrations` 的 `optionalDependencyFeatureOwnerRules`，从共享依赖所有者中移除 `'remote-connect'`，将 `northhing-runtime-ports` 所有者对齐为 `['deep-research']`，删除 10 个孤立依赖条目；从 `ownerCrateFeatureAssemblyRules` 中移除 `'remote-connect'`
   - `rules/crate-rules.mjs`：从 `services-integrations` 的 `forbiddenNonOptionalDeps` 列表中移除已彻底删除的 `'tokio-tungstenite'`
   - `rules/source/required-rules.mjs`：删除 `services-integrations/src/remote_connect/*` 及对应 7 个测试文件的 required rules 规则块（共 646 行）
   - `self-test.mjs`：更新 `servicesOptionalOwnerRule` 校验列表；整段删除 `src/crates/services/services-integrations/src/remote_connect.rs` fixture 测试块（85 行）

6. **文档同步**：
   - `src/crates/services/services-integrations/AGENTS.md`：移除 Remote-connect 条目
   - `src/crates/services/AGENTS.md` & `AGENTS-CN.md`：更新表格中 `services-integrations` 职责描述，移除 Remote Connect 提及

7. **Cargo.lock 自动同步**：
   - 孤立依赖 `hostname`, `qrcode`, `x25519-dalek`, `zeroize_derive` 从 lock 文件中彻底移除，`services-integrations` 依赖列表中去除全部 10 个孤立依赖

---

## 每个 orphan dep 的处置与依据

| 依赖名称 | 处置 | 依据与核实事实 |
|---|---|---|
| `hostname` | **删除** | 唯一 owner 为 `remote-connect`，经 `rg` 确认 crate 内无其它引用，`Cargo.lock` 已彻底剔除 |
| `image` | **删除** | 唯一 owner 为 `remote-connect`（`qr_generator.rs`），经 `rg` 确认 crate 内无其它引用 |
| `mac_address` | **删除** | 唯一 owner 为 `remote-connect`（`device.rs`），经 `rg` 确认 crate 内无其它引用 |
| `qrcode` | **删除** | 唯一 owner 为 `remote-connect`（`qr_generator.rs`），经 `rg` 确认 crate 内无其它引用，`Cargo.lock` 已彻底剔除 |
| `rustls` | **删除** | 唯一 owner 为 `remote-connect`（`relay_client.rs`），经 `rg` 确认 crate 内无其它引用 |
| `rustls-native-certs` | **删除** | 唯一 owner 为 `remote-connect`（`relay_client.rs`），经 `rg` 确认 crate 内无其它引用 |
| `schannel` | **删除** | 唯一 owner 为 `remote-connect`（`relay_client.rs` Windows 分支），经 `rg` 确认 crate 内无其它引用 |
| `tokio-tungstenite` | **删除** | 唯一 owner 为 `remote-connect`（`relay_client.rs`），经 `rg` 确认 crate 内无其它引用 |
| `urlencoding` | **删除** | 唯一 owner 为 `remote-connect`（`qr_generator.rs`），经 `rg` 确认 crate 内无其它引用 |
| `x25519-dalek` | **删除** | 唯一 owner 为 `remote-connect`（`encryption.rs`），经 `rg` 确认 crate 内无其它引用，`Cargo.lock` 已彻底剔除 |
| `northhing-runtime-ports` | **保留** (optional) | `deep-research` feature 引用，`deep_research.rs` 中使用 `northhing_runtime_ports::deep_research::*` |
| `aes-gcm` | **保留** (optional) | 共享 owner：`mcp`, `remote-ssh-concrete` |
| `anyhow` | **保留** (optional) | 共享 owner：`mcp`, `remote-ssh-concrete` |
| `base64` | **保留** (optional) | 共享 owner：`mcp`, `miniapp-runtime`, `remote-ssh-concrete` |
| `chrono` | **保留** (optional) | 共享 owner：`git`, `remote-ssh-concrete` |
| `futures` | **保留** (optional) | 共享 owner：`mcp` |
| `rand` | **保留** (optional) | 共享 owner：`mcp`, `remote-ssh-concrete` |
| `sha2` | **保留** (optional) | 共享 owner：`remote-ssh` |
| `tokio-util` | **保留** (optional) | 共享 owner：`remote-ssh` |
| `uuid` | **保留** (optional) | 共享 owner：`miniapp-runtime`, `remote-ssh-concrete` |

---

## 测试函数删/留判定表

| 测试文件 | 测试函数名 | 判定 | 依据 |
|---|---|---|---|
| `tests/pairing_qr_relay.rs` | `remote_connect_pairing_primitives_live_in_services_owner` | **删除** | 测试 `remote_connect` pairing 协议 |
| `tests/pairing_qr_relay.rs` | `remote_connect_qr_and_relay_primitives_live_in_services_owner` | **删除** | 测试 `remote_connect` QR 生成与 Relay 消息 |
| `tests/command_runtime.rs` | `remote_connect_command_owner_routes_send_message_and_prefers_explicit_images` | **删除** | 测试 `remote_connect::handle_remote_command` |
| `tests/command_runtime.rs` | `remote_connect_command_owner_preserves_cancel_and_group_routing` | **删除** | 测试 `remote_connect::handle_remote_command` |
| `tests/dialog_cancel_contracts.rs` | `remote_connect_cancel_and_restore_policy_preserve_runtime_decisions` | **删除** | 测试 `remote_connect` 取消/恢复策略 |
| `tests/dialog_cancel_contracts.rs` | `remote_connect_dialog_runtime_owns_restore_prewarm_and_submit_order` | **删除** | 测试 `remote_connect::submit_remote_dialog` |
| `tests/dialog_cancel_contracts.rs` | `remote_connect_dialog_runtime_preserves_explicit_turn_without_restore` | **删除** | 测试 `remote_connect::submit_remote_dialog` |
| `tests/dialog_cancel_contracts.rs` | `remote_connect_dialog_submit_outcome_builder_preserves_scheduler_shape` | **删除** | 测试 `remote_connect` 对话输出构造 |
| `tests/dialog_cancel_contracts.rs` | `remote_connect_dialog_runtime_keeps_legacy_restore_failure_tolerance` | **删除** | 测试 `remote_connect` 恢复容错 |
| `tests/dialog_cancel_contracts.rs` | `remote_connect_cancel_runtime_restores_missing_session_before_cancel` | **删除** | 测试 `remote_connect::cancel_remote_task` |
| `tests/dialog_cancel_contracts.rs` | `remote_connect_cancel_runtime_preserves_stale_and_idle_errors_without_restore` | **删除** | 测试 `remote_connect::cancel_remote_task` |
| `tests/dialog_cancel_contracts.rs` | `remote_connect_cancel_runtime_preserves_restore_failure_error` | **删除** | 测试 `remote_connect::cancel_remote_task` |
| `tests/file_transfer.rs` | `remote_connect_file_transfer_policy_preserves_limits_and_chunk_ranges` | **删除** | 测试 `remote_connect` 文件分块策略 |
| `tests/file_transfer.rs` | `remote_connect_file_transfer_policy_preserves_name_fallback` | **删除** | 测试 `remote_connect` 文件名显示 |
| `tests/file_transfer.rs` | `remote_connect_file_path_resolution_stays_within_workspace_root` | **删除** | 测试 `remote_connect` 路径解析 |
| `tests/file_transfer.rs` | `remote_connect_file_read_helpers_preserve_current_wire_inputs` | **删除** | 测试 `remote_connect` 文件读取 helper |
| `tests/file_transfer.rs` | `remote_connect_file_chunk_and_info_helpers_preserve_response_facts` | **删除** | 测试 `remote_connect` 文件分块与信息 |
| `tests/file_transfer.rs` | `remote_connect_file_response_assembly_owns_base64_wire_shape` | **删除** | 测试 `remote_connect` 文件响应组装 |
| `tests/file_transfer.rs` | `remote_connect_file_command_handler_owns_owner_flow_and_uses_host_root` | **删除** | 测试 `remote_connect` 文件命令处理 |
| `tests/model_catalog_tracker_poll.rs` | `remote_connect_model_catalog_builder_preserves_config_shape` | **删除** | 测试 `remote_connect` 模型目录构建 |
| `tests/model_catalog_tracker_poll.rs` | `remote_connect_tracker_registry_owns_lifecycle_without_core_state` | **删除** | 测试 `remote_connect` tracker 注册表 |
| `tests/model_catalog_tracker_poll.rs` | `remote_connect_tracker_preserves_streaming_snapshot_contract` | **删除** | 测试 `remote_connect` tracker 流式快照 |
| `tests/model_catalog_tracker_poll.rs` | `remote_connect_tracker_keeps_subagent_items_out_of_parent_accumulators` | **删除** | 测试 `remote_connect` tracker 子代理隔离 |
| `tests/model_catalog_tracker_poll.rs` | `remote_connect_tracker_broadcasts_tool_and_turn_events` | **删除** | 测试 `remote_connect` tracker 事件广播 |
| `tests/model_catalog_tracker_poll.rs` | `remote_connect_tracker_keeps_finished_turn_snapshot_until_persistence_finalizes` | **删除** | 测试 `remote_connect` tracker 持久化状态 |
| `tests/model_catalog_tracker_poll.rs` | `remote_connect_model_catalog_delta_preserves_poll_invalidation_policy` | **删除** | 测试 `remote_connect` 模型轮询 delta |
| `tests/model_catalog_tracker_poll.rs` | `remote_connect_model_selection_policy_owns_alias_and_config_reference_rules` | **删除** | 测试 `remote_connect` 模型选择规则 |
| `tests/model_catalog_tracker_poll.rs` | `remote_connect_poll_helpers_preserve_delta_and_completion_policy` | **删除** | 测试 `remote_connect` 轮询 helper |
| `tests/model_catalog_tracker_poll.rs` | `remote_connect_tracker_ignores_unrelated_direct_session_events` | **删除** | 测试 `remote_connect` tracker 事件过滤 |
| `tests/model_catalog_tracker_poll.rs` | `remote_connect_tool_preview_slimming_keeps_short_fields_and_drops_large_strings` | **删除** | 测试 `remote_connect` 工具预览精简 |
| `tests/session_wire_and_responses.rs` | `remote_connect_execution_response_helpers_preserve_wire_shape` | **删除** | 测试 `remote_connect` 执行响应格式 |
| `tests/session_wire_and_responses.rs` | `remote_connect_workspace_response_helpers_own_wire_shape` | **删除** | 测试 `remote_connect` 工作区响应格式 |
| `tests/session_wire_and_responses.rs` | `remote_connect_session_response_helpers_own_pagination_and_timestamps` | **删除** | 测试 `remote_connect` 会话响应格式 |
| `tests/session_wire_and_responses.rs` | `remote_connect_session_create_contract_preserves_workspace_binding` | **删除** | 测试 `remote_connect` 会话创建请求 |
| `tests/session_wire_and_responses.rs` | `remote_connect_agent_type_mapping_preserves_current_mobile_aliases` | **删除** | 测试 `remote_connect` 移动端别名映射 |
| `tests/session_wire_and_responses.rs` | `remote_connect_message_dtos_keep_current_wire_shape` | **删除** | 测试 `remote_connect` 消息 DTO wire shape |
| `tests/session_wire_and_responses.rs` | `remote_connect_command_wire_shape_lives_in_owner_contract` | **删除** | 测试 `remote_connect` 命令 wire shape |
| `tests/session_wire_and_responses.rs` | `remote_connect_response_wire_shape_lives_in_owner_contract` | **删除** | 测试 `remote_connect` 响应 wire shape |
| `tests/submission_images.rs` | `remote_connect_submission_contract_preserves_relay_source_and_turn_id` | **删除** | 测试 `remote_connect` 提交请求 |
| `tests/submission_images.rs` | `remote_connect_submission_contract_preserves_bot_source` | **删除** | 测试 `remote_connect` 提交请求 |
| `tests/submission_images.rs` | `remote_connect_image_attachment_contract_preserves_portable_metadata` | **删除** | 测试 `remote_connect` 图片附件 |
| `tests/submission_images.rs` | `remote_connect_image_submission_request_preserves_existing_source_and_turn_shape` | **删除** | 测试 `remote_connect` 图片提交 |
| `tests/submission_images.rs` | `remote_connect_image_context_policy_preserves_legacy_fallback_shape` | **删除** | 测试 `remote_connect` 图片上下文策略 |
| `tests/submission_images.rs` | `remote_connect_image_context_policy_prefers_explicit_contexts` | **删除** | 测试 `remote_connect` 图片上下文偏好 |
| `tests/submission_images.rs` | `remote_connect_image_context_adapter_owns_portable_conversion_shape` | **删除** | 测试 `remote_connect` 图片适配器 |
| `tests/submission_images.rs` | `remote_chat_history_assembly_preserves_message_shape_and_item_order` | **删除** | 测试 `remote_connect` 聊天历史组装 |
| `tests/submission_images.rs` | `remote_chat_history_assembly_skips_in_progress_assistant_history` | **删除** | 测试 `remote_connect` 进行中历史过滤 |
| `tests/common/mod.rs` | `remote_connect_image_context_adapter_owns_portable_conversion_shape` | **删除** | 测试 `remote_connect` 图片适配器 |
| `tests/remote_ssh_contracts.rs` | `remote_ssh_legacy_agent_auth_maps_to_default_private_key` | **保留** | SSH 认证契约测试，与 remote_connect 无关 |
| `tests/remote_ssh_contracts.rs` | `remote_workspace_defaults_keep_older_files_loadable` | **保留** | 远程 SSH 工作区默认值契约 |
| `tests/remote_ssh_contracts.rs` | `remote_workspace_path_helpers_preserve_current_identity_contract` | **保留** | 远程 SSH 路径与标识 helper 契约 |
| `tests/remote_ssh_contracts.rs` | `remote_workspace_session_paths_use_supplied_mirror_root` | **保留** | 远程 SSH 会话镜像路径契约 |
| `tests/remote_ssh_contracts.rs` | `local_workspace_identity_helpers_preserve_canonical_root_contract` | **保留** | 本地工作区规范根契约 |
| `tests/remote_ssh_contracts.rs` | `remote_workspace_registry_preserves_ambiguous_root_resolution_contract` | **保留** | 远程 SSH 工作区注册表解析契约 |
| `tests/remote_ssh_contracts.rs` | `remote_workspace_registry_preserves_legacy_state_and_clear_contract` | **保留** | 远程 SSH 工作区注册表状态管理契约 |

---

## 验证原始输出

### 1. 主门验证

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
**输出**：
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.55s
```

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
**输出**：
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.31s
```

### 2. SSH feature 自足性与 Default Features 验证

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-services-integrations --features remote-ssh,remote-ssh-concrete
```
**输出**：
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.96s
```

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-services-integrations
```
**输出**：
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.61s
```

### 3. Boundary 规则验证

```powershell
node scripts/check-core-boundaries.mjs
node scripts/core-boundaries/self-test.mjs
```
**输出**：
```text
Core boundary check passed.
```

### 4. 测试验证（services-integrations product-full 全绿）

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-services-integrations --features product-full
```
**输出**：
```text
     Running unittests src\lib.rs (target\debug\deps\northhing_services_integrations-60a021f502bab8b0.exe)

running 76 tests
test mcp::tool_name::tests::build_mcp_tool_name_normalizes_both_segments ... ok
test mcp::tool_name::tests::normalize_name_for_mcp_keeps_ascii_word_chars_and_hyphen ... ok
test mcp::tool_name::tests::normalize_name_for_mcp_replaces_spaces_and_symbols ... ok
test miniapp::host_dispatch::tests::command_basename_allows_windows_git_executable_paths ... ok
test miniapp::host_dispatch::tests::host_shell_exec_rejects_string_mode_with_newline_injection ... ok
test miniapp::host_dispatch::tests::host_shell_exec_rejects_string_mode_with_crlf_injection ... ok
test miniapp::host_dispatch::tests::host_shell_exec_rejects_string_mode_with_shell_metacharacters ... ok
test miniapp::storage_tests::tests::load_source_from_dirs_missing_index_html_returns_not_found ... ok
test mcp::auth::tests::clear_fails_closed_on_corrupted_vault_without_touching_file ... ok
test mcp::auth::tests::clear_fails_closed_on_truncated_vault_without_touching_file ... ok
test miniapp::builtin_io::tests::marker_io_ignores_invalid_marker_and_round_trips_valid_marker ... ok
test remote_ssh::manager_tests::tests::mkdir_all_prefixes_collapse_redundant_separators ... ok
test remote_ssh::manager_tests::tests::mkdir_all_prefixes_expand_absolute_posix_path ... ok
test miniapp::storage_tests::tests::load_source_from_dirs_missing_optional_files_stay_empty ... ok
test miniapp::storage_tests::tests::load_source_from_dirs_unreadable_index_html_returns_io ... ok
test miniapp::storage_tests::tests::load_source_from_dirs_unreadable_esm_deps_returns_io ... ok
test mcp::auth::tests::store_fails_closed_on_corrupted_vault_without_touching_file ... ok
test mcp::auth::tests::store_fails_closed_on_truncated_vault_without_touching_file ... ok
test miniapp::storage_tests::tests::load_source_from_dirs_unreadable_optional_file_returns_io ... ok
test remote_ssh::manager_tests::tests::rejects_saving_password_connection_without_password ... ok
test miniapp::storage_tests::tests::load_source_from_dirs_corrupt_esm_deps_returns_parse ... ok
test miniapp::storage_tests::tests::load_source_from_dirs_real_empty_files_stay_empty ... ok
test deep_research::tests::run_for_session_is_no_op_when_session_has_no_report ... ok
test miniapp::storage_tests::tests::draft_reads_skip_marked_active_root ... ok
test mcp::auth::tests::load_returns_error_on_corrupted_vault ... ok
test remote_ssh::password_vault::tests::migrate_fails_closed_on_corrupted_vault_without_touching_file ... ok
test remote_ssh::manager_tests::tests::prunes_password_connection_without_vault_entry ... ok
test remote_ssh::remote_exec::output::tests::head_tail_text_keeps_full_output_when_unbounded ... ok
test remote_ssh::remote_exec::output::tests::remote_exec_session_ids_match_local_test_baseline ... ok
test miniapp::storage_tests::tests::load_source_from_dirs_loads_all_present_files ... ok
test remote_ssh::workspace_search::service_helpers::tests::remote_search_cache_keys_normalize_workspace_root ... ok
test remote_ssh::workspace_search::service_helpers::tests::remote_search_context_ignores_stale_cache_before_resolving_connection ... ok
test remote_ssh::manager_tests::tests::prunes_remote_workspaces_without_saved_connection ... ok
test remote_ssh::workspace_search::tests::preserves_supported_linux_flashgrep_bundle_order ... ok
test remote_ssh::workspace_search::tests::remote_scan_fallback_retry_policy_preserves_current_contract ... ok
test remote_ssh::workspace_search::tests::remote_workspace_search_bundle_rejects_unsupported_linux_arch ... ok
test remote_ssh::workspace_search::tests::remote_workspace_search_mode_preserves_current_contract ... ok
test remote_ssh::workspace_search::tests::remote_workspace_search_paths_preserve_current_contract ... ok
test remote_ssh::workspace_search::tests::remote_workspace_search_probe_parsers_preserve_current_contract ... ok
test remote_ssh::workspace_search::tests::remote_workspace_search_scope_preserves_current_contract ... ok
test workspace_search::flashgrep::rpc_client::tests::drains_remote_stdio_content_length_messages ... ok
test workspace_search::flashgrep::rpc_client::tests::drains_remote_stdio_initialize_response_with_legacy_search_modes ... ok
test workspace_search::service::tests::content_search_converts_legacy_line_matches ... ok
test workspace_search::service::tests::content_search_output_modes_use_current_flashgrep_protocol_modes ... ok
test remote_ssh::password_vault::tests::migrate_fails_closed_on_truncated_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::remove_fails_closed_on_truncated_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::remove_fails_closed_on_corrupted_vault_without_touching_file ... ok
test deep_research::tests::end_to_end_renumbers_report_and_writes_sidecar ... ok
test mcp::auth::tests::vault_clear_deletes_file_when_last_entry_is_cleared ... ok
test miniapp::worker_pool::tests::runtime_port_adapter_preserves_existing_runtime_and_noop_install ... ok
test remote_ssh::password_vault::tests::store_fails_closed_on_corrupted_vault_without_touching_file ... ok
test miniapp::storage_tests::tests::customization_metadata_roundtrips ... ok
test remote_ssh::password_vault::tests::store_fails_closed_on_truncated_vault_without_touching_file ... ok
test deep_research::tests::run_for_session_renumbers_when_report_present ... ok
test miniapp::builtin_io::tests::prepare_builtin_seed_bundle_files_preserves_existing_storage ... ok
test miniapp::storage_tests::tests::saving_new_draft_isolates_marked_active_root_first ... ok
test mcp::auth::tests::vault_store_is_atomic_and_keeps_bak_of_previous_content ... ok
test remote_ssh::password_vault::tests::load_returns_error_on_corrupted_vault ... ok
test miniapp::storage_tests::tests::storage_adapter_uses_product_domain_layout_contract ... ok
test remote_ssh::manager_tests::tests::restores_connection_config_from_saved_password_profile ... ok
test miniapp::storage_tests::tests::mark_stale_drafts_moves_sandboxes_off_the_active_read_path ... ok
test miniapp::worker_pool::tests::install_deps_in_dir_noops_without_package_json ... ok
test miniapp::storage_tests::tests::saving_app_files_preserves_existing_storage_json ... ok
test remote_ssh::password_vault::tests::migrate_entry_moves_password_to_new_connection_id ... ok
test miniapp::storage_tests::tests::draft_storage_is_hidden_and_isolated_from_active_storage ... ok
test remote_ssh::password_vault::tests::vault_remove_deletes_file_when_last_entry_is_removed ... ok
test workspace_search::service_session::tests::schedule_repo_release_for_test_releases_idle_session ... ok
test remote_ssh::password_vault::tests::vault_store_is_atomic_and_keeps_bak_of_previous_content ... ok
test miniapp::storage_tests::tests::storage_port_adapter_preserves_existing_file_lifecycle ... ok
test miniapp::storage_tests::tests::import_bundle_io_preserves_copy_and_fallback_contract ... ok
test miniapp::storage_tests::tests::cleanup_marked_drafts_removes_quarantined_sandboxes_later ... ok
test miniapp::host_dispatch::tests::host_shell_exec_runs_git_with_workspace_cwd ... ok
test miniapp::host_dispatch::tests::host_shell_exec_allows_args_array_with_newline_in_arg ... ok
test miniapp::host_dispatch::tests::host_shell_exec_allows_clean_string_mode_command ... ok
test remote_ssh::workspace_search::service_helpers::tests::remote_search_rejects_non_linux_before_stdio_open ... ok
test remote_ssh::workspace_search::service_helpers::tests::remote_search_open_guard_is_removed_when_stdio_spawn_fails ... ok

test result: ok. 76 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s

     Running tests\announcement_contracts.rs (target\debug\deps\announcement_contracts-2cd74e7a51a555c7.exe)

running 4 tests
test announcement_state_and_trigger_defaults_preserve_runtime_assumptions ... ok
test announcement_modal_serialization_preserves_snake_case_contract ... ok
test announcement_card_deserialization_preserves_default_contract ... ok
test announcement_state_store_round_trips_state_and_defaults_missing_file ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests\config_and_server_lifecycle.rs (target\debug\deps\config_and_server_lifecycle-dec544e871e1942f.exe)

running 18 tests
test mcp_config_location_preserves_kebab_case_wire_contract ... ok
test mcp_config_authorization_helpers_preserve_header_precedence_and_normalization ... ok
test mcp_config_service_delete_user_fails_closed_on_config_store_read_error ... ok
test mcp_config_service_keeps_load_failures_as_empty_baseline ... ok
test mcp_config_service_save_project_fails_closed_on_config_store_read_error ... ok
test mcp_config_merge_helpers_preserve_precedence_and_dedup_contract ... ok
test mcp_config_service_save_user_fails_closed_on_unrecognized_existing_format ... ok
test mcp_config_service_save_project_fails_closed_on_unrecognized_existing_format ... ok
test mcp_config_service_delete_user_fails_closed_on_unrecognized_existing_format ... ok
test mcp_server_type_and_status_preserve_lowercase_wire_contract ... ok
test mcp_json_config_helpers_preserve_load_format_and_save_validation_contract ... ok
test mcp_config_service_orchestration_preserves_load_save_delete_contract ... ok
test mcp_config_service_save_project_preserves_upsert_contract ... ok
test mcp_config_service_save_user_fails_closed_on_config_store_read_error ... ok
test mcp_config_service_concurrent_user_saves_do_not_lose_entries ... ok
test mcp_server_process_owner_preserves_unsupported_remote_transport_contract ... ok
test mcp_config_service_concurrent_user_save_and_delete_stay_consistent ... ok
test mcp_config_service_concurrent_project_saves_do_not_lose_entries ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\context_enhancer_and_catalog.rs (target\debug\deps\context_enhancer_and_catalog-c6d0ace0df74f0ec.exe)

running 3 tests
test mcp_context_enhancer_preserves_resource_selection_contract ... ok
test mcp_catalog_cache_preserves_resource_prompt_lifecycle_contract ... ok
test mcp_catalog_cache_replacement_invalidates_stale_entries ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\dynamic_tools_and_runtime.rs (target\debug\deps\dynamic_tools_and_runtime-cc646cb069c8ce5d.exe)

running 9 tests
test mcp_dynamic_tool_descriptor_and_result_rendering_preserve_tool_contract ... ok
test mcp_cursor_format_helpers_preserve_cursor_compatibility_contract ... ok
test mcp_oauth_session_snapshot_preserves_camel_case_status_contract ... ok
test mcp_runtime_auth_error_classifier_preserves_process_status_contract ... ok
test mcp_runtime_notification_and_backoff_helpers_preserve_manager_contract ... ok
test mcp_server_config_preserves_transport_defaults_and_validation_contract ... ok
test mcp_runtime_remote_header_merge_preserves_legacy_env_authorization_fallback ... ok
test mcp_dynamic_tool_provider_preserves_manifest_order_and_metadata_snapshot ... ok
test mcp_dynamic_tool_provider_preserves_manifest_contract ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\file_watch_contracts.rs (target\debug\deps\file_watch_contracts-74f7ced378dbbe34.exe)

running 2 tests
test file_watch_event_kind_serializes_snake_case ... ok
test file_watch_preserves_missing_path_error ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\function_agent_contracts.rs (target\debug\deps\function_agent_contracts-600d7291a6e848d0.exe)

running 3 tests
test git_service_time_snapshot_uses_last_commit_timestamp ... ok
test git_service_builds_commit_snapshot_from_staged_diff_without_unstaged_content ... ok
test git_service_startchat_snapshot_preserves_no_head_and_non_git_fallback ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.79s

     Running tests\git_contracts.rs (target\debug\deps\git_contracts-1c653788c8bd200e.exe)

running 10 tests
test git_command_output_preserves_raw_stream_contract ... ok
test git_commit_params_preserves_no_verify_rename_contract ... ok
test git_changed_file_status_preserves_snake_case_contract ... ok
test git_diff_arg_builders_preserve_existing_command_contract ... ok
test git_graph_contract_preserves_camel_case_contract ... ok
test git_name_status_parser_preserves_common_status_contract ... ok
test git_text_parsers_preserve_branch_and_log_contracts ... ok
test git_worktree_parser_preserves_porcelain_contract ... ok
test git_worktree_info_preserves_camel_case_contract ... ok
test git_service_preserves_repository_status_contract ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

     Running tests\remote_ssh_contracts.rs (target\debug\deps\remote_ssh_contracts-adec0cb4282e6b3d.exe)

running 7 tests
test remote_workspace_session_paths_use_supplied_mirror_root ... ok
test remote_ssh_legacy_agent_auth_maps_to_default_private_key ... ok
test remote_workspace_path_helpers_preserve_current_identity_contract ... ok
test remote_workspace_defaults_keep_older_files_loadable ... ok
test remote_workspace_registry_preserves_legacy_state_and_clear_contract ... ok
test remote_workspace_registry_preserves_ambiguous_root_resolution_contract ... ok
test local_workspace_identity_helpers_preserve_canonical_root_contract ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\request_builders_and_adapters.rs (target\debug\deps\request_builders_and_adapters-c07565e414a5b04a.exe)

running 4 tests
test mcp_protocol_jsonrpc_helpers_preserve_wire_shape ... ok
test mcp_protocol_prompt_content_helpers_preserve_legacy_text_behavior ... ok
test mcp_protocol_request_builders_preserve_wire_shape ... ok
test mcp_resource_and_prompt_adapters_preserve_context_rendering_contract ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\tool_names_and_protocol.rs (target\debug\deps\tool_names_and_protocol-3037ebf40214d947.exe)

running 3 tests
test mcp_protocol_capability_contract_matches_existing_default ... ok
test mcp_tool_info_preserves_json_shape ... ok
test mcp_tool_name_contract_matches_existing_wire_format ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\workspace_search_contracts.rs (target\debug\deps\workspace_search_contracts-74edd1aa3b888cad.exe)

running 3 tests
test workspace_search::daemon_binary_contract_lists_current_platform_candidate ... ok
test workspace_search::daemon_missing_hint_preserves_env_override_guidance ... ok
test workspace_search::service_constructs_without_core_runtime_dependencies ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests northhing_services_integrations

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 5. 归零检查

```powershell
rg -n "remote_connect\b|\bRemoteConnect\b" src --glob "*.rs"
```
**输出**：
```text
src\crates\contracts\core-types\tests\surface_contracts.rs:64:        kind: ThreadEnvironmentKind::RemoteConnect,
src\crates\contracts\core-types\tests\surface_contracts.rs:73:    assert_eq!(json["kind"], "remote_connect");
src\crates\contracts\core-types\src\surface.rs:27:    RemoteConnect,
```
*(说明：命中项属于 contracts 层 `core-types`，依据任务书约束第 2 条“contracts 层零改动，C4 才修剪”，本批次不触碰)*

```powershell
rg -n "remote-connect" src scripts --glob "*.toml" --glob "*.mjs"
```
**输出**：
```text
(0 命中)
```

```powershell
rg -n "remote_connect\b|\bRemoteConnect\b" tests src/crates/services/services-integrations/tests
```
**输出**：
```text
(0 命中)
```

---

## 残留解释与遗留疑虑

1. `ThreadEnvironmentKind::RemoteConnect` 在 `src/crates/contracts/core-types/src/surface.rs` 及对应契约测试中的残留：属于 contracts 层（C4 批次才清理），严格遵守 Global Constraints 第 2 条“contracts 层零改动”，未做任何变动。
2. `remote_connection_id` / `lookup_remote_connection*` 等符号：属于 SSH 远程连接与工作区会话标识语义（`remote-ssh` / `remote_ssh` 模块），严格遵守最高危纪律第 1 条“SSH 语义零改动”，完整保留。
3. 未引入任何破坏性变更，工作区其它未跟踪或并行 session 修改（`memory/`、`.opencode/` 等）均未动、未 commit。
