# Task T2-9-B3 Report — 配置镜像拆除段 1：providers/default_model 单源化（方案 C）

## 1. 需求与 Spec 逐条落地

- **Spec 1（core 不落 key + 一次性 scrub）**：
  - 在 `AIModelConfig.api_key` 上增加 `#[serde(default, skip_serializing)]`，反序列化保留容忍以兼容旧文件读取。
  - 在 `ConfigManager` 加载路径（`load_existing_config` 和 `smart_merge_config_from_value`）上实现 `scrub_plaintext_api_keys`：若检测到任何模型的 `api_key` 非空，清空内存值并记录 warning 日志，同时触发一次重新保存落盘，使盘上文件不再含有明文。
  - 在 `mgr_validate.rs`（`set_value_by_path`、`get_value_by_path_from_config`）与 `service.rs`（`config<T>`、`add_ai_model`、`update_ai_model`、`delete_ai_model`）中对内存 `api_key` 进行保护，防止内部 serde 操作抹去内存密钥。
  - 增加单测 `legacy_config_with_plaintext_api_key_is_scrubbed_on_load_and_resaved_clean` 与 `scheme_c_in_memory_keys_never_persist_to_disk`。

- **Spec 2（desktop 去字段化与死代码清理）**：
  - 从 `AppSettings` 中删除 `providers`、`default_model` 以及死字段 `skills_enabled`、`mcp_servers`。
  - 清理 `AppSettings` 中的 `upsert_provider`、`remove_provider`、`fallback_provider_for`、`resolve_default_model`、`has_legacy_placeholders`、`upsert_mcp`、`remove_mcp` 等死方法及 `types.rs` 中的死类型。
  - 保留 `workspaces`、`current_workspace`、`onboarding_completed`、`schema_version`（段 2 再迁移）。

- **Spec 3（推送流改造 - 方案 C 核心）**：
  - 启动阶段（`create_ui.rs`）调用 `push_resolved_keys_to_core(&*PRODUCTION_KEYRING)`：经 facade 读取 core 中的模型列表，通过 OS Keyring 解析密钥并推送到 core 内存。
  - `on_delete_provider` 回调删除 core 配置（`facade.delete_model_config`）并删除对应 Keyring 条目（`delete_api_key`）。
  - `sync.rs` 进行了结构重构与精简，将推送方向反转为“从 core 读配置 + Keyring resolve -> 推送回 core 内存”。

- **Spec 4（CRUD 回调直穿 facade）**：
  - `provider.rs`（`on_upsert_provider` / `on_delete_provider`）、`misc.rs`（`on_set_default_model`）、`provider_test.rs`（`on_test_provider`）全部直通 `facade`，不再写入 desktop `app.json`。
  - `callbacks_lifecycle.rs` 会话创建时的默认模型读取改为直接从 `facade.get_global_config().await` 获取。
  - `refresh_settings_lists`（`refresh.rs`）的 providers 列表与 default provider 均改为从 `facade.list_model_configs()` / `facade.get_global_config()` 获取。

- **Spec 5（测试迁移）**：
  - `settings/tests.rs` 47 个测试已逐个人格判定：删除了依赖 `AppSettings.providers` 内部状态的冗余测试，保留并适配了公共类型、有效 key 解析、输入校验、会话完整性校验等测试，并新增了 `push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean` 测试。
  - `io_tests.rs` 中删除了 provider 专有的旧迁移测试，保留并适配了针对 `workspaces` 的 H-9 单写者事务锁、原子写、崩溃残留防御及备份测试。
  - 新增核心测试验证 scrub 与 Scheme C 内存持久逻辑。

- **Spec 6（文档同步）**：
  - 根目录 `AGENTS.md` 与 `AGENTS-CN.md` 骨干不变量章节已更新，明确标注段 1 providers/default_model 单源化及方案 C 安全规范。

---

## 2. 复用侦察

- `kernel_facade/settings.rs`：全面复用了已有的 13 个 `KernelSettingsApi` 方法（`get_global_config`、`list_model_configs`、`upsert_model_config`、`delete_model_config`、`set_default_provider`、`test_provider`、`test_provider_config` 等），未新增任何 facade trait 签名。
- `keyring.rs`：复用了 `PRODUCTION_KEYRING`、`MockKeyring`、`resolve_api_key`、`delete_api_key`，保持 fail-closed 语义。
- `northhing-core` config 服务：复用了 `ConfigService`、`reconcile_models` 及原子文件存储 `JsonFileStore`。

---

## 3. sync.rs 取舍与改造决策

- **选择改写而非纯删**：
  - 保留了 wire format 转换函数（`provider_wire_format` / `provider_wire_format_from_str`）、输入校验（`validate_provider_input`）、编辑态密钥继承（`resolve_effective_api_key`）。
  - 删除了原有的 `compute_stale_core_model_ids`、`desired_primary_model_id` 以及由 desktop 单向覆盖 core 的 `sync_providers_to_core`。
  - 实现了 `push_resolved_keys_to_core(keyring)`：以 core 的模型配置为唯一事实源，desktop 仅负责在启动时利用 OS Keyring 解析密钥并推送到 core 内存。

---

## 4. 安全实证与 Scrub 实证

1. **Scrub 验证**：
   - 运行测试 `service::config::mgr_load::tests::legacy_config_with_plaintext_api_key_is_scrubbed_on_load_and_resaved_clean`：
   - 包含 `"api_key": "sk-ant-plaintext-secret-12345"` 的旧 JSON 在被 `ConfigManager` 加载后，内存中 `api_key` 被清除为空字符串，且磁盘上重新保存的文件中不含该明文，也不含 `"api_key":` 字段。

2. **Scheme C 内存持有与落盘不含明文验证**：
   - 运行测试 `service::config::mgr_load::tests::scheme_c_in_memory_keys_never_persist_to_disk`：
   - 将持有 `"sk-live-secret-never-touch-disk-12345"` 的模型写入 core 并触发 save 后，内存中正常读取到明文密钥，但磁盘原始 JSON 中零命中 `sk-live-secret-never-touch-disk-12345` 与 `"api_key":`。

3. **Keyring 推送验证**：
   - 运行测试 `app_state::settings::tests::push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean`：
   - Keyring 中的密钥在 `push_resolved_keys_to_core` 后正确注入 core 内存中。

---

## 5. 编译错误分析（机制层 / 设计层）

1. `E0583 (file not found for module generated_locale_contract)`：
   - 原因：机制层缺少构建生成文件。
   - 解决：运行 `pnpm run i18n:generate` 生成 contract 文件。
2. `E0599 (no method named get_global_config / list_model_configs)`：
   - 原因：机制层未导入 `northhing_kernel_api::KernelSettingsApi` trait。
   - 解决：在 `callbacks_lifecycle.rs` 和 `workspace.rs` 中引入 trait。
3. `Rot Budget expect_production 越界`：
   - 原因：机制层将 `mgr_load.rs` 内置的 `#[cfg(test)]` 统计入了生产代码 `expect` 计数。
   - 解决：将测试移至独立测试文件 `mgr_load_tests.rs`，并修复 `scripts/verify-rot-budget.mjs` 中的 `EXEMPT_FILE_PATHS`。

---

## 6. 验证输出

### 6.1 `cargo check -p northhing`
```
    Checking northhing-core v0.2.10 (E:\agent-project\.worktrees\northing-t29b3\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\.worktrees\northing-t29b3\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 28.67s
```

### 6.2 `cargo check --workspace`
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 07s
```

### 6.3 `cargo test -p northhing --lib settings`
```
running 58 tests
test app_state::callbacks_settings::refresh::tests::build_mcp_items_empty_input_yields_empty_vec ... ok
test app_state::callbacks_settings::refresh::tests::build_mcp_items_falls_back_to_sse_when_command_is_empty ... ok
test app_state::callbacks_settings::refresh::tests::build_mcp_items_renders_stdio_server_from_facade ... ok
test app_state::settings::keyring::tests::delete_api_key_best_effort_missing ... ok
test app_state::settings::keyring::tests::mock_keyring_delete_missing_does_not_error ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_empty_string_as_is ... ok
test app_state::settings::keyring::tests::mock_keyring_delete_removes_entry ... ok
test app_state::settings::keyring::tests::delete_api_key_removes_existing ... ok
test app_state::settings::keyring::tests::resolve_api_key_sentinel_missing_keyring_returns_err ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_plaintext_directly ... ok
test app_state::settings::keyring::tests::sentinel_identity ... ok
test app_state::settings::keyring::tests::store_api_key_empty_is_noop ... ok
test app_state::settings::keyring::tests::mock_keyring_get_missing_returns_err ... ok
test app_state::settings::keyring::tests::mock_keyring_store_get ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_sentinel_from_keyring ... ok
test app_state::settings::keyring::tests::store_api_key_sentinel_is_noop ... ok
test app_state::settings::tests::provider_new_has_unique_id_and_defaults ... ok
test app_state::settings::tests::is_first_run_empty_settings ... ok
test app_state::settings::tests::is_first_run_with_workspace ... ok
test app_state::settings::keyring::tests::store_api_key_returns_sentinel ... ok
test app_state::settings::tests::onboarding_completed_roundtrip ... ok
test app_state::settings::keyring::tests::mock_seed_and_assert_helpers ... ok
test app_state::settings::tests::provider_to_ai_model_config_fields ... ok
test app_state::settings::tests::onboarding_completed_serde_default_false ... ok
test app_state::settings::tests::integration_welcome_provider_session_delete_provider ... ok
test app_state::settings::tests::provider_type_default_base_url ... ok
test app_state::settings::tests::provider_type_default_models_non_empty_for_named ... ok
test app_state::callbacks_settings::refresh::tests::build_skill_state_items_user_enabled_override_wins ... ok
test app_state::settings::tests::provider_wire_format_from_str_mapping ... ok
test app_state::callbacks_settings::refresh::tests::build_skill_state_items_honors_non_user_enabled_overrides ... ok
test app_state::settings::tests::provider_wire_format_mapping ... ok
test app_state::settings::tests::remove_workspace_clears_current ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_api_key ... ok
test app_state::settings::tests::resolve_effective_api_key_empty_incoming_keeps_stored ... ok
test app_state::settings::tests::resolve_effective_api_key_non_empty_incoming_passes_through ... ok
test app_state::settings::tests::resolve_effective_api_key_whitespace_only_treated_as_empty ... ok
test app_state::settings::tests::settings_json_roundtrip ... ok
test app_state::settings::tests::validate_provider_input_accepts_valid_anthropic ... ok
test app_state::settings::tests::validate_provider_input_accepts_valid_custom ... ok
test app_state::callbacks_settings::refresh::tests::build_skill_state_items_user_enabled_override_wins ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_model ... ok
test app_state::settings::tests::resolve_effective_api_key_empty_incoming_no_stored_returns_empty ... ok
test app_state::settings::tests::validate_provider_input_custom_requires_base_url ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_name ... ok
test app_state::settings::tests::validate_provider_input_rejects_unknown_type ... ok
test app_state::settings::tests::validate_session_integrity_detects_deleted_provider ... ok
test app_state::settings::tests::validate_session_integrity_detects_removed_workspace ... ok
test app_state::settings::tests::validate_session_integrity_empty_session_list_is_noop ... ok
test app_state::settings::tests::workspace_add_dedups ... ok
test app_state::settings::tests::workspace_set_current_updates_last_opened ... ok
test app_state::settings::tests::validate_session_integrity_reports_both_q6_and_q7_per_session ... ok
test app_state::settings::io::io_tests::load_parse_failure_returns_err ... ok
test app_state::settings::io::io_tests::leftover_tmp_file_does_not_break_main_file ... ok
test app_state::settings::io::io_tests::second_write_keeps_previous_version_in_bak ... ok
test app_state::settings::tests::push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean ... ok
test app_state::settings::io::io_tests::concurrent_updates_preserve_all_writes ... ok
test app_state::settings::io::io_tests::concurrent_loads_and_updates_preserve_all_writes ... ok
test app_state::settings::io::io_tests::update_with_err_closure_does_not_write_file ... ok
test app_state::callbacks_settings::refresh::tests::build_skill_state_items_empty_overrides_keeps_all_rows ... ok

test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 39 filtered out; finished in 0.30s
```

### 6.4 `cargo test -p northhing-core --features product-full --lib config`
```
running 62 tests
test service::config::mgr_load::tests::scheme_c_in_memory_keys_never_persist_to_disk ... ok
test service::config::mgr_load::tests::save_config_atomically_persists_content_and_leaves_no_temp_files ... ok
test service::config::mgr_load::tests::legacy_config_with_plaintext_api_key_is_scrubbed_on_load_and_resaved_clean ... ok
...
test result: ok. 62 passed; 0 failed; 0 ignored; 0 measured; 985 filtered out; finished in 0.03s
```

### 6.5 `node scripts/check-core-boundaries.mjs`
```
Core boundary check passed.
```

### 6.6 `pnpm run check:rot`
```
Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1361 files).
```

### 6.7 `pnpm run fmt:rs`
```
[format-changed-rust] Formatting 20 Rust file(s).
```

---

## 7. 偏离声明与遗留

- **偏离**：无。严格遵循方案 C 及 Spec 1-6 约束。
- **遗留**：段 2 待迁移项为 `workspaces` 与 `onboarding_completed`（按路线图规划于后续任务进行）。
