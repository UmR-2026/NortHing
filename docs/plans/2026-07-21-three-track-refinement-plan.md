# 三线精细化改良规划（评审稿 v0.1）

> 2026-07-21。供用户与外部 agent 评审。每条 ticket 为处方级：file:line + API 签名 + 字段清单 + 验收标准。
> 前置阅读：`docs/architecture/agent-kernel-northstar.md`（北极星 v0.3.1）、`AGENTS.md` 骨架不变量节、本文件末尾「评审待定项」。

---

# 修订记录 v0.2（2026-07-21 晚，外部评审后）

> 外部评审：`northing-review-consolidated-2026-07-21.md`（QoderWork）。本节优先级高于 v0.1 正文。

## v0.2.1 外部评审 §一~三 对账（防重复派工）

评审 §一（全仓代码）、§二（冗余清单）、§三（facade 设计稿）是对 7-19/20 旧快照的重复发现，**已落地，禁止再派工**：

- C-1 MiniApp 注入 ✅ `e19f0ec`+`0089ad3`（07-19）；C-2/C-3/I-7~I-11 ✅ A 批 `95682b8` + F1 重建；C-4 ✅ D 批 `2e86d54`；I-9 workaround ✅ `0fb5086` 拆除；I-1/I-3/M-1/M-2/M-3 ✅ C 批 `fac3013`+`81d78bc`；I-4~I-6 ✅ D 批 + 砍单批 `9b87431`。
- §二 冗余：E1-E4 + E3a/b/c + 砍单批全部落地（07-20）。残留核实（07-21）：NullEmitter/LoggingEmitter 与 agent-dispatch 锚点零命中（已删）；GitError/TerminalError 变体均在使用或为 #[from] 便利（不删）。
- §三 facade 三条：07-20 已修入文档（CI cargo tree 守卫 + desktop-tauri 独立 tree 验收 + workspace 覆盖路径核实）。
- **真正未修（转入队列）**：I-2 core 优雅关停（需设计）；M-7 队列锁/M-8 死 token/M-10 标题取消（低优）；M-9 敏感诊断默认 true（待用户拍板）。

## v0.2.2 用户拍板（2026-07-21）

1. **K3 闸门**：NO-GO 大拆。转低风险替代（K4 候选：cargo 配置/sccache/定向拆 crate），K3 探针数据存档备查。
2. **C1 身份文案**：对等同事，agent-centric——"不是助手/帮手，是旁边工位的同事"。详见 C1 修订段。
3. **C4 技能候选**：不自动启用，走人工审核（接受评审边界解释）。
4. B5 genRef 现在做；C6 并行白名单按 ToolContract readonly 标记；A5 用 tauri-plugin-opener；A 线一口气 10 单。
5. **A 线暂缓**：用户在做前端/logo 视觉调整，A 线 10 单 + A12-A16（下节）等其收官后启动，避免同仓碰撞。
6. 设置页最小可用不提前，仍归 F2。

## v0.2.3 外部评审 §6 收编（Track A 追加，随 A 线启动）

- A12 启动过渡态（logo + 「正在连接…」淡出，消灭 1-3s 黑屏）
- A13 空态文案改对等语气（随 C1 文案统一）
- A14 会话列表侧边栏（A8 懒建会话后必需）
- A15 消息时间分隔线（间隔 >5min）
- A16 窗口标题动态状态 + 任务栏闪烁
- A17 思考块展开/折叠平滑过渡（grid-template-rows）

## v0.2.4 执行状态（持续更新）

| 单 | 状态 | commit |
|---|---|---|
| B1a+B7 skills BOM/乱码/loader | ✅ judge PASS + follow-up 修复 | `46172ec` `eed3da8` |
| B1b 模型运行时信息注入 | ✅ judge PASS | `101bc1f` |
| B8 两个遗留测试修复 | ✅ judge PASS（skill_tool 系 B1a/B7 顺带治愈） | `82ea09f` |
| B3 subscribe_events Result + workspace_path | ✅ judge PASS + Err 路径测试补齐 | `6f039ca` `4f251da` |
| B2-core turn 相位一等事件 | ✅ judge FAIL→返修→PASS | `9493450` |
| B4 turn failed 正路由 + error_kind + result_count | ✅ judge PASS | `062959f` |
| B5 stale outcome 守卫 | ❌ judge FAIL（设计缺陷）→ **已 revert**，转设计先行 | `48f4ce2`+`c7eb712`(revert) |
| B6 K3 闸门 | NO-GO（用户拍板，转低风险替代） | — |
| C1 agent 身份重写（对等同事） | ✅ CLI exec 实证（独立智能体+LongCat-2.0 自报） | `0704ceb` |
| C2 Episode Log Phase 1 | ✅ judge FAIL（placeholder 测试+跨 round bug）→返修→PASS | `159c10d` |
| C3 结构化记忆 facts | ✅ judge FAIL×2（placeholder 测试+主段越界+去重缺陷）→返修+编排者收尾→PASS | `8e4eab2` `d7e6b62` |

**B5 复盘（设计先行要点，judge 复审产出）**：①`None→stale` 假设不成立——coordinator 直连路径（`AgentSubmissionPort::submit_message` subagent_ports.rs:93-133 直连 `start_dialog_turn`）与"outcome 先到、active 后插入"竞态下合法 outcome 无 active 记录；②get-check-then-remove 非原子（TOCTOU），应改 dashmap `remove_if` 按 expected turn_id 原子消费；③stale 分支的 `drain_for_turn` 会误吞新 turn 的 CurrentRunningTurn injection，需 `drain_exact_turn`；④重设计方向：outcome 分类（Matching/DifferentActive/MissingFirstOutcome/AlreadyProcessed，tombstone 集）+ scheduler-owned turn 在 coordinator spawn 前登记 turn id + 直连路径统一路由或保留旧生命周期 + 集成测试矩阵（直连+排队/早到/插入竞态/重复/cancelled/goal continuation）。**环境敏感测试家族**（tests_cancel/tests_timeout/tests_concurrent/tests_error，假设"本机无 LLM 微秒失败"）需改注入确定性 fake backend——独立测试基建单。

遗留 follow-up：①`Agent::system_prompt_cache_identity` scope_key 不含 model_name（judge M1，换模型命中旧 Runtime 段）；②turn_lifecycle 配置查找失败加 debug 日志（M2）；③两 build 方法 trim 不齐（M4）；④subagent_ports 环境敏感测试家族改造（见上）；⑤`scheduler` active remove 原子化已并入 B5 重设计。

## v0.2.5 成长架构 v2（外部评审 §5 采纳，替代 v0.1 C2-C4 的池化设计）

**Phase 0 第一刀 = judge agent 系统化**（复用 `agentic/deep_review`），一切固化动作过 judge 门禁。

已闭合边界（不可回退）：

| 边界 | 决议 |
|---|---|
| 成长轨迹 | = 日记：人类可读、**agent 不读它做决策**（防自我验证闭环） |
| 本体记忆 | = 封闭基底：仅 agent 维护，人类不可读（与日记是两个存储） |
| 技能库 ∥ 认知演化 | 并行，非取舍 |
| 固化目的地 | 分权：agent 提案 × judge 门禁 |
| 收敛锚点 | 否定式 invariant + judge 无权重定义 |

收敛锚点（P0，不可让步）：

1. **否定式红线优先于肯定式目标**：I-NEG-1~4 写入 frozen 只读区，judge 无权修改；红线定义在**状态层**（"文件在原路径仍可访问且非空"），非操作层。
2. **评判看世界状态，不信自评文本**：evidence_pack 必须含 trace / fs diff / 成功率对比 / 人类反馈；candidate 的"我认为这是进步"权重 = 0。

验收测试五硬化（结构层防御）：

1. 红线表即测试规格 + 异源生成器（与 judge 不同模型家族）+ 手写 canonical 违反集作 smoke test。
2. 哈希覆盖 redline **执行代码源码**，或 redline 执行器放隔离特权域。
3. soul 写入**默认 staging**，比例健康才提升正式；τ/N 进 frozen 区。
4. 泛化为 **cargo tree 断言零依赖边**（agent-runtime 对日记存储 crate），编译期阻止。
5. 滚动基线种子来自**冻结 golden 回归集** + ε = f(测试集规模)（Hoeffding 界）。

安全设计原则：**防御推到结构层（编译期不可能 / 默认隔离 / 无依赖边），不停留在检测层。**

soul-creep 检测：比例监控（soul 增速/skill 增速），不靠内容分类；连续 N 个候选被 reroute → 校准信号；检测由 judge 基于 trace 做出。

**C2/C3/C4 因此修订**：
- C2 Episode Log → 拆两个存储：`episodes/`（日记，人类可读，agent 不注入决策）与本体记忆（封闭，C3 facts 归入）。DB 结构同期埋入红线字段（评审 §7.6：不能先跑起来再加安全）。
- C3 facts → 本体记忆（agent 专属），日记不做 prompt 注入。
- C4 → Phase 0 之后：候选产出过 judge 门禁（不自动启用的人工审核即由 judge agent 承担）。
- C7（压缩蒸馏）等 C2 落地后再做（评审 §7.7）：压缩触发派蒸馏 subagent（可用小模型），锚点原文精度保留、叙事激进压缩、异步非阻塞、产物同时喂 C2（一次蒸馏两处受益）。

---

## 0. 背景与证据链

本规划输入四个证据源：

1. **K 线实测**：K0 基线（增量 check 29.86s）→ K2 验收实测 31.36s/31.49s，14.93s 目标未达（K2 不动主 workspace 结构，预期内）。K3 ROI 闸门待拍板。
2. **opencode（GitHub 实证）**：desktop 从 Tauri 迁 Electron（2026-02~05，`packages/desktop` 现为 electron-vite + SolidJS）。教训：壳内禁止长业务逻辑（他们被迫反复 port shell/env 逻辑）；前端壳可替换性验证成立。
3. **opencode/WorkBuddy/QoderWork 前端源码**（GitHub + 本地 asar 提取）：状态 shimmer、回到底部悬浮钮、输入历史上翻、TriggerTitle 工具标题结构化、时态切换（Searching→Searched）、错误分层（可恢复才给重试）、代际保护 genRef、草稿态懒建会话、每会话草稿持久化、流式工具卡片渲染缓存、sandbox 外链外开、KaTeX/Mermaid。
4. **自查发现**：`agentic_mode.md:1` 身份是 IDE 时代遗留（"You are northhing, an ADE (AI IDE)"），与锁定产品基线（隐藏 IDE 的通用 agent）冲突；同文件 line 58 存在编码损坏（`task鈥攖hree`）；`agentic/insights/` 已有 session 洞察子系统（friction/suggestions/wins 等 facet 提取）；`service/agent_memory/` 仅有 markdown 索引注入，无结构化事实库。

## 1. 全局约束（违反即 FAIL）

- **骨架不变量**（AGENTS.md）：配置单一来源 core GlobalConfig；facade 政策（产品面只经 kernel-api 触达 core）；slug 恒带路径哈希；desktop-tauri 不参与主 workspace；`cargo check -p northhing` 恒绿。
- **产品基线**：隐藏 IDE 的通用 agent；subagent 对人类隐藏（会话树可审核）；全免确认 + denylist 兜底；仅 Slint 桌面 + installer 发货（F4 前），desktop-tauri 为候选新基线。
- **派遣纪律**：实施默认 m27hs + 处方级任务书；验收 judge-m3；`--features product-full` 必显式；coder 任务期间编排者不留未提交改动；任务书明写"必须 commit、禁止 git restore 非你创建文件"。
- **壳薄纪律**（opencode 教训）：desktop-tauri 的 Rust 侧只做 core_rt + event bridge + commands 转发，业务逻辑禁止上行。

## 2. Track A — 前端 UI 美学/功能改良

> 全部位于 `src/apps/desktop-tauri/ui/src/`。除 A5 外不碰 Rust。每单验收含 `pnpm --dir src/apps/desktop-tauri/ui run type-check` 0 error。

### A1. 状态文字 shimmer 扫光

- 参照：opencode `packages/ui/src/components/text-shimmer.css`（渐变 background-clip:text + 逐字 delay + prefers-reduced-motion 降级）。
- 改动：`components/TurnTrace.tsx` agent-status 文本包 `<ShimmerText text active={live} />`（新组件，CSS 驱动，单元素实现：双层 char 太重的话简化为整文本渐变扫光）；`app.css` 加 `.text-shimmer` 规则（token：`--text-shimmer-duration: 1200ms`、step 45ms 逐字 delay 可选）。
- 验收：live turn 状态行（深度思考/生成回复中/执行工具）有扫光动画；系统开启 reduced-motion 时静态显示。

### A2. 「回到底部」悬浮按钮

- 改动：`components/MessageList.tsx`——新增 state `showJump`：onScroll 时若 `!stickToBottom.current` 且 `scrollHeight - scrollTop - clientHeight > 200` 则 true；messages/streamingText 变化时复算。按钮 fixed 于消息区底部中央（`.jump-latest` 样式：圆形 ↓、bg-elev + border + shadow-pop，msg-in 动画）。点击：`scrollTop = scrollHeight; stickToBottom.current = true; setShowJump(false)`。
- 参照：opencode `message-timeline.tsx` 的 jump-to-latest。
- 验收：上翻 >200px 后新 chunk 到达按钮出现；点击回底并消失；贴底时不出现。

### A3. 输入历史上翻回忆

- 新文件：`lib/promptHistory.ts`——`loadHistory(): string[]`（localStorage `northhing.prompt-history`，JSON 数组）、`pushHistory(text)`（trim 非空、与首条不同才 unshift、cap 100）、纯函数 `navigateHistory(direction, index, entries): {index, text | null}`（逻辑照 opencode `prompt-input/history.ts` 的 navigatePromptHistory：up 越界返回 null、down 回 index=-1 恢复草稿）。
- 改动：`components/Composer.tsx`——textarea onKeyDown 增加：光标在 0 且按 ↑ → recall；按 ↓ → 前进；发送成功后 pushHistory。草稿（进入历史前的 input）暂存，退出历史恢复。
- 验收：发送 3 条后可逐条上翻；↓ 退到底恢复未发送草稿；连续相同消息只存一条。

### A4. 工具节标题结构化 + completed 收敛态

- 改动：`components/TurnTrace.tsx` `ToolSection`——
  - started：name chip + summary + `…进行中` badge（保留）。
  - completed：去 badge，`.tool-summary` 整行 `color: var(--text-faint)`（收敛态）。
  - subtitle：`JSON.parse(entry.detail ?? "")` 容错后按优先键 `["filePath","path","cmd","command","query","url","description"]` 取首个 string，截断 60 字符（`…`），渲染为 `.tool-subtitle`（mono、dim）跟在 summary 后。
  - 整节 `React.memo`（见 A6）。
- 验收：真实 shell 调用截图——started 有 badge、completed 无 badge 且降 dim、subtitle 显示命令首参。

### A5. markdown 外链系统浏览器打开

- 现状隐患：`.md a` 点击在 webview 内无响应/误导航。
- 方案（首选）：src-tauri 加 `tauri-plugin-opener`（Cargo.toml + `tauri.conf` 无配置 + capabilities `core:opener:default`）+ ui 加 `@tauri-apps/plugin-opener`；`components/Markdown.tsx` components.a 渲染为 `<a href onClick={e => { e.preventDefault(); openUrl(href); }}>`。备选：无插件时 `window.open(href, "_blank")` 降级。
- 验收：`cargo check --manifest-path src/apps/desktop-tauri/src-tauri/Cargo.toml` 0 error；GUI 点链接系统浏览器打开。

### A6. 重渲染 memo

- 改动：`ToolSection`、`Markdown`（及其 CodeBlock）包 `React.memo`；`TurnContainer` 不动（props 每 chunk 变）。
- 验收：type-check；流式期间行为不变（工具节不随 chunk 重渲，profiler 可选）。

### A7. 每会话草稿持久化

- 新文件：`lib/draftStore.ts`——`loadDraft(sid): string`、`saveDraft(sid, text)`（空串则 removeItem）。
- 改动：`App.tsx`——effect `[sessionId]`：装载草稿到 input；effect `[input, sessionId]`：300ms debounce saveDraft；`handleSend` 成功后 saveDraft(sid, "")。
- 验收：输入未发送刷新应用后草稿恢复；发送后清空；切会话各存各的。

### A8. 草稿态懒建会话

- 参照：WorkBuddy「首发消息才 POST 建 session，避免空会话堆积」。
- 改动：`hooks/useChat.ts`——init effect 不再调 `getOrCreateLatestSession`：改 `listSessions()`，有→取最新 id + getMessages；无→`sessionId = null`（空态页）。`send` 内：`sessionId == null` 时先 `createSession()` → setSessionId → 再 sendMessage。`stop`/stopStreaming 守卫已有。api.ts 已有全部所需命令。
- 兼容：I-7 pendingEvents buffer 逻辑不变（sessionId null 时照旧 buffer）。
- 验收：全新环境（无会话）启动无空会话落盘；首发消息建会话并正常流式；有会话时行为同现状。

### A9. turn 失败错误卡 + 已中断分隔

- 改动：`hooks/useChat.ts` `TurnTraceData`（components/TurnTrace.tsx 接口）加 `error?: string; cancelled?: boolean`。终态分支：failed → traceMap[msgId].error = payload.error ?? "unknown error"；cancelled → cancelled = true。
- 渲染：`TurnContainer` 完成态——error → body 后渲染 `.error-card`（danger border + max-height 120 + 内部滚动 + 文案「本轮失败：{error}」）；cancelled → `.turn-divider`（细线 + 居中 dim 文案「已中断」）。
- 参照：opencode timeline Error row / MessageDivider；重试按钮依赖 B4 错误分类，本单不做。
- 验收：断网发消息出错误卡；stop 取消后出分隔线。

### A10. assistant 回复复制按钮

- 改动：`TurnContainer` 完成态 agent-row 右侧（trace pill 旁）加 `.icon-btn` 复制钮（复用 codeblock-copy 样式），onClick `navigator.clipboard.writeText(body)` + 「已复制」反馈 1200ms。
- 验收：hover agent-row 出现复制钮，复制内容=markdown 原文。

### A11. Backlog（本批不做，记录在案）

消息列表虚拟化（@tanstack/react-virtual）、Mermaid/KaTeX（lazy）、turn 结束 diff 汇总行、图片 lightbox、markdown 渲染 worker 化、结果计数摘要（依赖 B4 result_count）。

## 3. Track B — 后端架构改良

### B1. prompt 修复 + 模型运行时信息注入

- 文件：`src/crates/assembly/core/src/agentic/agents/prompts/agentic_mode.md` line 58（`task鈥攖hree` → `task—three`，并全仓 grep `鈥` 清查其他 prompt 文件）。
- 注入：prompt_builder（`src/crates/assembly/core/src/agentic/agents/prompt_builder/`）在系统提示末尾追加运行时段：`# Runtime\n- Current model: {model_name}\n- Context window: {tokens}`。`model_name` 参数链已存在（`system_prompt_cache_identity(model_name: Option<&str>)`，agents/mod.rs:117）；context window 读 model stats（`data/token_usage/model_stats.json` 的加载处，`Loaded statistics for N models` 日志点）。
- 验收：新增/更新 prompt 单测；`cargo test -p northhing-core prompt` 绿；CLI `exec` 问「你用什么模型」回答与配置一致。

### B2. turn 阶段作为一等事件（状态只认后端）

- 依据：WorkBuddy 纪律「状态映射只认接口顶层，禁止前端推导」。当前前端用 think/tools/body 启发式推导状态行。
- 改动链：
  - `northhing_events::agentic`（`src/crates/contracts/events`）：AgenticEvent 加 `TurnPhase { session_id, turn_id, phase: Thinking|Generating|ToolUse { name } }`（在 turn 状态机/round 起步处发射：turn_init、stream 首 chunk、tool call started——这些点已有 W4-P 探针可参照）。
  - `northhing_kernel_api::events`：KernelEventDto 加对应变体。
  - `kernel_facade/mod.rs` agentic_event_to_dto 映射 + 单测。
  - `event_bridge.rs`：`chat-turn-phase` Tauri 事件；api.ts `onTurnPhase`；`TurnTrace.tsx` statusLabel 改消费后端相位（启发式作为缺相位的降级）。
- 验收：`cargo check --workspace` + `--features product-full`；facade 映射单测；GUI 状态行与后端日志相位一致。

### B3. facade 生命周期 follow-ups（judge 遗留）

- `KernelEventsApi::subscribe_events` 返回 `Result<SubscriptionId, KernelError>`：kernel-api trait 签名改；`kernel_facade/mod.rs:494-500` sentinel `"not-initialized"` 删除改 Err；调用点 `event_bridge.rs:126` 同步。magic string 消灭。
- `TurnInputDto` 加 `workspace_path?: Option<String>`；`submit_turn`（kernel_facade/mod.rs:399-）优先 dto 值，缺省回落现有 resolve+default 链。
- 验收：`cargo check --workspace` + product-full；`cargo test -p northhing-core --features product-full --lib kernel_facade::` 全绿。

### B4. 事件契约 enrichment

- `ToolCallDto` 加 `result_count?: number`（core ToolEventData 可提取时填；不可得不填）。
- `KernelEventDto::TurnState` failed 时加 `error_kind: "recoverable" | "fatal"`：分类规则 v1——transport/timeout/abort 类 = recoverable，其余 = fatal（参照 WorkBuddy 错误分层：只有 recoverable 前端才显示重试钮）。
- 前端联动（A9 之后）：错误卡按 error_kind 显示/隐藏「重试」钮（重新 send 同文本）。
- 验收：facade 单测覆盖两分类；GUI 断网错误卡出现重试钮。

### B5. turn 代际保护（genRef 模式）

- 依据：WorkBuddy「restart 自增 genRef，每 await 后比对，旧流程不碰新 state」。
- 改动：`agentic/coordination` 的 turn 生命周期（`scheduler/scheduler_turn/turn_submit.rs`、`dialog_turn/`）加 generation counter：submit/cancel/restart 时自增；turn 执行循环关键 await 点（round 起步、stream 消费、finalize）比对，不符即静默返回。
- 验收：新增并发 submit/cancel 测试（旧 turn 不写终态）；现有 `cargo test -p northhing-core --lib` 108+ 全绿。

### B6. K3 ROI 闸门（待用户拍板）

- 数据：增量实测 31.36s/31.49s vs 基线 29.86s/29.58s；目标 14.93s 未达。GO → 按北极星 §K3 续行（先补 HIGH-fan-out 探针）；NO-GO → 记录原因关线，编译痛点转交 K4 候选手段（cargo 配置/sccache/拆 crate 直接做）。

### B7. builtin skills 加载修复（pre-existing）

- 症状：CLI 启动 20+ 条 `Failed to parse SKILL.md ... Failed to capture content`（`%APPDATA%\northhing\skills\.system\gstack-*`）。
- 排查：复现 → 定位 parse 点（skill_tool/skill 加载器）→ 修 schema 兼容或剔除损坏缓存。与 C4 技能锻造强依赖。
- 验收：CLI 启动零 parse 失败日志；skills 列表可用。

### B8. 两个 pre-existing 测试失败修复

- `turn_batch::load_session_tail_turns`、`skill_tool::remote_call_loads`（clean main 可复现，与本季工作无关）。
- 验收：`cargo test -p northhing-core --lib` 全绿。

## 4. Track C — core agent + subagent 架构

### C1. agent 身份重写（等用户拍板文案）

- 文件：`src/crates/assembly/core/src/agentic/agents/prompts/agentic_mode.md:1-2`（身份段）及共享模板的其他 modes（claw/cowork/debug/multitask/plan 均继承 `SHARED_CODING_MODE_PROMPT_TEMPLATE`，检查各自 prompt 文件身份段）。
- 方向（草案，待拍板）：northhing = 通用个人 agent；编程/Shell/文件/浏览器是后台能力，不对人类暴露 IDE/编程概念；删除 pair-programming 框架与 "open files/cursor/linter" 等 IDE 语境（line 2 整段）。
- 验收：CLI `exec` 问「你是谁」回答符合新定位；prompt 相关单测更新。

### C2. 自我成长 Phase 1：Episode Log（经验层）

- 新模块：`src/crates/assembly/core/src/agentic/episodes/`（mod.rs + writer.rs + store.rs）。
- 数据：`Episode { turn_id, session_id, workspace_slug, task_summary (首条 user 消息截 120), tools_used: Vec<{name, ok}>, failures: Vec<{tool, error, repair?: string}>, outcome: Completed|Failed|Cancelled, duration_ms, ts }`。
- 写入点：`execution/turn_finalize.rs`（turn 终态唯一收敛点）。存储：`dirs::data_dir()/northhing/episodes/<workspace-slug>.jsonl`（slug 复用现有带哈希规则），append-only，单文件 cap 5MB 轮转。
- kernel-api 透出：`KernelMemoryApi::list_episodes(workspace?) -> Vec<EpisodeDto>` + facade 实现 + desktop-tauri 暂不接 UI。
- 验收：写入/读取/轮转单测；真实一次 turn 后剧集落盘；`cargo check --workspace` + product-full。

### C3. 结构化记忆（facts.jsonl）

- 现状：`service/agent_memory/auto_memory.rs`（`ensure_workspace_memory_files_for_prompt`）只维护 memory.md 索引（200 行）+ topic 文件（30 个）并注入 prompt。
- Phase 1：`Fact { id, text, provenance: {session_id, turn_id}, confidence: low|med|high, scope: workspace|global, created_at }`，存 `memory/facts.jsonl`（同目录）。写入与 C2 同一蒸馏点（turn_finalize 提取「决策/偏好/纠正」类句子——先用规则提取，不接 LLM 蒸馏）。
- 注入面：prompt 注入改为 facts（按 scope 过滤 + token 预算截断）+ memory.md 兼容保留（不删）。
- 验收：fact 写入/检索/预算截断单测；现有 memory prompt 测试更新。

### C4. 技能锻造闸门（聚合 + 候选，不自动启用）

- 依赖：C2 数据 + B7 修复。
- 聚合：episodes 中同 `(tool, error_pattern)` 出现 N≥3 且有 repair → 生成候选 skill 文件（`%APPDATA%\northhing\skills\candidates\<name>.md`，SKILL.md schema），audit log 记录生成依据（episode id 列表）。
- 边界：候选不自动加载生效——操作免确认，但"能力诞生"走人工审核（基线边界，需用户确认此解释）。
- 验收：聚合单测；候选文件通过 B7 后的 loader 解析。

### C5. subagent 精细化

- 现状：`agents/definitions/subagents/{explore,file_finder,general_purpose,research_specialist,computer_use}.rs` + `custom/subagent.rs` + `agentic/subagent_runtime/`；D 批遗留「subagent 路径不发 DialogTurnCompleted」已枚举无回归。
- C5a 委派指引：主 agent prompt（agentic_mode.md）加委派矩阵段——何时委派（独立侦察/大批量搜索/长串机械操作）vs 自己做（小改动/需连续上下文判断）。纯 prompt 改动。
- C5b 结果传递盘点：subagent_runtime 结果回传路径代码走查 + 如需在 `so_lifecycle/cleanup.rs` 补发事件（D 批遗留指定的挂点）。
- C5c 新专用类型（设计稿先行）：`test_agent.rs`、`refactor_agent.rs`，复制 `explore.rs` 定义模式 + 各自 prompt 文件。
- 验收：cargo check；委派链冒烟（主 agent 委派 explore 完成一次搜索并回报）。

### C6. 工具调用效率（设计稿先行）

- 并行：`execution/round_executor/` + `round_subhandlers/` 现状盘点 → 只读类工具并行白名单（Read/Grep/Glob/WebFetch 类），写操作串行不变。防 loop 硬编码禁令（AGENTS.md agent loop 节）适用。
- 流式结果：ExecCommand 长输出增量事件（`ToolEventData` 加 output_delta 或新变体），前端 ToolSection detail 流式追加。
- 产出：设计稿 + judge 评审后再立项实施。

### C7. 上下文管理强化（设计稿先行）

- 现状：`execution/compression.rs`、`compress_run.rs`、`compress_summary.rs`、`token_pressure.rs`。
- 方向：文件内容懒加载（Read 分段+续读协议）；compaction 保留决策点（接 C2 蒸馏产物作为压缩后上下文的一部分）；摘要回溯（压缩时生成可回溯摘要块）。
- 产出：设计稿 + judge 评审后再立项实施。

## 5. 依赖图与建议批次

```
B1 B7 B8          → 立即可做（无依赖，core 快修批）
A1..A10           → 立即可做（A5 需 src-tauri 改动；A9 的错误分类依赖 B4 才有重试钮，可先无钮）
B3                → 独立，facade 批
B2                → 独立（事件链），B4 依赖 B2
C2                → 独立（core），C3/C4 依赖 C2；C4 另依赖 B7
C5a/C5b           → 独立（prompt/走查），C5c 依赖设计稿
C6/C7             → 设计稿批（judge 评审后立项）
B5                → 建议与 C 线同步（多会话/并发增强时价值最大）
B6                → 等用户闸门
F2 设置页          → A 线收官后（设置页要装的项随 A/B/C 增加：模型显示、记忆查看、episodes 查看）
```

建议执行序：**B1+B7+B8 → A 全线 → B2/B3/B4 → C2 → C3/C5 → C1（等文案）→ C4/C6/C7 设计稿 → B6 闸门 → F2**。

## 6. 评审待定项

1. **C1 身份文案**：由编排者出草案 vs 用户直接给方向？
2. **C4 边界**：技能候选不自动启用、走人工审核——与「全免确认」基线的这个解释是否接受？
3. **B6 K3 闸门**：GO（续按北极星拆 core）vs NO-GO（转 K4 替代手段）。
4. **A5 技术选型**：`tauri-plugin-opener`（推荐，正路）vs 仅 `window.open` 降级（零依赖但行为不可控）。
5. **B5 时机**：genRef 现在做 vs 多会话时代再做。
6. **C6 并行白名单**：只读工具并行的范围界定（哪些算"只读"——按 ToolContract 的 readonly 标记还是手工清单？）
7. **A 线是否拆批派遣**：10 单一批（推荐，m27hs 逐单）vs 拆两批。

## 7. 附：证据文件索引

- opencode UI：`packages/app/src/pages/session/timeline/message-timeline.tsx`、`packages/session-ui/src/components/basic-tool.tsx`、`packages/ui/src/components/text-shimmer.css`、`packages/app/src/components/prompt-input/history.ts`（GitHub dev 分支）。
- WorkBuddy：本地 asar 提取 `colleague-chat-page-BlXlFORB.js` / `colleague-chat-page-B1c9vUfN.css`（提取于 2026-07-21，Temp 目录；注释含状态映射纪律/草稿/genRef/artifact 对账等设计要点）。
- QoderWork：本地 asar 提取 `index-RKbJenAa.js`（StatusLine 时态切换、结果计数）、`chat-markdown-renderer-oDIhfgNI.js`（KaTeX/Mermaid/lazy）。
- 本仓锚点：K0 报告 `.opencode/sdd/kernel/k0-report.md`；K2 验收与 fix 链 `git log 5018aa4..HEAD`；`agentic_mode.md:1,58`；`agentic/insights/`；`service/agent_memory/auto_memory.rs`。
