# Task P22 — Room Workspace Session Resolution & Double-Create TOCTOU Fix Report

## 改动清单
- `src/apps/desktop/src/ui_dioxus/api.rs`:
  - `list_sessions_all_workspaces()` (line 58-61): 新增加载全工作区会话分组摘要的薄包装。
  - `pick_room_session()` (line 72-87): 纯函数选择器，根据 preferred_workspace 参数从 WorkspaceSessionsDto 列表中择优挑选 Room 会话。
  - `ROOM_SESSION_CACHE` (line 89): `tokio::sync::Mutex<Option<String>>` 进程级 Mutex 缓存，防止并发 `ensure_room_session()` 导致的双建 TOCTOU 竞态。
  - `ensure_room_session()` (line 91-125): 重写 ensure 逻辑，持锁读取缓存/解析 preferred workspace/调 `list_sessions_all_workspaces()`/`pick_room_session()`，若未命中则在 preferred workspace 下创建新诊室会话并更新缓存。
  - `tests` 模块 (line 217-310): 新增 4 条 `pick_room_session` 单元测试，并在 `test_api_functions_fail_cleanly_before_init` 中包含 `list_sessions_all_workspaces`。
- `src/apps/desktop/src/ui_dioxus/i18n.rs`:
  - 删除 line 387 `pub const ONBOARDING_BTN_COMPLETE` 孤儿 i18n 键（warnings 36 → 35）。

## 判定表逐行落点 (pick_room_session)
| 输入条件 | 实现代码 | 验证测试 |
|---|---|---|
| `preferred = Some(ws)` 且存在 `workspace_path == ws` 的组且 sessions 非空 | `groups.iter().find(|g| g.workspace_path == ws).and_then(|g| g.sessions.first())` | `test_pick_room_session_preferred_hit` |
| `preferred = Some(ws)` 但无匹配组或组 sessions 为空 | `find` 未找到组或 `sessions.first()` 为 `None`，返回 `None`（要求在 preferred 工作区新建，不回落其他工作区的旧会话） | `test_pick_room_session_preferred_miss_returns_none` |
| `preferred = None` | `groups.iter().find(|g| !g.sessions.is_empty()).and_then(|g| g.sessions.first())`，全空返回 `None` | `test_pick_room_session_no_preferred_picks_first_non_empty`, `test_pick_room_session_empty_groups_returns_none` |

## 缓存天花板声明 (Ponytail Ceiling)
- 缓存策略：`ROOM_SESSION_CACHE` 在进程生命周期内有效，Err 不缓存允许重试。
- 天花板：删除/归档会话后需重启应用才能重新解析新诊室会话。
- 升级路径：未来接入 `delete_session` 事件/通知时主动使缓存失效。

## 放弃修复 `entries.set` 启动竞态的理由 (P2a M1)
- 裁定不修理由：订阅流仅推送订阅建立之后产生的新事件；应用启动初期没有任何处于运行中（in-flight）的 dialog turn，因此不会在该覆盖窗口期产生需要保留的 Approval 卡。无真实触发路径，保持现存代码不动，ledger 注记保留。

## 复用侦察
1. `list_sessions_all_workspaces`: 经全仓搜索确认，此前除 facade / trait 定义外零 UI 消费者。
2. `load_app_settings`: 沿用 `super::super::app_state::settings::load_app_settings` / `crate::app_state::settings::load_app_settings` 既有路径模式。
3. 等价 Helper 检查: Slint 侧会话历史面板拥有类似的局部选组逻辑，但属跨层独立代码，不可与 Dioxus 侧共享，故实现独立纯函数 `pick_room_session`。

## 验证尾部输出

1. `cargo check -p northhing`
```
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
warning: `northhing` (bin "northhing") generated 36 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 53.55s
```

2. `cargo check -p northhing --tests`
```
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
warning: `northhing` (bin "northhing" test) generated 37 warnings (run `cargo fix --bin "northhing" -p northhing --tests` to apply 7 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 30.23s
```

3. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib ui_dioxus`
```
running 32 tests
test ui_dioxus::api::tests::test_pick_room_session_no_preferred_picks_first_non_empty ... ok
test ui_dioxus::api::tests::test_pick_room_session_preferred_hit ... ok
test ui_dioxus::api::tests::test_pick_room_session_empty_groups_returns_none ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_three ... ok
test ui_dioxus::api::tests::test_event_channel_returns_receiver ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_one ... ok
test ui_dioxus::api::tests::test_pick_room_session_preferred_miss_returns_none ... ok
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok
test ui_dioxus::app::tests::test_mix_hex_base ... ok
test ui_dioxus::app::tests::test_mix_hex_target ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_two ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_single ... ok
test ui_dioxus::pages_settings::tests::test_provider_active_matching ... ok
test ui_dioxus::pages_settings::tests::test_mcp_server_toggle_optimistic_update ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_system_and_tool_skipped ... ok
test ui_dioxus::pages_settings::tests::test_load_app_settings_resolves_workspace_path_or_default ... ok
test ui_dioxus::pages_settings::tests::test_update_app_settings_transaction_closure ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_three_history ... ok
test ui_dioxus::registry::tests::test_settings_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_notify_closed_with_gen_matching_vs_stale ... ok
test ui_dioxus::registry::tests::test_register_window_with_hwnd_and_mark_closing_target ... ok
test ui_dioxus::registry::tests::test_shell_window_manager_clone_state_sharing ... ok
test ui_dioxus::registry::tests::test_onboarding_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_space_registration_and_lifecycle ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_assistant_mixed_reasoning_fallback ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_assistant_mixed_with_tool_calls ... ok
test ui_dioxus::registry::tests::test_archive_registration_and_lifecycle ... ok
test ui_dioxus::session_mock::tests::test_seed_session_has_mock_approvals_with_call_ids ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_user_text_to_witness ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_empty_returns_empty ... ok
test ui_dioxus::api::tests::test_ensure_room_session_fails_cleanly_when_uninitialized ... ok
test ui_dioxus::api::tests::test_api_functions_fail_cleanly_before_init ... ok

test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 120 filtered out; finished in 0.02s
```

## 偏离及理由
- 无偏离。完全遵循 `task-p22-room-workspace-brief.md` 的所有硬约束与判定表要求。
