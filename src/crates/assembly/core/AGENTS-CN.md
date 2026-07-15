[English](AGENTS.md) | **中文**

# Core Agent 指南

## 范围

本文件适用于 `src/crates/assembly/core`。仓库级规则请参考顶层 `AGENTS.md`，并在存在更近的子指南时使用它。

## 角色

`northhing-core` 是共享的产品运行时外观。它仍拥有兼容性路径与 `product-full` 装配边界，但新的分解工作应当优先选择 `docs/architecture/core-decomposition.md` 与 `docs/architecture/agent-runtime-services-design.md` 中描述的所有者 crate。

主要区域：

- `src/agentic/`：agents、prompts、tools、sessions、execution、persistence
- `src/service/`：config、filesystem、terminal、git、LSP、MCP、remote connect、AI memory
- `src/infrastructure/`：AI 客户端、应用路径、事件系统、存储、调试日志服务端
- `src/product_runtime/`：product-full 兼容性适配器与运行时服务提供者连接

Agent 运行时心智模型：

```text
SessionManager -> Session -> DialogTurn -> ModelRound
```

## 边界规则

- 保持共享 core 与平台无关。避免使用宿主特定 API，如 `tauri::AppHandle`；请使用共享抽象，例如 `northhing_events::EventEmitter`。
- 仅限桌面的宿主适配器应放在 `src/apps/desktop`，并通过 transport/API 层回流。
- 不要在没有明确的 port/interface 边界的情况下，从 `service` 添加新的跨层引用到 `agentic`。
- 不要把平台特定逻辑、build-script 行为、产品能力选择或特定提供方的 AI 序列化移入共享 core。
- 当把所有权迁出 core 时，请在下游调用点有意识地迁移之前，通过外观或重新导出保留旧的导入路径。

## 分解规则

- 将 `northhing-core` 视为兼容性外观加上完整产品装配点，而不是新稳定契约的首选归属地。
- 在拥有明确所有者的 crate 中放置稳定的 DTO、事实、端口以及纯决策。在经过评审的 port/adapter/service 设计以及行为等价测试存在之前，请将具体的管理器、IO、平台适配器与产品执行保留在 core 中。
- 工具变更必须保留展开/收起暴露、对 prompt 可见的清单、`GetToolSpec`、权限行为、`ToolUseContext` 语义以及 desktop/MCP/ACP 目录行为。
- 运行时所有权迁移必须将具体的生命周期、IO、事件投递、权限编排以及远程/平台实现保留在 core 中，直到目标所有者具备经过评审的 port/adapter/service 设计以及行为等价测试。
- 产品域变更可以迁移纯产品域计划，前提是具备等价覆盖；但文件系统写入、worker/宿主副作用、Git/AI 具体调用、marker IO 与 path-manager 集成需保留在 core 中，除非经过评审的所有者设计另有说明。
- 远程/服务变更必须把外部协议生命周期、工作区投影、调度器/会话恢复、终端预热以及产品执行边界保持显式。
- 功能工作必须把 `product-full` 保留为兼容性产品装配边界，除非独立的产品矩阵评审改变了默认的能力选择。

## 所有者参考

需要所有权细节时请使用以下文件，而不是扩展本指南：

- `docs/architecture/core-decomposition.md`
- `docs/architecture/agent-runtime-services-design.md`
- `src/crates/execution/agent-runtime/AGENTS.md`
- `src/crates/execution/tool-contracts/AGENTS.md`
- `src/crates/execution/harness/AGENTS.md`
- `src/crates/contracts/product-domains/AGENTS.md`
- `src/crates/contracts/runtime-ports/` 以及 `src/crates/execution/runtime-services/` 的源码文档
- `src/crates/services/services-core/AGENTS.md`
- `src/crates/services/services-integrations/AGENTS.md`
- `src/crates/execution/tool-provider-groups/AGENTS.md`

一些子树已存在更精细的本地指南：

- `src/crates/adapters/ai-adapters/AGENTS.md`
- `src/agentic/execution/AGENTS.md`
- `src/agentic/deep_review/AGENTS.md`

## 验证

使用匹配所触及行为的最小检查：

```bash
cargo check --workspace
cargo test -p northhing-core <test_name> -- --nocapture
node scripts/check-core-boundaries.mjs
```

仅文档变更时，运行 `git diff --check`。
