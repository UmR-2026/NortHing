# 实现现状盘点 — 需求 vs 现状校准

> **仓库**：`E:\agent-project\NortHing` @ main HEAD  
> **日期**：2026-08-29  
> **范围**：Dioxus consult-room 桌面壳（唯一壳，Slint 已于 2026-08-28 物理删除）  
> **方法**：只读侦察（codegraph + rg + git）；所有结论带 file:line 证据

---

## 1. 会话系统

| 功能 | 状态 | 证据 |
|---|---|---|
| 创建 | ✅ 已接线 | `api.rs:100-133` `ensure_room_session()` → `kernel_facade().create_session()` |
| 列表 | ✅ 已接线 | `api.rs:57-64` `list_sessions()` / `list_sessions_all_workspaces()` |
| 切换 | ⚠️ 部分 | `api.rs:97-133` 有 `ROOM_SESSION_CACHE`（进程生命周期缓存），注释 `ponytail: restart required to switch`；无显式"切换会话"UI |
| 持久化 | ✅ 后端 | 由 ConversationCoordinator + SessionManager 自动持久化 |
| 删除 | ❌ 缺失 | 全仓 `ui_dioxus/` 零 `delete_session` UI 调用；`api.rs` 无 `delete_session` 包装函数 |
| 重命名 | ❌ 缺失 | 全仓零 `rename` 会话 UI |
| 导出 | ❌ 缺失 | 全仓零 `export` 会话功能 |
| 搜索 | ❌ 缺失 | 全仓零 `search` 会话 UI |

**结论**：创建/列表/持久化 有后端接线，切换有缓存但无 UI 入口；删除/重命名/导出/搜索 全链路缺失。

---

## 2. 对话系统

| 功能 | 状态 | 证据 |
|---|---|---|
| 文本输入 | ✅ 已接线 | `app.rs:241-283` `submit_turn()` → `api::submit_turn` |
| Markdown 渲染 | ❌ 缺失 | 消息渲染使用 `MockEntry`（`session_mock.rs`），纯文本展示，无 Markdown 解析/渲染 |
| 流式输出 | ✅ 已接线 | `app.rs:100-200` 监听 `KernelEventDto::TextChunk`，逐字追加到 `assistant_draft` |
| 停止生成 | ✅ 已接线 | `app.rs:285-295` `stop_action` → `api::stop_turn()` |
| 重新生成 | ❌ 缺失 | 全仓零 `regenerate` UI |
| 编辑消息 | ❌ 缺失 | 全仓零 `edit.*message` UI |
| 文件引用 | ❌ 缺失 | 输入区有 `⌗ 工作文件夹` 按钮但无 handler（`pages_space.rs:453`） |
| 图片输入 | ❌ 缺失 | 全仓零 `image.*input` 或 `multimodal.*input` UI |

**结论**：文本输入/流式/停止 有完整后端接线；Markdown/富文本/图片/文件引用 均为前端缺失，非后端问题。

---

## 3. 工具与确认门

| 功能 | 状态 | 证据 |
|---|---|---|
| 工具调用展示 | ⚠️ 部分 | `app.rs:113-131` 收到 `ToolCall(AwaitingConfirmation)` 时渲染 `MockEntry::Approval`（含 call_id/head/main/risk）；工具执行结果以 `MockEntry::Entity` + `ToolLog` 子条目展示（`session_mock.rs:130-135`）— 仅显示工具名，不展示参数/结果详情 |
| 确认交互（允许/拒绝） | ✅ 已接线 | `app.rs` 未直接渲染确认按钮，但 `api.rs:136-140` `respond_to_tool_confirmation()` 已接线；确认卡片有 `resolved` 状态位（`session_mock.rs:32-39`）但渲染为静态 mock |
| 超时取消 | ✅ 已接线 | `api.rs:52-54` `stop_turn()` → `kernel_facade().stop_turn()` |

**结论**：后端确认协议完整（`ToolConfirmationPlan` / `ToolConfirmationOutcome` / `respond_to_tool_confirmation`）；UI 层面有 Approval 卡片展示和取消发送功能，但无显式"允许/拒绝"按钮交互（卡片为只读展示）。

---

## 4. 技能系统

| 功能 | 状态 | 证据 |
|---|---|---|
| 技能加载 | ✅ 后端完整 | `skill registry`（`.agents/reference/skills/08-registry-full.rs`）支持 project/user/builtin 三级发现、SKILL.md 解析 |
| 技能匹配 | ✅ 后端完整 | `resolver.rs` + `policy.rs` 按 mode（agentic/coding/debug/plan 等）+ project override 过滤 |
| 技能注入 | ✅ 后端完整 | 匹配后的技能自动注入 prompt |
| 管理 UI | ❌ 缺失 | 内窗口有 "沉积skill" 卡（`windows.rs:291-303`）但仅展示 3 个硬编码候选（`INNER_SKILL_CAND_1/2/3`），无列表/开关/创建功能 |
| 创建 | ❌ 缺失 | 全仓零 `create_skill` UI |

**结论**：后端技能体系（加载/匹配/注入/策略）极为完整，但 Dioxus 前端无管理入口——内窗口 skill 卡是展示 mock。

---

## 5. 模型/provider

| 功能 | 状态 | 证据 |
|---|---|---|
| 配置 CRUD | ⚠️ 部分 | 列表+默认选择 ✅（`pages_settings.rs:364-429`）；新增（依赖 onboarding）✅；编辑弹窗 ✅（`pages_settings_provider_edit.rs` W7-2 今天交付）；删除 ❌（无 UI，仅 `api.rs` 测试引用 `delete_model_config`） |
| 默认模型 | ✅ 已接线 | `pages_settings.rs:386-394` → `api::set_default_provider()` |
| 连接测试 | ✅ 已接线 | `pages_onboarding.rs:556` `run_test_provider()` → `api::test_provider_config()` |
| 模型参数（temperature 等） | ❌ 缺失 | 设置页无 temperature/max_tokens 调节 UI；`AIModelConfigDto` 含 `temperature: Option<f64>` 和 `max_tokens: Option<u32>` 字段，但 Dioxus 侧无表单 |

**结论**：provider 的增/改/测试/默认 基本就绪（今天 F7 补了编辑弹窗）；temperature/max_tokens 参数调节 UI 缺失。

---

## 6. 工作区

| 功能 | 状态 | 证据 |
|---|---|---|
| 打开 | ⚠️ 部分 | `AppSettings.workspaces` + `current_workspace` 存储路径（`app_state/settings/mod.rs:58-72`），设置页 Card 5 显示路径（`pages_settings.rs:610-624`）；重新定位按钮有 UI 但无 handler |
| 类型检测 | ❌ 缺失 | 无 Dioxus UI 展示 workspace 类型 |
| 文件树 | ❌ 缺失 | 全仓 `ui_dioxus/` 零 `file_tree` / `read_dir` UI |
| 文件预览 | ❌ 缺失 | 同上 |
| 最近项目 | ❌ 缺失 | 无最近项目列表 UI |
| 项目规则读取 | ✅ 后端 | `bootstrap_impl.rs:105-128` 初始化时读取 `IDENTITY.md` / `SOUL.md` / `USER.md` / `BOOTSTRAP.md`；`PathManager::project_rules_dir()`（`project_paths.rs:62-64`）暴露 `.northhing/rules/` 路径 |

**结论**：后端 workspace 服务完整（本地+SSH 文件系统 abstraction），但 Dioxus UI 停留在路径显示层，无文件浏览/类型检测/规则展示。

---

## 7. 设置系统

| 功能 | 状态 | 证据 |
|---|---|---|
| 分类导航 | ✅ 实现 | 6 张可折叠卡片（`pages_settings.rs:73-85` folding states） |
| 持久化 | ✅ 已接线 | `load_app_settings()` / `update_app_settings()`（`app_state/settings/io.rs`），JSON 序列化到 `app.json` |
| 导入/导出 | ❌ 缺失 | 全仓零 `import` / `export` 设置 UI |
| 重置 | ❌ 缺失 | 全仓零 `reset` 设置 UI |
| Card 1 引擎 | ✅ 已接线 | 模型列表 + 设为默认 |
| Card 2 上下文 | ❌ Mock | 硬编码「全局作用域」+ seg bar，无后端接线（`pages_settings.rs:431-454`） |
| Card 3 接入点 | ⚠️ 部分 | 列表+默认+编辑弹窗 ✅；删除 ❌ |
| Card 4 MCP | ✅ 已接线 | toggle 启用/禁用 → `set_mcp_enabled()` |
| Card 5 工作区 | ⚠️ 只读 | 路径显示 + 重新定位按钮（无 handler） |
| Card 6 显示 | ❌ Mock | `display_breath` / `display_dual_optics` 是 mock 信号，注释 `TODO(data): no AppSettings field yet`（`pages_settings.rs:639-648`） |

---

## 8. 记忆系统

| 功能 | 状态 | 证据 |
|---|---|---|
| 后端存储 | ✅ 完整 | `MemoryDb`（SQLite + WAL + FTS5），`facts.jsonl` 迁移，distiller，dream sweep，judge-mom |
| 记忆面板（浏览/搜索） | ❌ 缺失 | 全仓 `ui_dioxus/` 零 `memory_panel` / `memory_ui` / `facts_browser` |
| 记忆导出 JSONL | ❌ 缺失 | 同上 |
| 编辑/删除 | N/A | PRD 明确无编辑删除，后端也不支持（`delete_fact` 只标记 superseded） |
| KernelMemoryApi::list_episodes | ✅ 后端 | `kernel_facade/memory.rs:9-56` 已实现，有测试；**但 Dioxus `api.rs` 未暴露此函数** |

**结论**：服务层极其完整（facts + FTS + dream sweep + distiller + episodes），但 Dioxus 端零 UI——记忆面板不存在。

---

## 9. 身份系统

| 功能 | 状态 | 证据 |
|---|---|---|
| 创建身份（四字段+五色板） | ✅ 已接线 | `pages_onboarding.rs:440-498`：用户名讳 + 实体名称 + 关系称谓 + 五色板选择（`SWATCHES`）；色板选中后显示预览（`pages_onboarding.rs:809` `preview-identity`） |
| 身份展示 | ⚠️ 部分 | 设置页左列 Card 3（`pages_settings.rs:320-340`）：「名讳」= 硬编码 `NortHing`，「位格」= 硬编码 `观测者 / 见证中心`；**不是从 IDENTITY.md 或 backend 读取的真实数据** |
| 演化审计 | ❌ 缺失 | 无身份变更历史/审计日志 UI |

**结论**：Onboarding 有完整的身份创建 UI（四字段+五色板），但身份展示是硬编码字符串，且无演化审计。后端有 `IDENTITY.md` / `SOUL.md` / `USER.md` persona 文件系统（`bootstrap_impl.rs`），但 Dioxus 不读取展示。

---

## 10. 成长/演化

| 功能 | 状态 | 证据 |
|---|---|---|
| Growth session | ❌ 缺失 | 全仓零 `growth_session` UI |
| Dream sweep | ✅ 后端 | `memory_db/dream.rs` + `auto_memory.rs` distiller 已实现 |
| 自评审（judge_gate） | ✅ 后端协议 | `judge_mom` 表 + `fact_reviews` 表，`memory_db.rs:605-625` 记录 review；protocol 层保留 |
| UI 接入 | ❌ 缺失 | 全仓 `ui_dioxus/` 零 dream/growth/judge 相关 UI |

**结论**：judge_gate 协议层保留，dream sweep 自动运行，但无任何用户可见界面。

---

## 11. 半被动交互

| 功能 | 状态 | 证据 |
|---|---|---|
| Cron 定时任务（后端） | ✅ 完整 | `CronJob` / `CronJobsFile` / `CronSchedule`（At/Every/Cron）+ store + service + runtime state machine（`scheduled_job.rs`）；enqueue/coalesce/retry/recover 全链路 |
| Cron UI | ❌ 缺失 | 全仓 `ui_dioxus/` 零 `cron` / `scheduled_job` / `timer` 相关 UI |

**结论**：核心定时任务引擎完整（含 one-shot/cron/recurring + 远程 workspace 支持），但 Dioxus 端零管理入口。

---

## 12. PCS 插件连接系统

| 功能 | 状态 | 证据 |
|---|---|---|
| 注册 | ❌ 缺失 | 全仓零 `PCS` / `pcs_plugin` / `plugin_registration` UI |
| 面板 | ❌ 缺失 | 同上 |
| 权限批准 | ❌ 缺失 | 同上 |

**备注**：`registry.rs` 中的 "plugin" 指 Dioxus 模块窗口插件（archive/space/settings/onboarding），非 PCS 插件连接系统。ACP 客户端（Opencode/ClaudeCode/Codex）在 CLI 侧有实现（`acp_cli.rs`），但不在 Dioxus UI 内。

**结论**：PCS 插件连接系统在 Dioxus 端完全不存在。

---

## 13. 降级即报错

| 功能 | 状态 | 证据 |
|---|---|---|
| Key 耗尽/quota 错误的分类 | ✅ 后端 | `errors.rs:155-163` `error_category()` 区分 AIClient/timeout/unknown；`AiErrorDetail` 结构化错误 |
| 用户可见路径 | ❌ 缺失 | Dioxus room 有 `send_error: Signal<Option<String>>`（`app.rs:52`），仅用于 submit 错误展示；无 quota 耗尽/速率限制的专项 banner/通知/降级 UI |

**结论**：后端有错误分类基础设施，Dioxus 端只有最基本的 submit 错误展示——无 quota 耗尽可见路径。

---

## 14. 遥测

| 功能 | 状态 | 证据 |
|---|---|---|
| 内部 telemetry | ✅ 仅内部 | `agent-dispatch/src/telemetry.rs`：`TelemetrySink` trait + `NoopTelemetrySink` 默认实现；actor tick 事件发往 sink |
| 外部遥测端点 | ✅ 零离机 | 全仓零 PostHog/Amplitude/Mixpanel/Sentry/DataDog/NewRelic 引用；`i18n:audit` 和 `rot-budget` 监控走本地 CI；HTTP client 仅用于 AI provider 调用 + review_platform (GitHub/GitLab/GitCode) + CDP 浏览器控制 + Exa 搜索 |

**结论**：零外部遥测发射点。内部 telemetry 为 actor 调度可观测性，不走网络。

---

## 15. Onboarding 六步验收环

| 步骤 | 状态 | 证据 |
|---|---|---|
| ① 安装 | ⚠️ 部分 | `northing-installer`（React + Tauri）有安装器（`App.tsx` 6 步流程），但不在 Dioxus 壳内 |
| ② 创建身份 | ✅ | `pages_onboarding.rs` 四字段+五色板+provider 配置+连接测试 |
| ③ 对话 | ✅ | Room 有文本输入+流式输出+停止 |
| ④ 记住 | ⚠️ 半自动 | 后端 distiller 自动蒸馏 facts（`distiller.rs`），但无 UI 让用户确认/查看 |
| ⑤ 隔天记得 | ⚠️ 半自动 | Facts 自动注入 prompt（`select_facts_for_prompt`），但无 "隔天/记忆回顾" UI |
| ⑥ PCS 装插件 | ❌ 缺失 | PCS 插件系统在 Dioxus 端不存在 |

**环的缺口**：步骤 ④⑤ 的后端自动运行，但用户无可见确认/回顾界面；步骤 ⑥ 完全缺失。

---

## 汇总表

| # | 领域 | 状态 | 一句话 |
|---|---|---|---|
| 1 | 会话系统 | ⚠️ 部分 | 创建/列表/持久化 ✅；删除/重命名/导出/搜索 ❌ |
| 2 | 对话系统 | ⚠️ 部分 | 文本/流式/停止 ✅；Markdown/图片/文件引用/重新生成/编辑 ❌ |
| 3 | 工具与确认门 | ⚠️ 部分 | 后端确认协议完整；UI 有 Approval 展示但无显式允许/拒绝按钮 |
| 4 | 技能系统 | ⚠️ 部分 | 后端加载/匹配/注入完整；前端无管理 UI |
| 5 | 模型/provider | ⚠️ 部分 | 列表/默认/测试/编辑弹窗 ✅；temperature 参数调节 ❌ |
| 6 | 工作区 | ⚠️ 部分 | 后端文件系统 abstraction 完整；UI 仅路径显示 |
| 7 | 设置系统 | ⚠️ 部分 | Card 1(引擎)/Card 3(接入点)/Card 4(MCP) ✅；Card 2/6 mock，Card 5 只读 |
| 8 | 记忆系统 | ❌ 缺失 | 后端完整；Dioxus 零 UI |
| 9 | 身份系统 | ⚠️ 部分 | Onboarding 创建完整；设置页展示硬编码 |
| 10 | 成长/演化 | ❌ 缺失 | 后端 dream sweep + judge_gate 完整；UI 零接入 |
| 11 | 半被动交互 | ❌ 缺失 | 后端 cron 引擎完整；UI 零接入 |
| 12 | PCS 插件 | ❌ 缺失 | 完全不存在 |
| 13 | 降级即报错 | ❌ 缺失 | 后端 error_category 有；UI 无 quota 可见路径 |
| 14 | 遥测 | ✅ 零离机 | 内部 telemetry 仅 actor 调度；零外部端点 |
| 15 | Onboarding 环 | ⚠️ 部分 | 安装→创建→对话 ✅；记忆回顾/PCS 插件 ❌ |

---

## 3 个最令人意外的发现

1. **记忆系统是"深海油田"——后端极完整但 UI 完全断连**。`MemoryDb`（SQLite+FTS5）、distiller（LLM 蒸馏 + keyword fallback）、dream sweep（24h gate + judge-mom review）、episode store（JSONL rotation）、`KernelMemoryApi::list_episodes` 均已实现且有测试——但 Dioxus `api.rs` 未暴露任何 memory API，全仓 UI 零记忆面板。这是"最大面积已实现但零可见"的领域。

2. **Slint 删除后残留的语义空洞**：`resolve_edit_api_key` + `resolve_effective_api_key` 在 `sync.rs` 是 dead code（`#[allow(dead_code)]`），`callbacks_settings/provider.rs` 的编辑/删除 flow 已随 Slint 物理删除，但今天 F7 的编辑弹窗（`pages_settings_provider_edit.rs`）重建了 UI 层——意味着 **Slint 的完整 provider 编辑能力（含 keyring 集成）在 Dioxus 侧是重新发明的，不是继承的**。

3. **技能系统的"后端-前端鸿沟"是最大规模的能力浪费**：技能体系（catalog 60+ built-in、policy 10 modes、resolver、registry、project override、builtin installer lock）是仓库中最复杂的后端子系统之一，但 Dioxus 内窗口的 "沉积skill" 卡片只渲染 3 个硬编码候选行（`INNER_SKILL_CAND_1/2/3`）——相当于花大力气建了完整机场，UI 只给了一个公交站牌。
