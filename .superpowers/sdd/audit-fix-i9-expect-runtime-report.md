# Audit I9 Fix Report: callbacks_lifecycle.rs 8 处 expect() 建 runtime 换 match+banner 范式

## 1. 实现内容

Task I9 修复完成。将 `src/apps/desktop/src/app_state/callbacks_lifecycle.rs` 中 8 处在 `std::thread::spawn` 内部直接调用 `.expect("failed to build tokio runtime...")` 导致 panic 的代码，全部替换为 `match build() { Ok(rt) => Some(rt), Err(e) => { error!(...); set_session_error(...); None } }` 范式。新增统一的 helper 函数 `build_ui_callback_runtime`，并移除一处历史遗留的重复误置注释。改后文件总行数为 1009 行（ceiling 预算 ≤ 1011）。

## 2. 复用侦察

- **`set_session_error` 签名与调用约定**：
  定义于 `src/apps/desktop/src/app_state/error_banners.rs` Line 26：
  `pub fn set_session_error(ui_weak: slint::Weak<AppWindow>, message: impl Into<String>)`
  该函数接收 `slint::Weak<AppWindow>` 与字符串消息，在 Slint 事件循环线程上安全更新 `ui.set_session_error` 并调度 5 秒自动清除。
- **8 个站点捕获的 UI Weak 变量名核对**：
  - Site 1 (`:294` / 原 `:297`): `ui_clone` (`let ui_clone = ui_weak3.clone();`)
  - Site 2 (`:390` / 原 `:394`): `ui_weak_msg` (`let ui_weak_msg = ui.as_weak();`)
  - Site 3 (`:432` / 原 `:437`): `ui_clone` (`let ui_clone = ui_weak5.clone();`)
  - Site 4 (`:537` / 原 `:543`): `ui_clone` (`let ui_clone = ui_weak7.clone();`)
  - Site 5 (`:639` / 原 `:646`): `ui_clone` (`let ui_clone = ui_weak8.clone();`)
  - Site 6 (`:714` / 原 `:722`): `ui_clone_for_refresh` (`let ui_clone_for_refresh = ui_clone.clone();`)
  - Site 7 (`:743` / 原 `:752`): `ui_clone_for_refresh` (`let ui_clone_for_refresh = ui_clone.clone();`)
  - Site 8 (`:821` / 原 `:831`): `ui_clone` (`let ui_clone = ui_weak_stop.clone();`)

## 3. 8 站点转换前后行号与 Action 对照表

| 站点 # | Action 标识 | 转换前行号 | 转换后行号 | Captured UI Weak 变量 | 转换代码 |
|---|---|---|---|---|---|
| 1 | `"new-session"` | :294-297 | :294-296 | `ui_clone` | `let Some(rt) = build_ui_callback_runtime(&ui_clone, "new-session") else { return };` |
| 2 | `"switch-session"` | :391-394 | :390-392 | `ui_weak_msg` | `let Some(rt) = build_ui_callback_runtime(&ui_weak_msg, "switch-session") else { return };` |
| 3 | `"delete-session"` | :434-437 | :432-434 | `ui_clone` | `let Some(rt) = build_ui_callback_runtime(&ui_clone, "delete-session") else { return };` |
| 4 | `"toggle-skill"` | :540-543 | :537-539 | `ui_clone` | `let Some(rt) = build_ui_callback_runtime(&ui_clone, "toggle-skill") else { return };` |
| 5 | `"load-more-messages"` | :643-646 | :639-641 | `ui_clone` | `let Some(rt) = build_ui_callback_runtime(&ui_clone, "load-more-messages") else { return };` |
| 6 | `"refresh-sessions"` | :719-722 | :714-716 | `ui_clone_for_refresh` | `let Some(rt) = build_ui_callback_runtime(&ui_clone_for_refresh, "refresh-sessions") else { return };` |
| 7 | `"refresh-messages"` | :749-752 | :743-745 | `ui_clone_for_refresh` | `let Some(rt) = build_ui_callback_runtime(&ui_clone_for_refresh, "refresh-messages") else { return };` |
| 8 | `"stop-streaming"` | :828-831 | :821-823 | `ui_clone` | `let Some(rt) = build_ui_callback_runtime(&ui_clone, "stop-streaming") else { return };` |

*注：`:866-874`（`export-markdown`）保留特化的 `导出失败: {e}` banner，不在 8 站点转换范围内。*

## 4. 编译错误追溯记录

| 错误代码 | 错误现象 | 归属层 | 解决方案 |
|---|---|---|---|
| E0599 | `get_global_config`, `get_skill` 等方法在 `KernelFacade` 上未找到 | 机制层 (Trait In-Scope) | 恢复被误删的 `use northhing_kernel_api::{KernelAgentsApi, KernelSettingsApi};` 导入 |
| E0425 | `sid_str` 未找到 | 机制层 (Variable Scope) | 修正 `on_toggle_skill` 闭包中的日志字段 `skill_name_str` |

## 5. 验证输出原文

### 命令 1: `pnpm run fmt:rs`
```
> northhing@0.2.10 fmt:rs E:\agent-project\northing
> node scripts/format-changed-rust.mjs

[format-changed-rust] Formatting 1 Rust file(s).
```

### 命令 2: `cargo check -p northhing`
```
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 48.52s
```

### 命令 3: `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib callbacks_lifecycle`
```
   Compiling northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 23.75s
     Running unittests src\lib.rs (target\debug\deps\northhing-4a70ae8bdb5acd3a.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 156 filtered out; finished in 0.00s
```

### 命令 4: 行数统计
```powershell
pwsh -Command "(Get-Content src\apps\desktop\src\app_state\callbacks_lifecycle.rs).Count"
1009
```

## 6. 文件清单与自审结论

- 涉及文件：`src/apps/desktop/src/app_state/callbacks_lifecycle.rs`
- Commit：`a8a0b70` (`fix(desktop): replace expect runtime builds with match+banner in callbacks_lifecycle.rs`)
- 自审发现：
  1. 8 处 expect 全部被替换为 `build_ui_callback_runtime` 且不再触发 panic；
  2. 运行时创建失败时写入日志 `"{action}: failed to build runtime: {e}"` 并设置 UI banner `"内部错误：无法启动运行时"`；
  3. 清理了 `:844` 上方重复错置的 8 行 rename-session 注释，保持格式整洁；
  4. 最终行数 1009 行，严格低于 ceiling 1011 行预算；
  5. `export-markdown` 范式块未被破坏。

## 7. 神级文件健康度观察

- **健康度评估**：持平且微幅更清晰。
- **依据**：文件总行数从 1011 降至 1009 行，消除了 8 处可能导致线程 panic 的致命坑点，收敛为统一的 `build_ui_callback_runtime` 错误处理 helper，降低了回调处理代码的隐患与重复度。
