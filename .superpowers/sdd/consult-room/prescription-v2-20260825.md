# consult-room 前端线 + 后端桥接 — 修正版处方方案

> 综合三方 judge（minimax-m3 / step-explore / ox-alpha）独立判决修订。
> 
> 原处方审查：`judge-brief-20260825.md` → `.superpowers/sdd/reviews/consult-room-judge-{minimax-m3,step-explore,ox-alpha}/report.md`

---

## 核心原则（三方共识）

1. **不复刻已有层**：Dioxus UI 侧走 `kernel_facade()` + `CoreAgentAdapter` 等价路径，不新建 parallel bridge
2. **不复用已有事件**：下行用 `KernelEventDto::TextChunk`（frozen），上行直接用 facade API 调用，不建 `event_bus.rs` / `AppEvent` enum
3. **ponytail**：单文件、薄封装、不移交抽象
4. **不碰 TRUTH_CSS**：视觉真值 CSS 逐字节锁死，动态行为在 Rust/Dioxus 层实现

---

## 后端（4 项）

### B1 — Dioxus ↔ 后端桥接缺失

**问题**：`ui_dioxus` 对 `app_state/` 的引用仅剩 `log_debug_event`，无任何 session/agent/streaming 接线

**处方**：
```
文件：src/apps/desktop/src/ui_dioxus/api.rs（单文件，~150 行）

暴露三个薄封装函数：
  - submit_turn(text: &str) -> Result<String>        // → kernel_facade().submit_turn(...)
  - cancel_turn(turn_id: &str) -> Result<()>         // → kernel_facade().cancel_turn(...)  
  - confirm_tool(call_id: &str, approve: bool) -> Result<()>  // → confirm_tool / reject_tool

下行事件泵（backend → Dioxus）：
  - subscribe_events() -> impl Stream<Item = KernelEventDto>
  - 内部用 EventQueue::subscribe() broadcast（已有，零新建）
  - Dioxus 侧 use_future + changed().await 转 Signal（已演示模式：app.rs:106-117）

不引入：session_api/dialog_api 子模块 / AppEvent enum / mpsc channel / DialogScheduler 直接引用
```

**Judge 反馈**：
- minimax-m3: SPEC FAIL（处方引用 tauri/DialogScheduler 措辞错位）→ 修正后 PASS
- step-explore: SPEC PASS / Q FAIL（应复用 CoreAgentAdapter pattern）
- ox-alpha: SPEC ✅ / Q ✅（处方方向对，两点须补明：下行事件泵显式定义、复用 Slint 侧编排逻辑）

**执行要点**：
- 调用 `kernel_facade()`（Slint 端 `event_bridge.rs:33` 同源）
- 下行用 `KernelEventDto::TextChunk`（已 frozen，零新建类型）
- 不引入新抽象层

---

### B2 — MCP env 变量明文存 app.json

**问题**：`MCPServerConfig.env: HashMap<String, String>` 经 serde 直序列化到 app.json

**处方**：
```
文件：src/crates/services/services-integrations/src/mcp/env_secret.rs（新）

pub fn store_env(server_id: &str, env: &HashMap<String, String>) -> Result<String>
  - JSON 序列化整块 → 存入 OS keyring（keyring entry: "mcp-env:{server_id}"）
  - 返回 sentinel "__kr_env__"

pub fn load_env(server_id: &str) -> Result<HashMap<String, String>>
  - 从 keyring 读 JSON → 反序列化
  - sentinel 不在 → 返回 Ok(HashMap::new()) 兼容旧数据

文件：src/apps/desktop/src/app_state/settings/io.rs
  - persist_app_settings 前：env 块 → store_env → 磁盘存 sentinel
  - load_app_settings 后：遇 sentinel → load_env 还原

不引入：McpEnvStore trait（等有第二消费者再加）/ per-variable sentinel
```

**Judge 反馈**：
- ox-alpha: SPEC ✅ / Q ✅（方向完全对齐 C3 模式，边界提醒：只覆盖 user 级 app.json，不碰 project 级 mcp 配置文件）
- minimax-m3: SPEC ✅ / Q ⚠️（trait get/set/remove 过宽，改为两个函数即可）

---

### B3 — CleanupService 从未调度

**问题**：`CleanupService::cleanup_all()` 全量实现但零实例化

**处方**：
```
文件：src/apps/desktop/src/lib.rs（app bootstrap 段）

// 在 kernel_facade() 初始化之后、Slint/Dioxus 两路启动之前：
tokio::spawn(async move {
    let svc = CleanupService::new(path_manager, CleanupPolicy::default());
    // 启动即跑一次（不要等 24h 第一轮）
    let _ = svc.cleanup_all().await;
    let mut tick = tokio::time::interval(Duration::from_secs(86400));
    loop {
        tick.tick().await;
        let _ = svc.cleanup_all().await;
    }
});

// session 删除回调加一行：
snapshot_system::cleanup_orphaned_snapshots().await;

不引入：新 crate / 新调度库 / event_bridge 耦合
```

**Judge 反馈**：
- minimax-m3: SPEC ✅ / Q ⚠️（位置应在 lib.rs bootstrap 而非 event_bridge，24h 首次应启动即跑）
- ox-alpha: SPEC ✅ / Q ✅（补 orphaned snapshots + 启动即跑，按家规 #4 带测试）

---

### B4 — Event queue 满时静默丢事件

**问题**：`EventQueue::enqueue` 满时 drop + `return Ok`，~10 处调用点全是 `let _ =`

**处方**：
```
文件：src/crates/assembly/core/src/agentic/events/queue.rs

改 enqueue 签名：Result<(), EventQueueFull>
  - Critical 优先级：跳过容量上限检查（一行），直接入队
  - 非 Critical 满时：返回 Err(EventQueueFull)
  - 不加"扩容"（会使 capacity 形同虚设）
  - 不加 blocking wait（有死锁风险）

文件：各调用点（~10 处 StreamEventSink::enqueue）
  - 不改 StreamEventSink trait 签名（影响面太大）
  - let _ = 改为 .map_err(|e| tracing::error!(...))
  - StreamEventSink::enqueue 内同样补 error 日志

不引入：新类型 / 新 trait / blocking channel
```

**Judge 反馈**：
- ox-alpha: SPEC ✅ / Q ⚠️（Critical block + 扩容过度设计 → 改为 Critical 跳 cap + 其余 Err + 补 call site 日志）
- minimax-m3: SPEC ✅ / Q ✅（caveat: AgenticEventPriority vs EventPriority 两类型需同步检查）

---

## 前端（6 项）

### F1 — 零真实数据流

**问题**：session_mock + STRATA + DOORS 全硬编码，page components 无动态数据

**处方**：
```
文件：src/apps/desktop/src/ui_dioxus/api.rs（与 B1 同文件）

结构：
  pub async fn list_sessions() -> Result<Vec<SessionSummary>>    // kernel_facade
  pub async fn load_session(session_id: &str) -> Result<SessionDetail>
  pub async fn submit_turn(text: &str) -> Result<String>

Dioxus 侧消费：
  use_future + subscribe_events() 收 KernelEventDto → 转 Signal
  - TextChunk → 增量追加到 entries Signal
  - TurnState → 更新 streaming/entries 状态

页面接线：
  app.rs: entries = use_signal(|| seed_session()) 
    → 启动时调 load_session 覆盖（seed_session 保留为 fallback）
  pages_archive.rs: STRATA 静态表 → 无一次性全改，逐步替换

不引入：AppEvent enum / event_bus.rs / mpsc channel
```

**Judge 反馈**：
- ox-alpha: SPEC ✅ / Q ✅（方向对，建议 AppEvent 拆成 command/data 两条通道——记入执行备注）
- step-explore: SPEC ✅ / Q ❌（应复用已有 AgenticEvent + EventQueue::subscribe，不自建 AppEvent）
- minimax-m3: NEEDS CTX / Q ❌（B1 悬空则 F1 全卡）

---

### F2 — 消息发送无 handler

**问题**：input box 是 div 占位符，send 按钮只 toggle streaming 信号

**处方**：
```
文件：src/apps/desktop/src/ui_dioxus/app.rs

1. input-box div → dioxus <input> 组件：
   input {
       class: "input-box",
       value: "{user_input}",
       oninput: move |e| user_input.set(e.value()),
       onkeydown: move |e| { if e.key() == "Enter" { send(); } }
   }

2. send 按钮 onclick：
   - streaming 时 → api.cancel_turn(active_turn_id) + streaming.set(false)
   - 非 streaming → api.submit_turn(&user_input) + user_input.set("") + streaming.set(true)

3. streaming 渲染：
   use_future 消费 subscribe_events() → TextChunk → 追加到 entries Signal
   不再 toggle streaming（由 TextChunk 结束事件判定）

不引入：StreamingUpdate event / 新事件类型
```

**Judge 反馈**：
- minimax-m3: NEEDS CTX / Q ⚠️（应调 kernel_facade.submit_turn 而非 DialogScheduler::submit；stop 走 cancel_turn）
- ox-alpha: SPEC ✅ / Q ✅（链路正确，复用既有调度器入口）
- step-explore: SPEC ✅ / Q ⚠️（应直接调 CoreAgentAdapter，不走 event_bus）

---

### F3 — Approval 卡 approve/reject 无 handler

**问题**：approve/reject 按钮无 onclick 绑定（app.rs:577-582）

**处方**：
```
文件：src/apps/desktop/src/ui_dioxus/app.rs（render_entry 内）

resolved == true：不绑事件（已有分支）
resolved == false：
  - approve 按钮 onclick → api.confirm_tool(call_id, true)
  - reject 按钮 onclick → api.confirm_tool(call_id, false)
  - 不加可选文本输入框（超出真值范围，DELIVERY-NOTES 红线"诗意<功能"）

路由：走 kernel_facade 的确认门（P1-6 修复后已是所有危险操作强制通道）
不引入：tool_confirmation 直接依赖 / AppEvent::Approve / 输入框
```

**Judge 反馈**：
- ox-alpha: SPEC ✅ / Q ✅（走既有确认门正确，reject 可选文本框保留可选项）
- minimax-m3: NEEDS CTX / Q ⚠️（需确认 facade 是否暴露 `respond_to_tool_confirmation` 或等价 DTO）

---

### F4 — 编年史条空白

**问题**：`#chronicle-bar` div 空壳（app.rs:317-322），truth HTML 定义的是事件驱动颜色沉积

**处方（修正版，基于三方 judge 反馈）**：
```
文件：src/apps/desktop/src/ui_dioxus/app.rs（chronicle-bar div）

原理：
  - 状态驱动：mind_base Signal 变化 → 重算 gradient stops
  - 静态渐变即可成立（生日灰【恒定、最左、100%】 → 历史色【衰退、中段、透明递增】 → 当前色【全饱和、右端】）
  - 换色过渡：mind_base Signal 变化时 Dioxus 自动重渲，WebView2 支持 CSS transition on background-image

实现：
  div {
      class: "chronicle-bar",
      style: format!(
          "background: linear-gradient(90deg, #B8B8B8 0%, {} 30%, {} 60%, {} 100%)",
          birth_gray,    // 恒定不褪
          mid_fade,      // 历史色褪向 birth_gray
          current_color  // 当前 --mind-base 全饱和
      )
  }

  - birth_gray = 固定出生灰（如 #888888）
  - mid_fade = color-mix(birth_gray, mind_base, 40%) // 半褪
  - current_color = mind_base（全饱和）
  - 左端宽度和退色曲线由 history_len Signal 驱动

不引入：@keyframes / CSS animation / JS rAF / TRUTH_CSS 改动
```

**Judge 反馈**：
- minimax-m3: SPEC ❌ / Q ❌（处方 30s keyframes 循环违反 truth 语义，idle 跑动画 = 错信息）
- ox-alpha: SPEC ❌ / Q ❌（处方是发明的需求；真相是事件驱动颜色沉积，必须从 state signal 渲染；换色事件时更新 stops）
- step-explore: SPEC ✅ / Q ✅（仅 CSS 方案——但未核查 truth HTML 具体语义）

**三方共识**：原处方 SPEC FAIL，须从"CSS 动画"改为"状态驱动渐变 + 换色事件更新"

---

### F5 — Settings 无持久化

**问题**：引擎/provider/MCP/display 设置全是 `use_signal`，关闭即丢

**处方**：
```
文件：src/apps/desktop/src/ui_dioxus/pages_settings.rs（就地修改，不新建文件）

Step 1：页面加载时调一次：
  let mut state = use_signal(|| {
      // 同步调已有 load_app_settings，提取 settings 相关字段
      // 首次运行用默认值
  });

Step 2：每次 toggle 后防抖调 persist：
  toggle 时直接调 persist_app_settings（settings IO 频率低，500ms debounce 即可）
  api_key 变更走 keyring.rs::store_api_key（C3 模式，不落 GlobalConfig 磁盘）

Step 3：复用已有层：
  - 用 app_state/settings/io.rs 的 load_app_settings / persist_app_settings
  - 用 app_state/settings/keyring.rs 的 KeyringBackend
  - 不要在 ui_dioxus 新建 settings_store.rs / AppEvent::SettingsUpdate

不引入：SettingsState struct / settings_store.rs / event_bus
```

**Judge 反馈**：
- minimax-m3: SPEC ✅ / Q ⚠️（SettingsState 无意义抽象，settings_store.rs 重复造轮——直接用已有 persist_app_settings）
- ox-alpha: SPEC ✅ / Q ✅（方向对，硬约束：provider key 走 C3 keyring，不落磁盘）

---

### F6 — Onboarding 无流程控制

**问题**：3 步仪式无 step 间约束、无 completion 判定、完成按钮只翻 flag

**处方**：
```
文件：src/apps/desktop/src/ui_dioxus/pages_onboarding.rs（就地修改，不新建文件）

Step 1：加 enum：
  enum Step { One, Two, Three }
  let mut current_step = use_signal(|| Step::One);

Step 2：每步完成条件（can_proceed）：
  Step::One → palette 已选 + agent_name 非空
  Step::Two → connection test 通过（调已有 test_provider_connection）
  Step::Three → workspace 路径有效

Step 3：完成时副作用：
  - api_key 走 keyring store（C3 模式）
  - AppSettings 写入 onboarding_completed + workspace
  - create_session() 启动第一个 session

不引入：onboarding_state.rs 单独文件 / AppEvent::OnboardingComplete / 状态机库
```

**Judge 反馈**：
- ox-alpha: SPEC ✅ / Q ✅（方向对，`onboarding_state.rs` 单独成档偏重，放页面内即可）
- minimax-m3: SPEC ✅ / Q ⚠️（provider key 必须走 C3 keyring）
- step-explore: NEEDS CTX / Q ❌（需先澄清 onboarding 宿主：HTML vs Dioxus）

**裁决**：onboarding 在 Dioxus 内完成（`pages_onboarding.rs` 已有完整实现），不上 HTML 原型。

---

## 执行优先级（修正版）

| 优先级 | 任务 | 内容 | 文件 | 依赖 |
|---|---|---|---|---|
| **P0a** | B1 桥接层 | `ui_dioxus/api.rs` 单文件，kernel_facade + subscribe_events | 1 新文件 ~150L | 无 |
| **P0b** | F2 消息发送 | input box → `<input>` + send/stop handler → api.rs | app.rs 修改 | P0a |
| **P0c** | F3 Approval 卡 | unresolved 卡按钮 → api.confirm_tool | app.rs 修改 | P0a |
| **P1a** | F4 编年史条 | 状态驱动 gradient stops，换色事件更新 | app.rs 修改 | 无 |
| **P1b** | F5 Settings 持久化 | toggle → persist_app_settings（复用已有层） | pages_settings.rs | 无 |
| **P1c** | B2 MCP env keyring | services-integrations/env_secret.rs | 1 新文件 ~60L | 无 |
| **P2a** | F1 全页数据流 | seed_session → load_session + subscribe_events | 各 page 文件 | P0a |
| **P2b** | B4 Event queue | Result 返回 + Critical 跳 cap + call site 日志 | queue.rs + ~10 处 | 无 |
| **P3a** | F6 Onboarding 流程 | step enum + completion 副作用 | pages_onboarding.rs | P0a, P1b |
| **P3b** | B3 Cleanup 调度 | lib.rs bootstrap spawn 24h loop | lib.rs 修改 | 无 |

---

## Judge 覆盖

| Judge | 模型 | 状态 | Spec/Quality |
|---|---|---|---|
| minimax-m3 | `minimax-m3` | ✅ 203 行报告 | 5 PASS / 3 FAIL / 2 NEEDS CTX / 6 Q FAIL |
| step-explore | `reviewer/step-explore_reviewer` | ✅ 107 行报告 | 7 PASS / 1 NEEDS CTX / 6 Q FAIL |
| ox-alpha | `judge-ox-alpha` (openrouter) | ✅ 89 行报告 | 9 PASS / 1 FAIL / 7 Q 相关 |

> ox-alpha 是首次通过 OpenRouter stealth 端点成功运行的 judge，三方独立判决覆盖完毕。
