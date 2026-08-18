# Task T2-2d Report: remote 栈子批 C2——agentic remote_file_delivery 通路删除

DONE

## 1. 任务完成概况

- **S1 生产者两处清理**：
  - `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_state.rs`：删除 `needs_computer_links_for_source` 提示词注入分支及相关 import。
  - `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs`：删除向 `context_vars` 注入 `TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY` 的分支及 import。
- **S2 context var 传递链清理**：
  - `src/crates/assembly/core/src/agentic/execution/turn_lifecycle.rs`：删除读取 `TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY` 与 `.with_remote_file_delivery_channel(...)` 链式调用及 import。
  - `src/crates/assembly/core/src/agentic/execution/` 下 10 个文件（`turn_lifecycle.rs`、`ai_message_build.rs`、`multimodal.rs`、`execution_engine.rs`、`loop_detection.rs`、`health_snapshot.rs`、`turn_finalize.rs`、`turn_tick.rs`、`token_pressure.rs`、`turn_init.rs`、`turn_main_loop.rs`）：清理未使用的 `TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY` import。
  - `src/crates/assembly/core/src/agentic/tools/tool_context_runtime/context_init.rs`：删除将 KEY 写入 `ToolUseContext.custom_data` 的分支及 import。
  - `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` 及 `dialog_turn/{workspace.rs, thread_goal.rs, session.rs, compaction.rs}`：删除未使用的三件套 import。
- **S3 PromptBuilderContext 字段清理**：
  - `src/crates/assembly/core/src/agentic/agents/prompt_builder/mod.rs`：删除 `pub remote_file_delivery_channel: bool` 字段、构造默认值初始化及 `with_remote_file_delivery_channel` 方法。
  - `src/crates/assembly/core/src/agentic/agents/prompt_builder/system_prompt.rs`：塌缩 `PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK` 替换逻辑为直接使用 workspace-relative 相对路径，删除 `user_workspace_relative_file_link` import。
  - `src/crates/assembly/core/src/agentic/agents/prompt_builder/tests.rs`：删除 `deep_research_report_link_uses_computer_scheme_for_remote_delivery` 测试。
- **S4 create_plan_tool 清理**：
  - `src/crates/assembly/core/src/agentic/tools/implementations/create_plan_tool.rs`：内联 `workspace_relative_user_link` helper，删除 `use_computer_link` 分支与 import，从返回值 JSON 中删除 `"computer_link"` 字段。
- **S5 删 remote_file_delivery.rs 整文件**：
  - 删除 `src/crates/assembly/core/src/agentic/remote_file_delivery.rs`（69 行）。
  - `src/crates/assembly/core/src/agentic/mod.rs` 删除 `pub(crate) mod remote_file_delivery;` 声明。
- **S6 boundary 规则核查**：
  - `scripts/core-boundaries/` 中无 `remote_file_delivery` 规则锚点，本项空转。
  - 运行 `node scripts/check-core-boundaries.mjs` 绿灯通过。

---

## 2. 验证原始输出

### 2.1 `cargo check --workspace` (MSVC)
```text
PS E:\agent-project\northing> & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 00s
```

### 2.2 `cargo check -p northhing` (MSVC)
```text
PS E:\agent-project\northing> & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 53.51s
```

### 2.3 `node scripts/check-core-boundaries.mjs`
```text
PS E:\agent-project\northing> node scripts/check-core-boundaries.mjs
Core boundary check passed.
```

### 2.4 Focused Tests (MSVC)
#### Prompt Builder:
```text
PS E:\agent-project\northing> & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib prompt_builder
     Running unittests src\lib.rs (target\debug\deps\northhing_core-b99735cdae331ad8.exe)

running 25 tests
test agentic::agents::prompt_builder::partitioned_loader::cache_identity_hash_is_stable_for_equivalent_inputs ... ok
test agentic::agents::prompt_builder::partitioned_loader::cache_identity_equality ... ok
test agentic::agents::prompt_builder::partitioned_loader::invalidate_system_prompt_only_caches_system ... ok
test agentic::agents::prompt_builder::partitioned_loader::invalidate_agent_prompt_clears_both_caches ... ok
test agentic::agents::prompt_builder::partitioned_loader::hash_string_is_deterministic ... ok
test agentic::agents::prompt_builder::partitioned_loader::loader_stores_template_name ... ok
test agentic::agents::prompt_builder::tests::exec_control_runtime_guidance_is_empty_for_remote_or_non_windows_hosts ... ok
test agentic::agents::prompt_builder::tests::exec_control_runtime_guidance_is_added_for_local_windows ... ok
test agentic::agents::prompt_builder::tests::exec_control_runtime_guidance_is_empty_when_exec_control_is_unavailable ... ok
test agentic::agents::prompt_builder::tests::local_exec_shell_runtime_guidance_is_added_for_powershell_shells ... ok
test agentic::agents::prompt_builder::tests::local_exec_shell_runtime_guidance_is_empty_for_non_powershell_shells ... ok
test agentic::agents::prompt_builder::tests::workspace_context_renders_related_directories_without_description ... ok
test agentic::agents::prompt_builder::tests::workspace_context_renders_related_directories ... ok
test agentic::agents::prompt_builder::tests::runtime_model_info_absent_when_model_name_is_none ... ok
test agentic::agents::prompt_builder::tests::prepended_reminders_omit_runtime_context_without_runtime_tool_needs ... ok
test agentic::agents::prompt_builder::tests::runtime_context_includes_workspace_info_for_workspace_tools ... ok
test agentic::agents::prompt_builder::tests::runtime_context_omits_workspace_root_for_remote_execution ... ok
test agentic::agents::prompt_builder::tests::runtime_context_includes_computer_use_info_only_when_needed ... ok
test agentic::agents::prompt_builder::tests::runtime_model_info_omits_window_and_output_when_not_set ... ok
test agentic::agents::prompt_builder::tests::runtime_model_info_injected_when_all_fields_present ... ok
test agentic::agents::prompt_builder::tests::deep_research_report_link_defaults_to_workspace_relative_path ... ok
test agentic::agents::prompt_builder::tests::builds_ordered_prepended_reminders_from_tool_listings_and_user_context ... ok
test agentic::agents::prompt_builder::partitioned_loader::agent_prompt_cache_hit_skips_rebuild ... ok
test agentic::agents::prompt_builder::partitioned_loader::system_prompt_cache_miss_after_tool_defs_change ... ok
test agentic::agents::prompt_builder::tests::runtime_context_includes_shell_info_when_exec_command_is_available ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 1015 filtered out; finished in 0.01s
```

#### Create Plan:
```text
PS E:\agent-project\northing> & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib create_plan
     Running unittests src\lib.rs (target\debug\deps\northhing_core-b99735cdae331ad8.exe)

running 1 test
test agentic::tools::implementations::create_plan_tool::tests::create_plan_is_collapsed_and_plan_mode_specific ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1039 filtered out; finished in 0.00s
```

#### Dialog Turn:
```text
PS E:\agent-project\northing> & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib dialog_turn
     Running unittests src\lib.rs (target\debug\deps\northhing_core-b99735cdae331ad8.exe)

running 13 tests
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_attachments_reject_unknown_kind ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_prepended_reminders_reject_unknown_kind ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_attachments_preserve_remote_image_context ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_prepended_reminders_preserve_session_message_kind ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_dialog_turn_started_produces_state_and_phase ... ok
test kernel_facade::tests::test_dialog_turn_failed_auth_is_fatal ... ok
test agentic::coordination::subagent_orchestrator::so_dispatch::subagent_turn_id_tests::uses_parent_dialog_turn_id_when_parent_info_present ... ok
test kernel_facade::tests::test_dialog_turn_failed_network_is_recoverable ... ok
test kernel_facade::tests::test_dialog_turn_failed_no_category_is_fatal ... ok
test agentic::execution::round_executor::tests::cancel_token_for_dialog_turn_returns_registered_token ... ok
test agentic::persistence::turn_io::tests::save_dialog_turn_updates_metadata_without_scanning_unrelated_turn_files ... ok
test agentic::persistence::turn_io::tests::concurrent_dialog_turn_saves_keep_metadata_counts_consistent ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_restore_dialog::start_dialog_turn_with_existing_context_persists_turn_and_snapshot ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 1027 filtered out; finished in 0.09s
```

#### Coordination:
```text
PS E:\agent-project\northing> & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib coordination
     Running unittests src\lib.rs (target\debug\deps\northhing_core-b99735cdae331ad8.exe)

running 52 tests
test agentic::coordination::a1_path::activation_tests::use_lightweight_actor_is_activated ... ok
test agentic::coordination::a1_path::mapping_tests::completed_maps_to_tool_result ... ok
test agentic::coordination::a1_path::mapping_tests::cancelled_maps_to_partial_timeout ... ok
test agentic::coordination::a1_path::mapping_tests::partial_timeout_with_timeout_reason_maps_to_timeout ... ok
test agentic::coordination::a1_path::mapping_tests::no_tool_matched_maps_to_partial_timeout ... ok
test agentic::coordination::a1_path::mapping_tests::partial_timeout_with_cancelled_reason_maps_to_cancelled ... ok
test agentic::coordination::a1_path::mapping_tests::partial_timeout_with_other_reason_maps_to_backend ... ok
test agentic::coordination::handoff::tests::handoff_error_display_includes_turn_id ... ok
test agentic::coordination::handoff::tests::counter_clone_shares_state ... ok
test agentic::coordination::handoff::tests::counter_first_call_succeeds_second_fails ... ok
test agentic::coordination::handoff::tests::counter_reset_clears_entry ... ok
test agentic::coordination::a1_path::mapping_tests::tool_result_maps_to_completed ... ok
test agentic::coordination::a1_path::mapping_tests::tool_result_json_parses_to_structured_output ... ok
test agentic::coordination::a1_path::mapping_tests::backend_error_maps_to_partial_timeout ... ok
test agentic::coordination::a1_path::mapping_tests::timeout_maps_to_partial_timeout ... ok
test agentic::coordination::handoff::tests::handoff_error_cancelled_maps_to_northing_cancelled ... ok
test agentic::coordination::handoff::tests::handoff_error_coordinator_unavailable_maps_to_service ... ok
test agentic::coordination::a1_path::mapping_tests::tool_result_invalid_json_leaves_structured_output_none ... ok
test agentic::coordination::handoff::tests::handoff_error_into_northing_error_maps_to_validation ... ok
test agentic::coordination::handoff::tests::handoff_impl_default_uses_fresh_counter ... ok
test agentic::coordination::handoff::tests::handoff_impl_with_counter_shares_state ... ok
test agentic::coordination::handoff::tests::counter_distinct_turns_do_not_interfere ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_attachments_preserve_remote_image_context ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_attachments_reject_unknown_kind ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_prepended_reminders_reject_unknown_kind ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::remote_queue_policy_preserves_confirmation_boundary ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_prepended_reminders_preserve_session_message_kind ... ok
test agentic::coordination::subagent_orchestrator::so_dispatch::subagent_turn_id_tests::falls_back_to_orphan_session_name_when_parent_info_absent ... ok
test agentic::coordination::subagent_orchestrator::so_dispatch::subagent_turn_id_tests::orphan_fallback_preserves_session_name_verbatim ... ok
test agentic::coordination::subagent_orchestrator::so_dispatch::subagent_turn_id_tests::uses_parent_dialog_turn_id_when_parent_info_present ... ok
test agentic::coordination::tests::turn_ports::background_subagent_display_text_is_concise ... ok
test agentic::coordination::tests::turn_ports::background_subagent_delivery_text_includes_background_task_id ... ok
test agentic::coordination::tests::turn_ports::clamps_subagent_max_concurrency_into_safe_range ... ok
test agentic::coordination::tests::turn_ports::conversation_coordinator_exposes_remote_runtime_ports ... ok
test agentic::coordination::tests::turn_ports::subagent_timeout_disable_clears_active_deadline ... ok
test agentic::coordination::tests::session_ports::subagent_session_config_preserves_registered_remote_workspace_identity ... ok
test agentic::coordination::tests::session_ports::agent_submission_create_session_preserves_creator_metadata ... ok
test agentic::coordination::tests::session_ports::hidden_btw_session_seeds_forked_listing_baselines ... ok
test agentic::coordination::tests::turn_ports::watchdog_does_not_detect_completed_turn ... ok
test agentic::coordination::tests::subagent_ports::tests_abort_exit::aborted_cancelled_exit_persists_and_clears_registry ... ok
test agentic::coordination::tests::subagent_ports::tests_abort_exit::aborted_timeout_exit_persists_failed_and_returns_timeout ... ok
test agentic::coordination::tests::subagent_ports::tests_timeout::subagent_timeout_returns_partial ... ok
test agentic::coordination::tests::subagent_ports::tests_parent_chain::subagent_parent_chain_propagates_through_nested_calls ... ok
test agentic::coordination::tests::subagent_ports::tests_error::subagent_error_propagates_to_result ... ok
test agentic::coordination::tests::subagent_ports::tests_concurrent::subagent_concurrent_cancellations_are_independent ... ok
test agentic::coordination::tests::subagent_ports::tests_cancel::subagent_cancel_propagates_to_result ... ok
test agentic::coordination::tests::subagent_ports::tests_cancel::subagent_cancel_takes_precedence_over_timeout ... ok
test agentic::coordination::tests::subagent_ports::tests_success::subagent_success_completes_with_text ... ok
test agentic::coordination::tests::subagent_ports::tests_success::subagent_success_transmits_large_payload ... ok
test agentic::coordination::tests::turn_ports::cancel_convergence_stale_cancel_does_not_emit ... ok
test agentic::coordination::tests::turn_ports::cancel_convergence_emits_terminal_event_when_turn_stuck ... ok
test agentic::coordination::tests::turn_ports::watchdog_detects_active_turn ... ok

test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 988 filtered out; finished in 1.52s
```

### 2.5 S5 归零复核
```text
PS E:\agent-project\northing> rg -n "remote_file_delivery|computer_link|TOOL_CONTEXT_REMOTE_FILE_DELIVERY|needs_computer_links" src --glob "*.rs"
(0 hits)

PS E:\agent-project\northing> rg -n "remote_file_delivery|computer_link|computer://|TOOL_CONTEXT_REMOTE_FILE_DELIVERY|needs_computer_links" src/crates/assembly --glob "*.rs"
(0 hits)
```
注：`src/crates/services/services-integrations` 下保留的 `computer://` 属 C3（remote_workspace_resolver）范围，未越界。

### 2.6 `git diff --stat src/`
```text
 .../core/src/agentic/agents/prompt_builder/mod.rs  |  9 ---
 .../agentic/agents/prompt_builder/system_prompt.rs | 11 +---
 .../src/agentic/agents/prompt_builder/tests.rs     | 15 -----
 .../core/src/agentic/coordination/coordinator.rs   |  3 -
 .../agentic/coordination/dialog_turn/compaction.rs |  3 -
 .../agentic/coordination/dialog_turn/session.rs    |  3 -
 .../coordination/dialog_turn/sub_handle_out.rs     |  7 ---
 .../coordination/dialog_turn/sub_handle_state.rs   | 10 +---
 .../coordination/dialog_turn/thread_goal.rs        |  3 -
 .../agentic/coordination/dialog_turn/workspace.rs  |  3 -
 .../core/src/agentic/execution/ai_message_build.rs |  1 -
 .../core/src/agentic/execution/execution_engine.rs |  1 -
 .../core/src/agentic/execution/health_snapshot.rs  |  1 -
 .../core/src/agentic/execution/loop_detection.rs   |  1 -
 .../core/src/agentic/execution/multimodal.rs       |  1 -
 .../core/src/agentic/execution/token_pressure.rs   |  1 -
 .../core/src/agentic/execution/turn_finalize.rs    |  1 -
 .../core/src/agentic/execution/turn_init.rs        |  1 -
 .../core/src/agentic/execution/turn_lifecycle.rs   |  9 +--
 .../core/src/agentic/execution/turn_main_loop.rs   |  1 -
 .../core/src/agentic/execution/turn_tick.rs        |  1 -
 src/crates/assembly/core/src/agentic/mod.rs        |  1 -
 .../core/src/agentic/remote_file_delivery.rs       | 69 ----------------------
 .../tools/implementations/create_plan_tool.rs      | 21 +++----
 .../tools/tool_context_runtime/context_init.rs     |  9 ---
 25 files changed, 13 insertions(+), 173 deletions(-)
```

---

## 3. 行数与约束对账

- **Contracts 层**：零改动。`DialogTriggerSource::RemoteRelay|Bot` 变体保留，`subagent_ports.rs:113` 的 `unwrap_or(Bot)` 保持原样。
- **SSH 语义**：`remote_connection_id`/`remote_ssh*` 零改动。
- **工作区纪律**：未 commit、未 push；未触碰 `memory/`、`.graph/`、`.opencode/`、`.superpowers/sdd/` 其它 task-* 文件及前端文件。
- **遗留疑虑 (Caveats)**：无。

---

## 4. Fix Round 1 (修复非任务格式化改动)

### 4.1 还原项
对照 `git diff 9c14d22 -- <file>` 逐处核实并还原了 3 处审查指出的格式化重排，确保 diff 严格仅包含 brief 指定的删除/修改：
1. `src/crates/assembly/core/src/agentic/coordination/dialog_turn/session.rs:216-220` — 还原 memory-reminder match 调用的多行格式。
2. `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs:405-410` — 还原 `active_turn_tasks.insert(...)` 的多行格式。
3. `src/crates/assembly/core/src/agentic/execution/execution_engine.rs:150-156` — 还原 `info!` 调用的多行格式。
4. 顺带复核并还原了 `system_prompt.rs`、`tests.rs`、`create_plan_tool.rs` 中因自动格式化引入的无意行重排。

### 4.2 验证原始输出

#### `cargo check --workspace` (MSVC)
```text
PS E:\agent-project\northing> & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 54.06s
```

#### `node scripts/check-core-boundaries.mjs`
```text
PS E:\agent-project\northing> node scripts/check-core-boundaries.mjs
Core boundary check passed.
```

#### `cargo test -p northhing-core --features product-full --lib dialog_turn` (MSVC)
```text
PS E:\agent-project\northing> & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib dialog_turn
     Running unittests src\lib.rs (target\debug\deps\northhing_core-b99735cdae331ad8.exe)

running 13 tests
test kernel_facade::tests::test_agentic_event_to_dtos_dialog_turn_started_produces_state_and_phase ... ok
test kernel_facade::tests::test_dialog_turn_failed_auth_is_fatal ... ok
test kernel_facade::tests::test_dialog_turn_failed_no_category_is_fatal ... ok
test kernel_facade::tests::test_dialog_turn_failed_network_is_recoverable ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_attachments_reject_unknown_kind ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_attachments_preserve_remote_image_context ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_prepended_reminders_reject_unknown_kind ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_prepended_reminders_preserve_session_message_kind ... ok
test agentic::coordination::subagent_orchestrator::so_dispatch::subagent_turn_id_tests::uses_parent_dialog_turn_id_when_parent_info_present ... ok
test agentic::execution::round_executor::tests::cancel_token_for_dialog_turn_returns_registered_token ... ok
test agentic::persistence::turn_io::tests::save_dialog_turn_updates_metadata_without_scanning_unrelated_turn_files ... ok
test agentic::persistence::turn_io::tests::concurrent_dialog_turn_saves_keep_metadata_counts_consistent ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_restore_dialog::start_dialog_turn_with_existing_context_persists_turn_and_snapshot ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 1027 filtered out; finished in 0.08s
```
