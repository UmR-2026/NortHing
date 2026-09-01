# Task Report — W2-3：r2#6 prepare_turn 历史恢复失败按会话历史分级日志

## 1. 实现内容 (Implementation)

在 `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_in.rs` 的 `prepare_turn` 阶段，修改了 `history restore` 的 `Err(e)` 匹配分支：
- 若 `session.dialog_turn_ids.is_empty()` 为 `true`（表示新会话无历史 turn），维持原有 `debug!` 输出。
- 若 `session.dialog_turn_ids` 非空（表示已有持久化 turn 记录但恢复失败），提升日志级别至 `warn!`，并输出持久化 turn 的数量 `session.dialog_turn_ids.len()` 及 `turn proceeds with partial context` 的效果提示。

代码变更对比：
```rust
Err(e) => {
    if session.dialog_turn_ids.is_empty() {
        debug!(
            "Failed to restore session history (may be new session): session_id={}, error={}",
            session_id, e
        );
    } else {
        warn!(
            "Failed to restore session history for session with {} persisted turns; turn proceeds with partial context: session_id={}, error={}",
            session.dialog_turn_ids.len(),
            session_id, e
        );
    }
}
```

## 2. 复用侦察 (Reuse Recon)

- `warn!` 宏：文件头部第 25 行已导入 `use tracing::{debug, info, warn};`，直接可用。
- `session.dialog_turn_ids`：第 124 行已实证该字段存在并支持 `.is_empty()` 和 `.len()` 方法。
- `session` 作用域：`session` 在第 35 行绑定，在第 180 行 `ctx.session = Some(session);` 转移之前在 `Err` 臂中完全有效且未被 move，零额外的 IO 或存储查询。

## 3. Banner-不采纳取舍记录 (Design Trade-off Note)

审计项提到 `(or a SystemError/banner)` 的备选方案。
采纳编排者裁定：**不采纳 UI Banner / SystemError**。本项属 Minor 级观测性与日志分级修复。会话历史恢复失败时模型降级为部分上下文回答，属于后端观测性增强，直接在日志中以 `warn!` 级别体现即可，弹 UI Banner 过于侵入且过度响应。

## 4. 测试与输出原文 (Verification Output)

### Command 1: `cargo check --workspace`

```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
warning: private item shadows public glob re-export
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_loop.rs:295:9

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_input.rs:191:9

warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:63:13

warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:35:13

warning: unused variable: `port`
   --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser.rs:137:13

warning: unused variable: `actions`
  --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser_telemetry.rs:26:13

warning: unused variable: `deep_review_subagent_role`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:80:5

warning: unused variable: `is_retry`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:84:5

warning: unused variable: `suppress_session_title_generation`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs:34:13

warning: unused variable: `turn_index`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:39:13

warning: unused variable: `workspace_turn_status`
   --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:373:17

warning: unused variable: `active_counter`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:67:13

warning: unused variable: `ws`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:236:36

warning: unused variable: `last_mentioned_at`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:291:80

warning: unused variable: `at_ms`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:743:85

warning: unused variable: `ws`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db\dream.rs:17:36

warning: unused variable: `params`
   --> src\crates\assembly\core\src\service\mcp\server\manager\interaction.rs:104:9

warning: `northhing-core` (lib) generated 18 warnings (run `cargo fix --lib -p northhing-core` to apply 17 suggestions)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
warning: unused imports: `QuestionData` and `QuestionOption`
  --> src\apps\cli\src\ui\question\mod.rs:15:33

warning: `northhing-cli` (bin "northhing-cli") generated 1 warning
warning: `northhing` (bin "northhing") generated 37 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 42.36s
```

### Command 2: `cargo test -p northhing-core --features product-full --lib dialog_turn`

```text
   Compiling northhing-services-core v0.2.10 (E:\agent-project\northing\src\crates\services\services-core)
   Compiling northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
   Compiling northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 06s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)

running 13 tests
test kernel_facade::tests::test_dialog_turn_failed_no_category_is_fatal ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_dialog_turn_started_produces_state_and_phase ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_attachments_preserve_remote_image_context ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_prepended_reminders_reject_unknown_kind ... ok
test kernel_facade::tests::test_dialog_turn_failed_auth_is_fatal ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_attachments_reject_unknown_kind ... ok
test agentic::coordination::scheduler::scheduler_turn::turn_submit::tests::agent_dialog_turn_prepended_reminders_preserve_session_message_kind ... ok
test kernel_facade::tests::test_dialog_turn_failed_network_is_recoverable ... ok
test agentic::coordination::subagent_orchestrator::so_dispatch::subagent_turn_id_tests::uses_parent_dialog_turn_id_when_parent_info_present ... ok
test agentic::execution::round_executor::tests::cancel_token_for_dialog_turn_returns_registered_token ... ok
test agentic::persistence::turn_io::tests::save_dialog_turn_updates_metadata_without_scanning_unrelated_turn_files ... ok
test agentic::persistence::turn_io::tests::concurrent_dialog_turn_saves_keep_metadata_counts_consistent ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_restore_dialog::start_dialog_turn_with_existing_context_persists_turn_and_snapshot ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 1039 filtered out; finished in 0.08s
```

## 5. 文件清单 (File Manifest)

- `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_in.rs` (修改代码)
- `.superpowers/sdd/w2-3-r2-6-restore-warn-report.md` (报告)

## 6. 自审发现 (Self-Review Findings)

1. 日志字符串全英文，无 emoji。
2. 遵守 Spec 限制，仅复用已有作用域内的 `session.dialog_turn_ids`。
3. `cargo check --workspace` 与 `cargo test -p northhing-core --features product-full --lib dialog_turn` 全部 100% 通过。
4. 未修改 `.superpowers/sdd/progress.md`。

## 7. 疑虑 (Concerns)

无。
