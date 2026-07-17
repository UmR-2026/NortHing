# 技术债调查报告 — northhing 仓库

**调查时间**: 2026-07-18  
**仓库路径**: E:\agent-project\northing  
**调查员**: subagent (depth 1/1)

---

### [P0] 桌面消息排队问题：运行中发送消息被拒 + 吞输入

- **现象**: 桌面端在 dialog turn 运行期间，用户输入的消息没有排队机制。`on_send_message` 回调直接调用 `coordinator.start_dialog_turn`，没有检查 `streaming_session` 状态。如果已有 turn 在运行，新消息的行为取决于 coordinator 层是否接受并发 turn。UI 层没有禁用输入框（`is-streaming` 属性绑定到了 ChatPaneView 的视觉效果，但未阻止 `on_send_message` 回调），用户可以继续打字并提交，但消息可能被静默丢弃或导致状态混乱。
- **证据**: 
  - `src/apps/desktop/src/app_state/callbacks_lifecycle.rs:22-67` — `register_send_message_callback` 中，`on_send_message` 闭包没有检查 `app_state.get_streaming_session()`，直接进入 `start_dialog_turn`
  - `src/apps/desktop/src/app_state/state.rs:35,136-151` — `streaming_session` 状态存在但未用于门控输入
  - `src/apps/desktop/src/ui/main.slint:92,258` — `is-streaming` 属性传递给了 ChatPaneView，但仅用于视觉指示，未禁用输入
  - 代码注释 `callbacks_lifecycle.rs:155` 表明 turn 完成后通过 event bridge 清理 streaming 状态，但运行期间无保护
- **建议修法**: 
  1. 在 `on_send_message` 回调入口检查 `app_state.get_streaming_session()`，如果与当前 session 匹配则将消息入队（pending message queue）而非直接 `start_dialog_turn`
  2. 或在 Slint 层用 `is-streaming` 绑定禁用输入框 + 发送按钮
  3. 实现 `DialogSteeringAction`（已有 `RoundInjection` 类型定义于 `agent_dialog.rs`）的消费路径，让排队消息在下一 round 注入
- **状态**: active

---

### [P0] AskUserQuestion 无超时 + 工具执行无 cancel select + turn 无总超时（挂死三件套）

- **现象**: 
  1. **AskUserQuestion 无超时**: `user_questions.rs` 中定义了 `AskUserQuestionInput` / `validate_ask_user_question_input`，但没有任何 `timeout` 字段或超时逻辑。问题提出后无限等待用户回答。
  2. **工具执行无 cancel select**: 工具执行路径中，`tokio::select!` 分支覆盖了 cancel token（`scheduler_cancel.rs`, `turn_cancel.rs` 存在 cancel 逻辑），但 AskUserQuestion 工具本身是一个阻塞等待用户输入的工具，它内部的 future 不响应 cancel — 即便 turn 被 cancel，AskUserQuestion 的等待不会中断。
  3. **turn 无总超时**: 搜索 `turn.*timeout` / `overall.*timeout` / `TURN_TIMEOUT`，没有找到 dialog turn 级别的总超时机制。只有 subagent 有 `timeout_seconds`（`so_lifecycle/lifecycle.rs:142`），但主 dialog turn 没有。
- **证据**:
  - `src/crates/execution/agent-runtime/src/user_questions.rs:1-80` — 完整文件无 timeout 相关代码
  - `src/crates/assembly/core/src/agentic/tools/user_input_manager.rs` — 无 timeout
  - `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_cancel.rs` — 存在 cancel 逻辑，但仅 cancel turn 级别的 token，不直接解决 AskUserQuestion 工具内部阻塞的问题
  - `src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_lifecycle/lifecycle.rs:142` — subagent 有 `timeout_seconds`，主 turn 没有
  - 搜索 `turn_timeout` / `TURN_TIMEOUT` / `overall.*timeout` 在整个 `src/` 中无匹配
- **建议修法**:
  1. 给 `AskUserQuestion` 工具加 `timeout_ms` 参数 + 默认值（如 300000ms/5min），超时自动返回 "user did not respond" 结果
  2. 在 AskUserQuestion 工具实现中使用 `tokio::select!` 包裹用户输入等待 + cancel_token，使 turn cancel 能中断工具
  3. 在 dialog turn 级别增加可配置的总超时（如 30min），超时后自动 cancel turn 并发出 `DialogTurnFailed` 事件
- **状态**: active

---

### [P1] 配置非原子写

- **现象**: `save_app_settings` 函数使用 `tokio::fs::write` 直接写入 `app.json`，没有采用 temp-file + rename 的原子写模式。代码注释明确承认了这一点："Phase 1: simple write — upgrade to atomic in Phase 5 if race conditions surface"。如果写入过程中进程崩溃或系统断电，配置文件可能被截断或损坏。
- **证据**: 
  - `src/apps/desktop/src/app_state/settings.rs:655-667` — 注释：`/// Atomic write via tmp-file + rename (Phase 1: simple write — upgrade to atomic in Phase 5 if race conditions surface).` 实际代码：`tokio::fs::write(&path, json).await`
  - `src/crates/assembly/core/src/infrastructure/storage/persistence.rs:15-20` — 存在 `FILE_LOCKS` 文件锁机制防止并发写入，但不是原子写
- **建议修法**: 写入临时文件 `app.json.tmp`，然后 `tokio::fs::rename` 覆盖目标文件（rename 在同一文件系统上是原子的）。已有 persistence.rs 中的 file lock 可配合使用。
- **状态**: active（注释中明确标注为 Phase 5 待修复）

---

### [P1] API key 明文存储

- **现象**: `ProviderConfig.api_key` 字段以明文存储在 `app.json` 中。代码注释直接承认："Stored in plaintext in app.json. Never logged." 没有使用 keyring、加密或任何保护机制。
- **证据**:
  - `src/apps/desktop/src/app_state/settings.rs:104-105` — `/// Stored in plaintext in app.json. Never logged.` + `pub api_key: String,`
  - 搜索 `keyring` / `encrypt` 在整个 `src/` 中无匹配（除了 relay-server 中 WebSocket 的 E2E 加密，与 API key 存储无关）
- **建议修法**:
  1. 短期：使用 OS keyring（`keyring` crate on Rust）存储 API key，`app.json` 中只存引用
  2. 中期：如果不能用 keyring，至少用 AES-256-GCM 加密存储，密钥派生自机器 ID + 用户密码
  3. 长期：支持环境变量注入，完全不在磁盘上存储 key
- **状态**: active

---

### [P1] Delete 不走回收站

- **现象**: `delete_local_path` 函数直接调用 `fs::remove_file` / `fs::remove_dir_all`，不经过回收站。远程删除使用 `rm -rf`。删除操作不可恢复。
- **证据**:
  - `src/crates/execution/tool-execution/src/fs/delete_path.rs:49-64` — `fs::remove_file` / `fs::remove_dir` / `fs::remove_dir_all`
  - `src/crates/execution/tool-execution/src/fs/delete_path.rs:70-75` — `build_remote_delete_command` 使用 `rm -rf` / `rm -f`
  - 搜索 `trash` / `recycle` 在 `src/` 中无匹配（除了无关的 cleanup.rs 中的 trash 注释）
- **建议修法**:
  1. 使用 `trash` crate（跨平台回收站支持）替代 `fs::remove_*`
  2. 或增加一个配置选项让用户选择是否走回收站
  3. 远程删除可以通过 `trash-put` 命令（如果远程已安装）或保持 `rm` 但加确认提示
- **状态**: active

---

### [P1] 移动端重配对无引导 + i18n 乱码

- **现象**: 
  1. **重配对无引导**: `PairingPage.tsx` 实现了配对流程，但未发现"重配对"引导逻辑。当配对失效（如 desktop 端重启、relay 变更）时，用户需要手动重新走配对流程，没有引导性的错误提示或重连向导。`useConnectionHealth.ts` 中有 `unpaired` 状态检测，但只是设置状态，无引导。
  2. **i18n 乱码**: mobile-web 有完整的 i18n 系统（`i18n/messages.ts`, `localeRegistry.ts`, `LanguageToggleButton.tsx`），支持 `en-US` 和 `zh-CN`。但在 Rust 桌面端代码中存在大量 UTF-8 编码损坏的中文字符串（如 `callbacks_lifecycle.rs` 中的 `set_inline_error(&ui, "褰撳墠娌℃湁姝ｅ湪杩愯鐨勫洖澶?")` — 这是 GBK/UTF-8 编码混乱导致的乱码）。桌面端 Slint UI 中的中文也是乱码（如 `settings.rs` 中 `ProviderType::display_label` 的 `鑷畾涔?(OpenAI 鍏煎)`）。
- **证据**:
  - `src/mobile-web/src/pages/PairingPage.tsx` — 有配对逻辑，无重配对引导
  - `src/mobile-web/src/hooks/useConnectionHealth.ts` — 有 `unpaired` 状态
  - `src/apps/desktop/src/app_state/callbacks_lifecycle.rs:534` — `"褰撳墠娌℃湁姝ｅ湪杩愯鐨勫洖澶?")` — 明显的 UTF-8 乱码
  - `src/apps/desktop/src/app_state/settings.rs:88-89` — `鑷畾涔?(OpenAI 鍏煎)` — 乱码
  - `src/apps/desktop/src/app_state/event_bridge.rs:229` — `"LLM 璋冪敤澶辫触: {error}"` — 乱码
  - `src/mobile-web/src/i18n/messages.ts` — i18n 系统本身完整可用
- **建议修法**:
  1. 在 `PairingPage` 中检测配对失效后，显示"连接已断开，点击重新配对"的引导 UI
  2. 修复桌面端所有中文乱码：这些是 GBK 编码的字节被当作 UTF-8 解释导致的。需要重新用正确的 UTF-8 编码写入中文字符串，或将所有面向用户的字符串迁移到 i18n 系统
  3. 桌面端可以借鉴 mobile-web 的 i18n 模式，建立统一的字符串管理
- **状态**: active

---

### [P1] relay 默认 0.0.0.0 无鉴权

- **现象**: Relay server 默认监听 `0.0.0.0:9700`，对局域网内所有设备开放。默认不启用 API key 鉴权（`api_key: None`）。虽然 2026-06-26 添加了 `RELAY_API_KEY` 环境变量支持，但默认配置仍然是无鉴权的。CORS 也是通配符 `*`。
- **证据**:
  - `src/apps/relay-server/src/config.rs:30` — `listen_addr: ([0, 0, 0, 0], 9700).into(),`
  - `src/apps/relay-server/src/config.rs:41` — `cors_allow_origins: vec!["*".to_string()],`
  - `src/apps/relay-server/src/config.rs:42` — `api_key: None,` (默认无鉴权)
  - `src/apps/relay-server/src/config.rs:63-67` — `RELAY_API_KEY` 环境变量可选启用鉴权
  - `src/apps/relay-server/src/routes/api.rs:32-72` — `AuthExtractor` 实现了 X-API-Key 头验证，但仅在 `api_key` 为 `Some` 时生效
  - 注释：`// SECURITY: wildcard CORS — acceptable for local dev, must be restricted in production deployment.`
  - 注释：`// None disables authentication (development mode only). Production deployments MUST set RELAY_API_KEY`
- **建议修法**:
  1. 默认绑定 `127.0.0.1` 而非 `0.0.0.0`，需要外部访问时通过配置显式开启
  2. 首次启动时自动生成一个随机 API key 并写入配置文件，默认启用鉴权
  3. CORS 默认设为 `http://localhost:*` 而非 `*`
  4. 在启动日志中打印安全警告如果运行在无鉴权 + 0.0.0.0 模式
- **状态**: active（部分缓解：已有 `RELAY_API_KEY` 可选鉴权，但默认配置不安全）

---

### [P2] CLI 无发布形态 + doctor 假阳性

- **现象**: CLI 有 `doctor` 命令（`acp_cli::print_doctor` / `management::print_doctor`），但搜索发现 doctor 命令主要检查 ACP server 和 MCP 管理状态。代码中存在 `println!("northhing ACP doctor")` 等 输出。关于"假阳性"：doctor 命令检查的项目可能存在误报，如检查 ACP server 是否就绪但未检查实际连接。CLI 的发布形态方面，未发现独立的发布/打包配置（如 Homebrew formula、npm package、binary release）。
- **证据**:
  - `src/apps/cli/src/acp_cli.rs` — `print_doctor` 函数，检查 ACP readiness
  - `src/apps/cli/src/main.rs` — `Commands::Doctor` 和 `McpAction::Doctor` 两个 doctor 入口
  - `src/apps/cli/src/management.rs` — `print_doctor` 函数
  - `src/apps/cli/src/main.rs` 中 `doctor` 命令输出 `"northhing ACP doctor"` 等文本
  - 未发现 `.github/workflows` 中的 CLI binary release 配置
- **建议修法**:
  1. 统一两个 doctor 命令为一个，避免分裂
  2. doctor 检查项增加 false-positive 防护：实际连接测试而非仅检查进程存在
  3. 添加 CLI binary 的 CI 发布配置（GitHub Release + cargo install 支持）
- **状态**: active

---

### [P2] 两个 app 实例互踩配置

- **现象**: 搜索 `single instance` / `lock file` / `already running` / `prevent multiple` 在桌面端代码中无匹配。没有单实例锁机制。如果用户同时启动两个桌面实例，它们会共享同一个 `~/.northhing/config/app.json`，后写入的覆盖先写入的配置。两个实例的 session 状态也可能冲突。
- **证据**:
  - 搜索 `single.*instance|lock.*file|already.*running|prevent.*multiple|instance.*check` 在 `src/apps/desktop/` 中无结果
  - `settings.rs:655-667` 的 `save_app_settings` 无文件锁（虽然 `persistence.rs` 有 `FILE_LOCKS`，但 `save_app_settings` 未使用它）
  - `src/crates/assembly/core/src/infrastructure/storage/persistence.rs:15-20` — 文件锁机制存在但未被 `save_app_settings` 使用
- **建议修法**:
  1. 在 `main.rs` 启动时创建 lock file（如 `~/.northhing/app.lock`），如果已存在则提示用户并退出
  2. 或使用 Tauri/Slint 的 single-instance 插件
  3. 让 `save_app_settings` 使用 `persistence.rs` 中的 file lock 机制
- **状态**: active

---

### [P2] 上下文紧急截断无可见标记

- **现象**: 上下文压缩（compression）逻辑存在于 `turn_tick.rs` 和 `compress_run.rs` 中。`ContextCompressionStarted` 和 `ContextCompressionCompleted` 事件已定义并发送。但这些事件是否在 UI 上呈现为可见标记？桌面端 `event_bridge.rs` 处理了多种 `AgenticEvent`，但搜索 `ContextCompression` 在 event_bridge 中未找到处理逻辑。CLI 端 `run.rs` 中也未发现对 `ContextCompression` 事件的显式渲染。压缩发生时用户看不到任何提示。
- **证据**:
  - `src/crates/assembly/core/src/agentic/execution/turn_tick.rs` — 触发压缩逻辑，emit `ContextCompressionStarted` 事件
  - `src/crates/assembly/core/src/agentic/execution/compress_run.rs:53-63` — 发送 `AgenticEvent::ContextCompressionStarted` 事件
  - `src/crates/contracts/events/src/agentic.rs` — 定义了 `ContextCompressionStarted` / `ContextCompressionCompleted` 事件
  - `src/apps/desktop/src/app_state/event_bridge.rs` — 未匹配 `ContextCompression` 事件（只处理 `DialogTurnStarted/Completed/Cancelled/Failed`）
  - `src/apps/cli/src/modes/chat/run.rs` — 未发现 `ContextCompression` 事件处理
- **建议修法**:
  1. 在桌面端 `event_bridge.rs` 中添加 `ContextCompressionStarted` / `Completed` 的处理，显示一个临时 UI 标记（如 "上下文已压缩" banner）
  2. 在 CLI 端显示压缩提示（如 `[context compressed: 12000 → 4000 tokens]`）
  3. 在消息列表中插入一个系统消息标记压缩发生的位置
- **状态**: active

---

### [P2] 快照/日志无清理任务

- **现象**: 存在 `CleanupService`（`cleanup.rs`）实现了快照、日志、缓存的清理逻辑，且 `SessionManager` 有 `spawn_cleanup_task` 和 `spawn_auto_save_task`。但 `CleanupService.cleanup_all()` 需要被显式调用 — 没有找到自动调度它的逻辑（没有 cron job 或 periodic task 调用 `cleanup_all`）。`spawn_cleanup_task` 清理的是 expired sessions，不是快照/日志文件。
- **证据**:
  - `src/crates/assembly/core/src/infrastructure/storage/cleanup.rs:54-76` — `CleanupService` 完整实现，有 `cleanup_all` / `cleanup_temp_files` / `cleanup_old_logs` / `cleanup_oversized_cache`
  - `src/crates/assembly/core/src/agentic/session/session_manager_auto_save_cleanup.rs:135-170` — `spawn_auto_save_task` 和 `spawn_cleanup_task`，清理 expired sessions（非文件）
  - `src/crates/assembly/core/src/service/snapshot/snapshot_system.rs:446` — `cleanup_orphaned_snapshots` 方法存在，但无自动调度
  - 搜索 `cleanup.*schedule|periodic.*cleanup|cron.*cleanup|cleanup.*loop` 无匹配
  - `CleanupService` 虽然定义了 `auto_cleanup_enabled: true` 默认值，但没有找到任何代码实际创建 `CleanupService` 实例并调用 `cleanup_all()`
- **建议修法**:
  1. 在 app 启动时创建 `CleanupService` 并 spawn 一个 periodic task（如每 24h 调用一次 `cleanup_all()`）
  2. 或在 session 删除时触发相关快照的清理
  3. 将 `cleanup_orphaned_snapshots` 纳入 `CleanupService` 的清理范围
- **状态**: active（基础设施已就位，但缺少调度触发）

---

### [P2] 失败 turn 无历史解释

- **现象**: `DialogTurnFailed` 事件已定义并包含 `error` / `error_category` / `error_detail` 字段。桌面端 `event_bridge.rs` 处理了该事件，设置错误消息并显示。CLI 端 `run.rs` 也处理了 `DialogTurnFailed`。但失败信息仅作为临时错误显示，没有被持久化到对话历史中 — 用户刷新消息列表后看不到之前 turn 失败的记录。
- **证据**:
  - `src/crates/contracts/events/src/agentic.rs:159-166` — `DialogTurnFailed` 事件定义，包含 error/category/detail
  - `src/apps/desktop/src/app_state/event_bridge.rs:222-260` — 处理 `DialogTurnFailed`，设置 `set_session_error` + `set_inline_error`，但未写入消息列表
  - `src/apps/cli/src/modes/chat/run.rs` — 处理 `DialogTurnFailed`，显示错误但未持久化
  - `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs` — 持久化 completed/cancelled/failed turn，但 failed turn 的持久化仅记录 turn 元数据，不在消息列表中显示失败原因
- **建议修法**:
  1. 在 `DialogTurnFailed` 处理中，将失败原因作为一条系统消息插入对话历史
  2. 或在消息列表中为失败的 assistant 消息添加 error 标记和错误详情
  3. CLI 端在对话历史渲染时显示 `[失败] {error}` 条目
- **状态**: active

---

### [P2] 排队消息失败静默丢弃

- **现象**: 事件队列 `EventQueue` 在队列满时（`max_queue_size: 10000`）会静默丢弃新事件，仅打印 warn 日志。`StreamEventSink::enqueue` 的实现甚至丢弃了 `enqueue` 的返回值（`let _ = EventQueue::enqueue(self, event, priority).await;`），调用方无法知道事件是否被丢弃。虽然这是事件队列而非消息队列的行为，但如果 `DialogTurnFailed` 等关键事件被丢弃，用户将完全看不到错误。
- **证据**:
  - `src/crates/assembly/core/src/agentic/events/queue.rs:85` — `warn!("Event queue full, dropping event: event_id={}", event_id);` 然后返回 `Ok(event_id)`（假成功）
  - `src/crates/assembly/core/src/agentic/events/queue.rs:127` — `StreamEventSink` impl: `let _ = EventQueue::enqueue(self, event, priority).await;` — 完全忽略结果
  - `src/crates/assembly/core/src/agentic/events/queue.rs:24` — `max_queue_size: 10000` 默认值
  - 关于消息排队：由于桌面端目前没有实现消息排队（见 P0 #1），"排队消息失败丢弃"的问题主要表现在事件层面。如果未来实现消息排队，需要确保排队失败时有用户可见的反馈。
- **建议修法**:
  1. `enqueue` 在队列满时返回 `Err` 而非 `Ok`，让调用方决定重试或通知用户
  2. 对 `Critical` 优先级的事件不允许丢弃，强制入队或阻塞
  3. `StreamEventSink::enqueue` 应处理 `Err` 并 log error 级别日志
  4. 未来实现消息排队时，确保排队失败的消息有持久化降级路径（如写入磁盘待重发）
- **状态**: active

---

## 汇总

| # | 级别 | 标题 | 状态 |
|---|------|------|------|
| 1 | P0 | 桌面消息排队问题 | active |
| 2 | P0 | 挂死三件套（AskUserQuestion 无超时 + 工具无 cancel + turn 无总超时） | active |
| 3 | P1 | 配置非原子写 | active (Phase 5 待修) |
| 4 | P1 | API key 明文存储 | active |
| 5 | P1 | Delete 不走回收站 | active |
| 6 | P1 | 移动端重配对无引导 + i18n 乱码 | active |
| 7 | P1 | relay 默认 0.0.0.0 无鉴权 | active (部分缓解) |
| 8 | P2 | CLI 无发布形态 + doctor 假阳性 | active |
| 9 | P2 | 两个 app 实例互踩配置 | active |
| 10 | P2 | 上下文紧急截断无可见标记 | active |
| 11 | P2 | 快照/日志无清理任务 | active (基础设施已就位，缺调度) |
| 12 | P2 | 失败 turn 无历史解释 | active |
| 13 | P2 | 排队消息失败静默丢弃 | active (事件层面) |

**所有 13 条技术债均为 active 状态，无 frozen/absent。**

### 关键发现

1. **P0 问题最严重**: 桌面端无消息排队 + 无超时挂死风险，这两条直接影响用户体验和系统可靠性
2. **安全相关**: API key 明文 + relay 无鉴权是两个安全问题，建议优先处理
3. **i18n 乱码广泛**: 桌面端 Rust 代码中大量中文字符串存在 GBK/UTF-8 编码损坏，影响所有中文用户
4. **基础设施已有但未接入**: CleanupService 和 file lock 机制已实现但未被实际使用，接入成本低
5. **事件系统隐患**: 事件队列满时静默丢弃 + `StreamEventSink` 忽略错误，可能导致关键事件丢失
