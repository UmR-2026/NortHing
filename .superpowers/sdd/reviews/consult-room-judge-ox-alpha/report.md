# Judge Report — consult-room 前端线 + 后端桥接 全局问题处方评审

- Judge: ox-alpha（独立评审，未参考先前分析结论；全部证据自行 grep/读源验证）
- Date: 2026-08-25
- Brief: `judge-brief-20260825.md`（副本见本目录 `brief.md`）
- 设计真值: `docs/design/2026-07-22-frontend-redesign/consult-room/`

## Problem B1（Dioxus ↔ 后端桥接缺失）

- SPEC: PASS
- QUALITY: PASS
- Notes: 问题属实——`ui_dioxus` 模块对 `app_state` 的引用仅剩 `log_debug_event`（registry.rs/windows.rs），无任何 session/agent/streaming 接线；`GlobalConfigManager` 与 `DialogScheduler` 确实存在可复用。两点须在任务书里补明：(1) 下行事件泵（backend → Dioxus）在处方里是隐式的，必须显式定义为桥接的一部分并与 F1 的 event_bus 合流，否则出现两条并行通道；(2) 尽量复用 Slint 侧 `app_state` 中与 UI 无关的编排逻辑（turn_runtime / sessions），只把"写 Slint 属性"那层换成 Dioxus signal，不要平行重写一遍会话编排。

## Problem B2（MCP env 明文落盘）

- SPEC: PASS
- QUALITY: PASS
- Notes: 与 ledger P1-8 一致；`env: HashMap<String,String>` 在 `services-integrations/src/mcp/server/mod.rs:107`（serde 直序列化）与 `kernel-api/settings.rs:77` 双处确认明文。C3 keyring + sentinel 模式已存在（`app_state/settings/keyring.rs`，含 MockKeyring 测试先例、fail-closed 原则），照抄即可。边界提醒：project 级 mcp 配置文件（Cursor 格式兼容）里 env 明文是该格式行业惯例，keyring 化只应覆盖 user 级 `app.json` 路径，任务书应明确此范围，避免把共享配置文件也改坏。

## Problem B3（CleanupService 从未调度）

- SPEC: PASS
- QUALITY: PASS
- Notes: grep 全仓确认 `CleanupService` 零实例化，与 ledger P2-4 逐字吻合，处方即 ledger proposed fix 的前两条。两个小缺口记入任务书：ledger 第 (3) 条"orphaned snapshots 纳入 CleanupService"未覆盖；24h 周期意味着启动后首次清理要等一天，建议启动时先跑一次再进周期。若用 cancellation token 实现循环，按家规 #4 必须带自动化测试。

## Problem B4（Event queue 满时静默丢事件）

- SPEC: PASS
- QUALITY: FAIL
- Notes: 问题属实（queue.rs:85-88 drop 后 `return Ok`，且 drop 路径连 broadcast 都不发）。"返回 Err 让调用方决定"方向正确且必要——但现状约 10 处调用点全是 `let _ = self.event_queue.enqueue(...)`，StreamEventSink impl 也是 `let _ =`（queue.rs:227）；不改这些调用点，Err 什么都没改变，处方对此只字未提。Critical "block + 扩容" 是过度设计且有死锁风险：阻塞式 enqueue 在生产者与消费者同上下文时互锁，扩容则使 max_queue_size 对 Critical 形同虚设。最简正确解：Critical 直接跳过容量上限检查入队（一行），其余优先级返回 Err 并在 StreamEventSink / 各 `let _` 调用点补 error 级日志。

## Problem F1（零真实数据流）

- SPEC: PASS
- QUALITY: PASS
- Notes: `session_mock.rs` 五类硬编码记录属实（seed_session 逐条对照过真值 HTML）；渐进四步（AppEvent enum → event_bus → use_future 消费 → B1 接入）依赖顺序合理。一个结构建议：AppEvent 把命令（SendMessage/SettingsUpdate）和数据（SessionList/SessionTurns）混在一个 enum 里，命令走 mpsc、数据回推建议拆成两条通道（上行 command channel / 下行 event broadcast），否则背压语义含混。不阻塞判定，任务书写清即可。

## Problem F2（消息发送无 handler）

- SPEC: PASS
- QUALITY: PASS
- Notes: 属实——app.rs:373 `streaming.set(!streaming())`、pages_space.rs:460 `is_streaming.toggle()`，send 按钮确实只是 streaming 开关。链路 on_send → AppEvent::SendMessage → bus → B1 → `DialogScheduler::submit` → StreamingUpdate 逐 token 渲染是真值页面的直接对应物，复用现有调度器入口，无新抽象。

## Problem F3（Approval 卡无 handler）

- SPEC: PASS
- QUALITY: PASS
- Notes: 属实——app.rs:577-582 approve/reject 按钮无 onclick。走既有 tool_confirmation 确认门是正确的复用（该门在 P1-6 修复后已是所有危险操作的强制通道）。reject 附可选文本框超出真值所示（真值只有 resolved 态文案），保留为可选项即可，别让它膨胀成必做项。

## Problem F4（编年史条空白）

- SPEC: FAIL
- QUALITY: FAIL
- Notes: 处方与设计意图相悖。真值文件（consult-room-main.html:548-584）定义的编年史条是**事件驱动的颜色沉积**：出生灰恒定最左、历史 mind 色随龄褪向底色、右端≡当前 --mind-base 全饱和、换色时新色自 100% 进入旧 stop 左移沉降——语义是"状态的历史"，不是装饰动画。处方给的 `@keyframes chronicle-shift` 30s 循环渐移位是一个与任何事件无关的永动 loop，语义错误。另注：migration 注释明确"真值 JS/rAF 一律不移植"，纯 CSS 也无法插值 gradient stops。正确最小实现：从状态 signal 渲染多 stop 渐变（静态即可成立），换色事件时更新 stops（WebView2 下用 @property 注册自定义属性或分层透明度过渡实现平滑）。问题定性"空白"基本成立：现 css.rs:190 只是一条 bg3→accent-solid 的假静态渐变，不携带任何编年史数据。需重写处方后再执行。

## Problem F5（Settings 无持久化）

- SPEC: PASS
- QUALITY: PASS
- Notes: 属实——全 ui_dioxus 目录 grep 无 persist/save/AppSettings/GlobalConfig 引用，引擎/provider/MCP 全是裸 use_signal。四步方案成立。两条硬约束进任务书：(1) provider key 持久化必须走 C3 keyring 路径，core 不落 api_key（Scheme C 骨干不变量），不能因"写入后端"顺手把 key 写进 GlobalConfig 磁盘；(2) 优先复用 Slint 侧 `app_state::settings` 的 io/keyring/sync 中 UI 无关部分，别新建平行 settings 存储层。

## Problem F6（Onboarding 无流程控制）

- SPEC: PASS
- QUALITY: PASS
- Notes: 问题表述部分过时——pages_onboarding.rs 已有 `ritual_completed` 信号和色板前置校验（:536-541）；真正缺的是真值清单的逐步判定（step II 要求连接测试通过"未测试→已通畅"、step III 锚定边界）以及完成后的真实副作用（当前完成按钮只翻 flag，什么都不发生）。处方的状态机 + OnboardingComplete → AppSettings + 启动 session 与真值三关卡语义吻合，判 PASS；但 `onboarding_state.rs` 单独成档偏重，三个 bool 或一个 enum 放在页面模块内即可，除非后续有跨窗口消费需求。

## Summary

- Spec compliant: **9/10**（唯一 FAIL：F4，处方模型与真值的"颜色沉积"语义不符，须重写）
- Quality concerns:
  1. **B4**：Critical "block + 扩容" 过度设计且有死锁风险 → 改为 Critical 跳过容量上限 + 其余返回 Err + 补齐 ~10 处 `let _` 调用点与 StreamEventSink 的错误日志，否则 Err 无效果。
  2. **F4**：30s keyframes 循环是发明的需求；重写为状态驱动渐变（signal 渲染 stops + 换色事件过渡）。
  3. **B1/F1/F5**：三张处方各自引入通道抽象（api 子模块 / event_bus / settings_store mpsc），落地时必须收敛为"上行 command 一条、下行 event 一条"，并最大化复用 Slint 侧 app_state 的 UI 无关逻辑，防止同一套编排长出第二份。
  4. **F5/F6 安全红线**：provider key 一律走 C3 keyring + 内存 facade，禁止借持久化任务把 api_key 写回磁盘。
  5. 小项：B3 补 orphaned snapshots 与启动即跑一次；B2 明确只覆盖 user 级配置；F6 状态机不必单独建文件。
- Recommended priority order:
  1. **B1**（一切前端真实化的地基）
  2. **F1**（bus 定义，B1 的下行出口依赖它）
  3. **F2**（发送 + streaming，P0 核心体验）
  4. **F3**（approval 闭环，P0 收尾）
  5. **B4**（真实流量接入前修掉静默丢失，改动小收益大；按上文修订版执行）
  6. **F5**（settings 持久化，依赖 F1 bus）
  7. **F6**（onboarding 完成写盘 + 启动 session，依赖 F5/B1）
  8. **B2**（安全债，机械复用 C3，随时可插队但非阻塞）
  9. **B3**（卫生接线， trivial）
  10. **F4**（先重写处方再动工，纯视觉不阻塞任何项）

调整理由：原表 B2(P1) 排在全部 P2 之前，但它是孤立的安全加固，而 B4 在真实会话流量上线后会直接造成用户可见的事件丢失，性价比排序应让 B4 提前；F4 因 SPEC FAIL 移至末位待重开。
