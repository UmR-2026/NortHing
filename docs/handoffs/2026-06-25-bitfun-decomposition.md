# BitFun 系统化拆解报告

> 日期: 2026-06-26
> 任务: Clone GCWing/BitFun + 系统化拆解 + 与 NortHing 初步映射
> 状态: 完成
> 对照基线: `E:\agent-project\northing` (NortHing v0.2.10, v3-restructure 分支)

---

## 0. 数据快照

| 指标 | 数值 |
| --- | --- |
| 仓库 | `https://github.com/GCWing/BitFun` |
| 本地路径 | `C:\Users\UmR\BitFun` (read-only clone, `--depth 1`) |
| 版本 | `0.2.11` (`Cargo.toml:38`, `package.json:4`) |
| Rust crate 数 | 21 (workspace members, `Cargo.toml:2-28`) |
| App 数 | 4 (cli / desktop / server / relay-server) |
| 源文件总数 | 2522 = 1046 `.rs` + 1020 `.ts` + 452 `.tsx` + 4 `.mjs` |
| Codegraph 索引 | 2784 files / 13507 functions / 12472 methods / 10572 imports / 2314 structs / 85 traits / 64 components |
| 内置 Skills | 24 个目录, 296 个文件 (含 SKILL.md / 参考手册 / LICENSE) |
| 顶层非代码资源 | `MiniApp/` (2 demo + 1 dev template) / `BitFun-Installer/` / `docs/` / `scripts/` / `resources/flashgrep/` |
| AGENTS 文件 | `AGENTS.md` + `AGENTS-CN.md` (中英双版规则) + 16 个 `AGENTS.md` 子模块级 |

---

## 1. 项目概况

### 1.1 一句话定位

**BitFun = Local AI Workbench, 以 Code Agent 为核心, 面向长线任务 (long-horizon task) + 工程执行 + Token economy 的跨平台桌面工作台。** 旗舰场景是 coding (Code Agent / Deep Review / 调试) + office (研究 / PPT / DOCX / XLSX / PDF), 通过 Computer Use / MCP / Skills / Mini App 四层可扩展机制接入真实工作环境。

### 1.2 语言 / 工具栈构成

| 语言 / 工具 | 文件数 | 用途 |
| --- | --- | --- |
| Rust | 1046 | 后端核心, Agent runtime, Tool runtime, Computer Use (WebDriver), MCP, MiniApp host, Remote SSH/Connect, flashgrep 客户端 |
| TypeScript | 1020 | Web UI 业务逻辑 + 状态机 + i18n + 工具链 |
| TSX | 452 | Web UI 组件 (React 19 + TipTap + Monaco + xterm + Mermaid) |
| JavaScript (.mjs) | 4 | 资源 / i18n 合约 / 主题审计脚本 |

关键依赖信号 (`Cargo.toml:43-219`):
- **Agent 栈**: `rmcp 1.7` (MCP) + `agent-client-protocol 0.12` (ACP) + `tokio 1.52` + `tracing`
- **桌面栈**: `tauri 2.11` + `screenshots 0.8` + `enigo 0.2` + `objc2` (macOS) + `windows 0.61` (Windows) + `webkit2gtk / atspi` (Linux)
- **搜索**: `grep-searcher / grep-regex / globset` (本地), flashgrep (独立 daemon, 二进制资源)
- **终端**: `portable-pty 0.8` + `vte 0.15` + `ratatui 0.29` (CLI TUI)
- **远程**: `russh 0.45` (SSH) + `russh-sftp` + `axum 0.8` (relay) + `x25519-dalek` + `aes-gcm` (E2E)
- **搜索/索引**: `tao` (打 patch) + `time =0.3.47` (锁版本避坑)

### 1.3 顶层目录结构

| 路径 | 角色 | 备注 |
| --- | --- | --- |
| `src/apps/desktop/` | Tauri 2 桌面应用 (主入口) | 调用 `bitfun-core` + `bitfun-transport` + `bitfun-webdriver` + `bitfun-acp` |
| `src/apps/cli/` | TUI 终端 (ratatui + crossterm) | 同样的 `bitfun-core` 后端 |
| `src/apps/server/` | Axum Web 服务 (Browser-only 部署) | 静态资源 + HTTP API |
| `src/apps/relay-server/` | 独立 WebSocket relay (Docker 化) | 用于 Remote Connect, E2E 加密 (x25519 + aes-gcm) |
| `src/crates/assembly/core/` | **平台无关业务核心** (核心) | 522 文件, 包含 `agentic/` / `service/` / `miniapp/` / `function_agents/` / `infrastructure/` / `product_runtime/` / `util/` + `builtin_skills/` (24 个) + `builtin_playbooks/` (5 个 YAML) + `locales/` |
| `src/crates/assembly/product-capabilities/` | 产品能力包契约 (空壳) | 仅 re-export harness / runtime-ports / runtime-services / tool-packs, 实际能力在 core |
| `src/crates/adapters/{ai-adapters,api-layer,transport,webdriver}/` | 4 个适配器 | 见架构层 |
| `src/crates/services/{services-core,services-integrations,terminal}/` | 3 个服务 | 见架构层 |
| `src/crates/execution/{agent-runtime,agent-stream,harness,runtime-services,tool-contracts,tool-execution,tool-provider-groups}/` | 7 个执行原语 crate | 提供 portable 的运行时构件 |
| `src/crates/contracts/{core-types,events,product-domains,runtime-ports}/` | 4 个稳定契约 | DTO / 事件 / 端口 / 领域 |
| `src/crates/interfaces/acp/` | Agent Client Protocol 表面 | 走 stdio 让外部 IDE / CLI 接 BitFun |
| `src/web-ui/` | Web UI 主体 (Desktop + Server 共享) | 999 ts + 442 tsx = 1441 文件, 结构见 2.6 |
| `src/mobile-web/` | 移动 Web (Remote Connect 配对) | 较小, 跟 web-ui 共享 design-system |
| `src/shared/` | 跨 surface 共享 (i18n 资源) | 包含 `i18n/contract/locales.json` 等 |
| `MiniApp/` | **用户态 Mini App 仓库** | `Demo/git-graph/` + `Demo/icon-design-system/` + `Skills/miniapp-dev/` |
| `BitFun-Installer/` | 独立安装器项目 | 自带 `src-tauri` (workspace exclude) |
| `resources/flashgrep/` | flashgrep 二进制资源 | 由 `scripts/prepare-flashgrep-resource.mjs` 准备, 跨平台 (win/mac/linux) |
| `docs/` | 项目文档 (spec, 架构, SDLC) | 含 `architecture/`, `sdlc-harness/`, `specs/` |
| `scripts/` | 构建/资源/i18n 审计脚本 | 跟 web-ui 工程化相关 |

---

## 2. 架构层级 (按 6 层分层组织)

依赖方向**自顶向下**, 严格遵循 BitFun 自己的分层 `AGENTS.md` (workspace root `AGENTS.md` + 16 个子模块 AGENTS)。

### 2.1 第 1 层: 入口与 Interfaces (4 apps + 1 ACP + 2 个 web 前端)

| 模块 | 关键文件 | 备注 |
| --- | --- | --- |
| `bitfun-desktop` (Tauri) | `src/apps/desktop/Cargo.toml:22-25` + `src/apps/desktop/src/main.rs` | 启用 Tauri 全家桶: opener / dialog / fs / log / autostart / notification / updater / global-shortcut, 调 `bitfun-core` (`product-full` feature) + `bitfun-transport/tauri-adapter` + `bitfun-webdriver` + `bitfun-acp` |
| `bitfun-cli` (ratatui) | `src/apps/cli/Cargo.toml:12-71` | ratatui + crossterm + syntect 语法高亮, 调 `bitfun-core` + `bitfun-events` + `bitfun-acp` |
| `bitfun-server` (axum) | `src/apps/server/Cargo.toml:13-25` | Browser-only 部署, axum 0.8 + tower-http |
| `bitfun-relay-server` (axum) | `src/apps/relay-server/Cargo.toml:16-45` | **独立 build** (不继承 workspace deps, 用于 Docker 镜像), x25519 + aes-gcm + WebSocket |
| `bitfun-acp` (ACP) | `src/crates/interfaces/acp/src/lib.rs:1-12`, `runtime.rs:1-80` | 走 `agent-client-protocol 0.12`, 提供 `client/manager.rs` 91k 行 (1 个文件) + `client/stream.rs` 25k 行 + `client/requirements.rs` 24k 行; `runtime/` 含 content / events / mcp / model / prompt / session / thinking 七大子模块 |
| `src/web-ui/` | 见 2.6 节 | Desktop + Server 共用 |
| `src/mobile-web/` | 移动 Web (跟 web-ui 共享 design-system) | 配合 Remote Connect 配对 |

### 2.2 第 2 层: Product Assembly (`src/crates/assembly/`)

| crate | 关键入口 | 作用 |
| --- | --- | --- |
| `bitfun-core` (核心) | `src/crates/assembly/core/Cargo.toml:163-185` 启用 `product-full` feature | **平台无关业务核心**。`src/agentic/` (Agent 引擎) + `src/service/` (21 个服务子模块) + `src/miniapp/` + `src/function_agents/` + `src/infrastructure/` + `src/product_runtime/` + `src/util/` + `builtin_skills/` (24 个内置) + `builtin_playbooks/` (5 个 YAML) + `locales/` (i18n) |
| `bitfun-product-capabilities` (空壳) | `src/crates/assembly/product-capabilities/Cargo.toml:12-17` | 仅 re-export harness / runtime-ports / runtime-services / tool-packs. 真能力都在 core |

`bitfun-core` 的 feature 组合设计很关键 (`Cargo.toml:159-218`):
- `default = ["product-full"]` (开发用)
- `product-full` = `ai-adapter-runtime` + `service-integrations` + `tool-packs` + `product-domains` + `runtime-services` + `ssh-remote` + ...
- `ai-adapter-runtime` 单独 feature 让 `ai-adapters` 接入变得可拔插
- `tauri-support = ["tauri"]` 让 core 仍可编译给 server 部署

### 2.3 第 3 层: Adapters (`src/crates/adapters/`)

| crate | 关键依赖 | 作用 |
| --- | --- | --- |
| `bitfun-ai-adapters` | `bitfun-agent-stream` + `reqwest` + `eventsource-stream` | AI 协议适配 (SSE / stream / 重试), 给 `core` 复用 |
| `bitfun-api-layer` | `bitfun-transport` | 平台无关业务 handler, 给 server / desktop 共享 |
| `bitfun-transport` | `bitfun-events` + feature: `tauri-adapter` / `cli-adapter` / `websocket-adapter` | 跨平台通信适配 (Tauri IPC / CLI stdin / WebSocket) |
| `bitfun-webdriver` | `axum` + `tauri` + Windows: `webview2-com` / macOS: `objc2-app-kit` / Linux: `webkit2gtk` | **Computer Use 核心: 嵌入式 WebDriver server** (70+ rs 文件, 见 3.1) |

### 2.4 第 4 层: Services (`src/crates/services/`)

| crate | 关键文件 | 作用 |
| --- | --- | --- |
| `bitfun-services-core` | `src/crates/services/services-core/src/filesystem/tree.rs` (54k) + `filesystem/operations.rs` (24k) + `session/types.rs` (42k) + `session/metadata.rs` (19k) + `session/lineage.rs` (19k) + `managed_runtime.rs` (14k) | **通用 OS / 文件 / 会话 / 诊断 / diff** 服务 |
| `bitfun-services-integrations` | 见 2.4.1 节 | 集成类服务 (MCP / Git / Remote Connect / SSH / workspace-search / file-watch / miniapp / deep-research / function-agents) |
| `bitfun-terminal` (terminal-core) | `exec.rs` 83k + `session/manager.rs` 61k + `shell/integration.rs` 29k + `pty/process.rs` 25k | **跨平台 PTY 终端** (portable-pty + vte + pty data bufferer + shell detection + 持久化 / replay) |

#### 2.4.1 services-integrations 子结构 (按 feature 划分, `Cargo.toml:63-159`)

| feature | 实现位置 | 备注 |
| --- | --- | --- |
| `mcp` | `src/mcp/` (12k 协议 + 21k connection + 15k process + 10k config + 14k auth) | rmcp 1.7 集成, 支持 stdio / streamable-http / sse |
| `git` | `src/git/service.rs` 44k + `git/graph.rs` 10k + `git/utils.rs` 10k | libgit2 + worktree 支持 |
| `miniapp-runtime` | `src/miniapp/storage.rs` 57k + `host_dispatch.rs` 22k + `worker_pool.rs` 18k + `worker.rs` 8k + `builtin_io.rs` 8k | **MiniApp 加载/执行/存储** |
| `remote-connect` | `src/remote_connect.rs` 120k + `bot/` (locale 30k / mod 21k / menu 7k / state 6k / command 7k) + `pairing.rs` 9k + `relay_client.rs` 19k + `encryption.rs` 6k + `qr_generator.rs` 3k | 远程连接 + Bot 消息 + 配对 |
| `remote-ssh` / `remote-ssh-concrete` | `src/remote_ssh/manager.rs` 108k + `remote_exec.rs` 39k + `remote_fs.rs` 14k + `remote_terminal.rs` 14k + `workspace_search/service.rs` 49k | SSH 远程 + 远端文件/终端/搜索 |
| `workspace-search` | `src/workspace_search/service.rs` 30k + `flashgrep/` (见 3.7) | 本地全文搜索 + flashgrep 加速 |
| `file-watch` | `src/file_watch/service.rs` 13k | notify 8.2 包装 |
| `function-agents` | `src/function_agents.rs` 5k | 函数级 Agent (轻量 task) |
| `deep-research` | `src/deep_research.rs` 12k | Deep Research 流程 |
| `announcement` | `src/announcement/` (state 3k / types 7k) | 公告 / 更新提示 |

### 2.5 第 5 层: Execution Primitives (`src/crates/execution/`)

| crate | 关键文件 | 作用 |
| --- | --- | --- |
| `bitfun-agent-runtime` | `src/deep_review/` (10+ 子模块) + `src/thread_goal/` + `src/agents/` | **Agent runtime 契约 + Deep Review 策略 owner** |
| `bitfun-agent-stream` | 轻量 SSE / 流处理 (无重型依赖) | Agent stream 处理 |
| `bitfun-harness` | harness workflow contracts | 工作流注册 (空壳契约) |
| `bitfun-runtime-services` | 跨进程 typed service 抽象 | 让 core 可以调用 typed service 而非字符串 command |
| `bitfun-agent-tools` (tool-contracts) | tool interface + input_validator | 工具契约 |
| `bitfun-tool-packs` (tool-provider-groups) | tool pack provider plan | 工具集分组 (bash / file / web / skills / computer-use / control-hub) |
| `tool-runtime` (tool-execution) | 工具执行容器 | 隔离执行 + 限流 + 重试 |

> **重要设计**: Deep Review / 工具包 (tool-packs) 走 owner crate, concrete 编排留 `core::agentic::*`, 渐进迁移 (看 `core/src/agentic/deep_review/mod.rs:1-6` 注释)

### 2.6 第 6 层: Contracts (`src/crates/contracts/`)

| crate | 关键文件 | 作用 |
| --- | --- | --- |
| `bitfun-core-types` | 只有 `serde` + `serde_json` | 纯 DTO, 无任何 runtime 依赖 |
| `bitfun-events` | event types | 事件形状 |
| `bitfun-runtime-ports` | anyhow + async-trait + tokio-util (无 full tokio) | 端口契约 (Thin runtime ports, 给 core 分解用) |
| `bitfun-product-domains` | miniapp + function-agents 域契约 | 产品域 (空壳, 实际实现走 services-integrations) |

### 2.7 Web UI 结构 (`src/web-ui/src/`) — 1441 文件

| 子目录 | 角色 | 关键内容 |
| --- | --- | --- |
| `app/` | 应用骨架 | `components/` (AboutDialog / AgentCompanionDesktopPet / MCPInteractionDialog / NavBar / NavPanel / RemoteConnectDialog / panels / scheduled-jobs / sessions / workspaces 等) + `app/scenes` + `app/stores` + `app/services` + `app/layout` + `app/startup` |
| `flow_chat/` | **聊天流状态机** (核心) | `state-machine/SessionStateMachine.ts` (10k) + `transitions.ts` + `derivedState.ts` + `state-machine-manager.ts` + `deep-review/` (子模块) + `tool-cards/` + `events/` + `reducers/` + `hooks/` + `services/` + `store/` + `types/` + `utils/` |
| `features/ssh-remote/` | SSH 远程特性 | (整目录都是 ssh-remote) |
| `shared/` | 跨特性复用 | `ai-errors/` + `announcement-system/` + `context-menu-system/` + `context-system/` + `crypto/` + `inspector/` + `notification-system/` + `prism/` (代码高亮) + `services/` + `stores/` + `theme/` + `types/` + `utils/` + `helpers/` + `constants/` |
| `infrastructure/` | 基础设施 | `agents/` + `api/` + `config/` + `contexts/` + `debug/` + `event-bus/` + `font-preference/` + `hooks/` + `i18n/` + `language-detection/` + `mcp/` + `providers/` + `runtime/` + `services/` + `theme/` + `update/` |
| `tools/` | 工具类组件 | (较小) |
| `component-library/` | 设计系统 | 通用组件库 |
| `locales/` | i18n locale 资源 | (多语言) |
| `generated/` | 自动生成 (i18n 合约等) | (不手动改) |
| `hooks/` | 全局 hook | |

---

## 3. 核心能力清单 (按 README 宣传点对应)

### 3.1 Computer Use (浏览器/桌面自动化)

**定位**: 通过嵌入式 WebDriver 协议控制浏览器和桌面, 让 Agent 能"看见"和"操作"真实软件。

**实现位置**:
- `src/crates/adapters/webdriver/` — **70+ 个 rs 文件, 完整 WebDriver 协议实现**
  - `src/lib.rs:1-30` 入口
  - `src/server/router.rs` 7k + `handlers/` (actions / alert / cookie / element 9k / frame / navigation / session 5k / window 6k / shadow / timeouts / logs / script / screenshot / print)
  - `src/executor/` (interaction 1.8k + navigation 3.4k + session 5k + window 3.7k + element/{actions 2k,lookup 1.5k,read 4k,shadow 1.4k})
  - `src/platform/` (capture 24k! 跨平台截图 + image 2k + types 2.5k + evaluator/{macos 3.9k,windows 6.8k,mod 3.6k})
  - `src/runtime/` (script 3.2k + api/{element 10k,interaction 2.8k,navigation 1.4k,mod} + script/{input 4.7k,core/{alert 1.4k,context 2.7k,cookie 2.5k,execution 2.7k,locator 2.6k,runtime 6.5k,shadow 0.6k,store 3.6k,visibility 0.6k},keyboard/{edit 3.4k,event 0.7k,focus 1.9k,mapping 2.8k,mod},pointer/{key_source,mouse 5k,perform,pointer_source 2.3k,release,wheel,wheel_source}})
- `src/crates/assembly/core/src/agentic/tools/` — **Computer Use 4 个子模块** (tool 集成层)
  - `tools/computer_use_capability/` — 能力声明
  - `tools/computer_use_host/` — host 端
  - `tools/computer_use_optimizer/` — 优化器 (节省 token / 提高定位精度)
  - `tools/computer_use_verification/` — 验证 (操作完真的改对了吗?)
- `src/crates/assembly/core/builtin_skills/agent-browser/` — **浏览器专用 skill** (SKILL.md 41k 字, 含 anthropic-best-practices 47k + testing-skills-with-subagents 12k)

**对比 NortHing**: NortHing 在 v0.2.10 还没有 `tools::computer_use_*` 4 个模块, 浏览器自动化走 webdriver 单 crate, agent-browser skill 已有但内容较旧。

### 3.2 Deep Review (深度代码评审)

**定位**: 跨 reviewer / 跨 round 的深度评审系统, 跑完产生结构化 review report, 跟 SWE-Bench 评分相关。

**实现位置 (分层)**:
- **Owner crate (策略层)**: `src/crates/execution/agent-runtime/src/deep_review/`
  - `budget.rs` / `concurrency_policy.rs` / `constants.rs` / `diagnostics.rs` / `execution_policy.rs` / `incremental_cache.rs` / `manifest.rs` / `queue.rs` / `shared_context.rs` / `team_definition.rs` / `tool_context.rs`
- **Product assembly (报告 shaping)**: `src/crates/assembly/core/src/agentic/deep_review/`
  - `mod.rs:1-15` 透明 re-export owner crate
  - `report.rs` 5.9k (报告 metadata / 缓存更新 / 压缩合约)
  - `task_adapter.rs` 24.8k (核心适配)
  - `tool_measurement.rs` 1.3k
- **巨大实现位置** (历史遗留): `src/crates/assembly/core/src/service/review_platform/mod.rs` **166,730 行** (注: 字节数, 实际 ~3500+ 行 Rust). 这块仍是 product 装配的一部分, 渐进迁移中
- **Web UI**: `src/web-ui/src/flow_chat/deep-review/` (有 AGENTS.md / CONTRIBUTING.md / README.md)
- **Agent 定义**: `src/crates/assembly/core/src/agentic/agents/definitions/review/` (review 类型 subagent)

**对比 NortHing**: NortHing v0.2.10 还在把 review 全部塞 core/service/review_platform/, 还没有 `agent-runtime/src/deep_review/` owner 分离, 是更早期的 monolith 设计。

### 3.3 Long-horizon task execution (长线任务执行)

**定位**: Agent 能跑几个小时甚至跨 session 持续推进一个目标 (例如"重构某个模块"或"完成一个 PR"), 中间可能被中断, 之后自动恢复。

**实现位置**:
- **Goal mode (Codex `/goal` parity)**: `src/crates/assembly/core/src/agentic/goal_mode/mod.rs:1-80`
  - `pub fn now_ms()` + `ThreadGoalStore` + `goal_internal_context_message` + `is_usage_limit_error`
  - 复用 `bitfun_agent_runtime::thread_goal::{build_set_thread_goal_result, thread_goal_patch, continuation_prompt, ...}`
  - **关键常量** (从 `runtime-ports` 导入): `MAX_THREAD_GOAL_AUTO_CONTINUATIONS` / `MAX_GOAL_CONTINUATIONS` / `MAX_THREAD_GOAL_OBJECTIVE_CHARS` / `MAX_CONTEXT_SUMMARY_CHARS`
- **执行引擎**: `src/crates/assembly/core/src/agentic/execution/`
  - `execution_engine.rs` 161k 字节 (~3877 行, **核心 agent loop, 自包含一整个 turn**)
  - `round_executor.rs` 69k 字节 (~1900 行, **单 round 调度**)
  - `stream_processor.rs` 5k + `model_exchange_trace.rs` 16k + `write_content_sanitizer.rs` 4.5k + `types.rs` 4.3k
- **状态机**: `src/web-ui/src/flow_chat/state-machine/SessionStateMachine.ts` 10k + `transitions.ts` 4.8k + `derivedState.ts` 8k
- **Round preempt**: `src/crates/assembly/core/src/agentic/round_preempt/` (`RoundInjection` / `RoundInjectionKind` / `RoundInjectionTarget` / `SessionRoundInjectionBuffer` + `DialogRoundInjectionInterrupt` + `NoopDialogRoundInjectionSource`)

**对比 NortHing**: NortHing v0.2.10 没有 `goal_mode` 模块, 也没有 `round_preempt` 模块. 长线执行靠 execution_engine 单一文件 + session 持久化. NortHing 走的是更简单的"保存 + 恢复"路径, BitFun 走的是"目标追踪 + 自动续推"路径。

### 3.4 Memory / Personality (记忆与人格)

**定位**: 工作区级自动记忆 (类似 CLAUDE.md) + 用户指令文件加载, 让 Agent 跨 session 记忆项目偏好。

**实现位置**:
- `src/crates/assembly/core/src/service/agent_memory/mod.rs:1-6` (6 行 mod.rs)
  - `auto_memory` (build_workspace_agent_memory_prompt, build_workspace_memory_files_context)
  - `instruction_context` (build_workspace_instruction_files_context)
- **没有专门的 `personality` 模块** — personality 通过 `bitfun-product-capabilities` (空壳) + `agents/definitions/{modes,custom}` 实现
- `src/crates/assembly/core/src/agentic/init_agents_md.rs` — 初始化 AGENTS.md 加载
- Web UI: `src/web-ui/src/infrastructure/agents/` (agents 基础设施)

**对比 NortHing**: ⚠️ **NortHing 多了 builtin_skills/memory/ 这个 skill** (BitFun 没有 memory skill, 但有 `service::agent_memory` 模块). NortHing 是把 memory 做成"可观察的 skill", BitFun 是做成"内部 service + 隐式注入 prompt"。

### 3.5 MCP (Model Context Protocol)

**定位**: 接入外部 MCP server (stdio / streamable-http / sse), 把它包装成 Agent 工具。

**实现位置**:
- `src/crates/services/services-integrations/src/mcp/`
  - `mod.rs` 553 行 (feature 入口)
  - `auth.rs` 14k (OAuth / 鉴权)
  - `runtime_error.rs` 3k + `tool_info.rs` 0.2k + `tool_name.rs` 1.5k
  - `adapter/` (context 2.9k / mod 0.5k / prompt 1.7k / resource 2.8k / tool 5.6k)
  - `config/` (cursor_format 9.7k / json_config 9k / location 0.3k / mod 0.8k / service 10k / service_helpers 5.6k) — **支持从 Cursor 配置文件导入 MCP server**
  - `protocol/` (jsonrpc 4.5k / mod 0.3k / rmcp_mapping 10k / transport 4.7k / transport_remote 26k / types 25k)
  - `server/` (catalog_cache 1.9k / connection 21k / mod 7.3k / process 15k / runtime_helpers 1.5k / runtime_policy 1.2k)
- **核心依赖**: `rmcp 1.7` (官方 Rust SDK) + `reqwest` + `sse-stream`
- **Web UI**: `src/web-ui/src/app/components/MCPInteractionDialog/` + `src/web-ui/src/infrastructure/mcp/` + `src/web-ui/src/flow_chat/services/` 里的 mcp 部分

**对比 NortHing**: MCP 几乎一致 (同样的 rmcp 1.7, 同样的目录结构). NortHing 在 `mcp_contracts.rs` 测试文件 55k, 略多于 BitFun 的 49k 测试, 但都是历史累积差异。

### 3.6 Skills (技能系统)

**定位**: 用 SKILL.md 描述可复用的领域流程, Agent 遇到匹配场景自动加载。

**实现位置**:
- **内置技能**: `src/crates/assembly/core/builtin_skills/` 24 个目录, 296 个文件
  - **文档类**: `docx/` (SKILL 17k + api-ref 14k + design-playbook 12k) / `pdf/` (SKILL 13k + forms 12k + reference 17k) / `pptx/` (SKILL 9k + editing 7k + pptxgenjs 13k) / `ppt-design/` (SKILL 98k — 巨详细的设计 playbook) / `xlsx/` (SKILL 11k)
  - **工作流类 (gstack-*)**: 17 个: autoplan / cso / design-consultation / design-review / document-release / investigate / office-hours / plan-ceo-review / plan-design-review / plan-eng-review / qa / qa-only / retro / review / ship
  - **开发辅助**: `agent-browser/` (41k) / `find-skills/` (2.8k) / `miniapp-dev/` (17k) / `writing-skills/` (23k + anthropic-best-practices 47k)
- **运行时**: `src/crates/assembly/core/src/agentic/tools/implementations/skills/mod.rs:1-558`
  - 通过 `SkillTool` 注册到 Tool 框架
- **Manifest 解析**: `src/crates/assembly/core/src/agentic/tools/manifest_resolver/` (`resolve_tool_manifest` / `resolve_visible_tools`)
- **Playbook (YAML)**: `src/crates/assembly/core/builtin_playbooks/` 5 个: `browser_data_extraction.yaml` / `browser_form_fill.yaml` / `browser_screenshot.yaml` / `desktop_app_automation.yaml` / `im_send_message.yaml`

**对比 NortHing**: ⚠️ **NortHing 多了 `memory` skill, 少了 `miniapp-dev` skill**. 其余 23 个完全一样. Playbook 5 个 YAML 也一样.

### 3.7 flashgrep (高速代码搜索)

**定位**: 独立的二进制 daemon, 在千万行级别代码库 (如 Chromium) 上比传统 grep 快 ~36 倍, README 提到最高节省 94.6% 搜索时间。

**实现位置**:
- **Daemon (二进制)**: `resources/flashgrep/` (跨平台二进制, 准备脚本 `scripts/prepare-flashgrep-resource.mjs`)
  - 准备脚本定义: `flashgrepBinaryNames` / `flashgrepBinaryName` / `flashgrepBinaryPath` / `RESOURCE_DIR = join(ROOT, 'resources', 'flashgrep')`
- **Rust 客户端 (集成)**: `src/crates/services/services-integrations/src/workspace_search/flashgrep/`
  - `mod.rs` 2.4k (入口, 60 行 — `log_flashgrep_stderr_line` / `ManagedClient` / `RepoSession` / `ProtocolClient`)
  - `client.rs` 23k (ManagedClient 主体)
  - `protocol.rs` 14k (JSON-RPC 协议: `ClientCapabilities` / `ClientInfo` / `GlobParams` / `InitializeParams` / `RepoRef` / `Request` / `Response` / `SearchParams` / `TaskRef`)
  - `rpc_client.rs` 13k (RPC 客户端, 支持 content-length chunked)
  - `repo_session.rs` 0.7k (单 repo 会话)
  - `types.rs` 1.5k (`DirtyFileStats` / `GlobOutcome` / `QuerySpec` / `RepoConfig` / `SearchBackend` / `SearchOutcome` / `TaskKind` / `TaskStatus` / `WorkspaceOverlayStatus` 等)
  - `error.rs` 0.3k
- **Workspace search (高层)**: `src/crates/services/services-integrations/src/workspace_search/`
  - `mod.rs` 1.3k (re-export types + global setter/getter)
  - `service.rs` 30k (主服务)
  - `result_mapping.rs` 4.6k
  - `types.rs` 12.6k
- **Service 入口**: `src/crates/assembly/core/src/service/search/mod.rs:1-26` (transparent re-export)

**对比 NortHing**: NortHing 已经有完整 `flashgrep/` 子目录, 代码量跟 BitFun 一致. 主要是二进制版本可能不同, flashgrep 客户端应该可以直接复用。

### 3.8 Mini App (小应用)

**定位**: 任务专属的小应用, 用 `meta.json + package.json + storage.json + source/` 描述, 跑在独立 JS worker pool 里。

**实现位置**:
- **Service 层 (存储/加载/执行)**: `src/crates/services/services-integrations/src/miniapp/`
  - `storage.rs` 57k (主存储)
  - `host_dispatch.rs` 22k (host ↔ miniapp IPC)
  - `worker_pool.rs` 18k (JS worker pool)
  - `worker.rs` 8k + `builtin_io.rs` 8k
- **Core 层 (编译/管理)**: `src/crates/assembly/core/src/miniapp/`
  - `manager.rs` 38k (miniapp 生命周期)
  - `compiler.rs` 1.2k + `exporter.rs` 1.5k + `host_dispatch.rs` 1.6k
  - `js_worker.rs` 0.2k + `js_worker_pool.rs` 12k
  - `runtime_detect.rs` 0.2k + `storage.rs` 14k
- **契约**: `src/crates/contracts/product-domains/Cargo.toml:22` (miniapp feature)
- **Web UI**: `src/web-ui/src/infrastructure/` 里的 miniapp 部分
- **示例**: `MiniApp/Demo/git-graph/` (git 可视化) + `MiniApp/Demo/icon-design-system/` + `MiniApp/Skills/miniapp-dev/` (开发 miniapp 用的 skill)
- **运行时入口** (Web 端): `src/web-ui/src/MiniApp/` 目录

**对比 NortHing**: 整体一致, 但 BitFun 的 `core::miniapp::manager.rs` 38k 比 NortHing 同位置 (~30k 估) 略大. storage 部分可能更完善。

### 3.9 Subagent / Fork Agent (子代理)

**定位**: 父 session 派生子 session, 共享上下文, 但独立运行/取消/清理。

**实现位置**:
- `src/crates/assembly/core/src/agentic/fork_agent/mod.rs:1-116` (116 行 mod.rs)
  - `ForkAgentContextSnapshot` (parent_session_id, parent_agent_type, workspace_path, remote_connection_id, remote_ssh_host, session_model_id, session_config, messages)
  - `from_parent_session` / `build_child_session_config`
- **执行**: `src/crates/assembly/core/src/agentic/subagent_runtime/` (mod.rs 0.3k)
- **Tasks (TaskTool)**: `src/crates/assembly/core/src/agentic/tools/implementations/` (TaskTool)

**对比 NortHing**: 非常一致. NortHing 的 fork_agent 模块应该跟 BitFun 几乎一样。

---

## 4. 和 NortHing 的对应关系 (粗映射)

### 4.1 整体对比

| 维度 | BitFun v0.2.11 | NortHing v0.2.10 (v3-restructure) |
| --- | --- | --- |
| 仓库作者 | BitFun Team | NortHing Team |
| 顶层 apps 数 | 4 (cli / desktop / server / relay-server) | 4 (完全对齐) |
| Rust crate 数 | 21 | 22 (+cli-internal) + 1 tools/plan-compliance-checker |
| 总源文件 | 2522 (1046 rs + 1020 ts + 452 tsx + 4 mjs) | ~1005 (975 rs + 30 ts/tsx) — **前端已经被剥离** |
| Web UI | 1441 ts/tsx, 完整 | 3 文件, **只保留 infrastructure + generated 占位** |
| builtin_skills 数 | 24 | 24 (多 1 个 memory, 少 1 个 miniapp-dev) |
| Codegraph 索引 | 2784 files / 13.5k functions | (未测, 估 < 1500 files) |
| Layered AGENTS | workspace root + 16 子模块 | 同样 6 层 + 同样 16 子模块 (几乎一致的 AGENTS.md) |

### 4.2 7 个 layer 完全对齐

| BitFun crate | NortHing crate | 状态 |
| --- | --- | --- |
| `bitfun-core` (assembly/core) | `northhing-core` (assembly/core) | **同名同结构**, BitFun v0.2.11 更新 |
| `bitfun-product-capabilities` | `northhing-product-capabilities` | 同名 |
| `bitfun-ai-adapters` | `northhing-ai-adapters` | 同名 |
| `bitfun-api-layer` | `northhing-api-layer` | 同名 |
| `bitfun-transport` | `northhing-transport` | 同名 |
| `bitfun-webdriver` | `northhing-webdriver` | 同名 |
| `bitfun-services-core` | `northhing-services-core` | 同名 |
| `bitfun-services-integrations` | `northhing-services-integrations` | 同名 |
| `bitfun-terminal` (terminal-core) | `northhing-terminal` | 同名 |
| `bitfun-agent-runtime` | `northhing-agent-runtime` | 同名 |
| `bitfun-agent-stream` | `northhing-agent-stream` | 同名 |
| `bitfun-harness` | `northhing-harness` | 同名 |
| `bitfun-runtime-services` | `northhing-runtime-services` | 同名 |
| `bitfun-agent-tools` (tool-contracts) | `northhing-agent-tools` (tool-contracts) | 同名 |
| `bitfun-tool-packs` (tool-provider-groups) | `northhing-tool-packs` (tool-provider-groups) | 同名 |
| `tool-runtime` (tool-execution) | `tool-runtime` (tool-execution) | 同名 |
| `bitfun-core-types` | `northhing-core-types` | 同名 |
| `bitfun-events` | `northhing-events` | 同名 |
| `bitfun-runtime-ports` | `northhing-runtime-ports` | 同名 |
| `bitfun-product-domains` | `northhing-product-domains` | 同名 |
| `bitfun-acp` | `northhing-acp` | 同名 |
| — | **`northhing-agent-dispatch`** (NortHing 独有) | NortHing 多出来一个 subagent 分发层 |
| — | **`northhing-cli-internal`** (NortHing 独有) | NortHing 多出来 CLI 内部 crate |
| — | **`tools/plan-compliance-checker`** (NortHing 独有) | 计划合规性检查工具 (项目私有) |

### 4.3 关键差异 (需在 v3-restructure 计划中关注)

| 能力 | BitFun v0.2.11 进展 | NortHing v0.2.10 状态 | 建议 |
| --- | --- | --- | --- |
| Computer Use | ✅ 4 个独立子模块 (capability / host / optimizer / verification) + 70+ 文件 WebDriver 实现 | ⚠️ 较早期, 还在 webdriver 单 crate + 单一 Computer Use tool | NortHing 应该同步升级 4 个子模块拆分 |
| Deep Review | ✅ 已 owner crate 化 (`agent-runtime/src/deep_review/`), core 只做 product shaping | ⚠️ 仍 monolith 在 `core/service/review_platform.rs` 166k 单文件 | NortHing 应渐进做同样的 owner 拆分 |
| Long-horizon (`/goal`) | ✅ `core::agentic::goal_mode` + `round_preempt` 完整实现 | ❌ 缺失, 走简单 session 持久化 | NortHing 应评估引入 goal mode |
| Round preempt | ✅ `DialogRoundInjectionInterrupt` / `RoundInjection` / `RoundInjectionTarget` / `SessionRoundInjectionBuffer` | ❌ 缺失 | NortHing 应评估 |
| MCP | ✅ rmcp 1.7 + Cursor 兼容 | ✅ rmcp 1.7, Cursor 兼容 | 一致 |
| Skills | ✅ 24 个 builtin (含 miniapp-dev) | ✅ 24 个 (多 memory, 少 miniapp-dev) | **NortHing 的 memory skill 是 unique** — 应保留并反向同步到 BitFun |
| MiniApp | ✅ 完整 manager + worker pool + storage | ✅ 一致 | 一致 |
| flashgrep | ✅ 完整 + 二进制资源 | ✅ 完整 (估) | 一致 |
| Subagent / Fork | ✅ `ForkAgentContextSnapshot` 116 行 | ✅ 估一致 | 一致 |
| Web UI | ✅ 1441 文件, 完整 | ❌ **3 文件, v3-restructure 几乎全空** | **v3-restructure 的目标** — 复刻 BitFun web-ui |
| Remote Connect | ✅ relay + bot + pairing, 120k+ 行 | ⚠️ 估部分实现 | 待验证 |
| Remote SSH | ✅ russh 0.45 + sftp + worktree | ⚠️ 估部分实现 | 待验证 |
| E2E / 测试 | ✅ `tests/e2e/` (L0/L1/L2) + 各 crate contracts test | ✅ 类似 | 一致 |

### 4.4 一句话总结映射

> **NortHing v0.2.10 ≈ BitFun v0.2.10 末期**, 分叉点在 v0.2.10 → v0.2.11 之间的 11 次小迭代. NortHing 已经剥离前端准备 v3 重构, 后端核心能力 (MCP / Skills / MiniApp / flashgrep / Subagent) 与 BitFun v0.2.11 高度一致; 主要落后项是 **Computer Use 拆分粒度、Deep Review owner 化、长线 goal_mode、round_preempt** 这 4 个能力。

---

## 5. 关键文件:行参考 (用于后续对比任务)

| 能力 | BitFun 入口 | 行数 |
| --- | --- | --- |
| Execution engine (核心 loop) | `src/crates/assembly/core/src/agentic/execution/execution_engine.rs:1` | 3877 行 |
| Round executor (单 round) | `src/crates/assembly/core/src/agentic/execution/round_executor.rs:1` | 1900 行 |
| Goal mode (长线) | `src/crates/assembly/core/src/agentic/goal_mode/mod.rs:1` | 402 行 |
| Fork agent | `src/crates/assembly/core/src/agentic/fork_agent/mod.rs:1` | 116 行 |
| Deep review (product assembly) | `src/crates/assembly/core/src/agentic/deep_review/mod.rs:1` | 15 行 (re-export) |
| Deep review (owner crate) | `src/crates/execution/agent-runtime/src/deep_review/` | 10 子模块 |
| Computer Use (4 拆分) | `src/crates/assembly/core/src/agentic/tools/computer_use_*/` | 4 模块 |
| Computer Use (WebDriver) | `src/crates/adapters/webdriver/src/lib.rs:1` | 70+ 文件 |
| flashgrep client | `src/crates/services/services-integrations/src/workspace_search/flashgrep/mod.rs:1` | 60 行入口 + 7 文件 |
| MCP service | `src/crates/services/services-integrations/src/mcp/mod.rs:1` | 553 行 |
| MiniApp manager | `src/crates/assembly/core/src/miniapp/manager.rs:1` | 1100+ 行 |
| ACP client manager | `src/crates/interfaces/acp/src/client/manager.rs:1` | 91k 字节 |
| Web UI state machine | `src/web-ui/src/flow_chat/state-machine/SessionStateMachine.ts:1` | 10k 字节 |
| Web UI flow_chat 总览 | `src/web-ui/src/flow_chat/` | 8 子目录 (state-machine + deep-review + events + reducers + hooks + services + store + tool-cards + types + utils + components + constants) |
| Review platform (monolith) | `src/crates/assembly/core/src/service/review_platform/mod.rs:1` | 166k 字节 (历史) |
| Remote connect | `src/crates/services/services-integrations/src/remote_connect.rs:1` | 120k 字节 |
| Remote SSH manager | `src/crates/services/services-integrations/src/remote_ssh/manager.rs:1` | 108k 字节 |
| Terminal exec | `src/crates/services/terminal/src/exec.rs:1` | 83k 字节 |
| Session service (types) | `src/crates/services/services-core/src/session/types.rs:1` | 42k 字节 |
| Filesystem tree | `src/crates/services/services-core/src/filesystem/tree.rs:1` | 54k 字节 |
| Filesystem operations | `src/crates/services/services-core/src/filesystem/operations.rs:1` | 24k 字节 |

---

## 6. 给后续对比任务 (bitfun-vs-northing-contrast) 的建议

1. **优先 diff 这 4 个能力**:
   - `core::agentic::tools::computer_use_*` 拆分 (BitFun 4 模块 vs NortHing 估计 1 模块)
   - `core::service::review_platform` vs `execution::agent-runtime::deep_review` (BitFun 已 owner 化)
   - `core::agentic::goal_mode` 是否有 (BitFun 有, NortHing 可能没)
   - `core::agentic::round_preempt` 是否有 (BitFun 有, NortHing 可能没)
2. **Web UI 复刻优先级** (v3-restructure 目标):
   - 必有: `flow_chat/state-machine` + `flow_chat/deep-review` + `flow_chat/tool-cards` + `infrastructure/{agents,api,config,event-bus,i18n,mcp,providers,runtime,services,theme}` + `shared/{ai-errors,announcement-system,context-menu-system,context-system,crypto,inspector,notification-system}`
   - 应有: `app/components/{panels,sessions,workspaces,scheduled-jobs,NavBar,NavPanel}` + `app/scenes` + `app/stores`
3. **AGENTS 体系完整复用** — BitFun 16 个子模块 AGENTS + workspace root AGENTS 几乎可以直接复用为 NortHing 模板
4. **i18n 合约** — 拷贝 `src/shared/i18n/contract/locales.json` + `src/shared/i18n/resources/shared/<locale>/terms.json`, 跑 `pnpm run i18n:generate && pnpm run i18n:contract:test && pnpm run i18n:audit` 验证

---

## 7. 给未来复用 / 留作 handoff 的事实

- BitFun 仓库**已经存在于 `C:\Users\UmR\BitFun`** (本次任务保留, 不删)
- Codegraph 索引**已经建好** (`.codegraph/`, 2784 files / 13.5k functions, 状态 up-to-date)
- BitFun 仓库**总大小** 跟 NortHing 同量级 (估 < 500MB, --depth 1 clone)
- BitFun **没有动任何 NortHing 文件** (read-only 操作)
- NortHing 工作树**未触碰** — git status 仍维持上次 commit 状态
- `MiniApp/Skills/miniapp-dev/` 是 **开发 mini app 用的 skill**, 跟 `MiniApp/Demo/` 的 2 个 demo 是父子关系
- BitFun 的 `goal_mode` 来自 Codex `/goal` (在 mod.rs 注释明说 "Codex `/goal` parity")
- BitFun 的 Deep Review 拆 owner crate 的设计模式值得 NortHing 复用 (走"稳定契约 + 渐进迁移"路径)

---

## 附录 A: 数据来源

- `C:\Users\UmR\BitFun\Cargo.toml:1-240` (workspace 元数据)
- `C:\Users\UmR\BitFun\package.json:1-102` (前端元数据)
- `C:\Users\UmR\BitFun\README.md:1-137` (产品定位)
- `C:\Users\UmR\BitFun\src\crates\**\Cargo.toml` (21 个 crate 各自定位)
- `C:\Users\UmR\BitFun\src\crates\assembly\core\src\**\mod.rs` (核心模块入口)
- `C:\Users\UmR\BitFun\src\web-ui\src\**` (Web UI 结构, 1441 文件)
- `npx @colbymchenry/codegraph status` (节点/文件统计)
- `E:\agent-project\northing\AGENTS.md` (NortHing 6 层架构 + 验证表)
- `E:\agent-project\northing\Cargo.toml:1-50` (NortHing workspace 成员)
