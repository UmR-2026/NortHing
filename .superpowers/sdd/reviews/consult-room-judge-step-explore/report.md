# Consult-Room 全局问题处方评审 — step-explore 独立审判

> 独立视角。逐条验证证据，不受 prior analysis 约束。ponytail 原则 enforced。

---

## Problem B1 — Dioxus ↔ 后端桥接缺失

- SPEC: **PASS** — `ui_dioxus` 当前完全由 `seed_session()` 硬编码 mock 驱动（`app.rs:101`），无任何 kernel facade 接线。设计规范（PAGES-BRIEF §3 space/§4 交付）要求真实会话交互，桥接缺失 = 规格违反。
- QUALITY: **FAIL** — 处方在 `ui_dioxus` 内建 `api` 子模块（`session_api` + `dialog_api`），违反分层边界（AGENTS.md Layer 1 Interfaces 不拥有 concrete service behavior）。已有 `CoreAgentAdapter`（`src/apps/cli/src/agent/core_adapter.rs`）封装了 `Arc<ConversationCoordinator>` + `send_message`/`cancel`/`confirm_tool`/`reject_tool`，处方应复用该模式而非新建桥接层。此外 `tauri::AppHandle` 引用是多余的——桌面端用 Slint，Tauri 仅限 installer（AGENTS.md 明文）。

** rationale: ** 桥接确需，但应走 existing `CoreAgentAdapter` pattern + `DesktopEventBridge` 的 `kernel_facade()` 接线方式，不要在 UI crate 内置 backend API。

---

## Problem B2 — MCP env 变量明文存 app.json

- SPEC: **PASS** — tech-debt P1-8 明确登记此问题。`MCPServerConfig.env: HashMap<String, String>` 经 `Serialize` 落盘（`server/mod.rs:107`），MCP server 凭据（API keys、tokens）随 env block 明文写入 `app.json`，违反安全基线。
- QUALITY: **PASS** — 处方正确复用 P1-C3 的 `KeyringBackend` trait + sentinel 模式（`keyring.rs:72-180` 已有完整实现）。`McpEnvStore::get/set/remove` 在 `HashMap<String, String>` 和 keyring 之间做序列化转换，`persist_app_settings` 前调 `flush`——与 api_key sentinel 方案同形，零新抽象范式。唯一建议：env block 整块存为一个 keyring entry（JSON blob），不用逐 key 拆分，保持简单。

** rationale: ** 处方与已establish的 keyring 迁移模式完全对齐，无过度设计。

---

## Problem B3 — CleanupService 从未调度

- SPEC: **PASS** — `CleanupService::cleanup_all()`（`cleanup.rs:59`）全量实现但无调用方，tech-debt P2-4 登记。`auto_cleanup_enabled` 默认 true 却不被触发 = 技术债。
- QUALITY: **FAIL** — 处方 conflates 两个独立 concern：① temp/log/cache 文件清理（`CleanupService` 的职责）和 ② session 删除时的即时清理（完全不同的语义，可能指 `ProcessManager::cleanup_all_processes()` 或 session 文件回收）。将二者耦合在同一个启动调度器里是错位抽象。更好的方案：① 在应用生命周期钩子（如 `AppState` drop 或系统托盘 idle 回调）spawn 24h cleanup loop；② session 删除触发 reckon 路径应在 session 管理器内处理，不走 `CleanupService`。

** rationale: ** 处方混了不同职责域，调度位置和触发语义都不精确。

---

## Problem B4 — Event queue 满时静默丢事件

- SPEC: **PASS** — `queue.rs:85-88` 满队列时仅 `warn!` + 返回 `Ok(event_id)`，调用方无从知晓事件已丢。tech-debt P2-6 登记， Critical 事件丢失影响正确性。
- QUALITY: **FAIL** — 两处问题：① "Critical 事件永不丢（block + 扩容）" 会在 producer 端引入 unbounded 阻塞——当 consumer 死锁或严重落后时，Critical producer（可能是 LLM stream handler）被阻塞 → 级联超时，不如在 queue 层加一个小的 express lane（如 `crossbeam_channel` bounded+try_send 失败则 panic 进 warn-log + 独立兜底）。② `StreamEventSink::enqueue` trait 返回 `()`（`queue.rs:226`），改回 `Result` 需要改动 trait contract 和全部 5 个实现方，影响面超出 brief 所述。最小正确方案：只改 `EventQueue::enqueue` 返回 `Result`，`StreamEventSink::enqueue` 保持 swallow-error。

** rationale: ** blocking Critical 有级联风险；trait 签名变更影响面被低估。

---

## Problem F1 — 零真实数据流

- SPEC: **PASS** — 全站硬编码：`seed_session()` 返回固定 5 条 `MockEntry`，`entries = use_signal(|| seed_session())` 无任何动态更新。设计规范要求真实会话流（PAGES-BRIEF §3 space 描述"当前会话=亮着的诊室门"）。
- QUALITY: **FAIL** — 四步渐进方案在 Step 1 定义 `AppEvent` enum 时风险重复造轮子——核心已有 `AgenticEvent`（`src/crates/assembly/core/src/agentic/events/types.rs`）+ `StreamEventSink` trait + `EventQueue::subscribe()` broadcast。Dioxus frontend应消费现有 `AgenticEvent` 流，而非新建平行事件类型。Step 2 `event_bus.rs` (tokio mpsc) 如果只是转发 `AgenticEvent` 则多余——直接 `subscribe()` 即可。最小方案：在 `ui_dioxus` 的 `DesktopEventBridge` 层做 `AgenticEvent` → Dioxus `UseSignal` 映射，一步到位。

** rationale: ** 自建 AppEvent enum 是典型的平行抽象，应消费现有的 AgenticEvent 流。

---

## Problem F2 — 消息发送无 handler

- SPEC: **PASS** — 发送按钮仅 toggle `streaming` 信号（`app.rs:372-374`），input box 是静态占位符（`app.rs:363-367`）。用户实际无法发送消息，规格违反。
- QUALITY: **FAIL** — 提案依赖 F1 的 event_bus → B1 的桥接 → `DialogScheduler::submit`，形成三环循环依赖。现有 `CoreAgentAdapter::send_message`（`core_adapter.rs:191-244`）已封装完整路径（ensure_session → start_dialog_turn → turn_id 管理）。最小方案：Dioxus on_send → bridge 直接调用 `CoreAgentAdapter` 等价方法，不经过 event_bus 中转。streaming 消费用 `use_future` + `EventQueue::subscribe()`。

** rationale: ** 三环依赖使 P0 任务无法并行；已有 CoreAgentAdapter 可直接复用。

---

## Problem F3 — Approval 卡 approve/reject 无 handler

- SPEC: **PASS** — approve/reject 按钮无 onclick 绑定（`app.rs:577-583`），approval 卡永远 pending。设计规范（PANELS-BRIEF §2）要求完整 approval-card 语法。
- QUALITY: **FAIL** — 提案路由经过 `AppEvent::Approve/Reject` → `tool_confirmation` 确认门，但现有 `CoreAgentAdapter` 已有 `confirm_tool(tool_id, input)` 和 `reject_tool(tool_id, reason)`（`core_adapter.rs:293-307`），两步直接调 facade 即可。绕 event_bus 中转 pure UI action 是多余的间接层。reject 可选文本输入框也已由 `reject_tool(tool_id, reason: String)` 原生支持，不需要额外设计。

** rationale: ** 忽略现有 confirm_tool/reject_tool 直接接口，增加不必要的 event routing 层。

---

## Problem F4 — 编年史条空白

- SPEC: **PASS** — `#chronicle-bar`（`app.rs:317-322`）是空 div，title 说"双击演示"但无视觉内容。设计规范（PANELS-BRIEF §1 chrome）要求"顶晕染色"。
- QUALITY: **PASS** — 纯 CSS 方案：`@keyframes chronicle-shift` + `background: linear-gradient(...)` + mind 色 token。最短可行解，不引入 JS 逻辑，不新增 infinite 动画（遵守呼吸纪律）。与 v3 系统的 `--mind-base` 变量对齐。

** rationale: ** 最薄方案，符合 ponytail 原则。

---

## Problem F5 — Settings 无持久化

- SPEC: **PASS** — 所有引擎/provider/MCP/display 设置均为 `use_signal`，刷新即丢。PAGES-BRIEF §2 "设施可调"要求设置可持久化。
- QUALITY: **FAIL** — 处方建议从零建 `settings_store.rs` + `SettingsState` struct，但已有完整设施：`ProviderConfig`（`types.rs:51-97`）带 full serialization、`WorkspaceEntry`、`keyring.rs` 持久化 api_key sentinel。正确做法是扩展现有 settings 读写路径（已经有 `push_resolved_keys_to_core` 等 sync 函数），在 Dioxus 侧只做 `use_signal` → emit → 走现有 sync 通道。从零建 store 是重复发明。

** rationale: ** 忽略已有 ProviderConfig / keyring / sync 基础设施，重复造轮。

---

## Problem F6 — Onboarding 无流程控制

- SPEC: **NEEDS CONTEXT** — PAGES-BRIEF §3 onboarding 描述"诞生仪式"4 字段 + provider + workspace，当前只是可选的 module window spawn（`app.rs:288-297`）。但设计文件（`consult-room-onboarding-v2.html`）是 HTML 静态原型，非 Dioxus 组件。brief 未明确 onboarding 是转 Dioxus 还是保持 HTML + 桥接。
- QUALITY: **FAIL (conditional)** — 如果 onboarding 保持 HTML 原型（当前架构），提案的 `onboarding_state.rs` Rust 状态机 + `AppEvent::OnboardingComplete` 发射是过度设计——HTML 页面通过 JS bridge 做 sequential flow 即可，不需要 Rust 端 state machine。如果 onboarding 将转 Dioxus，4 步 sequential UI 用 `use_signal` 跟踪 `current_step` 足够，不需要独立 state machine 模块。

** rationale: ** 未澄清 onboarding 的渲染宿主（HTML vs Dioxus），方案在两种路径上都过度设计；已有 identity.md + WorkspaceEntry 覆盖大部分 state。

---

## Summary

- **Spec compliant: 7/10** — B1, B2, B3, B4, F1, F2, F3, F4, F5 pass spec; only F6 needs context clarification.
- **Quality concerns:**
  1. **平行抽象泛滥**（F1 AppEvent、F2/F3 event_bus 中转）——已有 `AgenticEvent` + `CoreAgentAdapter` 足够，不需要在 UI crate 建新桥接层。这是最大的架构反模式。
  2. **职责耦合**（B3 把 file cleanup 和 session lifecycle 耦合在一起；B4 blocking Critical events 有级联风险）。
  3. **忽略现有基础设施**（F5 从零建 settings store、F2/F3 不调用 CoreAgentAdapter 现成方法）。
- **Recommended priority order:**
  - **P0（并行可行）**: B1+F1 合并为"桥接层"单任务（复用 CoreAgentAdapter pattern）；F2+F3 合并为"交互接线"（on_send + approve/reject 走 CoreAgentAdapter 直接方法，不走 event_bus）。
  - **P1**: B2（安全，独立）、B4（改 Result 但不 blocking）、F4（CSS 单文件）。
  - **P2**: F6（澄清 onboarding 宿主后才动）、F5（扩已有 settings，工作量小）、B3（cleanup 调度，无用户可见影响，可延后）。
