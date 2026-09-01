# Task Report — Audit I1: provider 编辑路径 keyring 读失败吞错修复

## 1. 实现内容

- **`src/apps/desktop/src/app_state/settings/sync.rs`**:
  - 新增 `pub fn resolve_edit_api_key(stored: anyhow::Result<String>, incoming: &str) -> anyhow::Result<String>`。
  - 当传入 key `incoming.trim().is_empty()` 时，原样返回 `stored`（包括 `Err` 的传播）；当非空时返回 `Ok(incoming.to_string())`。
- **`src/apps/desktop/src/app_state/callbacks_settings/provider.rs`**:
  - 编辑路径（`!id.is_empty() && pkey.trim().is_empty()`）使用 `resolve_edit_api_key(PRODUCTION_KEYRING.get(&pid), &pkey)`。
  - `Err(e)` 分支触发 `tracing::warn!(target: "app_state", "keyring read failed for provider {pid}: {e}")` 记录不含敏感信息的英文日志，调用 `set_inline_error(ui_weak.clone(), "读取密钥库失败，请重试；如持续失败请重新输入 API Key".to_string())` 设置 UI 提示，并 `return` 拒绝保存。
  - `Ok(key)` 及新建/已输入 key 分支行为完全保持不变。
- **`src/apps/desktop/src/app_state/settings/tests.rs`**:
  - 新增 4 个单元测试覆盖 `resolve_edit_api_key`：
    1. `resolve_edit_api_key_err_stored_blank_incoming_returns_err` (覆盖审计要求的 error arm)
    2. `resolve_edit_api_key_ok_stored_blank_incoming_returns_ok_stored`
    3. `resolve_edit_api_key_err_stored_non_blank_incoming_returns_ok_incoming`
    4. `resolve_edit_api_key_ok_stored_non_blank_incoming_returns_ok_incoming`

## 2. 复用侦察

- **查了哪些符号**：
  - `resolve_effective_api_key`: 位于 `sync.rs:5-11`，包含 4 个现有单元测试 (`tests.rs:321-344`)。按 Spec 与约束要求完整保留。
  - `validate_provider_input`: 位于 `sync.rs:48-76`，负责校验必填项。按 Spec 保留作为第二道校验。
  - `KeyringBackend::get` / `PRODUCTION_KEYRING`: 位于 `keyring.rs`，提供 OS 密钥库读取能力，返回 `anyhow::Result<String>`。
- **复用了什么**：
  - 复用了 `PRODUCTION_KEYRING.get(&pid)` 的 Result 返回结构；
  - 复用了 `set_inline_error` 的 Slint UI 错误通知机制；
  - 复用了 `tracing::warn!` 日志规范与 `app_state` target。
- **若新写等价物逐条给理由**：无新写等价物或重造轮子。新增 `resolve_edit_api_key` 为针对 Fail-Closed keyring 错误传播的专有逻辑。

## 3. 审计论断核实

- **审计声称**：“keyring 读失败 → `Some("")` 进 upsert → 抹掉用户 key”。
- **事实核实**：该声称**不成立**。
  - 在 `provider.rs:127` / `sync.rs:58-60` 中，`validate_provider_input` 包含拦截空 Key 的校验：`if api_key.trim().is_empty() { return Err("API Key 不能为空".to_string()); }`。
  - 旧代码若吞掉 keyring `get` 的 `Err`（转为 `None`），`resolve_effective_api_key` 会产生空字符串 `""`。
  - 该空字符串进入 `validate_provider_input` 时必被拦截并拒绝保存，因此**绝不会**触发后面的 `PRODUCTION_KEYRING.store` 或 `kernel_facade().upsert_model_config`。 key 无论在 keyring 还是 core memory 均不可能被抹掉。
- **实际闭合的残余缺陷**：
  1. 消除 fail-open 观测盲区：旧代码吞错无日志，新代码添加了 `tracing::warn!` 记录 keyring 读取失败。
  2. 修复 UI 错误消息误导：旧代码报错“API Key 不能为空”（暗指用户输入错误），新代码给出“读取密钥库失败，请重试；如持续失败请重新输入 API Key”（准确指示密钥库故障）。

## 4. 编译错误修正层级

- 实现过程顺畅，一次性通过 `cargo check` 与 `cargo test`，**无编译错误（E0xxx）发生**。

## 5. 测试与输出原文

### 命令 1
```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check -p northhing
```
**输出原文（尾部）**：
```text
warning: `northhing` (bin "northhing") generated 37 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 55.94s
```

### 命令 2
```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib settings
```
**输出原文（尾部）**：
```text
running 81 tests
test app_state::callbacks_settings::refresh::tests::apply_skill_filter_unknown_category_does_not_lit_any_partition ... ok
test app_state::callbacks_settings::refresh::tests::build_mcp_items_empty_input_yields_empty_vec ... ok
test app_state::callbacks_settings::refresh::tests::apply_skill_filter_no_match_yields_empty_list_and_no_visible_partitions ... ok
test app_state::callbacks_settings::refresh::tests::apply_skill_filter_empty_returns_all_and_lights_all_partitions ... ok
test app_state::callbacks_settings::refresh::tests::apply_skill_filter_substring_is_case_insensitive_and_searches_description ... ok
test app_state::callbacks_settings::refresh::tests::build_mcp_items_falls_back_to_sse_when_command_is_empty ... ok
test app_state::callbacks_settings::refresh::tests::build_mcp_items_renders_stdio_server_from_facade ... ok
test app_state::settings::keyring::tests::mock_keyring_delete_removes_entry ... ok
test app_state::settings::keyring::tests::mock_keyring_store_env_sentinel_is_noop ... ok
test app_state::settings::keyring::tests::mock_keyring_delete_missing_does_not_error ... ok
test app_state::settings::keyring::tests::delete_api_key_best_effort_missing ... ok
test app_state::settings::keyring::tests::delete_api_key_removes_existing ... ok
test app_state::settings::keyring::tests::mock_keyring_get_missing_returns_err ... ok
test app_state::settings::keyring::tests::mock_keyring_load_env_missing_returns_empty_map_fail_open ... ok
test app_state::settings::keyring::tests::mock_keyring_load_env_corrupt_json_returns_empty_map_fail_open ... ok
test app_state::settings::keyring::tests::mock_keyring_store_get ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_sentinel_from_keyring ... ok
test app_state::settings::keyring::tests::sentinel_identity ... ok
test app_state::settings::keyring::tests::mock_keyring_store_load_env_roundtrip ... ok
test app_state::settings::keyring::tests::mock_seed_and_assert_helpers ... ok
test app_state::callbacks_settings::refresh::tests::build_skill_state_items_user_enabled_override_wins ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_empty_string_as_is ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_plaintext_directly ... ok
test app_state::settings::keyring::tests::resolve_api_key_sentinel_missing_keyring_returns_err ... ok
test app_state::settings::keyring::tests::store_api_key_empty_is_noop ... ok
test app_state::settings::keyring::tests::store_api_key_returns_sentinel ... ok
test app_state::settings::keyring::tests::store_api_key_sentinel_is_noop ... ok
test app_state::settings::tests::is_first_run_empty_settings ... ok
test app_state::settings::tests::is_first_run_with_workspace ... ok
test app_state::settings::tests::integration_welcome_provider_session_delete_provider ... ok
test app_state::settings::tests::onboarding_completed_roundtrip ... ok
test app_state::callbacks_settings::refresh::tests::build_skill_state_items_honors_non_user_enabled_overrides ... ok
test app_state::settings::tests::onboarding_completed_serde_default_false ... ok
test app_state::settings::tests::provider_new_has_unique_id_and_defaults ... ok
test app_state::settings::tests::provider_type_default_base_url ... ok
test app_state::settings::tests::remove_workspace_clears_current ... ok
test app_state::settings::tests::provider_wire_format_from_str_mapping ... ok
test app_state::settings::tests::provider_wire_format_from_str_other_defaults_to_openai ... ok
test app_state::settings::tests::provider_type_default_models_non_empty_for_named ... ok
test app_state::settings::tests::resolve_edit_api_key_err_stored_non_blank_incoming_returns_ok_incoming ... ok
test app_state::settings::tests::resolve_edit_api_key_err_stored_blank_incoming_returns_err ... ok
test app_state::settings::tests::resolve_edit_api_key_ok_stored_blank_incoming_returns_ok_stored ... ok
test app_state::settings::tests::resolve_edit_api_key_ok_stored_non_blank_incoming_returns_ok_incoming ... ok
test app_state::settings::tests::resolve_effective_api_key_empty_incoming_keeps_stored ... ok
test app_state::settings::tests::resolve_effective_api_key_non_empty_incoming_passes_through ... ok
test app_state::settings::tests::resolve_effective_api_key_empty_incoming_no_stored_returns_empty ... ok
test app_state::settings::io::io_tests::mcp_env_fail_open_missing_entry_returns_empty_map ... ok
test app_state::settings::tests::resolve_effective_api_key_whitespace_only_treated_as_empty ... ok
test app_state::settings::tests::settings_json_roundtrip ... ok
test app_state::settings::tests::validate_provider_input_accepts_valid_anthropic ... ok
test app_state::settings::tests::validate_provider_input_accepts_valid_custom ... ok
test app_state::settings::tests::validate_provider_input_custom_requires_base_url ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_api_key ... ok
test app_state::settings::io::io_tests::mcp_env_keyring_sentinel_loaded_and_restored ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_model ... ok
test app_state::settings::io::io_tests::load_parse_failure_returns_err ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_name ... ok
test app_state::settings::tests::validate_provider_input_rejects_unknown_type ... ok
test app_state::settings::tests::validate_session_integrity_detects_deleted_provider ... ok
test app_state::settings::tests::validate_session_integrity_detects_removed_workspace ... ok
test app_state::settings::tests::validate_session_integrity_empty_session_list_is_noop ... ok
test app_state::settings::io::io_tests::mcp_env_fail_closed_on_store_error_does_not_corrupt_disk ... ok
test app_state::settings::tests::workspace_add_dedups ... ok
test app_state::settings::io::io_tests::mcp_env_idempotent_load_with_sentinel_does_not_rewrite_keyring ... ok
test app_state::settings::tests::workspace_set_current_updates_last_opened ... ok
test ui_dioxus::pages_settings::tests::test_mcp_server_toggle_optimistic_update ... ok
test ui_dioxus::pages_settings::tests::test_load_app_settings_resolves_workspace_path_or_default ... ok
test ui_dioxus::pages_settings::tests::test_provider_active_matching ... ok
test ui_dioxus::pages_settings::tests::test_update_app_settings_transaction_closure ... ok
test app_state::settings::tests::validate_session_integrity_reports_both_q6_and_q7_per_session ... ok
test ui_dioxus::registry::tests::test_settings_registration_and_lifecycle ... ok
test app_state::settings::io::io_tests::leftover_tmp_file_does_not_break_main_file ... ok
test app_state::settings::io::io_tests::mcp_env_keyring_migration_plaintext_to_sentinel_on_load ... ok
test app_state::settings::io::io_tests::second_write_keeps_previous_version_in_bak ... ok
test app_state::settings::tests::push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean ... ok
test app_state::settings::io::io_tests::concurrent_updates_preserve_all_writes ... ok
test app_state::settings::io::io_tests::mcp_env_update_app_settings_stores_new_env_in_keyring ... ok
test app_state::settings::io::io_tests::update_with_err_closure_does_not_write_file ... ok
test app_state::settings::io::io_tests::concurrent_loads_and_updates_preserve_all_writes ... ok
test app_state::callbacks_settings::refresh::tests::build_skill_state_items_workspace_overrides ... ok
test app_state::callbacks_settings::refresh::tests::build_skill_state_items_empty_overrides_keeps_all_rows ... ok

test result: ok. 81 passed; 0 failed; 0 ignored; 0 measured; 75 filtered out; finished in 0.41s
```

## 6. 修改文件清单

- `src/apps/desktop/src/app_state/settings/sync.rs`
- `src/apps/desktop/src/app_state/callbacks_settings/provider.rs`
- `src/apps/desktop/src/app_state/settings/tests.rs`

## 7. 自审发现与疑虑

- **自审发现**：日志无敏感信息（无 secret 打印）、为英文且无 emoji；UI 提示文本符合设计；测试覆盖完整。
- **疑虑**：无。
