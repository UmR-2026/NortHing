# W14-1c-3b Implementation Report — DeepReview 双 tracker reset seam（跨 crate，doc(hidden) 规则）

## 1. 改动清单 (Changed Files)

1. `src/crates/execution/agent-runtime/src/deep_review/budget/budget_state.rs`:
   - 在 `DeepReviewBudgetTracker` impl 上增加 `#[doc(hidden)] pub(crate) fn reset_for_test(&self)`，复用既有 `self.cleanup()` 逻辑清空 `turns` 并重置 `last_pruned_at`。
2. `src/crates/execution/agent-runtime/src/deep_review/queue.rs`:
   - 在 `DeepReviewQueueControlTracker` impl 上增加 `#[doc(hidden)] pub(crate) fn reset_for_test(&self)`，清空 `paused_tools`、`cancelled_tools`、`skip_optional_turns`。
3. `src/crates/execution/agent-runtime/src/deep_review/runtime_state.rs`:
   - 暴露 `#[doc(hidden)] pub fn reset_deep_review_budget_tracker_for_test()`。
   - 暴露 `#[doc(hidden)] pub fn reset_deep_review_queue_control_tracker_for_test()`。
   - 均附加规范注释 `/// 为 W14-1c 集成测试暴露；非公共 API`。
4. `src/crates/assembly/core/src/agentic/deep_review_policy.rs`:
   - 在 `#[cfg(test)]` 作用域下增加 `pub(crate) static TRACKER_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`，供跨测试模块互斥隔离全局单例修改，保障多线程并行与单线程串行测试全绿。
5. `src/crates/assembly/core/src/agentic/tools/implementations/code_review_tool/tests.rs`:
   - 在 3 个涉及 DeepReview tracker 的测试开头加测试锁及 tracker reset seam 调用。
6. `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_deep_review_tests.rs`:
   - 在 6 个涉及 tracker / queue control 的测试开头加测试锁及对应 tracker reset seam 调用。
7. `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_deep_review_tests_runtime.rs`:
   - 在 5 个涉及 tracker / queue control 的测试开头加测试锁及对应 tracker reset seam 调用。

---

## 2. 10 个测试归类与理由表 (Test Classification Table)

| # | 测试文件与行号 | 测试函数名 | 调用的 Reset Seam | 归类理由 |
|---|---|---|---|---|
| 1 | `code_review_tool/tests.rs:354` | `deep_review_submission_fills_concurrency_limited_from_runtime_tracker` | `reset_deep_review_budget_tracker_for_test()` | 测试通过 `record_deep_review_concurrency_cap_rejection` 注入并发受限记录，直接读写 `GLOBAL_DEEP_REVIEW_BUDGET_TRACKER`。 |
| 2 | `code_review_tool/tests.rs:395` | `deep_review_shared_context_diagnostics_stays_out_of_report` | `reset_deep_review_budget_tracker_for_test()` | 测试通过 `record_deep_review_shared_context_tool_use` 与 `deep_review_runtime_diagnostics_snapshot` 检验共享上下文调用量，数据由 `GLOBAL_DEEP_REVIEW_BUDGET_TRACKER` 维护。 |
| 3 | `code_review_tool/tests.rs:437` | `deep_review_submission_folds_capacity_skips_into_concurrency_limited_signal` | `reset_deep_review_budget_tracker_for_test()` | 测试调用 `record_deep_review_capacity_skip` 累加容量跳过指标，修改 `GLOBAL_DEEP_REVIEW_BUDGET_TRACKER`。 |
| 4 | `task_tool_deep_review_tests.rs:307` | `deep_review_capacity_queue_cancel_control_skips_waiting_reviewer` | `reset_deep_review_queue_control_tracker_for_test()` + `reset_deep_review_budget_tracker_for_test()` | 显式测试取消队列控制 `apply_deep_review_queue_control(..., Cancel)`，并占用 active reviewer 容量，操作两个 tracker。 |
| 5 | `task_tool_deep_review_tests.rs:346` | `deep_review_capacity_queue_records_one_runtime_wait_when_ready` | `reset_deep_review_budget_tracker_for_test()` | 测试 active reviewer 释放后排队恢复并断言 `queue_wait_count` 诊断，数据驻留在 `GLOBAL_DEEP_REVIEW_BUDGET_TRACKER`。 |
| 6 | `task_tool_deep_review_tests.rs:397` | `deep_review_capacity_queue_pause_does_not_expire_until_continued` | `reset_deep_review_queue_control_tracker_for_test()` + `reset_deep_review_budget_tracker_for_test()` | 测试队列暂停与恢复操作 `apply_deep_review_queue_control(..., Pause/Continue)` 并验证并发占位。 |
| 7 | `task_tool_deep_review_tests.rs:455` | `deep_review_capacity_queue_skip_optional_skips_optional_waiter` | `reset_deep_review_queue_control_tracker_for_test()` + `reset_deep_review_budget_tracker_for_test()` | 测试跳过可选审查员队列动作 `apply_deep_review_queue_control(..., SkipOptional)` 与 active reviewer 状态。 |
| 8 | `task_tool_deep_review_tests_runtime.rs:254` | `deep_review_provider_capacity_queue_retries_when_active_reviewer_frees_capacity` | `reset_deep_review_budget_tracker_for_test()` | 测试 `try_begin_deep_review_active_reviewer` 占位并在释放后触发 provider 队列重试，依赖 `GLOBAL_DEEP_REVIEW_BUDGET_TRACKER`。 |
| 9 | `task_tool_deep_review_tests_runtime.rs:317` | `deep_review_provider_retry_after_wait_ignores_active_reviewer_release` | `reset_deep_review_budget_tracker_for_test()` | 测试 active reviewer 释放不干扰 retry-after 等待，读写 `GLOBAL_DEEP_REVIEW_BUDGET_TRACKER`。 |
| 10 | `task_tool_deep_review_tests_runtime.rs:376` | `deep_review_provider_capacity_queue_cancel_control_skips_retry` | `reset_deep_review_queue_control_tracker_for_test()` + `reset_deep_review_budget_tracker_for_test()` | 测试 provider 队列取消动作 `apply_deep_review_queue_control(..., Cancel)` 并断言 `runtime_diagnostics` 诊断。 |
| 11* | `task_tool_deep_review_tests_runtime.rs:429` | `deep_review_provider_capacity_queue_pause_does_not_count_against_wait` | `reset_deep_review_queue_control_tracker_for_test()` + `reset_deep_review_budget_tracker_for_test()` | （补充覆盖）测试 provider 队列暂停/继续控制。 |

---

## 3. 复用侦察 (Reuse Scouting)

- `DeepReviewBudgetTracker` 已有 `pub fn cleanup(&self)` 清理 `turns.clear()` 与 `last_pruned_at`，`reset_for_test` 直接委托给既有 `self.cleanup()`，未新增冗余状态字段。
- `DeepReviewQueueControlTracker` 清空 `paused_tools` / `cancelled_tools` / `skip_optional_turns` 三个 `DashMap`，无额外结构分配。
- 跨 crate 访问直接复用既有依赖链 `northhing-core -> northhing-agent-runtime::deep_review`，不破坏任何分层边界。

---

## 4. 偏离说明 (Deviations)

- Brief 中提及 `task_tool_deep_review_tests_runtime.rs:375/428/460`，磁盘核查发现 line 376 为 cancel control 测试，line 429 为 pause control 测试（line 464 为其内部 continue 操作），同时同文件 line 254 / 317 亦包含 active reviewer tracker 占用逻辑。本实现对这些测试均一并施加了对应的 reset seam 与测试锁，确保无遗漏。
- 为满足并行运行 `cargo test -p northhing-core --features product-full deep_review` 与串行运行 `-- --test-threads=1` 均 100% 全绿，在 `northhing-core` 的 `#[cfg(test)]` 中定义了测试互斥锁 `TRACKER_TEST_LOCK`，防止多线程并行测试时互相清空其他线程正在断言的全局 tracker 数据。

---

## 5. 编译与设计层修复记录 (Compiler & Design Fixes)

- 本任务修改 0 个编译错误（E0xxx）。设计上严格遵循无条件 `pub` + `#[doc(hidden)]` + 规范注释，测试数未降，边界完整。

---

## 6. 验证证据 (Verification Evidence)

### 6.1 `cargo check -p northhing-agent-runtime`
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 48s
```

### 6.2 `cargo check -p northhing-core --features product-full`
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.16s
(0 error)
```

### 6.3 `cargo test -p northhing-agent-runtime`
```text
running 153 tests ... ok
test result: ok. 153 passed; 0 failed; 0 ignored; finished in 0.04s
(总计 261 passed; 0 failed)
```

### 6.4 `cargo test -p northhing-core --features product-full deep_review` (默认并行)
```text
running 57 tests
test agentic::agents::definitions::hidden::deep_review::tests::deep_review_agent_has_team_orchestration_tools ... ok
test agentic::agents::registry::tests::non_deep_review_builtin_subagents_default_to_primary ... ok
test agentic::agents::registry::tests::deep_review_family_defaults_to_fast ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_compression_signal_requires_completed_compression ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_full_depth_manifest_has_no_reduced_scope_signal ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_defaults_include_reduced_scope_reliability_signal ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_defaults_include_compression_contract_reliability_signal ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_invalid_evidence_pack_becomes_manifest_reliability_signal ... ok
test agentic::deep_review_policy::tests::only_missing_default_review_team_path_can_fallback_to_defaults ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_submission_defaults_missing_mode_to_deep ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_submission_fills_concurrency_limited_from_runtime_tracker ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_submission_marks_uninferable_packet_metadata_as_missing ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_submission_marks_existing_packet_metadata_as_reported ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_budget_tracker_caps_judge_per_turn ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_cancelled_reviewer_result_tells_parent_not_to_relaunch ... ok
test agentic::deep_review_policy::tests::compatibility_facade_preserves_deep_review_runtime_exports ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_legacy_manifest_without_scope_profile_has_no_reduced_scope_signal ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_submission_fills_runtime_reliability_signals ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_schema_accepts_reviewer_partial_output ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_concurrency_policy_blocks_reviewer_at_cap ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_reliability_contract_limit_uses_context_profile_policy ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_schema_requires_deep_review_fields ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_schema_accepts_reviewer_packet_fallback_metadata ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_schema_accepts_structured_reliability_signals ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_submission_infers_unique_reviewer_packet_from_manifest ... ok
test agentic::agents::registry::tests::built_in_deep_review_reviewers_are_marked_as_review_agents ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_policy_saturates_oversized_numeric_limits ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_concurrency_policy_returns_structured_cap_rejection ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_policy_caps_reviewer_and_judge_timeouts ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_concurrency_policy_blocks_judge_with_active_reviewers ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_retry_guidance_includes_budget_info ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_policy_allows_only_configured_team_members ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_retry_guidance_only_applies_to_initial_reviewer_timeout ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_retry_guidance_uses_manifest_policy_limit ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests_runtime::tests::deep_review_auto_retry_requires_review_team_opt_in ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests_runtime::tests::deep_review_auto_retry_opt_in_allows_guarded_admission ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests_runtime::tests::deep_review_provider_quota_error_is_not_capacity_skipped ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests_runtime::tests::deep_review_retry_accepts_reduced_partial_timeout_scope ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests_runtime::tests::deep_review_retry_rejects_broad_scope ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests_runtime::tests::deep_review_retry_rejects_missing_structured_coverage ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests_runtime::tests::deep_review_retry_rejects_non_queueable_capacity_reason ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests_runtime::tests::deep_review_retry_rejects_timeout_that_is_not_lowered ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests_runtime::tests::deep_review_retry_scope_prompt_prepend_bounds_review_files ... ok
test agentic::tools::tool_context_runtime::tests::call_runtime_tests::call_records_deep_review_read_file_measurement_without_touching_result ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_capacity_queue_pause_does_not_expire_until_continued ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_shared_context_diagnostics_stays_out_of_report ... ok
test agentic::tools::implementations::code_review_tool::tests::deep_review_submission_folds_capacity_skips_into_concurrency_limited_signal ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_capacity_queue_records_one_runtime_wait_when_ready ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_capacity_queue_cancel_control_skips_waiting_reviewer ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_capacity_queue_skip_optional_skips_optional_waiter ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_capacity_queue_waits_while_active_reviewer_is_running ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_capacity_queue_starts_later_batch_when_reviewer_capacity_frees ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests_runtime::tests::deep_review_provider_capacity_error_builds_capacity_skipped_payload_and_lowers_effective_cap ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests_runtime::tests::deep_review_provider_capacity_queue_cancel_control_skips_retry ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests_runtime::tests::deep_review_provider_capacity_queue_pause_does_not_count_against_wait ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests_runtime::tests::deep_review_provider_capacity_queue_retries_when_active_reviewer_frees_capacity ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests_runtime::tests::deep_review_provider_retry_after_wait_ignores_active_reviewer_release ... ok

test result: ok. 57 passed; 0 failed; 0 ignored; 0 measured; 1013 filtered out; finished in 2.29s
```

### 6.5 `cargo test -p northhing-core --features product-full deep_review -- --test-threads=1` (串行)
```text
test result: ok. 57 passed; 0 failed; 0 ignored; 0 measured; 1013 filtered out; finished in 2.33s
```

---

## 7. 状态 (Status)

DONE
