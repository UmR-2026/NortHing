[English](AGENTS.md) | **中文**

# Execution Primitives 层

本层拥有可复用的 agent、harness、stream、typed-service 与 tool execution 原语。它不是完整的 Agent Runtime SDK，也不是装配好的产品运行时。产品装配决定哪些原语、tool provider groups、harness providers、适配器与服务对某种交付形式生效。

## 模块

| Crate | 职责 | 本地文档 |
|---|---|---|
| `agent-runtime` | Agent 注册、调度、prompt 缓存、hooks、goals、prompt 事实、基于端口的 `AgentRuntime` 外观、DeepReview 与 provider 无关的状态、DeepResearch 引用重编号以及运行时控制契约 | [AGENTS.md](agent-runtime/AGENTS.md) |
| `agent-stream` | 与 provider 无关的 stream DTO、tool-call 累积以及重放契约 | [AGENTS.md](agent-stream/AGENTS.md) |
| `tool-contracts` | 工具契约、执行闸门、输入校验以及结果展示契约。Cargo 包名仍为 `northhing-agent-tools`。 | [AGENTS.md](tool-contracts/AGENTS.md) |
| `harness` | Harness 工作流契约与注册原语 | [AGENTS.md](harness/AGENTS.md) |
| `runtime-services` | 类型化的运行时服务装配以及服务可用性事实 | [AGENTS.md](runtime-services/AGENTS.md) |
| `tool-provider-groups` | Tool provider group 事实以及 product-full 工具组组合。Cargo 包名仍为 `northhing-tool-packs`。 | [AGENTS.md](tool-provider-groups/AGENTS.md) |
| `tool-execution` | 低层的 file/search/tool IO 辅助函数。Cargo 包名仍为 `tool-runtime`。 | [AGENTS.md](tool-execution/AGENTS.md) |

## 放置规则

- 在这里放置可移植的执行编排、agent 生命周期契约、工具契约、与 provider 无关的 stream 契约以及执行事实。
- 把具体的 filesystem、git、terminal、MCP server、远程 SSH 与 OS 行为保留在 `services`，除非这些代码是纯低层工具原语。
- 把协议投影与外部提供方请求整形保留在 `adapters`。
- 把产品特性选择与交付配置决策保留在 `assembly`，而不是执行原语中。
- Tool packs 应当描述 provider groups 与所需的服务；具体的服务访问应通过 ports 或类型化的运行时服务进行。

## 依赖边界

- 执行原语 crate 可以依赖 `contracts` 以及本层拥有的窄作用域、与 provider 无关的 DTO。
- 执行原语 crate 不得依赖 `assembly/core`、`src/apps`、前端代码、Tauri API 或产品表面生命周期。
- 本层不允许依赖 `adapters`。对 `services` 的新增依赖需要在最近的模块文档或 PR 描述中明确给出边界理由。
