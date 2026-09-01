# Task Audit-I6 Report: vault 钥匙文件原子写（password_vault + mcp auth）

## 1. 实现内容 summary

1. `json_store.rs`:
   - 新增 `pub async fn write_bytes_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), JsonFileStoreError>` Helper 方法。
   - 方法内完整实现父目录创建、按路径写锁获取、tmp 文件写入、带有指数退避重试的原子替换（`replace_file_from_temp`）以及 Windows 文件锁占用时的 `PermissionDenied` 降级直接写入。
   - 重构 `write_atomic`：将 JSON 序列化为 Pretty JSON 字节流后直接委托调用 `write_bytes_atomic`，保持原有外部行为与错误映射完全一致。

2. `password_vault.rs`:
   - 将 SSH 密码 vault 的 32 字节主密钥写入点（57-59 行）替换为：`JsonFileStore.write_bytes_atomic(&self.key_path, key.as_slice()).await.context("write ssh password vault key")?;`。
   - 后续 `#[cfg(unix)]` chmod 0o600 逻辑原样保留。

3. `auth.rs`:
   - 将 MCP OAuth vault 的 32 字节主密钥写入点（114-116 行）替换为：`JsonFileStore.write_bytes_atomic(&self.key_path, key.as_slice()).await.context("write MCP OAuth vault key")?;`。
   - 后续 `#[cfg(unix)]` chmod 0o600 逻辑原样保留。

4. `json_store_contracts.rs`:
   - 新增 `json_store_write_bytes_atomic_round_trips_raw_bytes` 测试：验证 `write_bytes_atomic` 写入 32 字节裸 key 后由 `tokio::fs::read` 读出逐字节一致。
   - 新增 `json_store_write_bytes_atomic_overwrites_and_cleans_up_temp_files` 测试：验证连续覆盖写后内容为最新，且目标目录下无 `.tmp` 临时文件残留。

## 2. 复用侦察

新提取的 `write_bytes_atomic` 共享并驱动了 `JsonFileStore` 内全套私有原子写机制：
- 写锁机制：`Self::get_file_write_lock(path)`
- 临时文件命名：`Self::build_temp_json_path(path, attempt)`（带有 process id + nanosecond nonce + attempt 的唯一临时文件名）
- 替换逻辑：`Self::replace_file_from_temp(path, &tmp_path)`（rename 尝试、失败时删目标重试）
- 重试判定与延迟：`Self::is_retryable_write_error` & `Self::retry_delay`
- Fallback 机制：Windows 环境下被索引器锁定时 `PermissionDenied` 退化为 direct overwrite

未发生任何重复代码或独立写循环逻辑的复制粘贴。

## 3. 编译错误归因与修复分析

本次实现与测试无编译错误（0 compile errors）。

## 4. Ponytail 残余声明

1. **不加 fsync**：钥匙文件写入继承 `JsonFileStore` 现有写策略，不执行额外 `fsync` / `sync_all` 目录同步，保持与 `JsonFileStore` 全局统一。
2. **chmod 在 rename 后**：Unix 环境下 0o600 权限调整保持在 `replace_file_from_temp` 成功之后执行，与 `write_vault` 现有一致，残余毫秒级 umask 默认权限窗口。

## 5. 验证命令与输出原文

### 1) `$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP; cargo check --workspace`

```
    Checking northhing-services-core v0.2.10 (E:\agent-project\northing\src\crates\services\services-core)
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 26s
```

### 2) `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-core --test json_store_contracts`

```
   Compiling northhing-services-core v0.2.10 (E:\agent-project\northing\src\crates\services\services-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.91s
     Running tests\json_store_contracts.rs (target\debug\deps\json_store_contracts-8efbd074796955c7.exe)

running 5 tests
test json_store_reports_no_parent_directory ... ok
test json_store_returns_none_for_missing_file ... ok
test json_store_write_bytes_atomic_overwrites_and_cleans_up_temp_files ... ok
test json_store_creates_parent_dirs_and_round_trips_payload ... ok
test json_store_write_bytes_atomic_round_trips_raw_bytes ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### 3) `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-integrations --features product-full --lib password_vault`

```
   Compiling northhing-services-core v0.2.10 (E:\agent-project\northing\src\crates\services\services-core)
   Compiling northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 12.29s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_integrations-482ce0ac9a8d71b5.exe)

running 10 tests
test remote_ssh::password_vault::tests::remove_fails_closed_on_corrupted_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::migrate_fails_closed_on_truncated_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::remove_fails_closed_on_truncated_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::migrate_fails_closed_on_corrupted_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::store_fails_closed_on_corrupted_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::store_fails_closed_on_truncated_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::migrate_entry_moves_password_to_new_connection_id ... ok
test remote_ssh::password_vault::tests::load_returns_error_on_corrupted_vault ... ok
test remote_ssh::password_vault::tests::vault_store_is_atomic_and_keeps_bak_of_previous_content ... ok
test remote_ssh::password_vault::tests::vault_remove_deletes_file_when_last_entry_is_removed ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 37 filtered out; finished in 0.03s
```

### 4) `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-integrations --features product-full --lib mcp::auth`

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.21s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_integrations-482ce0ac9a8d71b5.exe)

running 7 tests
test mcp::auth::tests::clear_fails_closed_on_corrupted_vault_without_touching_file ... ok
test mcp::auth::tests::clear_fails_closed_on_truncated_vault_without_touching_file ... ok
test mcp::auth::tests::store_fails_closed_on_corrupted_vault_without_touching_file ... ok
test mcp::auth::tests::store_fails_closed_on_truncated_vault_without_touching_file ... ok
test mcp::auth::tests::load_returns_error_on_corrupted_vault ... ok
test mcp::auth::tests::vault_clear_deletes_file_when_last_entry_is_cleared ... ok
test mcp::auth::tests::vault_store_is_atomic_and_keeps_bak_of_previous_content ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 40 filtered out; finished in 0.03s
```

## 6. 修改文件清单

- `src/crates/services/services-core/src/json_store.rs`
- `src/crates/services/services-integrations/src/remote_ssh/password_vault.rs`
- `src/crates/services/services-integrations/src/mcp/auth.rs`
- `src/crates/services/services-core/tests/json_store_contracts.rs`

## 7. 自审发现与疑虑

- **自审结论**：按照 Brief Spec 1-4 要求无缝完成重构。现有契约测试及 vault 测试 100% 跑通，无编译告警或错误。
- **疑虑**：无。
