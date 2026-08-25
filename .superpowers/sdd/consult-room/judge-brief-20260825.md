# Judge Brief — consult-room 前端线 + 后端桥接 全局问题处方评审

## 任务

你是独立 judge。评审以下问题清单与处方方案，逐条给出判决：
- SPEC PASS / SPEC FAIL / NEEDS CONTEXT
- QUALITY PASS / QUALITY FAIL
- 每条约 1-2 句理由

**独立视角，不要被之前的分析带偏。自行验证证据。**

---

## 问题清单与处方

### 后端（4 项）

| # | 问题 | 处方 | 优先级 |
|---|---|---|---|
| B1 | **Dioxus ↔ 后端桥接缺失**：`ui_dioxus` 壳完全独立运行，无 session 数据读写 / agent 调用 / streaming output | 在 `ui_dioxus` crate 加 `api` 子模块（`session_api` + `dialog_api`），通过已有的 `GlobalConfigManager` + `DialogScheduler` 通路接线；桌面端用 `tauri::AppHandle`-less 的异步 tokio task 桥接 UI signal → core API | P0 |
| B2 | **MCP env 变量明文存 app.json**（tech-debt P1-8）：`MCPServerConfig.env` `HashMap<String, String>` 序列化到磁盘 | 复用 P1-C3 keyring 模式：env block 整块存入 OS keyring，磁盘仅存 sentinel key；实现 `McpEnvStore::get/set/remove` trait，`persist_app_settings` 前调 `McpEnvStore::flush` | P1 |
| B3 | **CleanupService 从未调度**（tech-debt P2-4）：`cleanup.rs` 全量实现但无调用方 | `app_state/event_bridge.rs` startup 路径 spawn `tokio::spawn(cleanup_loop)`；每 24h 跑一次 `cleanup_all`，session 删除时触发即时清理 | P2 |
| B4 | **Event queue 满时静默丢事件**（tech-debt P2-6）：`queue.rs:85` drop + `return Ok` | `enqueue` 返回 `Result<(), EventQueueFull>`；Critical 优先级事件永不丢（block + 扩容）；非 Critical 满时返回 Err 让调用方决定 | P2 |

### 前端（6 项）

| # | 问题 | 处方 | 优先级 |
|---|---|---|---|
| F1 | **零真实数据流**（session_mock + STRATA + DOORS 全硬编码） | 渐进式：Step 1 — 定义 `AppEvent` enum（SessionList / SessionTurns / SendMessage / SettingsUpdate）；Step 2 — 加 `event_bus.rs`（tokio mpsc channel）；Step 3 — page components 用 `use_future` 消费；Step 4 — 后端 B1 桥接接上 | P0 |
| F2 | **消息发送无 handler**（input box 占位，send button 只 toggle streaming） | 实现 `on_send_message` → emit `AppEvent::SendMessage(text)` → event_bus → 后端 B1 桥接 → `DialogScheduler::submit`；streaming 用 `use_future` 消费 `StreamingUpdate` event 逐 token 渲染 | P0 |
| F3 | **Approval 卡 approve/reject 无 handler** | approve → emit `AppEvent::Approve(call_id)` → 走 tool_confirmation 确认门 → 返回结果渲染；reject → emit `AppEvent::Reject(call_id)` + 可选文本输入框 | P0 |
| F4 | **编年史条空白**（`#chronicle-bar` div 空壳） | 加 CSS animation：`@keyframes chronicle-shift` 用 `background-size` + `background-position` 做 30s 循环渐移位；`chronicle-bar` 设 `background: linear-gradient(...)` 含 mind 色 token | P1 |
| F5 | **Settings 无持久化**（引擎/provider/MCP 全是 use_signal） | Step 1 — 加 `SettingsState { engine, providers, mcps, display }` struct；Step 2 — 加 `settings_store.rs`（mpsc → 后端）；Step 3 — toggle 时 emit `AppEvent::SettingsUpdate`；Step 4 — 启动时从后端 load | P1 |
| F6 | **Onboarding 无流程控制**（3 步仪式无 completion 判定） | 加 `onboarding_state.rs`：step 1→2→3 状态机；完成时 emit `AppEvent::OnboardingComplete { agent_name, palette, provider, workspace }` → 写入 AppSettings + 启动 session | P1 |

---

## 评审约束

1. 上述处方是**起点方案**，允许 judge 提议更优替代路径
2. 判定 SPEC 时以 `docs/design/2026-07-22-frontend-redesign/consult-room/` 真值文件为设计规范
3. 判定 QUALITY 时关注：ponytail 原则（最短可行）、复用已有模式、不引入新抽象层、家规合规
4. 不越权 judge Rust compile error（这不是编译审查）
5. 报告格式：

```
## Problem B1
- SPEC: PASS / FAIL / NEEDS CONTEXT
- QUALITY: PASS / FAIL
- Notes: <1-3 sentences>

## Problem F1
...
```

6. 末尾加一行总体评估：
```
## Summary
- Spec compliant: X/Y
- Quality concerns: <list>
- Recommended priority order: <reordered if different>
```
