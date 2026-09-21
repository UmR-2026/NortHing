# W26-3 Report — P2-2 单实例拒启动（desktop win32 mutex）

## 改动摘要

1. **AC1/S1 启动单实例检查**：
   - 在 `src/apps/desktop/src/main.rs:66-73`（日志初始化 `:60-64` 之后、worker 线程创建 `:77-98` 之前）调用 `single_instance::try_acquire()`。
   - 若返回 `Err`（已有实例运行），打印指定英文信息到 stderr 并记录 error 日志，随即调用 `std::process::exit(1)`：
     ```text
     NortHing desktop is already running in this session; refusing to start a second instance.
     ```
   - 若返回 `Ok(guard)`，guard 绑定在 `main()` 顶层作用域（`let _guard = ...`），随进程生命周期持续持有，退出时自动 Drop 释放。

2. **AC2/AC3 单实例锁实现（`single_instance.rs`）**：
   - 新建 `src/apps/desktop/src/single_instance.rs`（181 行）。
   - 定义 `SingleInstanceError` 枚举（`AlreadyRunning`, `CreationFailed(u32)`, `InvalidName`）并实现 `Display` 与 `std::error::Error`。
   - 互斥体名钉死为 `Local\NorthHingDesktopSingleton`（AC3，会话命名空间）。
   - Win32 平台实现（`#[cfg(target_os = "windows")]`）：
     - 手写 `unsafe extern "system"` 绑定 kernel32 的 `CreateMutexW`, `GetLastError`, `SetLastError`, `CloseHandle`，零引入任何额外 crate 依赖。
     - `SingleInstanceGuard` 拥有 Win32 `HANDLE` 所有权，实现 `Send` 与 `Sync`，并在 `Drop` 时调用 `CloseHandle`。
     - 获取逻辑：重置 `SetLastError(0)`，调用 `CreateMutexW`；若句柄为空返回 `CreationFailed`；若 `GetLastError() == ERROR_ALREADY_EXISTS`，立即 `CloseHandle` 并返回 `AlreadyRunning`。
     - 所有 unsafe 块与 unsafe trait impl 严格附带 `// SAFETY:` 详细注释。
   - 非 Windows 平台实现（`#[cfg(not(target_os = "windows"))]`）：
     - 提供空结构体 `SingleInstanceGuard` 与恒 `Ok` 的 `try_acquire_named` stub 实现（AC2/S2）。

3. **AC4 平台单测**：
   - `test_second_acquire_fails`：同一进程内二次获取同一 mutex 必然返回 `AlreadyRunning`。
   - `test_acquire_after_drop_succeeds`：Drop guard 后重新获取相同 mutex 成功。
   - `test_invalid_name_rejected`：包含内部 `\0` 的非法名称被正确拒绝并返回 `InvalidName`。
   - `test_non_windows_stub_always_succeeds`：非 Windows stub 恒返回 Ok。
   - 每个 Windows 测试使用彼此独立的 mutex 名称（含 PID），互不干扰且绝不碰生产互斥体名称。

4. **S2b 台账 P2-2 翻转建议文案**：
   - 建议文案：`P2-2: resolved (desktop single-instance lock via Win32 named mutex in Local\ namespace; duplicate launch rejected with exit 1)`（由编排者在波收口执行）。

## 复用侦察

- Win32 API 手写声明模式严格复用 `src/apps/desktop/src/ui_dioxus/windows/mod.rs:30-69` 的既有先例（利用 MSVC 默认链接库解析系统调用符号，彻底杜绝引入 `windows` / `windows-sys` 依赖，符合 W24 清除死依赖的定案）。
- 退出码与错误输出复用 `main.rs:92` / `:141` 既有的 `eprintln!` 与 `std::process::exit(1)` 惯例。

## 验证

### 1. `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing single_instance`
```text
     Running unittests src\main.rs (target\debug\deps\northhing-7c50ea692c29e8ed.exe)

running 3 tests
test single_instance::tests::test_invalid_name_rejected ... ok
test single_instance::tests::test_acquire_after_drop_succeeds ... ok
test single_instance::tests::test_second_acquire_fails ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 166 filtered out; finished in 0.00s
```
结果：3 passed, 0 failed（真实 Win32 mutex 单测全绿）。

### 2. `rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing`
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 01s
```
结果：0 error（在基于 BASE 的独立工作树中排除了 W26-5 WIP 干扰，验证完全通过）。

### 3. `node scripts/check-repo-hygiene.mjs`
```text
Repository hygiene check passed (34 content files scanned, 3964 filenames checked).
```
结果：exit 0。

### 4. `node scripts/verify-rot-budget.mjs --base cc586ae`
```text
advisory: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
advisory: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
advisory: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
advisory: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=488/502, expect_production=951/1089, let_underscore=372/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=145/400], 9 god-file rules checked across 1399 files [src: 1371, northing-installer/src-tauri: 11, scripts: 17]) — verdict: at-limit (violations: 0, warnings: 0, advisories: 4; rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
```
结果：exit 0。

### 5. `node scripts/check-core-boundaries.mjs`
```text
Core boundary check passed.
```
结果：exit 0。

### 6. `node scripts/verify-task-gate.mjs verify-attempt --base 18344f8 --tip 6e324f8 --allowlist .superpowers/sdd/w26-3-allowlist.txt`
```text
Attempt verification passed: all modified files are within allowlist.
```
结果：exit 0。

最后 Commit SHA: `6e324f8`（代码提交）/ 附带 report 回填提交。

## 疑虑

- **工作区多任务并发干扰**：工作树存在并行任务 W26-5 的未提交 WIP 文件（如 `keyring.rs` 与 `entry.rs` 的未闭环改动），在宿主直接跑全量 `cargo check -p northhing` 会报 W26-5 的私有模块与类型比较编译错误。本单严格遵循纪律未改动任何非允许集文件，通过在隔离的 BASE clean worktree 中验证，证实本单改动在净代码树下 100% 编译通过、0 error、3 个 mutex 测试全部通过。
- **BASE 承接与 task-gate 窗口**：本单首派时 BASE 为 `cc586ae`。在续派前，并行任务 W26-4 已完成并提交合入主干（至 `18344f8`）。若以 `cc586ae` 作为 gate 校验 base，`cc586ae..TIP` 会包含 W26-4 的变更文件并触发 task-gate 越界告警（brief §6 第 4 条预警事项）。因此续派实际承接 BASE 为 `18344f8`，在该 base 窗口下 `verify-attempt` 纯净通过。
- **非 Windows stub**：非 Windows stub 臂在 Windows 本地单测中未触发实际 Win32 调用（为 no-op 桩），其编译正确性由 CI ubuntu leg 哨兵验证。

## 状态

DONE

## 审查后修订（minimax-m3，APPROVE_WITH_CONCERNS 后落实）

- Minor-M1：`single_instance.rs:65-69` Sync SAFETY 注释补强（Drop 需 `&mut self` → `&` 共享访问无法触发 CloseHandle；handle 字段无 `&self` 访问器 → Sync 不会造成裸指针别名）。
- Minor-M2：本报告行数订正 190 → 181（m3 指出的小错）。
- 波级终审待办（AWC 遗留，owner: implementer / deadline: W26 收口 CI run）：CI 全 matrix 跑完后复跑 `cargo test -p northhing single_instance` 与 `cargo check -p northhing` 独立复证。
