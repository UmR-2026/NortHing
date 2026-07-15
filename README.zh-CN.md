**中文**  [English](README.md)

<div align="center">

![northhing](./png/northhing_title.png)

</div>
<div align="center">

[![GitHub release](https://img.shields.io/github/v/release/GCWing/northhing?style=flat-square&color=blue)](https://github.com/GCWing/northhing/releases)
[![Website](https://img.shields.io/badge/Website-opennorthhing.com-6f42c1?style=flat-square)](https://opennorthhing.com/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow?style=flat-square)](https://github.com/GCWing/northhing/blob/main/LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue?style=flat-square)](https://github.com/GCWing/northhing)

</div>

---

> ⚠️ **过期提醒（2026-06-20）：** 本 README 提到 **Tauri** 作为桌面技术栈，但当前
> `v3-restructure` 分支（HEAD `840bd4f`，47 commits）已将 Tauri
> 替换为 **Slint + Material** 作为桌面 Shell。README 中一些行仍含有
> 历史编码问题残留的乱码字符。
> **当前架构、项目状态和构建方法，请参阅
> [`HANDOFF.md`](./HANDOFF.md)——它是截至 2026-06-20 的准确描述。**
>
> 本 README 保留用于对外的产品描述。编辑时请谨慎：它链接自 GitHub 仓库首页。

---

## 什么是 northhing

**northhing 是一个桌面级 Agent 运行时（Local Agent Runtime），同时也是一套开箱即用的桌面 Agent 应用。**

- 它是**基座**——Rust 内核 + Slint + Material UI 外壳，内置会话、工具、记忆、MCP、远程控制协议，为长期运行而生；
- 它是**产品**——下载安装就拥有统一的 Code Agent：编码、研究、任务自动化，几乎覆盖当前业界所有主流 Agent 能力形态。

> **一次安装，既能当 Agent 用，也能当 Runtime 用。**

northhing 的目标是：将 **Code Agent 的编码力、Deep Research 的研究深度、OpenClaw 的助理体验、Computer Use 的操控力** 等业界最受欢迎的 Agent 能力，装进同一个桌面端，并把底层协议栈（Agentic Runtime、工具、记忆、MCP、Skills、上下文压缩、远程控制）全部默认就绪——你拿来就能用，也可以基于它定义**你自己的领域 Agent**。


![readme_hero](./png/readme_hero.png)


---

## 为什么选 northhing

- **一个应用，几乎覆盖全部业界主流 Agent 能力**：Code / 研究 / Research / Computer Use / 文档协作 / MCP / 远程控制——不用在多个工具之间切换，也不用各配一个订阅费；
- **下载即用，不做拼装工作**：MCP / 文件系统 / 终端 / Git / 远程 SSH 全部内置，模型配好就能开跑，省掉自己从零搭建协议栈的时间；
- **数据在你自己的机器上**：会话、记忆、工作目录都存在 `.northhing/sessions/` 下，可迁移、可导出、可审计；没有强制上云，隐私与合规场景都能用；
- **极致可定制，从一个 Markdown 文件到整仓 fork 没有断点**：90% 的领域化需求一个 `.md` 就能搞定；缺工具？缺界面？要改产品？在 northhing 里直接让 Code Agent 动手——*你定制它的方式，就是用它本身*；
- **手机也能指挥桌面**：扫码、Telegram、飞书 Bot、微信 Bot 都是远控入口。Agent 在桌面上干活，你在路上看进度；
- **真正能装机长用的桌面应用**：Rust 内核 + Slint 外壳，冷启动快、常驻资源低，长时间后台运行也不心疼电脑；
- **会自我迭代**：97%+ 的代码由 northhing 内置 Code Agent 通过 Vibe Coding 完成，天然亲和 AI 开发。

---

## 最新动态

northhing 通过将 **flashgrep** 与 **ripgrep** 联动形成增强版的检索链路，在 Chromium 这类超大代码仓库中将代码搜索耗时最高降低约 **94.6%**、平均加速约 **36.1×**，显著缩短项目探索时间。

![flashgrep feature](./png/feat_flashgrep.png)

---

## 紧追前沿 · 开箱即用

Agent 领域几乎每周都有新范式出现。northhing 的节奏是：**看到好东西，就把它装进桌面，并让它和已有能力无缝协同。**


![first_screen_screenshot](./png/first_screen_screenshot.png)

以下是 northhing 的**官方 Agent 和能力清单**，及对业界最前沿 Agent 范式的复现进度。零配置，下载即用：

| 能力 | 说明 |
| --- | --- |
| **Code Agent** | 四种模式：Agentic（自主读 / 改 / 跑 / 验证）/ Plan（先规划后执行）/ Debug（插桩取证 / 根因定位）/ Review（基于仓库规范审核） |
| **深度审核** | 面向高风险代码变更的并行代码审核团队，内置专项审核员、质量把关和用户确认后的修复流程 |
| **会话用量报告** | 在聊天中输入 `/usage`，查看当前会话的记录耗时、Token 用量和模型 / 工具 / 文件摘要 |
| **Research Agent** | PDF / DOCX / XLSX / PPTX 原生处理能力，可从 Skill 市场按需扩展 |
| **文档协作** | 在文档里边写边问，AI 直接在段落上改写、续写、总结、排版 |
| **Computer Use** | 看屏幕、动鼠标键盘，操作浏览器与任意桌面应用，把手动点点的活交给 Agent |
| **个人助理** | 长期记忆、个性设定，按需调度 Code / 研究 / Research / Computer Use / 自定义 Agent |
| **远程控制 / IM 接入** | 手机扫码、Telegram、飞书 Bot、微信 Bot 远程下达指令，实时查看进度 |
| **MCP / MCP App** | 任意外部工具一键接入，MCP 也能打包成可安装的 App |
| **生成式 UI** | 对话过程中按需生成可交互 UI 组件，嵌在消息流里直接用 |
| **Mini App** | 一句话生成独立可运行的应用，即生即跑，一键打包成桌面端 |
| **Markdown 定义 Agent** | 写一个 `.md` 文件，立即在 Runtime 里跑起来，满足大多数领域化需求 |
| **长期记忆 + 项目上下文** | 跨会话积累，任意 Agent 可读 |
| **自我迭代** | Code Agent 直接改 northhing 自己的仓库 |
| **⋯⋯** | 下一个热点持续跟进中，欢迎 Issue 提需求 |

---

## 怎么定制自己的 northhing

不同深度的定制需求，对应不同成本的扩展路径。按"从轻到重"依次选择即可：

| 层级 | 方式 | 适合做 | 改动成本 |
| --- | --- | --- | --- |
| **L1** | **Markdown 自定义 Agent** | 换提示词 + 挑选工具组合，即可定义一个**新的 Agent 能力**，满足大多数领域化需求 | 写一个 `.md` 文件 |
| **L2** | **Mini App** | 需要用界面交互的能力（面板、表单、可视化、业务流程） | 一句话生成，即生即跑 |
| **L3** | **源码级添加工具** | 新工具、新模型适配、新协议接入——给自定义 Agent 补齐它需要但 northhing 还没有的 `tool` | 用 northhing 的 Code Agent 改 northhing 自己的源码 |
| **L4** | **自由改源码** | 换品牌、重做 UI、改会话模型、做完全不一样的产品 | 整仓 fork，天然亲和 Vibe Coding 开发模式 |

### 一个例子：Code Agent 与 Research Agent 的差别其实很小

在 northhing 里，一个 Agent = **一段提示词（系统角色 + 行为约束）+ 一组它能调用的工具**。官方的 Code Agent 与 Research Agent 区别就仅在于此：

|  | Code Agent | Research Agent |
| --- | --- | --- |
| **提示词** | 面向仓库工作的角色、规范、四种工作模式 | 面向知识工作的角色、文档处理流程 |
| **工具链** | 文件 / 终端 / Git / LSP / 构建与测试 | PDF / DOCX / XLSX / PPTX / Skill 市场 |
| **共用底盘** | 同一套会话、记忆、MCP、远控、UI、模型适配 | 同一套会话、记忆、MCP、远控、UI、模型适配 |

**所以，如果你想做"法律审阅 Agent"、"科研文献 Agent"或"运维应急 Agent"——L1 就够了：**

1. 写一个 Markdown，定好它的角色 / 禁区 / 工作流程
2. 从工具注册表里勾上它该用的工具（文件、浏览器、特定 MCP⋯⋯）
3. 如果缺了一个特定工具——**L3**，打开 northhing 让 Code Agent 帮你加进源码
4. 如果这个 Agent 需要一个专属界面——**L2**，一句话生成一个 Mini App
5. 如果你要做一个完全不一样的产品——**L4**，fork 整个仓库，让 Code Agent 陪你改

**关键点**：L3 和 L4 都不用你离开 northhing——*打开 northhing，对 Code Agent 说你要改什么，它就改给你看*。**你定制它的方式，就是用它本身。**

> 从一个 Markdown 文件到完整 fork，中间没有断点。这正是"会自我迭代的基座"的含义。

---

## 平台支持

桌面端基于 Slint 构建，支持 Windows / macOS / Linux；远程控制支持手机浏览器、Telegram、飞书、微信。

---

## 快速开始

### 下载和使用

从 [Releases](https://github.com/GCWing/northhing/releases) 页面下载最新桌面端安装包，安装后配置模型即可开始使用。

### 从源码构建

**前置依赖：**

- [Node.js](https://nodejs.org/)（推荐 LTS 版本）
- [pnpm](https://pnpm.io/)
- [Rust 工具链](https://rustup.rs/)
- [Slint 前置依赖](https://v2.tauri.app/start/prerequisites/)（桌面端开发需要）

**运行指令：**

```bash
# 安装依赖
pnpm install

# 以开发模式运行桌面端
cargo run -p northhing

# 构建桌面端
cargo build -p northhing --release
```

更多详情请参阅[贡献指南](./CONTRIBUTING_CN.md)。

---

## 项目结构一览

```
src/crates/interfaces/         # ACP 等产品协议接口
src/crates/assembly/           # 兼容门面与产品能力组装
src/crates/adapters/           # AI、API、transport、WebDriver 适配器
src/crates/services/           # 可复用的 OS、terminal、MCP、remote、git、filesystem 服务
src/crates/execution/          # Agent、harness、stream、typed-service、tool 原语
src/crates/contracts/          # 稳定 DTO、事件、runtime ports、产品域契约
src/apps/desktop        # Slint 桌面宿主
src/apps/server         # Web 服务端运行时
src/apps/cli            # CLI 运行时
src/web-ui              # 桌面 / Web 共用前端
```

设计原则：**产品逻辑保持平台无关，通过适配器对外暴露**。详见 [AGENTS.md](./AGENTS.md)。

---

## 贡献

我们欢迎好的创意和代码；对 AI 生成的代码持最大接纳态度。请直接向 `main` 分支提交 PR；我们在 `main` 上评审与合并。

**我们重点关注的贡献方向：**

1. **Runtime 内核**：会话模型、工具注册表、记忆系统、协议适配器
2. **参考 Agent**：Code / 研究 / Research / 个人助理的能力与体验
3. **生态扩展**：Skills、MCP、LSP 插件、Mini App 模板、新的垂直 Agent
4. 想法 / 创意（功能、交互、视觉）——欢迎提 Issue

---

## 声明

1. 本项目为业余时间探索、研究、构建下一代人机协同交互，非商用盈利项目。
2. 超过 97% 由 Vibe Coding 完成。代码问题也欢迎指正，鼓励通过 AI 进行重构优化。
3. 本项目依赖和参考了众多开源软件。感谢所有开源作者。**如侵犯您的相关权益请联系我们整改。**

---
