# Prescription v2 Review — consult-room
**Reviewer**: step-explore (independent)
**Date**: 2026-08-25
**Scope**: prescription-v2-20260825.md vs. judge findings (minimax-m3 / step-explore / ox-alpha)

---

## Check B1
- Judge findings resolved: YES
- New over-engineering introduced: NO
- Feasible: YES
- Notes:
  - minimax-m3: original FAIL was tauri::AppHandle-less措辞错位 + DialogScheduler直连违规. v2 用 `kernel_facade()` + 单文件 `api.rs` 替代，措辞正确，PASS.
  - step-explore: original Q FAIL was 应复用 CoreAgentAdapter pattern. v2 principle 2 says "kernel_facade + CoreAgentAdapter 等价路径"，实际处方调用 `kernel_facade().submit_turn()` 等，走 Slint 端 event_bridge.rs:33 同源. PASS.
  - ox-alpha: 两点补明（下行事件泵显式定义、复用 Slint 侧编排逻辑）均在处方中出现. PASS.
  - 无新抽象层（无 AppEvent / mpsc / DialogScheduler 引用）. 单文件 ~150L，ponytail 合规.
  - 残余风险：处方未显式验证 kernel_facade 暴露的方法名（submit_turn / cancel_turn / confirm_tool）与实际签名一致——minimax-m3 原 NEEDS CTX 涉及此问题. 实现前需在 event_bridge.rs 核对一次方法名.

## Check B2
- Judge findings resolved: YES
- New over-engineering introduced: NO
- Feasible: YES
- Notes:
  - minimax-m3 Q 警告：trait get/set/remove 过宽 → 改为两个函数. v2 用 `store_env` / `load_env` 两个独立函数，无 McpEnvStore trait. PASS.
  - ox-alpha: 边界提醒（只覆盖 user 级 app.json，不碰 project 级 mcp 配置文件）在处方隐含 upheld（只修改 settings/io.rs 的 persist/load 路径）. PASS.
  - Sentinel `"__kr_env__"` 简单清晰. Keyring 存储 JSON 块，与 C3 模式一致.
  - ~60L 单文件，ponytail 合规.

## Check B3
- Judge findings resolved: YES
- New over-engineering introduced: NO
- Feasible: YES
- Notes:
  - minimax-m3: 位置（lib.rs bootstrap）正确. 24h 首次改为启动即跑，PASS.
  - ox-alpha: orphaned snapshots + 启动即跑均在处方中. PASS.
  - 用 `tokio::spawn` + `tokio::time::interval`（std lib），无新 crate/调度库. 简洁.
  - `let _ = svc.cleanup_all().await` 用 `let _` swallow error——与家规一致（cleanup 失败不阻断启动），但 ox-alpha 原判要求带测试. 处方未提及测试，需在实现时补充.

## Check B4
- Judge findings resolved: PARTIAL
- New over-engineering introduced: NO
- Feasible: NEEDS CLARIFICATION
- Notes:
  - ox-alpha: Critical 跳 cap + 其余 Err + 补 call site 日志，均在处方中. PASS.
  - minimax-m3 caveat（AgenticEventPriority vs EventPriority 两类型）处方未回应——需确认 Critical 判断用哪个类型.
  - **矛盾点**：处方说"改 enqueue 签名：Result<(), EventQueueFull>"，又说"不改 StreamEventSink trait 签名（影响面太大）". 若 `StreamEventSink::enqueue` 是 trait 方法，改签名必然改变 trait. 若 enqueue 是 inherent method 则无矛盾. **实现前需确认 enqueue 是 trait method 还是 inherent method**——这是阻塞性歧义.
  - "~10 处" 调用点未枚举，实现者需自行 grep. 处方应补一句 "grep -r 'StreamEventSink::enqueue'" 或等效指引.

## Check F1
- Judge findings resolved: YES
- New over-engineering introduced: NO
- Feasible: YES (depends on B1)
- Notes:
  - step-explore Q FAIL 核心是不自建 AppEvent / event_bus. v2 明确写"不引入：AppEvent enum / event_bus.rs / mpsc channel". PASS.
  - ox-alpha 建议 AppEvent 拆 command/data 两条通道——处方标记为"执行备注"而非结构变更，ponytail 合规（不到第二消费者不加抽象）. PASS.
  - F1 处方与 B1 共享 `api.rs`（"与 B1 同文件"），无重复抽象. PASS.
  - 新增 `list_sessions` / `load_session` 在 api.rs 扩展，薄封装.
  - 优先级 P2a 依赖 P0a，时序正确.
  - "seed_session 保留为 fallback"——处方未定义 seed_session 行为，但这是已有代码，非新抽象.

## Check F2
- Judge findings resolved: YES
- New over-engineering introduced: NO
- Feasible: YES (depends on B1)
- Notes:
  - minimax-m3: 用 kernel_facade.submit_turn / cancel_turn. v2 走 `api.submit_turn` / `api.cancel_turn`（B1 同路径）. PASS.
  - step-explore: 不走 event_bus. v2 直接调 api 函数. PASS.
  - ox-alpha: 链路正确. PASS.
  - 不引入 StreamingUpdate event / 新事件类型. 简洁.

## Check F3
- Judge findings resolved: YES
- New over-engineering introduced: NO
- Feasible: YES (depends on P0a)
- Notes:
  - minimax-m3 NEEDS CTX 关于 facade 是否暴露 confirm_tool——处方直接用 `api.confirm_tool(call_id, true/false)`，假设 B1 的 api.rs 提供此包装. 与 B1 一致，PASS.
  - ox-alpha: reject 可选文本框保留为可选项——处方不引入输入框，符合最小解. PASS.
  - 走既有确认门（P1-6），不加新依赖. PASS.

## Check F4
- Judge findings resolved: YES
- New over-engineering introduced: NO
- Feasible: NEEDS CLARIFICATION
- Notes:
  - minimax-m3 原 FAIL（30s @keyframes 循环 = 错信息）→ v2 完全去掉 animation. PASS.
  - ox-alpha 原 FAIL（发明需求）→ v2 改为状态驱动. PASS.
  - step-explore 仅 CSS 方案——v2 是 CSS + Dioxus Signal 驱动，比纯 CSS 更正确.
  - **差距1**：处方描述 `mind_base Signal` 和 `history_len Signal` 驱动重算，但未说明这些 Signal 是否已存在或需新建. 实现者需在 app.rs 中定位现有 Signal 或确认新建.
  - **差距2**："换色事件"触发 gradient stops 更新的机制未说明——是 KernelEventDto 某类型？还是 Signal 直接由其他 Dioxus state 变更驱动？处方应补一句数据流.
  - **差距3**：`color-mix()` CSS 函数在 WebView2 的版本兼容性未注明（Edge WebView2 基于 Chromium，color-mix 在 Chromium 111+ 支持——需确认部署基线）.
  - 方向正确，但实现细节不够完整.

## Check F5
- Judge findings resolved: YES
- New over-engineering introduced: NO
- Feasible: YES
- Notes:
  - minimax-m3: SettingsState / settings_store.rs 警告已消除——处方直接用 `load_app_settings` / `persist_app_settings`. PASS.
  - ox-alpha: provider key 走 C3 keyring，不落磁盘. PASS.
  - 500ms debounce 对 settings IO 合理（频率低），非 magic number.
  - 就地修改 pages_settings.rs，不新建文件. 简洁.

## Check F6
- Judge findings resolved: YES
- New over-engineering introduced: NO
- Feasible: YES (depends on P0a, P1b)
- Notes:
  - ox-alpha: `onboarding_state.rs` 单独文件 → 改为页面内 Step enum（PASS）.
  - step-explore NEEDS CTX（HTML vs Dioxus 宿主）→ 裁决：Dioxus 内完成. PASS.
  - minimax-m3: provider key 走 C3 keyring. PASS.
  - 不引入状态机库. Step enum 3 个变体是最小状态机.
  - 就地修改 pages_onboarding.rs. 无新文件.

---

## Overall Assessment
- All judge findings addressed: PARTIAL
- New issues introduced: 2
  1. **B4 内部矛盾**："改 enqueue 签名" 与 "不改 StreamEventSink trait 签名" 可能矛盾——需确认 enqueue 是 trait method 还是 inherent method.
  2. **F4 数据流不完整**：未说明 mind_base/history_len Signal 是否已存在，以及"换色事件"的触发机制.
  3. **B4 未回应 minimax-m3 caveat**：AgenticEventPriority vs EventPriority 两类型需同步检查，处方未提及.
- Ready for implementation: NEEDS MINOR REVISION
- Changes needed before implementation:
  1. B4: 确认 `StreamEventSink::enqueue` 是 trait method 还是 inherent method；若为 trait method，需改为在方法内部返回 `Err` 而不改签名（如 `fn enqueue(...)` 保持原返回类型，内部记录错误）或在 trait 层同步修改所有实现.
  2. B4: 回应 minimax-m3 caveat——明确 Critical 跳 cap 用哪个优先级类型（AgenticEventPriority 还是 EventPriority），或在处方中注明"与 [文件名] 同步".
  3. F4: 补充数据流说明——mind_base/history_len Signal 的来源（已有还是新建），以及 gradient stops 更新的触发源（KernelEventDto 事件类型？Signal 变更监听？）.
  4. F4: 注明 `color-mix()` 依赖的 WebView2/Chromium 基线版本，或提供 fallback.
  5. B3: 补一条测试要求（ox-alpha 原判要求带测试，处方未提及）.
  6. 建议 B4: 在处方中补一句 grep 指引（如 `grep -r "StreamEventSink::enqueue" src/`）帮助实现者定位 ~10 处调用点.
