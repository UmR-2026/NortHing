# W14-1c-4b 实施报告 — C/D 类锁纪律全仓扫描

> 来源：`.superpowers/sdd/w14-1c-4b-brief.md`  
> BASE：`b7675d1`  
> 状态：DONE

---

## 1. 扫描命令与命中计数

### 1.1 环境变量与工作目录修改扫描
```powershell
rg -n "(set_var|remove_var|set_current_dir)" src northing-installer tests
```
- **命中行数**：17 行，分布于 3 个文件。
- **命中明细**：
  - `northing-installer/src-tauri/src/installer/ai_config.rs` (4 处: 行 404, 434, 441, 471)
  - `src/crates/assembly/core/tests/path_manager_uninit.rs` (7 处: 行 24, 26, 42, 43, 44, 45, 46)
  - `src/crates/assembly/core/src/infrastructure/app_paths/path_manager.rs` (6 处: 行 273, 274, 275, 276, 306, 308)

### 1.2 `set_current_dir` 专项全仓扫描
```powershell
rg -n "set_current_dir" .
```
- **命中行数**：0 行（全仓测试及业务代码均无动态修改进程 CWD 的调用）。

### 1.3 既有同步原语与锁使用位点扫描
```powershell
rg -n "(CWD_LOCK|ENV_LOCK|REMOTE_SEARCH_TEST_LOCK|TEST_GLOBAL_CONFIG_MUTEX)" src northing-installer tests
```
- **命中行数**：28 行，分布于 5 个文件（`api_settings.rs` 2 处, `api_provider_edit.rs` 8 处, `kernel_facade/tests.rs` 12 处, `service_helpers.rs` 4 处, `path_manager.rs` 2 处）。

---

## 2. 全量清单表

| 文件:行 | 测试名 | 改什么 | 持什么锁 | 判定（合规/违规） |
|---|---|---|---|---|
| `northing-installer/src-tauri/src/installer/ai_config.rs:401` | `write_model_then_theme_preserves_both` | `NORTHHING_INSTALLER_CONFIG_DIR` | 无（已补 `ENV_LOCK`） | 违规（已修复） |
| `northing-installer/src-tauri/src/installer/ai_config.rs:438` | `write_theme_then_model_preserves_both` | `NORTHHING_INSTALLER_CONFIG_DIR` | 无（已补 `ENV_LOCK`） | 违规（已修复） |
| `src/crates/assembly/core/tests/path_manager_uninit.rs:33` | `e2e_storage_guard_rejects_missing_isolated_roots` | `northhing_USER_ROOT`, `northhing_E2E_USER_ROOT`, `northhing_HOME`, `northhing_E2E_HOME`, `northhing_E2E_STORAGE_GUARD` | 无（已补 `ENV_LOCK`） | 违规（已修复） |
| `src/crates/assembly/core/src/infrastructure/app_paths/path_manager.rs:260` | `env_overrides_keep_e2e_storage_out_of_real_user_profile` | `northhing_USER_ROOT`, `northhing_E2E_USER_ROOT`, `northhing_HOME`, `northhing_E2E_HOME` | `ENV_LOCK` | 合规 |
| `src/apps/desktop/src/ui_dioxus/api_settings.rs:198` | `test_persist_onboarding_provider_success_flow` | `GlobalConfig`, `FACADE` (`init_core`) | `TEST_GLOBAL_CONFIG_MUTEX` | 合规 |
| `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:200` | `test_edit_provider_blank_key_inherits_existing` | `GlobalConfig` | `TEST_GLOBAL_CONFIG_MUTEX` | 合规 |
| `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:233` | `test_edit_provider_new_key_overwrites_keyring` | `GlobalConfig` | `TEST_GLOBAL_CONFIG_MUTEX` | 合规 |
| `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:267` | `test_edit_provider_keyring_read_error_fails_closed` | `GlobalConfig` | `TEST_GLOBAL_CONFIG_MUTEX` | 合规 |
| `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:294` | `test_edit_provider_nonexistent_id_returns_error` | `FACADE` (`init_core`) | `TEST_GLOBAL_CONFIG_MUTEX` | 合规 |
| `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:322` | `test_delete_provider_default_provider_rejected` | `GlobalConfig.default_provider_id` | `TEST_GLOBAL_CONFIG_MUTEX` | 合规 |
| `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:349` | `test_delete_provider_success_cleans_config_and_keyring` | `GlobalConfig` | `TEST_GLOBAL_CONFIG_MUTEX` | 合规 |
| `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:371` | `test_edit_provider_validation_failure_zero_writes` | `GlobalConfig` | `TEST_GLOBAL_CONFIG_MUTEX` | 合规 |
| `src/crates/services/services-integrations/src/remote_ssh/workspace_search/service_helpers.rs:134` | `remote_search_rejects_non_linux_before_stdio_open` | `REMOTE_SEARCH_CONTEXTS`, `REMOTE_STDIO_SESSIONS` | `REMOTE_SEARCH_TEST_LOCK` | 合规 |
| `src/crates/services/services-integrations/src/remote_ssh/workspace_search/service_helpers.rs:159` | `remote_search_context_ignores_stale_cache_before_resolving_connection` | `REMOTE_SEARCH_CONTEXTS` | `REMOTE_SEARCH_TEST_LOCK` | 合规 |
| `src/crates/services/services-integrations/src/remote_ssh/workspace_search/service_helpers.rs:202` | `remote_search_open_guard_is_removed_when_stdio_spawn_fails` | `REMOTE_SEARCH_CONTEXTS`, `REMOTE_STDIO_OPEN_GUARDS` | `REMOTE_SEARCH_TEST_LOCK` | 合规 |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:969` | `list_tree_rejects_parent_dir_escape` | CWD 依赖（`current_dir()`） | `CWD_LOCK` | 合规（C1 禁碰文件） |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:992` | `list_tree_rejects_absolute_path` | CWD 依赖 | `CWD_LOCK` | 合规（C1 禁碰文件） |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1030` | `list_tree_lists_direct_children` | CWD 依赖 | `CWD_LOCK` | 合规（C1 禁碰文件） |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1052` | `read_file_rejects_too_large` | CWD 依赖 | `CWD_LOCK` | 合规（C1 禁碰文件） |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1071` | `read_file_round_trip_within_cap` | CWD 依赖 | `CWD_LOCK` | 合规（C1 禁碰文件） |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1082` | `read_file_rejects_escape` | CWD 依赖 | `CWD_LOCK` | 合规（C1 禁碰文件） |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1138` | `read_file_rejects_symlink_to_outside_target` | CWD 依赖 | `CWD_LOCK` | 合规（C1 禁碰文件） |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1168` | `list_tree_skips_symlink_to_outside_target` | CWD 依赖 | `CWD_LOCK` | 合规（C1 禁碰文件） |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1201` | `list_tree_with_explicit_workspace_root_uses_that_fence` | CWD 依赖 | `CWD_LOCK` | 合规（C1 禁碰文件） |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1221` | `read_file_with_explicit_workspace_root_uses_that_fence` | CWD 依赖 | `CWD_LOCK` | 合规（C1 禁碰文件） |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1241` | `list_tree_rejects_non_absolute_workspace_root` | CWD 依赖 | `CWD_LOCK` | 合规（C1 禁碰文件） |

---

## 3. Patch 清单

1. **`northing-installer/src-tauri/src/installer/ai_config.rs`**:
   - 新增 `static ENV_LOCK: Mutex<()> = Mutex::new(());` 及其说明注释。
   - 实现 `EnvVarGuard` RAII 守卫，在 drop 时还原原环境变量值，防止测试 panic 导致环境变量污染泄漏。
   - `write_model_then_theme_preserves_both` 与 `write_theme_then_model_preserves_both` 函数入口获取 `_guard = ENV_LOCK.lock().unwrap_or_else(...)` 并使用 `EnvVarGuard::set(&dir)`。
2. **`src/crates/assembly/core/tests/path_manager_uninit.rs`**:
   - 新增 `static ENV_LOCK: Mutex<()> = Mutex::new(());` 及其说明注释。
   - `e2e_storage_guard_rejects_missing_isolated_roots` 函数入口获取 `_guard = ENV_LOCK.lock().unwrap_or_else(...)`。

---

## 4. 验证命令与输出摘录

### 4.1 全工作区 Check
- **命令**：
  ```cmd
  cd /d E:\agent-project\NortHing && C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check --workspace
  ```
- **结果**：
  ```text
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.94s
  0 errors
  ```

### 4.2 单元测试验证
- **`northhing-core` 独立集成测试 `path_manager_uninit`**：
  ```cmd
  cd /d E:\agent-project\NortHing && C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full --test path_manager_uninit
  ```
  ```text
  running 1 test
  test e2e_storage_guard_rejects_missing_isolated_roots ... ok
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  ```
- **`northhing-core` 单元测试 `path_manager`**：
  ```cmd
  cd /d E:\agent-project\NortHing && C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full --lib path_manager
  ```
  ```text
  running 9 tests
  test infrastructure::app_paths::path_manager::tests::env_overrides_keep_e2e_storage_out_of_real_user_profile ... ok
  test infrastructure::app_paths::path_manager::project_paths::tests::long_path_slug_stays_within_max_len ... ok
  test infrastructure::app_paths::path_manager::project_paths::tests::same_path_generates_stable_slug ... ok
  test infrastructure::app_paths::path_manager::project_paths::tests::pure_ascii_path_also_carries_hash_suffix ... ok
  test infrastructure::app_paths::path_manager::project_paths::tests::cjk_paths_differing_only_in_non_ascii_produce_distinct_slugs ... ok
  test infrastructure::app_paths::path_manager::assistant_workspace::tests::legacy_assistant_workspace_paths_remain_at_northhing_root ... ok
  test infrastructure::app_paths::path_manager::assistant_workspace::tests::assistant_workspace_paths_use_personal_assistant_subdir ... ok
  test infrastructure::app_paths::path_manager::assistant_workspace::tests::is_local_assistant_workspace_path_detects_personal_assistant_and_legacy ... ok
  test infrastructure::app_paths::path_manager::project_paths::tests::project_runtime_root_uses_human_readable_workspace_slug ... ok
  test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 1061 filtered out; finished in 0.00s
  ```
- **`northing-installer` 单元测试**：
  ```cmd
  cd /d E:\agent-project\NortHing\northing-installer\src-tauri && C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo test
  ```
  ```text
  running 13 tests
  test installer::commands::tests::test_verify_uninstall_path_mismatch_rejected ... ok
  test installer::commands::tests::test_normalize_path_for_comparison ... ok
  test installer::commands::tests::test_verify_uninstall_path_junction_or_link_literal_mismatch_rejected ... ok
  test installer::commands::tests::test_verify_uninstall_path_matches ... ok
  test installer::commands::tests::test_verify_uninstall_path_no_registration_rejected ... ok
  test installer::extract::tests::test_reject_empty_manifest_path ... ok
  test installer::commands::tests::test_verify_uninstall_path_empty_request_rejected ... ok
  test installer::extract::tests::test_reject_absolute_paths_posix_and_windows ... ok
  test installer::extract::tests::test_reject_zip_slip_traversal ... ok
  test installer::generated_locale_contract::tests::generated_installer_contract_keeps_canonical_aliases ... ok
  test installer::extract::tests::test_valid_manifest_relative_paths ... ok
  test installer::ai_config::tests::write_model_then_theme_preserves_both ... ok
  test installer::ai_config::tests::write_theme_then_model_preserves_both ... ok
  test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
  ```

### 4.3 Git Diff Check
- **命令**：`git diff --check`
- **结果**：0 whitespace errors。

### 4.4 Rot 闸自查
- **命令**：`node scripts/verify-rot-budget.mjs`
- **结果**：
  ```text
  Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=371/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=35/400], 6 god-file rules checked across 1365 files).
  ```
  `let_underscore` 保持 371/388，未增长。

---

## 5. 状态结论

**DONE**
