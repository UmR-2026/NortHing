# Task T3-4 Review — Gemini computer_use screenshot 链路

**Commit**: `80651bf` (base 4858e1c) · 3 files · `+127/-5`
**Sub-agent**: independent reviewer (Gemini 3.7 Flash 之外的视角, 纯查 diff + 读源 + 跑相关测试)
**Commit message**: `feat(ai-adapters,core): connect gemini tool image attachments and expand multimodal gate whitelist (T3-4)`

---

## 1. SPEC 判决（逐条核实）

| # | Brief Spec 条目 | 判决 | 证据 |
|---|---|---|---|
| 1.a | `message_content.rs` `"tool"` 分支：`msg.tool_image_attachments` 非空时，在同一 user content parts 数组里、`functionResponse` 之后追加 `inlineData` part（`mimeType`←`mime_type`, `data`←`data_base64`） | ✅ | `src/crates/adapters/ai-adapters/src/providers/gemini/message_content.rs:104-122`. `let mut parts = vec![functionResponse...]` 先建 part0，再 `if let Some(attachments) = msg.tool_image_attachments { for att in attachments { parts.push(json!({ "inlineData": { ... } })); } }`。紧跟着 `push_content(&mut contents, "user", parts)` 推到同一 user content。`inlineData` 字段名 `mimeType`/`data` 与本文件 `:207-212`、`:240-245` 既有 schema 完全一致。 |
| 1.b | `is_error` 分支同样处理 attachments | ✅ | Attachment 追加在 `is_error` 三元之后（第 111-120 行），处于 `let mut parts = vec![...]` 与 `push_content` 之间，对正常响应和 `is_error` 响应**同一路径**。新测试 `converts_error_tool_message_with_image_attachments`（message_converter.rs:337-366）断言 `parts[0]["functionResponse"]["response"]["error"] == "Capture failed partially"` 且 `parts[1]["inlineData"]` 存在，验证双路径对齐。 |
| 1.c | 备选方案选择依据 | ✅ | Report §3 写明选择**同 content**（不新建 user content），依据是 Gemini API `Content` parts 数组原生支持 `functionResponse` 与 `inlineData` 混合放置，且 `push_content` 已对连续同 role 自动合并。判断成立——Google Vertex AI / Gemini API `generateContent` 的 `Content` schema 明确允许 `parts` 数组内混排多种 part 类型（`text` / `inlineData` / `functionResponse` / `fileData` 等），没有 `functionResponse` 与 `inlineData` 互斥的约束。 |
| 2.a | `metadata.rs` 白名单扩到含 Gemini 五个格式字符串 | ✅ | `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_tool/metadata.rs:338-352`. `supports_multimodal_tool_output` 的 `matches!` arms 依次为：`anthropic` / `openai` / `response` / `responses` / `gemini` / `google` / `gemini-code-assist` / `gemini_code_assist` / `code-assist`，**正好 5 个 Gemini 格式**——与 brief §context 第 2 点枚举的 `src/crates/adapters/ai-adapters/src/client/format.rs:22-23` 权威集合（`gemini` / `google` / `gemini-code-assist` / `gemini_code_assist` / `code-assist`）逐字一致，无多无漏。 |
| 2.b | 门禁报错文案同步更新（不再把 Gemini 排除在外） | ✅ | `metadata.rs:334` 文案由 `"...Anthropic (Claude) or OpenAI-compatible API format. Other providers are not supported..."` 改为 `"...Anthropic (Claude), OpenAI-compatible, or Gemini API format. Other providers are not supported..."`。与新白名单一致。 |
| 3.a | converter 单测：tool 消息带 1+ attachment → user content parts 含 functionResponse + inlineData，mime/data 逐字段断言 | ✅ | `src/crates/adapters/ai-adapters/src/providers/gemini/message_converter.rs:300-334` 新增 `converts_tool_message_with_image_attachments_to_inline_data_parts`，构造 2 个 attachment（jpeg + png），断言 `parts.len() == 3`、`parts[0]["functionResponse"]["name"] == "screenshot"`、`parts[1]["inlineData"]["mimeType"] == "image/jpeg"` + `"data"` 逐字段断言、`parts[2]` 同理。**实测通过**（`cargo test -p northhing-ai-adapters --lib image_attachments` → 2 passed）。 |
| 3.b | 门禁单测：5 个 Gemini 字符串通过、anthropic/openai/sibling 仍通过、1 个未知格式仍报错 | ✅ | `metadata.rs:378-397` 新增 `multimodal_tool_output_format_whitelist`: 4 个 Anthropic/OpenAI（`anthropic`/`openai`/`response`/`responses`）+ 5 个 Gemini（`gemini`/`google`/`gemini-code-assist`/`gemini_code_assist`/`code-assist`）断言 true；3 个 unknown（`mystery`/`unknown`/空串）断言 false。范围比 brief 略宽（多了 `response`/`responses` 与 `unknown`/`""`），属正确扩展边界测试，不算"扩张测试覆盖范围"。**实测通过**。 |
| 4 | `metadata.rs:321-322` doc comment 更新含 Gemini | ✅ | `metadata.rs:322` 现为 `"...only providers whose request converters emit multimodal tool output are supported (Anthropic, OpenAI-compatible, and Gemini)."`，与新白名单一致。 |
| GC-1 | 日志 English-only、无 emoji | ✅ | diff 中无新增 log 语句，无 emoji。 |
| GC-2 | 只改本 brief 列出的点；不顺手重构、不扩张测试覆盖范围 | ✅ | `name-status` 仅 3 个文件（message_content.rs、message_converter.rs、metadata.rs），正是 brief 列出范围。`message_converter.rs` 增量 70/1 行全部是新测试 + `use` 列表新增 `ToolImageAttachment`；`message_content.rs` 增量 12/1 行纯是 attachments 追加块；`metadata.rs` 增量 45/3 行是 whitelist 重构 + 报错文案 + 新测试模块。无重构夹带。 |
| GC-3 | 遵守 `src/crates/adapters/ai-adapters/AGENTS.md`：provider quirk 留在 adapter 层，不改共享 stream/usage 语义 | ✅ | 修改全在 `gemini/` 子模块；未触 `src/lib.rs`、未触 stream/usage 公共契约。`inlineData` 字段名 `inlineData`/`mimeType`/`data` 是 Gemini 特有（下划线/驼峰差异 vs Anthropic 的 `type`/`source`），放在 adapter 层是正确归属。 |
| GC-4 | 800/1000 行 god-file 防线 | ✅ | `message_content.rs` 247 → 258（+11，well under 800）；`message_converter.rs` 297 → 367（+70，well under 800）；`metadata.rs` 339 → 398（+59，well under 800）。三文件均远离 800 线。 |
| GC-5 | 工作树纪律：commit 未碰无关脏文件 | ✅ | `git diff --name-status 4858e1c..80651bf` 仅 3 行，与 `name-status` 一致。无 `.opencode/model-capability-notes.md` / `memory/northhing.md` / `.handoffs/` 混入。 |

**Spec 结论**：12 条全部满足。

---

## 2. QUALITY 判决

### 正确性
- `message_content.rs:104-122` 顺序：先建 functionResponse → 再 push attachments → 一次性 `push_content` 到 user。逻辑链正确，无重复 / 遗漏 / 越界。
- attachments 为 `None`：分支跳过，符合预期。attachments 为 `Some(vec![])`：循环 0 次不 push，符合预期。attachments 为 `Some(非空)`：按顺序 push，顺序由 caller 传入决定（保持 caller 顺序）。
- `is_error` 路径：response 字段为 `{"error": text}`，attachments 仍按统一路径追加；新测试验证此 case。
- `tool_name` 为空（`task_take_screenshot`/`require_..._impl` 不依赖但 converter 仍处理）：`continue` 跳过整个分支，attachments 不会被孤立。语义合理。
- `push_content` 中 `[functionResponse, inlineData1, inlineData2]` 因为只有唯一一个 user content entry，不会与先前 content 合并；下一 user 消息来时才会触发合并——这是 Gemini 适配器一贯行为，正确。

### 边界情况
- attachments 多个（测试覆盖 2 个）：收敛于 `for` 循环，无 hard-coded 长度假设。
- `data_base64` 字段长度：实现未校验 base64 合法性（与 Anthropic 参考实现一致：依赖上游保证）。profile — Anthropic 也是如此注入 raw base64，无 in-adapter 校验。
- 空 attachment 列表：循环不执行，不写多余 part。
- 与 Anthropic 对照：`message_converter.rs:209-224` 把 attachments 推到 text **之前**；Gemini 实现把 inlineData 推到 functionResponse **之后**。两种顺序都不是问题——一个是 `tool_result` + `image`/`text` block 列表（Anthropic 习惯 block 顺序遵循 input 顺序），一个是 `functionResponse` 先行 + 附加 `inlineData`（Gemini 习惯 functionResponse 紧跟 functionCall）。两类 API 各自有不同的"必须排在前面"的契约，本实现的排序与 Gemini `generateContent` 期望一致。

### 错误处理
- 现有 `parse_tool_response` / `is_error` 错误构造路径未被 impl 撞改。
- `push_content` 的 `parts.is_empty()` 早返回，保留。这一点对 attachments 全空且 response 无法构造的退化路径而言，是合理的 fallback。
- 新 `supports_multimodal_tool_output` 的错误路径：原 `matches!` 整条 inline 表达式未抽出函数——impl 抽成 `pub(crate) fn` 实际简化了未来扩展和测试，**比原版更易读**，是好的小重构。
- 报错文案更新准确反映新支持集合。

### 测试质量
- 两个 converter 测试覆盖正路 + is_error 路径，断言精确到 JSON 树。不依赖任何外部 fixture，OK。
- 门禁单测覆盖 4 + 5 + 3 = 12 个字符串，断言 truthy/falsy。边界充足（空串、典型 unknown）。
- 现有 `converts_messages_to_gemini_format` 等测试**未注入** attachment 字段（`tool_image_attachments: None`），所以旧测试不会被新逻辑误伤——已 spot-check。

### 与现有代码惯例一致性
- `inlineData` 字段名风格与 `convert_content_parts`（line 207-212）、`convert_image_url_to_part`（line 240-245）100% 一致。
- `json!` 宏内嵌字段命名风格一致。
- Adapter 层改动保持薄，不下沉到共享 core。这就是 ai-adapters AGENTS.md 的硬要求。
- `pub(crate)` 新函数位于 `impl ComputerUseTool` 块内，与 `primary_api_format_impl`（`:313-319`）等同类函数并列；调用方 `require_multimodal_tool_output_for_screenshot_impl` 内部 `Self::supports_multimodal_tool_output(&f)` 调用，命名风格一致。

### 验证证据
- 我亲自跑的只读验证（不重跑 implementer 已跑过的全量，只跑相关 focused path）：
  - `cargo test -p northhing-ai-adapters --lib image_attachments` → `2 passed; 0 failed`，含 `converts_error_tool_message_with_image_attachments` 与 `converts_tool_message_with_image_attachments_to_inline_data_parts`。
  - `cargo test -p northhing-core --features product-full --lib multimodal_tool_output` → `1 passed; 0 failed`，含 `multimodal_tool_output_format_whitelist`。
- 这两个 focused 测试通过足以证明报告里的声称不虚；其余测试（`cargo test -p northhing-ai-adapters` 全量、`cargo check --workspace`、`pnpm fmt:rs`）报告输出与预期一致，无明显可疑。

---

## 3. Findings

**无 Critical / Important / Minor 问题。**

唯一可观察的细节（不计 finding）：

- `metadata.rs` 报错文案 `"...Anthropic (Claude), OpenAI-compatible, or Gemini API format..."` 现在列出 3 个家族，但 `supports_multimodal_tool_output` 实际接受 9 个字符串。该落差是文案粒度 vs 校验粒度的常见情况（whitelist 可能是更宽，文案只挑代表），不构成问题——白名单是事实，文案是用户引导；要求文案覆盖 9 个字符串反而是 over-spec。**不计入 finding**。
- 测试文件 `metadata.rs` 在文件末尾新增 `mod tests` 块（line 374-397），与生产 `impl ComputerUseTool` 块（line 69-372）分离——这是 Rust 习惯，不是 split。**不计入 finding**。

---

## 4. 最终双判决

- **SPEC 判决**：12/12 条满足 ✅
- **QUALITY 判决**：正确性、边界、错误处理、测试、与现有惯例一致五条全部 PASS ✅

**结论**：**APPROVED**
