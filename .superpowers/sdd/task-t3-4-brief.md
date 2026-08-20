# Task T3-4 Brief — Gemini computer_use screenshot 链路（tool_image_attachments 贯通）

## 背景（已排查钉死，直接采信）

computer_use 工具的 screenshot 结果通过 `tool_image_attachments` 把 JPEG 挂进 tool 消息，但只有 Anthropic + OpenAI 系 converter 会把它序列化进请求；Gemini 链路两处断点导致截图永远到不了 Gemini 模型：

1. **转换器丢图**：`src/crates/adapters/ai-adapters/src/providers/gemini/message_content.rs:86-112`（`convert_messages` 的 `"tool"` role 分支）只构建 `functionResponse` part，`msg.tool_image_attachments` 被完全忽略。
   - 参照实现（不要照抄形状，照抄语义）：`src/crates/adapters/ai-adapters/src/providers/anthropic/message_converter.rs:209-224` 把 attachments 映射为 image blocks + text block。
   - 类型：`northhing_core_types::tool_image_attachment::ToolImageAttachment { mime_type: String, data_base64: String }`（`src/crates/contracts/core-types/src/tool_image_attachment.rs:6-9`）。
   - Gemini part 形状（本文件 `:195-201` 已有同款）：`{ "inlineData": { "mimeType": ..., "data": ... } }`。

2. **运行时门禁拦截**：`src/crates/assembly/core/src/agentic/tools/implementations/computer_use_tool/metadata.rs:323-336`（`require_multimodal_tool_output_for_screenshot_impl`）第二道门 `matches!(f.as_str(), "anthropic" | "openai" | "response" | "responses")` 不含 Gemini 格式，直接报错拒绝。
   - `f` 的来源 = `ctx.custom_data["primary_model_provider"]` = `ai_client.config.format` 原始字符串（`src/crates/assembly/core/src/agentic/execution/turn_init.rs:324`）。
   - Gemini 实际格式字符串集合（`src/crates/adapters/ai-adapters/src/client/format.rs:22-23` 权威）：`"gemini"`、`"google"`、`"gemini-code-assist"`、`"gemini_code_assist"`、`"code-assist"`。白名单必须覆盖这五个。
   - 第一道门 `ctx.primary_model_supports_image_understanding()` 由用户模型配置 capabilities/category 驱动（turn_init.rs:130-163），**不改**。

## Spec（必须全部满足）

1. `message_content.rs` 的 `"tool"` 分支：当 `msg.tool_image_attachments` 非空时，在同一 user-role content 的 parts 数组里、`functionResponse` part 之后，为每个 attachment 追加一个 `inlineData` part（`mimeType` ← `mime_type`，`data` ← `data_base64`）。is_error 分支同样处理（错误消息也可能带图）。
   - 若你确认 Gemini API 不允许 functionResponse 与 inlineData 共存于同一 content，备选方案：attachments 放进紧随其后的独立 user content。选哪个由你定，report 里必须写明选择及依据。
2. `metadata.rs` 白名单扩到含 Gemini 五格式字符串。门禁报错文案（`:333-335`）同步更新——现在还写着 "set the primary model to Anthropic (Claude) or OpenAI-compatible API format"，Gemini 已支持后这句就是谎言。
3. 新测试（最小集，失败即证明逻辑坏的级别）：
   - converter 单测：tool 消息带 1+ 个 attachment → 输出 user content parts 含 functionResponse + inlineData，mime/data 逐字段断言。放进现有测试模块（`src/crates/adapters/ai-adapters/src/providers/gemini/message_converter.rs:41` 起，该文件是 convert_messages 的薄 facade）。
   - 门禁判定单测：五个 Gemini 格式字符串通过、`"anthropic"`/`"openai"` 仍通过、一个未知格式（如 `"mystery"`）仍报错。ToolUseContext 夹具若太重，允许把格式判定抽成 module-private 纯函数（如 `fn supports_multimodal_tool_output(format: &str) -> bool`）再测它——gate 本体保持薄。
4. 注释更新：`metadata.rs:321-322` 的 doc comment（"only providers ... (Anthropic + OpenAI-compatible)"）改为含 Gemini。

## Global Constraints（逐字遵守）

- 日志 English-only、无 emoji（如新增日志）。
- 只改本 brief 列出的点；不顺手重构、不扩张测试覆盖范围。
- 遵守 `src/crates/adapters/ai-adapters/AGENTS.md`：provider quirk 留在 adapter 层，不改共享 stream/usage 语义。
- 生产 `.rs` 文件超 800 行有审查压力，超 1000 行必须拆或加 `// allow-god-file` 注释（`message_content.rs` 现 247 行，注意增长幅度）。

## 验证（最小集，命令 + 输出都要进 report）

环境：Windows，cargo 一律走 MSVC wrapper：
`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. `cargo test -p northhing-ai-adapters`（全绿，含新 converter 测试）
2. `cargo test -p northhing-core computer_use` 或能命中新门禁测试的最近 focused 测试命令（你跑什么写什么）
3. `cargo check --workspace`
4. `pnpm run fmt:rs`（改了 .rs 后）

## 报告

写到 `.superpowers/sdd/task-t3-4-report.md`，含：改动文件清单、每个 Spec 条目如何满足、上述每条验证命令 + 实际输出尾部、备选方案选择依据（如适用）、任何偏离 brief 之处。结束时回报状态：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## 派发元信息

- BASE commit（派发前 HEAD）：`4858e1c`
- 工作树有与本任务无关的脏文件（`.opencode/model-capability-notes.md`、`memory/northhing.md`、`.handoffs/`），**不要碰、不要提交它们**；commit 只 stage 你改的文件。
