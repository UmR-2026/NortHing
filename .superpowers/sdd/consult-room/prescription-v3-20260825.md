# consult-room 前端线 + 后端桥接 — 处方 v3（API 核实版）

> 前序：v1 三方 judge → v2 修正 → v2 复审发现 6 处 API 错位（minimax-m3 主审）→ 本版逐条按源码核实签名重写。
> 证据锚点全部带 file:line。judge 工件：`reviews/consult-room-judge-{minimax-m3,step-explore,ox-alpha}/` + `reviews/consult-room-prescription-v2-review{,-step}/`。
> F3 决策（用户 2026-08-25）：**方案 A——KernelToolsApi 新增 `respond_to_tool_confirmation`**。理由：Slint approval 线本就死了（全仓 grep 零接线），Dioxus v1 是现行前端，分层规则禁止 UI 直连 coordinator，CLI + Dioxus 双 owner 满足 contracts 稳定性门槛。

---

## B1 — Dioxus ↔ 后端桥接（P0a）

**新文件**：`src/apps/desktop/src/ui_dioxus/api.rs`（~180 行）

真实签名（已核实）：
- `kernel_facade()` → `Arc<KernelFacade>`（`kernel_facade/mod.rs:36`）
- `KernelTurnApi::submit_turn(&self, input: TurnInputDto) -> Result<DialogSubmitOutcomeDto, KernelError>`（`contracts/kernel-api/src/turn.rs:80`）
- `KernelTurnApi::stop_turn(&self, turn_id: &TurnId) -> Result<(), KernelError>`（`turn.rs:84`；`TurnId = String` 类型别名）
- `KernelSessionApi::list_sessions(&self) -> Result<Vec<SessionSummaryDto>, _>`（`session.rs:241`）
- `KernelSessionApi::get_session(&self, id: &SessionId) -> Result<SessionDto, _>`（`session.rs:253`）
- `KernelEventsApi::subscribe_events(&self, callback: Box<dyn Fn(KernelEventDto) + Send + 'static>) -> Result<SubscriptionId, _>`（`kernel_facade/events.rs:41`——**callback 模型，不是 Stream**）

结构：
```rust
// 上行（UI → kernel），全部薄封装 facade：
pub async fn submit_turn(session_id: &str, text: String) -> Result<TurnId, KernelError>
  // 内部构造 TurnInputDto { session_id, text, mode: "agentic", 
  //   policy: SubmissionPolicyDto { allow_subagent: true, max_turns: None },
  //   source: TriggerSourceDto::User, workspace_path: None }
  // 返回 outcome.turn_id；outcome.accepted == false 时 Err(outcome.error)
pub async fn stop_turn(turn_id: &TurnId) -> Result<(), KernelError>
pub async fn list_sessions() -> Result<Vec<SessionSummaryDto>, KernelError>
pub async fn get_session(id: &SessionId) -> Result<SessionDto, KernelError>

// 下行（kernel → UI），callback → mpsc 包装一次：
pub fn event_channel() -> tokio::sync::mpsc::Receiver<KernelEventDto>
  // let (tx, rx) = mpsc::channel(256);
  // spawn_blocking 一次性调 facade.subscribe_events(Box::new(move |dto| {
  //     let _ = tx.blocking_send(dto);   // 满则丢非关键事件，同 B4 语义
  // }))
  // Dioxus 侧 use_future { while let Some(dto) = rx.recv().await { ... } }
```

不做：Stream 适配器 / AppEvent enum / event_bus.rs / DialogScheduler 直连。

---

## F3 — facade 新增 approval 应答（P0a，与 B1 同批）

**契约变更**（contracts/AGENTS.md：双 owner=CLI+Dioxus 已满足稳定性门槛）：

`contracts/kernel-api/src/tools.rs` `KernelToolsApi` 追加：
```rust
/// Respond to a pending tool confirmation (approve or reject).
async fn respond_to_tool_confirmation(
    &self, tool_id: &str, approved: bool, reason: Option<String>,
) -> Result<(), KernelError>;
```

`kernel_facade/tools.rs` 实现（~20 行）：
```rust
async fn respond_to_tool_confirmation(&self, tool_id: &str, approved: bool, reason: Option<String>) -> ... {
    let coordinator = self.coordinator()?;
    if approved {
        coordinator.confirm_tool(tool_id, None).await  // coordinator_session.rs:219 已存在且 pub
    } else {
        coordinator.reject_tool(tool_id, reason.unwrap_or_default()).await
    }
    .map_err(|e| KernelError::Runtime(format!("respond_to_tool_confirmation failed: {e}")))
}
```

测试（家规 4 不适用——无 select!/取消竞争；但仍带 1 个单测）：facade 未初始化时返回 `KernelError::Internal`（coordinator None 分支）。

B1 的 `api.rs` 顺势暴露：
```rust
pub async fn respond_to_tool_confirmation(tool_id: &str, approved: bool) -> Result<(), KernelError>
```

---

## F2 — 消息发送接线（P0b，依赖 P0a）

**文件**：`ui_dioxus/app.rs`（就地改）

1. input-box div → Dioxus `input` 元素：`value` 绑 Signal + `oninput` 写入 + `onkeydown` Enter 触发 send（注意 IME 合成中 Enter 不应触发——检查 `e.data().key() == Key::Enter` 且非 composing）。
2. send/stop 合一按钮（保持真值"发送/停止合一"语义）：
   - 非 streaming：`api::submit_turn(&session_id, input)` 成功 → 清输入 + `streaming.set(true)` + 存 `active_turn_id`
   - streaming：`api::stop_turn(&active_turn_id)` → `streaming.set(false)`
3. streaming 渲染：`use_future` 消费 `api::event_channel()`，`KernelEventDto::TextChunk` 追加到 entries Signal；`TurnState{Completed/Failed/Cancelled}` 收尾 → `streaming.set(false)`。
4. `session_id` 来源：启动时 `list_sessions()` 取第一个，无则 `create_session`（签名见 F6）——本轮可硬编码一个 room session id 的 lazy 创建，落 F6 再正规化。

---

## F3-UI — Approval 卡接线（P0c，依赖 P0a）

**文件**：`ui_dioxus/app.rs` `render_entry` 的 Approval 分支

- `resolved == false` 卡：approve onclick → `api::respond_to_tool_confirmation(&call_id, true)`；reject → `..., false)`。成功后本地把该条 `resolved` 置 true（乐观更新）。
- `resolved == true` 卡：维持现状不绑事件。
- 无 reject 文本输入框（真值红线"诗意<功能"）。

---

## F4 — 编年史条（P1a，独立）

**文件**：`ui_dioxus/app.rs` + 新 Signal

三方 judge 共识：**truth HTML 548-584 是事件驱动颜色沉积**（出生灰恒定最左 / 历史色按龄褪向底色 / 右端=当前 mind 全饱和 / 换色时新色自右端进入），**禁止** keyframes 循环（idle 跑动画=错信息）。

实现：
1. 新 Signal：`mind_base`（当前 mind 色 hex）+ `mind_history: Vec<String>`（换色记录）。
2. 启动时 `mind_base` 从 AppSettings 读（onboarding 落盘的 palette 色；无则出生灰 `#888888`）。
3. 渐变 stops **在 Rust 侧算**（不依赖 `color-mix()`，规避 WebView2 版本问题），照 truth 的 mixHex 衰退曲线：
   ```rust
   fn fade_hex(birth: &str, color: &str, t: f64) -> String
   // t = 0.18 + 0.82 * (i / max(1, history.len()-1))，逐通道线性插值
   ```
4. div inline style：`background: linear-gradient(90deg, {stops})`；Signal 变化 → Dioxus 自动重渲。
5. 换色触发源：onboarding/设置改 palette 后写 settings（F5 路径）→ room 窗经 `event_channel` 或下次启动读取。本轮 room 窗内不主动改色（agent 自主换色属 growth 线，不在此批）。

不动 `TRUTH_CSS`。

---

## F5 — Settings 持久化（P1b，独立）

**文件**：`ui_dioxus/pages_settings.rs`（就地改）

真实签名（已核实 `app_state/settings/io.rs:54`）：
```rust
pub async fn update_app_settings<T>(f: impl FnOnce(&mut AppSettings) -> Result<T>) -> Result<T>
```

1. 页面加载：`load_app_settings()`（io.rs:23）→ 填 8 个 use_signal 初值。
2. 每个 toggle onclick：`update_app_settings(|s| { s.<field> = new_val; Ok(()) })`。settings IO 频率低（手动 toggle），**不做 debounce**（ponytail）。
3. provider api_key 变更走既有 `store_api_key`（`settings/keyring.rs:220`，签名 `(&dyn KeyringBackend, provider_id, plaintext)`），key 不落 GlobalConfig 磁盘（Scheme C 骨干不变量）。
4. 不建 `SettingsState` struct / `settings_store.rs`。

---

## B2 — MCP env keyring（P1c，独立）

落点修正：**不放 services-integrations**——`KeyringBackend` 本体在 desktop（`app_state/settings/keyring.rs`），env 与 api_key 同一 backend 同文件，避免跨层倒置。

1. `app_state/settings/keyring.rs` 追加：
   ```rust
   pub fn store_env(keyring: &dyn KeyringBackend, server_id: &str, env: &HashMap<String,String>) -> Result<String>
     // serde_json::to_string(env) → keyring entry "mcp-env:{server_id}" → 返回 "__kr_env__"
   pub fn load_env(keyring: &dyn KeyringBackend, server_id: &str) -> Result<HashMap<String,String>>
   ```
2. `io.rs`：`update_app_settings_at` 保存前把各 `MCPServerConfig.env` 非空块 → `store_env` → 磁盘写 sentinel；`load_app_settings_at` 遇 sentinel → `load_env` 还原（失败 warn + 空 map，fail-open 兼容旧数据）。
3. 范围：仅 user 级 `~/.northhing/config/app.json`；project 级 Cursor 格式 mcp 配置的 env 明文是行业惯例，不动。

---

## F1 — 全页数据流（P2a，依赖 P0a）

- `app.rs`：`entries` 启动时 `get_session` 覆盖 seed_session（seed 保留为 empty-session fallback）。
- `pages_archive.rs` STRATA / `pages_space.rs` DOORS：本轮**不动**，仅加 `// TODO(data): wire to session/archive query` 标记——真实归档查询接口不存在，属后续立项。
- `pages_settings.rs` 由 F5 覆盖。

---

## B4 — Event queue 丢事件（P2b，独立）

矛盾消解（复审 F4 项）：`EventQueue::enqueue` 是 **inherent method**（queue.rs:76），`StreamEventSink::enqueue` 是 trait method（返回 `()`）。两者独立改：

1. inherent `enqueue` → 返回 `Result<EventId, EventQueueFull>`；Critical 优先级跳过容量上限直接入队（一行 if）；其余满则 Err。
2. trait `StreamEventSink::enqueue` 签名**不动**；impl 内 `if let Err(e) = ... { tracing::error!(...) }` 返回 `()`。
3. 优先级类型：`AgenticEventPriority`（contracts/events/agentic.rs:7）与 core 侧 `EventPriority` 是 **alias 同一类型**（`events/types.rs:12` use 别名）——单文件同步即可，无双轨。
4. 调用点定位指引：`grep -rn "\.enqueue(" src/crates/assembly/core/src/agentic/`；已知生产点 `stream_processor.rs:192` / `turn_persist.rs:118,198`。
5. 家规 4：无 select!/取消竞争新增，不强制测试；加 1 个单测"满队列时 Critical 入队成功、Normal 返回 Err"。

---

## F6 — Onboarding 流程（P3a，依赖 P0a + P1b）

**文件**：`ui_dioxus/pages_onboarding.rs`（就地改，不建 onboarding_state.rs）

1. 页内 `enum Step { One, Two, Three }` + `current_step` Signal；底栏"下一步"按步校验：
   - One：palette 已选 + agent_name 非空
   - Two：`kernel_facade().test_provider_config(ProviderFormDto { provider_id, base_url, api_key, model, provider_type })`（settings.rs:183，**form 态用这个，不是 test_provider(id)**）通过
   - Three：`std::path::Path::new(&workspace).exists()`
2. 完成副作用：
   - api_key → `store_api_key`（C3 keyring，不落盘）
   - `update_app_settings(|s| { s.onboarding_completed = true; s.add_workspace(path); Ok(()) })`
   - `create_session(SessionConfigDto {...})`（session.rs:237，需构造 DTO）→ 启动首个 session
3. 状态全部页内 Signal，不外泄。

---

## B3 — Cleanup 调度（P3b，独立，范围收窄）

1. `apps/desktop/src/lib.rs` bootstrap（kernel_facade 初始化后、Slint/Dioxus 分支前）：
   ```rust
   tokio::spawn(async move {
       let svc = CleanupService::new(path_manager, CleanupPolicy::default());
       let _ = svc.cleanup_all().await;                    // 启动即跑一次
       let mut tick = tokio::time::interval(Duration::from_secs(86400));
       loop { tick.tick().await; let _ = svc.cleanup_all().await; }
   });
   ```
2. **范围收窄**：snapshot orphan 清理延期——`FileSnapshotSystem` 挂在每 workspace 的 `SnapshotService` 内（`service/snapshot/service.rs:36`），无全局实例；需 per-workspace 服务解析，属独立立项。ledger P2-4 相应改写。
3. 测试：CleanupService 既有测试覆盖 cleanup_all 本身；bootstrap spawn 不另测（无并发原语新增，家规 4 不触发）。

---

## 执行顺序（v3 终版）

| 批 | 内容 | 文件 | 验证 |
|---|---|---|---|
| **P0a** | F3 facade 方法 + B1 api.rs | contracts/kernel-api/tools.rs + kernel_facade/tools.rs + ui_dioxus/api.rs（新） | `cargo check -p northhing` + facade 单测 |
| **P0b** | F2 发送/stop/streaming | app.rs | 手动跑 Dioxus 壳 |
| **P0c** | F3-UI approval 卡 | app.rs | 同上 |
| **P1a** | F4 编年史（Rust fade + Signal） | app.rs | 截图走查 |
| **P1b** | F5 settings 持久化 | pages_settings.rs + io.rs | `cargo test -p northhing --lib settings` |
| **P1c** | B2 mcp env keyring | keyring.rs + io.rs | 单测 roundtrip |
| **P2a** | F1 数据流（room 窗） | app.rs | 手动 |
| **P2b** | B4 event queue | queue.rs + 调用点 | 新单测 |
| **P3a** | F6 onboarding | pages_onboarding.rs | 手动 |
| **P3b** | B3 cleanup 调度 | lib.rs | 启动日志观测 |

每批独立 commit；P0a 是唯一的 contracts 变更批（需同 commit 带 1 单测）。
