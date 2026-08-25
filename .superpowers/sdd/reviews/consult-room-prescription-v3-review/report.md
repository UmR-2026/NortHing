# Prescription v3 Review — consult-room

**Reviewer**: MiniMax-M3（独立视角，独立 grep/读源验证）
**Date**: 2026-08-25
**Scope**: `prescription-v3-20260825.md` — 验证 v2 复审 6 处 API 错位 + 1 个决策点（F3）的修复落实；不重审设计方向。

> 本评审不参考 `consult-room-prescription-v2-review/` 自身（避免互相污染），从源码回放每条处方，独立 verdict。
> 证据锚点：contracts/*、kernel_facade/*、app_state/settings/*、agentic/events/queue.rs、agentic/coordination/dialog_turn/coordinator_session.rs、docs/.../consult-room-main.html。

---

## 逐项验证（12 条清单）

### 1. `confirm_tool` / `reject_tool` 公开与签名
**VERIFIED** — `coordinator_session.rs:219` `pub async fn confirm_tool(&self, tool_id: &str, updated_input: Option<serde_json::Value>) -> NortHingResult<()>`、`coordinator_session.rs:224` `pub async fn reject_tool(&self, tool_id: &str, reason: String) -> NortHingResult<()>`。两方法均 `pub` 且未收 inherent `&mut self` 约束，`coordinator_session.rs:51` 借出 `&Arc<ConversationCoordinator>` 路径直接可用。处方 v3 line 60-69 facade `if approved { coordinator.confirm_tool(tool_id, None).await } else { coordinator.reject_tool(tool_id, reason.unwrap_or_default()).await }` 类型对得上。

### 2. `submit_turn(TurnInputDto)` + 字段
**VERIFIED** — `turn.rs:80` `async fn submit_turn(&self, input: TurnInputDto) -> Result<DialogSubmitOutcomeDto, KernelError>;`；`turn.rs:12-20` `TurnInputDto { session_id, text, mode, policy: SubmissionPolicyDto, source: TriggerSourceDto, workspace_path: Option<String> }` 字段顺序、类型全对。处方 v3 line 24-29 构造示例字段齐全；返回 `outcome.turn_id` 也对（`turn.rs:48`）。

### 3. `stop_turn(&TurnId)` 命名+类型
**VERIFIED** — `turn.rs:6` `pub type TurnId = String;`（newtype = String alias）、`turn.rs:84` `async fn stop_turn(&self, turn_id: &TurnId) -> Result<(), KernelError>;`。处方 v3 line 16 & line 30 命名/类型双对。

### 4. `subscribe_events` callback 模型
**VERIFIED** — `kernel_facade/events.rs:41-44` `async fn subscribe_events(&self, callback: Box<dyn Fn(KernelEventDto) + Send + 'static>) -> Result<SubscriptionId, KernelError>`；不是 `Stream` 返回；未初始化时 `kernel_facade/events.rs:45-53` 返回 `KernelError::Runtime("kernel facade not initialized — init_core not called")`（注：处方 v3 line 49 写 `KernelError::Internal` 用于 F3 facade；这里是 `KernelError::Runtime`——见下方 MISMATCH 备注）。

### 5. `update_app_settings` 闭包签名
**VERIFIED** — `app_state/settings/io.rs:54` `pub async fn update_app_settings<T>(f: impl FnOnce(&mut AppSettings) -> Result<T>) -> Result<T>`。签名、文档（transactional、闭包返回 Err 撤销事务）与处方 v3 line 130 完全一致。

### 6. `test_provider_config(ProviderFormDto)` 双签名
**VERIFIED** — `contracts/kernel-api/src/settings.rs:179` `test_provider(&self, id: &str) -> Result<ProviderTestResultDto, _>`；`settings.rs:183` `test_provider_config(&self, form: ProviderFormDto) -> Result<ProviderTestResultDto, _>` 双签名均在；F6 onboarding Step II 是 form 态，处方 v3 line 181 明确走 `test_provider_config(form)`，路径正确。

### 7. `create_session(SessionConfigDto) -> SessionId`
**VERIFIED** — `contracts/kernel-api/src/session.rs:237` `async fn create_session(&self, config: SessionConfigDto) -> Result<SessionId, KernelError>;`；facade 路径同签名（`kernel_facade/session.rs:15`）。处方 v3 line 186 明确要构造 `SessionConfigDto`，对齐。

### 8. B3 FileSnapshotSystem per-workspace（snapshot orphan 延期）
**VERIFIED** — `service/snapshot/service.rs:36` `let snapshot_system = FileSnapshotSystem::new(runtime_context.clone());` 挂在 `SnapshotService` 内；`service.rs:26-30` 构造函数签 `workspace_dir: PathBuf`（per-workspace）；全仓 grep 无全局 `FileSnapshotSystem` 实例——处方 v3 line 202 "orphan 清理延期"判定成立，无遗漏。

### 9. B4 trait/inherent 矛盾消解
**VERIFIED** — `queue.rs:76` inherent `pub async fn enqueue(&self, event: AgenticEvent, priority: Option<EventPriority>) -> NortHingResult<String>`（返回 `Result<String, ...>`）；`queue.rs:225-228` trait impl `async fn enqueue(&self, event: AgenticEvent, priority: Option<EventPriority>) { let _ = EventQueue::enqueue(self, event, priority).await; }`（返回 `()`）。两者独立 ✓。处方 v3 line 165-168：
   - inherent 改 `Result<EventId, EventQueueFull>`（新错误类型）✓（当前 `EventQueueFull` 类型尚未定义——implementation 期新增，符合契约路径）
   - trait 签名不动（`agent-stream/src/types.rs:62-63`：`async fn enqueue(&self, event: AgenticEvent, priority: Option<EventPriority>)`）✓
   - impl 内 `if let Err(e) = ... { tracing::error!(...) }` 返回 `()` ✓（修正 v2 中 `.map_err` 在返回 `()` 的 trait method 上的语法错误）
   - 优先级 alias：`events/types.rs:11-12` `pub use northhing_events::{... AgenticEventPriority as EventPriority ...}` — 同一类型两个名字 ✓；`agent-stream/src/types.rs:13` 同样 alias ✓；`contracts/events/src/agentic.rs:7` 定义源 — 单点同步即可 ✓

### 10. B2 归属修正（KeyringBackend 在 desktop）
**VERIFIED** — `apps/desktop/src/app_state/settings/keyring.rs:72` `pub trait KeyringBackend: Send + Sync + std::fmt::Debug`；实现 `ProductionKeyring`（line 90）、`MockKeyring`（line 161）同文件。处方 v3 line 142 归属判定成立——无 `services-integrations/src/mcp/env_secret.rs`、新 `store_env`/`load_env` 加在同一 desktop 文件内（line 146-149 pseudo-code），无跨层倒置。`store_api_key` 签名 line 220 `pub fn store_api_key(keyring: &dyn KeyringBackend, provider_id: &str, plaintext: &str) -> Result<String>` 与 v3 引用一致。

### 11. F4 truth mixHex 衰退曲线
**VERIFIED** — `docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.html:548-584`：
   - line 558 `function mixHex(a, b, t)`：逐通道线性插值
   - line 566 `const col = i === 0 ? c : mixHex(BIRTH, c, 0.18 + 0.82 * (i / (hist.length - 1)));` —— **衰退公式与处方 v3 line 115 完全一致**（`t = 0.18 + 0.82 * (i / max(1, history.len()-1))`，Rust 侧 `max(1, ...)` 防止 `n=1` 除零）
   - line 548-550 注释明确"出生灰恒定最左 / 历史色按龄褪向底色 / 右端=当前 mind 全饱和 / 换色时新色自右端进入"
   - 处方 v3 line 112-116 `fade_hex(birth, color, t)` 函数形态 + Rust 侧混色绕开 `color-mix()` WebView2 兼容问题 ✓

### 12. 家规 4 测试判定
**VERIFIED** —
   - **B4（Critical 跳 cap）**：当前 `queue.rs` 无 `#[cfg(test)] mod tests`（grep 0 命中），无 `Critical` 入队单测。处方 v3 line 171 明确"加 1 个单测'满队列时 Critical 入队成功、Normal 返回 Err'"——是 prescription 的实现期承诺，非错误。
   - **F3 facade（未初始化 Err）**：当前 `KernelFacade` 无 facade 方法（v3 P0a 待加），自然无测试。处方 v3 line 71 明确"facade 未初始化时返回 `KernelError::Internal`"——是承诺，非错误。
   - **B3 判定**：bootstrap `tokio::spawn(async move { ... loop { tick.tick().await; let _ = svc.cleanup_all().await; } })`（v3 line 196-200）——**无 `tokio::select!`、无 cancellation token、无 timeout race**。`tokio::time::interval` 单纯时钟滴答，`cleanup_all` 幂等（`infrastructure/storage/cleanup.rs:59`）。**家规 4 不触发**——处方 v3 line 203 "无 select!/取消竞争新增，不强制测试"判定成立 ✓。

---

## v3 引入的新错位 / 新过度设计排查

| 项 | 判定 | 说明 |
|---|---|---|
| 新方法 `respond_to_tool_confirmation` 加到 KernelToolsApi | 用户决策（F3 已拍 2026-08-25 方案 A） | 不算新错位 |
| 新 inherent 错误类型 `EventQueueFull` | implementation 期新建，符合 contracts 路径 | 不算新错位 |
| 新 Signal `mind_base` / `mind_history` | 局部状态增量，无外泄 | 不算过度设计 |
| `store_env` / `load_env` 与 `store_api_key` / `load_api_key` 同文件同 backend | 避免跨层倒置 | 显式简化设计 |
| `Step` enum / `update_app_settings` closure / `event_channel` callback→mpsc | Ponytail 合规 | 不算过度设计 |
| 无 `AppEvent` / `event_bus.rs` / `SettingsState` struct / `settings_store.rs` / `onboarding_state.rs` | Ponytail 合规 | 不算过度设计 |
| 无 debounce | 显式 Ponytail 取舍（v3 line 134） | 合规 |
| `tokenize_hue_mix_hex` 在 Rust 侧算 vs `color-mix()` | 主动规避 WebView2 Chromium < 111 兼容问题（v2 review line 140） | 必要兼容 |

---

## 编译性错误（不归本评审）

P0a contracts 变更（新增 `respond_to_tool_confirmation`）+ B4 inherent 错误类型 `EventQueueFull` 定义 + F4 新 Signal 接线——均属 implementation 期产物，编译性错误待 implementer 报。

## 已知 implementer 易踩点（非 v3 处方错，仅实施提示）

- 处方 v3 line 49 写 facade 未初始化返回 `KernelError::Internal`，但 `kernel_facade/events.rs:49` 现有 `subscribe_events` 同分支返回 `KernelError::Runtime`；新加的 `respond_to_tool_confirmation` 实现期需自决用 `Internal` 还是 `Runtime` 与现有 facade 错误体例保持一致。
- 处方 v3 line 185 `s.add_workspace(path)`：`add_workspace` 签名（`settings/mod.rs:88`）为 `fn add_workspace(&mut self, path: PathBuf)`，需 `PathBuf::from(path)` 转换——非错位，implementer 一次性 wrap。
- `event_channel()` 包装 v3 line 33-39 用 `spawn_blocking`，但 `subscribe_events` 是 `async fn` 且在 mpsc 满时只 `let _ = tx.blocking_send(dto)`——满载丢弃策略与 B4「Critical 跳 cap」非关键事件丢弃语义一致 ✓，但 implementer 需在 B4 单测同步加 1 个 mpsc 满载行为回归。

---

## 总判

`READY FOR USER REVIEW`

---

## 12 项判定一览

| # | 项 | 判定 | 一句证据 |
|---|---|---|---|
| 1 | confirm_tool/reject_tool pub + 签名 | VERIFIED | coordinator_session.rs:219 / 224 pub，签名对得上 |
| 2 | submit_turn + TurnInputDto | VERIFIED | turn.rs:80 签名 + turn.rs:12-20 字段全对 |
| 3 | stop_turn(&TurnId) | VERIFIED | turn.rs:6 `pub type TurnId = String;` + turn.rs:84 签名 |
| 4 | subscribe_events callback 模型 | VERIFIED | kernel_facade/events.rs:41-44 `Box<dyn Fn>`，非 Stream |
| 5 | update_app_settings 闭包 | VERIFIED | app_state/settings/io.rs:54 签名逐字对 |
| 6 | test_provider_config(ProviderFormDto) | VERIFIED | contracts/kernel-api/src/settings.rs:179 / 183 双签名在 |
| 7 | create_session(SessionConfigDto) | VERIFIED | contracts/kernel-api/src/session.rs:237 签名对 |
| 8 | FileSnapshotSystem per-workspace | VERIFIED | service/snapshot/service.rs:36 挂在 SnapshotService 内 |
| 9 | B4 trait/inherent 矛盾消解 | VERIFIED | queue.rs:76 inherent + queue.rs:225 trait impl + events/types.rs:11-12 alias 三点齐 |
| 10 | KeyringBackend 在 desktop | VERIFIED | apps/desktop/src/app_state/settings/keyring.rs:72 trait 定义 |
| 11 | truth mixHex 衰退曲线 | VERIFIED | consult-room-main.html:558-566 mixHex + t=0.18+0.82·(i/(n-1)) |
| 12 | 家规 4 判定 | VERIFIED | B3 单纯 interval 循环无 select!/cancel，rule 4 不触发；B4/F3 测试为 prescription 承诺 |
