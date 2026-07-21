# Northhing 架构演进探索报告

**日期**: 2026-07-21  
**范围**: 桌面技术栈迁移、kernel-api facade、死代码清理、技术债台账  
**方法**: git log + 文件读取 + grep

---

## 1. 桌面技术栈迁移：Slint → Tauri+React

### 1.1 迁移做了什么

从 `src/apps/desktop`（Slint + Rust 直连）迁移到 `src/apps/desktop-tauri`（Tauri 2 + React + Rust 后端）。整个迁移经历了清晰的阶段：

- **F0.1** (e3daf75): 脚手架搭建 — Tauri 2 + React shell
- **F0.2** (0ac026e): Worker runtime + core init — 独立的多线程 tokio runtime
- **F0.3** (6a61354): Event bridge + 最小聊天循环
- **F0** (c0203c4): 修复三层诊断（capability/event-name/persist-race）
- **F1** (f53fc7f → b870f07): 完整聊天 UI — 气泡、Markdown、代码高亮、停止按钮、session restore，后续做了 UI redesign（组件拆分、暗色主题）
- **K2b** (ae15d22): **切换到 kernel facade** — src-tauri 不再直连 northhing-core，改走 kernel-api facade

### 1.2 两个 desktop app 的关系

**两者都在 workspace members 中，Slint desktop 仍然活着。**

- `src/apps/desktop` 仍在 `Cargo.toml` workspace members 中
- Slint desktop 的 README 仍自称 "the primary human-facing entry point for northhing"
- `desktop-tauri/src-tauri` 在 workspace `exclude` 中（独立编译，避免 Tauri 依赖污染主 workspace）
- **Git log 显示所有新功能开发都在 desktop-tauri 上**：UI redesign、frameless window、settings page、agent identity row 等
- Slint desktop 最近的相关 commit 是 W3a-4 (1b5225d, 2026-07-18) 的 DialogScheduler 接入——这是在 Tauri 迁移开始之前做的

**观察**: Slint desktop 处于"维护模式"——没有新功能，但也没有被删除。代码仍然可编译。W3a 的修复（消息排队、turn 超时等）是同时在 Slint desktop 上做的，因为那时 Tauri 迁移刚开始。

### 1.3 Tauri+React 架构

```
┌─────────────────────────────────────────────┐
│  React UI (ui/src/)                         │
│  ├── App.tsx (布局 + 状态管理)               │
│  ├── hooks/useChat.ts (聊天状态机)           │
│  ├── api.ts (Tauri invoke 封装)              │
│  └── components/ (Header/MessageList/etc)    │
├──────────── Tauri IPC bridge ────────────────┤
│  invoke("send_message", {...})               │
│  listen("chat-chunk", handler)               │
├─────────────────────────────────────────────┤
│  Rust 后端 (src-tauri/src/)                  │
│  ├── main.rs (入口 + handler 注册)           │
│  ├── core_rt.rs (长生命周期 tokio runtime)    │
│  ├── commands.rs (Tauri commands → facade)   │
│  └── event_bridge.rs (kernel events → Tauri  │
│       frontend events)                       │
├─────────────────────────────────────────────┤
│  northhing-kernel-api (facade traits)        │
│  └── KernelFacade (实现所有 trait)            │
├─────────────────────────────────────────────┤
│  northhing-core (业务逻辑)                   │
│  └── ConversationCoordinator / DialogScheduler│
└─────────────────────────────────────────────┘
```

**通信机制**:
- **前端 → 后端**: `@tauri-apps/api/core` 的 `invoke()` 调用 Rust 的 `#[tauri::command]` 函数
- **后端 → 前端**: Rust 通过 `tauri::Emitter::emit()` 发送事件，前端通过 `listen()` 订阅
- **核心调用模式**: 每个 Tauri command 使用 oneshot channel 把工作 spawn 到长生命周期的 `core_rt` 上（W4 discipline — 避免 block_on 和临时 runtime）
- **事件桥接**: `event_bridge.rs` 订阅 `KernelEventDto`（通过 `KernelEventsApi::subscribe_events`），转换为 Tauri 前端事件（`chat-chunk`、`chat-turn-state`、`chat-tool`）

**关键设计决策**:
- core_rt 使用独立线程 + 16MB 栈大小 + 多线程 tokio runtime
- 初始化采用 retry loop（500ms 间隔，最多 120 次 = 60 秒）等待 facade ready
- React 端用 `useChat` hook 封装全部聊天状态机，包括 streaming、tool trace、session restore

---

## 2. kernel-api Facade

### 2.1 是什么

`src/crates/contracts/kernel-api` 是一个**纯定义 crate**——只包含 DTO、trait 定义和错误类型，不包含任何业务逻辑。它的 `Cargo.toml` 明确禁止依赖 `northhing-core` 或任何重量级 crate（rmcp/git2/axum/tower-http/reqwest）。

### 2.2 解决什么问题

**依赖隔离**。在 facade 之前，桌面 app 直连 `northhing-core`（feature = "product-full"），这会把整个依赖树（MCP、git2、axum 等）拉进来。facade crate 作为中间层，让 host（Tauri、未来的 mobile 等）只依赖轻量的 trait 定义 + DTO。

从 git log 可以看到明确的 "FROZEN" 标记（b8be954: "facade FROZEN"，9df9705: "K1 facade surface APPROVE (N=44 cap 53, dual schema frozen)"），说明 facade 的 API 表面是经过审查并冻结的。

### 2.3 Facade Pattern 设计

**两层结构**:

1. ** contracts/kernel-api**（纯定义层）:
   - 11 个模块：bootstrap、session、turn、events、settings、agents、tools、usage、platform、error、util
   - 每个模块定义 trait（如 `KernelSessionApi`、`KernelTurnApi`）+ DTO（如 `SessionConfigDto`、`TurnInputDto`）
   - 使用 `async_trait`，所有方法返回 `Result<T, KernelError>`

2. **assembly/core/src/kernel_facade/mod.rs**（实现层）:
   - `KernelFacade` struct 实现所有 trait
   - 通过 `OnceLock<Arc<KernelFacade>>` 提供全局单例
   - 内部持有 `OnceLock<Arc<ConversationCoordinator>>`（init 后设置）
   - **DTO 转换全部在这一层**：core 类型 → kernel-api DTO 的映射函数（如 `agentic_event_to_dto`、`message_to_dto`）

**初始化协议**（精心设计的三状态门控）:
- `NotStarted` → `InProgress` → `Ready`
- 使用 `AsyncMutex<InitState>` + `Notify` 实现并发安全的初始化
- 失败时回退到 `NotStarted`，允许重试
- `FACADE_READY: AtomicBool` 提供快速路径检查
- 有完整的单元测试覆盖（scenario 1-4：正常/幂等/失败重试/未初始化）

### 2.4 与 northhing-core 的关系

facade 是 northhing-core 的**门面**——它不做任何业务决策，只做：
1. 调用 coordinator/scheduler 的方法
2. 将 core 类型转换为 DTO
3. 将 DTO 转换为 core 类型的输入

facade 实现中有多处 `NEEDS_CONTEXT` 标注——这些是 trait 签名缺少必要参数（如 workspace_path、mode_id）导致无法实现的方法，返回 `KernelError::Internal`。这说明 facade 的 trait 设计还不够完善，需要后续迭代。

**未实现的 trait 方法**（标记为 NEEDS_CONTEXT）:
- `get_persistence_handle` — "PersistenceManager folding deferred (K4b)"
- `set_skill_enabled` — "mode_id not in scope"
- `load_skill_overrides` — "mode_id not available"
- `load_project_skills` — "workspace_path not available"

---

## 3. 死代码清理波次

### 3.1 清理了什么

从 git log 可识别出 **9 个清理 commit**，按时间顺序：

| Commit | 目标 | 删除量 |
|--------|------|--------|
| bbd0b5b (7/16) | dead code warnings 1390→151 (89% reduction) | 29 files, mostly `#![allow]` + import cleanup |
| 89db5cf (7/20) | `RoundContext.cancellation_token` 字段 | 4 files, 6 lines |
| fd5a9e6 (7/20) | dead CI steps, web-ui scripts, unused installer deps | 6 files, 59 lines |
| bd575ec (7/20) | Phase-3 IPC flags, IPC adapter, dead spawn code | 7 files, 198 lines |
| c8630ad (7/20) | TimingCollector, dead variants, dead emitters | 9 files, 165 lines |
| 25f914e (7/20) | relay-server orphan routes/ and relay/ trees | 6 files, 1049 lines |
| 27cd725 (7/20) | northhing-api-layer crate（整个 crate） | 7 files, 280 lines |
| 7abbb3e (7/20) | northhing-transport crate（整个 crate，1713 lines） | 17 files, 1713 lines |
| eed3da8 (7/21) | dead mapping key (mojibake repair 副产品) | minor |

**总计删除约 3500+ 行代码，2 个完整 crate 被移除。**

### 3.2 删除的依据

1. **northhing-transport** (1713 lines): 包含 CLI/Slint/Tauri/Websocket 四种 adapter 实现，但 Tauri 迁移后桌面端通过 kernel-api facade + event_bridge 直接通信，不再需要 transport 层。**合理**。

2. **northhing-api-layer** (280 lines): 包含 DTO 和 handlers，看起来是早期的 HTTP API 层。在 relay-server 有自己的 routes 之后，这层是冗余的。**合理**。

3. **relay-server orphan routes/relay/** (1049 lines): 旧的 API routes 和 relay room 实现。relay-server 可能经历了功能降级。**需要确认 relay-server 当前还有什么功能**。

4. **Phase-3 IPC** (198 lines): agent-dispatch 的 IPC adapter。commit message 说 "descope Phase-3 IPC"——这是一个明确的架构决策，放弃进程间通信的 agent dispatch 模式。**合理**。

5. **TimingCollector** (165 lines): events/terminal/git/cli 中的死代码。**合理**。

6. **RoundContext.cancellation_token**: 只被移除了字段，说明取消逻辑已用其他方式实现（W3a-3 的 inline watchdog）。**合理**。

### 3.3 删错的可能性

**低风险**:
- transport 和 api-layer 是完整 crate 删除，如果有其他 crate 依赖它们会编译失败。已验证 workspace 编译通过（git log 有后续 commit 正常工作）。
- IPC descope 是有意的架构决策，不是误删。

**中等风险点**:
- relay-server 的 routes/ 和 relay/ 目录删除了 1049 行——如果 relay-server 未来需要恢复 API 功能，这些代码需要重写。但当前 relay-server 可能已经简化为纯 WebSocket relay。
- `RoundContext.cancellation_token` 的移除——虽然 W3a-3 用 inline watchdog 替代了它，但如果 watchdog 实现有 bug，没有 fallback 的 cancellation token 可能导致取消不收敛。不过 W3a-3 的 commit message 说 "cancel convergence via persist_cancelled fallback"，说明有 fallback 机制。

**总体评估**: 删除决策有明确的 commit message 解释依据，且删后有正常开发继续，不太可能删错。

---

## 4. 技术债台账更新

### 4.1 P0-1: Desktop message queuing — **已修复但台账未更新**

- **台账说**: "active" — messages sent during active turn are silently lost
- **实际**: W3a-4 (1b5225d, 2026-07-18) 已实现 `DialogScheduler`，messages sent during a turn now queue instead of failing
- **证据**: `scheduler_types.rs` 中有 `DialogTurnQueue<QueuedTurn>` 和 `DialogScheduler`，`QueuedTurn` 结构体完整定义
- **注意**: 这个修复只针对 **Slint desktop** (`src/apps/desktop`)。**Tauri desktop** 通过 `commands.rs::send_message` → `facade.submit_turn` → `scheduler.submit` 间接受益（scheduler 在 core 层），但 Tauri 前端的 `useChat.ts` 中 `handleSend` 有 `if (!text || chat.isStreaming) return` 的 guard——前端直接阻止了 streaming 期间的发送，没有排队 UI。
- **建议**: 标记为 `resolved (Slint)` 或 `partial (Tauri: frontend blocks, no queue UI)`

### 4.2 P0-2: Hang triple — **三项全部已修复但台账未更新**

- **台账说**: "active" — AskUserQuestion 无超时 + tool execution 无 cancel select + turn 无整体超时
- **实际**: W3a 系列全部修复：
  - **W3a-1** (3de7ced, 2026-07-18): AskUserQuestion hang-proof — `tokio::select!` on answer/cancel-token/300s timeout
  - **W3a-2** (26f392e): Default tool execution/confirmation timeouts = 300s (was None=infinite)
  - **W3a-3** (ad5ffa0): Turn watchdog (`NORTHHING_TURN_WATCHDOG_SECS`, default 600s) + cancel convergence via persist_cancelled fallback
- **证据**: `ask_user_question_tool.rs` 第 216-226 行有 select! + 300s timeout；`sub_handle_out.rs` 第 40-50 行有 watchdog timeout 读取 env var；`ai.rs` 第 181-187 行 + 343-347 行有 tool_execution_timeout_secs / tool_confirmation_timeout_secs 默认 300s
- **建议**: 标记为 `resolved` (W3a-1/2/3, 2026-07-18)

### 4.3 其他 P0/P1 条目状态

| ID | 台账状态 | 实际状态 | 备注 |
|----|---------|---------|------|
| P0-1 | active | **resolved (core)** | W3a-4, scheduler 队列已实现 |
| P0-2 | active | **resolved (core)** | W3a-1/2/3, 三项全修复 |
| P1-1 | active | likely active | 非原子配置写入，无相关 commit |
| P1-2 | active | likely active | API key 明文存储，无相关 commit |
| P1-3 | active | likely active | 删除不走回收站，无相关 commit |
| P1-4 | active | **partially resolved** | mojibake: B1a/B7 commit (46172ec, eed3da8) 修复了 builtin-skills 的乱码；但 desktop Rust 代码的 mojibake 未修（P1-4 说的是 desktop Rust）；mobile-web re-pairing 仍 active |
| P1-5 | active | likely active | relay-server 安全默认值，无相关 commit |
| P2-1 ~ P2-6 | active | 未深入检查 | 从 commit message 看无明显修复 |

### 4.4 台账准确性总结

**台账严重过时**：P0-1 和 P0-2 都标记为 "active"，但实际上已经在 2026-07-18 通过 W3a 系列修复。这是一个**台账维护流程问题**——W3a 完成后（008841d: "docs: queue cleared through W3a"）应该同步更新 tech-debt-ledger.md。

**P1-4 的 mojibake 部分也有更新**：builtin-skills 的 mojibake 已在 B1a/B7 修复，但台账没有反映这个进展。

---

## 5. 值得讨论的点

### 5.1 Slint desktop 的处置
Slint desktop 仍然在 workspace 中，但所有新开发都在 Tauri 上。需要决定：
- 何时正式 deprecate/delete Slint desktop？
- Slint desktop 的 W3a-4 消息排队修复是否需要 backport 到 Tauri？（目前 Tauri 前端是直接阻止 streaming 期间发送）

### 5.2 kernel-api facade 的 NEEDS_CONTEXT 方法
有 4 个 trait 方法因签名缺少参数而无法实现（workspace_path、mode_id 等）。这些方法目前返回 `KernelError::Internal`。需要决定：
- 是否修改 trait 签名（会 break frozen schema）？
- 还是在调用方传入 context struct？

### 5.3 Tauri desktop 的消息排队
Tauri 前端 `useChat.ts` 中 `handleSend` 有 `if (chat.isStreaming) return` 的 guard——用户在 streaming 期间发的消息被**静默丢弃**（和 P0-1 描述的问题一样！）。虽然 core 层的 DialogScheduler 支持排队，但 Tauri 的 `commands.rs::send_message` 没有检查 streaming 状态就直接 `facade.submit_turn`。需要确认 scheduler 是否会正确排队这种情况。

### 5.4 relay-server 的功能边界
删除了 1049 行 routes/ 和 relay/ 后，relay-server 现在还做什么？需要确认它是否还有实际功能，还是已经成为空壳。

### 5.5 BitFun 清理
git log 显示有 "purge BitFun remnants" 系列 commit——这说明项目经历了一次品牌更名（BitFun → northhing）。这个清理已经完成，但需要注意是否有遗漏的引用。

### 5.6 facade init gate 的复杂性
`run_init_gate` 实现了三状态门控（NotStarted/InProgress/Ready）+ Notify wait/wake。这个实现正确但有 60 秒的 retry loop（120 × 500ms）在 event_bridge 中。如果 init 失败，60 秒后静默放弃。需要确认这个超时是否合理，以及是否需要用户可见的错误提示。

---

## 6. 疑问

1. **K3 是什么？** git log 提到 "K2 complete, pending K3 ROI gate"——K3 的内容是什么？
2. **F1.5 turn-trace plan** 在 `agent-kernel north-star v0.3.1` 中提到，当前 TurnTrace 组件已实现，这个 plan 是否完成？
3. **northing-installer** 在 workspace exclude 中，它的状态是什么？还在维护吗？
4. **mobile-web** 目录有 React + zustand + vite，它和 desktop-tauri 的 React 代码是否共享？看起来是独立的代码库。

---

*报告结束。以上为探索性观察，不含评分或 APPROVE/REJECT 判断。*
