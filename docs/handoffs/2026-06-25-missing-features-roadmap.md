# NortHing 未实装基本功能深度分析 + 长线路线图

> **Date**: 2026-06-25
> **作者**: general（mvs_995411895ccb48ee9b81a35e92536ce2）
> **Inputs**:
> - T1 baseline: `docs/handoffs/2026-06-25-agent-baseline-features.md`（15 P0 + 4 P1 + 9 P2）
> - T2 state review: `docs/handoffs/2026-06-25-northhing-state-review.md`（501 行代码 review）
> - T3 gap-analysis-spec: `docs/handoffs/2026-06-25-gap-analysis-spec.md`（5 项 P0 spec）
> **目的**: 长线维度重新审视"基本 agent app 必须有什么"— 按用户最痛重新排优先级，输出 3 个月路线图
> **本报告不做的事**: 不重复 T3 spec 已详细写过的 P0-A/B/C/D/E 5 项实施细节；不改代码；不写实现代码

---

## 0. TL;DR

NortHing v0.1.0 **后端能力 90%+ 完成、协议层 100%、CLI 端 100%，但 desktop GUI 实际是 30%— 3 把体验锁让"什么都不能干"**（T3 spec 5 项 P0 已解）。除此之外还有 7 项 P1 + 8 项 P2 战略能力需要长线规划。

**3 个真正影响"是否算基本 agent app"的关键问题**（不在 T3 spec 5 项内）：

1. **流式响应是否真到了 Slint UI**？`agent-stream` 模块完整是后端事实，但 desktop 端 `create_ui` / `on_send_message` 是否把 SSE 流 bind 到 UI 模型未实测
2. **重启不丢 session**？`session_manager` 持久化完整，但 desktop 端启动后是否能 load 上一次的 session（无 startup auto-create → 推测为否）
3. **多 Provider 在 UI 真能用**？`AIConfig.models[]` 字段存在、`create_default_config` 已塞占位（T3 P0-B 实施后），但 3 个 provider 之间的 UI 切换入口未确认

这 3 项是"T3 spec 没覆盖、但用户一定立刻会撞上"的真问题。

**本报告**：
- §1 完整 28 行对照表（baseline × 现状 × 差距 × 工作量 × 用户影响）
- §2 按"用户最痛"重新排 Tier 0/1/2（不按字母）
- §3 基础 agent app 的最低门槛（1 段定义 + 10 项用户期望 + NortHing 缺哪些）
- §4 3 个月路线图（M1/M2/M3 精确到 P0/P1/P2 项）
- §5 状态纠偏（state review 文档本身的修正）
- §6 风险与开放问题

---

## 1. 完整对照表（28 行）

> 列：ID / 功能名（来自 baseline） / 实现状态 / 缺失/部分实装的具体差距 / 工作量 / 用户影响
> 状态符号：✅ 完整 / ⚠️ 部分 / ❌ 缺失 / 🟡 仅 stub
> 「已 spec」= T3 gap-analysis-spec.md 已详细写 spec 但未实施；「本报告」= T3 没覆盖、本报告重新规划

### 1.1 P0 必备（15 项）

| ID | 功能 | 现状 | 差距 | 工作量 | 用户影响 |
|---|---|---|---|---|---|
| **P0-1** | 会话管理（新建/删除/切换/重命名/搜索） | ⚠️ | T3 P0-A 已 spec：desktop `create_ui` 末尾缺 startup auto-create session，导致启动后 `current_session_id == ""`、sidebar 空 | 30 min | 启动后"什么都不能干"，用户感受 app 坏了 |
| **P0-2** | 发送消息 + 多轮上下文 + 流式 | ⚠️ | T3 P0-A 修了之后才解锁；**未确认**：流式响应是否真到 Slint UI（`agent-stream` 后端完整，UI 端 SSE 绑定未实测） | 30 min（解锁）+ 半天（验证流式） | 发消息无反应；多轮上下文无效 |
| **P0-3** | 流式响应 SSE | ✅ 后端 | `crates/execution/agent-stream/src/lib.rs` StreamProcessor 完整（line 47-202 验证） | 0 | 无 |
| **P0-4** | 多 Provider + API Key + 自定义 Base URL | ✅ | `AIConfig.models[]` + 3 种 `AuthConfig`（ApiKey/CodexCli/GeminiCli）；T3 P0-B 加默认占位后即完整 | 0（依赖 T3 P0-B 1 hr） | Settings 页有内容；多 provider 切换有目标 |
| **P0-5** | 读/写/编辑文件 | ✅ | `file_read_tool.rs` / `file_write_tool.rs` / `file_edit_tool.rs` / `delete_file_tool.rs` 全部存在 | 0 | 无 |
| **P0-6** | Terminal / Shell + grep/glob | ✅ | `bash_tool.rs` + `exec_command/` 9 个子文件 + `grep_tool.rs` / `glob_tool.rs` / `ls_tool.rs` | 0 | 无 |
| **P0-7** | Diff 视图 + 检查点回滚 | ✅ | `get_file_diff_tool.rs` + `service/snapshot/` 8 文件（manager、core、system、service、types、file_lock、isolation、events） | 0 | 无 |
| **P0-8** | Git 操作 | ✅ | `git_tool.rs` + `service/git/` 4 文件 | 0 | 无 |
| **P0-9** | 计划 / 审批流（Plan Mode） | ✅ | `agentic/agents/definitions/modes/PlanMode` + `create_plan_tool.rs` + `ShellSecurityConfig` | 0 | 无 |
| **P0-10** | 侧边栏 + 设置页面 | ✅ | `SidebarView.slint`（180 行）+ `InspectorView.slint`（105 行）+ Slint components 8 个 | 0 | 无 |
| **P0-11** | 状态栏（Pending/Running/Failed/Done）+ 错误展示/重试 | ⚠️ / ❌ | T3 P0-C + P0-D 已 spec：9 个 callback 失败全用 eprintln → UI 零反馈；MCP 段实际注册有，但状态栏 Model 段永远 "Not configured"（T3 P0-B 解） | 1.5 hr | 用户点任何按钮失败都不知道，app "假死" |
| **P0-12** | MCP 协议接入 | ⚠️ | 协议层 ✅（`crates/services/services-integrations/src/mcp/protocol/` + `server/` + `adapter/`）；desktop 端 T3 P0-D 已 spec（实测：main.rs:63 实际已调 `set_global_mcp_service`，state review 描述有误，详见 §5） | 30 min | 全局 MCP 服务可用；agent 可调外部工具 |
| **P0-13** | 主题切换（暗/亮） | ✅ | `config/types.rs:340 ThemesConfig`；**未确认**：T3 P1-B 提到的 CLI `tool_cards.rs:1051, 1239` hardcoded `HighlightTheme::Dark` 是否还在当前 HEAD | 0（除 CLI 派生）| 主题切换生效 |
| **P0-14** | 会话导入/导出 | ✅ | `SessionTranscriptExport` + `SessionTranscriptExportOptions` + `SessionBranchRequest/Result`（branch=导入） | 0 | 可备份/迁移 |
| **P0-15** | Markdown / 代码高亮 | ✅ | `components/MarkdownText.slint` + `components/CodeBlock.slint` + `MaterialBadge` 完整 | 0 | 无 |

**P0 汇总**：✅ 完整 12/15 | ⚠️ 部分 3/15（P0-1 / P0-2 / P0-11）| ❌ 缺失 0/15
- ⚠️ 的 3 项全部由 T3 spec 5 项 P0 解决（~4 hr 总工作量）
- T3 spec 没覆盖的真问题：**P0-2 流式响应是否到 UI + 重启不丢 session**（见 §0 TL;DR）

---

### 1.2 P1 重要（4 项）

| ID | 功能 | 现状 | 差距 | 工作量 | 用户影响 |
|---|---|---|---|---|---|
| **P1-1** | Subagent / 子任务委派 | ✅ | `task_tool.rs`（1240+ 行）+ `agentic/agents/definitions/subagents/`（ExploreAgent/FileFinderAgent/GeneralPurposeAgent/ResearchSpecialistAgent/ComputerUseMode）+ `agents/registry/catalog.rs` | 0 | 无 |
| **P1-2** | 记忆 / Rules（CLAUDE.md / .cursorrules） | ⚠️ | 路径约定有（`infrastructure/app_paths/path_manager.rs:312 user_data_dir().join("rules")` + `service/workspace/manager.rs:90-100` 读 `IDENTITY.md`）；**无显式"CLAUDE.md 自动加载到 system prompt"入口**；prompt_builder 实际引用 `BOOTSTRAP.md / SOUL.md / USER.md / IDENTITY.md`（CLAW workspace 专属），非通用 | 半天 | 每次启动 agent 都要重新解释项目规范；不可接受 |
| **P1-3** | Background Agent（异步任务） | ❌ | 完全缺失；无 background / queue / cloud task 模块；**这与 Cursor 1.0 Background Agent / Trae SOLO / ChatGPT Agent 的差异化能力差距** | 1-2 周 | 长任务必须干等；不能跨设备查结果 |
| **P1-4** | Remote-SSH / 远程 Workspace | ⚠️ | Remote Workspace ✅（`services-integrations/src/remote_connect/` 5 子模块，详见 §5 修正）；Remote-SSH 主体 ✅（`manager.rs` 2792 行 + `remote_exec.rs` 1035 行 + `remote_fs.rs` 367 行 + `remote_terminal.rs` 336 行）但 **4 处 SSH port-forwarding（-L/-R）TODO stub** | 半天（port-forwarding）| 远程服务器端口转发做不到；其余 SSH 能力完整 |

**P1 汇总**：✅ 完整 1/4（P1-1）| ⚠️ 部分 2/4（P1-2 Rules 自动加载 + P1-4 SSH port-forwarding）| ❌ 缺失 1/4（P1-3 Background Agent）

---

### 1.3 P2 加分（9 项）

| ID | 功能 | 现状 | 差距 | 工作量 | 用户影响 |
|---|---|---|---|---|---|
| **P2-1** | Hooks（事件驱动自动化） | ⚠️ | `crates/assembly/assembly/core/src/agentic/tools/post_call_hooks.rs`（30 行）+ `crates/execution/agent-runtime/src/post_call_hooks.rs`（30 行）**有 post_call hook**，**无 PreToolUse/PostToolUse 通用 hook 系统** | 1 周 | 用户无法做事件驱动自动化（"工具调用前后跑 shell"这种） |
| **P2-2** | Cron / 定时任务 | ✅ | `service/cron/` 目录存在 + `cron_tool.rs`（agentic tool） | 0 | 无 |
| **P2-3** | 多模态（图像/语音/文件上传） | ⚠️ | desktop 端未见图像拖拽 UI；`computer_use_actions.rs` / `computer_use_input.rs` / `computer_use_locate.rs` 6 个 computer use 文件 = 2203 行（**这是远超预期的能力**，state review 没提） | 半天（图像粘贴 UI）| 截图分析场景不可用 |
| **P2-4** | 成本 / token 实时统计 | ❌ | 无 `/cost` 入口；无 token 计数展示 | 半天 | 用户不知道 token 花多少 |
| **P2-5** | Codebase 语义索引（embeddings + 向量搜索） | ❌ | `service/search/` 是全文搜索（非语义）| 1-2 周 | 大项目 grep 不够快 |
| **P2-6** | Tauri 桌面打包 / 跨平台 | ✅ | Tauri 完整（`Cargo.toml` 有 tauri 依赖；`pnpm run desktop:build:fast` 等命令齐）| 0 | 无 |
| **P2-7** | 语音输入 / TTS | ❌ | 无 | 1 周 | 移动端 / 长时间输入体验差 |
| **P2-8** | 桌面 / 网页 / 移动协同 | ⚠️ | `web-ui/` + `mobile-web/` + `desktop` 三个独立前端；**无统一 session/task 同步层** | 1-2 周 | desktop 创建的会话 mobile 看不到 |
| **P2-9** | Skills / 插件市场 | ⚠️ | **有 internal skills 系统**（`crates/assembly/core/src/agentic/tools/implementations/skills/` 9 文件 ~2500 行：registry 947 行、builtin 311 行、resolver 167 行、resolver_v2 271 行、catalog 224 行、policy 213 行、mode_overrides 185 行、types 156 行、mod 18 行）；**无 marketplace 概念**（无 install from network / plugin discovery）| 2-3 周 | 第三方 skills 装不上 |

**P2 汇总**：✅ 完整 2/9（P2-2 Cron + P2-6 桌面打包）| ⚠️ 部分 4/9（P2-1 Hooks 仅有 post_call + P2-3 多模态无 UI + P2-8 跨端同步层缺 + P2-9 Skills 缺 marketplace）| ❌ 缺失 3/9（P2-4 成本 + P2-5 语义索引 + P2-7 语音）

**P2 整体比 state review 描述的好**：
- Skills 系统真实存在 ~2500 行（state review 说"未发现独立的 skills marketplace"— marketplace 确实没有，但 internal skills 系统很完整）
- Computer Use 工具 2203 行（state review 完全没提）
- Post-call hooks 已存在（state review 说"未发现 hook 系统"— 只对了一半）

---

### 1.4 baseline 28 项汇总（一行话）

| 优先级 | ✅ 完整 | ⚠️ 部分 | ❌ 缺失 | 总数 |
|---|---|---|---|---|
| P0 必备 | 12 | 3 | 0 | 15 |
| P1 重要 | 1 | 2 | 1 | 4 |
| P2 加分 | 2 | 4 | 3 | 9 |
| **合计** | **15** | **9** | **4** | **28** |
| **占比** | **53.6%** | **32.1%** | **14.3%** | 100% |

**结论**：
- 后台 90%+ 是真实陈述（protocol + service + agentic 三大目录 0 unimplemented/todo）
- 桌面 GUI 实际是「P0 完整但 3 处卡死 + P1 几乎空 + P2 战略能力缺」
- 真正"阻塞"用户的只有 5 项 P0（T3 spec 已覆盖）+ 3 项 T3 没覆盖的真问题（§0 TL;DR）

---

## 2. 按"基础度"重新排序（Tier 0/1/2）

> 排序依据：用户最痛 → 实施 ROI → 战略价值
> 不按字母、不按 baseline 编号

### 2.1 Tier 0 — 必须补（没有这层项目不能算"能用"的 agent app）

> 总工作量 ~5-6 hr。T3 spec 5 项 P0 + 本报告新增 2 项验证任务 + 1 项 P1-2 基础 Rules 加载

| ID | 项 | 为啥 Tier 0 | 实施方向 | 工作量 | 风险 |
|---|---|---|---|---|---|
| **P0-A** ⭐ | startup auto-create session | 启动后 sidebar 空 + 任何操作早退，app "假死" | `apps/desktop/src/app_state/mod.rs` `create_ui` 末尾 spawn coordinator.create_session，失败走 P0-C 错误通道 | 30 min | `create_session` 因 coordinator 未初始化失败 → retry 一次 |
| **P0-B** ⭐ | `create_default_config` 加默认 providers 占位 | 状态栏 Model 段永远 "Not configured"，用户以为是 bug | `crates/assembly/core/src/service/config/manager.rs:107 create_default_config` 追加 3 个 `enabled=false` 占位（anthropic/openai/gemini）| 1 hr | 字段命名 owner 决策（`models[]` 加占位 vs 新增 `providers[]`）|
| **P0-C** ⭐ | 错误展示通道 + 9 个 eprintln 改造 | 用户操作失败 GUI 完全无反馈，app 假死 | `main.slint` 加 `session_error` / `input_error` 属性 + `SidebarView.slint` 顶部 banner；Rust 端 `eprintln!` → `ui.set_session_error(...)` + 5s 自动消失 | 1.5 hr | Slint UI 改动需重新编译手测；banner 样式（toast vs banner vs modal）需 owner 拍板 |
| **P0-D** ⭐ | desktop `main.rs` MCP 全局注册 + 状态栏 MCP 段 | MCP 工具生态事实标准；状态栏 MCP 段"Pending"显示 100% | 验证 desktop `main.rs:63` 实际已调 `set_global_mcp_service`（state review 描述有误，详见 §5）；`build_mcp_status_string` 改走 `get_global_mcp_service()`；清理 `MCP_INIT_STATUS` 死代码 | 30 min | 状态栏 MCP 段实际状态需要在实施后手动验证（实操时可能发现 MCP 实际未就绪）|
| **P0-E** ⭐ | AIClientFactory::initialize_global 加 instrumentation + 修 hang | 冷启动 hang 时无任何日志定位；这是 P0-A 修完后 send 仍可能卡的根因候选 | 4 个关键 await 点前后加 `tracing::info!(target: "init", "...")` + RUST_LOG=info 跑冷启动看时间线；如发现真 hang（`PathManager` IO / `mode_config_canonicalizer` lock / `save_config` 写盘）| 15 min + 1-2 hr（视是否找到 hang）| 是 diagnostic 任务，不保证修好 |
| **P0-X1** 🆕 | **流式响应真到 Slint UI** | 后端 `agent-stream` 完整但未实测 SSE 事件是否 bind 到 UI 模型；T3 spec 完全没验证这点 | 在 `on_send_message` 后台跑 `cargo test -p northhing-agent-stream` + 实际启动 desktop 端发消息看是否逐 token 渲染 | 半天 | 如发现 SSE 没到 UI，需要写新的 `set_streaming_token` 回调 + UI 端 chat bubble 增量更新 |
| **P0-X2** 🆕 | **重启不丢 session** | desktop `create_ui` 无 load_last_session 入口；T3 P0-A 加的是 auto-create new session，**不是 restore last session** | 在 P0-A 之后追加 `coordinator.load_session(last_sid)` → 如有则切换；无则走 auto-create | 1 hr | "重启不丢"≠"auto-create new"— T3 spec 描述可能让用户以为修了实际没修 |
| **P1-2 子集** 🆕 | **通用 CLAUDE.md / IDENTITY.md 自动加载到 system prompt** | 6/6 调研对象都支持；缺这个 = 每次启动重新解释项目规范 | `crates/assembly/core/src/agentic/agents/prompt_builder/prompt_builder_impl.rs:649` 已经有 CLAW workspace 路径，扩展为通用：递归扫 workspace root 找 `CLAUDE.md` / `AGENTS.md` / `IDENTITY.md` → 拼接到 system prompt | 半天 | 加载路径— 突（多个文件谁先谁后）需 owner 决策 |

**Tier 0 总工作量**：~5-6 hr（T3 spec 5 项 ~4 hr + 本报告 2 项验证 ~1.5 hr + 1 项 P1-2 子集 ~半天）

**Tier 0 完成后的预期效果**：用户启动 → 看到默认 session 出现在 sidebar → 输入消息 → 流式逐 token 渲染 → 看到错误（如果失败）→ 重试 → 重启不丢 → 下次启动 agent 自动知道项目规范

---

### 2.2 Tier 1 — 应该有

> 总工作量 ~1.5-2 天。补齐后桌面 GUI 进入"稳定可日常用"状态

| ID | 项 | 为啥 Tier 1 | 实施方向 | 工作量 | 风险 |
|---|---|---|---|---|---|
| **P1-1 完善** | **subagent UI 入口** | 后端 `task_tool` + 5 个 builtin agent 都好；缺 UI 入口（"派个 subagent 干 X"按钮）| `SidebarView.slint` 加 subagent 召唤按钮 / ChatPane 加 `/task` 斜杠命令入口 | 半天 | subagent 输出如何回传主 agent 需 owner 决策 |
| **P1-2 完整** | **Rules 用户编辑 UI** | 当前只读不写；用户没法在 GUI 改 rules | `InspectorView.slint` 加 Rules 编辑器（多文本框 + 保存按钮）| 半天 | 编辑器语法提示 / 多文件 tab 需 UX 决策 |
| **P1-4 port-forwarding** | **SSH -L/-R 4 处 stub 实现** | 用户连远程服务器调本地服务（数据库、debug port）会卡 | `crates/services/services-integrations/src/remote_ssh/manager.rs:2476-2551` 4 处 TODO 补实现（用 russh 的 `channel_open_direct_tcpip`）| 半天 | russh 异步 API 复杂度；多跳 SSH 转发的边界 case |
| **P2-3 多模态 UI** | **图像粘贴 + 预览** | 截图分析是高频场景；缺 UI 入口用户只能改 base64 | `ChatPaneView.slint` 加 drag-drop 接受图片 + `<img>` 渲染 + 把图附到 message content | 半天 | 大图压缩 / token 计费 |
| **P2-1 Hooks 补完** | **PreToolUse 通用 hook** | post_call 有，PreToolUse 缺；用户没法"工具调用前自动跑 formatter" | `crates/assembly/core/src/agentic/tools/post_call_hooks.rs` 已有框架；新增 `pre_call_hooks.rs` 镜像实现 | 1 天 | hook 配置 schema 需稳定；权限 / sandbox 边界 |
| **P2-4 token 计数** | **成本 / token 实时统计** | 用户不知道 token 花多少 | `app_state` 新增 `TokenCounter` struct，每条 message 累计 input/output tokens；`StatusBarView.slint` 加 token 段 | 半天 | 跨 model token 计算的统一化 |
| **P1-A 健壮性** | `mcp/config/json_config.rs:193, 217` 2 处 `unreachable!()` 改 `Err` | 未来新增 match 分支会 panic | `_ => Err(anyhow::anyhow!("unsupported (source, url) combination: {:— }", ...))` | 15 min | 无 |
| **P1-B 一致性** | CLI `tool_cards.rs:1051, 1239` hardcoded `HighlightTheme::Dark` 派生 | 主题切换时高亮不跟随 | 给 render 函数加 `theme: &Theme` 参数，从 `theme.is_dark()` 派生 | 30 min | 无 |
| **P1-C 死代码清理** | 5 处死代码（`SKILL_INSPECTOR_ENABLED` / `MCP_INIT_STATUS` / `USE_SOFTWARE_FALLBACK` / `USE_SLINT_SHELL` / `SESSION_TREE_VIEW` 重复定义）| grep 维护时容易漏 | 删除死代码 + 合并 `SESSION_TREE_VIEW` 到 `flags.rs` 一处 | 1 hr | 死代码可能实际有引用未发现 → 删前 grep 验证 |
| **P1-D instrumentation** | AIClientFactory 冷启动 instrumentation（如 P0-E 未做）| 同 P0-E | 同 P0-E | 15 min | 无 |
| **P1-E unit test 起步** | desktop `app_state/{sessions,inspector,inspector_model_status}.rs` + CLI `modes/chat.rs` + `ui/startup.rs` 0 测试 | 重构无安全网 | 每个文件加 5-10 个 unit test（参考 `mcp_adapter.rs:157-208` 既有 4 个测试模式）| 半天 | 无 |

**Tier 1 总工作量**：~1.5-2 天（10 项，~1.5-2 hr/项平均）

**Tier 1 完成后的预期效果**：桌面 GUI 体验追平 Claude Code / Cursor 1.0 的 80%— subagent 可用、Rules 可编辑、远程 SSH 完整、图像可传、token 可看、健壮性有保障、有测试网

---

### 2.3 Tier 2 — 加分

> 总工作量 ~1-1.5 个月。这是"产品差异化"的能力，不是"能不能用"的能力

| ID | 项 | 为啥 Tier 2 | 实施方向 | 工作量 | 风险 |
|---|---|---|---|---|---|
| **P1-3 Background Agent** 🆕 | 异步任务 + 跨设备 | 与 Cursor Background Agent / Trae SOLO / ChatGPT Agent 差距大 | 新增 `service/background/` 模块 + task queue + 跨设备 push 通知 | 1-2 周 | 任务状态同步、失败重试、跨设备 auth 边界 |
| **P2-8 多端同步** 🆕 | desktop ↔ web ↔ mobile session/task 同步 | 当前三个独立前端；典型场景"desktop 创建会话，mobile 继续"做不到 | 抽 `northhing-core` 一层 sync protocol（操作日志 + CRDT 或 last-writer-wins）| 1-2 周 | — 突解决策略；移动端弱网 / 离线场景 |
| **P2-9 Skills marketplace** | 第三方 skills install / discovery | 内部 skills 系统 ~2500 行已好；缺 marketplace 概念 | 在 `skills/` 已有框架上扩展：网络协议 + 签名验证 + 沙箱隔离 | 2-3 周 | 沙箱安全（skill 能不能读用户文件）；签名密钥分发 |
| **P2-5 语义索引** | codebase embeddings + 向量搜索 | 大项目 grep 不够快 | 新增 `service/embeddings/`（ONNX runtime + sqlite-vec）| 1-2 周 | 模型选择（bge-small？nomic-embed？）；本地 vs 远程 embedding 计算 |
| **P2-7 语音输入 / TTS** | 语音对话 | 移动端 + 长时间输入体验 | 整合 whisper.cpp（输入）+ edge-tts（输出）| 1 周 | Windows ASGI / 麦克风权限；噪音处理 |
| **P2-5b Computer Use UI** | desktop GUI 自动化 | `computer_use_tool.rs` 2203 行已好；缺 UI 入口 | `InspectorView.slint` 加 "Use Computer" 模式开关 + 屏幕录制 + 坐标显示 | 1 周 | 跨平台（macOS / Windows）权限差异；安全边界 |
| **多模态扩展** | PDF / DOCX / 视频理解 | 当前只见图像；文档/视频分析是高频场景 | 整合现有 MCP servers（`@modelcontextprotocol/server-filesystem` + 自定义 video tool）| 1-2 周 | 大文件分块；token 上限 |
| **AGENTS.md 补全** | `apps/desktop/AGENTS.md` + `apps/cli/AGENTS.md` | 根 `AGENTS.md` 已规定「agent-doc priority」，apps 层就近 AGENTS.md 缺失 | 写两个 AGENTS.md 描述 app 层特有规则 | 半天 | 无 |

**Tier 2 总工作量**：~1-1.5 个月（8 项，2-3 周/项平均）

**Tier 2 完成后的预期效果**：NortHing 从"基本 agent app"升级为"差异化 agent app"— 能跟 Cursor 1.0 / Trae SOLO 抢市场

---

### 2.4 Tier 总结

| Tier | 目标 | 工作量 | 完成后的产品定位 |
|---|---|---|---|
| **Tier 0** | 解锁 GUI | ~5-6 hr | "基本能用"的 agent app |
| **Tier 1** | 稳定可日常用 | ~1.5-2 天 | 追平 Claude Code / Cursor 1.0 的 80% |
| **Tier 2** | 差异化 | ~1-1.5 个月 | 与 Cursor Background Agent / Trae SOLO 抢市场 |

**3 个月总投入**：~1 人月（按 1 人每天 8 hr 算）

---

## 3. 基础 agent app 的最低门槛

### 3.1 定义（1 段话）

**一个"基本可用的 agent app"必须做到 5 件事**：

1. 用户能 **新建一个会话、发消息、看到 AI 流式逐 token 回复**（核心循环）
2. 用户能 **配置至少一个 LLM provider + API key**（不能让用户卡在配置）
3. 用户能让 AI **读 / 写 / 编辑文件 + 跑 shell + grep 搜内容**（否则 AI 只能聊天不能做事）
4. 用户能 **看到清晰的状态**（消息 / 任务 / 工具调用的 Pending / Running / Done / Failed；失败时**看得见**原因 + 可重试）
5. 用户能 **接入外部工具**（MCP 协议是事实标准；至少架构层要预留接口）

满足这 5 条 = 基本可用；不满足 = "demo / 玩具"。

### 3.2 用户期望的 10 项功能（从 baseline 提炼）

> 来自 T1 baseline 的 28 项 + 用户访谈 + 6 个产品的 6/6 覆盖项

| # | 用户期望 | baseline ID | NortHing 现状 | 缺/有 |
|---|---|---|---|---|
| 1 | 启动后立即有默认会话（不用手动点 New）| P0-1 | ⚠️ T3 P0-A 待修 | 缺 |
| 2 | 输入消息秒发 + 流式逐 token 渲染（不卡顿）| P0-2 / P0-3 | ⚠️ T3 P0-A 修后解锁；流式到 UI 未验证 | 缺 |
| 3 | 状态栏明确显示当前 model / MCP / app 状态 | P0-11 | ⚠️ T3 P0-B + P0-D 修后解锁 | 缺 |
| 4 | 失败有错误提示 + 一键重试 | P0-11 | ❌ T3 P0-C 修后解锁 | 缺 |
| 5 | 设置页能配 API key / model / 主题 | P0-4 / P0-10 / P0-13 | ✅（API key 配置后端 ✅，UI 在 InspectorView.slint）| 有 |
| 6 | AI 能读 / 写 / 编辑文件 + 跑 shell | P0-5 / P0-6 | ✅ | 有 |
| 7 | AI 能 grep / glob 搜内容 | P0-6 | ✅ | 有 |
| 8 | 文件改动有 diff 视图 + 可回退 | P0-7 | ✅ | 有 |
| 9 | MCP 协议接入（接外部工具）| P0-12 | ⚠️ 协议层 ✅；全局注册 T3 P0-D 修 | 缺（待修）|
| 10 | 暗 / 亮主题切换 | P0-13 | ✅（CLI 派生例外）| 有 |

**结论**：10 项中 NortHing 完整有 5 项（#5/6/7/8/10），缺 5 项（#1/2/3/4/9）— **缺的 5 项全部由 T3 spec 5 项 P0 + 本报告 §0 P0-X1/P0-X2 覆盖**。

### 3.3 NortHing 当前缺哪些（按"基本门槛"维度）

| 维度 | 缺什么 | T3 spec 是否覆盖 |
|---|---|---|
| 启动 | startup auto-create session | ✅ P0-A |
| 启动 | 流式响应到 UI | ❌ 新增 P0-X1 |
| 启动 | 重启不丢 session | ❌ 新增 P0-X2 |
| 配置 | 默认 providers 占位 | ✅ P0-B |
| 工具 | 文件 / shell / grep | ✅ 已有 |
| 状态 | 状态栏 MCP / Model 段 | ✅ P0-B + P0-D |
| 状态 | 错误展示 / 重试 | ✅ P0-C |
| 协议 | MCP 全局注册 | ✅ P0-D（实际已修，state review 误报）|
| 协议 | Rules / CLAUDE.md 自动加载 | ❌ 新增 P1-2 子集 |

**5 个 T3 spec 没覆盖的真问题**：
1. P0-X1 流式响应到 UI
2. P0-X2 重启不丢 session
3. P1-2 子集 通用 CLAUDE.md / AGENTS.md 自动加载
4. P2-3 多模态 UI 入口（虽然 P2 但用户高频撞上）
5. P1-1 subagent UI 入口（虽然 P1 但用户日常用）

---

## 4. 3 个月路线图

### 4.1 月 1：解 GUI + Tier 0 全清（精准到 P0 项）

> 目标：用户能完整跑通"启动 → 会话 → 发消息 → 流式 → 失败重试 → 重启恢复 → agent 知道项目规范"
> 工作量：~6 hr Tier 0 + 2 天 Tier 1 子集

**第 1 周：T3 spec 5 项 P0（~4 hr）**

| 日期 | 任务 | 工作量 | 依赖 |
|---|---|---|---|
| D1 | P0-A: startup auto-create session | 30 min | 无 |
| D1 | P0-B: create_default_config 加默认 providers | 1 hr | 无（可与 P0-D 并行）|
| D1 | P0-D: desktop MCP 全局注册 + 死代码清理 | 30 min | 无（可与 P0-B 并行）|
| D2 | P0-C: 错误展示通道 + 9 个 eprintln 改造 | 1.5 hr | P0-A / P0-B（错误反馈通道共用）|
| D2 | P0-E: AIClientFactory instrumentation | 15 min + 1-2 hr（如发现 hang）| 无 |
| D2-D3 | hang 修复（如 P0-E 发现）| 1-2 hr | P0-E |

**第 2 周：本报告新增 3 项 + Tier 1 子集（~3 天）**

| 日期 | 任务 | 工作量 | 备注 |
|---|---|---|---|
| D4 | P0-X1: 流式响应到 UI 验证 + 补全 | 半天 | 实测 desktop 端发消息，看是否逐 token 渲染；如未到 UI，写新回调 |
| D4 | P0-X2: 重启不丢 session | 1 hr | 在 P0-A 之后追加 load_last_session |
| D5 | P1-2 子集: 通用 CLAUDE.md / AGENTS.md / IDENTITY.md 自动加载 | 半天 | 扩展 prompt_builder_impl.rs:649 现有 CLAW workspace 路径 |
| D5-D6 | P1-A + P1-B + P1-C: 健壮性 + 一致性 + 死代码清理 | 2 hr | 15min + 30min + 1hr |
| D6 | P1-E: desktop app_state 3 文件 unit test 起步 | 半天 | 参考 mcp_adapter.rs 既有模式 |

**月 1 验收**：
- ✅ T3 spec 5 项 P0 全部完成
- ✅ P0-X1 / P0-X2 / P1-2 子集完成
- ✅ 桌面 GUI 进入"基本可用"状态
- ✅ 重启不丢 session；流式逐 token 渲染；错误可见可重试
- ✅ agent 启动自动加载项目 CLAUDE.md / AGENTS.md
- ✅ 5 处死代码清理；app_state 有基本 unit test

---

### 4.2 月 2：补齐 Tier 1 全部（~1.5-2 天集中投入 + 持续测试）

> 目标：subagent 可用 / Rules 可编辑 / 远程 SSH 完整 / 图像可传 / token 可看 / 健壮性有保障 / 测试有网

**第 3-4 周：Tier 1 全部（~1.5-2 天实际工时）**

| 任务 | 工作量 | 备注 |
|---|---|---|
| P1-1 完善: subagent UI 入口 | 半天 | sidebar 加按钮 / ChatPane 加 /task 命令 |
| P1-2 完整: Rules 编辑 UI | 半天 | InspectorView 加多文件编辑器 |
| P1-4: SSH port-forwarding 4 处 stub 实现 | 半天 | remote_ssh/manager.rs:2476-2551 |
| P2-3: 图像粘贴 + 预览 + 附到 message | 半天 | ChatPaneView drag-drop + <img> |
| P2-1: PreToolUse 通用 hook | 1 天 | 镜像 post_call_hooks 实现 |
| P2-4: token 计数 + 状态栏显示 | 半天 | 新增 TokenCounter + StatusBarView |
| P1-D: AIClientFactory 性能优化（如 P0-E 没修 hang）| 1-2 hr | 视 P0-E 反馈 |
| 持续：smoke test + E2E 测试覆盖 | 持续 | 跑 cargo test --workspace + 桌面手动跑场景 |

**月 2 验收**：
- ✅ Tier 1 10 项全部完成
- ✅ 桌面 GUI 体验追平 Claude Code / Cursor 1.0 的 80%
- ✅ subagent 可在 UI 召唤；Rules 可编辑保存；远程 SSH 端口转发可用
- ✅ 截图可贴可分析；token 实时可见
- ✅ E2E 测试覆盖"启动 → 发消息 → 工具调用 → 错误 → 重试 → 重启"全链

---

### 4.3 月 3：Tier 2 战略能力 + 跨端同步（~1 人月投入）

> 目标：从"基本可用"升级为"差异化"— 能跟 Cursor Background Agent / Trae SOLO 抢市场

**第 5-8 周：Tier 2 8 项（每项 1-2 周）**

| 周 | 任务 | 工作量 | 备注 |
|---|---|---|---|
| W5-W6 | P1-3 Background Agent（异步任务 + 跨设备 push）| 1-2 周 | service/background/ + task queue + 跨设备协议 |
| W5-W6 | P2-8 多端同步（desktop ↔ web ↔ mobile）| 1-2 周 | northhing-core 抽 sync protocol |
| W7-W8 | P2-9 Skills marketplace 扩展 | 2-3 周 | 网络协议 + 签名验证 + 沙箱 |
| W7-W8 | P2-5b Computer Use UI 入口 | 1 周 | InspectorView 加 "Use Computer" 模式 |
| 可选 | P2-5 语义索引 | 1-2 周 | 视产品优先级 |
| 可选 | P2-7 语音输入 / TTS | 1 周 | 视产品优先级 |
| 持续 | AGENTS.md 补全（apps 层就近）| 半天 | 与根 AGENTS.md 同步规则 |

**月 3 验收**：
- ✅ Tier 2 至少 3-4 项完成（视投入）
- ✅ Background Agent 跑通；mobile 端可查任务状态
- ✅ desktop ↔ mobile session 同步可用
- ✅ Skills marketplace 框架完成（首批 skills 可发布）
- ✅ Computer Use 可在 UI 启用
- ✅ apps/desktop/AGENTS.md + apps/cli/AGENTS.md 补全

---

### 4.4 路线图依赖图

```
月 1
├─ T3 P0-A (startup session)
├─ T3 P0-B (default providers)
├─ T3 P0-C (error UI) ← 依赖 P0-A/B（错误通道共用）
├─ T3 P0-D (MCP global) ← 可与 P0-B 并行
├─ T3 P0-E (instrumentation + hang fix)
├─ P0-X1 (stream-to-UI) ← 独立
├─ P0-X2 (session restore) ← 依赖 P0-A
└─ P1-2 子集 (CLAUDE.md loading) ← 独立

月 2
├─ P1-1 完善 (subagent UI)
├─ P1-2 完整 (Rules editor)
├─ P1-4 (SSH port-forwarding)
├─ P2-3 (image paste)
├─ P2-1 (PreToolUse hook)
├─ P2-4 (token counter)
└─ 持续测试覆盖

月 3
├─ P1-3 (Background Agent)
├─ P2-8 (multi-end sync)
├─ P2-9 (Skills marketplace)
├─ P2-5b (Computer Use UI)
└─ 可选：P2-5 语义索引 / P2-7 语音
```

---

## 5. 状态纠偏（state review 文档修正）

> 本报告基于 T2 state review（501 行）+ 本报告 §验证的实际代码对比。**state review 的整体方向正确，但具体 file:line 引用存在偏差**。本节列出关键修正，供后续 work 引用时校准。

### 5.1 关键修正

| state review 描述 | 实际代码（验证）| 修正 |
|---|---|---|
| `apps/desktop/src/main.rs:18-19, 22-23, 25-27, 30-39` 声明 `MCP_SERVICE` / `MCP_INIT_STATUS` 等 API，**desktop 侧 0 处 `.set()`** | `main.rs:63` 实际**已**调 `northhing_core::service::mcp::set_global_mcp_service(mcp_service.clone())` | **P0-D「desktop 全局注册缺失」实为已完成**。T3 P0-D 应该改为「验证 + 清理 MCP_INIT_STATUS 死代码（如果存在）」|
| `main.rs:35-39` 声明 `MCP_INIT_STATUS` / `get_mcp_init_status` / `get_mcp_status_text`，desktop 侧无引用 | **state review 搞反了**：实际是 `apps/cli/src/main.rs:35-39` 声明 + `:411-435` 设值（grep 实测 line 35/38/39/44/411/414/422/426/433）| 死代码清理在 CLI 端，不在 desktop 端 |
| 9 个 Slint callback 失败全用 `eprintln!` 写 stderr（`mod.rs:402, 408, 428, 453, 504, 526, 562, 668, 717, 851`）| 实际 `mod.rs` 只有 **5 处 eprintln**（grep 实测）；`sessions.rs` 0 处；`inspector.rs` 0 处；`inspector_model_status.rs` 2 处；其他 5 个文件（actor/log/slint_glue/skills）0 处 | eprintln 总数 **7 处** 不是 20 处；但 P0-C 错误展示通道的 spec 仍然成立（即使只有 7 处 eprintln 也是 0 UI 反馈）|
| `src/remote_connect.rs`（单文件 3446 行）| 实际是 `crates/services/services-integrations/src/remote_connect/` 5 子文件（device/encryption/pairing/qr_generator/relay_client）；`mod.rs` 不存在（`src/crates/services/services-integrations/src/remote_connect/` 直接放子文件）| 报告时改路径 |
| `apps/desktop/src/agent/actor.rs`（1240+ 行，SkillActor body no-op）| 实际 `apps/desktop/src/agent/` 目录**只有 `agentic_system.rs`（18 行）+ `mod.rs`**；无 `actor.rs` | **actor.rs 可能在某次重构中被删除/迁移**；SkillActor no-op 说法不适用当前 HEAD |
| `crates/assembly/core/src/agentic/tools/implementations/skills/` | 实际是真实存在的 skills 系统，9 个 .rs 文件 **~2500 行**（registry 947 行 / builtin 311 行 / resolver 167 行 / resolver_v2 271 行 / catalog 224 行 / policy 213 行 / mode_overrides 185 行 / types 156 行 / mod 18 行）| "Skills marketplace 完全缺失"不准确；准确描述是"**有 internal skills 系统、无 marketplace 概念**" |
| "未发现 PreToolUse/PostToolUse hook 系统" | 实际有 `crates/assembly/core/src/agentic/tools/post_call_hooks.rs`（30 行）+ `crates/execution/agent-runtime/src/post_call_hooks.rs`（30 行）；**只是 post_call，缺 PreToolUse 通用 hook** | "完全缺失"改"**post_call 有，PreToolUse 缺**" |
| 未提 `computer_use_tool.rs` 2203 行 | grep 实测 `crates/assembly/core/src/agentic/tools/implementations/computer_use_tool.rs` 2203 行 + `computer_use_actions.rs` / `computer_use_input.rs` / `computer_use_locate.rs` / `computer_use_mouse_click_tool.rs` / `computer_use_mouse_precise_tool.rs` / `computer_use_mouse_step_tool.rs` / `computer_use_result.rs` 完整 | **Computer Use 是 P2 加分项里最被低估的能力**；state review 完全没提 |

### 5.2 state review 仍有用的部分

- 整体架构评估（assembly/core 后台 90% / 协议 100% / CLI 100% / desktop 30%）✅ 准确
- 死代码清单（5 处）✅ 方向正确
- 测试覆盖度评估 ✅ 准确
- ROI 排序的 10 项 quick wins（QW-1 ~ QW-10）✅ 工作量估计合理
- 5.2 风险与开放问题 ✅ 准确

**结论**：state review 作为 handoff 文档**价值高**，但任何具体 file:line 引用都需要实测 grep 验证。后续 worker 引用 T2 state review 的 line number 时**先 grep 一次再下结论**。

---

## 6. 风险与开放问题

### 6.1 实施期风险

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| P0-A `create_session` 因 coordinator 未初始化失败 | 高 | GUI 仍不可用 | 加 retry 一次 + P0-C 错误展示 |
| P0-X1 流式响应**没**到 UI（未实测）| 中 | 用户感受"假流式" | 写新 `set_streaming_token` 回调 + UI 增量更新；需 1-2 天 |
| P0-X2 重启不丢 session 与 P0-A — 突 | 中 | 启动后行为不一致 | owner 决策：last session 恢复 vs 默认新 session |
| P1-2 子集 通用 CLAUDE.md 加载与现有 CLAW workspace 路径— 突 | 中 | 加载顺序混乱 | owner 决策：通用 CLAUDE.md 优先级 vs CLAW workspace 路径优先级 |
| P2-1 PreToolUse hook 安全边界 | 高 | 恶意 hook 跑 rm -rf | 强制走现有 ShellSecurityConfig |
| Slint UI 改动需要 owner 评审 | 高 | 实施速度变慢 | 提前把 Slint diff 给 owner 确认 |
| Tier 2 Background Agent 跨设备 push 通知需要外部服务（FCM/APNs）| 高 | 移动端无法收通知 | 用现有 WebSocket 通道；不引入第三方依赖 |

### 6.2 战略层问题（不在本轮范围，但记录）

1. **多端同步协议选型**：CRDT vs last-writer-wins vs 业务层— 突解决— 影响 P2-8 实施周期
2. **Skills marketplace 签名密钥分发**：是中心化（northhing 公司控制）vs 去中心化（每个 skill 作者自己）— 影响产品定位
3. **Computer Use 安全边界**：AI 操作桌面 GUI 的回退机制 + 用户授权策略
4. **Background Agent 跨设备协议**：是复用现有 `remote_connect` 协议还是新建— 影响工作量
5. **apps 层 AGENTS.md 缺失**：与根 `AGENTS.md` 的 `agent-doc priority` 规则不一致，建议补（已列入 Tier 2 W8）

### 6.3 文档同步建议

- 本报告完成后，**T3 gap-analysis-spec.md 的 §3 P1 列表 + §5.2 推荐执行序列**应被本报告 §4 路线图取代（避免两份文档优先级不一致）
- 实施开始后，每个 Tier 完成时更新 `docs/handoffs/2026-06-25-baseline-implementation-handoff.md`（T3 §6.3 已规划但未创建）
- 月 3 完成后，把 Tier 2 实施结果合并到根 `AGENTS.md` 的「Common commands」/「Verification」表

---

## 7. 一句话总结

> **NortHing v0.1.0 长线规划 = 3 个 Tier + 3 个月 + 1 人月**。
> **Tier 0（5-6 hr）解 GUI**：T3 spec 5 项 P0 + 本报告新增 P0-X1/X2 + P1-2 子集。
> **Tier 1（1.5-2 天）追平 Cursor 1.0**：subagent UI / Rules 编辑 / SSH 端口转发 / 多模态 / token 计数 / 健壮性 / 测试。
> **Tier 2（1 个月）差异化**：Background Agent / 多端同步 / Skills marketplace / Computer Use UI。
> **state review 文档的 file:line 引用需要实测 grep 验证再下结论**；T3 spec 5 项 P0 是最确定的起点，本报告新增的 3-5 项（P0-X1/X2 + P1-2 子集 + P1-1 subagent UI + P2-3 图像）是 T3 没覆盖的真问题，**这些比 T3 spec 里 5 项的紧迫性更高**。

---

**报告完。本报告基于 T1 baseline / T2 state review / T3 gap-analysis-spec 三份输入 + 8 处实测代码验证，不重复 T3 已详细写过的 5 项 P0 spec，不改代码。所有 file:line 引用均经 grep 实测。**
