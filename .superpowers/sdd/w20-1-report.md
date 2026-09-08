# W20-1 Report — P2-23 清账：terminal-core 非 Windows E0624 修复

## 改动摘要

- **根因**：`src/crates/services/terminal/src/exec/types.rs:197` 中 `impl LocalPipeControlState` 的 `fn deadline` 方法缺少可见性修饰（默认为模块私有），在 non-Windows unix 目标下被兄弟模块 `exec/output.rs:482` 和 `exec/output.rs:493` 跨模块调用时触发 `error[E0624]: method deadline is private` ×2。
- **改动**：
  1. `src/crates/services/terminal/src/exec/types.rs`：将 `fn deadline(self) -> tokio::time::Instant` 提升为 `pub(crate) fn deadline(self) -> tokio::time::Instant`，对齐 `types.rs` 全文件统一风格。
  2. `docs/status/tech-debt-ledger.md`：依据家规 2 同 commit 将 P2-23 状态由 `deferred` 翻转为 `resolved`。
- **Commit**：`18abc57` (`fix(terminal-core): promote LocalPipeControlState::deadline to pub(crate) (W20-1)`)。

## 验证

### BASE 负向证据（复跑确认）

命令：`rustup run stable-x86_64-pc-windows-msvc cargo check -p terminal-core --target x86_64-unknown-linux-gnu`
Exit code: 101
输出：
```
    Checking terminal-core v0.2.10 (<REPO_ROOT>\src\crates\services\terminal)
error[E0624]: method `deadline` is private
   --> src\crates\services\terminal\src\exec\output.rs:482:57
    |
482 |                 if tokio::time::Instant::now() >= state.deadline() {
    |                                                         ^^^^^^^^ private method
    |
   ::: src\crates\services\terminal\src\exec\types.rs:197:5
    |
197 |     fn deadline(self) -> tokio::time::Instant {
    |     ----------------------------------------- private method defined here

error[E0624]: method `deadline` is private
   --> src\crates\services\terminal\src\exec\output.rs:493:45
    |
493 |                 .map(LocalPipeControlState::deadline)
    |                                             ^^^^^^^^ private method
    |
   ::: src\crates\services\terminal\src\exec\types.rs:197:5
    |
197 |     fn deadline(self) -> tokio::time::Instant {
    |     ----------------------------------------- private method defined here

For more information about this error, try `rustc --explain E0624`.
error: could not compile `terminal-core` (lib) due to 2 previous errors
```

---

### 验证 1：Linux 目标 check

命令：`rustup run stable-x86_64-pc-windows-msvc cargo check -p terminal-core --target x86_64-unknown-linux-gnu`
Exit code: 0
输出：
```
    Checking terminal-core v0.2.10 (<REPO_ROOT>\src\crates\services\terminal)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.22s
```

---

### 验证 2：macOS (aarch64) 目标 check

命令：`rustup run stable-x86_64-pc-windows-msvc cargo check -p terminal-core --target aarch64-apple-darwin`
Exit code: 0
输出：
```
    Checking terminal-core v0.2.10 (<REPO_ROOT>\src\crates\services\terminal)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.18s
```

---

### 验证 3：Windows workspace check 回归

命令：`rustup run stable-x86_64-pc-windows-gnu cargo check --workspace`
（环境变量：`PATH` 注入 MSYS2 MinGW 工具链目录 `<MSYS2_MINGW_BIN>`，`TEMP/TMP/TMPDIR` 指向 `<TEMP_DIR>`）
Exit code: 0
输出：
```
    Checking northhing-services-integrations v0.2.10 (<REPO_ROOT>\src\crates\services\services-integrations)
    Checking northhing-core v0.2.10 (<REPO_ROOT>\src\crates\assembly\core)
    Checking northhing v0.2.10 (<REPO_ROOT>\src\apps\desktop)
    Checking northhing-acp v0.2.10 (<REPO_ROOT>\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (<REPO_ROOT>\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 24s
```

---

### 验证 4：Windows terminal-core 测试回归

命令：`rustup run stable-x86_64-pc-windows-gnu cargo test -p terminal-core`
Exit code: 0
输出：
```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 28.31s
     Running unittests src\lib.rs (target\debug\deps\terminal_core-ca223e5655c53ed1.exe)

running 21 tests
test pty::process::tests::keeps_non_host_tauri_and_normal_env_vars ... ok
test pty::process::tests::strips_tauri_host_configuration_from_parent_env ... ok
test session::session_manager::tests::stream_output_delta_returns_none_when_output_is_unchanged ... ok
test session::session_manager::tests::stream_output_delta_returns_utf8_suffix_without_cutting_chars ... ok
test shell::integration::shell_integration::tests::test_unescape_value ... ok
test shell::integration::shell_integration::tests::test_parse_command_line ... ok
test shell::integration::shell_integration::tests::continuation_prompt_is_recorded_as_recent_plain_output ... ok
test shell::integration::shell_integration::tests::test_parse_prompt_start ... ok
test shell::integration::shell_integration::tests::test_parse_command_finished_with_exit_code ... ok
test session::session_manager::tests::stream_output_delta_resets_when_previous_snapshot_is_not_prefix ... ok
test session::session_manager::tests::completion_reason_serializes_with_camel_case_contract ... ok
test shell::scripts_manager::tests::test_get_script_content ... ok
test shell::integration::shell_integration::tests::post_command_prompt_is_recorded_as_recent_plain_output ... ok
test shell::integration::shell_integration::tests::test_parse_cwd_property ... ok
test shell::scripts_manager::tests::test_get_script_path ... ok
test shell::scripts_manager::tests::test_compute_hash_is_deterministic ... ok
test session::binding::tests::test_binding_operations ... ok
test session::serializer::tests::test_serialize_deserialize ... ok
test pty::data_bufferer::tests::test_buffering_disabled ... ok
test pty::data_bufferer::tests::test_max_buffer_size_flush ... ok
test pty::data_bufferer::tests::test_buffering_enabled ... ok

test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests\terminal_singleton_uninit.rs (target\debug\deps\terminal_singleton_uninit-903e3b48201d7b35.exe)

running 1 test
test test_session_manager_not_initialized ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests terminal_core

running 3 tests
test src\crates\services\terminal\src\pty\process.rs - pty::process::spawn_pty (line 311) ... ignored
test src\crates\services\terminal\src\session\singleton.rs - session::singleton::init_session_manager (line 30) ... ignored
test src\crates\services\terminal\src\session\singleton.rs - session::singleton::session_manager (line 52) ... ignored

test result: ok. 0 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

---

### 验证 5：仓库卫生检查

命令：`node scripts/check-repo-hygiene.mjs`
Exit code: 0
输出：
```
Repository hygiene check passed (2 content files scanned, 3874 filenames checked).
```

---

## 残余风险与环境说明

1. 本任务的跨平台验证使用 `cargo check --target` 进行 cross-target 类型检查，不等于目标平台的真机构建（不进行二进制链接，亦无目标系统 CI runner 的集成执行测试）。
2. 本单严格限界于解决 `terminal-core` 的编译错误挂账，workspace 其余 crate 在非 Windows 目标下的编译状态未做验证，按既定拍板 CI 矩阵仍维持 windows-only。

## 状态

DONE
