# W8-1 Task Implementation Report — input.rs 拆分

## 1. 状态与元信息

- **状态**：DONE
- **Commit SHA**：`3337c739dcad0859c49fc32aa5f9a3e94cab3239`
- **目标文件**：`src/apps/cli/src/modes/chat/input.rs` (802 lines) → `src/apps/cli/src/modes/chat/input/`
- **行为变更**：零行为变化（纯位移重构 + async 桥接 helper 提取）

## 2. Git 变更统计

```
commit 3337c739dcad0859c49fc32aa5f9a3e94cab3239
Author: Mavis <mavis@northhing.local>
Date:   Sat Aug 29 00:36:55 2026 +0800

    refactor(cli): split input.rs god-file into modular directory with bridge helper (W8-1)

 scripts/rot-budget.json                          |  15 +-
 src/apps/cli/src/modes/chat/input.rs             | 802 -----------------------
 src/apps/cli/src/modes/chat/input/bridge.rs      |  11 +
 src/apps/cli/src/modes/chat/input/key_actions.rs | 235 +++++++
 src/apps/cli/src/modes/chat/input/key_popups.rs  | 428 ++++++++++++
 src/apps/cli/src/modes/chat/input/mod.rs         |  46 ++
 src/apps/cli/src/modes/chat/input/non_key.rs     | 169 +++++
 7 files changed, 894 insertions(+), 812 deletions(-)
```

## 3. 新文件行数清单

| 文件路径 | 行数 | 职责说明 |
|---|---|---|
| `src/apps/cli/src/modes/chat/input/bridge.rs` | 11 | 通用 async bridge helper (`bridge<F, T>(rt_handle, fut)`) |
| `src/apps/cli/src/modes/chat/input/mod.rs` | 46 | 模块门面、子模块声明、`handle_key_event` 入口分发 |
| `src/apps/cli/src/modes/chat/input/key_popups.rs` | 428 | Popup 导航 (`any_popup_visible`/`close_all_popups`/`navigate_back`) 与 5 层弹窗/提示拦截处理 |
| `src/apps/cli/src/modes/chat/input/key_actions.rs` | 235 | 常规按键绑定与输入操作 (快捷键、输入法、翻页、历史等) |
| `src/apps/cli/src/modes/chat/input/non_key.rs` | 169 | 非按键事件 (`handle_non_key_event`) 与退出副作用分发 (`apply_exit_reason`) |

所有新文件均远低于 800 行上限（最大 428 行）。

## 4. 逐臂位移核对清单

### 4.1 `bridge.rs` 提取（消除 7 处重复）

| 原位置 | 场景 | 捕获变量与异步调用 | 现位置 |
|---|---|---|---|
| `input.rs:121` | PermissionAction::AllowOnce | `agent.confirm_tool(&tool_id, None)` | `key_popups.rs:117` (`bridge(rt_handle, async move { ... })`) |
| `input.rs:135` | PermissionAction::AllowAlways | `agent.confirm_tool(&tool_id, None)` + `set_config` | `key_popups.rs:133` (`bridge(rt_handle, async move { ... })`) |
| `input.rs:156` | PermissionAction::Reject | `agent.reject_tool(&tool_id, reason_clone)` | `key_popups.rs:152` (`bridge(rt_handle, async move { ... })`) |
| `input.rs:181` | QuestionAction::Submit | `agent.submit_user_answers(&tool_id, answers)` | `key_popups.rs:177` (`bridge(rt_handle, async move { ... })`) |
| `input.rs:444` | Ctrl+C cancel turn | `agent.cancel_current_turn()` | `key_actions.rs:32` (`bridge(rt_handle, async move { ... })`) |
| `input.rs:504` | Enter send message | `agent.send_message(input_clone, &agent_type)` | `key_actions.rs:82` (`bridge(rt_handle, agent.send_message(...))`) |
| `input.rs:606` | Esc cancel turn | `agent.cancel_current_turn()` | `key_actions.rs:181` (`bridge(rt_handle, async move { ... })`) |

### 4.2 `handle_key_event` 拦截层与分支搬迁

| 原始拦截层 / 分支 | 原行号 | 搬迁目标函数 / 文件 | 逻辑/条件变化 |
|---|---|---|---|
| `key.kind` press/repeat 预检 | L108-110 | `input/mod.rs` (`handle_key_event`) | 零变化，直接返回 `Ok(None)` |
| Layer 1: Permission Prompt 拦截 (AllowOnce / AllowAlways / Reject / None) | L112-170 | `key_popups.rs` (`handle_permission_prompt_key`) | 零变化，顺序与分支完全一致 |
| Layer 2: Question Prompt 拦截 (Submit / Reject / None) | L172-201 | `key_popups.rs` (`handle_question_prompt_key`) | 零变化，顺序与分支完全一致 |
| Layer 3: Global popup navigation (Ctrl+W close all, Esc navigate back) | L206-218 | `key_popups.rs` (`handle_popup_key`) | 零变化，顺序与分支完全一致 |
| Layer 4: Info popup 拦截 (dismiss on any key) | L220-224 | `key_popups.rs` (`handle_popup_key`) | 零变化，顺序与分支完全一致 |
| Layer 5: Command palette 拦截 (Execute / Dismiss / None) | L226-236 | `key_popups.rs` (`handle_popup_key`) | 零变化，顺序与分支完全一致 |
| Layer 6a: Model selector (Up / Down / Enter / Char('e')) | L239-259 | `key_popups.rs` (`handle_popup_key`) | 零变化，顺序与分支完全一致 |
| Layer 6b: Theme selector (Up / Down / Enter) | L261-286 | `key_popups.rs` (`handle_popup_key`) | 零变化，顺序与分支完全一致 |
| Layer 6c: Agent selector (Up / Down / Enter) | L288-302 | `key_popups.rs` (`handle_popup_key`) | 零变化，顺序与分支完全一致 |
| Layer 6d: Session selector (Switch / Delete / Close / None) | L304-316 | `key_popups.rs` (`handle_popup_key`) | 零变化，顺序与分支完全一致 |
| Layer 6e: Skill selector (Up / Down / Enter / Space) | L318-331 | `key_popups.rs` (`handle_popup_key`) | 零变化，顺序与分支完全一致 |
| Layer 6f: Subagent selector (Up / Down / Enter / Space) | L333-346 | `key_popups.rs` (`handle_popup_key`) | 零变化，顺序与分支完全一致 |
| Layer 6g: MCP selector (Up / Down / Enter / Char('a','d','e') / cancel confirm) | L348-384 | `key_popups.rs` (`handle_popup_key`) | 零变化，顺序与分支完全一致 |
| Layer 6h: MCP add dialog (Confirm / Cancel / None) | L386-399 | `key_popups.rs` (`handle_popup_key`) | 零变化，顺序与分支完全一致 |
| Layer 6i: Provider selector (handle_key) | L401-406 | `key_popups.rs` (`handle_popup_key`) | 零变化，顺序与分支完全一致 |
| Layer 6j: Model config form (Save / Cancel / None) | L408-424 | `key_popups.rs` (`handle_popup_key`) | 零变化，顺序与分支完全一致 |
| Layer 7: Ctrl+V (Clipboard paste) | L428-437 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Ctrl+C (Cancel / Quit) | L439-456 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Ctrl+P (Command palette) | L458-461 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Alt+Enter (Newline) | L464-466 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Enter (Slash command / Agent message / Menu confirm) | L468-516 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Backspace | L518-520 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Left / Right | L522-527 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Ctrl+O / Ctrl+J / Ctrl+K (Block tool expand / cycle) | L530-542 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Up / Down (Input history / Command menu) | L545-558 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Ctrl+Home / Ctrl+End / Home / End | L560-577 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Ctrl+U (Clear input) | L579-581 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Ctrl+E (Toggle browse mode) | L583-591 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: PageUp / PageDown | L593-600 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Esc (Cancel processing / Exit browse mode) | L602-620 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Tab / BackTab (Cycle agent) | L622-632 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Char(c) (Text input) | L634-638 | `key_actions.rs` (`handle_key_action`) | 零变化 |
| Layer 7: Catch-all (`_ => {}`) | L640 | `key_actions.rs` (`handle_key_action`) | 零变化 |

### 4.3 其它方法搬迁

| 原始函数/方法 | 原行号 | 搬迁目标位置 | 说明 |
|---|---|---|---|
| `any_popup_visible` | L21-34 | `key_popups.rs:17-30` | 零变化，12 种 popup 可见性检测 |
| `close_all_popups` | L37-55 | `key_popups.rs:33-51` | 零变化，关闭全部 popup 并清栈 |
| `navigate_back` | L58-98 | `key_popups.rs:54-94` | 零变化，popup 栈 pop + 上一个 popup re-show |
| `apply_exit_reason` | L647-679 | `non_key.rs:14-46` | 零变化，保留原有 8 参数签名与分支 |
| `handle_non_key_event` | L682-801 | `non_key.rs:49-168` | 零变化，Mouse/Paste/Resize 分支完整位移 |

## 5. 验证证据

### 5.1 `cargo check -p northhing-cli`
```
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing-cli
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
warning: `northhing-cli` (bin "northhing-cli") generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.31s
```
（唯一 warning 为既有的 `src\apps\cli\src\ui\question\mod.rs` 未使用 import，无新增 warning/error，0 error）

### 5.2 `cargo test -p northhing-cli`
```
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-cli
    Finished `test` profile [unoptimized + debuginfo] target(s) in 11.36s
     Running unittests src\main.rs (target\debug\deps\northhing_cli-dbd0e8af6897a04e.exe)

running 38 tests
test acp_cli::tests::cli_acp_permission_mode_converts_correctly ... ok
test acp_cli::tests::acp_help_text_contains_command_placeholder ... ok
test acp_cli::tests::external_acp_client_id_matches_expected_values ... ok
test acp_cli::tests::external_acp_client_display_name_matches_expected_values ... ok
test commands::tests::test_exact_match ... ok
test acp_cli::tests::render_command_joins_command_and_args ... ok
test acp_cli::tests::shell_command_appends_acp_subcommand ... ok
test acp_cli::tests::print_generic_config_contains_transport_stdio ... ok
test acp_cli::tests::print_zed_config_contains_agent_servers_key ... ok
test commands::tests::test_case_insensitive ... ok
test commands::tests::test_substring_match ... ok
test acp_cli::tests::external_acp_client_config_has_enabled_true ... ok
test commands::tests::test_match_does_not_mutate_specs ... ok
test commands::tests::test_multiple_substring_matches ... ok
test commands::tests::test_no_match ... ok
test commands::tests::test_prefix_match ... ok
test commands::tests::test_startup_command_specs_help ... ok
test ui::chat::state_split_tests::popup_stack_operations ... ok
test commands::tests::test_startup_command_specs_no_match ... ok
test commands::tests::test_startup_command_specs_prefix_match ... ok
test commands::tests::test_empty_query_returns_empty ... ok
test config::tests::default_cli_config_has_expected_values ... ok
test ui::chat::state_split_tests::accessor_methods_work_correctly ... ok
test commands::tests::test_mid_string_match ... ok
test ui::chat::state_split_tests::chatview_new_initializes_all_substructures ... ok
test ui::chat::state_split_tests::chatview_fields_accessible_after_refactor ... ok
test ui::chat::state_split_tests::clear_screen_resets_all_substructures ... ok
test ui::chat::state_split_tests::mouse_state_new_initializes_all_fields ... ok
test ui::chat::state_split_tests::popup_manager_new_initializes_all_states ... ok
test ui::chat::state_split_tests::selection_state_new_initializes_all_fields ... ok
test ui::model_config_form::state::tests::validate_allows_blank_api_key_in_edit_mode ... ok
test ui::model_config_form::state::tests::validate_blocks_blank_api_key_in_add_mode ... ok
test commands::tests::test_slash_only_returns_empty ... ok
test config::tests::config_toml_round_trip_preserves_values ... ok
test ui::theme::tests::eight_digit_hex_colors_are_supported ... ok
test keyring_keys::tests::typed_key_wins_over_keyring ... ok
test keyring_keys::tests::missing_keyring_entry_resolves_to_empty ... ok
test ui::theme::tests::builtin_themes_resolve_for_dark_and_light ... ok

test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 5.3 `node scripts/verify-rot-budget.mjs`
```
node scripts/verify-rot-budget.mjs
Rot budget verification passed (5 grep rules [unwrap_production=474/502, expect_production=940/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=264/400], 7 god-file rules checked across 1348 files).
```

## 6. 观察清单（既有事实保留，本波未擅动）

1. **三种错误/副作用风格并存**：`handle_key_event` 返回 `Result<Option<ChatExitReason>>`；`apply_exit_reason` 通过 8 个参数直接就地修改副作用；`handle_non_key_event` 返回 `Result<NonKeyEventOutcome>`。遵从 brief 铁律，不做风格统一。
2. **`apply_exit_reason` 8 参数**：保留原有参数签名 `(reason, this, chat_view, chat_state, session_id, rt_handle, should_quit, exit_reason)`，不封装临时 context 对象。
3. **Popup 事件分发架构**：目前仍为集中式 switch 处理（11 个 popup 状态轮询），未下沉至各 popup view 模块的 trait 方法。
4. **Ctrl+V Windows 剪贴板直读 workaround**：保留 arboard `Clipboard::new()` 处理以绕过 crossterm bracketed paste 问题 (#962)。

## 7. 偏离清单

- **偏离项**：0 项。严格遵循 brief 所有条款与 Global Constraints。
