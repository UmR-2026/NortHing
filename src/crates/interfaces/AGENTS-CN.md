[English](AGENTS.md) | **中文**

# Interface 层

本层拥有 Rust 协议或面向宿主的入口，用于暴露已装配好的产品行为。UI 应用与交付宿主仍位于 `src/apps`、`src/web-ui`、`src/mobile-web` 与 `northhing-Installer`，并由它们各自最近的本地 `AGENTS.md` 管理。

## 模块

| Crate | 职责 | 本地文档 |
|---|---|---|
| `acp` | 在装配好的产品运行时之上的 Agent Client Protocol 接口 | [AGENTS.md](acp/AGENTS.md) |

## 放置规则

- 当协议入口依赖 `assembly/core` 或某个装配好的产品 profile 时，把它们放在这里。
- 把 transport/protocol 适配器放在 `adapters`。
- 把可复用的 OS、filesystem、terminal、MCP、远程以及 git 实现放在 `services`。

## 依赖边界

- 接口 crate 可以依赖 `assembly/core` 来暴露选定的交付 profile。
- 接口 crate 不得拥有产品策略、可复用服务、协议 transport 内部细节或执行原语。
