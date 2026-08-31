# 功能实现真值核查报告（2026-08-31 文档对代码实地审计）

> 审计基线：仓库 `E:\agent-project\NortHing`，分支 `main`，HEAD `f5dc0ef`。  
> 审计标准：严格按照四段论判定——**UI 有元素 + 有事件处理器 + 处理器调到真实 API + API 到后端有实现**。四段缺一段即判定为半接线或摆设。

---

## 零、特别点名必答：`session_mock.rs` 明确结论

- **文件定位**：`src/apps/desktop/src/ui_dioxus/session_mock.rs`（305 行）。
- **性质定性**：**已被生产 UI 深度引用，是当前生产环境的消息 DTO 映射层与初始启动 Fallback 数据源，而非纯测试数据。**
- **证据与调用链**：
  1. `src/apps/desktop/src/ui_dioxus/mod.rs:50`：`mod session_mock;` 编译进生产目标。
  2. `src/apps/desktop/src/ui_dioxus/app.rs:30`：`use super::session_mock::{seed_session, MockEntry};` 引入生产 UI 主窗。
  3. `src/apps/desktop/src/ui_dioxus/app.rs:57`：`let mut entries = use_signal(|| seed_session());` 生产主窗 Signal 初始值直接取 `seed_session()` 硬编码的 5 条 mock 记录。
  4. `src/apps/desktop/src/ui_dioxus/app.rs:74`：`super::session_mock::messages_to_entries(msgs)` 是生产环境中将 kernel `MessageDto` 转为 UI 显示模型的唯一转换器。
  5. `src/apps/desktop/src/ui_dioxus/approval_card.rs:14`：引入 `MockEntry` 作为审批卡片状态更新目标。
- **副作用与风险**：当新建空会话或 `get_messages` 为空时，`if !converted.is_empty()` 判定不成立，UI 会**残留展示 5 条硬编码 mock 消息**（包括假 prompt、假见证者、假高危授权审批卡片）。

---

## 一、表1：真接线功能清单（四段完整）

| 功能项 | 需求 ID | UI 侧 (file:line) | API 桥接层 (file:line) | 后端实现 (file:line) |
|---|---|---|---|---|
| 会话列表浏览 | SE-02 | `pages_archive.rs:135-154` | `ui_dioxus/api.rs:61` | `assembly/core/.../kernel_facade/session.rs:107` |
| 会话数据持久化恢复 | SE-04 | `app.rs:65-89` (hydrate) | `ui_dioxus/api.rs:71` | `assembly/core/.../session_manager/session_manager_lifecycle.rs:25` |
| 会话删除 | SE-05 | `pages_archive.rs:535-546` | `ui_dioxus/api.rs:76` | `assembly/core/.../kernel_facade/session.rs:159` |
| 会话重命名 | SE-06 | `pages_archive.rs:483-495` | `ui_dioxus/api.rs:81` | `assembly/core/.../kernel_facade/session.rs:144` |
| 会话 Markdown 导出 | SE-07 | `pages_archive.rs:584-602` | `ui_dioxus/api.rs:71` | `pages_archive_search.rs:106` 格式化导出至 `exports/` |
| 会话全文搜索 | SE-08 | `pages_archive.rs:318-325` | `ui_dioxus/api.rs:86` | `assembly/core/.../kernel_facade/session.rs:175` (FTS5) |
| 子代理标记可见 | SE-08 / C3 | `pages_archive.rs:447-454` | `pages_archive.rs:28` | `northhing_kernel_api/src/session.rs:125` (parent_session_id) |
| 文本对话输入提交 | CH-01 | `app.rs:258-301, 531-544` | `ui_dioxus/api.rs:28` | `assembly/core/.../kernel_facade/turn.rs:19` |
| 对话流式输出展示 | CH-03 | `app.rs:112-118, 502-509` | `ui_dioxus/api_events.rs:17` | `execution/agent-stream/.../stream_processor.rs:102` |
| 中止生成 (Stop) | CH-04 | `app.rs:304-314, 549-556` | `ui_dioxus/api.rs:51` | `assembly/core/.../kernel_facade/turn.rs:51` |
| 工具执行三档确认门 | TO-02 / 原则7 | `approval_card.rs:118-146` | `ui_dioxus/api.rs:156` | `assembly/core/.../kernel_facade/tools.rs:24` |
| 技能加载与注入 | SK-01~04 | (自动匹配注入提示词) | `assembly/core/.../skills` | `assembly/core/.../skills_registry/discovery.rs:1` |
| 技能启停管理 UI | SK-05 | `pages_settings_skills.rs:50-115` | `ui_dioxus/api_settings.rs:72` | `assembly/core/.../kernel_facade/settings.rs:164` |
| 模型引擎切换 | MO-02 / MO-04 | `pages_settings.rs:327-336` | `ui_dioxus/api_settings.rs:28` | `assembly/core/.../kernel_facade/settings.rs:44` |
| Provider 连接测试 | MO-03 | `pages_settings_provider_edit.rs:121-140` | `ui_dioxus/api_provider_edit.rs:14` | `assembly/core/.../kernel_facade/settings.rs:69` |
| Provider 编辑/删除 | MO-01 | `pages_settings_provider_edit.rs:142-230` | `ui_dioxus/api_provider_edit.rs:28` | `assembly/core/.../kernel_facade/settings.rs:56,87` |
| 工作区文件树展开 | WS-03 | `panel_files.rs:58-111, 118-280` | `ui_dioxus/api_fs.rs:20` | `assembly/core/.../service/filesystem.rs:1` |
| 工作区文本文件预览 | WS-04 | `panel_files.rs:107-109, 282-350` | `ui_dioxus/api_fs.rs:57` | `assembly/core/.../service/filesystem.rs:1` |
| 项目规则自动读取 | WS-06 | (自动注入上下文) | `assembly/core/.../prompt` | `assembly/core/.../workspace/rules.rs:1` (读取 AGENTS.md) |
| 全局设置界面 | ST-01 | `pages_settings.rs:31-741` | `registry.rs:147` | 独立 OS 窗口挂载与生命周期管理 |
| 配置持久化存储 | ST-02 | `pages_settings.rs:150-207` | `app_state/settings/io.rs:1` | `GlobalConfig` (`app.json`) + `AppSettings` |
| MCP 服务启停切换 | TO-04(部分) | `pages_settings.rs:492-506` | `ui_dioxus/api_settings.rs:52` | `assembly/core/.../kernel_facade/settings.rs:136` |
| 记忆只读面板(TH-3) | TH-3 | `pages_memory.rs:1-310` | `ui_dioxus/api_memory.rs:11,16` | `assembly/core/.../kernel_facade/memory.rs:33,48` (SQLite+FTS5) |
| 记忆导出 JSONL | TH-3 | `pages_memory.rs:176-211` | `ui_dioxus/api_memory.rs:11` | 导出至 `northhing/exports/memory-<ts>.jsonl` |
| 设置页沉积记忆卡 | TH-3 / 沉积卡 | `pages_settings_cards.rs:152-170` | `ui_dioxus/api_memory.rs:11` | 真实统计 facts 与 skills 条数 |
| 设置页编年史卡 | 编年史卡 | `pages_settings_cards.rs:173-202` | `ui_dioxus/api.rs:61` | 真实提取最早与最新会话时间戳 |
| 降级即报错横幅 | 原则9 | `app.rs:493-495` / `turn_banner.rs:1` | `app.rs:188, 297` | 捕获 LLM 额度/Key 耗尽并置顶 amber 告警条 |
| 零外部遥测 | 论题 | (全仓无遥测端点) | `noop_telemetry_sink.rs:1` | 本地数据闭环不离机 |

---

## 二、表2：半接线功能清单（缺段降级）

| 功能项 | 需求 ID | 现有部分 | 缺失段（为什么算半接线） | 补齐成本 |
|---|---|---|---|---|
| 会话创建 (多会话) | SE-01 | `ensure_room_session()` 与 onboarding 能建会话 | **UI 缺少显式"新建独立会话"按钮**；当前为单 Room 绑定，用户无法在主界面一键开启新会话 | S |
| 会话切换 | SE-03 | 归档页可点"查看消息"读取详情 (`pages_archive.rs:188-206`) | **无法将归档会话设为当前 Room 活跃会话**；Room 会话在进程内被 `ROOM_SESSION_CACHE` 锁定 | S |
| 实体身份与名讳 | 论题/身份 | onboarding 能输入名字；设置页展示名讳 (`pages_settings_cards.rs:218-232`) | **名讳是借用 Provider 的 display_name 权宜映射**；"位格"诚实为空态"未配置"，无独立身份演化存储通路 | M |
| Provider 新增 | MO-01 | 设置页支持编辑、测试、删除已有 provider (`pages_settings_provider_edit.rs`) | **设置页缺少"添加新 Provider"入口**；新增只能在首次 onboarding 阶段添加 | S |
| 工作区路径重设 | WS-01 | 设置页展示当前工作区真实路径 (`pages_settings.rs:557-563`) | **"重新定位"按钮无事件处理器**（`pages_settings.rs:564-568` 无 onclick），无法更换工作区 | S |
| 最近项目切换 | WS-05 | `AppSettings.workspaces` 数组持久化记录了工作区列表 | **UI 缺少最近项目下拉列表与切换动作** | S |
| 显示模式控制 | ST-01 / Card 6 | 开关状态能持久化写入 `AppSettings` (`pages_settings.rs:584-612`) | **UI 视觉未接入**（页面标注"注：呼吸 / 双光学的视觉绑定将在后续视觉更新中生效"） | M |
| 单工具粒度禁用 | TO-04 | 支持 MCP 模块级和技能级启停 | **缺少单原生工具级别（如单独禁用 bash、只留 read）的开关** | M |
| 工具超时取消 | TO-06 | 后端具备 token 超时与取消机制 | **UI 仅有全局 Stop 按钮，无单工具超时倒计时或细粒度超时中断提示** | S |

---

## 三、表3：摆设与 Mock 清单（假数据 / 空动作 / 纯视觉占位）

| 摆设位置 | 所属模块 | 假在哪里（代码实锤证据） | 做真成本 |
|---|---|---|---|
| `ui_dioxus/pages_space.rs:47-139` | 走廊窗口 (Space) | **全页假数据**：`DOORS` 常量硬编码 7 扇假门（"诊室 03 · 此刻"、"重新定义对齐"、假标签、假产物）；输入框是静态 `<div>`；日志为静态文本 | L |
| `ui_dioxus/windows/work.rs:174-188` | 右抽屉 (Work) 路由卡 | **硬编码 mock**："架构师 · 介入中"、"search · Haiku · 待命"，纯静态文本 | M |
| `ui_dioxus/windows/work.rs:199-216` | 右抽屉 (Work) 规划卡 | **硬编码 mock**："重新定义对齐"、"├ 读取沉积记忆"、"└ 写入行动准则"，纯静态文本 | M |
| `ui_dioxus/windows/work.rs:226-231` | 右抽屉 (Work) Diff 卡 | **硬编码 mock**："alignment.md +18 -06"、"已撤销" 按钮，纯静态文本 | M |
| `ui_dioxus/windows/work.rs:233-237` | 右抽屉 (Work) 终端栏 | **硬编码 mock 文本**：`"$ northing inspect --boundary..."`，无真实 PTY | M |
| `ui_dioxus/windows/self_app.rs:119, 181-275` | 左抽屉 (Self) | **硬编码 mock 数据堆砌**：token 固定 `128_437`（点清空仅重置本地 signal）；假词条 `# 边界不是围墙`；假 `@philosophy-core`；假候选技能；假准则 | L |
| `ui_dioxus/app.rs:460-475` | 主窗编年史指示条 | **双击演示 mock**：双击仅在 5 种硬编码十六进制色值间循环切换，未接后端状态 | S |
| `ui_dioxus/app.rs:518-530` | 主窗附件按钮 (Attach) | **空按钮**：`<button class="attach">` 没有任何 `onclick` 事件处理器，无法选文件/图片 | S |
| `ui_dioxus/app.rs:782-791` | 主窗消息子项 Chip | **空按钮**：`ToolLog` 和 `ArtifactChip` 渲染为无 `onclick` 的空 `<button>`，无法展开查看工具入参/结果 | M |
| `ui_dioxus/pages_archive.rs:250-267` | 归档页侧边栏筛选 | **纯视觉占位**："已归档会话"、"全部"、"本周"、"更早" 均无 `onclick` 事件处理器 | S |
| `ui_dioxus/pages_settings.rs:383-394` | 设置页 Card 2 (上下文) | **纯视觉占位**：静态显示"全局作用域"和假能量条，无任何数据源和事件绑定 | S |
| `ui_dioxus/pages_settings.rs:347-368, 448-462, 515-536` | 设置页空态 Fallback | **TODO(data) mock**：当列表为空时回退展示硬编码的 Claude/Gemini/GPT-4o、Anthropic/Google、Filesystem/Terminal 假卡片 | S |

---

## 四、后端完整但 UI 零接入清单（能力浪费盘点）

| 能力项 | 后端核心入口 (file:line) | Facade / Trait 暴露情况 | UI 接入情况 | 浪费程度分析 |
|---|---|---|---|---|
| **Episode 日志记录** | `src/crates/assembly/core/src/service/agent_memory/mod.rs:1` | `kernel_facade/memory.rs:9` (`list_episodes`) | **零接入** (`api.rs` 未封装，UI 无法查看每轮历史执行轨迹与失败修复日志) | 高（核心自省数据沉睡） |
| **记忆蒸馏 (Distillation)** | `src/crates/assembly/core/src/service/agent_memory/distiller.rs:1` | 仅内部后台调度调用 | **零接入** (无蒸馏触发、进度或手动整理界面) | 中（后端自动跑，但用户不可见） |
| **Dream 演化与自评审** | `src/crates/assembly/core/src/service/agent_memory/dream.rs:1` | `memory_db/dream.rs:1` | **零接入** (论题核心"身份自主演化"在 UI 侧完全黑盒) | 高（论题核心价值未透出） |
| **Cron 定时引擎** | `src/crates/assembly/core/src/service/cron/service_impl.rs:1` | `agentic/tools/implementations/cron_tool.rs:1` | **零接入** (无法在 UI 创建、查看、管理半被动定时任务) | 高（TH-6 半被动能力完全无入口） |
| **会话/技能快照 (Snapshot)** | `src/crates/assembly/core/src/service/snapshot/snapshot_system.rs:1` | `agentic/tools/product_runtime/snapshot.rs:1` | **零接入** (快照创建、回滚完全无 UI 按钮) | 中（仅能由 agent 工具调用） |
| **DeepReview 深度评审** | `src/crates/execution/agent-runtime/src/deep_review/reviewer_admission_queue.rs:1` | `agentic/deep_review_policy.rs:1` | **零接入** (多评审员排队、准入、打分机制在 UI 零展示) | 中（作为内部执行机制运行） |
| **ACP 外部控制协议** | `src/crates/interfaces/acp/src/lib.rs:1` | `src/apps/server/src/rpc_dispatcher.rs:1` | **零接入** (桌面端无 ACP 连接管理或状态面板) | 低（设计即面向 Headless/远程） |
| **LSP 语言服务协议** | `src/crates/assembly/core/src/service/lsp/manager.rs:1` | `service/lsp/mod.rs:28` | **零接入** (后端实现进程生命周期/跳转/补全/诊断，UI 零展示) | 高（庞大重型设施沉睡） |
| **Terminal 终端服务** | `src/crates/services/terminal/src/lib.rs:1` | `agentic/tools/implementations/terminal_control_tool.rs:1` | **零接入** (UI `work.rs` 和 `space.rs` 仅为静态假文本，无真 PTY 交互) | 高（完整 terminal 库未接桌面） |
| **Git 完整服务** | `src/crates/assembly/core/src/service/git/git_service.rs:1` | `agentic/tools/implementations/git_tool/` | **零接入** (UI `work.rs` 仅有假 diff 文本，无真实分支/提交/差异查看) | 高（已有完整 git 工具但无 UI） |
| **工作区全局代码检索 (flashgrep)** | `src/crates/services/services-integrations/src/workspace_search/service_search.rs:1` | `execution/tool-execution/src/search/grep_search.rs:1` | **零接入** (UI 仅有会话和记忆搜索，无工作区代码全文检索面板) | 中（Agent 自用工具，缺用户入口） |

---

## 五、各 Surface 交付与可用状态核实

| Surface | 路径 | 是否存在 | 构建与 Shipping 状态 | 最近改动（时间 + Commit + 简述） |
|---|---|---|---|---|
| **Desktop (Dioxus)** | `src/apps/desktop` | ✅ 存在 | **唯一主力 Shipping 壳**（Slint 已删） | `2026-08-31 2b3ecfb` feat(desktop): wire fulltext session search into archive page (W12-2) |
| **CLI** | `src/apps/cli` | ✅ 存在 | **可独立运行维护中** | `2026-08-29 69fb851` fix(cli): chat model edit inherits stored keyring key (W11-3) |
| **Server** | `src/apps/server` | ✅ 存在 | **Frozen-Experimental**（已冻结） | `2026-08-21 1d1d4ff` security: enforce WS Origin check, pin ACP client versions (T1-10) |
| **Installer** | `northing-installer` | ✅ 存在 | **Shipping 交付**（Windows NSIS 安装器） | `2026-08-22 c8868fe` refactor(cleanup): sweep one-off scripts and fix installer doc paths |
| **ACP Interface** | `src/crates/interfaces/acp` | ✅ 存在 | **库接口层** | `2026-08-21 e7af0bf` refactor(events,dispatch): converge dead event pipeline (T2-9-B2) |
| **E2E Tests** | `tests/e2e` | ✅ 存在 | **测试集** | `2026-08-19 a930c93` chore: MiniApp deletion M1 - entry excision |
| **Web UI** | `src/web-ui` | ❌ **代码缺失** | 仅存 2 个 i18n 预设空文件和 .gitkeep，无任何 Web 页面实现（AGENTS.md 标 missing） | `2026-08-22 c8868fe` |

---

## 六、总结结论

### 1. 现状统计

- **真接线项**：**27 项**（覆盖会话归档管理、流式对话、三档确认门、技能启停、模型切换与测试、文件树与预览、记忆只读浏览与检索导出、降级报错横幅等核心功能）。
- **半接线项**：**9 项**（多会话一键新建/切换缺失、身份名讳权宜绑定、设置页缺 Provider 新增入口、工作区重选按钮无动作、显示模式无视觉响应、单工具粒度禁用等）。
- **摆设 / Mock 项**：**12 处**（走廊 Space 全页假门、右抽屉 Work 路由/规划/Diff/终端三卡假文本、左抽屉 Self 假数据堆、主窗附件与 Chip 空按钮、设置页 Card 2 假能量条等）。

### 2. 这个产品现在"能用"到什么程度？

> **一句话定性**：  
> **现在是一个"具备单房间真实 Agent 对话与工具调用、三档授权确认门完备、工作区文件可树状预览、会话与记忆支持检索导出的单兵工作台"，但"多会话切换、多窗口联动（走廊/左右抽屉）仍充斥大量概念设计阶段的静态 Mock 摆设，且聊天输入仅支持单行纯文本（无 Markdown 高亮/无附件图片）"。**

### 3. 离"个人 AI 同事"产品目标还差哪几个硬骨头？

1. **聊天基础体验补齐（P0 骨头）**：聊天渲染必须支持真正的 Markdown / 代码高亮 / 工具折叠详情展示（消灭纯文本 `MockEntry` 机制）；输入框改为多行并支持真正的文件/图片拖入。
2. **多房间/会话切换与走廊（Space）做真（P1 骨头）**：废除 `pages_space.rs` 中写死的 7 扇假门，把后端的真实活跃/归档/沉浸 session 真正变成可以在走廊里查看、点击切换进入的房间；解绑单 Room 锁定的内存缓存。
3. **左右抽屉（Self / Work）彻底清洗做真（P1 骨头）**：彻底清除 `self_app.rs` 和 `work.rs` 里的硬编码假数据（假的路由、规划、Diff、Token 消耗），将其接入真实的 Agent 执行状态、Git 真实变更和真实的后端 Token 统计。
4. **后端重型基础设施的透出与激活（P2 骨头）**：
   - 将后端已实现的 **Cron 定时引擎** 暴露给 UI，达成 TH-6 要求的"半被动交互"；
   - 将 **Dream / 自评审** 协议层透出为真正的成长轨迹与身份演化面板；
   - 决定是否将 **Terminal / LSP / Git** 等深水区设施做成真正的交互面板，或明确裁撤其多余代码。
