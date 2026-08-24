# Task T2-2d Brief: remote 栈子批 C2——agentic remote_file_delivery 通路删除

## Source
- 决策：T-01/TH-4（remote 栈整删）；本子批 = 侦察报告 `.superpowers/sdd/task-t2-2c-recon.md` Q1 第 3 条的 C2 子批
- 前置：C1 已并（`fa88342`，core remote_connect 模块已删）——RemoteRelay 触发源的生产者已不存在
- 行号以当前 main（HEAD `9c14d22`）实测为准

## 语义裁决（编排者已核，照执行勿请示）
- `DialogTriggerSource::RemoteRelay|Bot` **contracts 变体保留**（wire 稳定 + `runtime-ports/src/agent/agent_dialog.rs:62` policy + contracts 测试不动）
- `subagent_ports.rs:113` 的 `unwrap_or(Bot)` 默认值**保留不动**（其语义悬空问题已记台账，留给终审 triage / TH-5）
- remote_file_delivery 整链是 remote/mobile/bot 通道专属的 computer:// 链接提示词与渲染通路；desktop/接口层对 `computer_link`/`computer://` **零消费**（已核）。整链删除。

## 删除清单（全部在 `src/crates/assembly/core/src/agentic/`）

### S1. 生产者两处
- `coordination/dialog_turn/sub_handle_state.rs:119-122`：删 `needs_computer_links_for_source(...)` 条件块（reminder 注入）+ :23 import
- `coordination/dialog_turn/sub_handle_out.rs:153-155`：删 context_vars 设置 `TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY` 的块 + :26 import

### S2. context var 传递链
- `execution/turn_lifecycle.rs:104-120`：删读 KEY → `with_remote_file_delivery_channel` 的整段（:104-106 读取、:120 map 应用）
- `execution/` 其余 9 文件（ai_message_build.rs:28、multimodal.rs:28、execution_engine.rs:19、loop_detection.rs:28、health_snapshot.rs:27、turn_finalize.rs:28、turn_tick.rs:28、token_pressure.rs:28、turn_init.rs:28、turn_main_loop.rs:28）：删 `use ...TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY` import 行；**若文件内有该 KEY 的转发/读取代码一并删**（以 rg 命中为准）
- `tools/tool_context_runtime/context_init.rs:214-217`：删把 context var 复制进 ToolUseContext 的块 + :2 import
- `coordination/coordinator.rs:28-29`、`dialog_turn/workspace.rs:34`、`thread_goal.rs:34`、`session.rs:34`、`compaction.rs:34`：这些文件 import 了三件套（needs_computer_links_for_source / remote_file_delivery_reminder / KEY）——**逐文件核实实际使用点**（可能在文件内共享 helper），全部删净

### S3. PromptBuilderContext 字段
- `agents/prompt_builder/mod.rs`：删 :100 `pub remote_file_delivery_channel: bool`、:119 默认值初始化项、:155-156 `with_remote_file_delivery_channel` builder 方法
- `agents/prompt_builder/system_prompt.rs:120-123` 与 :257-260`：`user_workspace_relative_file_link(path, self.context.remote_file_delivery_channel)` 改为直接产出 workspace-relative 链接（恒 false 塌缩）；:7 import 同步
- `agents/prompt_builder/tests.rs:222`：使用 `with_remote_file_delivery_channel(true)` 的测试改为断言新行为（workspace-relative），或删除该测试的 computer:// 断言分支

### S4. create_plan_tool
- `tools/implementations/create_plan_tool.rs:6` import、:217-224 `use_computer_link` 分支：塌缩为 workspace-relative user_link
- 输出 JSON（:240 附近）：删 `"computer_link"` 字段（零消费方已核；工具当轮输出非持久协议）
- 相关测试同步

### S5. 删 remote_file_delivery.rs 整文件
- `agentic/remote_file_delivery.rs`（69 行含测试）删除
- `agentic/mod.rs` 的 `mod remote_file_delivery;` 声明行删除（实测行号）
- helper 处置：`workspace_relative_link` 若 S3/S4 后仍有调用方则**内联保留**（可移入调用方所在模块）；`computer_link`/`user_file_link`/`user_workspace_relative_file_link`/`needs_computer_links_for_source`/`remote_file_delivery_reminder`/`TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY` 全部删除
- 归零复核：`rg -n "remote_file_delivery|computer_link|computer://|TOOL_CONTEXT_REMOTE_FILE_DELIVERY|needs_computer_links" src --glob "*.rs"` → 0（历史 docs 不动）

### S6. boundary 规则核查
- `rg -n -i "remote_file_delivery|computer" scripts/core-boundaries/`——若有锚点同步删；无则本项空转并在报告注明
- 跑 `node scripts/check-core-boundaries.mjs` 必须绿

## Constraints
- 不 commit、不 push；改动留工作区
- contracts 层（runtime-ports/core-types/agent-runtime）**零改动**；DialogTriggerSource 变体保留
- SSH 语义（remote_connection_id/remote_ssh*）不动；services-integrations 不动（C3）
- `subagent_ports.rs:113` 的 `unwrap_or(Bot)` 不动
- cargo 一律 `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`；timeout 给足
- 勿碰并行 session 资产：`memory/`、`.graph/`、`.opencode/`、`.superpowers/sdd/` 其它 task-* 文件、前端文件
- 排除项：miniapp、tests/e2e/、mobile-web、docs/sdlc-harness/
- 家规 4 提示：本批不动 tokio::select!/cancellation/timeout 竞态；若实际碰到，必须带测试

## Verification（报告贴原始输出）
1. `cargo check --workspace`（MSVC）pass
2. `cargo check -p northhing`（MSVC）pass（家规 6）
3. `node scripts/check-core-boundaries.mjs` pass（或 S6 空转注明 + 仍跑确认绿）
4. focused 测试：`cargo test -p northhing-core --features product-full --lib prompt_builder`、`--lib create_plan`、`--lib dialog_turn`、`--lib coordination`（贴实际命令与输出）
5. S5 归零 grep 输出（命令 + 命中数）
6. `git diff --stat` 摘要

## Report
写 `.superpowers/sdd/task-t2-2d-report.md`，首行 `DONE` / `DONE_WITH_CONCERNS` / `NEEDS_CONTEXT` / `BLOCKED`。含逐项状态、验证原始输出、行数对账、遗留疑虑。报告之外只回状态 + 一行测试摘要 + concerns。
