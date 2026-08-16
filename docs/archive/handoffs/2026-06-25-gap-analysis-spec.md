# NortHing 基本功能差距分析 + 优先级实现 Spec

> **Date**: 2026-06-25
> **Inputs**:
> - T1 baseline: `docs/handoffs/2026-06-25-agent-baseline-features.md`（15 P0 + 4 P1 + 9 P2）
> - T2 state review: `docs/handoffs/2026-06-25-northhing-state-review.md`（501 行代码 review）
> - 已知问题: `docs/handoffs/2026-06-25-post-v010-p0-p1-rename-handoff.md` §3 + §4
> **作者**: Mavis（owner）—— T3 worker session 在写作过程中进入 error state，由 owner 接手完成
> **目的**: 把 15 P0 baseline 映射到当前实现，输出 5 项可在本轮立即动手的 P0 spec

---

## 0. TL;DR

NortHing v0.1.0 **后端能力 90% 完成**（14/15 P0 主体都在），但 **desktop GUI 30% 完成**——3 个体验锁（缺 startup session / 缺默认 providers / 缺 UI 错误展示）让"什么都不能干"。

**本轮 P0 范围**：5 项，总工作量 **~4 小时**，解锁 GUI 基本聊天循环（startup → new session → send message → 看见错误 → 重试）。

**P1 范围**：5 项，总工作量 **~3 小时**，覆盖健壮性 + 主题一致性 + 调试能力。

**P2 范围**：5 项，总工作量 **~1 天**，清理死代码 + 加测试 + 战略能力（Skills/Background Agent 仍不在此范围）。

---

## 1. 差距矩阵（baseline × 现状）

> 行 = T1 baseline 19 项 P0+P1 能力
> 列 = 期望 / 现状 / 差距 / 优先级
> 期望列：6/6 表示该能力是 6 个调研对象都有的"基本"

| ID | baseline 能力 | 期望 | 现状 | 差距 | 优先级 |
|---|---|---|---|---|---|
| **A1** | 会话管理（新建/删除/切换/重命名/搜索） | 6/6 | ⚠️ 后端 ✅ desktop ⚠️ | desktop 无 startup auto-create session | **P0** |
| **A2** | 发送消息 + 多轮 + 流式 | 6/6 | ⚠️ 后端 ✅ desktop ⚠️ | `on_send_message` 在 `session_id == ""` 时早退，依赖 A1 修复 | **P0**（依赖 A1） |
| **A3** | 流式响应 SSE | 6/6 | ✅ 完整 | `agent-stream` 模块完整 | — |
| **A4** | 历史回看 + 全文搜索 | 6/6 | ✅ 完整（独立 `service/search/`） | 无 | — |
| **B1** | 模型切换 | 6/6 | ✅ 5 种选择子 | 无 | — |
| **B2** | 多 Provider | 6/6 | ✅ OpenAI/Anthropic/Gemini + 3 种 Auth | 无 | — |
| **B3** | API Key 管理 | 6/6 | ✅ `AIModelConfig.api_key` | 无 | — |
| **B4** | 自定义 Base URL | 6/6 | ✅ `AIModelConfig.base_url` | 无 | — |
| **C1** | 读/写/编辑文件 | 6/6 | ✅ `file_read_tool/write_tool/edit_tool` | 无 | — |
| **C2** | Terminal/Shell | 6/6 | ✅ `bash_tool` + `exec_command/` | 无 | — |
| **C3** | grep/glob | 6/6 | ✅ `grep_tool/glob_tool/ls_tool` | 无 | — |
| **C4** | Diff 视图 + 检查点 | 6/6 | ✅ `get_file_diff_tool` + snapshot 8 文件 | 无 | — |
| **C5** | Git 操作 | 5/6 | ✅ `git_tool` | 无 | — |
| **D1** | Plan / 审批 | 5/6 | ✅ `PlanMode` + `create_plan_tool` + `ShellSecurityConfig` | 无 | — |
| **D2** | Subagent 委派 | 3/6 | ✅ `task_tool` + 5 个 builtin agent | 无 | — |
| **E1** | 会话重命名/搜索/删除 | 6/6 | ✅ AI title gen + `SessionTitleMethod` | 无 | — |
| **E2** | 导入/导出 | 6/6 | ✅ `SessionTranscriptExport` | 无 | — |
| **F1** | 侧边栏 | 6/6 | ✅ `SidebarView.slint` | 无 | — |
| **F2** | 状态栏 | 6/6 | ⚠️ UI 在 / 后端 MCP 全局注册缺失 + Model 段永远 Not configured | 见 P0-B + P0-D | **P0** |
| **F3** | 错误展示 / 重试 | 6/6 | ❌ 9 个 callback 全部 `eprintln!`，UI 无反馈通道 | 见 P0-C | **P0** |
| **F4** | 主题切换 | 6/6 | ✅ `ThemesConfig` | CLI `tool_cards.rs` 2 处 hardcoded | **P1** |
| **F5** | 设置页 | 6/6 | ✅ `ConfigManager::set` + dot-path | 无 | — |
| **G1** | Remote Workspace | 5/6 | ✅ 完整（3446 行 `remote_connect.rs`） | SSH port-forwarding 4 处 stub | **P2** |
| **G2** | Background Agent | 3/6 | ❌ 完全缺失 | 不在本轮范围 | P3 |
| **G3** | 跨端协同 | 3/6 | ⚠️ 部分（web + mobile-web 独立前端） | 同步层缺失 | P3 |
| **H1** | MCP 协议 | 6/6 | ⚠️ 协议层 ✅ + desktop 全局注册缺失 | 见 P0-D | **P0** |
| **H2** | Skills 市场 | 4/6 | ❌（内部 `SkillActor` body 是 no-op） | 不在本轮范围 | P3 |
| **H3** | Rules / CLAUDE.md | 6/6 | ⚠️ 路径约定有 / 无自动加载到 system prompt | 不在本轮范围 | P2 |
| **H4** | Cron | 2/6 | ✅ 目录存在，未深查 | 不在本轮范围 | — |
| **H5** | Hooks | 1/6 | ❌ | 不在本轮范围 | P3 |

**汇总**：
- P0 baseline 完整 14/15（93%）—— A3/A4/B/C/D/E/F1/F5/G1 workspace 都好
- P0 部分 3/15（20%）—— A1/A2 desktop 端、F2 状态栏
- P0 缺失 1/15（6.7%）—— F3 错误展示
- P1 baseline 完整 1/4 —— 主题有但 CLI 不一致
- P1 部分 2/4 —— H1 协议有但 desktop 全局注册缺、H3 部分
- P1 缺失 1/4 —— H2 Skills 市场

---

## 2. P0 实现 spec（5 项，本轮立即动手）

> 顺序：先解锁"启动就能用"（P0-A）→ "配置有内容"（P0-B）→ "失败看得见"（P0-C）→ "MCP 状态对"（P0-D）→ "hang 能定位"（P0-E）

### P0-A：启动时自动建默认 session

| 项 | 内容 |
|---|---|
| **用户故事** | 作为用户，启动客户端后立即能看到一个空会话，sidebar 出现一项，输入消息可发送 |
| **验收标准** | 1. 启动客户端 ≤3s 内 sidebar 出现 "Default Session"；2. 输入消息点发送，消息出现在右侧；3. 不依赖用户手动点 New Session |
| **关键文件** | `apps/desktop/src/app_state/mod.rs:912`（`create_ui` `Ok(ui)` 之前） |
| **改法** | 1. 在 `create_ui` 末尾 spawn `coordinator.create_session("Default Session".to_string(), "code".to_string(), config).await`；2. 成功时调 `app_state.set_current_session_id(sid)` + `ui.set_current_session_id(sid.into())` + `refresh_sessions_ui`；3. 失败时走 P0-C 的错误通道（不要 eprintln） |
| **依赖** | 无（但要 P0-C 才能显示错误） |
| **风险** | `create_session` 可能因 coordinator 未初始化而失败 → 必须有 retry 一次 + 错误反馈 |
| **工作量** | 30 min（含手测） |

### P0-B：`create_default_config` 写默认 provider 占位

| 项 | 内容 |
|---|---|
| **用户故事** | 状态栏 Model 段显示 "anthropic / openai / gemini" 而不是 "Not configured" |
| **验收标准** | 1. 首次启动后 `~/.northhing/config/app.json` 包含 3 个 `enabled=false` 的 provider；2. 状态栏 Model 段显示 provider 列表；3. 用户在 Settings 填 API key 后能 `enabled=true` |
| **关键文件** | `crates/assembly/core/src/service/config/manager.rs:107-114` `create_default_config` |
| **改法** | 在 `create_default_config` 追加 `self.config.ai.models.push(AIModelConfig { id, provider, enabled: false, api_key: "".into(), base_url: default_for(provider), ... })` × 3（anthropic / openai / gemini），封装成 `Self::add_default_*` 辅助 |
| **依赖** | 无 |
| **风险** | 字段命名（`models[]` vs 新增 `providers[]`）影响面大——**owner 决策**：本 spec 推荐改 `models[]`（向后兼容） |
| **工作量** | 1 hr |

### P0-C：错误展示通道（error banner + 9 个 eprintln 改造）

| 项 | 内容 |
|---|---|
| **用户故事** | 用户操作失败时 GUI 立即显示错误信息（不是"什么都不能干"） |
| **验收标准** | 1. `main.slint` 加 `in-out property <string> session-error / input-error`；2. `SidebarView.slint` 顶部加 error banner（条件渲染）；3. 9 处 `eprintln!` 改为 `ui.set_session_error(format!(...))` / `ui.set_input_error(...)`；4. 加 `clear_error` callback；5. 失败时 banner 显示 + 5s 自动消失 |
| **关键文件** | `apps/desktop/src/ui/main.slint` + `SidebarView.slint`；`apps/desktop/src/app_state/mod.rs:402, 408, 428, 453, 504, 526, 562, 668, 717, 851`；`sessions.rs:168, 194` |
| **改法** | 1. Slint 加属性 + 顶部 banner；2. Rust 端 `eprintln!` → `ui.set_session_error(...)`；3. banner 5s 自动消失用 tokio::spawn + sleep |
| **依赖** | 无（但 P0-A/B 的失败也要走这个通道） |
| **风险** | Slint UI 改动需要重新编译 + 手测；error banner 风格需要 owner 决策（toast vs banner vs modal） |
| **工作量** | 1.5 hr |

### P0-D：desktop `main.rs` 调 `set_global_mcp_service()` + 删 MCP_INIT_STATUS 死代码

| 项 | 内容 |
|---|---|
| **用户故事** | 状态栏 MCP 段从 "Pending" 变为 "Ready"（或实际状态），而不是永远 Pending |
| **验收标准** | 1. 启动后 MCP 段显示真实状态；2. 全局 `GLOBAL_MCP_SERVICE` 不再 None；3. 死代码 `MCP_INIT_STATUS` / `get_mcp_init_status` / `get_mcp_status_text` 删除 |
| **关键文件** | `apps/desktop/src/main.rs` + `apps/desktop/src/app_state/inspector.rs:14-26` |
| **改法** | 1. 在 `init_agentic_system_for_desktop` 之后调 `service::mcp::set_global_mcp_service(arc.clone())`；2. 删 `MCP_INIT_STATUS` 等 3 个 API；3. `build_mcp_status_string` 改走 `service::mcp::get_global_mcp_service()` |
| **依赖** | 无 |
| **风险** | CLI 路径也用了 `MCP_INIT_STATUS`（`apps/cli/src/ui/startup.rs:627`），删除前要确认 CLI 端是否还能编译 |
| **工作量** | 30 min |

### P0-E：AIClientFactory::initialize_global 加 instrumentation + 修 hang

| 项 | 内容 |
|---|---|
| **用户故事** | 如果冷启动 hang，能从日志定位到具体哪个 await 点 |
| **验收标准** | 1. 在 4 个关键 await 点前后加 `tracing::info!(target: "init", "before/after X, took {:?}s", elapsed)`；2. 启动时 RUST_LOG=info 能看到完整 init 时间线；3. 如果发现 hang 真实位置（`PathManager` IO / `mode_config_canonicalizer` lock / `save_config` 写盘），给出修复 |
| **关键文件** | `crates/assembly/core/src/infrastructure/ai/client_factory.rs:237-257` + `service/config/global.rs:86-127` |
| **改法** | instrumentation + 跑一次冷启动看日志；如发现 hang，根据日志定位修 |
| **依赖** | 无（但 P0-A 修了之后 send 仍可能卡——必须先 instrumentation 才能确认） |
| **风险** | 这是 diagnostic 任务，不保证一定修好 hang；可能需要再开一轮 |
| **工作量** | 15 min（instrumentation）+ 1-2 hr（如发现 hang） |

**P0 总工作量**：~4 hr（不含 hang 修复）

---

## 3. P1 实现 spec（5 项，本轮可选）

> 顺序独立，可并行。挑 2-3 项做性价比最高。

### P1-A：`mcp/config/json_config.rs` 2 处 `unreachable!()` 改 `Err`
- **位置**：`crates/services/services-integrations/src/mcp/config/json_config.rs:193, 217`
- **改法**：`_ => Err(anyhow::anyhow!("unsupported (source, url) combination: {:?}", (source, has_url)))`
- **风险**：无；与 handoff P1-extra-2 同型
- **工作量**：15 min

### P1-B：CLI `tool_cards.rs` 2 处 hardcoded theme 派生
- **位置**：`apps/cli/src/ui/tool_cards.rs:1051, 1239`
- **改法**：给 render 函数加 `theme: &Theme` 参数，从 `theme.is_dark()` 派生 `HighlightTheme::Dark/Light`
- **风险**：低
- **工作量**：30 min

### P1-C：5 处死代码清理（`SKILL_INSPECTOR_ENABLED` / `MCP_INIT_STATUS` / `USE_SOFTWARE_FALLBACK` / `USE_SLINT_SHELL` / `SESSION_TREE_VIEW`）
- **位置**：`apps/desktop/src/main.rs:18-39, 49, 52, 57-58, 68-69`
- **改法**：删除 5 处死代码 / 永远 true 分支
- **风险**：低（已在 review 中确认是死代码）
- **工作量**：1 hr

### P1-D：AIClientFactory::initialize_global instrumentation（如果 P0-E 没做）
- 同 P0-E 描述的前半段
- **工作量**：15 min

### P1-E：CLI + desktop smoke test 起步
- **位置**：`apps/cli/src/modes/chat.rs:3362`、`apps/cli/src/ui/startup.rs:1977` + desktop `app_state/{sessions,inspector,inspector_model_status}.rs`
- **改法**：每个文件加 5-10 个 unit test 覆盖关键路径
- **风险**：低，但工作量大
- **工作量**：半天

**P1 挑 2 项性价比最高**：P1-A（15min 改 2 处）+ P1-C（1hr 清死代码）

---

## 4. P2 列表（本轮不做）

| ID | 项 | 工作量 |
|---|---|---|
| P2-1 | SSH port-forwarding 4 处 stub 实现 | 半天 |
| P2-2 | Rules / CLAUDE.md 自动加载到 system prompt | 半天 |
| P2-3 | Mobile web + Desktop web 跨端同步层 | 1 天 |
| P2-4 | `apps/cli/src/ui/startup.rs:1977` 12 条 TIPS emoji 改纯文本（违反 Logging 规则） | 30 min |
| P2-5 | `apps/desktop/src/app_state/{sessions,inspector,inspector_model_status}.rs` unit test | 半天 |

---

## 5. 实现顺序建议

### 5.1 P0 依赖图

```
P0-A (startup session)
   │
   └── P0-C (错误展示) ←──── P0-B (默认 providers)
                              │
                              └── P0-D (MCP 全局注册)

P0-E (instrumentation + hang) 独立
```

### 5.2 推荐执行序列

| 轮次 | 任务 | 工作量 | 是否可并行 |
|---|---|---|---|
| 1 | P0-A | 30 min | ❌（先做，让后续有 session） |
| 2 | P0-B | 1 hr | ✅ 可与 P0-D 并行 |
| 2 | P0-D | 30 min | ✅ |
| 3 | P0-C | 1.5 hr | ✅ 可与 P0-E 并行 |
| 3 | P0-E | 15 min | ✅（instrumentation） |
| 4 | hang 修复（如果 P0-E 发现） | 1-2 hr | — |
| 5 | P1-A + P1-C（可选收尾） | 1.25 hr | ✅ |

**总工作量（不含 hang 修复）**：~3.5 hr
**含 hang 修复**：~5.5 hr

### 5.3 Owner 决策点（实现前需确认）

1. **P0-B 字段命名**：在 `models[]` 加 3 个 `enabled=false` 占位（推荐，影响面小） vs 新增 `providers[]` 字段（更语义化但要改 schema 迁移）
2. **P0-C 错误展示样式**：顶部 banner（推荐） vs 右下角 toast vs modal
3. **P0-D 是否同步清理 CLI 端的 `MCP_INIT_STATUS` 引用**（`apps/cli/src/ui/startup.rs:627`）

---

## 6. 风险与开放问题

### 6.1 实施期风险

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| P0-A `create_session` 因 coordinator 未初始化失败 | 高 | GUI 仍不可用 | 加 retry 一次 + P0-C 错误展示 |
| P0-B `models[]` 字段命名 owner 决策延后 | 中 | 阻塞 P0-B | 默认走推荐方案，owner 后续可调整 |
| P0-C Slint UI 改动引入编译错误 | 中 | 阻塞 P0-C | 改完后跑 `cargo check -p northhing-desktop` 验证 |
| P0-D 删除 `MCP_INIT_STATUS` 后 CLI 编译失败 | 中 | 阻塞 P0-D | 先 grep CLI 端引用，确认无引用再删 |
| P0-E 找不到 hang 真实位置 | 中 | GUI 仍卡 | 保留 instrumentation 长期有用；下一轮再排查 |
| Slint 生成代码 + unsafe 块改动需要 owner 评审 | 高 | 速度变慢 | 提前把 Slint diff 给 owner 确认 |

### 6.2 架构层面问题（本轮不修，但记录）

1. **GUI 错误反馈架构缺失**：`on_*` callback 全靠 stderr/debug log，应设计统一的 `ErrorReporter` trait，让 desktop 端能自定义 sink（banner / toast / 日志）
2. **apps/desktop 没有 AGENTS.md**：与根 `AGENTS.md` 的 `agent-doc priority` 规则不一致，建议补
3. **轻量 actor vs 真 phase 1/2/3 路径关系未理清**：`apps/desktop/src/agent/actor.rs:376-396` 自承 "demonstration wiring, not a replacement"，需后续澄清
4. **Background Agent / Skills 市场 / Hooks 三大战略能力完全缺失**：v0.2 路线图需要列入

### 6.3 文档同步

- 本 spec 完成后，更新 `docs/handoffs/2026-06-25-post-v010-p0-p1-rename-handoff.md` §4，把优先级表替换为新 spec
- 实施完成后，更新根 `AGENTS.md` 的「Verification」表，加入 P0 修复后的 cargo check 命令
- 创建 `docs/handoffs/2026-06-25-baseline-implementation-handoff.md` 记录本轮实施结果

---

## 7. 一句话总结

> **5 项 P0（~4 hr）= 解锁 GUI 聊天循环**；P0-A/B/C/D 都是"小改动大体验"型，性价比极高；P0-E 是诊断性任务，不保证一定修好 hang 但能定位。Owner 3 个决策点（字段命名、错误样式、CLI 端 MCP 引用清理）需要在 P0-B/C/D 实施前拍板。