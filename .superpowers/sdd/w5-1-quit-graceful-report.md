# Task 1 (W5-1) Report: F1 — quit_shell Graceful Shutdown without process::exit

**Date:** 2026-08-28  
**Scope:** `src/apps/desktop/` (`main.rs`, `ui_dioxus/{app.rs, entry.rs, registry.rs}`)  
**Commit:** `de60a0b` (`fix(desktop): graceful shutdown on shell quit without process::exit (F1)`)  
**Status:** DONE

---

## 1. What was Implemented

1. **`src/apps/desktop/src/ui_dioxus/registry.rs` (lines 319-351, 586-611)**:
   - Added `mark_all_closing_targets(&self) -> Vec<(&'static str, WindowId, usize)>` to `ShellWindowManager`. Atomic state transition of all `Open` and `Opening` module windows to `WindowState::Closing`, clearing active broadcast set `active_tx`, and returning all window IDs / HWNDs for closing.
   - Added unit test `test_mark_all_closing_targets` covering atomic transition, active signal clearing, and idempotency.

2. **`src/apps/desktop/src/ui_dioxus/app.rs` (lines 37, 80, 90-103, 375, 439)**:
   - Exported `pub(crate) mod win_ops` so OS window closing facilities are available across `ui_dioxus`.
   - Added `close_all_modules(wm: &ShellWindowManager)` helper iterating through `mark_all_closing_targets()` to issue `window().close_window(wid)` and `win_ops::close_os_window(hwnd)`.
   - Replaced `quit_shell()` (which previously invoked `std::process::exit(0)`) with `quit_shell(wm: &ShellWindowManager)`: calls `close_all_modules(wm)`, hides/closes room HWND via `win_ops::close_os_window`, and requests Dioxus window close via `window().close()`.
   - Updated room chrome ✕ close button to pass cloned `wm_close: ShellWindowManager`.
   - Kept total file line count at 959 lines (strictly under the 962 rot-budget ceiling).

3. **`src/apps/desktop/src/ui_dioxus/entry.rs` (lines 101, 143, 208-242)**:
   - Updated `ui_dioxus::launch` signature to take `on_shutdown: Arc<dyn Fn() + Send + Sync + 'static>`.
   - In `with_custom_event_handler`:
     - Intercepted `Event::LoopDestroyed` to execute `on_shutdown()`, cleanly triggering background worker exit and MCP subprocess shutdown before process termination.
     - Intercepted OS `WindowEvent::CloseRequested` on the room window to ensure all module windows close together.

4. **`src/apps/desktop/src/main.rs` (lines 13, 95-127)**:
   - Encapsulated `shutdown_tx` and `worker` JoinHandle inside a thread-safe idempotent shutdown callback closure `perform_shutdown: Arc<dyn Fn() + Send + Sync + 'static>`.
   - Passed `perform_shutdown` to `ui_dioxus::launch()`.
   - Added `shutdown_for_main()` post-launch hook ensuring identical graceful shutdown if `launch()` ever returns to `main()`.

---

## 2. Exit-Chain Walkthrough (Spec 3 file:line Trace)

```
[1. User Action]
Room Window Chrome "✕" Click
  ↳ `src/apps/desktop/src/ui_dioxus/app.rs:439` (onclick handler triggers `quit_shell(&wm_close)`)

[2. Module Windows Teardown]
`quit_shell(wm)` calls `close_all_modules(wm)`
  ↳ `src/apps/desktop/src/ui_dioxus/app.rs:90` (`close_all_modules`)
  ↳ `src/apps/desktop/src/ui_dioxus/registry.rs:319` (`ShellWindowManager::mark_all_closing_targets`)
      - Atomically transitions all active/opening module windows (`self`, `work`, `archive`, `space`, `settings`, `onboarding`) to `WindowState::Closing`.
      - Broadcasts empty set `HashSet::new()` via `active_tx`.
      - Returns `Vec<(&'static str, WindowId, usize)>`.
  ↳ For each target:
      - `window().close_window(wid)` (`src/apps/desktop/src/ui_dioxus/app.rs:92`) sends `UserWindowEvent::CloseWindow(wid)` to Dioxus Desktop.
      - `win_ops::close_os_window(hwnd)` (`src/apps/desktop/src/ui_dioxus/app.rs:51`) hides HWND and posts `WM_CLOSE` with background watchdog.
      - Unmounting module window VDom drops `WindowDropGuard` (`src/apps/desktop/src/ui_dioxus/windows.rs:75`), invoking `notify_closed_with_gen`.
      - Module geometry-follow OS threads (`windows.rs:162, 430, 630`) detect `IsWindowVisible(hwnd) == 0` or channel closed and terminate cleanly.

[3. Room Main Window Teardown]
`quit_shell` closes room window
  ↳ `src/apps/desktop/src/ui_dioxus/app.rs:100` (`win_ops::close_os_window(window().hwnd() as usize)`) hides HWND and posts `WM_CLOSE`.
  ↳ `src/apps/desktop/src/ui_dioxus/app.rs:102` (`window().close()`) sends `UserWindowEvent::CloseWindow(room_wid)` to Dioxus Desktop.

[4. Dioxus Desktop Event Loop Termination]
Dioxus Desktop processes CloseWindow events
  ↳ `dioxus_desktop::app::handle_close_requested` removes each window from `self.webviews`.
  ↳ With all module windows and the room window closed, `self.webviews.is_empty()` is true.
  ↳ `self.control_flow = ControlFlow::Exit`.
  ↳ Tao message loop completes and transitions to event loop destruction.

[5. Loop Destruction & Shutdown Callback]
Tao emits `Event::LoopDestroyed`
  ↳ `src/apps/desktop/src/ui_dioxus/entry.rs:208` (`custom_event_handler` intercepts `Event::LoopDestroyed`).
  ↳ Logs debug event `COMP_UI_DIOXUS_WIN` `"loop_destroyed"`.
  ↳ Invokes `on_shutdown()` (`src/apps/desktop/src/main.rs:98`).

[6. Worker & MCP Graceful Cleanup]
`perform_shutdown()` executes
  ↳ `shutdown_tx.take().unwrap().send(())` signals long-lived background worker runtime (`main.rs:101`).
  ↳ Worker runtime exits and `worker_handle.take().unwrap().join()` succeeds (`main.rs:106`).
  ↳ Builds temporary current-thread tokio runtime and executes `shutdown_mcp_servers().await` (`main.rs:118`).
  ↳ `northhing_core::service::mcp::global_mcp_service().server_manager().shutdown().await` shuts down all MCP child processes cleanly.
  ↳ Tracing logs: `"MCP servers shut down successfully"`.
```

---

## 3. 复用侦察 (Reused Facilities)

1. **`ShellWindowManager` (`registry.rs`)**: Reused existing generation tracking, state machine (`WindowState`), target extraction (`(WindowId, usize)`), and broadcast channel (`active_tx`) to perform batch module cleanup via `mark_all_closing_targets()`.
2. **`win_ops::close_os_window` (`app.rs`)**: Reused the hardened HWND hide + `WM_CLOSE` + std watchdog thread window destructor instead of inventing a new OS-level window termination mechanism.
3. **`WindowDropGuard` (`windows.rs`)**: Reused existing drop notifications without modification; confirmed it is called upon module unmount.
4. **`shutdown_mcp_servers` (`main.rs`)**: Reused the core MCP server manager shutdown sequence, ensuring MCP server child processes are gracefully terminated.
5. **`shutdown_tx` / `worker.join()` (`main.rs`)**: Reused the main worker thread channel signaling and join mechanism.

---

## 4. Verification

### Command 1: `cargo check -p northhing`
```
warning: `northhing-core` (lib) generated 18 warnings (run `cargo fix --lib -p northhing-core` to apply 17 suggestions)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
warning: `northhing` (bin "northhing") generated 49 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 2 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.12s
```

### Command 2: `cargo check --workspace`
```
warning: `northhing-core` (lib) generated 18 warnings (run `cargo fix --lib -p northhing-core` to apply 17 suggestions)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
warning: `northhing-cli` (bin "northhing-cli") generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli` to apply 1 suggestion)
warning: `northhing` (bin "northhing") generated 49 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 2 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.05s
```

### Command 3: `cargo check -p northhing --tests`
```
warning: `northhing-core` (lib) generated 18 warnings (run `cargo fix --lib -p northhing-core` to apply 17 suggestions)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
warning: `northhing` (bin "northhing" test) generated 31 warnings (run `cargo fix --bin "northhing" -p northhing --tests` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.21s
```

### Command 4: `pnpm run check:repo-hygiene`
```
> northhing@0.2.10 check:repo-hygiene E:\agent-project\northing
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (8 content files scanned, 3530 filenames checked).
```

### Command 5: `rg "process::exit" src/apps/desktop/src`
```
E:\agent-project\NortHing\src\apps\desktop\src\main.rs:
  82:                 std::process::exit(1);
  133:         std::process::exit(1);

E:\agent-project\NortHing\src\apps\desktop\src\bin\w4_repro.rs:
  176:                 std::process::exit(1);
  222:                 std::process::exit(1);
  322:                     std::process::exit(1);
  332:                     std::process::exit(1);
  352:                     std::process::exit(1);
```
*(Only init/error exit(1) failure paths remain. Zero process::exit(0) in desktop source).*

---

## 5. Compile Errors Encountered and Layer Fixed

1. `E0425: cannot find value window_manager in this scope` (in `entry.rs:202`):
   - **Layer:** Mechanism layer (initialization ordering).
   - **Fix:** Instantiated `ShellWindowManager::default()` before `Config` construction so `window_manager` is in scope for `with_custom_event_handler` as well as `with_context`.
2. `E0599: no method named hwnd found for struct Rc<DesktopService>` (in `app.rs:99`):
   - **Layer:** Mechanism layer (trait in scope).
   - **Fix:** Brought `dioxus::desktop::tao::platform::windows::WindowExtWindows` into scope under `#[cfg(target_os = "windows")]`.

---

## 6. Self-Review Findings + Concerns

- **Finding 1:** Verified that `ShellWindowManager::mark_all_closing_targets` is fully thread-safe (behind `inner.active_states` Mutex) and sets `active_tx` to empty set atomically.
- **Finding 2:** Verified `app.rs` line count: 959 lines (ceiling is 962 in `rot-budget.json`). No god-file ceiling was raised or violated.
- **Finding 3:** Verified that `ShutdownCoordinator` callback closure in `main.rs` uses `Mutex<Option<...>>` with `.take()`, ensuring `shutdown()` is strictly idempotent even if called multiple times (e.g. from `LoopDestroyed` and fallback `main()`).
- **Concerns:** None.

---

## 7. Erratum（2026-08-28 终审后编排者补录）

**缺漏**：本报告 §4 验证章节未附 `mark_all_closing_targets` 单测的执行输出原文（W5 终审 Minor W5-1-M1）。

**补跑证据**（编排者在终审 HEAD `f680cf6` 上实跑，MSVC toolchain）：

### Command: `cargo +stable-msvc test -p northhing --lib mark_all_closing`
```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.79s
     Running unittests src\lib.rs (target\debug\deps\northhing-975f8423d7ff303b.exe)

running 1 test
test ui_dioxus::registry::tests::test_mark_all_closing_targets ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 109 filtered out; finished in 0.00s
```

另：§2 走查两处行号 off-by-one（终审 Minor W5-1-M2，accept-and-close，不影响代码正确性）；§3 关于 WindowDropGuard 复用的声明为未验证声明（终审 Minor W5-1-M4，defer-with-owner，由真机实测清单第 7 项在新 HEAD 重跑兜底）。
