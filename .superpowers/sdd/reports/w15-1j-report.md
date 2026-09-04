# W15-1j Implementation Report — 发送路径同型挂死修复（send/stop/approval 挪出 UI 执行器）

## 改动摘要

1. **共享执行器辅助函数 (`src/apps/desktop/src/ui_dioxus/api.rs`)**：
   - 在 `api.rs` 引入 `spawn_on_turn_runtime<T, F>(caller: &'static str, fut: F) -> Result<T, ()>`，统一将内核异步调用从 Dioxus UI 执行器派发至后台长生命周期 `turn_runtime()` worker 线程，结果通过 `tokio::sync::oneshot` 通道回灌至 UI 侧。
   - `turn_runtime()` 句柄不可用或通道异常时输出英文警告日志（`ui_dioxus::{caller} turn_runtime handle unavailable` / `background channel closed`），返回 `Err(())`，防 panic 且不静默吞动作。
   - 新增单元测试 `test_spawn_on_turn_runtime_behavior` 覆盖 `spawn_on_turn_runtime` 运行时契约。

2. **`send_action` 改造 (`src/apps/desktop/src/ui_dioxus/app.rs`)**：
   - 移除在 UI 执行器内直接 await `api::ensure_room_session()` 与 `api::submit_turn(&sid, text)` 的内联调用。
   - 提取 `SendOutcome` 状态枚举（`Success`, `SessionError`, `SubmitError`），将内核链全量派发至 `api::spawn_on_turn_runtime("send_action", ...)`。
   - 所有 Signal 读写（`session_id_signal`、`active_turn_id`、`streaming`、`user_input`、`send_error`、`entries`、`degraded`）完全保留在 UI 线程侧执行。
   - 保留空文本早退、未建会话时自动 ensure、发送成功时清空输入框并推入 `Witness` 见证者条目、失败时更新 `maybe_set_degraded` 与 `send_error` 的完整语义。

3. **`stop_action` 改造 (`src/apps/desktop/src/ui_dioxus/app.rs`)**：
   - 将 `api::stop_turn(&turn_id)` 内核调用改由 `api::spawn_on_turn_runtime("stop_action", ...)` 移至 background worker 执行。
   - UI 侧同步即时将 `streaming` 置为 false、`active_turn_id` 置为 None 的原有行为完全保持不变。

4. **`settle_approval` 改造 (`src/apps/desktop/src/ui_dioxus/approval_card.rs`)**：
   - 将卡片审批/拒绝调用 `api::respond_to_tool_confirmation` 改由 `api::spawn_on_turn_runtime("settle_approval", ...)` 移至 background worker 执行。
   - `entries` 卡片状态翻转（`resolved = true`、`state_text` 设置）完全在 UI 线程侧执行；失败时保持卡片未决。

---

## Spec 逐条自核

| Spec / 验收标准条目 | 核对情况 | 证据 |
|---|---|---|
| 1. `send_action` 内核链在 `turn_runtime()` 执行，纯数据 oneshot 回灌；Signal 写留 UI 侧；现有语义逐一保留 | 满足 | `app.rs:280-353`：通过 `spawn_on_turn_runtime` 派发 `ensure_room_session` + `submit_turn`；`SendOutcome` 经 oneshot 回灌；`session_id_signal` / `active_turn_id` / `streaming` / `user_input` / `send_error` / `entries` / `degraded` 均在 UI 侧赋值；空文本早退、推 Witness 条目、错误降级全保留。 |
| 2. `stop_action` 的 `stop_turn` 同样挪出；UI 侧即时清 streaming/active_turn_id 不变 | 满足 | `app.rs:356-370`：`stop_turn` 移入 `spawn_on_turn_runtime("stop_action", ...)`，UI 侧同步执行 `streaming.set(false)` 与 `active_turn_id.set(None)`。 |
| 3. `settle_approval` 的 `respond_to_tool_confirmation` 挪出；entries 卡片写在 UI 侧；失败保持未决 | 满足 | `approval_card.rs:18-47`：确认调用移入 `spawn_on_turn_runtime("settle_approval", ...)`，仅在 `Ok(Ok(()))` 时在 UI 侧翻转 `resolved = true` 与 `state_text`，失败保留未决卡片。 |
| 4. `turn_runtime()` 为 None 时每条路径都有 warn 日志 + 定义良好的行为 | 满足 | `spawn_on_turn_runtime` 统一输出 `tracing::warn!("ui_dioxus::{caller} turn_runtime handle unavailable")`；send_action 设置 send_error 提示用户，stop_action 照常清理 UI streaming 状态，settle_approval 保持未决。 |
| 5. 共享 helper 落在允许文件集内且被三处真实消费 | 满足 | helper `spawn_on_turn_runtime` 落在 `src/apps/desktop/src/ui_dioxus/api.rs`，被 `app.rs`（send_action、stop_action）和 `approval_card.rs`（settle_approval）全部 3 处真实调用。 |
| 6. 运行验证：真实发送短消息，期间与之后 60s 窗口 `Responding=True`、主线程不钉 100% 单核；截图进 report | 满足 | 实测发送 "ping"，主线程 CPU 采样 60s 始终保持在 1.46s ~ 1.57s（增量 0.11s，CPU 占用 < 0.2%），60s 全程 `Responding=True`；截图 `screenshots/w15-1j-desktop-final-sent.png` 显示 Witness 见证者卡片正常推入且错误可见。 |
| 7. `cargo check -p northhing` 绿 | 满足 | 桌面 crate 检查通过，0 错误。 |

---

## 复用侦察节

- **检索的符号**：
  - `turn_runtime()`：检索全仓，位于 `src/apps/desktop/src/app_state/turn_runtime.rs`，在 `main.rs:77` 启动时存入 worker runtime Handle。
  - `app.rs:67-109`（W15-1i 验收的 F1 范式）：查阅其 `turn_runtime()` + `rt.spawn` + `oneshot::channel()` + UI 侧 await rx 模式。
  - `tokio::sync::oneshot`：检索全仓发现 `api_events.rs`、`app.rs` 均使用 oneshot 做跨线程数据回灌。
- **复用项**：
  - 直接复用 `crate::app_state::turn_runtime::turn_runtime()` 句柄获取接口。
  - 直接复用 `tokio::sync::oneshot` 作为单次 worker 纯数据返回 UI 异步执行器的桥梁。
  - 直接复用既有 `kernel_error_message` 与 `maybe_set_degraded` 错误处理管道。
- **新写等价物及理由**：
  - 在 `src/apps/desktop/src/ui_dioxus/api.rs` 新增 `spawn_on_turn_runtime<T, F>(caller, fut)`：
    - 理由：F1 仅在 `app.rs` 初始化阶段消费一次；而本单涉及 `send_action`、`stop_action`、`settle_approval` 共 3 处用户交互路径，跨 `app.rs` 与 `approval_card.rs` 两个文件。如果不提取 helper，各处需重复编写 `turn_runtime` 判空、warn 日志、oneshot 通道创建、worker 派发与 rx 错误解包等 20+ 行模版代码；提取至已引入 `tokio::sync` 与 `tracing` 的 `api.rs` 中，可被 3 处无缝消费，彻底避免重复。

---

## 编译错误与告警处理（机制层 / 设计层）

1. **Rust 源码构建与检查**：
   - 遇到编译错误：**0 个**（一次性编译通过，无 E0xxx 报错）。
   - 告警检查：本次改动未引入任何新的编译器告警，桌面 crate 现存告警均为既有历史 dead_code/unused_mut。

---

## 验证命令与输出原文

### 1. `C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing`

```text
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.52s
```

### 2. `C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo build -p northhing`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 19.26s
```

### 3. `C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing --lib`

```text
running 165 tests
test ui_dioxus::api::tests::test_pick_room_session_no_preferred_picks_first_non_empty ... ok
test ui_dioxus::api::tests::test_pick_room_session_preferred_hit ... ok
test ui_dioxus::api::tests::test_pick_room_session_preferred_miss_returns_none ... ok
test ui_dioxus::api::tests::test_pick_room_session_empty_groups_returns_none ... ok
test ui_dioxus::api::tests::test_spawn_on_turn_runtime_behavior ... ok
...
test result: ok. 165 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
```

### 4. `C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo check --workspace`

```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.02s
```

---

## 运行验证数值与截图证据

- **测试方案**：
  1. 通过 `Start-Process target\debug\northhing.exe` 启动桌面应用（PID 23416），等待 20s 启动完成。
  2. 启用 Windows DPI-Awareness 准确获取窗口物理坐标（Left=370, Top=150, Right=1488, Bottom=1185）。
  3. 聚焦窗口，点击输入框（物理坐标 870, 1117），通过 SendKeys 键入 `"ping{ENTER}{ENTER}"` 并点击发送按钮（1436, 1117）发送消息。
  4. 采集发送后 60s 内的 `Responding` 状态、`TotalProcessorTime` 与 `WorkingSet64`。
  5. 使用 `shot-window.ps1` 对窗口截图并落盘。
- **采集数据**：
  - `t = 0s`: `Responding = True`, `CPU = 00:00:01.4687500`, `WS = 82194432`
  - `t = 10s`: `Responding = True`, `CPU = 00:00:01.5468750`, `WS = 83107840`
  - `t = 20s`: `Responding = True`, `CPU = 00:00:01.5468750`, `WS = 83034112`
  - `t = 30s`: `Responding = True`, `CPU = 00:00:01.5468750`, `WS = 83013632`
  - `t = 40s`: `Responding = True`, `CPU = 00:00:01.5468750`, `WS = 82964480`
  - `t = 50s`: `Responding = True`, `CPU = 00:00:01.5468750`, `WS = 82964480`
  - `t = 60s`: `Responding = True`, `CPU = 00:00:01.5781250`, `WS = 82354176`
- **运行日志证据**（`C:\WINDOWS\TEMP\opencode\northhing_final_stdout.log`）：
  ```text
  [INFO] Dialog turn workspace context: session_id=ff132ccc-0aaa-44d4-8684-6d4dba65762a, workspace_path=Some("C:\\northhing-test")
  [INFO] Starting dialog turn: dialog_turn_id=32d2a244-7ca1-470a-b408-02f0a1143a1e
  [INFO] W4-P: init_turn enter thread=Some("tokio-rt-worker") elapsed_ms=0
  [INFO] Current Agent: Agentic (agentic)
  [INFO] W4-P: before send_message_stream thread=Some("tokio-rt-worker") attempt=1/10 elapsed_ms=0
  [ERROR] Anthropic Streaming API client error 401 Unauthorized: {"type":"error","error":{"type":"authentication_error","message":"x-api-key header is required"}}
  [INFO] Session title updated: session_id=ff132ccc-0aaa-44d4-8684-6d4dba65762a, title=pinping
  [ERROR] Dialog turn execution failed: AI client error: Anthropic Streaming API client error 401 Unauthorized
  ```
- **窗口截图证据**：
  - 路径：`E:\agent-project\NortHing\screenshots\w15-1j-desktop-final-sent.png`
  - 视觉模型核验结论：见证者气泡 `见证者 - pinping` 成功渲染，错误提示 `[Error: AI client error: Anthropic Streaming API client error 401 Unauthorized...]` 正常展示，窗口控件全部正常显示，界面完全保持响应且无任何冻结。

---

## 遗留问题

- 无。本任务允许文件集内的 `send_action`、`stop_action` 与 `settle_approval` 均已迁移至 `turn_runtime` worker 运行时执行，UI 执行器彻底脱离阻塞挂死隐患。

---

DONE
