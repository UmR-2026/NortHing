# W26-4 Report — P2-18 预留标注 + stop_server 死分支清理（lsp manager.rs）

## 改动摘要

1. **AC1 预留标注**：
   - 在 `src/crates/assembly/core/src/service/lsp/manager.rs:103` 为 `uninstall_plugin` 添加规范标注注释：
     ```rust
     // reason: uninstall_plugin() is reserved for the upcoming plugin-management surface; today plugins are registered during initialize
     ```
   - 标注形态严格对齐仓内先例（`workspace.rs:450`）。
2. **AC2 死分支清理**：
   - 清理 `manager.rs:130-132`（`uninstall_plugin` 中的 `stop_server` 调用）与 `manager.rs:302-304`（`shutdown` 中的 `stop_server` 调用）两处不可达的 `if let Err` 死分支。
   - `stop_server` 内部本身为 warn-and-continue 且恒返 `Ok(())`（:231-243），两处调用点直接改为 `let _ = self.stop_server(...).await;` 并附单行注释说明其刻意不报错设计。
   - 保持 `stop_server` 签名与行为不变（兼容其他调用方）。
   - 保留 `rollback_registration`：`:129-134` 死分支内的 rollback 虽随死分支移除，但 `rollback_registration` 在 `:137`（文件删除失败回滚）仍有生产活跃调用，函数完整保留未删。
3. **AC3 行数硬约束达标**：
   - 改动前：836 行（rot-budget ceiling 836）。
   - 改动后：833 行。
   - 净减 3 行，满足 `< 836` 净减硬约束。
4. **AC4 台账 P2-18 翻转文案建议**：
   - 建议文案：`P2-18: resolved (uninstall_plugin reserved annotation added; unreachable Err branches for stop_server in manager.rs:130 and manager.rs:302 cleaned to warn-and-continue semantics)`（限定 manager.rs，format.rs 不在本单声明范围）。由编排者在波收口执行。

## 复用侦察

- 预留标注格式复用仓内先例：`src/crates/assembly/core/src/service/workspace/workspace_manager/workspace.rs:450`（`// reason: ensure_server_running() is reserved for the upcoming server auto-start path...`）与 `client.rs:283`。
- warn-and-continue 语义注释（`// stop_server intentionally never errors (warn-and-continue design).`）复用仓内既有错误处理惯例。

## 验证

### 1. `rustup run stable-x86_64-pc-windows-msvc cargo check --workspace`
在基于 BASE `cc586ae` 的干净工作树中执行（排除工作区其它并行任务未完成代码的编译干扰）：
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 03s
```
结果：0 error。

### 2. `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full lsp`
在干净工作树中执行（排除其它并行任务 tests.rs 语法错误干扰）：
```text
running 15 tests
test service::lsp::plugin_loader::tests::validated_plugin_id_accepts_safe_ids ... ok
test service::lsp::plugin_loader::tests::validated_plugin_id_error_kinds_are_precise ... ok
test service::lsp::plugin_loader::tests::validated_plugin_id_rejects_unsafe_ids ... ok
test service::lsp::plugin_loader::tests::uninstall_missing_plugin_errors ... ok
test service::lsp::manager::tests::uninstall_file_delete_failure_rolls_back_registration ... ok
test service::lsp::plugin_loader::tests::install_rejects_missing_manifest_with_zero_fs_effect ... ok
test service::lsp::plugin_loader::tests::install_rejects_corrupt_archive_with_zero_fs_effect ... ok
test service::lsp::plugin_loader::tests::uninstall_refuses_target_outside_plugins_dir_via_symlink ... ok
test service::lsp::plugin_loader::tests::install_extract_failure_in_staging_leaves_no_half_install ... ok
test service::lsp::manager::tests::uninstall_unregistered_plugin_keeps_unregister_error_and_skips_stop ... ok
test service::lsp::plugin_loader::tests::install_already_installed_fails_no_residue ... ok
test service::lsp::plugin_loader::tests::load_plugin_rejects_mismatched_manifest_id ... ok
test service::lsp::plugin_loader::tests::install_then_uninstall_roundtrip_no_residue ... ok
test service::lsp::plugin_loader::tests::install_rejects_invalid_id_with_zero_fs_effect ... ok
test service::lsp::manager::tests::uninstall_stops_servers_by_resolved_language_keys ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 1059 filtered out; finished in 1.17s
```
结果：15 passed, 0 failed. LSP manager 既有用例全部通过。

### 3. 行数证据
- `manager.rs` countLines:
  - 改动前：836
  - 改动后：833
  - 净减：-3 行（< 836 达标）

### 4. `node scripts/check-repo-hygiene.mjs`
```text
Repository hygiene check passed (36 content files scanned, 3962 filenames checked).
```
结果：exit 0。

### 5. `node scripts/verify-rot-budget.mjs --base cc586ae`
```text
Rot budget verification passed (5 grep rules [unwrap_production=488/502, expect_production=951/1089, let_underscore=372/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=144/400], 9 god-file rules checked across 1399 files [src: 1371, northing-installer/src-tauri: 11, scripts: 17]) — verdict: at-limit (violations: 0, warnings: 0, advisories: 4; rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
```
结果：exit 0。`god_file:src/crates/assembly/core/src/service/lsp/manager.rs` 833 < 836。

### 6. `node scripts/check-core-boundaries.mjs`
```text
Core boundary check passed.
```
结果：exit 0。

### 7. `node scripts/verify-task-gate.mjs verify-attempt --base cc586ae --tip c5f2b9c --allowlist .superpowers/sdd/w26-4-allowlist.txt`
```text
Attempt verification passed: all modified files are within allowlist.
```
结果：exit 0。

最后 Commit SHA: `c5f2b9c` (以及随 report 回填的 commit)

## 疑虑

- **工作区多任务并发干扰**：工作树存在并行任务 W26-5 的未提交 WIP 文件（如 `src/apps/desktop/src/ui_dioxus/entry.rs` 及 `src/crates/assembly/core/src/service/mcp/server/manager/tests.rs`），导致在宿主工作树直接跑全量 `cargo check --workspace` 和 `cargo test -p northhing-core` 时会撞上 W26-5 的编译错误。本单严格遵循纪律未动非允许集文件，通过在隔离的 BASE clean worktree 中验证，证实本单改动在净代码树下 100% 编译通过、0 error、15 测试全绿。

## 状态

DONE
