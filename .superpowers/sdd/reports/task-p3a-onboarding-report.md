# Task P3a — Onboarding 三步校验流 + 完成副作用接线 Implementation Report

## 1. 改动清单 (Change Manifest)

| 文件 | 行号范围 | 说明 |
|---|---|---|
| `src/apps/desktop/src/ui_dioxus/api.rs` | L1-18, L100-120, L150-185 | 引入 `ProviderFormDto` / `ProviderTestResultDto`；新增 `test_provider_config` 与 `store_provider_api_key` 薄封装；补齐 uninit 测试链。 |
| `src/apps/desktop/src/ui_dioxus/pages_onboarding.rs` | L1-35, L26-55, L115-175, L540-560, L600-720, L820-860 | 引入 `Step` 枚举、`step_gate` 门控纯函数；接入 `test_provider_config` 进行 Card 2 & 底栏 Step::Two 连接测试；实现 Step::Three 目录存在性校验；接入完成三副作用（store_provider_api_key fail-closed, update_app_settings fail-closed, create_session best-effort）；新增 `step_gate` 3 条单元测试。 |

## 2. 复用侦察结论 (Recon Summary)

1. **api.rs 既有包装**：`api.rs` 此前无 `test_provider_config` 或 `store_provider_api_key` 包装，按 brief 规范新增薄包装。
2. **ProviderFormDto 组装先例**：`ProviderFormDto` 主要在 core Facade (`kernel_facade/settings.rs`) 及 Slint 回调 (`callbacks_settings/provider_test.rs`) 组装。`ui_dioxus` 处方在 `pages_onboarding.rs` 中按 Step 2 表单输入实时构造 DTO（`provider_id="onboarding"`, `model="default"`）。
3. **update_app_settings 调用先例**：确认 `update_app_settings` 闭包签名 `FnOnce(&mut AppSettings) -> Result<T>`。在 `pages_onboarding.rs` 中使用 `crate::app_state::settings::update_app_settings(|s| { s.onboarding_completed = true; s.add_workspace(ws_buf.clone()); Ok(()) })`，与 `app_state::settings` 保持完全一致。

## 3. 验证输出 (Verification Output)

### ① `cargo check -p northhing`
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 49s
(0 compilation errors)
```

### ② `cargo check -p northhing --tests`
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 28s
(0 compilation errors)
```

### ③ `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib ui_dioxus`
```
running 28 tests
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok
test ui_dioxus::app::tests::test_mix_hex_base ... ok
test ui_dioxus::app::tests::test_mix_hex_target ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_single ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_three_history ... ok
test ui_dioxus::api::tests::test_event_channel_returns_receiver ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_two ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_one ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_three ... ok
test ui_dioxus::pages_settings::tests::test_mcp_server_toggle_optimistic_update ... ok
test ui_dioxus::pages_settings::tests::test_update_app_settings_transaction_closure ... ok
test ui_dioxus::pages_settings::tests::test_load_app_settings_resolves_workspace_path_or_default ... ok
test ui_dioxus::pages_settings::tests::test_provider_active_matching ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_user_text_to_witness ... ok
test ui_dioxus::api::tests::test_ensure_room_session_fails_cleanly_when_uninitialized ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_empty_returns_empty ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_assistant_mixed_reasoning_fallback ... ok
test ui_dioxus::registry::tests::test_register_window_with_hwnd_and_mark_closing_target ... ok
test ui_dioxus::registry::tests::test_notify_closed_with_gen_matching_vs_stale ... ok
test ui_dioxus::registry::tests::test_archive_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_settings_registration_and_lifecycle ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_assistant_mixed_with_tool_calls ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_system_and_tool_skipped ... ok
test ui_dioxus::registry::tests::test_onboarding_registration_and_lifecycle ... ok
test ui_dioxus::session_mock::tests::test_seed_session_has_mock_approvals_with_call_ids ... ok
test ui_dioxus::registry::tests::test_space_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_shell_window_manager_clone_state_sharing ... ok
test ui_dioxus::api::tests::test_api_functions_fail_cleanly_before_init ... ok

test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 120 filtered out; finished in 0.01s
```

## 4. 偏离及理由 (Deviations)

无偏离。严格遵循 task-p3a-onboarding-brief.md 规范进行改动与验证，未修改 CSS 样式、DOM 结构及 i18n 键。
