# Task T3-4 Implementation Report — Gemini computer_use screenshot 链路（tool_image_attachments 贯通）

## 1. 改动文件清单

- `src/crates/adapters/ai-adapters/src/providers/gemini/message_content.rs` (258 lines)
- `src/crates/adapters/ai-adapters/src/providers/gemini/message_converter.rs` (367 lines)
- `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_tool/metadata.rs` (398 lines)

## 2. Spec 满足情况

- **Spec 1（Gemini 转换器图片附件贯通）**：
  在 `message_content.rs` 的 `"tool"` 分支中，当 `msg.tool_image_attachments` 为 `Some(attachments)` 时，在构建的 `parts` 数组（首项为 `functionResponse`）后，依次遍历追加 `json!({ "inlineData": { "mimeType": att.mime_type, "data": att.data_base64 } })`。无论是正常 tool 响应还是 `is_error` 分支均统一执行该逻辑。
- **Spec 2（运行时门禁白名单扩充与文案更新）**：
  在 `metadata.rs` 中实现 `supports_multimodal_tool_output(format: &str) -> bool`，白名单覆盖 Gemini 五个格式字符串（`"gemini"`, `"google"`, `"gemini-code-assist"`, `"gemini_code_assist"`, `"code-assist"`）以及既有格式（`"anthropic"`, `"openai"`, `"response"`, `"responses"`）。门禁报错文案同步更新为包含 Gemini API format。
- **Spec 3（单测最小集覆盖）**：
  - `message_converter.rs`：新增 `converts_tool_message_with_image_attachments_to_inline_data_parts`（断言 2 个 attachment 生成对应 mimeType 与 base64 data 的 inlineData parts）与 `converts_error_tool_message_with_image_attachments`（断言 error 响应带图）。
  - `metadata.rs`：新增 `multimodal_tool_output_format_whitelist`（断言 5 个 Gemini 格式通过、4 个 Anthropic/OpenAI 格式通过、未知格式报错）。
- **Spec 4（注释更新）**：
  `metadata.rs:require_multimodal_tool_output_for_screenshot_impl` doc comment 更新为 `(Anthropic, OpenAI-compatible, and Gemini)`。

## 3. 方案选择与依据

- **选择方案**：在同一 `user` content 的 `parts` 数组里紧随 `functionResponse` 追加 `inlineData` part。
- **依据**：Gemini 官方 API 标准中，`Content`（role 为 `user`）支持混合放置 `functionResponse` 与多模态 `inlineData` parts。此外 `push_content` 实现本身会对连续同 role 进行合并。主方案保持单 turn 干净结构，无需额外构造 synthetic 消息。

## 4. 偏离 brief 说明

无偏离。

## 5. 验证命令与输出记录

### 1) `cargo test -p northhing-ai-adapters`
命令：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-ai-adapters
```
输出尾部：
```text
test providers::gemini::message_converter::tests::converts_error_tool_message_with_image_attachments ... ok
test providers::gemini::message_converter::tests::converts_tool_message_with_image_attachments_to_inline_data_parts ... ok
test providers::gemini::message_converter::tests::converts_messages_to_gemini_format ... ok
...
test result: ok. 129 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
...
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.93s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.31s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

### 2) `cargo test -p northhing-core --features product-full computer_use`
命令：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full computer_use
```
输出尾部：
```text
running 24 tests
test agentic::tools::computer_use_host::tests::app_selector_constructors_populate_only_one_field ... ok
test agentic::tools::computer_use_host::tests::click_index_target_serializes_with_kind_tag ... ok
test agentic::tools::computer_use_host::tests::interactive_view_opts_apply_defaults_on_minimal_json ... ok
test agentic::agents::definitions::subagents::computer_use::tests::computer_use_mode_basics ... ok
test agentic::tools::computer_use_host::tests::interactive_scroll_params_apply_defaults ... ok
test agentic::tools::computer_use_host::tests::interactive_click_params_apply_defaults ... ok
test agentic::tools::implementations::computer_use_input::tests::screenshot_params_silently_ignore_crop_half_extent ... ok
test agentic::tools::implementations::computer_use_input::tests::screenshot_params_silently_ignore_legacy_quadrant_and_crop_fields ... ok
test agentic::tools::implementations::computer_use_input::tests::screenshot_params_silently_ignore_implicit_center ... ok
test agentic::tools::computer_use_host::tests::interaction_state_serializes_expected_shape ... ok
test agentic::tools::computer_use_host::tests::visual_mark_params_apply_defaults ... ok
test agentic::tools::implementations::computer_use_input::tests::screenshot_params_honor_window_flag ... ok
test agentic::tools::computer_use_host::tests::interactive_type_text_params_round_trip ... ok
test agentic::tools::implementations::computer_use_result::tests::append_interaction_state_includes_structured_block ... ok
test agentic::tools::implementations::computer_use_tool::metadata::tests::multimodal_tool_output_format_whitelist ... ok
test agentic::tools::implementations::computer_use_result::tests::screenshot_body_keeps_existing_fields_and_adds_interaction_state ... ok
test agentic::tools::computer_use_host::tests::app_click_params_apply_defaults_on_deserialize ... ok
test agentic::tools::computer_use_host::tests::click_target_serializes_with_kind_tag ... ok
test agentic::tools::computer_use_host::tests::app_wait_predicate_round_trips_each_variant ... ok
test agentic::tools::computer_use_host::tests::interactive_view_round_trips ... ok
test agentic::tools::implementations::control_hub_tool_tests::control_hub_tests::description_points_desktop_and_system_work_to_computer_use ... ok
test agentic::agents::prompt_builder::tests::runtime_context_includes_computer_use_info_only_when_needed ... ok
test agentic::agents::registry::tests::computer_use_is_builtin_subagent_not_mode ... ok
test agentic::tools::registry::tests::registry_exposes_controlhub_and_computer_use ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 993 filtered out; finished in 0.01s
```

### 3) `cargo check --workspace`
命令：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
输出尾部：
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 47.91s
```

### 4) `pnpm run fmt:rs`
命令：
```powershell
pnpm run fmt:rs
```
输出：
```text
[format-changed-rust] Formatting 3 Rust file(s).
```

## 6. 提交信息

- Commit: `80651bf`
- Message: `feat(ai-adapters,core): connect gemini tool image attachments and expand multimodal gate whitelist (T3-4)`
