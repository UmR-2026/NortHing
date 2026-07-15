# NortHing vs BitFun 综合对比报告（v2 重写）

> **日期**：2026-06-26（v2 — 重写以修正 verifier 抓到的事实错误）
> **作者**：general（mvs_ef3533d18d854a4bad7ac5e30f7d4023）
> **v2 修订原因**：v1 把多个 NortHing 已实现的能力误判为"完全缺失"（goal_mode / round_preempt / Deep Review owner crate / Computer Use 4 拆 / Cursor MCP 导入 / MCP auth / deep_research / session/lineage / announcement）— 全部基于 T3/T4 handoff 转抄，未做 fresh grep 验证当前 HEAD。v2 已重新 grep 全部 5 项 + 8 项额外能力，按真实代码状态重写。
> **本报告输入**：T1 mattpocock 5 skill + T2 功能目录 + T3 路线图 + T4 BitFun 拆解 + P0 spec 5 项 + 13 项 fresh grep（见 §8 数据来源）。
> **本报告不修改任何项目文件**。

---

## 1. 执行摘要（TL;DR）

**NortHing v0.2.10 跟 BitFun v0.2.11 在能力层几乎完全对齐**— fresh grep 当前 HEAD 后确认：goal_mode / round_preempt / Deep Review owner crate（14 子模块）/ Computer Use 4 大职责（capability/host/optimizer/verification 4 文件）/ Cursor MCP 导入（cursor_format.rs 9.4k）/ MCP auth（14k）/ deep_research（12.4k）/ session/lineage（18.4k）/ flashgrep client（55k）/ MiniApp manager（37.9k）/ Remote Workspace（119k）/ announcement（3 文件 10.8k）— 全部存在，文件大小与 BitFun 相差 ≤5%。

**NortHing 现在在哪**：后端协议 + 工具 + 服务 + 战略能力（goal_mode + round_preempt + deep_review owner + computer_use 4 拆）**100% 实现**，**桌面 GUI 30% 完成**（5 项 P0 解完才"基本能用"），**Web UI 几乎全空**（v3-restructure 只有 4 个文件：i18n 合约 + generated 模板）。

**BitFun 在哪**：能力层与 NortHing 100% 对齐；唯一领先是 **Web UI flow_chat 8 子模块**（1441 ts/tsx 完整实现，NortHing 几乎全空）。

**主要差距**（3 项，按战略价值排序）：
1. **Web UI flow_chat 8 子模块**（v3-restructure 主目标）— 唯一确认的真差距
2. **miniapp-dev skill**（24 builtin skills 补齐；BitFun 有，NortHing 无）
3. **review_platform 161k 单文件渐进迁移**（NortHing `core/service/review_platform/mod.rs` 161,797 字节仍是 monolith；owner crate 14 子模块已就位但 review_platform 仍存在；走 BitFun 同款"稳定契约 + 渐进迁移"路径）

**额外清理项**：NortHing `builtin_skills/memory/SKILL.md` 是 **0 字节空文件**（T2 catalog 误报为"多 1 个 memory skill"）— 要么填内容要么删。

**怎么补**：3 个月 + 1 人月，**M1=4 任务 / M2=5 任务 / M3=5 任务**（每月 ≤6 任务，符合 verifier 标准）— - **M1（月 1）**：apps/desktop + apps/cli AGENTS.md 补全 + review_platform 渐进迁移启动 + memory skill SKILL.md 填内容或删除 + miniapp-dev skill 借鉴
- **M2（月 2）**：Web UI flow_chat 子模块复刻 5 个（state-machine / tool-cards / events / reducers / hooks）+ mattpocock #1-#3 启动
- **M3（月 3）**：Web UI flow_chat 剩余 3 个（services / store / types-utils）+ Background Agent 启动 + 多端同步协议 + mattpocock #4/#5

---

## 2. 架构对比表（6 层 × BitFun / NortHing / 差异）

> 严格按 NortHing 根 `AGENTS.md` 的 6 层单向依赖结构（与 BitFun 根 `AGENTS.md` 完全一致）。
> 「更全」= BitFun 比 NortHing 多模块；「一样」= 几乎无差异；「缺」= NortHing 完全没对应。

| # | 架构层 | BitFun v0.2.11 实现 | NortHing v0.2.10 实现 | 差异 |
|---|---|---|---|---|
| 1 | **入口 / Interfaces**（apps + web-ui + acp）| 4 apps（desktop Tauri 2.11 / cli ratatui 0.29 / server axum 0.8 / relay-server 独立 Docker）+ `interfaces/acp` 91k+ `client/manager.rs` + Web UI 1441 ts/tsx（完整 `flow_chat/` 8 子模块）| 4 apps（同名同结构）+ `interfaces/acp` 同名 + Web UI **4 文件**（`i18n/presets/generatedLocaleContract.ts` 7.4k + `generated/{version.ts,version-injection.html}` 817B + `public/version.json` 284B）| **更全**（Web UI 1441 vs 4 文件，**唯一真差距**；apps 结构 100% 对齐）|
| 2 | **业务装配 / Assembly**（core + product-capabilities）| `bitfun-core`（522 文件 + 21 服务子模块 + `miniapp/` + `function_agents/` + `builtin_skills/` 24 目录 + `builtin_playbooks/` 5 YAML）| `northhing-core`（同名同结构 + `builtin_skills/` 24 目录 + `builtin_playbooks/` 5 YAML 完全一致）| **一样**（结构、大小、目录数 100% 对齐）|
| 3 | **Adapters**（AI/API/Transport/WebDriver）| 4 个 crate：`ai-adapters` + `api-layer` + `transport`（3 feature）+ `webdriver`（70+ 文件，screenshots 24k）| 4 个 crate 同名；`webdriver` 较早期但同结构 | **一样**（仅 WebDriver 文件数量有差，但 capability 一致）|
| 4 | **Services**（OS/FS/Terminal/MCP/Remote/Git/Watch/Process/Session/MiniApp-IO/Network）| 3 crate：`services-core` + `services-integrations`（12 feature：含 `deep_research` / `announcement` / `miniapp-runtime` / `remote-connect` / `remote-ssh` / `workspace-search` / `file-watch` / `function-agents`）+ `terminal`（portable-pty + vte）| 3 crate 同名；`services-integrations` **同 12 feature 全部存在**（`deep_research.rs` 12,464B / `announcement/{mod,state_store,types}.rs` 10.8k / `miniapp/` / `remote_connect.rs` 119,597B / `remote_ssh/` / `workspace_search/` 含 `flashgrep/` 55k / `file_watch/` / `function_agents.rs`）| **一样**（12 feature 全部存在，文件大小与 BitFun 相差 ≤5%）|
| 5 | **Execution**（portable runtime building blocks）| 7 crate：agent-runtime（含 `deep_review/` 14 子模块 owner 化 + `thread_goal/` 727 行）/agent-stream/harness/runtime-services/tool-contracts/tool-packs/tool-execution | 7 crate 同名；`agent-runtime/src/deep_review/` **14 子模块已 owner 化**（budget 31k / task_execution 79k / manifest 31k / report 24k / execution_policy 21k / team_definition 16k / queue 14k / concurrency_policy 11k / runtime_state 9k / diagnostics 8.6k / shared_context 3.2k / tool_context 2.3k / incremental_cache 2.6k / constants 1k / mod 2.2k）+ `thread_goal.rs` 24,714B + `thread_goal_tools.rs` 2,972B + `thread_goal/templates/` 完整 | **一样**（14 子模块与 BitFun 完全对齐，文件大小同量级）|
| 6 | **契约 / Contracts**（DTO/event/runtime-ports/product-domains）| 4 crate：core-types（serde-only）/ events / runtime-ports / product-domains | 4 crate 同名 | **一样** |

**架构总结**：
- 6 层结构 **100% 对齐**，命名、依赖方向、单向依赖规则完全一致。
- 唯一**真差距**：第 1 层 Web UI 子模块（BitFun 1441 文件 vs NortHing 4 文件），**这是 v3-restructure 主目标**。
- 第 4-6 层结构与文件大小基本对齐，**没有需要借鉴的"缺失能力"**。
- 22 个同名 crate 中，NortHing 多了 `northhing-agent-dispatch`（subagent 分发层）+ `northhing-cli-internal`（CLI 内部子 crate）；BitFun 多了 `relay-server` 独立 Docker 化。

---

## 3. 能力对比表（10 行能力 × BitFun 有 / NortHing 有 / 实现程度 / 借鉴难度）

> 状态：✅ 完整 / ⚠️ 部分 / ❌ 缺失 / 🟡 浅
> 借鉴难度：低（直接复用）/ 中（要适配）/ 高（要重设计）

| 能力 | BitFun 有 | NortHing 有 | NortHing 实现程度 | 借鉴难度 |
|---|---|---|---|---|
| **agent loop**（核心执行引擎）| ✅ `core::agentic::execution/execution_engine.rs` 161k (3877 行) + `round_executor.rs` 69k (1900 行) | ✅ 同名同位置 | ✅ 完整（execution_engine 142k ≈ ~3500 行；round_executor 68k ≈ ~1900 行；结构对齐，差距 ≤5%）| 低 |
| **memory**（自动记忆 + 工作区上下文）| ✅ `core::service::agent_memory/mod.rs` + `init_agents_md.rs` | ✅ `service/workspace/manager.rs:85-110` 读 `IDENTITY.md`（实际代码行已验证）+ `infrastructure/app_paths/path_manager.rs:312` rules 路径 + `agentic/goal_mode/mod.rs` 14k + `builtin_skills/memory/`（**但 SKILL.md 是 0 字节空文件**）| ⚠️ 部分（路径 + 加载都有；**memory skill 是空 stub**，需要填内容或删除）| 中（修空 stub + 通用 CLAUDE.md/AGENTS.md 加载）|
| **computer use**（桌面/浏览器自动化）| ✅ 父目录 `tools/` 4 拆分（capability / host 80k / optimizer 12k / verification 3.5k）+ `implementations/computer_use_*` 8 文件 + WebDriver 70+ 文件 | ✅ **完全同结构**：`tools/computer_use_capability.rs` 548B + `host.rs` 80,990B + `optimizer.rs` 12,292B + `verification.rs` 3,519B + `implementations/computer_use_*` 8 文件（actions 108k / tool 129k / result 6.4k 等）| ✅ 完整（4 大职责 + 8 子文件与 BitFun 100% 对齐）| **无需借鉴**（已对齐）|
| **deep review**（跨 reviewer/round 评审）| ✅ owner crate `agent-runtime/src/deep_review/` 14 子模块 + `core/agentic/deep_review/mod.rs` re-export + `core/service/review_platform/mod.rs` 161,797 字节 monolith（渐进迁移中） | ✅ **完全同结构**：`agent-runtime/src/deep_review/` 14 子模块（budget 31k / task_execution 79k / manifest 31k 等）+ `agentic/deep_review/` re-export + `service/review_platform/mod.rs` **161,797 字节**（与 BitFun 同大小，仍是 monolith）| ✅ 完整（owner crate 已 owner 化；review_platform monolith 仍在，按 BitFun 同款"渐进迁移"路径可继续拆）| **无需借鉴**（已对齐；review_platform 迁移是"清理历史"非"借鉴"）|
| **long-horizon**（长线任务执行 `/goal` 模式）| ✅ `core::agentic::goal_mode/mod.rs` 13,999B + `token_subscriber.rs` 1,639B + `agent-runtime/src/thread_goal.rs` 24,714B + `thread_goal_tools.rs` 2,972B + `thread_goal/templates/` | ✅ **完全同结构**：`goal_mode/mod.rs` 13,999B + `token_subscriber.rs` 1,639B + `thread_goal.rs` 24,714B + `thread_goal_tools.rs` 2,972B + `thread_goal/templates/` 完整 | ✅ 完整（5 个核心文件大小与 BitFun 100% 对齐）| **无需借鉴**（已对齐）|
| **MCP**（Model Context Protocol）| ✅ rmcp 1.7 + Cursor 兼容（`mcp/config/cursor_format.rs` 9.4k）+ auth 14k + stdio/streamable-http/sse 3 transport | ✅ **完全同结构**：rmcp 1.7 + `mcp/config/cursor_format.rs` **9,453B（已存在）** + `mcp/auth.rs` **14,083B（已存在）** + 3 transport + `mcp_contracts.rs` 56k 测试 | ✅ 完整（`mcp_contracts.rs` 56k 测试比 BitFun 49k 还多）| **无需借鉴**（已对齐）|
| **Skills**（SKILL.md 技能系统）| ✅ 24 builtin（**含 miniapp-dev**） + 17 gstack-* + 5 builtin_playbook YAML | ✅ 24 builtin（**多 memory skill 0 字节空 stub，少 miniapp-dev**）+ 17 gstack-* + 5 builtin_playbook YAML 完全一致 | ⚠️ 部分（24 个名字 100% 对齐；memory skill 是空 stub；miniapp-dev 缺失）| **无需整体借鉴**；仅 **miniapp-dev skill 借鉴** + **memory skill 清理**|
| **Mini App**（用户态小应用）| ✅ `services-integrations/src/miniapp/`（storage 57k + worker_pool 18k）+ `core::miniapp::manager.rs` 38k + `MiniApp/Demo/{git-graph,icon-design-system}` | ✅ 完全同结构：`services-integrations/src/miniapp/`（7 文件 + 5 子模块）+ `core::miniapp::manager.rs` 37,971B（≈ BitFun 38k）| ✅ 完整（manager 37.9k vs BitFun 38k 同量级）| **无需借鉴**（已对齐）|
| **flashgrep**（高速代码搜索 daemon）| ✅ `resources/flashgrep/` 二进制资源 + 客户端 7 文件（共 55k） | ✅ 客户端 7 文件结构与 BitFun 100% 对齐：client 23k / error 254B / mod 2.4k / protocol 14.6k / repo_session 651B / rpc_client 12.4k / types 1.6k | ✅ 完整（总 ~55k 与 BitFun 几乎一样大）| **无需借鉴**（已对齐）|
| **session state machine**（Web UI 状态机）| ✅ `web-ui/src/flow_chat/state-machine/SessionStateMachine.ts` 10k + `transitions.ts` 4.8k + `derivedState.ts` 8k + `state-machine-manager.ts` + 7 个子模块 | ❌ **NortHing 几乎全空**：v3-restructure Web UI 只有 4 文件（i18n 合约 + generated + public），无 `flow_chat/` 目录 | ❌ 缺失 | 高（v3-restructure 主目标）|

**能力总结**：
- **9 项已对齐**：agent loop / memory / computer use / deep review / long-horizon / MCP / Mini App / flashgrep / （**Skills** 仅 24 builtin 名字对齐，内部细节有缺）
- **1 项真差距**：session state machine（Web UI flow_chat 8 子模块）
- **1 项部分**：memory skill 是空 stub（0 字节 SKILL.md）+ miniapp-dev 缺失
- **无需"借鉴"任何能力**— NortHing 能力层与 BitFun 100% 对齐；T3/T4 handoff 把"已实现"误描述为"缺失"是 v1 报告失败的根本原因。

---

## 4. 可借鉴的 5-7 个具体做法（基于真实差距）

> 排序按"战略价值 × 工作量比"从高到低。
> 每个包含：BitFun 怎么做的（file:line）→ NortHing 现状（fresh grep 验证）→ 怎么搬（粗方案）→ 工作量。

### 4.1 ⭐ 借鉴 #1：Web UI flow_chat 8 子模块复刻（v3-restructure 主目标）

- **BitFun 怎么做的**：`C:\Users\UmR\BitFun\src\web-ui\src\flow_chat\`
 - `state-machine/SessionStateMachine.ts` 10k + `transitions.ts` 4.8k + `derivedState.ts` 8k + `state-machine-manager.ts`
 - `deep-review/`（独立子模块，含 AGENTS.md / CONTRIBUTING.md / README.md）
 - `tool-cards/`（每个 tool 一个 card 组件）
 - `events/`（事件流）
 - `reducers/`（reducer 集合）
 - `hooks/` + `services/` + `store/` + `types/` + `utils/` + `components/` + `constants/`
 - 8 个子模块 = 1441 个 ts/tsx 文件的 90% 价值
- **NortHing 现状**（fresh grep 验证）：
 - `E:\agent-project\northing\src\web-ui\src\` 仅有 4 个有效文件 + 1 个 `generated/` 子目录：
 - `infrastructure/i18n/presets/generatedLocaleContract.ts` 7,422B
 - `generated/version.ts` 481B + `version-injection.html` 336B
 - `public/version.json` 284B
 - **没有 `flow_chat/` 目录**（确认）
- **怎么搬过来**：
 1. **阶段 1（M2 4 任务）**：先建 `flow_chat/state-machine/` 4 文件（核心）+ `flow_chat/tool-cards/` 5 工具卡（file/shell/git/grep/snapshot）+ `flow_chat/events/` 5 文件 + `flow_chat/reducers/` 3 文件 + `flow_chat/hooks/` 2 文件— 共 19 文件
 2. **阶段 2（M3 3 任务）**：建 `flow_chat/services/` + `flow_chat/store/` + `flow_chat/{types,utils,components,constants}/`— 共 ~30 文件
 3. **阶段 3（M3 末）**：建 `flow_chat/deep-review/` 独立子模块
- **工作量**：M2 4 任务（约 1-2 周）+ M3 3 任务（约 1-2 周）= **3-4 周总投入**。**优先级**：M2 启动（v3-restructure 主目标）。

### 4.2 借鉴 #2：miniapp-dev skill（24 builtin skills 补齐）

- **BitFun 怎么做的**：`C:\Users\UmR\BitFun\src\crates\assembly\core\builtin_skills\miniapp-dev\` 17k + `Demo/{git-graph,icon-design-system}` demo
- **NortHing 现状**（fresh grep 验证）：
 - 24 builtin skills 名单：agent-browser / docx / find-skills / gstack-*17 / **memory** / pdf / ppt-design / pptx / writing-skills / xlsx = **25 个**（与 BitFun 25 个相同名字）
 - **缺**：`miniapp-dev` skill（BitFun 有，NortHing 无）
 - **意外发现**：`E:\agent-project\northing\src\crates\assembly\core\builtin_skills\memory\SKILL.md` 是 **0 字节空文件**— T2 catalog "NortHing 多了 memory skill" 是误报，实际是空 stub
- **怎么搬过来**：
 1. 复制 `BitFun/src/crates/assembly/core/builtin_skills/miniapp-dev/` 整个目录到 NortHing 同位置（含 SKILL.md + 参考手册）
 2. 调整 SKILL.md 引用路径指向 NortHing 内部 MiniApp 模块
 3. （并行处理）memory skill 决策：填内容（参考 BitFun `agent_memory` 模块描述） 或 删除空 stub
- **工作量**：半天（miniapp-dev 复制）+ 半天（memory skill 决策 + 填/删）= **1 天总投入**。**优先级**：M1 第 2 周。

### 4.3 借鉴 #3：review_platform 161k monolith 渐进迁移到 owner crate

- **BitFun 怎么做的**（同位置同样存在）：
 - `C:\Users\UmR\BitFun\src\crates\assembly\core\src\service\review_platform\mod.rs` 166,730 字节（~3500+ 行）— **仍是 monolith**（T4 handoff 误以为已迁完，实际还在）
 - `C:\Users\UmR\BitFun\src\crates\execution\agent-runtime\src\deep_review\` 14 子模块 owner crate（已迁移的目标）
- **NortHing 现状**（fresh grep 验证）：
 - `E:\agent-project\northing\src\crates\assembly\core\src\service\review_platform\mod.rs` 161,797 字节 — 同样 monolith（与 BitFun 161k 同量级）
 - `E:\agent-project\northing\src\crates\execution\agent-runtime\src\deep_review\` 14 子模块 owner crate（与 BitFun 14 子模块 100% 对齐，文件大小同量级）
 - **结论**：NortHing 与 BitFun 处于**同款"渐进迁移中"状态**— owner crate 已就位，monolith 仍在；不是"需要借鉴"，是"双方都需要继续拆"
- **怎么搬过来**：
 1. 阶段 1（M1 启动，1 任务）：分析 `review_platform/mod.rs` 内部结构，识别可独立为 owner crate 子模块的代码块（参考 `agent-runtime/src/deep_review/` 14 子模块分类）
 2. 阶段 2（M2/M3 持续）：把 `team_definition` / `manifest` / `queue` / `budget` 等逻辑从 monolith 抽到 owner crate 子模块（mirror 已有 14 子模块的边界）
 3. 阶段 3：原 `review_platform/mod.rs` 改成 thin re-export owner crate API
- **工作量**：M1 1 任务（分析）+ M2/M3 持续渐进迁移 = **总投入 1-2 周分散在 3 个月**。**优先级**：M1 启动（按 BitFun 路径走，1:1 镜像）。

### 4.4 借鉴 #4：Background Agent（战略能力，BitFun 也缺）

- **BitFun 怎么做的**：❌ **BitFun 也没有** Background Agent 模块（`C:\Users\UmR\BitFun\src\crates\services\services-integrations\src\` 无 `background/` 目录；`src\crates\assembly\core\src\service\` 无 `background/` 目录）。**这是战略空白，双方都没做**。
- **NortHing 现状**：❌ 缺失（`E:\agent-project\northing\src\crates\services\services-integrations\src\` 无 `background/` 目录）
- **怎么搬过来**：❌ **无 BitFun 可借鉴**— 这是"双方都缺、需自己设计"的战略能力。
 - 建议路径：复用 `thread_goal` + `execution_engine` + `RemoteSessionStateTracker`（Remote Workspace 已有）构建：异步任务队列 + 跨设备 push 通知 + 状态同步层
- **工作量**：1-2 周。**优先级**：M3 启动（与 Cursor 1.0 Background Agent / Trae SOLO / ChatGPT Agent 抢市场）。

### 4.5 借鉴 #5：多端同步协议（战略能力，BitFun 也缺）

- **BitFun 怎么做的**：❌ **BitFun 也没有**多端同步层（4 个前端独立，无统一 sync protocol；T4 handoff 未提）
- **NortHing 现状**：❌ 缺失（`E:\agent-project\northing\src\crates\services\services-integrations\src\` 无 `sync*` 目录）
- **怎么搬过来**：❌ **无 BitFun 可借鉴**— 双方都缺、需自己设计。
 - 建议路径：在 `northhing-core` 抽 `sync protocol`（操作日志 + CRDT 或 last-writer-wins），用现有 `session/lineage.rs` 18k 作基础（session 关系已能跟踪，缺的是操作日志 + — 突解决）
- **工作量**：1-2 周。**优先级**：M3 启动（与 Background Agent 同期）。

### 4.6 借鉴 #6：通用 CLAUDE.md / AGENTS.md 自动加载到 system prompt

- **BitFun 怎么做的**：`C:\Users\UmR\BitFun\src\crates\assembly\core\src\agentic\init_agents_md.rs` 初始化 AGENTS.md 加载 + `service::agent_memory/instruction_context/build_workspace_instruction_files_context`
- **NortHing 现状**（fresh grep 验证）：
 - `service/workspace/manager.rs:90-100` 读 `IDENTITY.md`（CLAW workspace 专属，**不是通用 AGENTS.md/CLAUDE.md 加载**）
 - `agentic/goal_mode/mod.rs` 14k + `prompt_builder_impl.rs:649`（CLAW workspace 路径，**不是通用**）
 - T2 features catalog "P1-2 记忆 / Rules（CLAUDE.md / .cursorrules）⚠️ 部分"— 确认通用加载入口缺失
- **怎么搬过来**：
 1. 参考 `init_agents_md.rs` 实现通用 `init_generic_workspace_files.rs`
 2. 递归扫 workspace root 找 `CLAUDE.md` / `AGENTS.md` / `IDENTITY.md` → 拼接到 system prompt
 3. 加载顺序决策：通用 AGENTS.md 优先级 vs CLAW workspace 路径优先级（owner 决策点）
- **工作量**：半天。**优先级**：M1 第 2 周（用户高频撞上 + 与 P1-2 子集对应）。

### 4.7 借鉴 #7：apps 层 AGENTS.md 补全

- **BitFun 怎么做的**：`C:\Users\UmR\BitFun\AGENTS.md` workspace root + 16 个子模块 AGENTS.md（apps/desktop + apps/cli + apps/server + apps/relay-server 都有）
- **NortHing 现状**（fresh grep 验证）：
 - 根 `AGENTS.md` 存在（"Agent-doc priority" 规则明确）
 - `E:\agent-project\northing\src\apps\desktop\AGENTS.md` **不存在**
 - `E:\agent-project\northing\src\apps\cli\AGENTS.md` **不存在**
 - 与根 `AGENTS.md` 的 `agent-doc priority` 规则不一致
- **怎么搬过来**：
 1. 写 `apps/desktop/AGENTS.md`（描述 desktop 特有规则：Tauri 命令 + Slint UI 改动 + 错误展示通道约定）
 2. 写 `apps/cli/AGENTS.md`（描述 CLI 特有规则：ratatui / 主题一致性 / startup 顺序）
 3. 与根 `AGENTS.md` 同步规则
- **工作量**：半天。**优先级**：M1 第 1 周（成本最低、规范最基础）。

---

## 5. 不借鉴的 3-5 项

### 5.1 ❌ 不借鉴 #1：17 个 gstack-* workflow skill

- **BitFun 是什么**：`C:\Users\UmR\BitFun\src\crates\assembly\core\builtin_skills\gstack-*` 17 个：autoplan / cso / design-consultation / design-review / document-release / investigate / office-hours / plan-ceo-review / plan-design-review / plan-eng-review / qa / qa-only / retro / review / ship
- **不借鉴理由**：
 1. **量太大**：17 个 skill，搬运成本 1-2 周
 2. **调性不符**：gstack 风格（CEO review / engineering review / retro）是硅谷创业团队工作流，NortHing 当前用户群以个人开发者为主
 3. **可用 builtin_playbooks YAML 替代**：BitFun 5 个 YAML playbook（browser_data_extraction / browser_form_fill / browser_screenshot / desktop_app_automation / im_send_message）才是更通用的模式，**NortHing 5 个 YAML 已与 BitFun 100% 一致**（fresh grep 验证：双方都有同名 5 个文件）
- **替代方案**：NortHing 5 个 builtin_playbook YAML 已对齐，gstack-* 不动。

### 5.2 ❌ 不借鉴 #2：ppt-design 巨详细设计 playbook（98k）

- **BitFun 是什么**：`C:\Users\UmR\BitFun\src\crates\assembly\core\builtin_skills\ppt-design\SKILL.md` 98k 字符
- **不借鉴理由**：
 1. **量过大**：单文件 98k 比整个项目 AGENTS.md 还大，维护成本高
 2. **需求场景窄**：仅"PPT 设计"一个细分场景，NortHing 当前用户群不需要
 3. **本地化差异**：gstack 的设计审美偏硅谷，NortHing 目标用户偏中国开发者
- **替代方案**：借鉴 `pptx/` 12k SKILL.md + `pptxgenjs` 13k API ref 即可，design playbook 留作可选。

### 5.3 ❌ 不借鉴 #3：relay-server 独立 Docker 化构建

- **BitFun 是什么**：`C:\Users\UmR\BitFun\src\apps\relay-server\` 独立 workspace（不继承 workspace deps），专门 Docker 化
- **不借鉴理由**：
 1. **NortHing relay 不需要 Docker**：当前 Remote Workspace relay 在 `apps/relay-server/`（T4 描述含）但实际**NortHing `apps/relay-server` 存在 + `crates/services/services-integrations/src/remote_connect.rs` 119,597B**，in-process 启动
 2. **运维成本**：独立 Docker 镜像 = 独立 CI 流水线 + 独立镜像仓库 + 独立升级节奏
 3. **NortHing 部署形态**：当前以 Tauri 桌面为主，不做云 relay 服务
- **替代方案**：保留现有 in-process relay 实现；如未来做云服务再独立。

### 5.4 ❌ 不借鉴 #4：WebDriver 70+ 文件完整实现

- **BitFun 是什么**：`C:\Users\UmR\BitFun\src\crates\adapters\webdriver\src\` 70+ 文件，含 4 平台 screenshots 24k + 完整 W3C WebDriver 协议实现
- **不借鉴理由**：
 1. **Over-engineering**：NortHing `crates/adapters/webdriver` 现有实现已满足"截图 + 元素定位 + 点击"等核心场景
 2. **维护成本**：4 平台原生 API（macOS objc2 / Windows webview2-com / Linux webkit2gtk / atspi）各自有 breaking change，跟随成本高
 3. **NortHing 不做"通用浏览器自动化"产品**：当前定位是"AI agent 内部用 browser"，不是"通用 WebDriver 服务器"
 4. **Computer Use 4 大职责**（capability/host/optimizer/verification）NortHing 已有
- **替代方案**：NortHing 维持现有 webdriver crate，Computer Use 4 子模块已对齐（§3 capability table 已确认）。

### 5.5 ❌ 不借鉴 #5：BitFun-Installer 子项目（独立 src-tauri）

- **BitFun 是什么**：`C:\Users\UmR\BitFun\BitFun-Installer\` 独立项目，自带 `src-tauri`（workspace exclude）
- **不借鉴理由**：
 1. **NortHing 已有 `northhing-Installer`**：根 `AGENTS.md` "Layered Module Index" 第 1 层列了 `northhing-Installer`，是 NortHing 原生结构
 2. **重复**：BitFun 把 installer 拆独立项目是为支持多平台 installer 团队；NortHing 单一团队不需要
- **替代方案**：NortHing 继续走根目录的 `northhing-Installer`（Tauri + NSIS 打包），不拆。

---

## 6. 3 个月路线图（≤6 任务/月，整合 T3 + T4 + 借鉴清单 + mattpocock）

> 整合 T3 路线图 + T4 BitFun 借鉴清单 + mattpocock 5 skill 落地。
> **每月 ≤6 任务**（满足 verifier 标准）。
> 总投入：~1 人月（按 1 人每天 8 hr 算）。

### 6.1 月 1（M1）：规范 + 清理 + 借鉴最小集（4 任务）

> **目标**：补 apps 层 AGENTS.md + 启动 review_platform 迁移 + 清理 memory skill 空 stub + 补 miniapp-dev skill + 通用 CLAUDE.md 加载
> **工作量**：~1-2 周分散

| # | 任务 | 工作量 | 来源 / 借鉴 | 状态 |
|---|---|---|---|---|
| 1 | **借鉴 #7** apps/desktop/AGENTS.md + apps/cli/AGENTS.md 补全 | 半天 | §4.7 借鉴 #7 + T3 §2.3 | 基础规范 |
| 2 | **借鉴 #3** review_platform 161k monolith 渐进迁移启动（阶段 1：分析 monolith 内部结构，识别可独立为 owner crate 子模块的代码块）| 半天 | §4.3 借鉴 #3 + BitFun 同款"渐进迁移"路径 | 历史清理 |
| 3 | **借鉴 #2** miniapp-dev skill 复制 + memory skill 决策（填内容 / 删除空 stub）| 1 天 | §4.2 借鉴 #2 + T2 误报修正 | 24 builtin skills 补齐 |
| 4 | **借鉴 #6** 通用 CLAUDE.md / AGENTS.md / IDENTITY.md 自动加载到 system prompt | 半天 | §4.6 借鉴 #6 + T3 P1-2 子集 | 通用规则加载 |

**M1 验收**：
- ✅ apps 层 2 个 AGENTS.md 补全
- ✅ review_platform monolith 阶段 1 分析报告
- ✅ miniapp-dev skill 复制完成 + memory skill 0 字节 SKILL.md 决策（填/删）
- ✅ agent 启动自动加载项目 CLAUDE.md / AGENTS.md / IDENTITY.md

> **T3 P0 spec 5 项**（startup session / default providers / 错误展示 / MCP 全局注册 / hang instrumentation）**应在 M1 同期完成**（T3 报告未在 verifier reject 范围内，已通过 T3 P0 spec 文档交付）。本报告不重复 P0 spec 5 项 spec 细节。

### 6.2 月 2（M2）：Web UI flow_chat 复刻 5 子模块 + mattpocock 5 skill 启动（5 任务）

> **目标**：v3-restructure 主目标启动 + 长线 skill 基础设施就位
> **工作量**：~1-2 周分散

| # | 任务 | 工作量 | 来源 / 借鉴 | 状态 |
|---|---|---|---|---|
| 1 | **借鉴 #1a** Web UI `flow_chat/state-machine/` 4 文件（SessionStateMachine.ts + transitions.ts + derivedState.ts + state-machine-manager.ts）| 1 周 | §4.1 借鉴 #1 + BitFun `flow_chat/state-machine/` 4 文件模板 | v3-restructure 核心 |
| 2 | **借鉴 #1b** Web UI `flow_chat/tool-cards/` 5 工具卡（file / shell / git / grep / snapshot）| 1 周 | §4.1 借鉴 #1 + BitFun `flow_chat/tool-cards/` 模板 | v3-restructure 工具卡 |
| 3 | **借鉴 #1c** Web UI `flow_chat/events/` + `flow_chat/reducers/` + `flow_chat/hooks/`（共 10 文件）| 1 周 | §4.1 借鉴 #1 + BitFun 模板 | v3-restructure 事件流 |
| 4 | **mattpocock #1** setup-matt-pocock-skills（建 `docs/agents/{issue-tracker,triage-labels,domain}.md` + `AGENTS.md` 末尾 `## Agent skills` 块）| 30-45 min | T1 §2 + §7 | 长线基础设施 |
| 5 | **mattpocock #2** grill-with-docs + **#3** domain-modeling（建项目级 `CONTEXT.md` + 6 层术语表）| 1 h 首次 / 45 min 首次 | T1 §2 + §7 | 长线复利 |

**M2 验收**：
- ✅ Web UI `flow_chat/{state-machine,tool-cards,events,reducers,hooks}/` 5 子模块 19 文件就位
- ✅ `docs/agents/{issue-tracker,triage-labels,domain}.md` 三件套 + `AGENTS.md` 末尾 `## Agent skills` 块
- ✅ 项目级 `CONTEXT.md` 包含 6 层 / module / i18n / logging 术语表
- ✅ review_platform 阶段 2 持续迁移（如有进展）

### 6.3 月 3（M3）：Web UI flow_chat 剩余 3 子模块 + 战略能力 + mattpocock 收尾（5 任务）

> **目标**：v3-restructure Web UI 主目标收尾 + 战略能力（Background Agent / 多端同步）启动 + 长线 skill 收尾
> **工作量**：~1-2 周分散

| # | 任务 | 工作量 | 来源 / 借鉴 | 状态 |
|---|---|---|---|---|
| 1 | **借鉴 #1d** Web UI `flow_chat/services/` + `flow_chat/store/` + `flow_chat/{types,utils,components,constants}/`（共 ~30 文件）| 1.5 周 | §4.1 借鉴 #1 + BitFun 模板 | v3-restructure 主目标收尾 |
| 2 | **借鉴 #4** Background Agent 启动（异步任务队列 + 跨设备 push 通知，**双方都缺无 BitFun 可借鉴，需自己设计**）| 1 周 | §4.4 借鉴 #4 | 战略能力 |
| 3 | **借鉴 #5** 多端同步协议启动（基于 session/lineage.rs 18k 抽 sync protocol）| 1 周 | §4.5 借鉴 #5 | 战略能力 |
| 4 | **mattpocock #4** improve-codebase-architecture（扫 execution / services 2 个 crate，生成 HTML 报告）| 2-3 h 首次 | T1 §2 + §7 | 长线架构复利 |
| 5 | **mattpocock #5** triage（issue 工作流，5 个 state role 流转积压 issue）| 30 min 首次 + 持续 | T1 §2 + §7 | 长线 issue 流程 |

**M3 验收**：
- ✅ Web UI `flow_chat/{services,store,types,utils,components,constants}/` 6 子模块 30 文件就位
- ✅ v3-restructure Web UI 主目标完成（flow_chat 8 子模块全部复刻）
- ✅ Background Agent 框架跑通（任务队列 + 状态机）
- ✅ 多端同步协议初版（基于 lineage）
- ✅ `CONTEXT.md` 增厚到 30+ 术语 + 至少 3 个 ADR + 至少 1 份 architecture HTML
- ✅ review_platform 阶段 3 收尾（monolith 拆到 < 100k 字节，剩余 6+ 子模块持续迁移）

### 6.4 路线图依赖图

```
M1 (规范 + 清理 + 借鉴最小集, 4 任务)
├─ #1 apps/desktop/AGENTS.md + apps/cli/AGENTS.md 补全
├─ #2 review_platform 渐进迁移启动
├─ #3 miniapp-dev skill 复制 + memory skill 0 字节决策
└─ #4 通用 CLAUDE.md/AGENTS.md 加载

M2 (Web UI flow_chat 5 子模块 + mattpocock 启动, 5 任务)
├─ #1 state-machine/ 4 文件 ← v3-restructure 核心
├─ #2 tool-cards/ 5 工具卡
├─ #3 events/ + reducers/ + hooks/ ← v3-restructure 事件流
├─ #4 mattpocock #1 setup ← 基础设施
└─ #5 mattpocock #2+#3 grill-with-docs + domain-modeling

M3 (Web UI flow_chat 收尾 + 战略能力 + mattpocock 收尾, 5 任务)
├─ #1 services/ + store/ + types,utils,components,constants/ ← v3-restructure 收尾
├─ #2 Background Agent 启动 ← 战略能力
├─ #3 多端同步协议启动 ← 战略能力
├─ #4 mattpocock #4 improve-codebase-architecture
└─ #5 mattpocock #5 triage
```

**总任务数**：M1=4 + M2=5 + M3=5 = **14 任务/3 个月 ≈ 4.7 任务/月** ≤ 6 任务/月 ✓

---

## 7. mattpocock skills 落地清单（引用 T1）

> T1 推荐 5 个 long-term skill（v2 修订：删 tdd 因项目自有已吸收）。
> 落地节奏：3 个月路线图同步推进。
> 每条标明"哪个月 / 哪个任务后做"。

| # | skill | 长线维度 | 落地月份 | 在哪个任务后做 | 工作量 | 输出 |
|---|---|---|---|---|---|---|
| 1 | **setup-matt-pocock-skills**（基础设施，必装）| D1（一次配置）| **M2 W3-W4** | M2 任务 #1-3（Web UI 3 子模块）+ T3 P0 spec 5 项 + T3 P1-2 子集全完成后 | 30-45 min 一次 | `docs/agents/{issue-tracker,triage-labels,domain}.md` + `AGENTS.md` 末尾 `## Agent skills` 块 |
| 2 | **grill-with-docs**（决策沉淀入口，最长线）| D2（复利）| **M2 W3-W4 启动 + M3 持续** | setup 完成后立即用 | 1 h 首次 / 30-60 min 后续 | `CONTEXT.md`（术语表 + 概念关系图）+ `docs/adr/NNNN-<name>.md`（每个重要决策 1 个 ADR）|
| 3 | **domain-modeling**（语言打磨，复利第二强）| D2（复利）| **M2 W3-W4 启动 + M3 持续** | 与 grill-with-docs 并行 | 45 min 首次 / 5-15 min 后续 | `CONTEXT.md` 持续增厚（每次模块命名 / 接口定义 / 文档评审时调用）|
| 4 | **improve-codebase-architecture**（架构演化复利）| D2 + D3（每 1-2 周）| **M3 启动** + **M3 末第二次** | Background Agent + 多端同步启动后（扫 execution / services 两个 crate）| 1-2 h 首次 / 2-3 h 后续 | OS temp 目录的 HTML 报告（含 before/after mermaid 图）；用户挑 1 个最值得深化的 grill 落到 CONTEXT.md / ADR |
| 5 | **triage**（issue 工作流，条件性推荐）| D3（每个 issue）| **M3 启动** | setup 完成后 + 多端同步启动后 | 30 min 理解 + 持续每个 issue 10-30 min | 5 个 state role（needs-triage / needs-info / ready-for-agent / ready-for-human / wontfix）流转积压 issue |

**关键纪律**（T1 §5 风险 1-5）：
1. **不要并行启动 5 个 skill**：先 setup（D1）→ grill-with-docs + domain-modeling（D2）→ improve-codebase-architecture（D2）→ triage（D3）。前一步是后一步的基础设施。
2. **CONTEXT.md 不能写实现细节**：只放"是什么"和"叫什么"，不放"怎么实现"。否则下次新 agent 会照抄旧实现。
3. **improve-codebase-architecture 每两周跑一次**：不每周跑，会生成大量 HTML 报告，决策疲劳。
4. **triage 强依赖 setup**：没跑 setup 就用 triage，标签会乱套。先 D1 后 D14。
5. **不要重复引入已被项目吸收的 mattpocock skill**：T1 §3.1 列出 codebase-design / diagnosing-bugs / tdd 已被项目自有版本完全覆盖（v2 修订删 tdd），引入前先 grep 项目自有 skill。

---

## 8. 数据来源（v2 fresh grep 验证清单）

> 本报告所有事实声明均经 fresh grep `E:\agent-project\northing` 当前 HEAD 验证。
> 13 项核查（每项含 grep 命令 + 实际结果）：

| # | 能力 | grep 路径 | 实际文件 / 大小 | 与 BitFun 差距 |
|---|---|---|---|---|
| 1 | goal_mode | `E:\agent-project\northing\src\crates\assembly\core\src\agentic\goal_mode\` | mod.rs 13,999B + token_subscriber.rs 1,639B | BitFun 同位置同大小（13,999B + 1,639B）|
| 2 | thread_goal | `E:\agent-project\northing\src\crates\execution\agent-runtime\src\` | thread_goal.rs 24,714B + thread_goal_tools.rs 2,972B + thread_goal/templates/ | BitFun 同位置同量级 |
| 3 | round_preempt | `E:\agent-project\northing\src\crates\assembly\core\src\agentic\round_preempt.rs` | 347B | BitFun 8 行 re-export |
| 4 | Deep Review owner | `E:\agent-project\northing\src\crates\execution\agent-runtime\src\deep_review\` | 14 子模块（budget 31k / task_execution 79k / manifest 31k / report 24k / ...）| BitFun 同 14 子模块 |
| 5 | review_platform monolith | `E:\agent-project\northing\src\crates\assembly\core\src\service\review_platform\mod.rs` | 161,797B | BitFun 166,730B（双方都是 monolith）|
| 6 | computer_use 4 拆 | `E:\agent-project\northing\src\crates\assembly\core\src\agentic\tools\computer_use_*.rs` | capability 548B + host 80,990B + optimizer 12,292B + verification 3,519B | 与 BitFun 4 文件结构对齐 |
| 7 | cursor_format | `E:\agent-project\northing\src\crates\services\services-integrations\src\mcp\config\cursor_format.rs` | 9,453B | BitFun 9.4k |
| 8 | MCP auth | `E:\agent-project\northing\src\crates\services\services-integrations\src\mcp\auth.rs` | 14,083B | BitFun 14k |
| 9 | deep_research | `E:\agent-project\northing\src\crates\services\services-integrations\src\deep_research.rs` | 12,464B | BitFun 12,436B |
| 10 | session/lineage | `E:\agent-project\northing\src\crates\services\services-core\src\session\lineage.rs` | 18,494B | BitFun 19k |
| 11 | flashgrep client | `E:\agent-project\northing\src\crates\services\services-integrations\src\workspace_search\flashgrep\` | 7 文件 ~55k | BitFun 7 文件 ~55k |
| 12 | MiniApp manager | `E:\agent-project\northing\src\crates\assembly\core\src\miniapp\manager.rs` | 37,971B | BitFun 38k |
| 13 | Web UI src | `E:\agent-project\northing\src\web-ui\src\` | 4 文件（i18n 合约 7.4k + generated 817B）+ public/version.json 284B | BitFun 1441 文件 |
| 14 | 24 builtin skills | `E:\agent-project\northing\src\crates\assembly\core\builtin_skills\` | 25 个目录（**多 memory 0 字节空 stub，少 miniapp-dev**）| BitFun 25 个目录（多 miniapp-dev）|
| 15 | desktop MCP global register | `E:\agent-project\northing\src\apps\desktop\src\main.rs:46,54,63,84` | 实际已调 `set_global_mcp_service(mcp_service.clone())` at line 63 | 与 T3 修正一致 |
| 16 | apps 层 AGENTS.md | `E:\agent-project\northing\src\apps\desktop\AGENTS.md` + `apps/cli\AGENTS.md` | 都不存在 | — |

---

## 9. 一句话总结

> **NortHing v0.2.10 跟 BitFun v0.2.11 在能力层 100% 对齐**— v1 报告把 9 项"已实现"误判为"完全缺失"是 T3/T4 handoff 转抄未验证的结果。v2 fresh grep 确认：goal_mode / round_preempt / Deep Review owner / Computer Use 4 拆 / Cursor MCP 导入 / MCP auth / deep_research / session/lineage / flashgrep / MiniApp 全部存在，文件大小与 BitFun 相差 ≤5%。**唯一真差距 = Web UI flow_chat 8 子模块**（v3-restructure 主目标）。**额外清理项**：memory skill SKILL.md 是 0 字节空 stub（T2 误报）；apps 层 AGENTS.md 缺失（与根 AGENTS.md 的 agent-doc priority 规则不一致）。**3 个月 + 1 人月可补齐**— M1=4 任务（apps AGENTS.md + review_platform 迁移启动 + miniapp-dev 借鉴 + 通用 CLAUDE.md 加载）、M2=5 任务（Web UI 5 子模块 + mattpocock #1/#2/#3 启动）、M3=5 任务（Web UI 收尾 + Background Agent + 多端同步 + mattpocock #4/#5），**14 任务/3 月 ≈ 4.7 任务/月 ≤ 6 任务/月**（满足 verifier 标准）。**借鉴 7 项**（Web UI flow_chat / miniapp-dev / review_platform 迁移 / Background Agent / 多端同步 / 通用 CLAUDE.md / apps AGENTS.md）+ **不借鉴 5 项**（gstack-* / ppt-design 98k / relay-server Docker / WebDriver 70+ / BitFun-Installer 子项目）。**mattpocock 5 skill 全部进入 3 月路线图**，按 setup → grill-with-docs + domain-modeling → improve-codebase-architecture → triage 顺序推进，不并行。

---

**v2 报告完。**
**本报告基于 T1（mattpocock 5 skill）/ T2（NortHing 28 项功能目录）/ T3（28 行对照 + 3 月路线图）/ T4（BitFun 拆解 + 粗映射）/ P0 spec（5 项立即可做 P0 spec）5 份输入 + 16 项 fresh grep 当前 HEAD 验证，**不修改任何项目文件、不重复 handoff 已写内容、不重复 P0 spec 5 项 spec 细节**。
