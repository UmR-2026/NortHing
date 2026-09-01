# Task Report — Audit I3：growth 蒸馏移出 turn 完成事件临界路径

## 1. 实现内容 (Implementation Summary)

在 `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs` 中将 `DialogTurnCompleted` 事件入队与 watchdog oneshot `tx.send` 信号发射调整至 `finalize_persisted_turn_in_workspace_if_needed(...).await` 之前执行：

1. 先判断并入队完成事件 `if let Some(ref completed_event) = workspace_turn_status.1 { event_queue.enqueue(...).await; }`。
2. 提前克隆 `turn_status_for_finalize = workspace_turn_status.0.clone()`。
3. 调用 `tx.send(workspace_turn_status)` 触发 watchdog 与外层 `select!` 立即响应完成态，释放 UI 等待。
4. 随后在该 spawned background task 内异步调用 `Self::finalize_persisted_turn_in_workspace_if_needed(...)` 执行 episode 日志追加、facts 蒸馏与 dream sweep（最长 30s LLM 调用）。
5. 添加英文注释说明重排语义与审计 I3 动机。

## 2. 复用侦察 (Reuse Survey)

- `finalize_persisted_turn_in_workspace_if_needed` 签名与调用点：
  - 定义：`src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs:273`
  - 主对话轮次调用点：`sub_handle_out.rs:357`（本任务重排目标）
  - Subagent 生命周期调用点：`subagent_orchestrator/so_lifecycle/lifecycle.rs` (3处) 及 `cleanup.rs` (1处)
- 测试 Harness 侦察：
  - `src/crates/assembly/core/src/agentic/coordination/tests/` 下包含 `turn_ports.rs`、`session_ports.rs` 及 `subagent_ports/`。

## 3. 测试勘察结论 (Test Investigation Conclusion)

- `coordination/tests/` 现有单测主要针对 `turn_ports`（取消收敛、watchdog 状态检测、并发限制截断、文本格式化）以及 `subagent_ports`。
- `sub_handle_out.rs` 执行路径深度依赖完整执行引擎（`execution_engine.execute_dialog_turn`）、会话上下文及工作区，无轻量 mock harness 能单步驱动至 `sub_handle_out` 完成并注入延迟。
- 家规 4 判定：本 diff 不修改 `select!` 分支结构、cancellation token 取消逻辑或超时 race 规则，仅将已持久化完成事件与 oneshot 发送前移。不硬造无断言价值的假单测。

## 4. 编译错误处置与分层定位 (Compiler Error Disposition & Layer Attribution)

- **E0433 (未指定 feature 导致 `agentic` / `ai` 模块缺失)**：
  - 定位：机制层（Cargo feature 配置）。`northhing-core` 的 `Cargo.toml` 中 `default = []` 为空以解耦下游依赖，`agentic` 模块由 `feature = "product-full"` 门控。在运行 `cargo test -p northhing-core` 时显式附带 `--features product-full` 或 `--all-features`。
- **业务代码编译**：`sub_handle_out.rs` 零编译错误、零类型/所有权违例。

## 5. 验证命令与输出原文 (Verification Commands & Raw Outputs)

### 5.1 `cargo check --workspace`

```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 33s
```

### 5.2 `cargo test -p northhing-core --features product-full --lib dialog_turn`

```text
running 13 tests
test kernel_facade::tests::test_dialog_turn_failed_auth_is_fatal ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_prepended_reminders_reject_unknown_kind ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_attachments_preserve_remote_image_context ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_dialog_turn_started_produces_state_and_phase ... ok
test kernel_facade::tests::test_dialog_turn_failed_no_category_is_fatal ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_attachments_reject_unknown_kind ... ok
test kernel_facade::tests::test_dialog_turn_failed_network_is_recoverable ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_prepended_reminders_preserve_session_message_kind ... ok
test agentic::coordination::subagent_orchestrator::so_dispatch::subagent_turn_id_tests::uses_parent_dialog_turn_id_when_parent_info_present ... ok
test agentic::execution::round_executor::tests::cancel_token_for_dialog_turn_returns_registered_token ... ok
test agentic::persistence::turn_io::tests::save_dialog_turn_updates_metadata_without_scanning_unrelated_turn_files ... ok
test agentic::persistence::turn_io::tests::concurrent_dialog_turn_saves_keep_metadata_counts_consistent ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_restore_dialog::start_dialog_turn_with_existing_context_persists_turn_and_snapshot ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 1039 filtered out; finished in 0.08s
```

### 5.3 `cargo test -p northhing-core --features product-full --lib coordination`

```text
running 52 tests
test agentic::coordination::a1_path::activation_tests::use_lightweight_actor_is_activated ... ok
test agentic::coordination::a1_path::mapping_tests::cancelled_maps_to_partial_timeout ... ok
test agentic::coordination::a1_path::mapping_tests::backend_error_maps_to_partial_timeout ... ok
test agentic::coordination::a1_path::mapping_tests::completed_maps_to_tool_result ... ok
test agentic::coordination::a1_path::mapping_tests::no_tool_matched_maps_to_partial_timeout ... ok
test agentic::coordination::a1_path::mapping_tests::partial_timeout_with_cancelled_reason_maps_to_cancelled ... ok
test agentic::coordination::a1_path::mapping_tests::partial_timeout_with_other_reason_maps_to_backend ... ok
test agentic::coordination::a1_path::mapping_tests::partial_timeout_with_timeout_reason_maps_to_timeout ... ok
test agentic::coordination::a1_path::mapping_tests::tool_result_invalid_json_leaves_structured_output_none ... ok
test agentic::coordination::a1_path::mapping_tests::tool_result_maps_to_completed ... ok
test agentic::coordination::a1_path::mapping_tests::tool_result_json_parses_to_structured_output ... ok
test agentic::coordination::a1_path::mapping_tests::timeout_maps_to_partial_timeout ... ok
test agentic::coordination::handoff::tests::handoff_error_cancelled_maps_to_northing_cancelled ... ok
test agentic::coordination::handoff::tests::handoff_error_coordinator_unavailable_maps_to_service ... ok
test agentic::coordination::subagent_orchestrator::so_dispatch::subagent_turn_id_tests::falls_back_to_orphan_session_name_when_parent_info_absent ... ok
test agentic::coordination::subagent_orchestrator::so_dispatch::subagent_turn_id_tests::orphan_fallback_preserves_session_name_verbatim ... ok
test agentic::coordination::handoff::tests::counter_first_call_succeeds_second_fails ... ok
test agentic::coordination::handoff::tests::handoff_impl_with_counter_shares_state ... ok
test agentic::coordination::handoff::tests::counter_clone_shares_state ... ok
test agentic::coordination::handoff::tests::handoff_impl_default_uses_fresh_counter ... ok
test agentic::coordination::handoff::tests::handoff_error_display_includes_turn_id ... ok
test agentic::coordination::handoff::tests::counter_distinct_turns_do_not_interfere ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_attachments_preserve_remote_image_context ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::remote_queue_policy_preserves_confirmation_boundary ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_prepended_reminders_reject_unknown_kind ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_prepended_reminders_preserve_session_message_kind ... ok
test agentic::coordination::handoff::tests::counter_reset_clears_entry ... ok
test agentic::coordination::handoff::tests::handoff_error_into_northing_error_maps_to_validation ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_attachments_reject_unknown_kind ... ok
test agentic::coordination::subagent_orchestrator::so_dispatch::subagent_turn_id_tests::uses_parent_dialog_turn_id_when_parent_info_present ... ok
test agentic::coordination::tests::turn_ports::background_subagent_display_text_is_concise ... ok
test agentic::coordination::tests::turn_ports::background_subagent_delivery_text_includes_background_task_id ... ok
test agentic::coordination::tests::turn_ports::clamps_subagent_max_concurrency_into_safe_range ... ok
test agentic::coordination::tests::turn_ports::conversation_coordinator_exposes_remote_runtime_ports ... ok
test agentic::coordination::tests::turn_ports::subagent_timeout_disable_clears_active_deadline ... ok
test agentic::coordination::tests::session_ports::subagent_session_config_preserves_registered_remote_workspace_identity ... ok
test agentic::coordination::tests::session_ports::hidden_btw_session_seeds_forked_listing_baselines ... ok
test agentic::coordination::tests::session_ports::agent_submission_create_session_preserves_creator_metadata ... ok
test agentic::coordination::tests::turn_ports::watchdog_does_not_detect_completed_turn ... ok
test agentic::coordination::tests::subagent_ports::tests_abort_exit::aborted_timeout_exit_persists_failed_and_returns_timeout ... ok
test agentic::coordination::tests::subagent_ports::tests_abort_exit::aborted_cancelled_exit_persists_and_clears_registry ... ok
test agentic::coordination::tests::subagent_ports::tests_parent_chain::subagent_parent_chain_propagates_through_nested_calls ... ok
test agentic::coordination::tests::subagent_ports::tests_timeout::subagent_timeout_returns_partial ... ok
test agentic::coordination::tests::subagent_ports::tests_error::subagent_error_propagates_to_result ... ok
test agentic::coordination::tests::subagent_ports::tests_concurrent::subagent_concurrent_cancellations_are_independent ... ok
test agentic::coordination::tests::subagent_ports::tests_cancel::subagent_cancel_propagates_to_result ... ok
test agentic::coordination::tests::subagent_ports::tests_cancel::subagent_cancel_takes_precedence_over_timeout ... ok
test agentic::coordination::tests::subagent_ports::tests_success::subagent_success_transmits_large_payload ... ok
test agentic::coordination::tests::subagent_ports::tests_success::subagent_success_completes_with_text ... ok
test agentic::coordination::tests::turn_ports::cancel_convergence_stale_cancel_does_not_emit ... ok
test agentic::coordination::tests::turn_ports::cancel_convergence_emits_terminal_event_when_turn_stuck ... ok
test agentic::coordination::tests::turn_ports::watchdog_detects_active_turn ... ok

test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 1000 filtered out; finished in 1.54s
```

## 6. 修改文件清单 (Modified Files)

- `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs` (411 lines, well below 800-line budget)

## 7. 自审发现 (Self-Review Findings)

1. `workspace_turn_status` 所有权在 `tx.send` 时 move，在其之前预先提取 `turn_status_for_finalize` 供后移的 `finalize_persisted_turn_in_workspace_if_needed` 使用，无多余分配或锁持有。
2. 禁区文件（`turn_persist.rs`、`distiller.rs`、`dream.rs`、`progress.md`）保持零修改。
3. 纯英文注释，无 emoji。

## 8. 疑虑 (Concerns)

无。
