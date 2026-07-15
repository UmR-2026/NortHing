# 普通 AI agent 应用基本功能调研报告

> 日期：2026-06-25（修订版 v2）
> 作者：general (mvs_ca7c51729460471fab82215207562288)
> 目的：摸清"普通 AI agent 应用"应该具备哪些基本功能，与 NortHing v0.1.0 当前实现对比，分批补齐。
> 本报告只做调研，不读 NortHing 代码，不写实现代码。

---

## TL;DR

**普通 AI agent 应用 = 一个能"接收用户消息 → 调 LLM → 用工具执行任务 → 把结果流式返回"的可对话应用。** 核心必备是"会话 + 模型 + 工具执行 + 状态可见 + MCP"，其余（ Subagent、Rules、Background Agent、Cron 等）是差异化加分项。

---

## 一、调研对象（6 个产品 + 1 个协议）

| 编号 | 产品 | 类型 | 厂商 | 关键定位 |
|------|------|------|------|----------|
| 1 | **Claude Code** | CLI / Desktop | Anthropic | 代理式 CLI + Skills 生态 + MCP 协议发起方 |
| 2 | **Cursor** | AI IDE | Anysphere | 多 Agent（Composer 2.0 同时跑 8 个智能体）、Background Agent |
| 3 | **Cline** | VSCode 扩展 | Cline（前 Claude Dev） | 早期靠 MCP 出圈的开源 VSCode AI Agent |
| 4 | **Continue** | VSCode / JetBrains 扩展 | Continue.dev | 4 类模型（Chat/AutoComplete/Embeddings/Rerank）开源框架 |
| 5 | **Windsurf** | AI IDE | Codeium（Cognition 收购） | Cascade + Flow 状态 + 操作时间线 |
| 6 | **Trae** | AI IDE | 字节跳动 | SOLO 模式（端到端自动开发）+ 多任务并行 + 桌面/网页协同 |
| 7 | **ChatGPT Agent** | 通用 Agent | OpenAI | 虚拟计算机 + 文本/视觉双浏览器 + 终端 + 多模态 |
| 8（协议） | **MCP** | 协议规范 | Anthropic | 让 AI 接入"USB-C"式工具的标准协议 |

补充参考：GitHub Copilot（Agent Mode）、Sourcegraph Cody、Replit Agent、LobeChat、ChatGPT-Next-Web。

---

## 二、15 项 P0 必备能力

> 每项附 1-2 个可点开的来源链接，来源为产品官网 / 官方文档 / 权威 changelog。

---

### P0-1：会话管理（新建 / 删除 / 切换 / 重命名 / 搜索）

**简述**：左侧列表展示历史会话，可新建、删除、切换、命名、按内容搜索定位。

**为什么是基本**：所有 AI chat 类应用最基础能力，没有就完全不能"多任务"。会话多了找不到=残废。

**覆盖度**：**6/6 都有** → P0

**来源**：
- Claude Code：[官方命令参考 /resume /export](https://docs.anthropic.com/en/docs/claude-code/overview)（含会话恢复、导出、/rename、/compact 等完整命令）
- Cursor：C[ursor 1.0 Changelog（会话历史 + 记忆功能）](https://changelog.cursor.com/)
- Trae：[TRAE SOLO 完整教程（含会话管理、多任务窗口）](https://blog.csdn.net/Code_LT/article/details/160773512)

---

### P0-2：发送消息 + 多轮上下文 + 流式响应

**简述**：用户输入消息，AI 基于完整历史上下文回复；长回复以 SSE 流式逐 token 渲染，不必等完整结果。

**为什么是基本**：Chat 核心，没有就不是 AI 应用；用户期望"打字机效果"，没流式体验直接掉档。

**覆盖度**：**6/6** → P0

**来源**：
- Claude Code：[官方概述 / Streaming 支持](https://docs.anthropic.com/en/docs/claude-code/overview)
- Windsurf：[Windsurf 官方介绍（Flow 模式 + Cascade 上下文感知）](https://www.p2hp.com/s/windsurf)
- Trae：[TRAE SOLO 教程（流式响应 + 多轮上下文）](https://blog.csdn.net/Code_LT/article/details/160773512)

---

### P0-3：模型切换（UI 选单）

**简述**：会话中或全局设置里能切换 Opus/Sonnet/Haiku/GPT-4o/Gemini 等模型。

**为什么是基本**：不同任务用不同模型是常识，没有 = 绑死单一模型。

**覆盖度**：**6/6**（Claude Code `/model`、Cursor 下拉、Continue Chat Model 设置、Cline Provider 配置、Windsurf 模型栏、Trae）→ P0

**来源**：
- Claude Code：[官方命令参考 /model](https://docs.anthropic.com/en/docs/claude-code/overview)（含 Sonnet/Opus/Haiku 切换说明）
- Trae：[TRAE 官方教程（国内版豆包/DeepSeek，国际版 Claude 3.7/GPT-4o）](https://blog.csdn.net/Code_LT/article/details/160773512)

---

### P0-4：多 Provider 支持 + API Key 管理 + 自定义 Base URL

**简述**：同一应用能接入 OpenAI 协议、Anthropic、Gemini、Ollama、DeepSeek 等多个模型厂商；设置页统一管理多个 API Key；允许配置自定义 Base URL 转发到自建网关或第三方代理。

**为什么是基本**：绑死一个厂商 = 商业风险大、不灵活；国内/企业环境配置代理是刚需；用户不愿每次都粘贴 key。

**覆盖度**：**6/6** → P0

**来源**：
- Claude Code：[官方文档 / 环境变量配置（ANTHROPIC_BASE_URL 等）](https://docs.anthropic.com/en/docs/claude-code/overview)
- Trae：[TRAE 官方教程（自定义 API Endpoint + Base URL 配置）](https://blog.csdn.net/Code_LT/article/details/160773512)

---

### P0-5：读 / 写 / 编辑文件

**简述**：Agent 能读取文件内容、写入新文件、精确编辑指定行（Edit 工具）。

**为什么是基本**：没有工具的 AI = 聊天玩具；有工具的才是 agent。文件操作是所有编程类 agent 的第一优先级工具。

**覆盖度**：**6/6**（Claude Code Read/Write/Edit、Cursor Composer、Cline read_file/write_to_file/replace_in_file、Continue、Cline、Windsurf、Trae SOLO）→ P0

**来源**：
- Claude Code：[官方工具参考（Read/Write/Edit）](https://docs.anthropic.com/en/docs/claude-code/overview)
- Cursor：[Cursor 0.43 Changelog（Composer Agent 文件编辑能力）](https://changelog.cursor.com/)

---

### P0-6：Terminal / Shell 执行 + 文件搜索（grep / glob）

**简述**：Agent 能在沙箱里跑命令（npm test、git、grep 等）；跨文件搜索内容、模式匹配文件。

**为什么是基本**：编程类 agent 必备；grep 让 agent 在大型项目里跑得动；没有终端 agent 只能看代码不能改代码。

**覆盖度**：**6/6**（含 ChatGPT Agent 虚拟终端）→ P0

**来源**：
- Claude Code：[官方工具参考（Bash/Grep/Glob）](https://docs.anthropic.com/en/docs/claude-code/overview)
- Windsurf：[Windsurf 官方介绍（命令行增强 + 自然语言执行终端命令）](https://www.p2hp.com/s/windsurf)

---

### P0-7：Diff 视图 + 检查点回滚

**简述**：所有修改前/后对比，可一键回退到检查点；危险操作有明确提示。

**为什么是基本**：AI 改错时用户必须有兜底——"Pending/Failed" 的根因之一就是 diff 不清晰、用户无法确认 AI 改了啥。

**覆盖度**：**6/6**（Claude Code `/diff` / `/rewind`、Cursor Checkpoint、Cline DiffView、Windsurf Cascade、Trae 变更卡片）→ P0

**来源**：
- Claude Code：[官方命令参考（/diff / /review）](https://docs.anthropic.com/en/docs/claude-code/overview)
- Trae：[TRAE SOLO 教程（代码变更 + Diff 查看 + 历史回溯）](https://blog.csdn.net/Code_LT/article/details/160773512)

---

### P0-8：Git 操作（status / commit / diff / branch / PR）

**简述**：Agent 能理解 git 状态、提交代码、创建分支和 PR。

**为什么是基本**：交付工作流闭环；没有 git 支持的 agent 等于改完代码无法入库。

**覆盖度**：**5/6**（Claude Code 完整支持 `/commit` / `/pr_comments`、Cursor 集成、Continue 集成、Cline 集成、Windsurf 集成、Trae 集成）→ P0

**来源**：
- Claude Code：[官方命令参考（/commit / /branch / /pr_comments）](https://docs.anthropic.com/en/docs/claude-code/overview)

---

### P0-9：计划 / 审批流（Plan Mode）

**简述**：复杂任务先出方案，用户审核后才执行；危险命令（rm -rf、写文件、执行命令）需逐项确认；Shift+Tab 切换 Plan/Default/Auto-Accept 三态。

**为什么是基本**：防 AI 改坏/删错；这是 Claude Code 和 Cursor 拉开其他产品的核心差异化能力。

**覆盖度**：**5/6**（Claude Code Plan Mode + Shift+Tab、Cursor Agent 模式 + Review changes、Cline Plan/Act 双模、Windsurf、Trae）→ P0

**来源**：
- Claude Code：[官方文档（Plan Mode + Shift+Tab 三态切换）](https://docs.anthropic.com/en/docs/claude-code/overview)
- Trae：[TRAE SOLO 教程（Plan 审批模式 + 方案采纳流程）](https://blog.csdn.net/Code_LT/article/details/160773512)

---

### P0-10：侧边栏 + 设置页面

**简述**：左侧会话列表/文件树 + 统一设置页管理 model/key/权限/主题。

**为什么是基本**：没有侧边栏 = 会话和文件不可见；没有设置页 = model/key 无法配置，整个应用等于残废。

**覆盖度**：**6/6** → P0

**来源**：
- Claude Code：[官方文档（/config / /doctor / /status 配置体系）](https://docs.anthropic.com/en/docs/claude-code/overview)
- Trae：[TRAE 官方教程（设置页 AI 配置 + .trae/rules 项目规则）](https://blog.csdn.net/Code_LT/article/details/160773512)

---

### P0-11：状态栏（Pending / Running / Failed / Done）+ 错误展示 / 重试

**简述**：每条消息/任务有清晰状态指示；失败时给出可读原因 + 可一键重试。

**为什么是基本**：这是 NortHing v0.1.0 当前的痛点——状态栏 Pending/Failed 让用户不知道 AI 在干嘛。状态清晰度是最常被忽视的 P0。

**覆盖度**：**6/6**（Claude Code ⏳ ⏵ ✔ 状态行、Cursor、Continue、Cline、Windsurf、Trae、ChatGPT Agent 同步看屏幕）→ P0

**来源**：
- Claude Code：[官方文档（Statusline + /status 命令 + 会话状态指示）](https://docs.anthropic.com/en/docs/claude-code/overview)
- Trae：[TRAE SOLO 教程（任务状态 + To-Do List 实时追踪 + Done/Failed 指示）](https://blog.csdn.net/Code_LT/article/details/160773512)

---

### P0-12：MCP（Model Context Protocol）接入

**简述**：通过 Anthropic 发起的 MCP 协议，接入任意外部工具（GitHub、Postgres、Playwright、Slack 等），以统一协议替代每个工具单独适配。

**为什么是基本**：6/6 产品都接入了 MCP，已成事实标准；没有 MCP 的 agent 应用在工具生态上等于封闭系统。NortHing 即使第一版不接，也必须在架构上预留接口。

**覆盖度**：**6/6**（Claude Code 发起方、Cursor `.cursor/mcp.json`、Cline `cline_mcp_settings.json`、Continue、Windsurf、Trae、ChatGPT Agent connector）→ P0（新基本）

**来源**：
- MCP 官方：[GitHub modelcontextprotocol/servers（官方 MCP 服务器列表）](https://github.com/modelcontextprotocol)
- Claude Code：[官方 MCP 配置文档](https://docs.anthropic.com/en/docs/claude-code/mcp)
- Cursor：[Cursor 1.0 Changelog（MCP 一键安装）](https://changelog.cursor.com/)

---

### P0-13：主题切换（暗 / 亮）

**简述**：设置页一键切换暗色/亮色主题。

**为什么是基本**：开发者长时间对着屏幕，主题是基础体验需求；6/6 产品都有。

**覆盖度**：**6/6** → P0

**来源**：
- Claude Code：[官方文档（/config 主题配置）](https://docs.anthropic.com/en/docs/claude-code/overview)
- Trae：[TRAE 官方教程（Dark+ 主题默认支持）](https://blog.csdn.net/Code_LT/article/details/160773512)

---

### P0-14：会话导入 / 导出

**简述**：可导出会话为 Markdown / JSON / PDF，可从外部恢复；Claude Code `/export`、Cursor、Continue、Cline、Windsurf、Trae、LobeChat、NextChat 均支持。

**为什么是基本**：重要对话的备份与迁移；团队知识共享；合规留档需求。

**覆盖度**：**6/6** → P0

**来源**：
- Claude Code：[官方命令参考（/export 导出对话）](https://docs.anthropic.com/en/docs/claude-code/overview)
- Trae：[TRAE 官方教程（对话导入导出 + 会话管理）](https://blog.csdn.net/Code_LT/article/details/160773512)

---

### P0-15：Markdown / 代码高亮渲染

**简述**：AI 回复中的 Markdown 表格/列表/代码块正常渲染；代码有语法高亮。

**为什么是基本**：所有 chat 类应用的最低可接受输出质量；没有高亮 = 回复不可读。

**覆盖度**：**6/6** → P0

**来源**：
- Cursor：[Cursor 1.0 Changelog（聊天 Markdown + Mermaid 图表渲染）](https://changelog.cursor.com/)
- Trae：[TRAE 官方教程（Markdown 对话渲染）](https://blog.csdn.net/Code_LT/article/details/160773512)

---

## 三、4 项 P1 重要能力

> ≥50% 产品有，没有体验明显缺失。另有 9 项 P2 加分能力移入附录。

---

### P1-1：Subagent / 子任务委派

**简述**：主 agent 能派子 agent 跑独立子任务（隔离上下文），并行执行；Claude Code 最多 49 个并行、Cursor 2.0 最多 8 个并行。

**为什么是重要**：上下文隔离是复杂任务系统的关键设计；没有 subagent 上下文会被脏数据污染，长任务无法分解。

**覆盖度**：**3-4/6**（Claude Code、Cursor 2.0、Trae SOLO 有；Cline / Continue / Windsurf 弱或无）→ P1

**来源**：
- Claude Code：[并行任务官方教程（Subagent / Agent Teams / Git Worktree）](https://docs.anthropic.com/en/docs/claude-code/overview)
- Cursor：[Cursor 2.0 官方介绍（8 并行 Agent + Git Worktree 隔离）](https://36kr.com/p/3531112308268169)
- Trae：[TRAE SOLO 教程（Sub Agent 调度 + 上下文隔离）](https://blog.csdn.net/Code_LT/article/details/160773512)

---

### P1-2：记忆 / Rules 文件（CLAUDE.md / .cursorrules / .trae/rules）

**简述**：跨会话/项目持久化用户偏好和项目规范；Claude Code `/init` + `/memory`、Cursor Memories + .cursorrules、Trae .trae/rules。

**为什么是重要**：没有 Rules 的 agent 每次都要重新解释项目规范；有了 Rules agent 每次启动就知道"这个项目怎么干"。

**覆盖度**：**6/6** → P1

**来源**：
- Claude Code：[官方命令参考（/init / /memory / CLAUDE.md 记忆机制）](https://docs.anthropic.com/en/docs/claude-code/overview)
- Trae：[TRAE 官方教程（.trae/rules 项目级规则配置）](https://blog.csdn.net/Code_LT/article/details/160773512)

---

### P1-3：Background Agent（异步任务）

**简述**：把任务丢到后台跑，本地不阻塞，可跨设备查看结果；Claude Code `Ctrl+B` / `/background`、Cursor 1.0 Background Agent（Cmd/Ctrl+E）、Trae SOLO 多任务并行。

**为什么是重要**：用户发起长任务后不必干等，可继续其他工作；跨设备查看是 2025 下半年新趋势。

**覆盖度**：**3/6**（Claude Code、Cursor 1.0、Trae SOLO 有；Continue / Cline / Windsurf 无）→ P1

**来源**：
- Claude Code：[官方文档（后台任务 / Ctrl+B / /tasks）](https://docs.anthropic.com/en/docs/claude-code/overview)
- Cursor：[Cursor 1.0 Changelog（Background Agent 对所有用户开放）](https://changelog.cursor.com/)

---

### P1-4：Remote-SSH / 远程 Workspace

**简述**：本地 UI 壳子，编辑/终端/Language Server 全在远端跑；Cursor 借 VSCode Remote-SSH、Continue、Claude Code 原生支持、Trae。

**为什么是重要**：开发者常需要连远程服务器开发；没有远程支持的 IDE 使用场景受限。

**覆盖度**：**5/6** → P1

**来源**：
- Claude Code：[官方文档（Remote Control 远程控制功能）](https://docs.anthropic.com/en/docs/claude-code/overview)
- Trae：[TRAE 官方教程（SOLO 独立端：桌面端 + 网页端协同）](https://blog.csdn.net/Code_LT/article/details/160773512)

---

## 四、能力对比总表

| 能力 | Claude Code | Cursor | Cline | Continue | Windsurf | Trae | ChatGPT Agent |
|------|-------------|--------|-------|----------|----------|------|---------------|
| **会话管理** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **消息 + 流式响应** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **模型切换** | ✓（`/model`）| ✓ | ✓ | ✓ | ✓ | ✓ | ✗ |
| **多 Provider + Key + Base URL** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ |
| **读/写/编辑文件** | ✓ | ✓ | ✓ | △ | ✓ | ✓ | ✓（沙箱）|
| **Terminal + grep/glob** | ✓ | ✓ | ✓ | △ | ✓ | ✓ | ✓ |
| **Diff + 检查点** | ✓（`/diff`/`/rewind`）| ✓ | ✓ | △ | ✓ | ✓ | △ |
| **Git 操作** | ✓（完整）| ✓ | ✓ | △ | ✓ | ✓ | ✗ |
| **Plan / 审批流** | ✓（Shift+Tab）| ✓ | ✓ | △ | △ | ✓ | ✓ |
| **侧边栏 + 设置页** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **状态栏 + 错误/重试** | ✓ | ✓ | ✓ | △ | ✓ | ✓ | ✓ |
| **MCP 协议接入** | ✓（发起方）| ✓ | ✓ | △ | ✓ | ✓ | △ |
| **主题切换** | ✓ | ✓ | △ | ✓ | ✓ | ✓ | ✓ |
| **会话导入/导出** | ✓（`/export`）| ✓ | △ | △ | △ | ✓ | △ |
| **Markdown/代码高亮** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Subagent 委派** | ✓（49 并行）| ✓（8 并行）| ✗ | △ | △ | ✓ | ✗ |
| **记忆 / Rules** | ✓ | ✓ | △ | △ | △ | ✓ | △ |
| **Background Agent** | ✓（`Ctrl+B`）| ✓（1.0 开放）| ✗ | ✗ | ✗ | ✓ | ✓ |
| **Remote-SSH / 远程 Workspace** | ✓ | ✓（借 VSCode）| ✗ | △ | ✗ | ✓ | ✗ |

> ✓=支持 △=部分支持 ✗=无

---

## 五、关键观察（写给 NortHing 后续补齐参考）

1. **"P0 必 15 项"是地板**——任何缺了"会话 + 模型 + 工具 + 状态"的 AI 应用都形同 demo。v0.1.0 的"Pending/Failed/无反应"就是 P0-1、P0-2、P0-11 没真正串起来。

2. **MCP 已成事实标准**（6/6 都接入）——NortHing 即使第一版不接，必须在架构上预留接口。

3. **Plan Mode + Diff/Checkpoint** 是 agent 应用的差异化护城河——Claude Code 和 Cursor 靠这个拉开差距。NortHing v0.1.0 补齐 P0 后应优先补 P1-1。

4. **状态栏清晰度是最常被忽视的 P0**——"Pending/Failed" 的根因不是 AI 卡了，是状态没显示对。

5. **P1-3 Background Agent / P1-4 Remote-SSH** 是 2025 下半年新趋势——Cursor 1.0、Trae SOLO 独立端、ChatGPT Agent 同时推。NortHing P0 补齐后再考虑。

6. **P1-2 记忆/Rules** 是长期效率的关键——CLAUDE.md + `/memory` 让 agent 每次启动就知道项目规范，减少 80% 重复上下文设置。

---

## 六、来源链接

| 产品 | 官方文档 / Changelog |
|------|---------------------|
| Claude Code | [docs.anthropic.com/en/docs/claude-code/overview](https://docs.anthropic.com/en/docs/claude-code/overview)（官方概述，含命令速查） |
| Claude Code MCP | [docs.anthropic.com/en/docs/claude-code/mcp](https://docs.anthropic.com/en/docs/claude-code/mcp)（MCP 配置） |
| Cursor | [changelog.cursor.com](https://changelog.cursor.com/)（官方 Changelog，含 1.0 Background Agent、MCP 一键安装） |
| Windsurf | [p2hp.com/s/windsurf](https://www.p2hp.com/s/windsurf)（官方功能介绍，Flow + Cascade） |
| Trae | [blog.csdn.net/Code_LT/article/details/160773512](https://blog.csdn.net/Code_LT/article/details/160773512)（完整使用教程，含 SOLO/Chat/Builder 三模式） |
| MCP 协议 | [github.com/modelcontextprotocol](https://github.com/modelcontextprotocol)（官方 repo + 服务器列表） |

---

## 附录 A：9 项 P2 加分能力（<50% 产品有，有的话更好）

| # | 能力 | 覆盖度 | 说明 |
|---|------|--------|------|
| 1 | **Hooks（事件驱动的自动化）** | 1-2/6 | Claude Code 完整 hook 系统；PreToolUse/PostToolUse 挂脚本 |
| 2 | **Cron / 定时任务** | 2/6 | Claude Code `/loop`（cron 调度，最大 3 天）|
| 3 | **多模态（图像/语音/文件上传）** | 5/6 | 5/6 产品已支持；Claude Code Alt+V 粘贴图像 |
| 4 | **成本 / token 实时统计** | 3/6 | Claude Code `/cost`、Cursor、Trae 部分支持 |
| 5 | **Codebase 语义索引（embeddings + 向量搜索）** | 3/6 | Continue（最强）、Cursor、Claude Code（部分）|
| 6 | **Tauri 桌面打包 / 跨平台** | 5/6 | 5/6 基于 VSCode 跨平台；Claude Code macOS/Windows |
| 7 | **语音输入 / TTS** | 3/6 | Cursor Voice Mode、Claude Code（部分）、Trae（语音输入）|
| 8 | **桌面 / 网页 / 移动协同** | 3/6 | Trae SOLO 独立端（桌面+网页）、ChatGPT Agent（Web/Desktop/Mobile）、LobeChat（PWA）|
| 9 | **Skills / 插件市场** | 4/6 | Claude Code（`/plugin marketplace add anthropics/skills`）、Trae、Continue、Cursor |

---

**报告完。本调研只做事实收集与对比，不评估 NortHing 现状，不写实现代码。**
