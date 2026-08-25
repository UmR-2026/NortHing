# Prescription v2 Review — consult-room

**Reviewer**: MiniMax-M3（独立视角，独立 grep/读源验证）
**Date**: 2026-08-25
**Scope**: `prescription-v2-20260825.md` vs. 三方 judge（minimax-m3 / step-explore / ox-alpha）findings

> 本评审不参考既有 step-explore-prescription-v2-review/（避免互相污染），从源码回放每条处方，独立 verdict。
> 证据锚点：kernel-api 合约（frozen contracts）、app_state/event_bridge.rs（Slint 既有参考实现）、app_state/settings/io.rs、queue.rs、F4 truth HTML 行 548-592。

---

## Check B1（Dioxus ↔ 后端桥接缺失）

- **Judge findings resolved**: **PARTIAL**
- **New over-engineering introduced**: **NO**
- **Feasible**: **NEEDS CLARIFICATION**
- **Notes**:
  - ✅ **minimax-m3 FAIL 修正**：`tauri::AppHandle-less` + DialogScheduler 直连措辞已删除；v2 走 `kernel_facade()`（`src/crates/assembly/core/src/kernel_facade/mod.rs:36`，function-level OnceLock getter），与 Slint 端 `event_bridge.rs:33` 同源。这是正确修正。
  - ✅ **step-explore FAIL 修正**：v2 不引入 `AppEvent` / `event_bus.rs` / `session_api` + `dialog_api` 子模块，单文件 `api.rs`。Ponytail 合规。
  - ✅ **ox-alpha 补救**：下行事件泵在处方中显式定义为 `subscribe_events()` + `EventQueue::subscribe()` broadcast；明确复用 Slint 侧 `app_state` 编排逻辑（仅"写 UI 属性"那层换 Dioxus signal）。
  - ⚠️ **API 签名错位**（实现前必改）：
    1. `submit_turn(text: &str) -> Result<String>` — 实际签名是 `KernelTurnApi::submit_turn(input: TurnInputDto) -> Result<DialogSubmitOutcomeDto, KernelError>`（contracts/kernel-api/src/turn.rs:80）。`text: &str` 必须包成 `TurnInputDto`；`String` 不能直接返回，而应解 `DialogSubmitOutcomeDto.turn_id: TurnId`。
    2. `cancel_turn(turn_id: &str) -> Result<()>` — 实际名称是 `stop_turn`，参数类型 `turn_id: &TurnId`（newtype），不是 `&str`。命名/类型双错。
    3. `subscribe_events() -> impl Stream<Item = KernelEventDto>` — 实际签名是 `subscribe_events(callback: Box<dyn Fn(KernelEventDto) + Send>) -> Result<SubscriptionId, KernelError>`（callback 模型，非 stream）；async + 1 个 boxed closure。无原生 `Stream` 适配。实现时必须自行 spawn 一个 bridge task 收 callback → 写入 `tokio::sync::mpsc` 或 use `tokio::sync::watch` channel，再 `use_future` poll。这层包装处方未给细节。
    4. `confirm_tool(call_id: &str, approve: bool)` — 见 **F3**，**不存在** facade 方法。
  - 整体可行但**实现前必须按真实签名替换 4 处函数名/参数类型**——尤其 `submit_turn`、`stop_turn` 名字，不替换会让 implementer 在 facade 上找不到方法。

---

## Check B2（MCP env 明文落盘 / P1-8）

- **Judge findings resolved**: **YES**
- **New over-engineering introduced**: **NO**
- **Feasible**: **YES**
- **Notes**:
  - ✅ **minimax-m3 Q 警告**：trait `get/set/remove` 过宽 → v2 改为 `store_env`/`load_env` 两个独立函数，无 `McpEnvStore` trait。PASS。
  - ✅ **ox-alpha 边界提醒**：只覆盖 user 级 `app.json`，不碰 project 级 Cursor 格式 mcp 配置——v2 在 settings/io.rs 落点已隐含限定，PASS。
  - Sentinel `"__kr_env__"` 简单清晰，与 C3 `API_KEY_SENTINEL` 同形；JSON 块整存 keyring 一个 entry 是最简做法。
  - Ponytail 合规：~60L 单文件、不引 trait、不加 sentinel-per-key、不加 per-server 配置。
  - 归属正确：`services-integrations/src/mcp/env_secret.rs`（与 `services-integrations/AGENTS.md` 锁死的"不依赖 app crates"边界吻合）；desktop settings/io.rs 走 `store_env`/`load_env` 做 sentinel 替换。
  - 无遗留 concern。

---

## Check B3（CleanupService 从未调度 / P2-4）

- **Judge findings resolved**: **PARTIAL**
- **New over-engineering introduced**: **NO**
- **Feasible**: **NEEDS CLARIFICATION**
- **Notes**:
  - ✅ **minimax-m3 Q FAIL 修正**：位置 `app_state/event_bridge.rs` → `apps/desktop/src/lib.rs` bootstrap 段，PASS。
  - ✅ **ox-alpha 补救**：orphaned snapshots + 启动即跑均在处方中（`snapshot_system::cleanup_orphaned_snapshots().await` + 启动时 `let _ = svc.cleanup_all().await`）。
  - ✅ Stdlib-only：`tokio::spawn` + `tokio::time::interval`（无新 crate / 无 cron 库），Ponytail 合规。
  - ⚠️ **签名错位**：处方写 `snapshot_system::cleanup_orphaned_snapshots().await`，暗示是 free function / static method。实际定义是 `pub async fn cleanup_orphaned_snapshots(&mut self) -> SnapshotResult<usize>`（`service/snapshot/snapshot_system.rs:446`，在 `FileSnapshotSystem` impl 块内，line 27）。需要 `&mut FileSnapshotSystem`，目前没有全局 `FileSnapshotSystem` 实例公开——`service.rs:36` 在初始化时才构造，session 删除回调链尚未明确。**处方需补一句 "通过现有 service provider 句柄获取 SnapshotSystem"，否则 implementer 无法定位。**
  - ⚠️ **测试缺失**：ox-alpha 原判 "按家规 #4 必须带自动化测试"（cancellation / 24h loop 类），v2 处方未提及测试要求。需补："session 删除 callback + cleanup 跑一次后确认 orphan snapshot 被删" 至少 1 个集成测试。
  - ⚠️ **session 删除回调位置不清**：处方说"session 删除回调加一行"，但当前 session 删除 API 是 `KernelSessionApi::delete_session`（contracts/kernel-api/src/session.rs:257），facade 这层之后由谁触发 callback？implementer 需要明确："在 `delete_session` 实现尾部追加 `cleanup_orphaned_snapshots`" 还是另起一个 wrapper？处方没说。

---

## Check B4（Event queue 静默丢事件 / P2-6）

- **Judge findings resolved**: **PARTIAL**
- **New over-engineering introduced**: **NO**
- **Feasible**: **NEEDS CLARIFICATION**
- **Notes**:
  - ✅ **ox-alpha Q FAIL 修正**：原 "block + 扩容" 改为 "Critical 跳 cap（一行） + 其余返回 Err + 补 call site 日志"。PASS。
  - ⚠️ **minimax-m3 caveat 未正面回应**：caveat "AgenticEventPriority vs EventPriority 两类型需同步检查"。实际两者是**同一类型两个别名**（`AgenticEventPriority` 在 contracts/events/agentic.rs:7 定义；`EventPriority` 在 core 是 `use` alias（`events/types.rs:12`），在 agent-stream 是 alias（`agent-stream/types.rs:13`））。所以 caveat 实际不成立，但**处方只在「反馈」段一行字带过（line 138），没在 prescription body 里说"两者是同一类型"**——会让 reviewer 误以为需要双轨同步检查。**建议在 prescription 主体明确"两者是 alias，同一文件同步即可"。**
  - ⚠️ **处方内部矛盾**："改 `enqueue` 签名：`Result<(), EventQueueFull>`" 同时 "不改 `StreamEventSink` trait 签名（影响面太大）"。但**实际 `StreamEventSink::enqueue`（queue.rs:226）是 trait method**（`impl StreamEventSink for EventQueue`），签名是 `async fn enqueue(&self, event: AgenticEvent, priority: Option<EventPriority>)` ——**返回 `()`，不是 `Result`**。所以：
    - 改 `EventQueue::enqueue`（inherent，queue.rs:76）签名为 `Result` 与改 trait 是两件独立的事——inherent 改了不影响 trait。inherent 改 `Result<(), EventQueueFull>` 即可。
    - 但 `StreamEventSink::enqueue` 实现内调 inherent `enqueue` 后，**怎么把 Err 传给 caller**？trait 签名锁死 `()`，不可能。要么改 trait（影响所有 impl，5 个 mock + 4 个生产点），要么把 loss 攒到一个内部计数器周期 log，trait impl 内部 error!() 但仍返回 `()`。
    - 处方说"不改 trait 签名"是**正确的 Ponytail 选择**（避免下游契约变动），但矛盾需要消解：**处方应写明"在 StreamEventSink::enqueue 内捕获 Err 记 tracing::error!，返回 () 不变；inherent enqueue 仍然返回 Result 供新调用点显式处理"。** 这才是自洽路径。
  - ⚠️ **签名语法错误**：处方说"let `_` = 改为 `.map_err(|e| tracing::error!(...))`"——但 `StreamEventSink::enqueue` 返回 `()`，**不能 `.map_err`**。正确写法：`if let Err(e) = inner { tracing::error!(...) }` 或 `match inner { Ok => {}, Err(e) => tracing::error!(...) }`。需在 prescription body 里写对。
  - ⚠️ **call sites 未枚举**：处方只说"~10 处 `StreamEventSink::enqueue`"。grep 实测：`apps/desktop/src/app_state/event_bridge.rs`、`adapters/ai-adapters/tests/common/stream_test_harness.rs` 等只看到 mock/test impl；真正的 ~10 处必然在 production 调用点（stream_processor.rs:192 / turn_persist.rs:118,198 + 其它）。**应附 `grep -r "enqueue.*EventPriority" src/` 指引**，否则 implementer 会全仓盲搜。

---

## Check F1（零真实数据流）

- **Judge findings resolved**: **YES**
- **New over-engineering introduced**: **NO**
- **Feasible**: **YES（依赖 B1）**
- **Notes**:
  - ✅ **step-explore Q FAIL 修正**：v2 明确"不引入：AppEvent enum / event_bus.rs / mpsc channel"。PASS。
  - ✅ **ox-alpha 建议**："AppEvent 拆 command/data 两条通道"被标记为"执行备注"而非结构变更——yagni 合规（除非出现第二消费者）。
  - ✅ 与 B1 共享 `api.rs` 单文件，无重复抽象。
  - ⚠️ **api.rs 接口签名继承 B1 错位**：`list_sessions` / `load_session` 在 facade 上确实存在（`session.rs:241, 253`），但**返回类型**分别是 `Vec<SessionSummaryDto>` 和 `SessionDto`（来自 contracts），不是处方里"Result<Vec<SessionSummary>>"简化名。implementer 需对接 DTO 完整字段，不能省略中间层。这一点是固有的、需要细化。
  - ✅ **seed_session 保留 fallback** 正确——`pages_archive.rs:113-120` 静态 STRATA 不动是对的，避免一次性全改。
  - 可行性依赖 P0a 完成（api.rs 落地）。

---

## Check F2（消息发送无 handler）

- **Judge findings resolved**: **PARTIAL**
- **New over-engineering introduced**: **NO**
- **Feasible**: **NEEDS MINOR CLARIFICATION**
- **Notes**:
  - ✅ **minimax-m3 NEEDS CTX**：input-box div → dioxus `<input>`，streaming 时调 `cancel_turn` —— 方向正确（v2 把按钮拆 "streaming 时 → cancel" + "非 streaming → submit"）。
  - ✅ **step-explore FAIL**：不走 event_bus；直接调 `api.submit_turn` / `api.cancel_turn`。PASS。
  - ✅ 不引入 `StreamingUpdate` event，复用 frozen `KernelEventDto::TextChunk`。
  - ⚠️ **API 名/类型错位**（继承 B1）：
    1. `api.submit_turn(&user_input)` → 实际是 `kernel_facade().submit_turn(input: TurnInputDto)`，需要构造 `TurnInputDto { session_id, content: vec![MessageContentDto::Text(user_input)] }`（按 turn_input DTO 实际字段，implementer 必须先看 kernel-api/src/turn.rs 顶端或 DTO definitions）。
    2. `api.cancel_turn(active_turn_id)` → 实际 `kernel_facade().stop_turn(turn_id: &TurnId)`，`TurnId` 是 newtype。
    3. `api.submit_turn 返回 String` 实际是 `DialogSubmitOutcomeDto`。需要解 `.turn_id`。
  - ⚠️ **`onkeydown` Enter 行为**：处方说"if e.key() == 'Enter' { send(); }"——Dioxus 0.8 在 WebView2 下 `e.key()` 返回值需 verify 是否含 IME 状态过滤（中/日输入法时 Enter 不应触发）；非阻塞但需 implementer 注意。
  - 可行性依赖 B1（P0a）落地。

---

## Check F3（Approval 卡 approve/reject 无 handler）

- **Judge findings resolved**: **PARTIAL**
- **New over-engineering introduced**: **NO**
- **Feasible**: **NEEDS MAJOR CLARIFICATION**
- **Notes**:
  - ✅ **ox-alpha 方向对**：走既有确认门（tool_confirmation / agent-runtime 内部 channel），不引入新依赖。
  - ✅ **reject 不加输入框**：与 `DELIVERY-NOTES` "诗意<功能"红线一致。
  - ❌ **minimax-m3 NEEDS CTX 未解**：`api.confirm_tool(call_id: &str, approve: bool)` —— 此方法**在 `KernelFacade` 上根本不存在**。
    - 实测：`KernelToolsApi` trait 只暴露 `list_tools` / `register_tool` / `request_user_input`（contracts/kernel-api/src/tools.rs:84-93），**无 `confirm_tool`**。
    - 真正的 `confirm_tool(tool_id: &str, updated_input: Option<Value>)` 在 `coordination/dialog_turn/coordinator_session.rs:219`（私有）+ `tools/pipeline/tool_pipeline/pipeline_pre.rs:186`，通过 `DashMap<String, oneshot::Sender<ConfirmationResponse>>` 内部 channel 走。
    - 这意味着 approve/reject 流程的 user-facing API **不存在**——所有现有 approve 都走 `core_adapter::confirm_tool`（CLI/Slint 端旁路），未暴露到 frozen kernel-api。
  - ⚠️ **处方路径不可行**：implementer 会撞上"找不到 facade 方法"。**处方必须二选一**：
    - **路径 A**（推荐，Ponytail 不增加新 method）：在 Dioxus 侧直接持有 `ConversationCoordinator` handle，调 `coordinator_session.confirm_tool(...)`（`Arc` 访问 core 内部构造）。但这违反"不暴露 concrete scheduler/session lifecycle"层级规则——`core/AGENTS.md` 明确禁止。
    - **路径 B**（scope 扩张）：新增 `respond_to_tool_confirmation(tool_id, approve, reason)` 方法到 `KernelToolsApi` trait + kernel-facade impl + 内部调度到 coordinator。这要改 `contracts/kernel-api/src/tools.rs` + `kernel_facade/tools.rs` + 至少 1 个测试。
  - **v2 处方断言该方法存在但实际不存在——要么澄清（路径 B 是契约变更，需用户决策），要么降级为 NEEDS CONTEXT**。这是**整个处方最严重的可行性瑕疵**。

---

## Check F4（编年史条空白）

- **Judge findings resolved**: **YES**
- **New over-engineering introduced**: **NO**
- **Feasible**: **NEEDS MINOR CLARIFICATION**
- **Notes**:
  - ✅ **minimax-m3 / ox-alpha 双 FAIL 修正**：v2 完全去除 `@keyframes chronicle-shift`（30s 循环违反 truth 语义、idle 跑动画=错信息）。PASS。
  - ✅ **ox-alpha 真值语义还原**：从"事件驱动颜色沉积"改为"状态驱动 gradient stops"。方向正确。
  - ✅ 不改 `TRUTH_CSS`（`css.rs:190` 现有静态渐变保留边界），不引入 keyframes/rAF 强制循环。
  - ⚠️ **Signal 来源不明**：处方说"mind_base Signal"和"history_len Signal"驱动重算——但当前 `app.rs:98-103` 没有这些 Signal；现有 use_signal 是 `theme_dark`/`head_folded`/`streaming`/`entries`/`active_set`。需要 implementer **新加**这两个 Signal（如 `let mut mind_base = use_signal(|| "#888888".to_string()); let mut history_len = use_signal(|| 0usize);`），处方未说"新建"。
  - ⚠️ **换色事件触发源不明**：处方描述"换色过渡：mind_base Signal 变化时 Dioxus 自动重渲"——但**谁动 mind_base Signal**？truth HTML 是 `cbar.addEventListener('dblclick', ...)`（demo 演示）。production 流程应是 KernelEventDto::Banner / ToolCall 等事件？还是 ui_dioxus 内某 change-mind 按钮？处方把"换色事件触发"当已知事实，实际是规划缺口。
  - ⚠️ **`color-mix()` 浏览器兼容**：WebView2 基于 Chromium 111+（Edge Chromium），`color-mix()` 自 Chromium 111 支持（2023-03）。Windows 11 默认 WebView2 runtime 是 Edge ≥ 110，OK；但**老 Win10 / 自打包 WebView2 < 111 不支持**。v2 处方未注明 floor；要么 inline 计算（用 `mixHex` 类似的 Rust fn），要么声明 WebView2 min version。
  - ⚠️ **truth mixHex 衰退曲线**：truth JS 是 `mixHex(BIRTH, c, 0.18 + 0.82 * (i / (hist.length - 1)))`——这层"历史色按龄褪向底色"语义 v2 简化为"mid_fade = color-mix(birth_gray, mind_base, 40%)"。**v2 实际上没有正确的 fade 函数**（只混了 40%，没有按 history index 计算权重）。这会让历史色看上去不像"沉淀"而像"中点灰"——truth 视觉丢失。
  - 可行但**Signal 来源 + 触发源 + mixHex 算法** 三项需在 brief 里补完。

---

## Check F5（Settings 无持久化）

- **Judge findings resolved**: **PARTIAL**
- **New over-engineering introduced**: **NO**
- **Feasible**: **NEEDS MAJOR CLARIFICATION**
- **Notes**:
  - ✅ **minimax-m3 Q FAIL 修正**：`SettingsState` struct / `settings_store.rs` / `AppEvent::SettingsUpdate` 全部删除——直接调 `load_app_settings` / `persist_app_settings`，不新建平行层。
  - ✅ **ox-alpha 硬约束**：provider key 走 C3 keyring，不落 GlobalConfig 磁盘。
  - ❌ **API 名错位（重大）**：处方反复说"调 `persist_app_settings`"——但**该函数不存在**。实测 `app_state/settings/io.rs`：
    - `load_app_settings() -> Result<AppSettings>`（公开）
    - `update_app_settings<T>(f: impl FnOnce(&mut AppSettings) -> Result<T>) -> Result<T>`（公开，**transactional closure pattern**）
    - 没有 `persist_app_settings` 函数，也没有 `save_*` 顶层包装——save 只走 `update_app_settings` 的 closure 路径。
  - ⚠️ **正确接法**：每个 toggle 后应写 `update_app_settings(|s| { s.provider = X; Ok(()) }).await`。处方应明确"用 `update_app_settings` closure，不要直接覆盖 `AppSettings` 全量"。
  - ⚠️ **`store_api_key` 签名错位**：处方说"走 `keyring.rs::store_api_key`（C3 模式）"——实际签名是 `pub fn store_api_key(keyring: &dyn KeyringBackend, provider_id: &str, plaintext: &str) -> Result<String>`（keyring.rs:220）。需要先构造 `KeyringBackend`（`MockKeyring` 或 OS keyring 适配），处方未说。
  - ⚠️ **500ms debounce 措辞**：当前 `pages_settings.rs:84-91` 有 8 个 `use_signal`，每次 toggle 直接发 persist 是高频 IO；但 v2 没有给出 debounce 简单实现（要不就每 toggle 一次写，反正频率低；要不就 `tokio::time::sleep(500ms)` + cancellation token，但状态机变重）。ponytail 看：settings IO 频率低（用户手动 toggle），**直接每次写最简单，debounce 是 over-engineering**。处方保留 debounce 是可商榷的，但不必删。
  - 必须把 `persist_app_settings` 替换为 `update_app_settings` 实际函数名 + 实际签名——这是**整篇处方最严重的命名错位**。

---

## Check F6（Onboarding 无流程控制）

- **Judge findings resolved**: **PARTIAL**
- **New over-engineering introduced**: **NO**
- **Feasible**: **NEEDS MAJOR CLARIFICATION**
- **Notes**:
  - ✅ **ox-alpha FAIL 修正**：`onboarding_state.rs` 单独文件→页面内 `Step` enum。PASS。
  - ✅ **step-explore NEEDS CTX 裁决**：Dioxus 内完成（`pages_onboarding.rs` 已有完整 Dioxus 组件），不上 HTML 原型。PASS。
  - ✅ **minimax-m3 Q 警告**：provider key 走 keyring。PASS。
  - ✅ 不引入状态机库（`statig`/`rust_fsm`），3 步 enum 是最简。
  - ⚠️ **`test_provider_connection` 不存在**：实际是 `kernel_facade().test_provider(id: &str) -> Result<ProviderTestResultDto, KernelError>` 和 `test_provider_config(form: ProviderFormDto) -> ...`（contracts/kernel-api/src/settings.rs:179, 183）。如果是"用户已选过的 provider"，调 `test_provider(id)`；如果是"用户在 onboarding 实时填的 form"，调 `test_provider_config(form)`。处方需明确**用哪个**——F6 Step II 现在是 form 态，应该用 `test_provider_config`。
  - ⚠️ **`create_session` 签名错位**：处方写 `create_session()`——实际是 `create_session(config: SessionConfigDto) -> Result<SessionId, KernelError>`。需要构造 `SessionConfigDto { workspace_slug, agent_id?, kind: SessionKindDto::Room, ... }`，无 free-form call。
  - ⚠️ **`AppSettings::onboarding_completed` 和 `current_workspace` 字段存在**（app_state/settings/mod.rs:62-68）——✅ 这一步可行。但需要 closure 内通过 `update_app_settings(|s| { s.onboarding_completed = true; s.add_workspace(path); Ok(()) })`，同样命中 F5 的命名错位问题。
  - ⚠️ **`workspace 路径有效` 校验**：处方说"Step::Three → workspace 路径有效"——是 `Path::exists()`？还是 `add_workspace` 内部已经做了？需 implementer 选一个简单路径（`std::path::Path::new(s).exists()`），处方未说。

---

## Overall Assessment

- **All judge findings addressed**: **PARTIAL**
  - B1 / B2 / F1 / F4 **全部 PASS**——方向修正、新抽象已消除。
  - B3 / B4 / F2 / F3 / F5 / F6 **PARTIAL**——方向对，但落到代码上命名/签名错位或方法不存在。
  - 三方 judge 指出的"5 个 FAIL 项 + 2 个 NEEDS CTX 项"中，仅 F4 / B4 真正落实；其余都遗留了"假设方法存在 / 类型对得上"的乐观判断。
- **New issues introduced**: **6 项 factual/API 错位**
  1. **`persist_app_settings` 不存在**（F5 + B2 引用）—— 实际是 `update_app_settings<T>(f: FnOnce(&mut AppSettings) -> Result<T>)`。
  2. **`cancel_turn` → 实际 `stop_turn`** + 参数 `&TurnId` 而非 `&str`（B1 + F2）。
  3. **`submit_turn` 参数类型错**：`&str` → 实际 `TurnInputDto`（B1 + F2）。
  4. **`confirm_tool` 在 `KernelFacade` 上不存在**——这是 frozen contracts 缺口（F3 最严重）；实现必须新增 facade 方法或降级 NEEDS CONTEXT。
  5. **`test_provider_connection` 不存在**——实际 `test_provider(id)` / `test_provider_config(form)`（F6）。
  6. **`subscribe_events` 模型错**：处方写 `impl Stream`，实际是 `callback: Box<dyn Fn>` 模式（B1）。
  - 加上 B3 `cleanup_orphaned_snapshots` 是 method on `FileSnapshotSystem` 非 free function；B4 `let _ = ... map_err(...)` 在返回 `()` 的 trait method 上不成立；F4 Signal 来源 + 触发源 + mixHex 算法未补；F6 `create_session()` 需要 `SessionConfigDto` 参数。
- **Ready for implementation**: **NEEDS MINOR REVISION**（在 API 核对与三处命名错位修正后） / **NEEDS MAJOR REVISION**（若 F3 的 `confirm_tool` 缺失被视为契约变更）。

### Changes needed before implementation can begin

1. **(P0 阻塞 F3)** 明确 `KernelToolsApi` 是否新增 `respond_to_tool_confirmation(tool_id, approve, reason) -> Result<(), KernelError>` 方法，或允许 ui_dioxus 直持 `ConversationCoordinator` (违规)。用户决策：scope 扩张 vs 恢复 NEEDS CTXT。
2. **(P0 全面) 修正 API 命名/类型**
   - B1：`submit_turn`/`stop_turn` 改为真实签名 + TurnInputDto 构造示例；`subscribe_events` 改 callback → mpsc → use_future pattern。
   - B2：保持。
   - B3：补 `cleanup_orphaned_snapshots` 获取方式（`FileSnapshotSystem` 实例来源）+ 测试要求。
   - B4：明确 `AgenticEventPriority == EventPriority (alias)`；理清 `StreamEventSink::enqueue` 返回 `()` 与 inherent `enqueue` 返回 `Result` 不矛盾；.map_err 写法改成 `if let Err(e) = ... error!()`。
   - F5：把 `persist_app_settings` 全部替换为 `update_app_settings` + closure pattern；补 `store_api_key(&dyn KeyringBackend, ...)` 注入。
   - F6：`test_provider_connection` → `test_provider_config(form)`；`create_session()` → `create_session(SessionConfigDto)`。
3. **(P1) F4 Signal/事件来源补完**：注明 `mind_base` / `history_len` 是新建还是映射现有 Signal；注明"换色事件"的实现源（KernelEventDto？ui_dioxus button？）；补全 mixHex 衰退曲线（不能简化为 40% color-mix）；注明 WebView2 ≥ Chromium 111 floor 或提供 Rust mixHex fallback。
4. **(P2) B3 测试**：加 "session 删除 callback + cleanup_all 后 orphan snapshot 被删" 集成测试。
5. **(P3) 一致性收敛**：P0a/B1/F1/F2/F3 之间已无平行抽象重复风险；但 F5/F6 都涉及 settings 写盘，需在 brief 里说清"两个任务都改 pages_settings.rs / pages_onboarding.rs，依赖 P1b 与 F2 同源 input"——避免 implementer 误以为两条平行 IO 路径。
6. **(P3) 优先级微调**：建议把 **F4 提前到 P0c**（F4 涉及 TRUTH_CSS 边界 + WebView2 兼容性问题，越早暴露问题越好；当前 P1a 排在 P0 之后，F1 真实数据流上线后会审视 F4）。

### 已落定的优秀修正（保留）

- 不自建 `AppEvent` / `event_bus.rs`（step-explore 主线）。
- 单文件 `ui_dioxus/api.rs` + 不拆 `session_api`/`dialog_api` 子模块（minimax-m3 主线）。
- F4 删除 `@keyframes chronicle-shift`（minimax-m3 / ox-alpha 双 FAIL 共同根因）。
- B2 两个函数 + 无 trait（minimax-m3 Q 警告）。
- F5 直接调 `update_app_settings`，不新加 `SettingsState`/`settings_store.rs`。
- F6 `Step` enum 在页面内，不单独 `onboarding_state.rs`。

> 此评审基于 freeze 后的 contract + 当前 ui_dioxus 源码逐一回放验证；未做编译/集成测试验证。`KernelToolsApi` 的 `respond_to_tool_confirmation` 是否新增属于产品决策，已逐条列出，由用户在派 implementer 前裁定。
