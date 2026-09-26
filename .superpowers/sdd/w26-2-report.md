# W26-2 Report — P2-5 失败留痕：DialogTurnData + error_detail 字段级方案

- BASE（实际采用）：`831094c`（第一个 commit 前 `git log --oneline -1` 实测 = 831094c，兄弟单当时未落 commit）
- 代码 commit（TIP1）：`f54a3fe`
- report/allowlist 回填 commit：TIP2（本文件所在提交，见 §6.5 说明）

## 改动摘要

| 文件 | 改动 |
|---|---|
| `src/crates/services/services-core/src/session/dialog_turn.rs` | `DialogTurnData` 新增 `pub error_detail: Option<AiErrorDetail>`（置于 `token_usage` 之后、`status` 之前），serde 属性逐字按 AC1：`#[serde(default, skip_serializing_if = "Option::is_none", alias = "error_detail")]`；`new_with_kind` 构造补 `error_detail: None`；新增内联 `#[cfg(test)]` 模块 4 个 serde 兼容测试（AC5：旧格式无字段反序列化 OK / camelCase 主名往返 / snake alias 命中 / None 跳过序列化） |
| `src/crates/assembly/core/src/agentic/session/session_persistence/turn_lifecycle.rs` | `fail_dialog_turn` 签名增 `error_detail: Option<AiErrorDetail>`（AC2），落盘 `turn.error_detail = error_detail;`；引入 `northhing_core_types::AiErrorDetail` |
| `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs` | 唯一调用方适配（S2，rg 全仓确认仅 2 处命中：定义+此调用）：`error.error_detail()` 提取一次，事件用 `Some(error_detail.clone())`、`fail_dialog_turn` 传 `Some(error_detail)` —— 事件与落盘为同款 detail |
| `src/crates/assembly/core/src/agentic/session/session_persistence/save.rs` | AC3 合成消息：`build_messages_from_turns` 在 `has_text \|\| has_thinking` 分支之后新增 `else if turn.status == TurnStatus::Error` 分支——仅无文字/思考产物的 Error turn 且 `error_detail.is_some()` 时合成 `Message::assistant_with_reasoning(None, "[Error: {类别}: {provider_message}]", [])`（provider_message 空/缺失时退化为 `[Error: {类别}]`）；`error_detail` 为 None 的旧数据维持原跳过行为（双向兼容）。新增 `error_category_label`（13 类别中文标签，穷尽 match）与 `format_turn_error_message`（`pub(crate)`，desktop/CLI 零改动复用）。测试按 S3 钉死在 save.rs 内联 `#[cfg(test)]`：detail→合成、None→跳过、fallback 类别描述，共 3 个 |
| `src/crates/assembly/core/src/agentic/episodes/distill.rs` | 机械波及：`make_test_turn` struct literal 补 `error_detail: None`。另含该文件其余位置的 rustfmt 重排 hunk（见疑虑 3） |
| `src/crates/assembly/core/src/service/session_usage/service.rs` | 机械波及：`test_turn_with_tools` struct literal 补 `error_detail: None` |

AC4 双 surface 渲染 = 零 surface 代码改动，证据（现状引用）：

- desktop：`src/apps/desktop/src/ui_dioxus/app.rs:74` `session_mock::messages_to_entries(msgs)` → `session_mock.rs:102-149` 将 `MessageRoleDto::Assistant` 的 `Text`/`Mixed` 文本原样作为 Entity body 渲染 → 重建路径的合成 `[Error: ...]` 消息刷新后可见。
- CLI：`src/apps/cli/src/modes/chat/run.rs:84` 与 `session.rs:60` 调 `ChatState::from_core_messages`（`chat_state_core.rs:94`）→ assistant 文本消息直通渲染；live 路径同型形态证据 `chat_state_core.rs:317` `format!("[Error: {}]", error)` 与 desktop live `turn_banner.rs:51-52` `[Error: {err_text}]`——合成文本与之同前缀同风格（`[`Error: ` 英文括号前缀 + 界面中文类别标签）。
- 共享重建路径：`load.rs:10 rebuild_messages_from_turns` → `save.rs:45 build_messages_from_turns`（`session_manager_metadata.rs:452`、`restore_apply.rs:250/269` 消费）。

S5 声明：不做存量数据迁移（旧失败 turn 无 detail 即不可见，一期接受）；Error + 有部分文字的 turn 不合成（live 是 draft+error 拼接，重建侧一期只覆盖无产物 turn）；`fail_maintenance_turn` 不存 detail（out-of-scope，maintenance turn 非 model-visible）。

## 复用侦察

- **AiErrorDetail 直接复用**（`src/crates/contracts/core-types/src/errors.rs:40-58`，实测 derive `Clone, PartialEq, Eq, Serialize, Deserialize` + `rename_all="camelCase"`），未新建任何平行 error-detail 类型（AC1/C1 强制结论）。
- **serde 模式复用同文件先例**：新字段属性逐字对齐 `token_usage`（`dialog_turn.rs:65`）的 `default + skip_serializing_if + snake alias`，camelCase 主名由结构级 `rename_all` 自动得出。
- **事件同款 detail 复用**：`turn_persist.rs` 原事件构造处已持有 `error.error_detail()`，本单提取为局部变量一次求值、两处使用，未新增错误分类逻辑。
- **类别中文标签**：`error_category_label` 穷尽匹配 `ErrorCategory` 13 变体（与 `classify_ai_error_message` 同一枚举），编译期保证枚举扩展时此处强制跟进。
- **Message 构造模式复用**：合成消息沿用同函数 `has_text || has_thinking` 分支的 `Message::assistant_with_reasoning(...).with_turn_id(...)` 构造式（`msg_build.rs:75`），第三参空 tool_calls。

## 验证

命令树别说明：**宿主树** = `E:\agent-project\northing`（载有 W26-1/5 兄弟未提交 WIP）；**隔离树** = `C:\Windows\Temp\opencode\w26-2-verify`（`git worktree add` 于 TIP1 `f54a3fe`，即 BASE+本单唯一进场，共享 `--target-dir E:\agent-project\northing\target`）。

### 6.0 前置：`cargo check -p northhing-core --features product-full`（宿主树，非测试编译，佐证代码可编译）

```text
CHECK_OK   (exit 0)
```

（宿主树测试编译当时被 W26-5 兄弟 WIP 的 15 个 cfg(test) 错误阻断，见疑虑 1；故 core 测试证据全部改在隔离树。）

### 6.1 `rustup run stable-x86_64-pc-windows-msvc cargo check --workspace`（隔离树，TIP1，`--target-dir E:\agent-project\northing\target` 复用依赖产物）

全日志 527 行，`^error` 计 0。关键节选：

```text
    Checking northhing-core v0.2.10 (C:\Windows\Temp\opencode\w26-2-verify\src\crates\assembly\core)
    Checking northhing v0.2.10 (C:\Windows\Temp\opencode\w26-2-verify\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 58s
```

结果：0 error。desktop `northhing` bin 在隔离树（BASE+TIP1，无兄弟 WIP）编译通过，反证宿主树已知的 3 个 desktop 错误（entry.rs:104 / keyring.rs:312,330）纯系 W26-5 未提交 WIP，与本单无关。

### 6.2 `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full -- save:: distill:: session_usage::`（宿主树，含本单 WIP）

宿主树 lib-test 编译被兄弟 WIP 阻断（预期内，见疑虑 1）。15 个错误全部位于 `src/crates/assembly/core/src/service/mcp/server/manager/tests.rs`（W26-5 领地，本单未触碰），本单进场文件 0 错误。错误头逐条原文：

```text
error[E0425]: cannot find type `HashMap` in this scope
  --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:36:27
error[E0433]: cannot find type `Arc` in this scope
  --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:63:17
error[E0433]: cannot find type `Arc` in this scope
  --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:71:26
error[E0433]: cannot find type `MCPServerManager` in this scope
  --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:77:19
error[E0433]: cannot find type `HashMap` in this scope
  --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:79:23
error[E0422]: cannot find struct, variant or union type `MCPServerConfig` in this scope
  --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:81:18
error[E0433]: cannot find type `HashMap` in this scope
  --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:88:14
error[E0433]: cannot find type `Arc` in this scope
   --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:119:17
error[E0433]: cannot find type `Arc` in this scope
   --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:122:26
error[E0599]: no associated function or constant named `new_isolated_for_test` found for struct `ConfigService` in the current scope
   --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:124:52
error[E0433]: cannot find type `MCPServerManager` in this scope
   --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:128:19
error[E0433]: cannot find type `HashMap` in this scope
   --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:130:23
error[E0422]: cannot find struct, variant or union type `MCPServerConfig` in this scope
   --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:132:18
error[E0433]: cannot find type `HashMap` in this scope
   --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:139:14
error[E0599]: no associated function or constant named `new_isolated_for_test` found for struct `ConfigService` in the current scope
  --> src\crates\assembly\core\src\service\mcp\server\manager\tests.rs:73:52
error: could not compile `northhing-core` (lib test) due to 15 previous errors; 16 warnings emitted
```

green 证据替代链（同一 TIP1 代码）：

1. §6.0 宿主 `cargo check -p northhing-core --features product-full`（非测试编译）exit 0；
2. 隔离树（BASE+TIP1，无兄弟 WIP）过滤测试复跑 —— 绿（首次链式命令把 `--target-dir` 误置于 `--` 后被 libtest 拒绝，二进制编译成功后按 long-running-shell 规则 2 直跑测试二进制补完）：

```text
running 37 tests
test agentic::episodes::distill::tests::distill_with_no_tools ... ok
test agentic::episodes::distill::tests::error_message_truncation ... ok
test agentic::session::session_persistence::save::tests::test_build_messages_error_turn_fallback_category_description ... ok
test agentic::session::session_persistence::save::tests::test_build_messages_error_turn_with_detail_synthesizes_error_message ... ok
test agentic::episodes::distill::tests::repair_content_from_input_when_no_result_for_assistant ... ok
test agentic::episodes::distill::tests::distill_with_repair_across_rounds ... ok
test agentic::episodes::distill::tests::distill_with_failure_no_repair ... ok
test agentic::episodes::distill::tests::distill_with_tools ... ok
test service::session_usage::tracking::tests::cache_hit_rate_excludes_unreported_records_from_denominator ... ok
test service::session_usage::tracking::tests::cache_hit_rate_computes_when_all_records_report_cache ... ok
test service::session_usage::tracking::tests::cache_hit_rate_is_none_when_no_record_reports_cache ... ok
test service::session_usage::tracking::tests::cache_hit_rate_none_when_input_sum_is_zero ... ok
test agentic::session::session_persistence::save::tests::test_build_messages_error_turn_without_detail_is_skipped ... ok
test service::session_usage::tracking::tests::per_model_cache_hit_rate_isolated_per_model ... ok
test service::session_usage::persist::tests::report_uses_persisted_model_span_facts_without_token_records ... ok
test service::session_usage::tracking::tests::report_scopes_by_workspace_identity ... ok
test service::session_usage::persist::tests::remote_workspace_without_snapshot_marks_file_stats_partial ... ok
test service::session_usage::persist::tests::report_marks_remote_snapshot_stats_partial ... ok
test service::session_usage::persist::tests::aggregates_operation_summary_file_stats_without_reading_file_bodies ... ok
test service::session_usage::tracking::tests::report_counts_failed_and_cancelled_tool_duration_when_available ... ok
test service::session_usage::tracking::tests::report_uses_cached_tokens_when_provider_reports_them ... ok
test service::session_usage::persist::tests::file_rows_preserve_operation_turn_and_session_scopes ... ok
test service::session_usage::aggregation::tests::report_merges_legacy_model_timing_into_token_model_row_for_same_turn ... ok
test service::session_usage::tracking::tests::report_computes_tool_p95_only_with_multiple_duration_spans ... ok
test service::session_usage::aggregation::tests::report_excludes_local_command_turns_from_usage_metrics ... ok
test service::session_usage::aggregation::tests::report_adds_turn_anchors_to_slowest_spans ... ok
test service::session_usage::aggregation::tests::report_active_runtime_uses_active_span_union ... ok
test service::session_usage::aggregation::tests::report_adds_representative_anchors_to_model_tool_and_error_rows ... ok
test service::session_usage::tracking::tests::report_marks_cache_unavailable_for_zero_filled_cache_source ... ok
test service::session_usage::aggregation::tests::report_uses_clear_label_when_model_identity_is_missing ... ok
test service::session_usage::aggregation::tests::report_includes_error_examples_for_failed_turns_and_tools ... ok
test service::session_usage::persist::tests::remote_workspace_uses_wrapped_tool_inputs_for_file_rows ... ok
test service::session_usage::tracking::tests::report_sums_tool_phase_timings_and_marks_phase_coverage_available ... ok
test service::session_usage::aggregation::tests::report_active_runtime_includes_incomplete_turn_child_spans ... ok
test service::session_usage::tracking::tests::report_slowest_tool_spans_summarize_url_inputs ... ok
test service::session_usage::tracking::tests::report_slowest_tool_spans_include_diagnostic_fields ... ok
test service::session_usage::format::tests::report_slowest_tool_input_summary_redacts_common_secrets ... ok

test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 1044 filtered out; finished in 0.02s
```

### 6.3 `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-services-core`（宿主树，AC5 serde 测试）

```text
   Compiling northhing-services-core v0.2.10 (E:\agent-project\northing\src\crates\services\services-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.21s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_core-f64d1e29c2ef8d22.exe)

running 56 tests
test session::dialog_turn::tests::test_serde_snake_case_alias_error_detail ... ok
test session::dialog_turn::tests::test_serde_backward_compatibility_old_format_without_error_detail ... ok
test session::dialog_turn::tests::test_serde_roundtrip_with_error_detail_camel_case ... ok
test session::dialog_turn::tests::test_serde_skip_serializing_if_none ... ok

test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.09s
```

结果：lib 56/0（含 4 个新 serde 测试）+ 9 个集成测试目标 + doc-test 全绿（11 个 `test result: ok`，0 failed）。
隔离树复跑（TIP1 自足性复核，同命令）：

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2m 34s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.91s
```

### 6.4 `node scripts/check-repo-hygiene.mjs`（宿主树）

```text
Repository hygiene check passed (28 content files scanned, 3969 filenames checked).
```

exit 0。（输出前的 3 行 `LF will be replaced by CRLF` 为脚本内部 git status 对兄弟单未提交 brief 文件的 warning，非失败。）

### 6.5 `node scripts/verify-task-gate.mjs verify-attempt --base 831094c --tip f54a3fe --allowlist .superpowers/sdd/w26-2-allowlist.txt`（宿主树）

```text
Warnings:
  - Unfulfilled allowlist entry (not modified): .superpowers/sdd/w26-2-brief.md
  - Unfulfilled allowlist entry (not modified): .superpowers/sdd/w26-2-report.md
  - Unfulfilled allowlist entry (not modified): .superpowers/sdd/w26-2-allowlist.txt
Attempt verification passed: all modified files are within allowlist.
```

exit 0。tip 取 `f54a3fe`（本单最后**代码** commit）；report+allowlist 按 W26-3 先例以回填 commit 进场（其文件均在 allowlist 内，窗口 BASE..回填 commit 复跑结果见文末追加）。

**TIP2 回填后终窗复跑（`--base 831094c --tip 7610de5`）**：

```text
Warnings:
  - Unfulfilled allowlist entry (not modified): .superpowers/sdd/w26-2-brief.md
Attempt verification passed: all modified files are within allowlist.
```

exit 0。brief 未进场项说明：工作树中 w26-2-brief.md 的改动系编排者续派重钉 BASE（434f9a4→831094c），非本单工作产物，故未随 TIP2 进场，留待波收口。

### rustfmt

6 个进场文件逐一 `rustfmt --edition 2021 --check`（仓根 rustfmt.toml：max_width=120）全部 clean，宿主 GNU rustfmt（与 `pnpm run fmt:rs` 同一二进制）。

## 疑虑

1. **宿主树 §6.2 被兄弟 WIP 阻断（非本单问题）**：宿主 `cargo test -p northhing-core` lib-test 编译报 15 错，全部位于 `src/crates/assembly/core/src/service/mcp/server/manager/tests.rs`（`HashMap`/`Arc`/`MCPServerManager` E0425/E0433/E0599）——W26-5 领地，本单未触碰；错误行号指向其未提交的 credentials 重构。故 §6.2 证据改在隔离树（BASE+TIP1）取得，宿主树另以 6.0 非测试 check 佐证。**给编排者**：W26-5 落地前宿主树 core 测试目标不可编译，波收口 CI 或需其先行。
2. **隔离 worktree 缺 gitignore 的生成物**：`src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs` 为 i18n:generate 生成文件（ignored，不在 git），干净 checkout 缺失导致 core 编译 E0583。已从宿主原样复制补齐（字节相同，非源码改动，worktree git status 保持干净）。建议后续 brief 的 §6.1 模板加一句「worktree 建好后先复制该生成文件」。
3. **distill.rs 波及面超出 :164-183 区**：除钉死的 `error_detail: None` 一处外，另有 4 个 hunk（`extract_tool_records`/`extract_failures` 链式调用合行、`use` 换行、一处 `vec![]` 合行）——为 AGENTS.md 规则 3 的 `fmt:rs` 产物（仓 rustfmt.toml max_width=120 下这些行本应收敛为单行，BASE 文件在该配置下非 fmt-clean）。`rustfmt --check` 现全 clean。如实申报供 reviewer 裁量。
4. **services-core 既有告警**：`tests/session_layout_contracts.rs:3` unused imports `PathBuf`/`Path`——BASE 原样存在（该文件未进场、无兄弟改动），非本单引入，未越界修复。
5. **S5 已知边界（brief 钉死，非疑虑）**：存量失败 turn 不可见；Error+部分文字 turn 不合成；maintenance turn 不存 detail。
6. **台账 P2-5 建议翻转文案（编排者收口用，本单不进场 ledger）**：
   > P2-5: resolved (2026-09-26, W26-2: failed dialog turns persist AiErrorDetail as sibling optional `error_detail` on DialogTurnData (serde bidirectional compat, token_usage precedent); build_messages_from_turns synthesizes an `[Error: 类别: provider_message]` assistant message for Error turns without text/thinking output, so failures survive refresh on both surfaces via the shared rebuild path; legacy detail-less Error turns stay skipped — no stock migration, accepted phase 1)

## 状态

DONE —— §6.1（隔离树 workspace check 0 error）/ §6.2（宿主阻断逐条披露 + §6.0 非测试 check 绿 + 隔离树 37/0 绿）/ §6.3（宿主绿 + 隔离绿）/ §6.4（exit 0）/ §6.5（exit 0，窗口 831094c..7610de5）全闭环；疑虑节各项为环境/并行观察项与本单外的边界声明，无未决正确性疑虑。

## commit 后磁盘版修订（追加于 TIP2 `7610de5` 之后，未再进场）

- 触发原因：报告正文按编排者续派指令承诺「§6.2/§6.3 宿主树结果 + 隔离复跑文末追加」；TIP2 落地后隔离链跑完，按承诺回填 §6.2 节 green 证据链第 2 项（37/0 全量原文）、§6.3 节隔离复跑块、状态行 DONE_WITH_CONCERNS→DONE。
- 本节之后的磁盘修订均不改代码：TIP1 `f54a3fe`（全部源码改动）+ TIP2 `7610de5`（report/allowlist）仍是本单全部进场文件，与 `git diff 831094c..7610de5 --name-only` 一致。
- task-gate 最终窗口复跑（TIP2 落地后即时执行，见 §6.5 下方追加）：exit 0。
- 临时物清理：`git worktree remove C:\Windows\Temp\opencode\w26-2-verify`（隔离树内唯一额外文件为复制的 ignored 生成物 generated_locale_contract.rs，随 worktree 一并销毁，宿主树未受影响）。
