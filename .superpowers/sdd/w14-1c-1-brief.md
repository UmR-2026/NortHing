# W14-1c-1 Brief — A 类「未初始化断言」测试迁移（5 个，一测试一文件）

> 来源：`.superpowers/sdd/w14-1b-arbitration.md` §2.1（A 类分派表）+ §5 附带条件 #2/#3/#9。仲裁已闭环，本 brief 是执行处方。
> BASE commit：`e151b54`（派发前 HEAD）。

## 1. 背景（为什么）

A 类 5 个测试断言「全局单例未初始化时必须失败」。它们与被测单例**同进程**跑在 module 测试里，只要排在任何会初始化全局状态的测试之后就必挂（O-1 误诊两周的根因）。仲裁裁定：**进程级隔离 = 每个测试独占一个 `tests/*.rs` 集成测试文件**（独立测试二进制 = 独立进程 = 单例从空开始）。不用 `--test-threads=1` 约定替代。

## 2. 编排者预检结论（直接采信，勿重复侦察）

| # | 测试 | 现位置 | 落点 | 关键事实（已磁盘核实） |
|---|---|---|---|---|
| 1 | `test_ensure_room_session_fails_cleanly_when_uninitialized` | `src/apps/desktop/src/ui_dioxus/api.rs:170`（`#[tokio::test]`，在 `mod tests`） | `src/apps/desktop/tests/desktop_uninit_a.rs` | `ensure_room_session` 已 `pub`（api.rs:120）；desktop 已有 `src/lib.rs`（`pub mod ui_dioxus` 等 4 个 pub mod），**无需拆 lib+bin**；crate 名 = `northhing` |
| 2 | `test_api_functions_fail_cleanly_before_init` | `src/apps/desktop/src/ui_dioxus/api_settings.rs:198`（`#[tokio::test]`） | `src/apps/desktop/tests/desktop_uninit_b.rs` | 测试体连打 12+ 个 api 函数（api.rs:200-211 区域，全 `pub`）+ 构造 `MCPServerDto`；逐字搬迁，包括其中的 `let _ =` 调用（见约束 C5，tests/ 目录不占 rot 配额） |
| 3 | `test_result_methods_return_error_before_init` | `src/crates/assembly/core/src/kernel_facade/tests.rs:381` | `src/crates/assembly/core/tests/kernel_facade_uninit.rs` | `kernel_facade()` 与 `KernelFacade::new()` 均 `pub`；`kernel_facade` 模块**无 feature 门控**（core lib.rs:18）；core 已有 `tests/` 目录 |
| 4 | `e2e_storage_guard_rejects_missing_isolated_roots` | `src/crates/assembly/core/src/infrastructure/app_paths/path_manager.rs:237` | `src/crates/assembly/core/tests/path_manager_uninit.rs` | 测试依赖同文件 `mod tests` 里的 `EnvVarGuard`（:259）和 `ENV_LOCK`（:209）——**在新文件内重建精简版 helper**（单测试文件不需要 ENV_LOCK）；若被测函数/类型可见性不足，只允许 `#[cfg(test)] pub` 形态（约束 C2） |
| 5 | `test_session_manager_not_initialized` | `src/crates/services/terminal/src/session/singleton.rs:92`（`#[tokio::test]`） | `src/crates/services/terminal/tests/terminal_singleton_uninit.rs` | `session_manager()` / `is_session_manager_initialized()` **已是 pub**（singleton.rs:59/64，经 `session/mod.rs:21` re-export）——**不需要仲裁书假设的 seam**；crate 名 = `terminal-core`；迁移后**去掉 `if !is_session_manager_initialized()` 软守卫**（独立进程保证未初始化，改硬断言 `assert!(session_manager().is_none())`） |

## 3. Spec（全部满足才算完）

- S1：新建 5 个 `tests/*.rs` 文件，每个**恰好一个** `#[test]`/`#[tokio::test]`，测试体从源位置逐字迁移（#5 按上表去软守卫除外）。
- S2：每个新文件顶部 3 行注释：①此文件因 A 类「未初始化断言」单测独占进程而独立成文件；②不要向本文件添加任何会触发 `init_core()` / `init_*()` / 全局单例初始化的测试；③违反即回归。
- S3：源位置的 5 个旧测试删除；每个 crate 的测试总数不下降（迁移前后 `cargo test` 计数对比进 report）。
- S4：验证全绿（命令见 §5），含 `--test-threads=1` 串行一遍。
- S5：`git diff` 中不得出现非 `#[cfg(test)]` 的可见性提升；不得改动被测实现代码（除 C2 允许的 cfg(test) seam）。

## 4. Global Constraints（逐字遵守）

- C1：一测试一文件，不合并、不加第二个测试。
- C2：**禁 `pub(crate) → pub`**；唯一允许的可见性变更形态是 `#[cfg(test)] pub`，且须带注释「测试专用 seam，release 构建不存在」。
- C3：不许动 `FACADE` 的 `OnceLock` 形态、`global_scheduler`、六层依赖方向。
- C4：git 纪律：禁 `git add -A` / `git restore .` / `git checkout .` / `git stash`；只许点名 `git add <file>`；commit 前 `git diff --cached --name-only` 复核。
- C5：`let_underscore` rot 闸 388/388 零余量——`src/` 内非测试路径不许新增 `let _ =`；`tests/` 目录不占配额（已核 checker 排除逻辑），迁移携带的既有 `let _ =` 合法。
- C6：测试不得触生产存储（真实 keyring / 真实 config 目录 / 真实 memory.db）。
- C7：失败处置：环境性失败（链接器等）上报 NEEDS_CONTEXT 并附原文，不许假绿。

## 5. 验证（命令 + 输出原文进 report）

cargo 一律走 MSVC：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo ...`。输出重定向用 `cmd /c "... > log 2>&1"`（**禁 PowerShell 管道**，会因子进程继承句柄永久阻塞）。长命令 PTY 后台 + 轮询。

1. `cargo check --workspace`（0 error）
2. `cargo check -p northhing`（0 error，家规 6）
3. `cargo test -p northhing --test desktop_uninit_a --test desktop_uninit_b`（2 passed）
4. `cargo test -p northhing-core --features product-full --test kernel_facade_uninit --test path_manager_uninit`（2 passed）
5. `cargo test -p terminal-core --test terminal_singleton_uninit`（1 passed）
6. 回归：迁移前在各 crate 跑一次相关 module 测试计数，迁移后同命令复跑，计数对比（不许下降）
7. 串行：上述 3/4/5 各加 `-- --test-threads=1` 再跑一遍
8. `pnpm run check:rot`（绿）

## 6. 报告

写到 `.superpowers/sdd/w14-1c-1-report.md`：迁移清单（旧 file:line → 新 file）/ 验证命令+输出原文 / 测试计数对比表 / 「复用侦察」节（查了哪些符号、EnvVarGuard 重建的理由）/ 遇到的每个编译错误修在哪一层 / 结尾状态词（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）。

## 7. 派发元信息

- BASE：`e151b54`；完成后自行 commit（conventional commits，message 含 W14-1c-1），commit 前按 C4 复核。
- 禁区：`FACADE`/`global_scheduler`/六层方向/生产存储/非点名文件。
