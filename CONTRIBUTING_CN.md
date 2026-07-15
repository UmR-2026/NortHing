# 贡献指南

[English](./CONTRIBUTING.md) | **中文**

感谢你对 northhing 的关注！northhing 是一个由 Rust 与 TypeScript 驱动的多平台 AI 编程环境，其核心逻辑在 Desktop/CLI/Server 上共享。本指南会说明如何高效地参与贡献。

## 行为准则

保持尊重、善意与建设性。我们欢迎来自任何背景、任何经验水平的贡献者。

## 快速开始

### 先决条件

- Node.js（推荐 LTS 版本）
- pnpm
- Rust 工具链（通过 [rustup](https://rustup.rs/) 安装）
- 桌面开发需要 [Tauri 先决条件](https://v2.tauri.app/start/prerequisites/)

#### Windows：OpenSSL 配置

大多数 Windows 贡献者不需要手动配置 OpenSSL。使用 `pnpm run desktop:dev` 或常规的 `desktop:build*` 脚本即可；它们在需要时会引导预编译的 OpenSSL 包。

仅在引导失败、准备 CI，或你有意使用 `pnpm run desktop:dev:raw` 时才自行处理 OpenSSL。在这种情况下，请运行 `scripts/ci/setup-openssl-windows.ps1`，或将 `OPENSSL_DIR` 设置为一个预编译的 x64 OpenSSL 目录，并将 `OPENSSL_STATIC` 设为 `1`。

### 安装依赖

```bash
pnpm install
```

### 常用命令

```bash
# 桌面（推荐日常开发使用）
pnpm run desktop:dev                # 完整热重载：Vite HMR + Rust 自动重建 & 重启

# 桌面（轻量预览，不自动重建 Rust）
pnpm run desktop:preview:debug      # 复用预构建二进制 + Vite HMR；Rust 变更需要手工重启

# 桌面（生产构建）
pnpm run desktop:build

# 端到端
pnpm run e2e:test
```

> **`desktop:dev` 与 `desktop:preview:debug` 的区别**：`desktop:dev` 运行 `tauri dev`，提供**完整的热重载** —— 前端修改通过 Vite HMR 立即生效，Rust/后端变更会触发增量重建并自动重启应用。这是在积极开发时的推荐工作流。`desktop:preview:debug` 在 Vite dev 服务器旁启动一个预构建的调试二进制；前端编辑仍然能享受 HMR，但 **Rust 端的修改不会自动重建** —— 你必须先停止再重新运行该命令（或使用 `--force-rebuild`）。当你只想迭代前端代码或希望更快冷启动、无需等待 `tauri dev` 初始化时，请使用 `desktop:preview:debug`。

> 完整脚本列表请参见 [`package.json`](package.json)。面向 agent 的命令、验证与架构规则，请参见 [`AGENTS.md`](AGENTS.md)。

### 桌面调试工具

桌面开发构建启用了 `devtools` Cargo 功能。在原生 webview 中按 `F12` 打开 DevTools。`Cmd/Ctrl + Shift + I` 切换 northhing 的元素检查器，`Cmd/Ctrl + Shift + J` 同样可以打开原生 DevTools。这些工具在面向终端用户的 `release` 构建中会被禁用。

## 代码标准与架构约束

请将 [`AGENTS.md`](AGENTS.md) 视为架构敏感规则、模块边界以及验证矩阵的权威来源。以贡献者视角来看：

- 日志仅使用英文，并且应当有用而不是噪声大。
- 面向用户的文案应使用项目的 i18n 流程；不要在小型产品表面上共享 Web UI 的区域目录。
- 共享 core 必须保持与平台无关。桌面/Tauri 细节属于应用适配器，并通过 transport/API 层回流。
- Tauri 命令使用 `snake_case` 的命令名与结构化 `request` 负载。
- Core 分解、功能边界、依赖边界与构建速度相关的工作必须遵循 `docs/architecture/core-decomposition.md`。
- 特定功能的规则应当归入最近的模块级 `AGENTS.md`。

## 关键贡献方向

1. 通过提交 Issue 贡献好的想法/创意（功能、交互、视觉等）
   > 欢迎产品经理和 UI 设计师通过 PI 快速提交想法，我们会协助打磨成可开发的方案。
2. 改进 Agent 系统与整体质量
3. 提升系统稳定性并加强基础能力
4. 拓展生态（Skills、MCP、LSP 插件，或对特定领域开发场景的更好支持）

## 贡献工作流与 PR 期望

### 除了功能与修复之外，还能贡献什么？

我们欢迎超出常规功能或缺陷修复 PR 之外的各种贡献。例如：

| 贡献领域 | 位置 / 文件 | 示例 |
| --- | --- | --- |
| Prompts | `src/crates/assembly/core/src/agentic/agents/prompts/` | 添加或打磨 prompts，并按需更新相关逻辑 |
| Tools | `src/crates/assembly/core/src/agentic/tools/implementations/`、`src/crates/assembly/core/src/agentic/tools/registry.rs` | 添加工具实现并在工具注册表中注册 |
| Subagents | `src/crates/assembly/core/src/agentic/agents/custom_subagents/`、`src/crates/assembly/core/src/agentic/agents/registry.rs` | 添加子 agent 实现并在子 agent 注册表中注册 |
| 模式贡献 | `src/crates/assembly/core/src/agentic/agents/*_mode.rs`、`src/crates/assembly/core/src/agentic/agents/prompts/*_mode.md`、`src/web-ui/src/locales/*/settings/modes.json` | 添加/改进 agent 模式（如 Plan/Debug/Agentic 或自定义模式），并保持 prompts 与 UI 文案同步 |
| Code Agent 与 AIIde 的场景指南 | `website/src/docs/` | 添加工作流、剧本与真实场景文档（或在 `README.md` 中链接它们） |

### 开始之前

- 提交一个 Issue 来描述问题或方案，特别是较大的变更，以避免重复与设计冲突
- 对于新功能或 UI 变更，请尽早讨论设计方向以保证与产品体验契合
- 使用 Issue 与 PR 模板作为指引。保持 PR 聚焦，并在必要时说明跳过了哪些验证

### PR 标题与描述

我们建议使用 Conventional Commits，以获得更清晰的历史并便于自动化：

- `feat:` 新功能
- `fix:` 缺陷修复
- `docs:` 文档
- `chore:` 维护/依赖
- `refactor:` 不改变行为的重构
- `test:` 测试

UI 变更应附上改动前/后的截图或一段简短视频以便快速评审。

如果你的工作是 AI 辅助的，请在 PR 中注明并说明测试程度（未测试/轻度测试/完整测试），以帮助评审者评估风险。

不要提交临时的 AI 提示词、本地绝对路径、生成的临时文件、配对密钥、令牌、证书或无关产物。保持 PR 聚焦于目标的产品或维护变更。

### 分支管理

**`main` 分支是默认的协作分支，接受特性 PR。** 由于本仓库鼓励产品经理与开发者使用 AI 生成的代码进行快速验证或想法提交，**请把所有 PR 都打到 `main` 分支**。

### 范围

保持 PR 小而聚焦。避免捆绑不相关的变更。

## 测试与验证

运行与所修改文件和行为相匹配的最小检查。CI 负责完整构建与广覆盖测试套件；本地预检应保持聚焦，除非变更影响构建、打包、发布行为，或 CI 无法覆盖某条路径。

常用本地检查：

| 变更类型 | 常见验证 |
| --- | --- |
| 仓库元信息或 GitHub 配置 | `pnpm run check:repo-hygiene && pnpm run check:github-config && git diff --check` |
| 前端运行时或 UI | `pnpm run type-check:web`，并在行为变化时附加最近的聚焦测试 |
| Mobile web | `pnpm --dir src/mobile-web run type-check` |
| Rust 共享运行时或服务 | `cargo check --workspace`，并在行为变化时附加聚焦 `cargo test` |
| 桌面/Tauri 集成 | `cargo check -p northhing-desktop` |
| i18n 资源或契约 | 使用 `AGENTS.md` 中对应的 i18n 行 |

对于 UI 变更，请在合适时附上截图或简短视频。如果你无法运行相关检查，请在 PR 中说明原因并提供一个风险更低的手工验证路径。

## 安全与合规

- 不要提交密钥、令牌、证书或任何敏感数据
- 添加依赖时请确认许可证兼容性并说明用途

## 致谢

每一份贡献都至关重要。欢迎 Issue、PR 与建议！
