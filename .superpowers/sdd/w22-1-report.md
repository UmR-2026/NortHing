# W22-1 Report — P2-3 窄义版：ContextCompression 事件可见化（live 提示）

## 改动摘要

1. **Kernel 桥映射与单元测试** (`src/crates/assembly/core/src/kernel_facade/events.rs`)
   - 在 `agentic_event_to_dtos` 中为三类压缩事件增加映射，复用既有 `KernelEventDto::Banner { level, message }`（变体钉死，未新增变体）：
     - `AgenticEvent::ContextCompressionStarted` -> `Banner { level: BannerLevel::Info, message: "Context compression started" }`
     - `AgenticEvent::ContextCompressionCompleted` -> `Banner { level: BannerLevel::Info, message: "Context compressed: {tokens_before} → {tokens_after} tokens" }`
     - `AgenticEvent::ContextCompressionFailed` -> `Banner { level: BannerLevel::Error, message: "Context compression failed: {error}" }`
   - 在同文件底部新增单元测试 `test_agentic_event_to_dtos_context_compression_banners`，覆盖三事件向 Banner DTO 的转换断言。

2. **Desktop Banner** (`src/apps/desktop/src/ui_dioxus/app.rs`)
   - 引入 `banner: Signal<Option<String>>` 信号，在事件环中响应 `KernelEventDto::Banner`。
   - **streaming 门控**：仅在 `*streaming.read()` 为 true 时写入 `banner` signal，并在 UI 渲染层门控 `if streaming()`；在 `TurnStateKind::Completed`、`Failed`、`Cancelled` 以及 `submit_action` / `stop_action` 时均重置 `banner.set(None)`，防止跨会话后台压缩噪声横幅干扰前台。
   - **Desktop 文件选择理由**：brief 中允许文件集包含 `app.rs` 与可选 `turn_banner.rs`。由于在 `app.rs` 内直接使用 `banner.set(...)` 及 `degraded-banner` CSS 样式类即可完全满足要求，无需拆分独立 helper，遵循最小代码修改原则（Ponytail ladder），故保持 `turn_banner.rs` 不动，仅修改 `app.rs`。

3. **CLI 提示输出** (`src/apps/cli/src/modes/chat/run.rs` & `src/apps/cli/src/modes/exec.rs`)
   - `chat/run.rs`：在事件环处理三类压缩事件，当属于当前 turn 时调用 `chat_view.set_status(...)` 更新 TUI 状态栏并置 `needs_redraw = true`。严格未做历史落痕与 system message 插入（恪守禁区）。
   - `exec.rs`：在事件环处理三类压缩事件，仅通过 `self.print_text` 打印输出（Started/Completed 走 `println!`，Failed 走 `eprintln!`）。严格不做 JSON emit（恪守窄义）。输出全英文。

4. **技术债台账翻转** (`docs/status/tech-debt-ledger.md`)
   - P2-3 条目 status 翻转为 `resolved (2026-09-09, W22-1: live 提示已通 — kernel 桥 Banner + desktop streaming banner + CLI 打印；历史落痕归 P2-5)`。

### 三事件 Banner 映射文案原文

- Started: `"Context compression started"` (level: `BannerLevel::Info`)
- Completed: `"Context compressed: {tokens_before} → {tokens_after} tokens"` (level: `BannerLevel::Info`)
- Failed: `"Context compression failed: {error}"` (level: `BannerLevel::Error`)

### Desktop streaming 门控说明

`KernelEventDto::Banner` 契约中不携带 `session_id`。为防止后台其它会话触发自动压缩时在前台活跃会话中误弹提示，`app.rs` 中：
1. 事件环中仅当 `*streaming.read() == true` 时才将 message 写入 `banner` 信号；
2. UI 渲染时 `if streaming() { if let Some(msg) = banner.read().as_ref() { div { class: "degraded-banner", "{msg}" } } }`；
3. 在每次 turn 启动、完成、失败或取消时均执行 `banner.set(None)` 及时复位。

### CLI 两种输出形态

- `chat/run.rs` (TUI 交互流)：通过 `chat_view.set_status(Some(...))` 实时更新在底部状态栏，不破坏终端全屏绘制，不插入消息流历史。
- `exec.rs` (命令行执行模式)：仅在 `self.print_text` 条件下直接打印（`println!("\n...")` / `eprintln!("\n...")`），不通过 `self.emit` 生成 JSON 事件。

---

## 验证

### 1. `cargo check --workspace`
- 命令：`C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check --workspace`
- 输出截选：
```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 31.09s
```
- exit code: 0

### 2. 单元测试（带 `--features product-full`）
- 命令：`C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full --lib test_agentic_event_to_dtos_context_compression_banners`
- 输出：
```text
running 1 test
test kernel_facade::events::tests::test_agentic_event_to_dtos_context_compression_banners ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1071 filtered out; finished in 0.00s
```
- exit code: 0

### 3. Desktop 编译闸 (`cargo check -p northhing`)
- 命令：`C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing`
- 输出截选：
```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 38.72s
```
- exit code: 0

### 4. CLI 编译覆盖 (`cargo check -p northhing-cli`)
- 命令：`C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing-cli`
- 输出截选：
```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 06s
```
- exit code: 0

### 5. 代码质量与边界验证
- `pnpm run fmt:rs`：Formatted 4 Rust files.
- `pnpm run check:repo-hygiene`：passed (5 content files scanned, 3885 filenames checked).
- `node scripts/check-core-boundaries.mjs`：Core boundary check passed.

---

## 状态

DONE
