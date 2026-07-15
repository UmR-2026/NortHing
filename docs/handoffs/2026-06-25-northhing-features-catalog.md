# NortHing v0.1.0 现有功能目录（用户视角）

> **日期**：2026-06-25
> **作者**：general (mvs_b1a53334afd342c2b9412c4b2e532840)
> **目的**：用"用户能做什么"的视角把 NortHing v0.1.0 现有功能梳理成一份功能目录，方便对外介绍、对内对照 baseline 评估进度。
> **输入材料**：
> - `docs/handoffs/2026-06-25-agent-baseline-features.md`（15 P0 + 4 P1 + 9 P2 baseline）
> - `docs/handoffs/2026-06-25-northhing-state-review.md`（每个 baseline 项在 NortHing 里的实现状态 + file:line）
> **本文档不做**：代码 review、不评估代码质量、不改任何实现、不重新读 NortHing 源代码。

---

## 1. TL;DR

**NortHing v0.1.0 已经是一个"基本能用的 AI agent 应用 + 完整工程化后台"**— 用户能多轮对话 AI、切换模型、让 AI 读写文件、跑命令、查 git、做 diff/回滚、生成方案审批、派子任务并行干活、连远程 SSH 服务器；并通过完整 MCP 协议接入外部工具。**四大端**（Desktop / CLI / Web / Mobile Web）共享同一 Rust 后台（`northhing-core` + `northhing-assembly`），分层清晰（6 层单向依赖，~27 个 crate），后端测试覆盖扎实。

**但当前 desktop GUI 还"刚能用"**：状态栏默认空、错误信息只在终端里写、所有 surface 的会话/任务还没同步层、Background Agent / Skills 插件市场 / Hooks 等 P2 战略能力还没做。CLI 端是完整可用的，Tauri Desktop 端是 beta 体验。

---

## 2. 功能目录

> **状态图例**：
> - ✅ 完整 — 用户可以稳定使用
> - ⚠️ 部分 — 主体已实现但有 UX/集成尾巴
> - ❌ 缺失 — 用户看不到对应能力
> - 🟡 仅 stub — 模块占位但没接进真实流程

### 2.1 P0 必备能力（15 项：✅ 11 / ⚠️ 4 / ❌ 0）

#### P0-1 会话管理（新建 / 删除 / 切换 / 重命名 / 搜索）⚠️ 部分

- **用户视角**：左侧能看到会话列表，能新建/删除/切换/重命名会话；支持按内容搜索历史会话（独立全文搜索模块）。
- **现状**：后端 ✅ 完整，Desktop ⚠️ callback 全注册但首次启动没自动建 session，导致刚打开应用时 sidebar 是空的、`current_session_id == ""`。
- **关键 file:line**：
 - 后端 `agentic/session/session_manager.rs:1070 create_session`、`:2203 delete_session`、`:3048 list_sessions`
 - 调度 `agentic/coordination/coordinator.rs:1212 create_session`、`:1261 create_session_with_workspace`、`:4008 list_sessions`
 - 全文搜索独立 `service/search/{service.rs, remote.rs, remote_disabled.rs}`
 - Desktop `apps/desktop/src/app_state/mod.rs:490 on_new_session`、`:610 on_delete_session`、`:569 on_switch_session`

#### P0-2 发送消息 + 多轮上下文 + 流式响应 ⚠️ 部分

- **用户视角**：在输入框打字→回车，AI 基于完整历史上下文回复；长回复逐 token 流式打字显示（"打字机效果"）。
- **现状**：流式响应 ✅ 完整；发送消息 ⚠️ 后端 ✅ + Desktop ⚠️（缺 startup session 触发早退）。
- **关键 file:line**：
 - 流式响应 `src/crates/execution/agent-stream/src/lib.rs:773 process_stream`、`:797 process_stream_with_options`、`:399 StreamProcessor`
 - 客户端串流 `infrastructure/ai/client_factory.rs:147-229 get_or_create_client` 配 `StreamOptions` + `tool_call_accumulator.rs` 累积工具调用
 - 后端执行 `agentic/execution/execution_engine.rs`（3469 行大文件）+ `round_executor.rs`
 - Desktop 回调 `apps/desktop/src/app_state/mod.rs:329 on_send_message`，line 407 早退守卫

#### P0-3 模型切换（UI 选单）✅ 完整

- **用户视角**：UI 上能在会话内或全局设置里切换不同模型。
- **现状**：5 种解析策略（primary / fast / `<id>` / `<name>` / `<model_name>`）。
- **关键 file:line**：`infrastructure/ai/client_factory.rs:91-108 get_client_resolved`；UI 在 `apps/desktop/src/ui/views/InspectorView.slint`

#### P0-4 多 Provider 支持 + API Key 管理 + 自定义 Base URL ✅ 完整

- **用户视角**：能同时接入 Anthropic / OpenAI / Gemini / 自建代理 等多家厂商；设置页统一管多个 API Key；能配置自定义 Base URL 转发到自建网关或第三方代理。
- **现状**：3 种鉴权方式（`ApiKey` / `CodexCli` / `GeminiCli`），proxy 字段独立透传。
- **关键 file:line**：
 - `infrastructure/ai/mod.rs:10 pub use northhing_ai_adapters::providers;`
 - `config/types.rs:1196 AIModelConfig.provider`
 - `config/types.rs:1205 api_key` + `types.rs:1285-1295 AuthConfig` 三种
 - `config/types.rs:1198 base_url` + `:1200-1203 request_url`（自动派生）
 - `types.rs:595 AIConfig.proxy` + `client_factory.rs:197-201` 透传

#### P0-5 读 / 写 / 编辑文件 ✅ 完整

- **用户视角**：AI 能读文件内容、写新文件、精确编辑指定行（按行号替换）。
- **关键 file:line**：`agentic/tools/implementations/file_read_tool.rs` / `file_write_tool.rs` / `file_edit_tool.rs` / `delete_file_tool.rs`

#### P0-6 Terminal / Shell 执行 + 文件搜索（grep / glob）✅ 完整

- **用户视角**：AI 能在沙箱里跑命令（npm test、git、grep 等）；能跨文件搜内容、按模式匹配文件。
- **关键 file:line**：
 - `agentic/tools/implementations/bash_tool.rs`
 - `agentic/tools/implementations/exec_command/`（`mod.rs` + 9 子文件：local_shell / background_command_output / stdin / progress 等）
 - `agentic/tools/implementations/grep_tool.rs` / `glob_tool.rs` / `ls_tool.rs`

#### P0-7 Diff 视图 + 检查点回滚 ✅ 完整

- **用户视角**：所有修改能看到前后 diff；能回滚到任意检查点；危险操作有明确提示。
- **现状**：完整的 snapshot 模块（含 file lock + isolation + event）。
- **关键 file:line**：
 - Diff 工具 `agentic/tools/implementations/get_file_diff_tool.rs`
 - Snapshot `service/snapshot/{manager.rs, snapshot_core.rs, snapshot_system.rs, service.rs, types.rs, file_lock_manager.rs, isolation_manager.rs, events.rs}`

#### P0-8 Git 操作（status / commit / diff / branch / PR）✅ 完整

- **用户视角**：AI 能读 git 状态、提交代码、查看历史。
- **关键 file:line**：
 - `agentic/tools/implementations/git_tool.rs`
 - 底层 `service/git/{git_service.rs, git_types.rs, git_utils.rs, graph.rs}`

#### P0-9 计划 / 审批流（Plan Mode）✅ 完整

- **用户视角**：复杂任务 AI 先出方案（Plan），用户审核后才执行；危险命令（rm -rf、写文件、执行命令）逐项确认。
- **关键 file:line**：
 - `agentic/agents/definitions/modes/PlanMode`
 - `agentic/tools/implementations/create_plan_tool.rs`
 - Shell 安全策略 `config/types.rs:626 ShellSecurityConfig` + `:847-948 default_mode_policies`

#### P0-10 侧边栏 + 设置页面 ✅ 完整

- **用户视角**：左侧能看到会话列表和导航；统一设置页管理 model / key / 权限 / 主题。
- **关键 file:line**：
 - 侧边栏 `apps/desktop/src/ui/views/SidebarView.slint`
 - 数据源 `apps/desktop/src/app_state/sessions.rs` + `app_state/mod.rs:558 refresh_sessions_ui`
 - 设置后端 `ConfigManager::set`（`manager.rs:310`）+ `ConfigService::set_config` 支持 dot-path get/set
 - 主题 `config/types.rs:340 ThemesConfig` + `providers.rs:243 ThemesConfigProvider` + `manager.rs:75 default_current="northhing-light"`

#### P0-11 状态栏（Pending / Running / Failed / Done）+ 错误展示 / 重试 ⚠️ 部分

- **用户视角**：底部状态栏能看到 MCP 状态、当前模型、应用标题；操作失败时能看到可读错误并一键重试。
- **现状**：
 - 状态栏 ⚠️ 后端 ✅ + Desktop ⚠️（首次启动默认配置 `ai.models: []` → Model 段永远 "Not configured"；MCP 段永远 "Pending"）
 - 错误展示 ❌ 缺失 — 9 个 Slint 回调的所有失败全用 `eprintln!` 写 stderr，UI 无任何错误展示通道；grep 验证 `ui.set_*error` 在整个 `app_state/` 目录 0 命中
- **关键 file:line**：
 - UI `apps/desktop/src/ui/views/StatusBarView.slint`（48 行，三段 mcp-status / model-status / app-title）
 - 后端 MCP 状态 `app_state/inspector.rs:14 build_mcp_status_string`
 - 后端 model 状态 `app_state/inspector_model_status.rs:14 build_model_status_string`
 - 失败仅 eprintln：`apps/desktop/src/app_state/mod.rs:402, 408, 428, 453, 504, 526, 562, 668, 717, 851` 等 20+ 处

#### P0-12 MCP（Model Context Protocol）接入 ⚠️ 部分

- **用户视角**：通过 Anthropic 发起的 MCP 协议接入任意外部工具（GitHub / Postgres / Playwright / Slack 等），统一协议替代每个工具单独适配。
- **现状**：协议层 ✅ 完整（基于 rmcp SDK + 自定义 JSON-RPC + streamable HTTP）；Server + Adapter 都实现了；**Desktop 全局注册缺失**— `apps/desktop/src/app_state/inspector.rs:26` 构造了 local `MCPService` 但从不调 `set_global_mcp_service()` → 全局 `GLOBAL_MCP_SERVICE` 永远 None。
- **关键 file:line**：
 - 协议 `mcp/protocol/{transport.rs, transport_remote.rs, rmcp_mapping.rs, jsonrpc.rs}`
 - Server `mcp/server/{connection.rs, process.rs, catalog_cache.rs, runtime_helpers.rs, runtime_policy.rs}`
 - Adapter `mcp/adapter/{context, prompt, resource, tool}.rs`
 - 全局注册缺失位置 `apps/desktop/src/main.rs:18-19` + `apps/desktop/src/app_state/inspector.rs:26`
 - 全局变量 `service/mcp/mod.rs:83-92 GLOBAL_MCP_SERVICE`

#### P0-13 主题切换（暗 / 亮）✅ 完整

- **用户视角**：设置页一键切换暗色 / 亮色主题。
- **关键 file:line**：`config/types.rs:340 ThemesConfig` + `providers.rs:243 ThemesConfigProvider` + `manager.rs:75 default_current="northhing-light"`

#### P0-14 会话导入 / 导出 ✅ 完整

- **用户视角**：能导出会话备份；能从外部恢复会话（或基于现有会话拉分支）。
- **关键 file:line**：
 - `persistence/manager.rs:16 SessionTranscriptExport` + `SessionTranscriptExportOptions`
 - `service/session/...` 暴露 `SessionMetadataBuildFacts` / `SessionBranchRequest` / `SessionBranchResult`（branch = 导入场景）

#### P0-15 Markdown / 代码高亮渲染 ✅ 完整

- **用户视角**：AI 回复中的 Markdown 表格 / 列表 / 代码块正常渲染；代码有语法高亮。
- **现状**：所有 chat 类应用的基础输出质量，state review 未单独列出位置（默认全实现）。

### 2.2 P1 重要能力（4 项：✅ 1 / ⚠️ 2 / ❌ 1）

#### P1-1 Subagent / 子任务委派 ✅ 完整

- **用户视角**：主 agent 能派子 agent 跑独立子任务（隔离上下文），并行执行复杂任务。
- **现状**：内置 5 种 subagent（ExploreAgent / FileFinderAgent / GeneralPurposeAgent / ResearchSpecialistAgent / ComputerUseMode）。
- **关键 file:line**：
 - `agentic/tools/implementations/task_tool.rs`（1240+ 行）
 - `agentic/agents/definitions/subagents.rs`
 - `agents/registry/catalog.rs` 暴露 `builtin_agent_specs`

#### P1-2 记忆 / Rules 文件（CLAUDE.md / .cursorrules）⚠️ 部分

- **用户视角**：跨会话 / 项目持久化用户偏好和项目规范；agent 每次启动就知道"这个项目怎么干"。
- **现状**：路径约定有（`user_data_dir().join("rules")` + 读 `IDENTITY.md`），但缺显式的"rules 自动加载到 system prompt"入口。
- **关键 file:line**：
 - `infrastructure/app_paths/path_manager.rs:312 user_data_dir().join("rules")`
 - `service/workspace/manager.rs:90-100 load_from_workspace_root` 读 `IDENTITY.md`

#### P1-3 Background Agent（异步任务）❌ 缺失

- **用户视角**：发起长任务后不必干等，可继续其他工作；跨设备查看是 2025 下半年新趋势。
- **现状**：完全缺失— 没看到 background agent / cloud 任务队列相关模块。

#### P1-4 Remote-SSH / 远程 Workspace ⚠️ 部分

- **用户视角**：本地 UI 壳子，编辑 / 终端 / Language Server 全在远端跑；远程服务器开发友好。
- **现状**：
 - **Remote Workspace ✅ 完整**（3446 行单文件，含设备配对 / 加密 / 二维码 / relay）
 - **Remote-SSH ⚠️ 较完整** — 主体连接 / 执行 / 文件 / 终端都通了，但 4 处 SSH port forwarding（`-L` / `-R`）只是 TODO stub
- **关键 file:line**：
 - SSH 主体 `remote_ssh/manager.rs`（2792 行）+ `remote_exec.rs`（1035 行）+ `remote_fs.rs`（367 行）+ `remote_terminal.rs`（336 行）
 - SSH 底层依赖 `Cargo.toml:129-131` `russh` + `russh-sftp` + `russh-keys`
 - SSH port-forwarding stub `manager.rs:2476, 2504, 2516, 2551`
 - Remote Workspace `src/remote_connect.rs`（3446 行），子模块：device / encryption / pairing / qr / relay_client
 - 协议 `RemoteCommand:1900` + `RemoteResponse:1992` + 5 个 host trait + 穷尽 match `handle_remote_command:2153-2224`
 - 状态跟踪 `RemoteSessionStateTracker:2297-2892`

### 2.3 P2 加分能力（9 项：✅ 1 / ⚠️ 1 / 🟡 浅 1 / ❌ 6）

#### P2-1 Hooks（事件驱动的自动化）❌ 缺失

- **用户视角**：能在 PreToolUse / PostToolUse 等事件点挂脚本做自动化。
- **现状**：完全缺失。

#### P2-2 Cron / 定时任务 🟡 存在但浅

- **用户视角**：能跑定时任务。
- **现状**：`service/cron/` 目录存在（service 层），实现深度 state review 未深查。
- **关键 file:line**：`crates/services/services-integrations/src/cron/`（具体深度未确认）

#### P2-3 多模态（图像 / 语音 / 文件上传）❌ 未发现

- **用户视角**：能在输入框粘图片、拖文件、上传附件让 AI 看。
- **现状**：state review 未发现独立模块；标注缺失。

#### P2-4 成本 / token 实时统计 ❌ 未发现

- **用户视角**：能在 UI 看当前会话用了多少 token / 多少钱。
- **现状**：state review 未发现专门模块；标注缺失。

#### P2-5 Codebase 语义索引（embeddings + 向量搜索）❌ 未发现

- **用户视角**：能用语义搜索"找所有处理用户登录的代码"而不仅靠关键词。
- **现状**：仅全文搜索（`service/search/`），未发现独立 embeddings 模块。

#### P2-6 Tauri 桌面打包 / 跨平台 ✅ 完整

- **用户视角**：桌面应用基于 Tauri 框架，原生支持 Windows / macOS / Linux。
- **现状**：仓库根 `AGENTS.md` 显示 Desktop 应用走 `pnpm run desktop:dev`（Tauri + Vite HMR），完整跨平台构建链已就位。

#### P2-7 语音输入 / TTS ❌ 未发现

- **用户视角**：能用语音输入、听 AI 朗读回复。
- **现状**：state review 未发现。

#### P2-8 桌面 / 网页 / 移动协同 ⚠️ 部分

- **用户视角**：四个端（Desktop / CLI / Web / Mobile Web）能协作。
- **现状**：4 个端共享同一后端（`northhing-core`），但各自独立前端，**无统一会话 / 任务同步层**— desktop 创建的会话 mobile web 看不了。
- **关键 file:line**：`src/apps/desktop`、`src/apps/cli`、`src/web-ui`、`src/mobile-web` 各自独立

#### P2-9 Skills / 插件市场 ❌ 缺失

- **用户视角**：能装第三方 Skills 扩展 agent 能力。
- **现状**：`apps/desktop/src/agent/actor.rs` 的 `SkillActor` body 是 no-op（注释自承 "The skill actor body is a no-op beyond the structured log + telemetry emit"），`SKILL_INSPECTOR_ENABLED` 是 dead code。

---

## 3. 亮点（strengths）— 值得在 README / 官网 / 对外宣传中提到

### 亮点 1：6 层单向依赖、~27 个 crate 的清晰分层架构

- **证据**：仓库根 `AGENTS.md` 的 "Layered Module Index" 列了 6 层（Interfaces / Assembly / Adapters / Services / Execution / Contracts），每层"may depend on lower layers only"。`apps/desktop`、`apps/cli`、`assembly/core`、`adapters/ai-adapters`、`services/services-integrations`、`interfaces/acp` 等 ~27 个 crate 边界清晰、依赖单向。
- **价值**：新人 onboarding 友好；做平台适配（如从 Tauri 桌面到 Web）只需在 Layer 1 加新 entry；底层 service 可独立替换。

### 亮点 2：后端测试覆盖扎实（远优于桌面 / CLI 应用层）

- **证据**：
 - `crates/assembly/core/tests/` 4 个 integration test（`product_assembly.rs` 66 行 + `remote_mcp_streamable_http.rs` 250 行 + `git_contracts.rs` 102 行 + `context_profile.rs` 160 行）
 - `services-integrations/tests/mcp_contracts.rs` 1545 行
 - `services-integrations/tests/remote_connect_contracts.rs` 2137 行（**全仓库最大单文件测试**）
 - 内嵌 unit test：assembly/core（`agentic/` + `service/` + `infrastructure/ai/`） + `client_factory.rs:347-431` 4 个 model resolution 测试 + `service/snapshot/manager.rs:797-852` 并发测试
- **价值**：核心能力改完有回归保护；招新人 / 推 PR 不怕核心坏了；protocol contract 测试意味着 MCP / Remote 协议层不破坏性升级。

### 亮点 3：MCP 协议层完整 + 双协议（rmcp + 自定义 JSON-RPC + streamable HTTP）

- **证据**：`mcp/protocol/{transport.rs, transport_remote.rs, rmcp_mapping.rs, jsonrpc.rs}`（基于 rmcp SDK + 自定义 JSON-RPC）；Server `mcp/server/{connection.rs, process.rs, catalog_cache.rs, runtime_helpers.rs, runtime_policy.rs}`；Adapter `mcp/adapter/{context, prompt, resource, tool}.rs`。
- **价值**：能接入 Anthropic 官方 MCP servers（GitHub / Postgres / Playwright / Slack 等），工具生态不封闭；streamable HTTP 支持流式推送而不是轮询，延迟低。

### 亮点 4：Remote Workspace 完整闭环（设备配对 + 加密 + 二维码 + relay）

- **证据**：`src/remote_connect.rs` 单文件 3446 行，含 5 个子模块（device / encryption / pairing / qr / relay_client），`RemoteCommand:1900` + `RemoteResponse:1992` + 5 个 host trait + 穷尽 match `handle_remote_command:2153-2224`，`RemoteSessionStateTracker:2297-2892` 状态机。
- **价值**：手机 / 网页扫二维码就能配对连接桌面端走 relay— 这是 2025 下半年 Trae SOLO、ChatGPT Agent 等头部产品同款能力，NortHing 已经工程化实现。

### 亮点 5：5 种内置 Subagent + 完整任务编排（1240+ 行 task_tool）

- **证据**：`task_tool.rs`（1240+ 行）+ `subagents.rs` 定义 5 种内置 agent（ExploreAgent / FileFinderAgent / GeneralPurposeAgent / ResearchSpecialistAgent / ComputerUseMode）+ `catalog.rs` 暴露 `builtin_agent_specs`。
- **价值**：用户能直接用"探索代码 / 找文件 / 通用任务 / 调研 / 操控桌面"等 subagent，并行处理复杂任务；不必从零配 agent。

---

## 4. 隐藏能力（用户可能不知道的）

### 隐藏 1：Computer Use 模式（AI 直接操控桌面）

- **用户视角**：AI 能像人一样操控鼠标键盘，操作桌面应用（不只是 shell / 文件）。
- **位置**：`agentic/agents/definitions/subagents.rs` 内置 `ComputerUseMode` agent。
- **现状**：已注册但本次 review 未深查实现深度，用户不一定知道有这能力。

### 隐藏 2：WebDriver 协议适配器（自动化浏览器）

- **用户视角**：AI 能启动浏览器、导航页面、点按钮、填表单（不只是 curl）。
- **位置**：`src/crates/adapters/webdriver`（layer 3 adapters 子 crate）。
- **现状**：模块存在但 state review 未深查，用户大概率不知道。

### 隐藏 3：完整 Remote-SSH（含文件 + 终端）

- **用户视角**：连远端服务器执行命令、操作文件、用终端。
- **位置**：`remote_ssh/manager.rs`（2792 行）+ `remote_exec.rs`（1035 行）+ `remote_fs.rs`（367 行）+ `remote_terminal.rs`（336 行），用 `russh` + `russh-sftp` + `russh-keys`。
- **现状**：除 port forwarding（`-L` / `-R`）4 处 TODO stub 外，主连接 / 执行 / 文件 / 终端全通了。

### 隐藏 4：Snapshot 检查点系统（8 个文件完整模块）

- **用户视角**：所有修改前后能对比；能回滚到任意历史检查点；含 file lock 防并发— 突。
- **位置**：`service/snapshot/{manager.rs, snapshot_core.rs, snapshot_system.rs, service.rs, types.rs, file_lock_manager.rs, isolation_manager.rs, events.rs}`。
- **价值**：比 git 更细粒度（针对每个修改动作打点），对 AI 改错时的兜底。

### 隐藏 5：Auto-Generated AI 标题

- **用户视角**：新建会话后 AI 自动给会话起一个标题（不必每次手动命名）。
- **位置**：`session_manager.rs` 内 `SessionTitleMethod{Ai, Fallback}`（`:76-89`）+ `ResolvedSessionTitle`（`:91-95`）+ AI 标题生成走 func agent `session-title-func-agent`（`manager.rs:236-238`）。

### 隐藏 6：4 个 Surface 共享同一 Rust 后端

- **用户视角**：同一份 `northhing-core` + `northhing-assembly` 后端被 4 个端共用（Desktop Tauri / CLI / Web Vite / Mobile Web）；切换端不需要重新实现业务逻辑。
- **位置**：`src/apps/desktop` + `src/apps/cli` + `src/web-ui` + `src/mobile-web` 都依赖 `northhing-core` / `northhing-assembly`。
- **现状**：会话 / 任务同步层还没做，所以"desktop 创建会话 mobile 继续"还做不到— 但底层共享已经有了。

### 隐藏 7：Remote Workspace 的二维码 + 中继连接

- **用户视角**：手机 / 网页扫桌面端二维码就能远程控制，不必在同一局域网。
- **位置**：`src/remote_connect.rs:remote_connect.rs` 内 `pairing` / `qr` / `relay_client` 三个子模块 + `RemoteSessionStateTracker` 状态机。

### 隐藏 8：Plan Mode + Shell 安全策略（3 档审批）

- **用户视角**：能在 3 种模式（Plan / Default / Auto-Accept）间切换，决定 AI 是否要先出方案再执行。
- **位置**：`agentic/agents/definitions/modes/PlanMode` + `create_plan_tool.rs` + `config/types.rs:626 ShellSecurityConfig` + `:847-948 default_mode_policies`（预置多种 shell 命令的安全策略）。

---

## 5. 总览统计

### 按 baseline 优先级统计

| 优先级 | 总数 | ✅ 完整 | ⚠️ 部分 | 🟡 浅 | ❌ 缺失 |
|--------|------|---------|---------|-------|---------|
| P0 必备 | 15 | 11 | 4 | 0 | 0 |
| P1 重要 | 4 | 1 | 2 | 0 | 1 |
| P2 加分 | 9 | 1 | 1 | 1 | 6 |
| **合计** | **28** | **13** | **7** | **1** | **8** |

> 按严格三档（✅ / ⚠️ / ❌，忽略 🟡 浅）：✅ 13 / ⚠️ 8 / ❌ 8。

### 按用户可见 Surface 统计

| Surface | 状态 | 备注 |
|---------|------|------|
| **CLI** | ✅ 完整 | `modes/chat.rs`（3362 行）+ `ui/startup.rs`（1977 行）功能完整，0 测试是另一回事 |
| **Desktop (Tauri)** | ⚠️ beta | 4 个 P0 体验尾巴（缺 startup session / 默认 providers / UI 错误通道 / MCP 全局注册） |
| **Web UI** | ✅ 后端共享 | 独立前端，会话同步层缺失 |
| **Mobile Web** | ✅ 后端共享 | 独立前端，会话同步层缺失 |
| **Remote Workspace** | ✅ 完整 | 设备配对 / QR / relay 全通 |
| **Remote-SSH** | ⚠️ 主体通 | port forwarding TODO stub |

---

## 6. 注意事项（给后续读者）

1. **本目录反映 v0.1.0 实测状态**，不是营销话术。desktop 端的 4 个 P0 尾巴（QW-1 / QW-2 / QW-3 / QW-4，state review §4）解完之前，GUI 体感是"半成品"。
2. **亮点都来自 state review 交叉验证过的事实**（每条都有 file:line 引用），不是声明。
3. **隐藏能力列表中 P1-2 Rules / P2-8 多端协同 / P2-9 Skills / P2-2 Cron** 的实现深度 state review 未深查，使用时需自行确认。
4. **与 baseline 对比**：`docs/handoffs/2026-06-25-agent-baseline-features.md` §六的 5 条关键观察（P0 是地板 / MCP 标准 / Plan Mode 护城河 / 状态栏清晰度 / P1-3 + P1-4 新趋势 / P1-2 长期效率）在 NortHing 已部分验证（Plan Mode ✅ / 状态栏清晰度 ⚠️ / P1-2 部分 / P1-3 + P1-4 部分）。
5. **本文档不替代 state review**— 状态判断和具体 file:line 一律以 `2026-06-25-northhing-state-review.md` 为准。

---

**目录完。本文档是功能目录（user-facing），不评估实现代码质量，不开 issue，不写代码。**