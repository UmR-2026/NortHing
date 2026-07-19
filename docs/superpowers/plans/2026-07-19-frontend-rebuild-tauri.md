# Frontend Rebuild (Tauri + React) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (local: coder-lc implementer, judge-m3 reviewer) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Dispatch briefs for F1+ UI tasks carry the exact component code (local convention: prescription-level briefs at dispatch); this plan locks file layout, interfaces, invariants, and acceptance.

**Goal:** Replace the Slint desktop UI with a Tauri 2 + React chat-app-style frontend, keeping the proven Rust core/runtime topology, and reserve a generic web-panel plug point for future ComfyUI-style integrations.

**Architecture:** New shell at `src/apps/desktop-tauri` (React+Vite frontend `ui/`, Tauri 2 Rust crate `src-tauri/`, excluded from the main cargo workspace like northing-installer). The Rust side spawns the SAME worker-thread multi-thread tokio runtime as the Slint app and initializes core services identically; turn dispatch goes to that worker runtime via a stored `tokio::runtime::Handle` (W4 discipline). Core events reach the frontend through a Tauri `AppHandle::emit` bridge registered as an internal `EventSubscriber` (W4b discipline: no block_on in subscriber context). The Slint app (`src/apps/desktop`) stays buildable and untouched until F4 baseline flip.

**Tech Stack:** Tauri 2, React 18 + Vite + TypeScript (mirrors northing-installer), react-markdown + rehype-highlight for assistant rendering, Rust workspace core crates unchanged.

**Style direction:** chat-app (会话列表 | 消息流 | 输入区 三列，Inspector 可折叠), dark theme, normal-sized bubbles (max-width ~720px message column), markdown + code highlighting.

## Global Constraints

- v0.1.0 基线变更须用户确认：F4 前 Slint 仍是发货面；F4 由用户拍板后翻基线并同步 AGENTS.md 骨架不变量节。
- **Runtime discipline (W4/W4b/D2i/D2j 血泪教训，全部任务隐含遵守)**:
  - Core 初始化与 turn dispatch 只能在 worker 线程的多线程 tokio runtime 上；Tauri 命令处理函数一律用 `tauri::async_runtime::spawn` 转发到该 runtime 的 `Handle`（见 F0 Task 2 的 `core_rt()`）。
  - 禁止在任何 async 上下文里 `block_on` 或新建 runtime。
  - `tokio::spawn` 的任务必须确认落在长生命周期 runtime 上。
  - UI 状态更新只走 Tauri event emit / command 返回值（无 Slint 跨线程写问题，但 emit 必须是 sync 调用）。
- Config 单一来源 = core `GlobalConfig`；Tauri 侧只读/写经由 core 服务，禁止第二个 runtime 可读配置文件。
- Logs English-only; UI copy Chinese（v0.1.0 现状沿用，i18n 工程冻结）。
- 验证：Rust 侧 `cargo check --manifest-path src/apps/desktop-tauri/src-tauri/Cargo.toml`；前端 `pnpm --dir src/apps/desktop-tauri/ui run type-check`；主仓 `cargo check -p northhing` 不许被弄坏。
- 不动 `src/apps/desktop`（Slint）与 `northing-installer`，直到 F4。
- 提交风格：小步快走，每个 Task 至少一个 commit，message 用仓库现有风格（`feat(desktop-tauri): ...` / `fix(desktop-tauri): ...`）。

---

## F0 — Tauri 壳 + core 内嵌 + 最小聊天打通（本计划唯一全处方任务）

**验收**: `pnpm --dir src/apps/desktop-tauri run tauri:dev` 启动窗口，输入 "hi" 发送后能看到纯文本流式回复逐字出现；关闭窗口进程干净退出。

### Task 0.1: 工程脚手架

**Files:**
- Create: `src/apps/desktop-tauri/src-tauri/Cargo.toml`
- Create: `src/apps/desktop-tauri/src-tauri/build.rs`
- Create: `src/apps/desktop-tauri/src-tauri/tauri.conf.json`
- Create: `src/apps/desktop-tauri/src-tauri/src/main.rs`
- Create: `src/apps/desktop-tauri/package.json`
- Create: `src/apps/desktop-tauri/ui/package.json`, `ui/index.html`, `ui/vite.config.ts`, `ui/tsconfig.json`, `ui/src/main.tsx`, `ui/src/App.tsx`
- Modify: `Cargo.toml` (root) — `exclude` 数组加 `"src/apps/desktop-tauri/src-tauri"`

**Interfaces:**
- Produces: crate `northhing-desktop-tauri`（bin name 同），Tauri identifier `com.northhing.desktop`，devUrl `http://localhost:5173`，frontendDist `../ui/dist`。

- [ ] **Step 1: src-tauri/Cargo.toml**（仿 northing-installer 的 Tauri 2 配置，但无 embed-resource/无 rlib 约束）

```toml
[package]
name = "northhing-desktop-tauri"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
northhing-core = { path = "../../../crates/assembly/core" }
```

- [ ] **Step 2: build.rs**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 3: tauri.conf.json** — productName `northhing`, identifier `com.northhing.desktop`, `build.devUrl = "http://localhost:5173"`, `build.frontendDist = "../ui/dist"`, 单窗口 1280x800 标题 `northhing`。图标先复用 `northing-installer/src-tauri/icons/` 里现有文件（拷贝过来，不改 installer）。
- [ ] **Step 4: React 脚手架** — Vite + React 18 + TS（版本对齐 northing-installer 的 react ^18.3.1），`ui/package.json` scripts: `dev: vite --port 5173 --strictPort`, `build: tsc && vite build`, `type-check: tsc --noEmit`。`App.tsx` 先渲染占位文本。**顶层 `src/apps/desktop-tauri/package.json`**（仿 northing-installer/package.json）scripts: `tauri:dev: "tauri dev"`（cwd 指向 src-tauri 或用 `pnpm --dir ui run dev` 并行，按 northing-installer 的实际做法抄）、`tauri:build: "tauri build"`、`type-check: "pnpm --dir ui run type-check"`，devDependencies 加 `@tauri-apps/cli` ^2。
- [ ] **Step 5: main.rs 最小 Builder**

```rust
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running northhing desktop");
}
```

- [ ] **Step 6: 根 Cargo.toml exclude 加 `"src/apps/desktop-tauri/src-tauri"`。**
- [ ] **Step 7: 验证 + commit**
  - `cargo check --manifest-path src/apps/desktop-tauri/src-tauri/Cargo.toml` → 0 error
  - `pnpm --dir src/apps/desktop-tauri/ui install && pnpm --dir src/apps/desktop-tauri/ui run type-check` → 0 error
  - `cargo check -p northhing` → 0 error（证明主仓未受影响）
  - commit `feat(desktop-tauri): scaffold Tauri 2 + React shell (F0.1)`

### Task 0.2: worker runtime + core 初始化 + turn runtime Handle

**Files:**
- Create: `src/apps/desktop-tauri/src-tauri/src/core_rt.rs`
- Modify: `src/apps/desktop-tauri/src-tauri/src/main.rs`

**Interfaces:**
- Produces: `pub fn core_rt() -> tokio::runtime::Handle`（OnceLock 存储；F0.3/F1+ 所有命令都用它 spawn core 调用）; `pub fn core_ready() -> bool`。

- [ ] **Step 1: core_rt.rs**

```rust
//! Worker runtime for core services (W4 discipline).
//! All core calls (init, scheduler submit, coordinator reads) MUST be
//! spawned onto this long-lived multi-thread runtime. Never block_on
//! inside async contexts; never build per-call runtimes.
use std::sync::OnceLock;
use tokio::runtime::Handle;

static CORE_RT: OnceLock<Handle> = OnceLock::new();
static CORE_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn core_rt() -> Handle {
    CORE_RT.get().expect("core runtime not initialized").clone()
}

pub fn core_ready() -> bool {
    CORE_READY.load(std::sync::atomic::Ordering::SeqCst)
}

/// Spawn the worker thread that owns the core runtime and runs
/// core-service initialization. Mirrors src/apps/desktop/src/main.rs.
pub fn init_core_runtime() {
    std::thread::Builder::new()
        .name("northhing-core-rt".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to build core runtime");
            let _ = CORE_RT.set(runtime.handle().clone());
            if let Err(e) = runtime.block_on(init_services()) {
                eprintln!("Error: failed to initialize core services: {e}");
            } else {
                CORE_READY.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            // Keep the runtime alive for the app lifetime (shutdown channel
            // mirrors desktop main.rs and preserves graceful MCP shutdown).
            let (_tx, rx) = std::sync::mpsc::channel::<()>();
            let _ = rx.recv();
        })
        .expect("failed to spawn core runtime thread");
}

async fn init_services() -> anyhow::Result<()> {
    northhing_core::service::config::initialize_global_config().await?;
    northhing_core::infrastructure::ai::AIClientFactory::initialize_global().await?;
    let _system = northhing_core::agentic::system::init_agentic_system().await?;
    // Wire the dialog scheduler (mirrors desktop init_agentic_system_for_desktop).
    let coordinator = _system.coordinator.clone();
    let session_manager = coordinator.session_manager().clone();
    let scheduler = northhing_core::agentic::coordination::DialogScheduler::new(
        coordinator.clone(),
        session_manager,
    );
    let notifier_ok = coordinator.set_scheduler_notifier(scheduler.outcome_sender());
    let injection_ok = coordinator.set_round_injection_source(scheduler.round_injection_monitor());
    anyhow::ensure!(notifier_ok && injection_ok, "dialog scheduler wiring conflict");
    northhing_core::agentic::coordination::set_global_scheduler(scheduler.clone());
    // Mirror desktop main.rs P0-D: register a global MCPService and init in background.
    match northhing_core::service::config::get_global_config_service().await {
        Ok(cfg_svc) => match northhing_core::service::mcp::MCPService::new(cfg_svc) {
            Ok(mcp_service) => {
                let mcp_service = std::sync::Arc::new(mcp_service);
                northhing_core::service::mcp::set_global_mcp_service(mcp_service.clone());
                tokio::spawn(async move {
                    if let Err(e) = mcp_service.server_manager().initialize_all().await {
                        tracing::warn!("failed to initialize MCP servers: {e}");
                    }
                });
            }
            Err(e) => tracing::warn!("failed to construct MCPService: {e}"),
        },
        Err(e) => tracing::warn!("failed to fetch global config service: {e}"),
    }
    tracing::info!("core services initialized (desktop-tauri)");
    Ok(())
}
```

（实施时注意：上述 core API 路径以 `src/apps/desktop/src/agent/agentic_system.rs` 实际调用为准，不一致就照它抄。）

- [ ] **Step 2: main.rs 在 `tauri::Builder::default()` 之前调用 `core_rt::init_core_runtime()`；tracing_subscriber fmt INFO 初始化。**
- [ ] **Step 3: 验证 + commit** — `cargo check` 0 error；commit `feat(desktop-tauri): worker runtime + core init (F0.2)`

### Task 0.3: 事件桥（AgenticEvent → Tauri emit）+ 最小聊天闭环

**Files:**
- Create: `src/apps/desktop-tauri/src-tauri/src/event_bridge.rs`
- Create: `src/apps/desktop-tauri/src-tauri/src/commands.rs`
- Modify: `src/apps/desktop-tauri/src-tauri/src/main.rs`
- Modify: `src/apps/desktop-tauri/ui/src/App.tsx`
- Create: `src/apps/desktop-tauri/ui/src/api.ts`

**Interfaces:**
- Consumes: `core_rt()` / `core_ready()`（Task 0.2）。
- Produces:
  - Commands: `create_session() -> String`, `list_sessions() -> Vec<SessionMetaDto>`, `send_message(session_id: String, text: String) -> Result<(), String>`, `get_messages(session_id: String) -> Vec<MessageDto>`
  - Events (Tauri emit, 前端 `listen`): `chat://chunk` payload `{ session_id, text }`, `chat://turn-state` payload `{ session_id, state: "started"|"completed"|"failed"|"cancelled", error?: string }`
  - DTO: `SessionMetaDto { id, name, updated_at }`, `MessageDto { id, role, content, is_streaming }`

- [ ] **Step 1: event_bridge.rs** — `TauriEventBridge { app: tauri::AppHandle }` 实现 `northhing_core::agentic::events::EventSubscriber`；`on_event` 里对 `TextChunk` emit `chat://chunk`，对 `DialogTurnStarted/Completed/Failed/Cancelled` emit `chat://turn-state`。**只调 sync emit，不 await 任何 IO**（W4b 纪律）。注册函数 `pub fn register(app: &tauri::AppHandle)`：`global_coordinator()` 可用时 `subscribe_internal("desktop-tauri", bridge)`，否则 spawn 到 `core_rt()` 里每 500ms 重试直到 coordinator 出现（初始化竞态）。注册调用必须 Arc 包装：`coordinator.subscribe_internal("desktop-tauri".to_string(), Arc::new(bridge) as Arc<dyn northhing_core::agentic::events::EventSubscriber>);`
- [ ] **Step 2: commands.rs** — 四个命令，body 模式统一：
  ```rust
  #[tauri::command]
  async fn send_message(session_id: String, text: String) -> Result<(), String> {
      let (tx, rx) = tokio::sync::oneshot::channel();
      crate::core_rt::core_rt().spawn(async move {
          let r = do_send(session_id, text).await.map_err(|e| e.to_string());
          let _ = tx.send(r);
      });
      rx.await.map_err(|_| "core runtime dropped".to_string())?
  }
  ```
  `do_send` 内部走 `global_scheduler().submit(...)`（参数照抄 `callbacks_lifecycle.rs:146-160` 的 submit 调用：mode 用 `agentic`，`DialogSubmissionPolicy::for_source(DialogTriggerSource::DesktopApi)`）。`create_session`/`list_sessions`/`get_messages` 走 `global_coordinator()` 对应方法（以 coordinator 公开 API 为准，参考 callbacks_lifecycle.rs / sessions.rs 的现有调用）。
- [ ] **Step 3: main.rs setup** — `tauri::Builder::default().setup(|app| { event_bridge::register(&app.handle()); Ok(()) }).invoke_handler(tauri::generate_handler![create_session, list_sessions, send_message, get_messages])`
- [ ] **Step 4: ui/src/api.ts + App.tsx** — 最小聊天页：输入框+发送按钮+消息列表（纯文本 `<pre>` 即可，本任务不做样式）。`listen('chat://chunk')` 追加到当前 streaming 消息；`chat://turn-state` completed/failed 时 finalize 并 `get_messages` 刷新。启动时若无 session 自动 `create_session`。
- [ ] **Step 5: 端到端验证 + commit**
  - `cargo check --manifest-path ...` 0 error；`type-check` 0 error
  - 构建运行（`pnpm --dir src/apps/desktop-tauri run tauri:dev` 或 `tauri build --no-bundle` 后跑 exe），发 "hi" → 流式回复逐字出现。**若挂起/无渲染：禁止猜测，按 systematic-debugging 加探针定位**（W4 教训：先确认 turn 是否在跑、事件是否 emit、前端是否 listen 到）。
  - commit `feat(desktop-tauri): event bridge + minimal chat loop (F0.3)`

---

## F1 — 聊天主界面（聊天应用风）

**验收**: 三列布局；会话列表（新建/选择/删除/重命名/时间排序）；消息流气泡正常尺寸（消息列 max-width ~720px，气泡不撑满）；markdown 渲染 + 代码块高亮 + 复制按钮；输入区 Enter 发送/Shift+Enter 换行（替代现状 Cmd+Enter）；流式光标与停止按钮（接 `on_stop_streaming` 等价的 cancel 命令）；空态/加载态/错误横幅（接 `chat://turn-state` failed）。

- Task 1.1: 设计 token + 布局骨架（sidebar / message column / composer；暗色主题；字号/间距 scale）
- Task 1.2: 会话列表 + 会话生命周期命令（delete/rename 已在 core 有现成路径，参考 Slint 回调）
- Task 1.3: 消息流 + 气泡（user/assistant 区分，正常尺寸——顺带关掉"气泡太大"问题）
- Task 1.4: markdown 渲染（react-markdown + rehype-highlight；代码块高亮+复制）
- Task 1.5: composer（autosize textarea、发送态、停止按钮 → cancel 命令）
- 每 Task 验证：`type-check` + 人工/自动化截图走查；commit 每 Task 一个。

## F2 — 设置 / onboarding / Inspector 迁移（含测试连接修复）

**验收**: 设置页全量功能（provider CRUD、设为默认、**测试连接真实可用**：结果就地显示 ✓/失败原因，in-flight 态禁用按钮）；workspace 管理；skills 开关；MCP 状态；onboarding 首启流程；Inspector（model pill 显示模型名而非 wire format——修遗留小问题）。

- Task 2.1: settings 命令层（providers/workspaces/skills/mcp CRUD + `test_provider(id)` / `test_provider_config(form)` 命令，逻辑照抄 `src/apps/desktop/src/app_state/callbacks_settings.rs` 的 `register_test_provider_callback`（:508-）与 `register_test_provider_config_callback`（:649-）现实现，返回值带 `success + first_line_error`）
- Task 2.2: 设置页 UI（表单 + 列表 + 测试按钮接线正确：测试只调 test 命令，保存只调 upsert——修掉 Slint 版"测试按钮错接 upsert"的 bug）
- Task 2.3: onboarding 流程迁移（首启判定参考 Slint first-run 逻辑）
- Task 2.4: Inspector 面板（model 显示名修复——wire format 泄漏源头在 `src/apps/desktop/src/app_state/inspector.rs`，迁移时改为显示模型名；MCP/skills 状态）

## F3 — Web 面板插口（ComfyUI 预留）

**验收**: `panels.json`（路径 `dirs::config_dir()/northhing/config/panels.json`，schema `{ "panels": [{ "name": string, "url": string, "icon"?: string }] }`）声明即在导航栏出现入口，点击渲染 iframe 面板；内置一条注释掉的 ComfyUI 示例（`http://127.0.0.1:8188`）；面板加载失败有可见错误态；远端内容 CSP/导航限制到声明的 URL。

- Task 3.1: panels 配置读取命令 + 前端面板容器（iframe + 错误态）
- Task 3.2: 导航集成 + 安全约束（只允许配置的 URL，禁窗口跳转）

## F4 — 基线切换

**验收（需用户拍板）**: F0-F2 人工验收通过后：安装器产出切换为 Tauri 桌面（或双发过渡期）；AGENTS.md 骨架不变量节更新（"Desktop package is northhing (Slint)" 条目改写）；Slint UI 标记 frozen/移除计划另行决定；本计划全部 commit 过 judge 复审 + handoff 更新。

---

## 风险与备注

- **F0.3 是全网最高点**：runtime 拓扑与事件链在新壳里第一公里；挂了按 systematic-debugging 处理，禁止边猜边改。
- Tauri 命令的 async 运行在 Tauri 自己的 runtime——所有 core 调用必须经 `core_rt().spawn` 转发（F0.3 命令模式即如此），否则 reqwest/锁/runtime 上下文会出 W4 同类问题。
- 主仓 `cargo check -p northhing` 必须始终绿；desktop-tauri 不参与主 workspace。
- 工作量预估：F0 ≈ 1-2 个 coder 单，F1 ≈ 3-5 单，F2 ≈ 3-4 单，F3 ≈ 1 单，F4 为编排/验收工作。
