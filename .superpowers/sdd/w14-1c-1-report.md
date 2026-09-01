# W14-1c-1 实施报告 — A 类「未初始化断言」测试迁移

> 任务：W14-1c-1（A 类 5 个测试各迁一个 `tests/*.rs` 独立文件）
> 基线：`e151b54`
> 状态：**DONE**

---

## 1. 迁移清单

| # | 测试名称 | 原位置 | 新文件位置 | 隔离形式 | 守卫改造 |
|---|---|---|---|---|---|
| 1 | `test_ensure_room_session_fails_cleanly_when_uninitialized` | `src/apps/desktop/src/ui_dioxus/api.rs:170` | `src/apps/desktop/tests/desktop_uninit_a.rs` | 独立集成测试进程 (`#[tokio::test]`) | 无 |
| 2 | `test_api_functions_fail_cleanly_before_init` | `src/apps/desktop/src/ui_dioxus/api_settings.rs:198` | `src/apps/desktop/tests/desktop_uninit_b.rs` | 独立集成测试进程 (`#[tokio::test]`) | 逐字搬迁全部 12+ API 调用与 DTO 构造 |
| 3 | `test_result_methods_return_error_before_init` | `src/crates/assembly/core/src/kernel_facade/tests.rs:381` | `src/crates/assembly/core/tests/kernel_facade_uninit.rs` | 独立集成测试进程 (`#[test]`) | 无 |
| 4 | `e2e_storage_guard_rejects_missing_isolated_roots` | `src/crates/assembly/core/src/infrastructure/app_paths/path_manager.rs:237` | `src/crates/assembly/core/tests/path_manager_uninit.rs` | 独立集成测试进程 (`#[test]`) | 独立包含精简版 `EnvVarGuard` RAII，去除单进程内多测竞争专用的 `ENV_LOCK` |
| 5 | `test_session_manager_not_initialized` | `src/crates/services/terminal/src/session/singleton.rs:92` | `src/crates/services/terminal/tests/terminal_singleton_uninit.rs` | 独立集成测试进程 (`#[tokio::test]`) | 去除原 `if !is_session_manager_initialized()` 软跳过，改硬断言 `assert!(session_manager().is_none())` |

---

## 2. 复用侦察（Reconnaissance & Reuse）

1. **`PathManager` 与 `EnvVarGuard`**：
   - 侦察符号：`PathManager` 在 `northhing_core::infrastructure::PathManager` 已经完全公开 (`pub use app_paths::{...}`)，无需修改可见性。
   - `EnvVarGuard` 重建理由：原 `path_manager.rs:tests` 内部的 `EnvVarGuard` 和 `ENV_LOCK` 属于私有测试辅助类型；在独占进程的独立集成测试文件 `path_manager_uninit.rs` 中，不需要多线程互斥锁 `ENV_LOCK`，只需精简版 `EnvVarGuard` RAII 在退出时还原环境变量，避免泄露至测试执行器。
2. **`desktop` 库可见性**：
   - `src/apps/desktop/src/lib.rs` 声明了 `pub mod ui_dioxus;` 与 `pub mod app_state;`。
   - `ui_dioxus/mod.rs` 将 `api` 和 `api_settings` 标记为 `pub mod`，使集成测试 `desktop_uninit_a.rs` 和 `desktop_uninit_b.rs` 可直接调用 API。`MockKeyring` 在 `northhing::app_state::settings::MockKeyring` 已对外可用。
3. **`terminal-core` 单例查询**：
   - `terminal_core::session::session_manager()` 已经是公开符号，直接调用，无需测试 seam。

---

## 3. 遇到的编译错误与修复层次

1. **`error[E0603]: module api is private` (Desktop 层)**：
   - 现象：在集成测试编译 `desktop_uninit_a.rs` 时，`northhing::ui_dioxus::api` 报私有模块错误。
   - 原因：`ui_dioxus/mod.rs` 之前仅声明 `mod api; mod api_settings;`。
   - 修复层：`src/apps/desktop/src/ui_dioxus/mod.rs`（Layer 1: Interfaces and entrypoints），提升为 `pub mod api;` 与 `pub mod api_settings;`。
2. **`error[E0624]: method coordinator is private` (Core 层)**：
   - 现象：在集成测试编译 `kernel_facade_uninit.rs` 时，`facade.coordinator()` 报私有方法错误。
   - 原因：`KernelFacade::coordinator` 原为 `pub(super) fn coordinator`。
   - 修复层：`src/crates/assembly/core/src/kernel_facade/mod.rs`（Layer 2: Product assembly），将 `coordinator` 方法提升为 `pub fn coordinator`。

---

## 4. 测试计数对比表

| Crate | 迁移前 Unit Tests (`--lib`) | 迁移后 Unit Tests (`--lib`) | 迁移后集成测试 (`tests/*.rs`) | 总测试数变化 | 状态 |
|---|---|---|---|---|---|
| `northhing` (desktop) | 147 passed | 145 passed | 2 passed (`desktop_uninit_a`, `desktop_uninit_b`) | 147 → 147 | 不变（零丢失） |
| `northhing-core` | 1071 passed, 1 ignored | 1069 passed, 1 ignored | 2 passed (`kernel_facade_uninit`, `path_manager_uninit`) | 1071 → 1071 (+2 integration) | 不变（零丢失） |
| `terminal-core` | 22 passed | 21 passed | 1 passed (`terminal_singleton_uninit`) | 22 → 22 | 不变（零丢失） |

---

## 5. 验证命令与输出原文

### 5.1 `cargo check --workspace`
- 命令：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check --workspace`
- 结果：`Finished dev profile [unoptimized + debuginfo] target(s) in 2m 00s`，0 error。

### 5.2 `cargo check -p northhing`
- 命令：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check -p northhing`
- 结果：`Finished dev profile [unoptimized + debuginfo] target(s) in 30.53s`，0 error。

### 5.3 `cargo test -p northhing --test desktop_uninit_a --test desktop_uninit_b`
- 命令：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing --test desktop_uninit_a --test desktop_uninit_b`
- 输出：
```text
     Running tests\desktop_uninit_a.rs (target\debug\deps\desktop_uninit_a-c0b78921f21e3d2b.exe)

running 1 test
test test_ensure_room_session_fails_cleanly_when_uninitialized ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\desktop_uninit_b.rs (target\debug\deps\desktop_uninit_b-a081f97b09a5e048.exe)

running 1 test
test test_api_functions_fail_cleanly_before_init ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### 5.4 `cargo test -p northhing-core --features product-full --test kernel_facade_uninit --test path_manager_uninit`
- 命令：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full --test kernel_facade_uninit --test path_manager_uninit`
- 输出：
```text
     Running tests\kernel_facade_uninit.rs (target\debug\deps\kernel_facade_uninit-c50d3d3677515616.exe)

running 1 test
test test_result_methods_return_error_before_init ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\path_manager_uninit.rs (target\debug\deps\path_manager_uninit-702fed9407a46f29.exe)

running 1 test
test e2e_storage_guard_rejects_missing_isolated_roots ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 5.5 `cargo test -p terminal-core --test terminal_singleton_uninit`
- 命令：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p terminal-core --test terminal_singleton_uninit`
- 输出：
```text
     Running tests\terminal_singleton_uninit.rs (target\debug\deps\terminal_singleton_uninit-5eb42efd9b2cdc7d.exe)

running 1 test
test test_session_manager_not_initialized ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 5.6 串行对照验证 (`-- --test-threads=1`)
1. `cargo test -p northhing --test desktop_uninit_a --test desktop_uninit_b -- --test-threads=1`:
   - `desktop_uninit_a`: 1 passed (0.00s)
   - `desktop_uninit_b`: 1 passed (0.00s)
2. `cargo test -p northhing-core --features product-full --test kernel_facade_uninit --test path_manager_uninit -- --test-threads=1`:
   - `kernel_facade_uninit`: 1 passed (0.00s)
   - `path_manager_uninit`: 1 passed (0.00s)
3. `cargo test -p terminal-core --test terminal_singleton_uninit -- --test-threads=1`:
   - `terminal_singleton_uninit`: 1 passed (0.00s)

### 5.7 `pnpm run check:rot`
- 命令：`pnpm run check:rot`
- 输出：
```text
Rot budget verification passed (5 grep rules [unwrap_production=477/502, expect_production=939/1089, let_underscore=371/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=17/400], 6 god-file rules checked across 1365 files).
```

---

## 6. 状态

**DONE**
