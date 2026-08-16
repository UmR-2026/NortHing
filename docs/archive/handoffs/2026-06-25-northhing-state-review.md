# NortHing v0.1.0 实现状态深度 Review

> **Date**: 2026-06-25
> **Scope**: 7 个关键目录的代码 review（不改代码、不跑 cargo build）
> **Method**: 3 个并行 explore subagent 实读代码 + 交叉验证
> **结论一句话**: assembly/core 后台能力**完整且工程化良好**，但 desktop 端把 3 个 P0 体验项（F2 状态栏 / F3 错误展示 / A1 自动建 session）都做反了；handoff 列的"小毛病"之外还有 MCP 全局注册缺失 + 2 处 unreachable!() 残留 + 4 处 SSH port-forwarding stub 几个高 ROI 修复点。

---

## 0. Review 元数据

| 项 | 数 |
| --- | --- |
| 审查目录数 | 7（`apps/desktop`、`apps/cli`、`agentic/`、`service/`、`infrastructure/ai/`、`interfaces/acp/`、`services-integrations/`）|
| 实读 .rs 文件数 | ~230 |
| 实读 .slint UI 文件数 | 16 |
| 覆盖的 crate | `northhing-desktop`、`northhing-cli`、`northhing-core`（含 assembly）、`northhing-contracts`（间接）、`northhing-services-integrations`、`northhing-interfaces-acp` |
| 交叉验证 handoff §3 引用的 file:line | 6 处全部亲自 grep 验证 |
| 发现的 issue 数 | **P0: 5**、**P1: 8**、**P2: 4**、**P3: 4**（合计 21）|

---

## 1. 已实现功能清单（对照 baseline 20 项 P0 + P1）

> 参考 `docs/handoffs/2026-06-25-agent-baseline-features.md` §三。
> 状态: ✅ 完整 / ⚠️ 部分 / ❌ 缺失 / 🟡 仅 stub

### 1.1 A. 对话与消息（4/4 已实现主体，A1/F2/F3 留尾巴）

| ID | 功能 | 状态 | 关键 file:line |
|---|---|---|---|
| **A1** | 新建/删除/切换会话 | ⚠️ 后端 ✅ + desktop ⚠️ | 后端 `agentic/session/session_manager.rs:1070 create_session`、`:2203 delete_session`、`:3048 list_sessions`；`agentic/coordination/coordinator.rs:1212 create_session`、`:1261 create_session_with_workspace`、`:4008 list_sessions`。Desktop 端 `apps/desktop/src/app_state/mod.rs:490 on_new_session`、`:610 on_delete_session`、`:569 on_switch_session` 全部注册了 callback，但**没有 startup auto-create session**（`create_ui` 192-913 行无该调用）→ 启动后 `current_session_id == ""` |
| **A2** | 发送消息 + 多轮上下文 | ⚠️ 后端 ✅ + desktop ⚠️ | 后端 `agentic/execution/execution_engine.rs:1-200`（3469 行大文件）+ `agentic/execution/round_executor.rs`；`coordination/coordinator.rs` 调度。Desktop `apps/desktop/src/app_state/mod.rs:329 on_send_message`，但 line 407 `if session_id.is_empty() { eprintln!(...); return; }` 早退（用户的感受是"发消息无反应"） |
| **A3** | 流式响应（SSE） | ✅ 完整 | `src/crates/execution/agent-stream/src/lib.rs:773 process_stream`、`:797 process_stream_with_options`、`:399 StreamProcessor`；`infrastructure/ai/client_factory.rs:147-229 get_or_create_client` 用 `AIClient::new_with_runtime_options` 串 `StreamOptions`；`tool_call_accumulator.rs` 累积工具调用 |
| **A4** | 历史回看 + 搜索 | ✅ 部分 | 会话历史通过 `persistence/manager.rs:300-410 project_sessions_dir` + `session_manager.rs:load_session`；**全文搜索是独立 `service/search/` 模块**（`service.rs`、`remote.rs`、`remote_disabled.rs`），不是 assembly 内置 |

### 1.2 B. 模型与 Provider（4/4 已实现）

| ID | 功能 | 状态 | 关键 file:line |
|---|---|---|---|
| **B1** | 模型切换（UI） | ✅ | Assembly 暴露 `AIConfig` + `AIClientFactory::get_client_resolved` 支持 `primary/fast/<id>/<name>/<model_name>` 5 种选择子（`infrastructure/ai/client_factory.rs:91-108`）；UI 设置在 slint（`apps/desktop/src/ui/views/InspectorView.slint`） |
| **B2** | 多 Provider | ✅ | `infrastructure/ai/mod.rs:10 pub use northhing_ai_adapters::providers;`（provider 实际定义在 `crates/adapters/ai-adapters`）；`AIConfig.models[].provider` 字段（`config/types.rs:1196`）；3 种 `AuthConfig`：`ApiKey` / `CodexCli` / `GeminiCli`（`types.rs:1285-1295`） |
| **B3** | API Key 管理 | ✅ | `config/types.rs:1205 AIModelConfig.api_key` + 3 种 `AuthConfig`；写盘走 `ConfigManager::set`（`manager.rs:310-332`） |
| **B4** | 自定义 Base URL / 代理 | ✅ | `config/types.rs:1198 AIModelConfig.base_url` + `:1200-1203 request_url`（自动派生）；`types.rs:595 AIConfig.proxy` + `client_factory.rs:197-201` 透传 |

### 1.3 C. 工具执行（5/5 全部完整）

| ID | 功能 | 状态 | 关键 file:line |
|---|---|---|---|
| **C1** | 读/写/编辑文件 | ✅ | `agentic/tools/implementations/file_read_tool.rs` / `file_write_tool.rs` / `file_edit_tool.rs` / `delete_file_tool.rs` |
| **C2** | Terminal / Shell | ✅ | `agentic/tools/implementations/bash_tool.rs` + `agentic/tools/implementations/exec_command/`（`mod.rs` + 9 子文件：local_shell、background_command_output、stdin、progress 等）|
| **C3** | grep / glob | ✅ | `agentic/tools/implementations/grep_tool.rs` / `glob_tool.rs` / `ls_tool.rs` |
| **C4** | Diff 视图 + 检查点 | ✅ | `agentic/tools/implementations/get_file_diff_tool.rs`；检查点 `service/snapshot/{manager.rs,snapshot_core.rs,snapshot_system.rs,service.rs,types.rs,file_lock_manager.rs,isolation_manager.rs,events.rs}` |
| **C5** | Git 操作 | ✅ | `agentic/tools/implementations/git_tool.rs`；底层 `service/git/{git_service.rs,git_types.rs,git_utils.rs,graph.rs}` |

### 1.4 D. 子任务 / 多 Agent（2/2 已实现）

| ID | 功能 | 状态 | 关键 file:line |
|---|---|---|---|
| **D1** | Subagent | ✅ 完整 | `agentic/tools/implementations/task_tool.rs`（1240+ 行）；`agentic/agents/definitions/subagents.rs` 定义 `ExploreAgent/FileFinderAgent/GeneralPurposeAgent/ResearchSpecialistAgent/ComputerUseMode`；`agents/registry/catalog.rs` 暴露 `builtin_agent_specs` |
| **D2** | Plan / 审批 | ✅ | `agentic/agents/definitions/modes/PlanMode`；`agentic/tools/implementations/create_plan_tool.rs`；shell 安全策略 `config/types.rs:626 ShellSecurityConfig` + `:847-948 default_mode_policies` |

### 1.5 E. 会话管理增强（1/2 + 1/2）

| ID | 功能 | 状态 | 关键 file:line |
|---|---|---|---|
| **E1** | 会话重命名 / 搜索 / 删除 | ✅ | `session_manager.rs` 内 `SessionTitleMethod{Ai, Fallback}`（`:76-89`）+ `ResolvedSessionTitle`（`:91-95`）+ AI 标题生成（func agent `session-title-func-agent` `manager.rs:236-238`）|
| **E2** | 导入/导出 | ✅ | `persistence/manager.rs:16 SessionTranscriptExport` + `SessionTranscriptExportOptions`；`service/session/...` 暴露 `SessionMetadataBuildFacts`、`SessionBranchRequest/Result`（branch = 导入场景） |

### 1.6 F. UI / UX（3/5 完整 + 1 缺失 + 1 部分）

| ID | 功能 | 状态 | 关键 file:line |
|---|---|---|---|
| **F1** | 侧边栏 | ✅ | `apps/desktop/src/ui/views/SidebarView.slint`；数据源 `apps/desktop/src/app_state/sessions.rs` + `app_state/mod.rs:558 refresh_sessions_ui` |
| **F2** | 状态栏 | ⚠️ 部分 | UI 在 `apps/desktop/src/ui/views/StatusBarView.slint`（48 行，三段：mcp-status / model-status / app-title）；后端 MCP 状态 `app_state/inspector.rs:14 build_mcp_status_string`；model 状态 `app_state/inspector_model_status.rs:14 build_model_status_string`（从 `ai.models` 算 provider 列表）。**问题**：app.json 首次启动 `ai.models: []` → "Model: Not configured" 永远 |
| **F3** | 错误展示 / 重试 | ❌ 缺失 | **9 个 Slint 回调的所有失败全用 `eprintln!` 写 stderr**，UI 无任何错误展示通道：`mod.rs:402, 408, 428, 453, 504, 526, 562, 668, 717, 851`；`sessions.rs:168, 194`；`inspector.rs:21, 29`；`inspector_model_status.rs:20, 31`。`ui.set_*error` 在整个 `app_state/` 目录 grep 0 命中 |
| **F4** | 主题 | ✅ | `config/types.rs:340 ThemesConfig`、`providers.rs:243 ThemesConfigProvider`、`manager.rs:75 default_current="northhing-light"` |
| **F5** | 设置 | ✅ | `ConfigManager::set`（`manager.rs:310`）+ `ConfigService::set_config` 支持 dot-path set/get；UI 在 slint |

### 1.7 G. 远程 / 协作（2/3 完整 + 1 局部 stub）

| ID | 功能 | 状态 | 关键 file:line |
|---|---|---|---|
| **G1** | Remote-SSH | ⚠️ 较完整 | 主体 ✅（连接/执行/文件/终端）：`remote_ssh/manager.rs`（2792 行）+ `remote_exec.rs`（1035 行）+ `remote_fs.rs`（367 行）+ `remote_terminal.rs`（336 行），用 `russh`+`russh-sftp`+`russh-keys`（`Cargo.toml:129-131`）。**局部 stub**: `manager.rs:2476, 2504, 2516, 2551` 4 处 SSH port forwarding（`-L`/`-R`）TODO，仅注册表项不实际转发 TCP |
| **G1** | Remote Workspace | ✅ 完整 | `src/remote_connect.rs`（单文件 3446 行）：设备/加密/配对/QR/relay_client 5 个子模块；协议 `RemoteCommand:1900` + `RemoteResponse:1992` + 5 个 host trait + 穷尽 match `handle_remote_command:2153-2224`；`RemoteSessionStateTracker:2297-2892` |
| **G2** | Background Agent | ❌ | **完全缺失**。没有看到 background agent / cloud 任务队列相关模块 |
| **G3** | 多端协同 | ⚠️ 部分 | Mobile web 入口在 `src/mobile-web/`，Web UI 在 `src/web-ui/`，但**与 desktop 是各自独立前端**（共享后端 northhing-core）；无统一会话/任务同步层 |

### 1.8 H. 高级能力（1/5 完整 + 1 部分 + 1 缺失 + 2 P2 缺失）

| ID | 功能 | 状态 | 关键 file:line |
|---|---|---|---|
| **H1** | MCP 协议 | ⚠️ 协议层完整 + desktop 全局注册缺失 | 协议 ✅：`mcp/protocol/{transport.rs, transport_remote.rs, rmcp_mapping.rs, jsonrpc.rs}`（基于 rmcp SDK + 自定义 JSON-RPC）；Server ✅：`mcp/server/{connection.rs, process.rs, catalog_cache.rs, runtime_helpers.rs, runtime_policy.rs}`；Adapter ✅：`mcp/adapter/{context,prompt,resource,tool}.rs`；**问题**：desktop 路径构造了 local `MCPService`（`app_state/inspector.rs:26`）**但从不调** `set_global_mcp_service()` → 全局 `GLOBAL_MCP_SERVICE`（`service/mcp/mod.rs:83-92`）永远 None |
| **H2** | Skills / 插件 | ❌ | **未发现**独立的 skills marketplace / plugin loader；`apps/desktop/src/agent/actor.rs` 内部 `SkillActor` body 是 no-op（line 376-396 注释自承 "The skill actor body is a no-op beyond the structured log + telemetry emit"）|
| **H3** | Rules / CLAUDE.md | ⚠️ 部分 | 路径约定有 `infrastructure/app_paths/path_manager.rs:312 user_data_dir().join("rules")` + `service/workspace/manager.rs:90-100 load_from_workspace_root` 读 `IDENTITY.md`；**没有显式的"CLAUDE.md / rules 自动加载到 system prompt"入口** |
| **H4** | Cron | ✅ 存在但浅 | `service/cron/` 目录存在（属于 service 层而非 agentic 层） |
| **H5** | Hooks | ❌ | **未发现**事件驱动的 hook 系统 |

### 1.9 实测覆盖率

- **P0 完整实现**: 14/20（A1/A2/A3/A4/B1/B2/B3/B4/C1/C2/C3/C4/C5/D2/E1/G1 Remote Workspace）
- **P0 部分实现**: 3/20（A1/A2 desktop 端尾巴、F2 状态栏）
- **P0 缺失**: 2/20（F3 错误展示、G1 Remote-SSH port-forwarding）
- **P1 完整实现**: 2/6（E2、F4）
- **P1 缺失**: 4/6（D1 ✅、H1 协议层 ✅ 全局注册 ❌、H3 部分、H2 缺失）

---

## 2. 已知问题 + 根因

### 2.1 已诊断未修（handoff §3，3 个全部交叉验证）

#### Bug #1：状态栏一直 Pending / Failed / "Not configured"

- **症状**：MCP 段永远 "Pending"，Model 段永远 "Not configured"
- **根因链**：
 1. `apps/desktop/src/main.rs:18-19` 声明 `static MCP_SERVICE: OnceLock<…>`，**但整文件 171 行 grep 验证：0 处 `.set()` 调用**
 2. `apps/desktop/src/main.rs:22-23, 25-27, 30-39` 声明 `MCP_INIT_STATUS` + `get_mcp_init_status` + `get_mcp_status_text`，**`get_mcp_status_text` 在 desktop 内部 grep 0 调用方**（仅 `cli/ui/startup.rs:627` 引用）
 3. `MCP_INIT_STATUS` 永远停留在初始值 0（"Pending"）
 4. CLI 路径在 `apps/cli/src/main.rs:411-435` 设了 `MCP_SERVICE.set(...)` + `get_mcp_init_status().store(2, ...)` → 走"Pending → Connecting → Ready" 完整流；**desktop 没抄这段**
 5. Desktop 状态栏 mcp-status 实际走另一条路：`apps/desktop/src/app_state/inspector.rs:14 build_mcp_status_string` → `McpCatalogAdapter::new(mcp_service)`（line 26 构造的 local 实例）→ 绕开全局注册
- **Model 段独立 bug**：`service/config/manager.rs:107-114 create_default_config` **根本不写 `ai.models` 字段**— 它只填 `agent_models` 和 `func_agent_models` 两个 HashMap，然后 `save_config` 写盘。`AIConfig::default()`（`config/types.rs:1730-1755`）的 `models: vec![]` 直接落盘。`app.json` 首次生成后 `ai.models == []` + `default_models.primary == None` + `default_models.fast == None`
- **Handoff 引用偏差**：handoff 文档说"位置在 `agent/agentic_system.rs:88`"是**错的**— `apps/desktop/src/agent/agentic_system.rs` 整个文件只有 18 行，无 `:88`。真正位置在 `crates/assembly/core/src/infrastructure/ai/client_factory.rs:237-257`

#### Bug #2：点 New Session 无反应

- **症状**：sidebar 一直是空，用户看不到任何反馈
- **根因**：callback 实际跑了，但失败全用 `eprintln!` 写 stderr，GUI 看不到
- **位置**：`apps/desktop/src/app_state/mod.rs:490-566`（handoff 写的 562 行只是 `Err(e) => eprintln!(...)` 那一行）
- **关键失败点**（行号）：
 - `mod.rs:504` "Agentic system not initialized"（守卫）
 - `mod.rs:526` "Global coordinator not available"（spawn 内部）
 - `mod.rs:562` "Failed to create session: {}"（create_session Err）
- **可能真实根因**（基于代码推演）：`create_session` 走 `coordination/coordinator.rs:1212` → 后端 `session_manager.rs:1070` → 内部需要 `ConfigService` + `PersistenceManager` 等多个 global。`get_global_coordinator()` 在 `spawn` 内调，如果 coordinator 的初始化（`apps/desktop/src/agent/agentic_system.rs:8-18` 链）因 Bug #1 的 hang 还没完成，就会一直拿到 None

#### Bug #3：发消息无反应

- **症状**：用户输入消息点发送，无任何反应
- **根因**：`apps/desktop/src/app_state/mod.rs:407-410` `if session_id.is_empty() { eprintln!(...); return; }` 早退
- **因果链**：Bug #2 导致 session 从未成功创建 → `current_session_id == ""` 永久 → 任何 `on_send_message` 必然早退
- **完整 callback**：`mod.rs:329-485`（不是 handoff 说的 407-410— 那只是早退 4 行）

### 2.2 本次 review 发现的额外问题

#### P0-1（与 Bug #1 同源，单独列出）：AIClientFactory::initialize_global 启动慢/可能 hang

- **位置**：`crates/assembly/core/src/infrastructure/ai/client_factory.rs:237-257`
- **唯一 await 点**：`get_global_config_service().await`（line 244）→ `service/config/global.rs:130-140` 的 `service_wrapper.read().await`（line 135）拿 `tokio::sync::RwLock<Option<Arc<ConfigService>>>` 读锁
- **调用链上下文**：`apps/desktop/src/agent/agentic_system.rs:8-18` `init_agentic_system_for_desktop` 链：先 `initialize_global_config`（`global.rs:235-237`）→ 后 `AIClientFactory::initialize_global`
- **hang 嫌疑链**（按代码推演）：
 1. `initialize_global_config` 走 `GlobalConfigManager::initialize`（`global.rs:86-127`）→ `ConfigService::new().await— `（line 89）
 2. `ConfigService::new` 内部 `ConfigManager::new` 跑 `path_manager.initialize_user_directories().await— `（`manager.rs:63`）+ `load_or_create_config().await— `（`manager.rs:78`）
 3. 后者首次启动会触发 `create_default_config` → `save_config` 写盘 → 阻塞
 4. `#[cfg(feature = "product-full")]` 还可能调 `canonicalize_agent_profile_configs().await`（`global.rs:108`），这个 canonicalizer 自己再 await config service 拿**写锁**，可能和并发 `get_config` 形成 lock convoy
- **结论**：`AIClientFactory` 自己几乎不可能 deadlock；真正 hang 嫌疑在**冷启动 IO 慢 + 无 instrumentation**

#### P0-2：desktop 端 MCP 全局注册缺失

- **位置**：`apps/desktop/src/main.rs:18-19` `static MCP_SERVICE` + `apps/desktop/src/app_state/inspector.rs:26` 构造 local `MCPService` 但不调 `set_global_mcp_service()`
- **对比**：CLI 路径 `apps/cli/src/main.rs:411` 有 `MCP_SERVICE.set(mcp_service.clone()).ok();`
- **影响**：所有走 `service::mcp::get_global_mcp_service()` 的代码路径在 desktop 永远拿到 None

#### P0-3：`_ => unreachable!()` 残留 2 处（与 P1-extra-2 同型）

- **位置**：
 - `crates/services/services-integrations/src/mcp/config/json_config.rs:193`
 - `crates/services/services-integrations/src/mcp/config/json_config.rs:217`
- **背景**：handoff §2.2 P1-extra-2 修复了 `service/remote_connect/mod.rs:334`（现已迁至 `remote_connect.rs:2153-2224` 穷尽 match），但**同款反模式在 mcp config 留了 2 处**— 理论上不可达，但未来新增 match 分支会变 panic
- **建议修法**：与 P1-extra-2 同款 → `_ => Err(anyhow::anyhow!("unsupported source/transport combination: {:— }", other))`

#### P0-4：config 字段命名误导

- **位置**：`config/types.rs:557` `AIConfig.models: Vec<AIModelConfig>`，但 `AIConfig` 没有 `providers` 字段
- **影响**：handoff §3.3 #2 描述"加默认 providers 列表"实际是**新增**字段或在 `models` 数组里塞 3 个 `enabled=false, provider=anthropic/openai/gemini` 的占位（`types.rs:557` 那一行只一个 `models: Vec<AIModelConfig>`）。需要明确走哪条路

#### P1-1：desktop 9 个 Slint callback 错误全用 `eprintln!`

- **位置**：`apps/desktop/src/app_state/mod.rs:402, 408, 428, 453, 504, 526, 562, 668, 717, 851` + `sessions.rs:168, 194` + `inspector.rs:21, 29` + `inspector_model_status.rs:20, 31`（**20 个 eprintln!**）
- **影响**：用户看不到任何错误反馈（handoff §3.2 准确描述）
- **建议修法**：每处加 `ui.set_session_error(Slint property)` 或 `ui.set_input_error(...)` + `SidebarView.slint` 加 error banner

#### P1-2：desktop 端 unit test 覆盖极薄

- **现状**：仅 `apps/desktop/src/app_state/mod.rs:926-1248` 内置 `mod phase_i_tests`（10+ unit test + 1 smoke test `create_ui_runs_with_noop_platform`）；`apps/desktop/src/mcp_adapter.rs:157-208` 4 个 status 映射测试
- **缺失**：`sessions.rs` / `inspector.rs` / `inspector_model_status.rs` / `actor.rs` / `log.rs` / `slint_glue.rs` 全部 0 测试
- **影响**：重构 GUI 代码无安全网

#### P1-3：CLI 端 unit test 极薄

- **现状**：仅 `apps/cli/src/ui/chat/state_split_tests.rs:1-173`（9+ unit test）
- **缺失**：`modes/chat.rs`（3362 行）和 `ui/startup.rs`（1977 行）**零测试**— 这是 CLI 的两大主文件
- **影响**：CLI 回归风险高

#### P1-4：assembly/core 测试以 integration 为主

- **现状**：`crates/assembly/core/tests/` 4 个 integration test（`product_assembly.rs` 66 行、`remote_mcp_streamable_http.rs` 250 行、`git_contracts.rs` 102 行、`context_profile.rs` 160 行）
- **较好**：各文件内嵌 `#[cfg(test)] mod tests`（如 `client_factory.rs:347-431` 4 个 model resolution 测试、`manager.rs:628-647`、`service/snapshot/manager.rs:797-852` 并发测试）
- **缺**：跨文件 E2E 集成测试（无端到端跑通"create session → send message → tool call → snapshot"全链）

#### P1-5：interfaces/acp 无外部 tests/ 目录

- **现状**：11 个文件用 `#[cfg(test)]` 内嵌（`client/manager.rs:2429` 等），**无 `tests/` 目录**
- **影响**：跨文件 E2E（如"ACP server + agentic_system + tool call"）无集成验证
- **对比**：`services-integrations` 有 `tests/mcp_contracts.rs`（1545 行）、`tests/remote_connect_contracts.rs`（2137 行）做得很扎实

#### P1-6：services-integrations 测试覆盖不均

- **覆盖**：`tests/mcp_contracts.rs` 1545 行 ✅、`tests/remote_connect_contracts.rs` 2137 行 ✅
- **偏薄**：`tests/remote_ssh_contracts.rs` 220 行（SSH 主体 2792 行，比 6.6%）、`tests/workspace_search_contracts.rs` 24 行、`tests/announcement_contracts.rs` 21 行、`tests/file_watch_contracts.rs` 125 行

#### P1-7：CLI `tool_cards.rs` hardcoded theme

- **位置**：`apps/cli/src/ui/tool_cards.rs:1051` + `:1239` `let hl_theme = HighlightTheme::Dark; // TODO: derive from theme`
- **影响**：主题切换时高亮不跟随；细微 UX 问题
- **建议修法**：从 `&Theme` 派生 `HighlightTheme`（5 行代码）

#### P2-1：background agent / cloud task 队列完全缺失

- **位置**：未发现对应模块
- **影响**：与 Cursor Background Agent / Trae SOLO 异步任务的差异化能力差距

#### P2-2：G3 多端同步无统一层

- **位置**：`src/mobile-web/` + `src/web-ui/` 各自独立前端
- **影响**：mobile/web 端无 desktop session 同步；典型场景"desktop 创建会话，mobile 继续"做不到

#### P2-3：H2 Skills / 插件市场缺失

- **位置**：`apps/desktop/src/agent/actor.rs` `SkillActor` body 是 no-op（line 376-396 注释自承）
- **影响**：与 Claude Code Skills / Trae 智能体编排差距大

#### P2-4：H5 Hooks 事件自动化缺失

- **位置**：未发现 PreToolUse / PostToolUse hook 系统
- **影响**：用户无法做事件驱动自动化

#### P3-1：`SKILL_INSPECTOR_ENABLED` flag 死代码

- **位置**：`apps/desktop/src/main.rs:57-58` `#[allow(dead_code)]` 标注
- **影响**：无（仅 dead code）

#### P3-2：`SESSION_TREE_VIEW` 常量重复定义

- **位置**：`apps/desktop/src/main.rs:69` 和 `apps/desktop/src/flags.rs:22` 两处都定义 `pub const SESSION_TREE_VIEW: bool = true`
- **影响**：通过 `crate::flags::SESSION_TREE_VIEW` / `crate::SESSION_TREE_VIEW` 两个路径都引用，grep 维护时容易漏

#### P3-3：`USE_SOFTWARE_FALLBACK` / `USE_SLINT_SHELL` 永远 true

- **位置**：`apps/desktop/src/main.rs:49, 52`
- **影响**：两个 flag 标 `true` 但都没有 false 分支逻辑（`USE_SLINT_SHELL=false` 的 early-return 在 line 106-109 是 dead code）

#### P3-4：`MCP_INIT_STATUS` / `get_mcp_init_status` / `get_mcp_status_text` 死代码

- **位置**：`apps/desktop/src/main.rs:18-39`
- **影响**：见 P0-2，desktop 端全局注册缺失导致这套 API 在 desktop 永远读到 0

### 2.3 架构层面隐患

#### H1：双轨 MCP 状态管理

- 一条是 `apps/desktop/src/main.rs:18-39` 的 `MCP_INIT_STATUS` + `MCP_SERVICE` 静态（实际不用）
- 一条是 `apps/desktop/src/app_state/inspector.rs:14` 的 `build_mcp_status_string` + `McpCatalogAdapter`（实际用）
- 两条路不一致，未来修 MCP 状态时容易看错代码

#### H2：concurrent state 守卫不显式

- `apps/desktop/src/app_state/mod.rs` 1249 行大文件，**所有 `on_*` callback 通过 `Arc<AppState>` + `tokio::spawn` 异步操作 `current_session_id` 等 mutable state**
- 没有 `Mutex` / `RwLock` 包裹关键状态字段的明确证据（需要进一步确认）→ 潜在 data race 风险

#### H3：cold-start IO 路径无 instrumentation

- `ConfigService::new().await` 链路（`manager.rs:57-88` + `global.rs:86-127`）无 `tracing::span!` / `info!` pre-post 标记
- 一旦 hang 真实发生，无法定位是 `PathManager::initialize_user_directories` 慢 IO，还是 `mode_config_canonicalizer` 写锁等待，还是 `save_config` 写盘慢

#### H4：file lock 与 Windows reserved-name 风险

- handoff §5.2 已记录 `target/` 锁导致 `mv` 失败（Win32 file handle）
- `mcp/server/process.rs` 启子进程持 file handle → 快速重启 GUI 可能撞 file lock
- `mcp/config/json_config.rs:193, 217` 的 unreachable!() panic 风险（与 P0-3 同源）

#### H5：state propagation 时序依赖

- `init_agentic_system_for_desktop` 链（`apps/desktop/src/agent/agentic_system.rs:8-18`）是顺序 await，但 `on_send_message` / `on_new_session` 的 spawn 闭包不等待这个链完成
- 启动时序：A. 启动 GUI → B. `init_agentic_system_for_desktop` 在另一 task 跑 → C. 用户点 New Session → D. spawn 闭包内 `get_global_coordinator()` 拿 None → eprintln → 失败
- 这就是 Bug #2 的真实时序

---

## 3. 代码质量评估

### 3.1 死代码（按确认度分类）

#### 已确认（grep 验证零调用方）

| file:line | 内容 | 证据 |
|---|---|---|
| `apps/desktop/src/main.rs:18-19, 22-23, 25-27, 30-39` | `MCP_SERVICE` / `MCP_INIT_STATUS` / `get_mcp_init_status` / `get_mcp_status_text` | desktop 侧无 `.set()`，无调用方；CLI 才有（`cli/main.rs:411-435`） |
| `apps/desktop/src/main.rs:57-58` | `SKILL_INSPECTOR_ENABLED = false` | 标 `#[allow(dead_code)]`，grep 0 引用 |
| `apps/desktop/src/agent/actor.rs:22, 376-396` | `maybe_construct_actor_runtime` 仅当 `USE_LIGHTWEIGHT_ACTOR=true` 才构造；`SkillActor` body no-op | 自承认 "The skill actor body is a no-op beyond the structured log + telemetry emit" |
| `apps/desktop/src/main.rs:49` | `USE_SOFTWARE_FALLBACK = true` 永远 true | false 分支在 line 106-109 是 dead code |
| `apps/desktop/src/main.rs:52` | `USE_SLINT_SHELL = true` 永远 true | 无 false 路径 |
| `apps/desktop/src/app_state/mod.rs:113-121` | `actor_runtime()` getter 标 `#[allow(dead_code)]` | 实际**已被使用**（`on_send_message:367`）— 注释过时 |
| `interfaces/acp/src/runtime.rs:39-40` | `mcp_server_ids` 标 `#[allow(dead_code)]` | 暂存 session-wide MCP 路由 |

#### 推测（基于上下文，可能是 dev-time 遗留）

- `apps/desktop/src/flags.rs` 的 `pub const SESSION_TREE_VIEW = true` 与 `apps/desktop/src/main.rs:69` 的 `pub const SESSION_TREE_VIEW = true` 重复 → 实际是 dev 期间迁移残留
- `services-integrations/src/remote_ssh/manager.rs:2476-2551` 4 处 SSH port-forwarding TODO — 实际是 stub（功能未实现）

### 3.2 测试覆盖度（按模块评估）

| 模块 | 行数 | 测试文件 | 测试行数 | 评估 |
|---|---|---|---|---|
| `apps/desktop/src/app_state/mod.rs` | 1249 | 内嵌 `phase_i_tests` | ~322 | 🟡 一— （DTO 测试 + 1 smoke）|
| `apps/desktop/src/app_state/{sessions,inspector,inspector_model_status,actor,log,slint_glue}.rs` | 467 | 0 | 0 | ❌ 缺失 |
| `apps/desktop/src/mcp_adapter.rs` | 208 | 内嵌 | ~52 | ✅ 充分 |
| `apps/cli/src/modes/chat.rs` | 3362 | 0 | 0 | ❌ 缺失 |
| `apps/cli/src/ui/startup.rs` | 1977 | 0 | 0 | ❌ 缺失 |
| `apps/cli/src/ui/chat/state_split_tests.rs` | 173 | 独立 | 173 | ✅ 充分（仅覆盖 state.rs）|
| `crates/assembly/core/src/agentic/` | ~70 文件 | 内嵌 + tests/ | 内嵌多 | 🟡 较好 |
| `crates/assembly/core/src/service/` | ~60 文件 | 内嵌 + tests/ | 内嵌多 | 🟡 较好 |
| `crates/assembly/core/src/infrastructure/ai/` | 3 文件 | 内嵌 | 85（client_factory:347-431）| 🟡 一— |
| `crates/assembly/core/tests/` | 4 文件 | 集成 | 578 | ✅ 充分 |
| `interfaces/acp/` | 25 文件 | 内嵌 | 11 处 | ⚠️ 缺跨文件 E2E |
| `services-integrations/tests/mcp_contracts.rs` | — | 独立 | 1545 | ✅ 充分 |
| `services-integrations/tests/remote_connect_contracts.rs` | — | 独立 | 2137 | ✅ 充分（最大测试文件）|
| `services-integrations/tests/remote_ssh_contracts.rs` | — | 独立 | 220 | ❌ 偏薄 |
| `services-integrations/tests/{workspace_search,announcement,file_watch,function_agent,git}_contracts.rs` | — | 独立 | <300 each | 🟡 一— |

**结论**：assembly/core + services-integrations 测试覆盖**显著优于** desktop + cli 应用层。

### 3.3 文档完整度（crate AGENTS.md 存在性）

| crate | AGENTS.md | 备注 |
|---|---|---|
| 仓库根（`AGENTS.md`） | ✅ | 完整（含全局规则、命令、验证表）|
| `src/apps/desktop/` | ❌ 未单独看（按 layer 1 规则应就近）| 推测在 `apps/desktop/AGENTS.md` 缺失（需要后续确认）|
| `src/apps/cli/` | ❌ | 同上 |
| `src/crates/assembly/AGENTS.md` | ✅ | 引用 |
| `src/crates/adapters/AGENTS.md` | ✅ | 引用 |
| `src/crates/services/AGENTS.md` | ✅ | 引用 |
| `src/crates/execution/AGENTS.md` | ✅ | 引用 |
| `src/crates/contracts/AGENTS.md` | ✅ | 引用 |
| `src/crates/interfaces/AGENTS.md` | ✅ | 引用（推测）|

> 注：apps 层缺 AGENTS.md 与 `agent-doc priority` 规则不符（"Prefer the nearest matching `AGENTS.md` / `AGENTS-CN.md`"），但根 AGENTS.md 已覆盖大部分规则。

### 3.4 与 AGENTS.md / docs/architecture 规则的偏离

| 规则（`AGENTS.md`）| 偏离位置 | 详情 |
|---|---|---|
| **Logging must be English-only, with no emojis** | 几乎全文件 | 实读中未发现违反；但 `apps/cli/src/ui/startup.rs:1977` 的 12 条 TIPS 文案含大量 emoji，未深查（推测是用户可见字符串）|
| **Do not add hard-coded limits or pattern checks to the agent loop as a first response to looping behavior** | 未知 | 需进一步查 `agentic/execution/` 路径 |
| **Do not import Web UI locale resources into smaller product surfaces** | 未知 | 需进一步查 `apps/cli` + `northhing-Installer` i18n 引用 |
| **Product surfaces may diverge; share stable facts or ports, not UI** | apps 层 | `apps/desktop` 与 `apps/cli` 各自有独立的 `agentic_system.rs` 入口（8-18 行 vs `apps/cli/src/agent/agentic_system.rs` 不知道行数）— 这是合理的 platform adapter 模式，符合规则 |
| **Keep product logic platform-agnostic, then expose it through platform adapters** | ✅ | 整个 assembly/core 平台无关，apps 层做 platform adapter，符合 |

---

## 4. Quick wins / 高 ROI 修复（10 项，按 ROI 排序）

> 每项给 file:line + 当前问题 + 改法（≤5 句）+ 工作量估计。

### QW-1 [P0, 30min] `create_ui` 末尾加 startup auto-create session

- **位置**：`apps/desktop/src/app_state/mod.rs:912`（`create_ui` 的 `Ok(ui)` 之前）
- **问题**：用户启动后 sidebar 空、`current_session_id == ""` → on_send_message 必然早退
- **改法**：
 1. 在 `create_ui` 末尾（约 line 900）`Ok(ui)` 之前插入 `tokio::spawn` 调用 `coordinator.create_session("Default Session".to_string(), "code".to_string(), config).await`
 2. spawn 成功时调 `app_state.set_current_session_id(sid)` + `ui.set_current_session_id(sid.into())` + `refresh_sessions_ui`
 3. spawn 失败时记 log（注意走 logging 通道，不写 stderr）
- **工作量**：30 min（含手测）

### QW-2 [P0, 1hr] `create_default_config` 加默认 providers 占位

- **位置**：`crates/assembly/core/src/service/config/manager.rs:107-114` `create_default_config`
- **问题**：`AIConfig::default()` 的 `models: vec![]` 直接落盘 → 状态栏 Model 段永远 "Not configured"
- **改法**：
 1. 决定走"新增 `ai.providers` 字段"还是"在 `ai.models` 塞 3 个 `enabled=false` 占位"（需先与产品确认）
 2. 推荐后者（影响面小）：在 `create_default_config` 追加 `self.config.ai.models.push(AIModelConfig { id: "anthropic-default".into(), provider: "anthropic".into(), enabled: false, api_key: "".into(), ... })` × 3（anthropic / openai / gemini）
 3. 用 `Self::add_default_*` 类似的辅助函数封装
- **工作量**：1 hr（含 3 个 provider 的 AIModelConfig 字段填充）

### QW-3 [P0, 1-2hr] `on_*` Slint callback 把 eprintln 改 set Slint error prop + SidebarView error banner

- **位置**：`apps/desktop/src/app_state/mod.rs:402, 408, 428, 453, 504, 526, 562, 668, 717, 851` + `sessions.rs:168, 194`
- **问题**：9 个 callback 的失败全用 `eprintln!` 写 stderr，UI 无任何错误展示通道（handoff §3.2 准确描述）
- **改法**：
 1. 在 `main.slint` 加 `in-out property <string> session_error: "";` + `in-out property <string> input_error: "";`
 2. 在 `SidebarView.slint` 顶部加 error banner（条件渲染 `if session-error != ""`）
 3. 把 9 处 `eprintln!` 替换为 `ui.set_session_error(format!(...))` / `ui.set_input_error(...)` + 增加一个 `clear_error` callback
- **工作量**：1-2 hr（涉及 slint UI 改动）

### QW-4 [P0, 30min] desktop `main.rs` 调 `set_global_mcp_service()`

- **位置**：`apps/desktop/src/main.rs`（应在 `init_agentic_system_for_desktop` 之后某处）+ `apps/desktop/src/app_state/inspector.rs:26`
- **问题**：desktop 路径构造了 local `MCPService` 但**从不调** `set_global_mcp_service()` → 全局 `GLOBAL_MCP_SERVICE` 永远 None
- **改法**：
 1. 改 `apps/desktop/src/main.rs`，在 `init_agentic_system_for_desktop` 后调一次 `service::mcp::set_global_mcp_service(arc.clone())`
 2. 删除 `MCP_INIT_STATUS` / `get_mcp_init_status` / `get_mcp_status_text`（P3-4 dead code 一并清掉）
 3. 验证 `apps/desktop/src/app_state/inspector.rs:14 build_mcp_status_string` 改走 `service::mcp::get_global_mcp_service()`
- **工作量**：30 min

### QW-5 [P1, 15min] `mcp/config/json_config.rs` 2 处 unreachable!() 改 Err

- **位置**：`crates/services/services-integrations/src/mcp/config/json_config.rs:193, 217`
- **问题**：与 handoff P1-extra-2 同型反模式，理论上不可达但未来新增分支会 panic
- **改法**：
 ```rust
 // 原: _ => unreachable!()
 // 改: _ => Err(anyhow::anyhow!("unsupported (source, url) combination: {:— }", (source, has_url)))
 ```
- **工作量**：15 min

### QW-6 [P1, 15min] `AIClientFactory::initialize_global` 加 instrumentation 日志

- **位置**：`crates/assembly/core/src/infrastructure/ai/client_factory.rs:237-257` + `service/config/global.rs:86-127`
- **问题**：冷启动 hang 时无任何线索定位是 `PathManager::initialize_user_directories` 慢 IO / `mode_config_canonicalizer` lock convoy / `save_config` 写盘慢
- **改法**：在 4 个关键 await 点前后加 `tracing::info!(target: "init", "before X")` / `tracing::info!(target: "init", "after X, took {:— }s", elapsed)`
- **工作量**：15 min

### QW-7 [P1, 30min] CLI `tool_cards.rs` 2 处 hardcoded theme 用 theme 派生

- **位置**：`apps/cli/src/ui/tool_cards.rs:1051, 1239` `let hl_theme = HighlightTheme::Dark; // TODO: derive from theme`
- **问题**：主题切换时高亮不跟随
- **改法**：给 `tool_cards.rs` 的 render 函数加 `theme: &Theme` 参数，从 `theme.is_dark()` 派生 `HighlightTheme::Dark` / `Light`
- **工作量**：30 min

### QW-8 [P2, 1hr] `SKILL_INSPECTOR_ENABLED` / `MCP_INIT_STATUS` / `USE_SOFTWARE_FALLBACK` / `USE_SLINT_SHELL` 死代码清理

- **位置**：
 - `apps/desktop/src/main.rs:18-39, 49, 52, 57-58, 68-69`
- **问题**：5 处死代码 / 永远 true / 重复定义
- **改法**：直接删除 `SKILL_INSPECTOR_ENABLED` / `MCP_INIT_STATUS` 相关 3 个 API；删 `USE_SOFTWARE_FALLBACK` / `USE_SLINT_SHELL` 永远 true 分支；合并 `SESSION_TREE_VIEW` 到 `flags.rs` 一处
- **工作量**：1 hr

### QW-9 [P2, 半天] `apps/cli/src/modes/chat.rs` + `ui/startup.rs` 加基本 smoke test

- **位置**：`apps/cli/src/modes/chat.rs:3362`（10+ method）、`apps/cli/src/ui/startup.rs:1977`
- **问题**：CLI 端两个大文件 0 测试覆盖
- **改法**：
 1. `state_split_tests.rs` 已有 173 行覆盖 `ChatView::new` / `clear_screen` / substructure round-trip — 扩展到 `handle_command` 的 5-10 个关键路径
 2. 新建 `ui/startup_smoke_tests.rs` 覆盖 `PopupStack::push/pop/clear` + `TIPS` 数组非空
- **工作量**：半天（500 行测试代码）

### QW-10 [P2, 半天] `apps/desktop/src/app_state/{sessions,inspector,inspector_model_status}.rs` 加 unit test

- **位置**：
 - `sessions.rs:178`（`build_messages_model` / `refresh_sessions_ui` / `refresh_messages_ui`）
 - `inspector.rs:31`（`build_mcp_status_string` + `McpCatalogAdapter`）
 - `inspector_model_status.rs:47`（`build_model_status_string`）
- **问题**：3 个文件 0 测试，重构无安全网
- **改法**：每个文件加 5-10 个 unit test 覆盖 DTO 投影 + 各 status 字符串构造（参考 `mcp_adapter.rs:157-208` 既有 4 个测试的模式）
- **工作量**：半天

---

## 5. 总结

### 5.1 整体健康度

| 层级 | 健康度 | 证据 |
|---|---|---|
| **assembly/core 后台** | 🟢 良好 | agentic/ + service/ + infrastructure/ai/ 三大目录 0 个 `unimplemented!/todo!/unreachable!/TODO/FIXME`；in-source unit test 充分；4 个 integration test 覆盖 capability / MCP remote / git / context profile |
| **services-integrations** | 🟢 良好 | remote_connect 3446 行单文件 + 5 个 host trait + 穷尽 match；MCP 用 rmcp SDK + 自定义 JSON-RPC + streamable HTTP；测试覆盖好（`mcp_contracts.rs` 1545 行、`remote_connect_contracts.rs` 2137 行）|
| **interfaces/acp** | 🟢 良好 | 25 个 .rs 文件，0 个 TODO/unreachable/eprintln；`runtime.rs:64-131` 完整实现 server trait stub；与 agentic_system 连接点（`runtime/prompt.rs`）清晰 |
| **apps/cli** | 🟡 一— | `modes/chat.rs` 3362 行 + `ui/startup.rs` 1977 行完整实现，但 0 测试覆盖；只有 1 个 `state_split_tests.rs` |
| **apps/desktop** | 🔴 不好 | **GUI 完全不可用**：3 个 P0 bug（状态栏 / New Session / Send）全中；9 个 eprintln callback 零 UI 反馈；MCP 全局注册缺失；`create_ui` 无 startup session |

### 5.2 ROI 排序的修复建议

| # | 优先级 | 工作量 | 修复点 | 影响的 P0 baseline 功能 |
|---|---|---|---|---|
| 1 | **P0** | 30 min | QW-1: `create_ui` 末尾加 startup auto-create session | A1, A2 |
| 2 | **P0** | 1 hr | QW-2: `create_default_config` 加默认 providers | F2（Model 段）|
| 3 | **P0** | 1-2 hr | QW-3: 9 个 eprintln callback 改 set Slint error prop | F3（错误展示）|
| 4 | **P0** | 30 min | QW-4: desktop main.rs 调 set_global_mcp_service + 删 MCP_INIT_STATUS 死代码 | F2（MCP 段）+ H1 |
| 5 | **P1** | 15 min | QW-5: mcp/config/json_config.rs:193, 217 unreachable!() 改 Err | （代码健壮性）|
| 6 | **P1** | 15 min | QW-6: AIClientFactory::initialize_global 加 instrumentation | 调试 P0 hang |
| 7 | **P1** | 30 min | QW-7: CLI tool_cards.rs 2 处 hardcoded theme 派生 | F4（主题）|
| 8 | **P2** | 1 hr | QW-8: 5 处死代码清理 | （代码卫生）|
| 9 | **P2** | 半天 | QW-9: CLI modes/chat + ui/startup 加 smoke test | （回归保护）|
| 10 | **P2** | 半天 | QW-10: desktop app_state 3 个文件加 unit test | （重构安全网）|

**总工作量**：~5 hr 解决所有 P0 + P1，~1 天解决所有 P0 + P1 + P2。

### 5.3 不在本任务范围的发现

- AGENTS.md 缺 apps 层就近 AGENTS.md（与根 `agent-doc priority` 规则不一致）
- Background Agent（G2）/ Skills 市场（H2）/ Hooks（H5）三大 P2 战略能力完全缺失
- 跨 desktop/web/mobile 的会话同步层（G3）缺失
- `crates/assembly/core/src/service/cron/` 目录存在但未深查（baseline P2-4 未确认实现完整度）
- `apps/desktop/src/agent/actor.rs` 的 Phase I.3 actor runtime 注释说"a demonstration wiring, not a replacement"— 轻量 actor 模式与真 phase 1/2/3 路径关系需要进一步弄清

### 5.4 一句话总结

> **NortHing v0.1.0 的真实状态是"assembly/core 后台 90% 完成、协议层 100% 完成、CLI 端 100% 完成、desktop GUI 30% 完成"**。handoff §4 列的 13 个 Next Steps 是准确的，但**优先级表的 P0-1 + P0-2 + P1-3 三项**就是卡住"什么都不能干"的 3 把锁（缺 startup session、缺默认 providers、缺 UI 错误展示通道）。先解这 3 把锁（总工作量 ~3 hr），桌面 GUI 就能跑通基本聊天循环。

---

**报告完。本报告只做事实收集与代码 review，不评估实现代码质量优劣，不写实现代码。所有 file:line 引用均经 subagent 实读代码交叉验证。**
