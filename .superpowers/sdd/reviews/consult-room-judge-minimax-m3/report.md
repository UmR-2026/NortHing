# consult-room 全局问题处方 — judge 评审（MiniMax-M3，独立视角）

**Brief**: `.superpowers/sdd/consult-room/judge-brief-20260825.md`
**Scope**: 10 项问题（B1-B4 后端 / F1-F6 前端 Dioxus 壳）处方评审
**判定依据**:
- `docs/design/2026-07-22-frontend-redesign/consult-room/` — 视觉真值
- `src/apps/desktop/src/ui_dioxus/` — 当前 Dioxus 壳实现
- `src/crates/contracts/kernel-api/` — frozen host↔kernel 边界
- `docs/status/tech-debt-ledger.md` — P1-8 / P2-4 / P2-6 已登记的债务
- `src/crates/*/AGENTS.md` — 层级边界与归属约束

> 独立视角：本文不复述 brief 既有分析，逐条按"是否对、是否最简"重新判决。

---

## Problem B1 — Dioxus ↔ 后端桥接缺失

**处方**: 在 `ui_dioxus` crate 加 `api` 子模块（`session_api` + `dialog_api`）；通过 `GlobalConfigManager` + `DialogScheduler` 通路接线；"tauri::AppHandle-less 的异步 tokio task 桥接 UI signal → core API"。

- **SPEC: FAIL**
- **QUALITY: FAIL**
- **Notes**:
  1. **"tauri::AppHandle-less" 措辞错位**：v0.1.0 桌面是 **Slint**（`AGENTS.md` 骨干不变量 #1），Tauri 仅属 `northing-installer`。后端桥在 Slint 端已落点为 `kernel_facade`（`app_state/event_bridge.rs:33` 已是 `northhing_core::kernel_facade::kernel_facade()`）。处方把"无 Tauri"当约束写，本身就是误读。ui_dioxus 应当镜像这条路：调 `kernel_facade()` + `subscribe_events`，不另起 `api::session_api`/`dialog_api` 子层。
  2. **归属与重复抽象**：现有 host↔kernel 边界就是 `northhing_kernel_api`（frozen contracts/AGENTS.md）。`session_api`/`dialog_api` 是给 ui_dioxus 加一层 thin wrapper，等于在 Dioxus 一侧复刻 `kernel_facade`。`core/AGENTS.md` 明确"stable DTOs/facts/ports 不要下沉 core；host adapters 归 `src/apps/desktop`"。所以正确的位置是 `src/apps/desktop/src/ui_dioxus/api.rs`（单文件），不要拆两文件，更不要把 `GlobalConfigManager`/`DialogScheduler` 直接暴露到 UI（应当由 kernel_facade 收敛）。
  3. **遗漏关键事实**：现有 `DesktopEventBridge` (`event_bridge.rs`) 已经在做 Slint 侧的"event 订阅 → UI 写入"全链路，是 P0 既有参考实现。ui_dioxus 应当直接克隆其订阅模式（`subscribe_events` → 收 `KernelEventDto` → 走 `tokio::spawn` 转 Dioxus `Signal`）。"通过已有的 DialogScheduler 通路接线"措辞不准——`DialogScheduler` 是 `agentic::coordination` 内部实现，UI 走的是 `kernel_facade::submit_turn`（见 `kernel_facade/turn.rs:49` 用 `global_scheduler()`），不是直接用 `DialogScheduler`。
  4. **遗漏**：处方未指明 streaming 路径用 `KernelEventDto::TextChunk`（frozen events.rs:71）。这是最简复用，不需新事件。

**最简替代**: `ui_dioxus/api.rs` 单文件，调 `kernel_facade()`（与 `event_bridge.rs` 同源），订阅 `KernelEventDto`（TextChunk/TurnState/ToolCall/TurnPhase），UI 侧用 `tokio::sync::mpsc` 收消息再 `use_future` 转 `Signal`。不引入新子模块、不新加 traits、不碰 `DialogScheduler`。

---

## Problem B2 — MCP env 变量明文存 app.json（P1-8）

**处方**: 复用 P1-C3 keyring 模式；env block 整块存入 OS keyring，磁盘仅存 sentinel key；`McpEnvStore::get/set/remove` trait；`persist_app_settings` 前调 `McpEnvStore::flush`。

- **SPEC: PASS**
- **QUALITY: FAIL**
- **Notes**:
  1. **方向对**：ledger P1-8 明确点名 "C3 keyring pattern 可复用"。env 整块存 keyring、磁盘留 sentinel 是合理迁移路径。
  2. **归属错位**：`MCPServerConfig.env` 实际归属 `services-integrations`（`services-integrations/AGENTS.md` + `mod.rs:95`），不在 core 也不在 desktop settings。`services-integrations` 已被 AGENTS.md 锁死"不依赖 app crates / desktop adapters / Tauri"。处方说"`McpEnvStore::flush` 在 `persist_app_settings` 前调"——但 `persist_app_settings` 在 desktop 的 `app_state/settings/io.rs`，services-integrations 不能反向依赖 desktop。正确归属：`McpEnvStore` trait + 实现应落 `services-integrations/src/mcp/server/` 或 `services-integrations/src/mcp/config/`，并通过 `KeyringBackend` port（与 `northhing_core::infrastructure::keyring::KEYRING_SERVICE` 同源）注入；desktop 侧只承担生产 `ProductionKeyring` 实例并注册到 assembly。
  3. **C3 模式的差异被忽略**：C3 处理的是单字符串 sentinel（`API_KEY_SENTINEL = "__kr__"`）。env 是 `HashMap<String, String>`，整块 JSON-序列化为单个 keyring entry 是最简做法，不需要 per-variable sentinel——处方"per-variable sentinel or single keyring entry"的二选一表述过于宽松，落到了"per-variable"，反而复杂。
  4. **接口过宽**：处方定义 `get/set/remove` trait，但调用方只在 `persist_app_settings` 路径用一次"写整块"。"set/remove"在当前路径无独立调用点（删除 = 整块清除）。ponytail 视角：先一个 `McpEnvStore::store(server_id, &HashMap) -> sentinel` + `McpEnvStore::load(server_id) -> HashMap` 就够了，等真出现 per-key 删除需求再扩。

**最简替代**: services-integrations 侧加 `mcp/env_secret.rs`：`pub fn store_env(server_id, env: &HashMap<String,String>) -> Result<&'static str>` 返回 `__kr_env__` sentinel，`pub fn load_env(server_id) -> Result<HashMap<String,String>>`；desktop settings 侧在 persist 路径把 `MCPServerConfig.env` 替换成 sentinel，restore 时再 load——trait 等到有第二消费者再加。

---

## Problem B3 — CleanupService 从未调度（P2-4）

**处方**: 在 `app_state/event_bridge.rs` startup 路径 spawn `tokio::spawn(cleanup_loop)`；每 24h 跑一次 `cleanup_all`；session 删除时触发即时清理。

- **SPEC: PASS**
- **QUALITY: FAIL**
- **Notes**:
  1. **症状与处方对位准**：P2-4 ledger 条目明确"`CleanupService` 已实现但无实例化"；处方补调度 + 24h 周期 + 删除触发，是合理的最小修补。
  2. **位置错位**：`app_state/event_bridge.rs` 是 **Slint 桥**，不是 app startup。Dioxus ui_dioxus 启动路径是 `ui_dioxus/entry.rs::launch()`。Slint 与 Dioxus 两套启动路径互不感知（`DIOXUS_SHELL` 标志切换），把调度塞进 event_bridge 等于只在 Slint 路径生效，Dioxus 路径完全不跑——与本次 P0 前端修复目标（让 Dioxus 壳真正可用）直接冲突。
  3. **正确位置**：`apps/desktop/src/lib.rs`（或 `main.rs`）的 app bootstrap 段，在 `kernel_facade()` 初始化之后、两路启动之前 spawn——Slint 与 Dioxus 共用同一后台循环。这同时满足 AGENTS.md "UI thread discipline" 边界（spawn 是 tokio 任务，不碰 UI 线程）。
  4. **24h `tokio::time::interval` 是 stdlib 一行**，不需要 `cron` 或 `tokio-cron-scheduler` 依赖（不存在该依赖，但处方暗示的方向可能引入）。session 删除即时触发走 `session_manager` 的删除回调即可，不要在 `CleanupService` 侧再加一个观察者路径。
  5. **漏掉一处**：`cleanup.rs:115` 提到的 `cleanup_orphaned_snapshots` 仍未被 `cleanup_all` 收录。处方"session 删除时触发即时清理"应当涵盖该路径，否则 ledger 描述的"orphan snapshots"永远清不掉。

**最简替代**: 在 `apps/desktop/src/lib.rs` bootstrap 段一次性 `tokio::spawn(async move { let svc = CleanupService::new(path_manager, CleanupPolicy::default()); let mut tick = tokio::time::interval(Duration::from_secs(86400)); loop { tick.tick().await; let _ = svc.cleanup_all().await; } })`；session 删除回调加 `snapshot_system::cleanup_orphaned_snapshots()` 一行。

---

## Problem B4 — Event queue 满时静默丢事件（P2-6）

**处方**: `enqueue` 返回 `Result<(), EventQueueFull>`；Critical 永不丢（block + 扩容）；非 Critical 满时返回 Err 让调用方决定。

- **SPEC: PASS**
- **QUALITY: PASS**（带一个 caveat）
- **Notes**:
  1. **症状对位准**：ledger P2-6 明确"`queue.rs:85` 满载静默 Ok + `StreamEventSink::enqueue` 用 `let _` 吞掉"。处方返回 `Result`、不丢 Critical、让调用方决定——三项正好是 P2-6 列出的修复项。
  2. **Critical 永不丢的落点**：`EventPriority::Critical` 已在三个生产点使用（`stream_processor.rs:192` / `turn_persist.rs:118,198`）。当前 `enqueue` 完全不看 priority；处方补"Critical 不丢"等价于把 priority 提升到 enqueue 的核心约束——这正是 P2-6 想要的。
  3. **实现层 caveat**："block + 扩容"措辞过实：阻塞 enqueue 会让 producer（model stream / tool result）反压，而 `StreamEventSink` trait 当前签名是 `async fn enqueue` 已是 future-friendly，加 `await on full for Critical` 即可，不需要"扩容"——扩容反而引入 capacity 不收敛风险。最简：`is_critical = priority == EventPriority::Critical`，Critical 走 `loop { match queue.push_back(...) { Ok => break, Err(TrySendError::Full(Q)) => Q.notify.notified().await } }`；非 Critical 直接 `try_send` 失败返回 `Err(EventQueueFull)`。让 `StreamEventSink::enqueue` 的现有 `let _ = ...` 改为 `.unwrap_or_else(|e| error!(...))` 让丢事件能被日志抓到。
  4. **遗漏**：`AgenticEventPriority`（`contracts/events/agentic.rs:692,726`）与 `EventPriority` 是两个类型（前者 contracts 侧、后者 core 侧 queue），都需要同步检查 Critical 永不丢的约束是否覆盖到 contracts 侧消费者——处方未提。

**最简替代**: 改 `queue.rs::enqueue` 签名返回 `Result<(), EventQueueFull>`；Critical 走 `notified().await` 反压，非 Critical 失败即 Err；`StreamEventSink::enqueue` 把 `let _` 改成 `error!` 记录丢失。

---

## Problem F1 — 零真实数据流

**处方**: Step 1 定义 `AppEvent` enum（SessionList / SessionTurns / SendMessage / SettingsUpdate）；Step 2 加 `event_bus.rs`（tokio mpsc）；Step 3 page components 用 `use_future` 消费；Step 4 后端 B1 桥接接上。

- **SPEC: NEEDS CONTEXT**
- **QUALITY: FAIL**
- **Notes**:
  1. **方向对**：ui_dioxus 当前 100% mock（`session_mock.rs` + `state.rs` Signal 局部状态），不接 kernel 是个真洞。但处方是"分步渐进"，且 Step 4 依赖 B1——B1 我已判 FAIL，所以 F1 的 Step 4 没有可落地的桥可接，整个分步计划悬空。
  2. **`AppEvent` enum 的归属问题**：处方把 `AppEvent` 当 Dioxus UI 内部事件。这没问题，但要明确它**不是** kernel 事件（kernel 事件是 frozen `KernelEventDto`）。换句话说，"SendMessage" 不是发到 core 的事件，而是触发 `kernel_facade().submit_turn()` 的本地 command；"SettingsUpdate" 同理——这两者是 UI-side intent，与 `KernelEventDto` 是正交两个轴。处方把两者混在一个 enum，命名会误导后续 reviewer。
  3. **缺设计真值**：brief 让评审"以 consult-room 设计真值为准"。`PAGES-BRIEF.md` 描述了 4 页（onboarding/settings/archive/space），但**没有任何文件描述数据流契约**。这是 NEEDS CONTEXT 的根因——评审无法独立验证"SendMessage 应触发 `submit_turn` vs `submit_user_input` vs 其他"这种选择是否对。
  4. **event_bus 抽象过重**：现有 `state.rs::GlobalTheme` 已经演示了"用 `tokio::sync::watch` 跨窗口同步状态"的模式。`mpsc` 适合 producer→consumer 单向，但"页面消费 core 推送"本质是**多订阅者拉取最新值 + 变更通知**——`broadcast::channel` 或 `watch` 比 mpsc 贴合。Dioxus 0.8 的 `use_future` + `changed().await` 模式已在 `app.rs:106-117`、`pages_settings.rs:54-65`、`pages_onboarding.rs:63-74` 三处演示，重复一遍不需要新加 `event_bus.rs`。
  5. **遗漏**：当前 mock 的 `seed_session()`（`session_mock.rs:53`）是设计真值的字面快照。F1 的 Step 1 隐含"丢掉 seed_session"，但 `DELIVERY-NOTES.md` 明确"v4 主 = 系统完整"且 Dioxus 迁移注释（`app.rs:9`）写明"mock 会话流：Signal 直推"。所以 F1 应当保留 seed_session 作为初次加载 fallback，而非"全部替换为 kernel 拉取"。

**最简替代**: 不加 `event_bus.rs`。`ui_dioxus/api.rs` 单文件暴露 `submit_turn` / `load_session` / `list_sessions` 三个函数（薄封装 `kernel_facade()`）；ui_dioxus 各页用 `use_future` 调这些函数，结果存进 per-window Signal；后端推送用 `subscribe_events` 走 `use_future` 转 Signal。`AppEvent` enum 只装 UI intent（SendMessage、SwitchSession、Approve、Reject、SettingsToggle），不混入 core 推送。

---

## Problem F2 — 消息发送无 handler

**处方**: 实现 `on_send_message` → emit `AppEvent::SendMessage(text)` → event_bus → 后端 B1 桥接 → `DialogScheduler::submit`；streaming 用 `use_future` 消费 `StreamingUpdate` event 逐 token 渲染。

- **SPEC: NEEDS CONTEXT**
- **QUALITY: FAIL**
- **Notes**:
  1. **症状描述与现实不符**：处方把"send button 只 toggle streaming"列为问题；事实是当前 `app.rs:368-376` 的 send 按钮**就是 toggle streaming**——这是 brief 描述的"演示态"，并非"handler 缺失"。**真正的缺口**是：(a) input box 是 div（`app.rs:363-367`）而非 `<input>`，根本没有文本可读；(b) send 没有真的提交任何内容。F2 应当从这两点出发，不是从"handler 接 DialogScheduler"出发。
  2. **`StreamingUpdate` 不存在**：kernel-api 只有一个 `KernelEventDto::TextChunk { session_id, text }`（frozen）。`StreamingUpdate` 是处方自创命名，对不上既有契约。要么用 `TextChunk`（推荐），要么明确这是 Dioxus 内部 wrapper（不必要）。
  3. **直接调用 `DialogScheduler::submit` 错位**：与 B1 同根错误。UI 侧应调 `kernel_facade().submit_turn(...)`，由 facade 内部走 `global_scheduler()`。直接 `DialogScheduler::submit` 绕过了 facade 隔离层——按 `core/AGENTS.md` 这是把"concrete scheduler/session lifecycle execution"反向暴露给 UI，违反层级。
  4. **遗漏**：send 按钮的"➤ / ■" 合一按钮语义来自 `DELIVERY-NOTES.md` §chrome 段（"发送/停止合一按钮"）。处方只讲"发"，未讲"停"——而 `app_state/streaming_lifecycle.rs` 已有 `SlintDispatcher` 模式，Dioxus 侧要做等价 stop handler（调 `kernel_facade().cancel_turn(active_turn_id)`）。

**最简替代**: `app.rs` 把 input-box div 换 `<input>`（dioxus 0.8 有 `use_signal` + oninput）；send 按钮 onclick 调 `submit_turn(text)`；stop 按钮（streaming 时显形）调 `cancel_turn`；token 流靠 `subscribe_events` 收 `TextChunk` 进 Signal 增量追加。

---

## Problem F3 — Approval 卡 approve/reject 无 handler

**处方**: approve → emit `AppEvent::Approve(call_id)` → 走 tool_confirmation 确认门 → 返回结果渲染；reject → emit `AppEvent::Reject(call_id)` + 可选文本输入框。

- **SPEC: NEEDS CONTEXT**
- **QUALITY: FAIL**
- **Notes**:
  1. **当前 mock 数据里 approval 是历史快照**：seed_session 里两个 Approval 一个 resolved=false 一个 resolved=true（`session_mock.rs:73-86`）。resolved=true 的不应该有按钮——这与处方"approve/reject 按钮"前提冲突。处方没澄清"只对 unresolved 卡加 handler"。
  2. **`tool_confirmation` 不是 host-facing API**：`tool_confirmation` 在 `agent-runtime`（execution 层），是内部 channel，UI 不应直接依赖。`agent-runtime/AGENTS.md` 第 4 行明确"agent-runtime 不依赖 northhing-core、app crates、Tauri、ACP protocol、web UI、concrete service crates"。处方让 ui_dioxus "走 tool_confirmation 确认门"是层级倒置。
  3. **真实接法**：UI 应通过 `kernel_facade().respond_to_tool_confirmation(call_id, decision: bool, optional_text: Option<String>)`（或等价 DTO），具体实现路径在 `kernel_facade/tools.rs` 或 `kernel_facade/session.rs`——这需要先确认 facade 暴露了这条命令。**brief 没有提供 facade API 表**，所以这是 NEEDS CONTEXT。
  4. **遗漏**：reject 后"可选文本输入框"是 UX 决策。`tool_confirmation` 内部支持 reject reason 但不是必填；处方未指明是必填还是可选，与"可选"措辞含糊。`DELIVERY-NOTES.md` 设计真值强调"诗意<功能"——硬塞输入框违反"诗性克制"红线，应默认不提供输入框。

**最简替代**: 渲染时按 `resolved` 分流（`app.rs:565` 已有此分支）；unresolved 卡按钮 onclick 调 `respond_to_tool_confirmation(call_id, true/false)`，无文本输入框；resolved 卡不绑事件。

---

## Problem F4 — 编年史条空白

**处方**: 加 CSS animation：`@keyframes chronicle-shift` 用 `background-size` + `background-position` 做 30s 循环渐移位；`chronicle-bar` 设 `background: linear-gradient(...)` 含 mind 色 token。

- **SPEC: FAIL**
- **QUALITY: FAIL**
- **Notes**:
  1. **直接违反设计真值**：truth HTML 548-592 行（已读到）明确编年史条是 **JS rAF 动态更新 `linear-gradient` stops**，不是 CSS `@keyframes`。`consult-room-main.css:96-99` 注释（中文乱码）写明"动态 stop 位置用…而固定 stop 渐变 + 层级宽一次（property animation）。近零。"——作者明确否决了 CSS animation 方案。
  2. **直接违反 Dioxus 迁移约束**：`app.rs:9` 注释"真值 JS/rAF 一律不移植"；`css.rs:5-8` 注释"CSS 原样内联（禁翻译成 Rust 样式）"；`css.rs:26` 注释"`TRUTH_CSS` 逐字节锁死（`assert_truth_css_byte_count` 守卫必过，禁改真值 CSS 文件）"。在 Dioxus 内自己造一个 CSS animation 等于伪造了真值 CSS 不存在的东西——下次真值 CSS 同步会与这段冲突。
  3. **30s 固定周期丢失语义**：truth 是"换色（新色自右进入）→ 旧 stop 慢慢左移沉降"，无换色时不动。`@keyframes chronicle-shift` 30s 循环意味着 idle 状态也跑——与"右端≡现在"语义冲突，会让用户以为"它一直在思考"（错信息）。
  4. **遗漏**：truth JS 用 `mixHex(BIRTH, color, age_fraction)` 把历史色褪向出生灰；这层语义完全丢。
  5. **实现层**：Dioxus 0.8 有 `use_future` + `request_animation_frame` 不存在（浏览器侧 raf 是 `gloo` crate 提供，但 ui_dioxus 用 WebView2，rAF 是可用的）。最简做法：组件挂载时 `use_future` 启一个 rAF 循环，每帧读 `now_color` / `history` Signal，重算 stops，写入 inline style。

**最简替代**: 不动 `TRUTH_CSS`。`ui_dioxus/app.rs` 里 `chronicle-bar` 的 div 改为带 inline style 的 div：`style: format!("background: linear-gradient(90deg, {})", stops_str)`；`use_future` 启 rAF 循环，根据当前 mind 色 + 历史 Signal 算 stops 写入。当 mind 色 Signal 变化时自动重渲。不引入 CSS keyframes。

---

## Problem F5 — Settings 无持久化

**处方**: Step 1 加 `SettingsState` struct；Step 2 `settings_store.rs`（mpsc → 后端）；Step 3 toggle 时 emit `AppEvent::SettingsUpdate`；Step 4 启动时从后端 load。

- **SPEC: PASS**（在设计真值层面）
- **QUALITY: FAIL**
- **Notes**:
  1. **症状对位准**：`pages_settings.rs:84-91` 显示 8 个 `use_signal`（engine、providers、mcps、display）纯本地，关闭即丢——这是真问题。
  2. **`SettingsState` struct 是无意义抽象**：Dioxus Signal 已经是结构化状态载体。把 `engine/providers/mcps/display` 收成 struct 仅在 Rust 类型层有意义，但 `use_signal(SettingsState::default())` 与 8 个独立 Signal 在功能上等价；唯一好处是序列化一次——这恰恰是 Step 4 的 `load` 需要的。问题是 Dioxus 的 `use_signal` 已经能存任何 `Serialize` 值，可以直接 `use_signal(|| load_initial())`，不需要专门 `SettingsState` 类型名（除非要 derive Serialize/Deserialize）。
  3. **`settings_store.rs` 重复抽象**：Step 2 说"mpsc → 后端"。当前 `app_state/settings/{io,sync,keyring}.rs` 已经是 desktop 侧 settings 的"store + persist + keyring"完整层（`AGENTS.md` 骨干不变量："desktop AppSettings retains workspaces/onboarding"）。ui_dioxus 应当复用 `app_state/settings/io.rs::load_app_settings` + `persist_app_settings`（这两个都是公开 API），不要再造一层 mpsc 通道——直接调函数即可。
  4. **`AppEvent::SettingsUpdate` 错位**：与 F1 同根——UI 端"toggle 引擎"是 UI intent，emit 后应当直接调 `persist_app_settings(&state)`，不需要绕 event bus。这是把"事件总线"概念套在了同步 IO 上。
  5. **遗漏**：当前 `pages_settings.rs` 的 interactive mock 没有"提交"按钮，toggle 是即时生效。F5 应当明确：每个 toggle 单独 persist（debounced 即可），不需要"submit"语义。

**最简替代**: `ui_dioxus/settings_io.rs` 一文件，封装 `load_initial_settings() -> SettingsState` / `save_setting(field, value)`；`pages_settings.rs` 把 8 个 `use_signal` 换成 `use_signal(load_initial_settings)` + 每个 toggle 调 `save_setting`（防抖 500ms 用 `tokio::time::sleep` + cancellation token，或干脆每次都写——settings IO 频率低，debounce 是 over-engineering）。

---

## Problem F6 — Onboarding 无流程控制

**处方**: `onboarding_state.rs`：step 1→2→3 状态机；完成时 emit `AppEvent::OnboardingComplete { agent_name, palette, provider, workspace }` → 写入 AppSettings + 启动 session。

- **SPEC: PASS**
- **QUALITY: FAIL**
- **Notes**:
  1. **症状对位准**：`pages_onboarding.rs:78-83` 有 `selected_palette` / `tested_connection` / `ritual_completed` 等 5 个 `use_signal`，step 间无约束（用户可随意跳），无 completion 落盘——这是真问题。
  2. **"状态机"是过度抽象**：3 步线性流用 `use_signal(Step::One)` + `match` 即可（`Step::One | Step::Two | Step::Three`，每步完成条件 `next` 按钮 enabled）。`onboarding_state.rs` 单文件加 enum + 三个 `can_proceed` 谓词即可，"状态机"措辞暗示引入 state-machine 库（`statig` / `rust_fsm`），违反 ponytail。
  3. **`OnboardingComplete` payload 设计错位**：palette（"颜色选择"）+ agent_name（"名字"）+ provider（"provider 配置"）+ workspace——这 4 个字段语义层级不同：provider 配置含 api_key（敏感），workspace 是路径。处方把它们塞一个 struct emit——按 P1-C3 模式 api_key 必须走 keyring，不应进 `OnboardingComplete` 直传。
  4. **"启动 session" 含糊**：`onboarding_complete` 应当触发什么？"启动一个 session"是 facade 的 `kernel_facade().create_session(...)`，不是 emit 一个事件就让某处 spawn session。这步应当在 ui_dioxus 层调 facade API，与 F1/B1 修正路径同源。
  5. **遗漏**：当前 `pages_onboarding.rs:81` 的 `tested_connection: use_signal(false)` 是 mock 状态（没有真测）。F6 应当明确"completion 判定"包含 connection test 实际跑（用 provider HTTP probe，参考 `kernel_facade/settings.rs::test_provider_connection` 之类是否已存在），而非"测过 = true"按钮 toggle。

**最简替代**: `pages_onboarding.rs` 内 `enum Step { One, Two, Three }` + `use_signal(Step::One)`；每步底栏"下一步"按钮 enabled 条件是本步必填字段非空；完成时调 `save_onboarding(&bundle)`（其中 api_key 走 `store_api_key`） + `create_session(...)`。不另起 `onboarding_state.rs`，不引 `AppEvent::OnboardingComplete`。

---

## Summary

- **Spec compliant**: 5/10（B2 PASS / B3 PASS / B4 PASS / F5 PASS / F6 PASS），1 NEEDS CONTEXT（F1 因依赖 B1 悬空 / F2 因 StreamingUpdate 不存在 / F3 因 facade API 未明），3 FAIL（B1 tauri 措辞错位 / F4 直接违反设计真值）。
- **Quality concerns**（最关键 3 条）：
  1. **B1 + F1 + F2 同根问题**：处方反复提"tauri::AppHandle-less / DialogScheduler / StreamingUpdate"，但 v0.1.0 桌面是 Slint 而非 Tauri；正确接法是 `kernel_facade()`（Slint 端 `event_bridge.rs:33` 既有示范）+ frozen `KernelEventDto::TextChunk`。把 UI 直接接到 `DialogScheduler` / 自创事件名 = 绕过 facade 隔离层（`core/AGENTS.md` 明确禁止）。
  2. **F4 直接违反设计真值**：truth HTML 用 JS rAF 动态 stop 迁移，`@keyframes chronicle-shift` 30s 循环是错的方案——idle 状态不该跑。Dioxus 迁移规则又锁死了 CSS 文件不动。该项需要"不改 TRUTH_CSS、写一个 rAF use_future 重算 stops"。
  3. **B2 / F5 / F6 同根问题**：抽象层数加 1 但落点错——`McpEnvStore::get/set/remove`（trait 过宽）、`SettingsState` + `settings_store.rs`（mpsc 套同步 IO）、`OnboardingComplete` struct（敏感字段没走 keyring）。ponytail 视角：单文件、单方向、最小 trait 表面，能复用 `app_state/settings/` 已有层就别新加。
- **Recommended priority order**（与 brief 不一致）：
  1. **B1 修正处方后提升到 P0**（原 P0）：没有它 F1/F2/F3 全卡。是当前最大阻断点。
  2. **B4 保留 P2**（保持原优先级）：3 行核心改动 + 1 处 log 升级，独立闭环，不依赖其他项。
  3. **F4 升 P0**（原 P1）：违反真值 = 违反设计红线，必须先于其他 F 项处理，否则会污染交付。
  4. **F1 + F2 + F3 一起 P0**：彼此互锁（都依赖 B1 修正），分派时序应为 F1 (api.rs) → F2 (send handler) → F3 (approval handler)。
  5. **F5 + F6 保持 P1**：settings/onboarding 持久化相对独立，可在 P0 落定后并行。
  6. **B2 P1 保持**：复用 C3 模式但归属修正（services-integrations 而非 core/desktop），落点确定后是 mechanical 工作。
  7. **B3 P2 降为 P3**：cleanup 周期任务不影响功能交付，24h 是软约束，session 删除即时清理与 cleanup_all 是正交两件事——可拆成两个 sub-task 分别估值。

---

## Caveats（自评）

- 不做编译审查：未跑 `cargo check`，未确认 facade API 的函数签名。F1/F2/F3 的 NEEDS CONTEXT 部分（如 `respond_to_tool_confirmation` 是否存在、`submit_user_input` vs `submit_turn` 的取舍）需 facade 侧核对。
- 不做视觉审查：F4 的实现层建议（rAF use_future）正确性需在 `cargo run -p northhing --features ui-dioxus` 后观察动画曲线验证。
- 评审仅基于读代码 + ledger + 设计真值文档；未读 session/turn 全部细节，judgment 标注的"最简替代"以 1-2 文件为约束，更大重构可能漏看。
