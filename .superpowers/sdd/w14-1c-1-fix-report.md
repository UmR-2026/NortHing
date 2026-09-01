# W14-1c-1 Fix-forward 报告 — 标记测试专用可见性为 doc(hidden)

> 任务：W14-1c-1 验收结论 fix-forward（三处为集成测试暴露的可见性添加 `#[doc(hidden)]` 注解）
> Commit：`9cd72f4` (`docs: mark W14-1c-1 test-only visibility as doc(hidden)`)
> 状态：**DONE**

---

## 1. 改动清单

1. `src/apps/desktop/src/ui_dioxus/mod.rs`：
   - 在 `pub mod api;` 上方添加 `#[doc(hidden)] // 为 W14-1c-1 集成测试暴露；非公共 API，勿在桌面 UI 之外依赖`
   - 在 `pub mod api_settings;` 上方添加 `#[doc(hidden)] // 为 W14-1c-1 集成测试暴露；非公共 API，勿在桌面 UI 之外依赖`
2. `src/crates/assembly/core/src/kernel_facade/mod.rs`：
   - 在 `pub fn coordinator(` 上方添加 `#[doc(hidden)] // 为 W14-1c-1 集成测试暴露；非公共 API`

---

## 2. 验证命令与输出原文

### 2.1 `cargo check -p northhing`
- 命令：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check -p northhing
```
- 输出：
```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 23.95s
```
- 结果：0 error。

### 2.2 `cargo test -p northhing --test desktop_uninit_a`
- 命令：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing --test desktop_uninit_a
```
- 输出：
```text
   Compiling northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
   Compiling northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 35.94s
     Running tests\desktop_uninit_a.rs (target\debug\deps\desktop_uninit_a-c0b78921f21e3d2b.exe)

running 1 test
test test_ensure_room_session_fails_cleanly_when_uninitialized ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
- 结果：1 passed。

---

## 3. Git 状态

```text
commit 9cd72f4f7c5966f494a66ca2ef8ced44ab3ac96f
Author: Mavis <mavis@northhing.local>
Date:   Wed Sep 2 02:52:09 2026 +0800

    docs: mark W14-1c-1 test-only visibility as doc(hidden)

 2 files changed, 3 insertions(+)
```

---

## 4. 状态

**DONE**
