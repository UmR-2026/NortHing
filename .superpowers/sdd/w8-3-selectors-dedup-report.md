# Implementation Report — W8-3: selectors.rs 消三处复制

- **状态**: DONE
- **仓库**: `E:\agent-project\NortHing` (main)
- **范围**: `src/apps/cli` + `scripts/rot-budget.json`

---

## 1. 改动清单

1. **ModelItem 映射去重**
   - 在 `src/apps/cli/src/ui/model_selector.rs` 为 `ModelItem` 提取 `ModelItem::from_config(&AIModelConfig) -> ModelItem` 与 `From<&AIModelConfig>` 实现。
   - 调用点切换：
     - `src/apps/cli/src/ui/startup/selectors.rs` (`show_model_selector`)
     - `src/apps/cli/src/modes/chat/model.rs` (`show_model_selector`)
   - 保持 `.filter(|m| m.enabled)` 语义与原有展示完全一致。
   - 附带 `ModelItem::from_config` 单元测试。

2. **time-ago 四档格式化去重**
   - 归属选择：置于 `src/apps/cli/src/ui/session_selector.rs`（`SessionItem` 的定义文件，`selectors.rs` 与 `chat/session.rs` 均直接引用该模块）。
   - 提取纯函数 `format_elapsed(std::time::Duration) -> String` 与 `format_time_ago(std::time::SystemTime) -> String`。
   - 调用点切换：
     - `src/apps/cli/src/ui/startup/selectors.rs` (`show_session_selector`)
     - `src/apps/cli/src/modes/chat/session.rs` (`show_session_selector`)
   - 附带覆盖全部四档阈值边界（<60s -> "just now", 60s..3600s -> "Xm ago", 3600s..86400s -> "Xh ago", >=86400s -> "Xd ago"）与 `format_time_ago` 的单元测试。

3. **custom_headers 解析去重**
   - 在 `src/apps/cli/src/ui/startup/selectors.rs` 提取私有 helper `parse_custom_headers(raw_headers: &str, headers_mode: &str) -> (Option<HashMap<String, String>>, Option<String>)`。
   - 调用点切换：`save_new_model` 与 `update_existing_model` 两处。

4. **rot-budget ceiling 下调**
   - `src/apps/cli/src/ui/startup/selectors.rs` 新行数：**861 行**（由 875 行下降 14 行）。
   - `scripts/rot-budget.json` 中 `god_file:src/apps/cli/src/ui/startup/selectors.rs` ceiling 下调至 **861**。

5. **深审观察项记录**
   - `provider_display_name` 隐式格式契约与 `UNIX_EPOCH` unwrap 为深审观察项，本波保持不动。

---

## 2. 验证证据

### (1) `check -p northhing-cli`
```text
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing-cli
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.05s
```

### (2) `test -p northhing-cli`
```text
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-cli
     Running unittests src\main.rs (target\debug\deps\northhing_cli-dbd0e8af6897a04e.exe)

running 41 tests
test acp_cli::tests::cli_acp_permission_mode_converts_correctly ... ok
test acp_cli::tests::acp_help_text_contains_command_placeholder ... ok
test acp_cli::tests::external_acp_client_display_name_matches_expected_values ... ok
test acp_cli::tests::external_acp_client_id_matches_expected_values ... ok
test acp_cli::tests::external_acp_client_config_has_enabled_true ... ok
test acp_cli::tests::print_generic_config_contains_transport_stdio ... ok
test acp_cli::tests::print_zed_config_contains_agent_servers_key ... ok
test acp_cli::tests::render_command_joins_command_and_args ... ok
test acp_cli::tests::shell_command_appends_acp_subcommand ... ok
test commands::tests::test_empty_query_returns_empty ... ok
test commands::tests::test_startup_command_specs_help ... ok
test commands::tests::test_exact_match ... ok
test commands::tests::test_match_does_not_mutate_specs ... ok
test commands::tests::test_mid_string_match ... ok
test commands::tests::test_no_match ... ok
test commands::tests::test_multiple_substring_matches ... ok
test commands::tests::test_prefix_match ... ok
test commands::tests::test_slash_only_returns_empty ... ok
test commands::tests::test_case_insensitive ... ok
test commands::tests::test_startup_command_specs_no_match ... ok
test commands::tests::test_startup_command_specs_prefix_match ... ok
test ui::chat::state_split_tests::mouse_state_new_initializes_all_fields ... ok
test commands::tests::test_substring_match ... ok
test config::tests::default_cli_config_has_expected_values ... ok
test ui::chat::state_split_tests::accessor_methods_work_correctly ... ok
test ui::chat::state_split_tests::chatview_new_initializes_all_substructures ... ok
test ui::chat::state_split_tests::chatview_fields_accessible_after_refactor ... ok
test ui::chat::state_split_tests::clear_screen_resets_all_substructures ... ok
test ui::chat::state_split_tests::popup_manager_new_initializes_all_states ... ok
test ui::chat::state_split_tests::popup_stack_operations ... ok
test config::tests::config_toml_round_trip_preserves_values ... ok
test ui::chat::state_split_tests::selection_state_new_initializes_all_fields ... ok
test ui::model_config_form::state::tests::validate_allows_blank_api_key_in_edit_mode ... ok
test ui::model_config_form::state::tests::validate_blocks_blank_api_key_in_add_mode ... ok
test ui::model_selector::tests::test_model_item_from_config ... ok
test ui::session_selector::tests::test_format_elapsed_four_tiers ... ok
test ui::theme::tests::eight_digit_hex_colors_are_supported ... ok
test ui::session_selector::tests::test_format_time_ago_recent ... ok
test keyring_keys::tests::missing_keyring_entry_resolves_to_empty ... ok
test keyring_keys::tests::typed_key_wins_over_keyring ... ok
test ui::theme::tests::builtin_themes_resolve_for_dark_and_light ... ok

test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### (3) `node scripts/verify-rot-budget.mjs`
```text
node scripts/verify-rot-budget.mjs
Rot budget verification passed (5 grep rules [unwrap_production=474/502, expect_production=940/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=274/400], 7 god-file rules checked across 1348 files).
```

---

## 3. 偏离清单

- 无任何偏离。
