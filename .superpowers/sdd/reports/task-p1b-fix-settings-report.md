# Task P1b-fix Implementation Report — F5 Settings 数据源修正与持久化接线

## 1. 状态 (Status)
**DONE**

## 2. 变更文件 (Files Changed)
- `src/apps/desktop/src/ui_dioxus/api.rs`: 添加 facade `KernelSettingsApi` 薄封装（`get_global_config`, `list_model_configs`, `set_default_provider`, `list_mcp_servers`, `set_mcp_enabled`）及单测。
- `src/apps/desktop/src/ui_dioxus/pages_settings.rs`: 接入 facade 数据源（Card 1 模型引擎、Card 3 接入点、Card 4 能力集），`use_future` 页面挂载单次拉取，乐观更新 + 失败 warn，空列表 fail-open 回退至 mock 显示；新增 DTO 匹配与乐观更新测试。

## 3. 接线 / 保留清单 (Wired / Kept Mock List)
- **已接线 (Wired)**:
  - **Card 1 (模型引擎 ENGINE)**: `list_model_configs()` 渲染模型列表；点击行调用 `set_default_provider(&model.id)` 并乐观更新选中状态；空列表 fail-open 回退 mock 并保留 TODO 注释。
  - **Card 3 (接入点 PROVIDER)**: `get_global_config().providers` 渲染；`default_provider_id` 判定 active；点击行调用 `set_default_provider(&provider.id)` 并乐观更新。
  - **Card 4 (能力集 MCP & SKILLS)**: `list_mcp_servers()` 渲染；toggle 点击调用 `set_mcp_enabled(server, !current)` 并乐观更新信号集合。
  - **Card 5 (工作区 WORKSPACE)**: 保留 `load_app_settings()` 驱动的 workspace_path 展示。
- **保留 Mock (Kept Mock)**:
  - **Card 2 (上下文 CONTEXT)**: 保持静态 mock。
  - **Card 6 (显示模式 DISPLAY)**: 呼吸节奏 / 双光学透镜 AppSettings 无对应字段，保持 mock + `// TODO(data)`。

## 4. 验证命令与输出原文 (Verification Outputs)

### 命令 1: `cargo check -p northhing --features ui-dioxus`
```
warning: `northhing-core` (lib) generated 18 warnings (run `cargo fix --lib -p northhing-core` to apply 17 suggestions)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
warning: `northhing` (bin "northhing") generated 35 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 51.36s
```

### 命令 2: `cargo test -p northhing --features ui-dioxus --lib ui_dioxus`
```
warning: `northhing-core` (lib) generated 18 warnings (run `cargo fix --lib -p northhing-core` to apply 17 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.06s
     Running unittests src\lib.rs (target\debug\deps\northhing-998dc8915b11e80b.exe)

running 20 tests
test ui_dioxus::api::tests::test_event_channel_returns_receiver ... ok
test ui_dioxus::app::tests::test_mix_hex_base ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_single ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_three_history ... ok
test ui_dioxus::session_mock::tests::test_seed_session_has_mock_approvals_with_call_ids ... ok
test ui_dioxus::pages_settings::tests::test_mcp_server_toggle_optimistic_update ... ok
test ui_dioxus::pages_settings::tests::test_provider_active_matching ... ok
test ui_dioxus::api::tests::test_ensure_room_session_fails_cleanly_when_uninitialized ... ok
test ui_dioxus::pages_settings::tests::test_update_app_settings_transaction_closure ... ok
test ui_dioxus::pages_settings::tests::test_load_app_settings_resolves_workspace_path_or_default ... ok
test ui_dioxus::registry::tests::test_archive_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_notify_closed_with_gen_matching_vs_stale ... ok
test ui_dioxus::registry::tests::test_onboarding_registration_and_lifecycle ... ok
test ui_dioxus::api::tests::test_api_functions_fail_cleanly_before_init ... ok
test ui_dioxus::registry::tests::test_register_window_with_hwnd_and_mark_closing_target ... ok
test ui_dioxus::registry::tests::test_shell_window_manager_clone_state_sharing ... ok
test ui_dioxus::registry::tests::test_settings_registration_and_lifecycle ... ok
test ui_dioxus::app::tests::test_mix_hex_target ... ok
test ui_dioxus::registry::tests::test_space_registration_and_lifecycle ... ok
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok

test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 110 filtered out; finished in 0.00s
```

## 5. 偏离声明 (Deviations)
无。严格遵守 brief 要求，未触碰 `io.rs`/`keyring.rs`，未触碰 Card 6 显示开关，未触碰其他页面。
