# W22-2 Implementation Report — P2-4 残余小改：session 删除挂钩文件清理

## 改动摘要

1. **cleanup 触发段与 `run_file_cleanup` 函数** (`src/crates/assembly/core/src/agentic/session/session_manager_lifecycle.rs`):
   - 在 `delete_session` 的快照清理阶段（`stage=snapshot_cleanup`）之后插入独立的 `file_cleanup` 阶段，成对使用 `started_at` 与 `debug!` 计时。
   - 调用抽取出的独立函数 `run_file_cleanup(&crate::infrastructure::path_manager_arc()).await`，遵循 `path_manager_arc()` 单一路标与 warn-continue 原则，失败仅记录 `warn!`，不中断删除主流程。
   - 实现 `pub(crate) async fn run_file_cleanup(path_manager: &PathManager)`，内部使用 `CleanupService::new(path_manager.clone(), CleanupPolicy::default())` 执行 `cleanup_all().await`。

   **`run_file_cleanup` 签名原文**：
   ```rust
   pub(crate) async fn run_file_cleanup(path_manager: &PathManager)
   ```

   **`delete_session` 插入段 diff 摘要**：
   ```rust
           debug!(
               "Session deletion stage completed: session_id={}, stage=snapshot_cleanup, duration_ms={}",
               session_id,
               elapsed_ms_u64(snapshot_stage_started_at)
           );
   
   +       let cleanup_stage_started_at = Instant::now();
   +       debug!(
   +           "Session deletion stage starting: session_id={}, stage=file_cleanup",
   +           session_id
   +       );
   +       run_file_cleanup(&crate::infrastructure::path_manager_arc()).await;
   +       debug!(
   +           "Session deletion stage completed: session_id={}, stage=file_cleanup, duration_ms={}",
   +           session_id,
   +           elapsed_ms_u64(cleanup_stage_started_at)
   +       );
   
           let context_stage_started_at = Instant::now();
   ```

2. **单元测试与隔离机制** (`src/crates/assembly/core/src/agentic/session/session_manager_lifecycle_tests/session_manager_lifecycle_tests_rollback_delete.rs`):
   - 添加测试 `run_file_cleanup_deletes_expired_temp_files_in_isolated_user_root`。
   - **测试隔离机制说明**：
     - 测试使用 `TestWorkspace::new()` 在系统的临时目录（`std::env::temp_dir()`）下创建独立随机目录。
     - 通过 `workspace.path_manager()` 调用 `PathManager::with_user_root_for_tests(workspace.path().join("user-root"))`，将 `user_root` 重定向到工作区内的私有子目录。
     - `PathManager::temp_dir()`、`logs_dir()`、`cache_root()` 均派生自此独立 `user_root`，完全不触碰生产或真实用户 profile 路径（`~/.config/northhing`）。
     - 测试在私有 `temp` 目录下创建 8 天前修改的过期临时文件和新鲜临时文件，调用 `run_file_cleanup` 后断言过期文件被删除、新鲜文件被保留。
     - `TestWorkspace` 在测试退出时自动 `remove_dir_all`，保证无残留。

3. **台账翻转** (`docs/status/tech-debt-ledger.md`):
   - 翻转 P2-4 状态为 `resolved (2026-09-09, W22-2: session 删除触发全局 temp/log/cache 清理已通；孤儿快照纳入为独立设计题划出)`。

---

## 验证

### 1. `cargo check --workspace`

- **Command**: `C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check --workspace`
- **Exit Code**: `0`
- **Output 原文**:
```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
warning: `northhing-core` (lib) generated 16 warnings (run `cargo fix --lib -p northhing-core` to apply 15 suggestions)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
warning: `northhing` (lib) generated 2 warnings (run `cargo fix --lib -p northhing` to apply 2 suggestions)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
warning: `northhing` (bin "northhing") generated 60 warnings (2 duplicates) (run `cargo fix --bin "northhing" -p northhing` to apply 9 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 32.28s
```

### 2. 新增测试跑绿 (`run_file_cleanup`)

- **Command**: `C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full --lib run_file_cleanup`
- **Exit Code**: `0`
- **Output 原文**:
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 22s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)

running 1 test
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_rollback_delete::run_file_cleanup_deletes_expired_temp_files_in_isolated_user_root ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1072 filtered out; finished in 0.01s
```

### 3. 既有测试回归 (`delete_session`)

- **Command**: `C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full --lib delete_session`
- **Exit Code**: `0`
- **Output 原文**:
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.02s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)

running 1 test
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_rollback_delete::delete_session_removes_workspace_cache_entry ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1072 filtered out; finished in 0.01s
```

---

## 状态

DONE
