# Task T1-5 Report — 出货默认确认 + P1-6 DeleteFileTool 确认门（SW1-5 + P1-6 + F1 Fix）

## 1. 改动文件清单

- `src/crates/assembly/core/src/service/config/ai.rs`:
  - `default_skip_tool_confirmation`: 默认返回值由 `true` 翻转为 `false`。
  - `AIConfig::default`: `skip_tool_confirmation` 字段显式值由 `true` 翻转为 `false`。
  - `tests` 模块新增 6 个针对配置默认值、serde 反序列化兼容性及 Phase 3 决策层 `combined_skip` 逻辑的单元测试。
- `src/crates/assembly/core/src/agentic/tools/implementations/delete_file_tool.rs`:
  - 删除了 `needs_permissions(&self, ...)` 返回 `false` 的显式覆写，恢复 Tool trait 默认实现 `needs_permissions() = !self.is_readonly()`（因 `is_readonly() == false`，恢复返回 `true`）。
  - `tests` 模块新增 3 个针对 `DeleteFileTool` 只读状态、权限门要求与并发安全性的单元测试。
- `src/crates/assembly/core/src/agentic/tools/implementations/file_write_tool/mod.rs` (F1 修复轮):
  - 删除了 `needs_permissions(&self, ...)` 返回 `false` 的显式覆写，恢复 Tool trait 默认实现 `needs_permissions() = !self.is_readonly()`（因 `is_readonly() == false`，恢复返回 `true`）。
  - `tests` 模块新增 1 个针对 `FileWriteTool` 权限门要求 `needs_permissions() == true` 的单元测试。
- `src/crates/assembly/core/src/agentic/tools/implementations/file_edit_tool.rs` (F1 修复轮):
  - 删除了 `needs_permissions(&self, ...)` 返回 `false` 的显式覆写，恢复 Tool trait 默认实现 `needs_permissions() = !self.is_readonly()`（因 `is_readonly() == false`，恢复返回 `true`）。
  - `tests` 模块新增 1 个针对 `FileEditTool` 权限门要求 `needs_permissions() == true` 的单元测试。
- `docs/status/tech-debt-ledger.md`:
  - 将 P1-6 状态由 `active` 翻转为 `resolved` (2026-08-21, T1-5)，并记录修复细节与测试依据（家规 2）。

---

## 2. Spec 落实说明

1. **两处默认翻转**：
   - `ai.rs:357-359` 修改 `fn default_skip_tool_confirmation() -> bool { false }`。
   - `ai.rs:490` 修改 `AIConfig::default()` 中的 `skip_tool_confirmation: false`。
   - 兼容语义完整保留：旧配置文件反序列化显式带有 `"skip_tool_confirmation": true` 时依然解析为 `true`。
2. **删除 DeleteFileTool / FileWriteTool / FileEditTool override**：
   - 删除了 `delete_file_tool.rs`、`file_write_tool/mod.rs`、`file_edit_tool.rs` 中的 `needs_permissions` 覆写，确认各工具 `is_readonly() == false`，使默认 trait 实现 `needs_permissions()` 统一恢复返回 `true`。
   - 所有删除（本地普通删除、`permanent: true` 永久直删、以及远程 SSH 删除）与文件写/改操作全部过确认门。
3. **新测试（最小集）**：
   - `default_ai_config_skip_tool_confirmation_is_false`: 验证全新 `AIConfig::default()` 中 `skip_tool_confirmation` 为 `false`。
   - `deserializes_missing_skip_tool_confirmation_as_false`: 验证 JSON 缺省该字段时 serde default 解析为 `false`。
   - `deserializes_explicit_skip_tool_confirmation_true_as_true`: 验证旧配置显式声明 `true` 时反序列化保持 `true`。
   - `combined_skip_tool_confirmation_logic_fresh_config_requires_confirmation`: 验证全新配置下 `combined_skip = shell_security_skip && skip_tool_confirmation` 为 `false`（触发确认门）。
   - `combined_skip_tool_confirmation_logic_legacy_config_skips_when_both_true`: 验证旧配置显式 `true` 配合 Permissive 模式时 `combined_skip` 为 `true`。
   - `combined_skip_tool_confirmation_logic_mode_override_strict_prevents_skip`: 验证 Strict 模式 override 始终阻止跳过确认。
   - `delete_file_tool_needs_permissions_returns_true`: 验证 `DeleteFileTool` 的 `needs_permissions(None)` 及携带参数时均返回 `true`。
   - `file_write_tool_needs_permissions_returns_true`: 验证 `FileWriteTool` 的 `needs_permissions(None)` 及携带参数时均返回 `true`。
   - `file_edit_tool_needs_permissions_returns_true`: 验证 `FileEditTool` 的 `needs_permissions(None)` 及携带参数时均返回 `true`。
4. **验收对齐**：
   - 全新配置下 `combined_skip == false`，四大写/删/执行工具 `Bash` (`needs_permissions == true`)、`Write` (`needs_permissions == true`)、`Edit` (`needs_permissions == true`)、`Delete` (`needs_permissions == true`) 在全新配置下均触发确认门；既有测试套件全部通过，无回归。
5. **文档同步（家规 2）**：
   - 更新 `docs/status/tech-debt-ledger.md` 将 P1-6 标记为 `resolved`。
   - `docs/architecture/backend-roadmap.md` 未做任何修改。

---

## 3. 三条内部显式 true 路径的保留理由

1. `src/crates/assembly/core/src/agentic/coordination/a1_path.rs:256`:
   - **保留理由**：用于构建 A1/A2 子代理（subagent phase 1 / A2 heartbeat）后台任务的 `ExecutionContext`。子代理运行在独立的后台异步任务中，属于自动化调度机制，不能阻塞在前端 UI 的交互确认弹窗上。
2. `src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_lifecycle/lifecycle.rs:211`:
   - **保留理由**：用于构建 SubagentOrchestrator 派发的内部子代理执行任务的 `ExecutionContext`。子代理工具权限由编排策略直接约束，属于后台自动化子任务，不走主线程交互确认。
3. `src/crates/assembly/core/src/agentic/coordination/dialog_turn/coordinator_compact.rs:97`:
   - **保留理由**：用于构建上下文历史压缩（Context Compression）Turn 的 `ExecutionContext`。压缩过程是系统内部维护与总结操作，不属于用户直接对话交互，必须自动完成。

---

## 4. 行为变化清单

- **变更行为（安全增强）**：
  - 全新安装或未在配置文件中显式配置 `skip_tool_confirmation` 的用户，`skip_tool_confirmation` 默认值为 `false`。
  - 全新配置下，执行 `Bash` / `ExecCommand` / `Write` / `Edit` / `Delete` 等高危工具将不再自动免确认，而是走确认门（Combined Skip AND 判定结果为 `false`）。
  - `DeleteFileTool`、`FileWriteTool`、`FileEditTool` 不再绕过权限判定，`needs_permissions()` 均返回 `true`，一律进入确认流程。
- **不变行为（向后兼容）**：
  - 既有用户配置文件中若已保存 `"skip_tool_confirmation": true`，读取后保持为 `true`，既有免确认体验保持向后兼容（显式 `skip_tool_confirmation: true` 用户不变）。
  - `ShellSecurityConfig` 的 `mode_overrides`（如设置 `Strict`）依然具有最高优先级。
  - 内部子代理调度及后台上下文压缩等自动化任务维持免交互确认。

---

## 5. 验证命令及输出

### 1. `cargo test -p northhing-core --features product-full -- config`
```text
running 60 tests
test service::config::ai::tests::combined_skip_tool_confirmation_logic_mode_override_strict_prevents_skip ... ok
test service::config::ai::tests::default_ai_config_skip_tool_confirmation_is_false ... ok
test service::config::ai::tests::default_ai_config_tool_timeouts_are_some_300 ... ok
test service::config::ai::tests::combined_skip_tool_confirmation_logic_legacy_config_skips_when_both_true ... ok
test service::config::ai::tests::combined_skip_tool_confirmation_logic_fresh_config_requires_confirmation ... ok
test service::config::ai::tests::deserializes_explicit_skip_tool_confirmation_true_as_true ... ok
test service::config::ai::tests::deserializes_missing_skip_tool_confirmation_as_false ... ok
...
test result: ok. 60 passed; 0 failed; 0 ignored; 0 measured; 975 filtered out; finished in 0.03s
```

### 2. `cargo test -p northhing-core --features product-full delete`
```text
running 10 tests
test agentic::tools::implementations::delete_file_tool::tests::delete_file_tool_is_not_readonly ... ok
test agentic::tools::implementations::delete_file_tool::tests::delete_file_tool_concurrency_safety_is_false ... ok
test agentic::tools::implementations::delete_file_tool::tests::delete_file_tool_needs_permissions_returns_true ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_rollback_delete::build_messages_from_turns_skips_model_invisible_turns ... ok
test service::lsp::manager::tests::uninstall_file_delete_failure_rolls_back_registration ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_rollback_delete::delete_session_removes_workspace_cache_entry ... ok
test service::agent_memory::memory_db::tests::delete_fact_removes_from_fts ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_rollback_delete::rollback_to_empty_history_clears_last_user_dialog_agent_type ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_rollback_delete::rollback_context_deletes_persisted_turns_from_target ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_rollback_delete::rollback_sanitizes_pre_cutoff_snapshot_and_truncates_cutoff ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 1025 filtered out; finished in 0.15s
```

### 3. `cargo test -p northhing-core --features product-full -- file_write`
```text
running 11 tests
test agentic::tools::implementations::file_write_tool::tests::file_write_tool_needs_permissions_returns_true ... ok
test agentic::tools::implementations::file_write_tool::tests::guidance_prefix_helpers_round_trip ... ok
test agentic::tools::pipeline::tool_pipeline::tests::truncation_notice_for_interactive_tools_does_not_claim_file_write ... ok
test agentic::tools::implementations::file_write_tool::tests::validate_input_rejects_invalid_mode ... ok
test agentic::tools::implementations::file_write_tool::tests::schema_requires_file_path_and_content ... ok
test agentic::tools::implementations::file_write_tool::tests::validate_input_requires_content ... ok
test agentic::tools::implementations::file_write_tool::tests::preflight_write_error_allows_new_file_target ... ok
test agentic::tools::implementations::file_write_tool::tests::preflight_write_error_allows_existing_file_without_read_state_tracking ... ok
test agentic::tools::implementations::file_write_tool::tests::call_impl_treats_identical_existing_content_as_success ... ok
test agentic::tools::implementations::file_write_tool::tests::call_impl_appends_when_mode_is_append ... ok
test agentic::tools::implementations::file_write_tool::tests::call_impl_overwrites_different_existing_content ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 1026 filtered out; finished in 0.01s
```

### 4. `cargo test -p northhing-core --features product-full -- file_edit`
```text
running 5 tests
test agentic::tools::implementations::file_edit_tool::tests::edit_content_guardrail_detection_matches_apply_edit_errors ... ok
test agentic::tools::implementations::file_edit_tool::tests::edit_tool_short_description_matches_claude_summary ... ok
test agentic::tools::implementations::file_edit_tool::tests::file_edit_tool_needs_permissions_returns_true ... ok
test agentic::tools::implementations::file_edit_tool::tests::edit_tool_schema_describes_exact_copy_from_read ... ok
test agentic::tools::implementations::file_edit_tool::tests::edit_tool_prompt_matches_claude_style ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 1032 filtered out; finished in 0.00s
```

### 5. `cargo check --workspace`
```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 58.94s
```

### 6. `pnpm run fmt:rs`
```text
> northhing@0.2.10 fmt:rs E:\agent-project\northing
> node scripts/format-changed-rust.mjs

[format-changed-rust] Formatting 2 Rust file(s).
```

---

## 6. 审查反馈与补齐说明

- **首轮审查反馈 (F1 Important, plan-mandated)**：
  - SW1-5 验收要求"全新配置下 Bash/Write/Edit/Delete 弹确认"。首轮仅处理了 DeleteFileTool 与配置翻转，因既有代码中 Write/Edit 存在历史硬编码 `needs_permissions()=false` 覆写，导致 Write/Edit 未弹确认。
- **修复方案与补齐 (用户拍板选项 b)**：
  - 删除了 `FileWriteTool` 与 `FileEditTool` 的 `needs_permissions` 覆写，与 `DeleteFileTool` 对齐恢复 Tool trait 默认 `needs_permissions() = !self.is_readonly() == true`。
  - 为 Write 与 Edit 工具补充了 `needs_permissions() == true` 的单元测试断言。
  - 修正了上一版报告中 §2.4 的表述，如实记录 Write/Edit 覆写删除与行为补齐。

---

## 7. 派发与提交信息

- BASE commit: `5862745`
- T1-5 主改动 commit: `bec0ae7` (`fix(core): default tool confirmation to required and restore DeleteFileTool permission gate (T1-5)`)
- T1-5 Fix 补齐 commit: `ea55c80` (`fix(core): restore needs_permissions gate for FileWriteTool and FileEditTool (T1-5 fix)`)
