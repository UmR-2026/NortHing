# Northhing 产品方向与用户体验探索报告

> 探索时间：2026-07-21
> 探索者：subagent (exploratory reviewer)
> 仓库：E:\agent-project\northing
> 方法：read / grep / git show，无打分，纯探索性观察

---

## 1. 产品形态演进

### 1.1 v0.1.0 的承诺 vs 现实

PRD-v0.1.0 描绘的产品形态是：**Slint + Material 桌面壳**，纯桌面 GUI 应用，CLI 仅作为内部机制。v0.1.0-human-usable 发版说明（2026-07-16）也确认"桌面 GUI (Slint + Tauri) 尚未在所有平台上验证"。

但实际上仓库里存在**两套桌面应用**：

1. **`src/apps/desktop/`** — Slint 桌面壳（原定方案），包含完整的 UI 视图（`ui/main.slint`、`ui/views/`、`ui/components/`）、app_state 管理（callbacks_lifecycle、callbacks_settings、sessions、inspector 等）。这是 v0.2.10 发版说明中描述的那个有 Settings 面板、Welcome 流、错误通道的版本。
2. **`src/apps/desktop-tauri/`** — Tauri + React 桌面壳（新方案），2026-07-21 大量迭代，是当前活跃开发的产品前端。

### 1.2 Tauri+React 桌面 app 的 UX 长什么样

从 `src/apps/desktop-tauri/ui/src/` 的组件代码看，当前 UX 是一个**极简的聊天应用**：

**整体布局**：
- 顶部 `Header`：logo 圆点 + 可点击改名的 agent 名称 + drag region + 设置按钮 + 窗口控制（最小化/最大化/关闭）。Frameless 窗口设计，集成了暗色标题栏。
- 中间 `MessageList`：消息列表，user 消息为右对齐气泡，assistant 消息为 `TurnContainer`（包含 agent-row + 思考过程 + 工具调用 + markdown body）。
- 底部 `Composer`：自适应高度的 textarea，Enter 发送、Shift+Enter 换行，流式时显示停止按钮。
- `SettingsView`：全屏替换式设置页（非侧抽屉），有左侧 nav rail（通用/调试）。

**关键交互流程**：
1. 启动 → `getOrCreateLatestSession` 自动恢复最近会话
2. 用户输入 → 乐观追加 user 气泡 → `sendMessage` → 等待 `TurnState(started)`
3. 流式 → `chat-chunk` 事件逐字追加到 `streamingText` → `parseThink` 分离思考过程和正文
4. 工具调用 → `chat-tool` 事件实时显示工具名、摘要、进行中 badge
5. 完成 → `TurnState(completed)` → 乐观追加 assistant 消息 → 250ms 后 `getMessages` 对账

**思考过程展示**：`parseThink` 从 `<think>` 标签中提取思考内容，在 `ThinkSection` 中默认展开，body 开始后自动折叠（用户可手动切换）。

**工具调用展示**：`ToolSection` 显示工具名 + 摘要 + 进行中 badge（脉冲动画），完成后 badge 消失，detail 可展开。

**状态行**：`TurnContainer` 的 `agent-row` 显示 agent 头像 + 名称 + 实时状态（"深度思考 · 3s" / "执行工具 · read_file · 5s" / "生成回复中 · 8s"）+ 完成后显示耗时按钮（可折叠/展开 trace 区域）。

**空态**：无消息时显示 logo + "有什么可以帮你？" + 三个建议气泡（"帮我写一段代码"、"解释一个概念"、"跑一条命令"）。

### 1.3 与 Slint 版本的对比

v0.2.10 发版说明描述的 Slint 版本有：Settings 面板（Provider/Skills/MCP/Workspace/General 五个 tab）、Welcome 流（3步）、会话侧边栏、工作区切换器、Banner 错误通道。

Tauri+React 版本目前**只有**：聊天 + 极简设置页（通用占位 + 调试开关）+ agent 改名。大量 v0.2.10 已有的 Slint 功能尚未迁移到 Tauri 版本。

### 1.4 观察与疑问

- **形态迁移正在进行中**：从 Slint → Tauri+React，但远未完成。PRD 里的"Slint + Material"技术选型已被推翻，但 PRD 文档本身未更新。
- **为什么迁移？** 从北极星文档和三轨计划推断：Slint 的 C++ 依赖导致 GNU 工具链冲突（`STATUS_ENTRYPOINT_NOT_FOUND`），且 Slint 1.16 有线程安全问题（9 个 setter 需 `invoke_from_event_loop` 包装）。Tauri+React 更主流，生态更丰富。
- **但功能大幅回退**：Tauri 版本目前缺少会话管理 UI、Provider 配置、Skills 管理、MCP 管理、Workspace 切换——这些都是 v0.2.10 已有的功能。用户实际使用的只是一个聊天界面。
- **值得讨论**：这个迁移的 ROI 是否合理？Slint 版本已经"human-usable"了，为什么要推翻重来？

---

## 2. Agent-Kernel 架构

### 2.1 kernel-api facade 是什么

`northhing-kernel-api` 是一个新的 crate（`src/crates/contracts/kernel-api/`），定义了**宿主与内核之间的唯一公共 API 面**。它只包含 DTO、trait 和错误类型，不含业务逻辑。

核心 trait 分为：
- `KernelBootstrapApi` — 初始化/就绪检查
- `KernelSessionApi` — 会话 CRUD + 消息获取
- `KernelTurnApi` — 提交/停止 turn
- `KernelEventsApi` — 订阅事件流（TextChunk / TurnState / ToolCall / Banner / Error）
- `KernelSettingsApi` — 全局配置读写
- `KernelAgentsApi` — agent/subagent/skill 信息
- `KernelToolsApi` — 工具注册/执行
- `KernelUsageApi` — token 使用统计
- `KernelPlatformApi` — 终端/图片/健康检查/面板等平台功能

### 2.2 与 northhing-core 直接调用的本质区别

**之前**：5 个宿主 crate（desktop、desktop-tauri、cli、cli-internal、acp）直接 `use northhing_core::` 引用 core 内部的 `agentic::`、`service::`、`infrastructure::`、`util::` 等模块。desktop 单文件最高 27 处 `northhing_core::` 引用。

**问题**：core 任何一行改动 → 5 个宿主全量重编 → 宿主体量巨大（desktop 含 Slint 生成码），日常迭代被编译扇出拖死。而且没有稳定面，core 内部重构外溢到宿主。

**之后**：宿主只依赖 `kernel-api` facade（极薄 DTO + trait），core 内部实现 facade。core 内部改动不再外溢到宿主。facade 有量化约束：方法数 ≤ ⌈N×1.2⌉ = 53，代码量 ≤ 1500 行。

**本质区别**：从"宿主深入 core 内部模块"变为"宿主只经 facade 触达 core"。这是一个**编译隔离层** + **认知解耦层**。

### 2.3 演进方向

北极星文档的终态架构是：
- kernel = 扩展后的 `execution/agent-runtime`（scheduler/runtime/session_control/events）
- 一切以端口挂在 kernel 上：providers / tools / services / persistence
- assembly/core 退化为纯 composition root（只 new + 注册，无业务）

但 K3（kernel 下沉）有一个 ROI 闸门：如果编译收益在 K2（facade 切换）后已达成，K3 降级为"有空再做"。当前 K2 已完成（desktop-tauri 切 facade），K3 闸门待拍板。

### 2.4 观察与疑问

- **facade 已经落地但很年轻**：`commands.rs` 显示 desktop-tauri 已完全通过 `kernel_facade()` 调用，不再直连 core 内部。但 facade 的方法列表（53个）是否能覆盖 Slint 版本的全部需求？Slint 版本的 `callbacks_lifecycle.rs` 单文件 27 处引用，迁移到 facade 后是否会暴露面不足？
- **编译收益量化**：K0 基线增量 check 29.86s，K2 实测 31.36s/31.49s——**反而变慢了**。目标 14.93s 未达。这说明 facade 切换本身没有减少编译时间（合理：facade 是新增层不是减层），K3 才是真正减少编译的步骤。但 K3 是否值得做是个开放问题。
- **值得讨论**：facade 的 1500 行限制是否过于刚性？如果未来 F 线（设置页、Inspector、产物面板）需要更多面，20% 余量很快会用完。

---

## 3. 三轨计划

### 3.1 三条线概述

**Track A — 前端 UI 美学/功能改良**（10 单 + 1 backlog）：
- A1 状态文字 shimmer 扫光
- A2 回到底部悬浮按钮
- A3 输入历史上翻回忆
- A4 工具节标题结构化 + completed 收敛态
- A5 markdown 外链系统浏览器打开
- A6 重渲染 memo
- A7 每会话草稿持久化
- A8 草稿态懒建会话
- A9 turn 失败错误卡 + 已中断分隔
- A10 assistant 回复复制按钮

**Track B — 后端架构改良**（8 单）：
- B1 prompt 修复 + 模型运行时信息注入
- B2 turn 阶段作为一等事件
- B3 facade 生命周期 follow-ups
- B4 事件契约 enrichment
- B5 turn 代际保护（genRef 模式）
- B6 K3 ROI 闸门
- B7 builtin skills 加载修复
- B8 两个 pre-existing 测试失败修复

**Track C — core agent + subagent 架构**（7 单）：
- C1 agent 身份重写
- C2 自我成长 Phase 1：Episode Log
- C3 结构化记忆（facts.jsonl）
- C4 技能锻造闸门
- C5 subagent 精细化
- C6 工具调用效率（设计稿）
- C7 上下文管理强化（设计稿）

### 3.2 当前进展

从 git log 看（截至 82ea09f）：
- **B1a/B7（builtin skills mojibake 修复）**：✅ 已完成（46172ec、eed3da8）
- **B1b（模型运行时信息注入）**：✅ 已完成（101bc1f）
- **B8（pre-existing 测试失败修复）**：✅ 已完成（82ea09f）
- **A 线**：尚未开始（git log 中无 A1-A10 的 commit）
- **B2/B3/B4/B5**：尚未开始
- **C 线**：尚未开始
- **B6（K3 闸门）**：等用户拍板

### 3.3 观察与疑问

- **A 线是用户最直接感知的改进**，但还没开始。当前 Tauri 版本的 UX 确实缺少很多打磨：没有回到底部按钮、没有输入历史、没有草稿持久化、没有错误卡、没有复制按钮。这些都是"用了才知道缺"的细节。
- **C 线是最有想象力的方向**：Episode Log + 结构化记忆 + 技能锻造，这是在向"自我成长的 agent"演进。但这些都是设计稿阶段，离落地还远。
- **C1 agent 身份重写**是个关键问题：当前 `agentic_mode.md:1` 写的是 "You are northhing, an ADE (AI IDE) that helps users with software engineering tasks"，还有 "pair programming with a USER"。这与 PRD 的"隐藏 IDE 的通用 agent"定位直接冲突。而且 line 58 有编码损坏（`task鈥攖hree`）。
- **值得讨论**：三轨计划的执行序建议是 B1+B7+B8 → A 全线 → B2/B3/B4 → C2 → ...。但 A 线 10 单一批做完是否太重？是否应该优先做 A8（草稿态懒建会话）和 A9（错误卡），因为这两个直接影响用户信任？

---

## 4. 设置页面

### 4.1 当前实现

`SettingsView.tsx` 是一个**极简骨架**：

- 左侧 nav rail：两个选项（"通用"、"调试"）
- 右侧 body：返回按钮 + section 内容
- 通用 tab：标题 "通用" + "更多设置即将到来" 占位文字
- 调试 tab：一个 checkbox（调试面板开关）+ 说明文字

### 4.2 用户能配置什么

当前只能配置：
1. **Agent 名称**（通过 Header 的可点击名称，存储在 `%APPDATA%\northhing\config\desktop-ui.json`）
2. **调试面板开关**（在 Composer 下方显示事件日志）

### 4.3 对比 v0.2.10 Slint 版本

Slint 版本的设置页有五个完整 tab：AI 服务（Provider）、技能（Skills）、工具集（MCP）、工作文件夹（Workspace）、通用。Tauri 版本的设置页几乎为空。

### 4.4 Rust 侧配置

`commands.rs` 中的配置只有 `UiPrefsDto { agent_name: String }`。所有的 provider/skill/mcp/workspace 配置都还没接入 Tauri 前端——虽然 kernel-api facade 已经定义了 `KernelSettingsApi`、`KernelAgentsApi` 等 trait，但 Tauri commands 层没有暴露这些 API。

### 4.5 观察与疑问

- **设置页是占位符**。三轨计划提到 "F2 设置页 → A 线收官后（设置页要装的项随 A/B/C 增加）"，说明设置页的充实是个后续任务。
- **但用户现在无法配置 Provider**。这意味着用户无法在 Tauri 版本中添加/修改 API Key、切换模型。如果 `core` 的默认配置没有正确的 provider，应用就是不可用的。
- **值得讨论**：是否应该优先做一个最小的 Provider 配置 UI？没有这个，Tauri 版本无法独立使用。

---

## 5. 内置 skill 修复

### 5.1 这些 skills 是什么

`src/crates/assembly/core/builtin_skills/` 下有 21 个 skill 目录，分为几类：

1. **gstack 系列**（12 个）：gstack-autoplan、gstack-cso、gstack-design-consultation、gstack-design-review、gstack-document-release、gstack-investigate、gstack-office-hours、gstack-plan-ceo-review、gstack-plan-design-review、gstack-plan-eng-review、gstack-qa、gstack-qa-only、gstack-retro、gstack-review、gstack-ship — 这是一套**软件开发工作流技能**，覆盖了从计划评审到 QA 到发布到回顾的完整流程。从 SKILL.md 内容看，这些技能定义了非常详细的自动化工作流（例如 gstack-ship 有 1900+ 行的发布检查清单）。
2. **文档技能**（4 个）：docx、pdf、pptx、xlsx — Office 文档处理能力。
3. **其他**（5 个）：agent-browser、find-skills、memory、ppt-design、writing-skills。

### 5.2 为什么它们会被 embed

`build.rs` 使用 `include_dir!` 宏将整个 `builtin_skills/` 目录嵌入到 `northhing-core` 二进制中。`builtin.rs` 实现了安装逻辑：首次启动时将嵌入的 skills 解压到 `%APPDATA%\northhing\skills\.system\` 目录，使用 SHA-256 bundle hash 做增量更新（hash 不变则跳过）。

这些 skills 是 **agent 的能力扩展**——它们在运行时被 skill loader 读取，front matter（name/description/triggers）用于匹配用户意图，body 注入到 system prompt 中。

### 5.3 为什么会出现 mojibake

从 commit message 和修复脚本看：
1. **BOM 问题**：部分 SKILL.md 文件带 UTF-8 BOM（`\u{feff}`），`FrontMatterMarkdown::load_str` 的正则 `^---` 无法匹配 BOM 前缀。修复：在 `load_str` 中加了 `content.strip_prefix('\u{feff}')`。
2. **GBK mojibake**：文件内容中有 GBK 编码被错误解码为 UTF-8 的乱码字符（如 `鈥` 是 em-dash `—` 在 GBK→UTF-8 误读的结果）。这些乱码出现在多个 gstack skills 的正文和 front matter 中。修复：`scripts/fix-bom-and-gbk.js` 脚本批量修复。
3. **剩余变种**：eed3da8 修了第二轮 mojibake（U+9241 checkmark、retro middle-dots、dead mapping key），说明第一轮修复有遗漏。

### 5.4 根因推测

这些 skills 文件很可能是在 Windows 中文环境（GBK 代码页）下创建或编辑的，然后被 git 以 UTF-8 签出，导致编码损坏。`build.rs` 用 `include_dir!` 嵌入时，编译期的 `read_to_string` 按 UTF-8 解码，遇到 GBK 字节序列就产生 mojibake。front matter loader 的正则匹配 `^---` 对 BOM 和编码损坏都很敏感。

### 5.5 观察与疑问

- **gstack skills 是从哪来的？** 从内容看，它们引用了 "Task sub-agents"、"AskUserQuestion"、"TodoWrite" 等概念，风格类似 Cursor/Claude 的 agent skills。这些是项目原创的还是从外部引入的？
- **为什么 embed 而不是运行时读取？** 嵌入保证了 skills 随二进制分发，不需要额外文件。但代价是更新 skills 需要重新编译。对于一个快速迭代的项目，这可能不是最佳选择。
- **front matter loader 的健壮性**：修复后的 `load_str` 加了 BOM 剥离，但仍然用正则匹配 front matter。如果 SKILL.md 内容本身包含 `---`（如 markdown 分割线），正则会误匹配。这个风险存在但概率不高。
- **值得讨论**：21 个内置 skills 中，gstack 系列占了 12 个，都是重型软件开发流程技能。这些技能对于一个"通用个人 agent"（PRD 定位）是否都需要？还是应该按需安装？

---

## 总结性观察

### 产品处于什么阶段

Northhing 正处于一个**产品形态迁移的中间态**：
- Slint 版本功能更完整（v0.2.10 有完整设置页、Welcome 流、会话管理），但技术栈遇到瓶颈（Slint C++ 冲突、线程安全问题）。
- Tauri+React 版本是未来方向，目前只有聊天界面 + 极简设置页，缺少 provider 配置、会话管理、workspace 切换等关键功能。
- kernel-api facade 已落地，解决了编译扇出问题的第一刀（facade 隔离），但 K3（kernel 下沉）的 ROI 闸门尚未拍板。

### 最值得讨论的点

1. **Tauri 版本如何快速补齐 Slint 版本已有的功能？** 特别是 Provider 配置——没有它，应用不可用。
2. **C1 agent 身份重写**：`agentic_mode.md` 还在说 "AI IDE" 和 "pair programming"，与产品定位冲突。这个文件是 agent 的核心 system prompt，身份认知错误会影响所有交互。
3. **三轨计划的执行优先级**：A 线（UI 打磨）vs C 线（agent 架构演进）的取舍。A 线是短期用户感知，C 线是长期产品差异化（Episode Log + 结构化记忆 + 技能锻造是真正让 northhing 不同于"又一个 ChatGPT clone"的方向）。
4. **内置 skills 的定位**：21 个 skills 中 12 个是 gstack 开发流程技能。这些技能定义了非常详细的工作流（gstack-ship 有 1900+ 行），但它们假设了一个软件开发场景。对于一个"通用个人 agent"，这些 skills 的适用面需要重新审视。
5. **编译时间**仍然是个核心痛点。K2 facade 切换后增量 check 反而从 29.86s 变成 31.36s。如果 K3 不做，编译收益主要在"core 内部改动不外溢"而非"编译变快"。这个收益是否值得 facade 的复杂度？

---

> 本报告为探索性观察，不含评分或评判。所有疑问均为开放讨论项。
