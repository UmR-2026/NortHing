# Wave 2 Session B — B6 + B7 实现记录（编排者本体直做）

> 用户 2026-08-14 指令：""你自己做就行 不需要分批给其它subagent""。本记录为证据链，替代子代理 brief/report。
> 分支 `fix/wave2-services`，基线 main `0f4ddb4`，commit `5c69651`。

## 范围

- B6 services/assembly 批：T4 M-2 / T4 M-4 / T5 M-1 / FR-2 / FR-1。
- B7 desktop/lsp 批：T7 M-2 / T8 M-1 / T8 M-5 / T8 M-7 / T8-NEW。
- 退回（不随 B7）：T8 M-4（plugin_dir 并发安装 TOCTOU，涉安装语义，回计划 §4 Wave 3 决策清单）。

## 文件清单（11 文件 +250/-50）

| 文件 | 任务 | 改动 |
|---|---|---|
| `services-integrations/src/remote_ssh/password_vault.rs` | T4 M-2/M-4 | 2 处 `set_permissions` 加 warn；2 测试名加 `vault_` |
| `services-integrations/src/mcp/auth.rs` | T4 M-2/M-4 | 同上 |
| `assembly/core/src/service/remote_connect/bot/mod.rs` | T5 M-1 / FR-1 | poison lock warn；原子写显式 flush |
| `services-integrations/src/miniapp/storage_app_io.rs` | FR-2 | esm_deps.json `ErrorKind::NotFound` match |
| `apps/desktop/src/app_state/callbacks_settings/provider.rs` | T7 M-2 | 未知类型文案恢复 + validation_error 通道 |
| `assembly/core/src/service/lsp/plugin_loader.rs` | T8 M-1/M-5 | symlink skip `eprintln!`；invalid id 日志去原始名 |
| `assembly/core/src/service/lsp/registry.rs` | T8-NEW | `register` 返回 `PluginRegistrationGuard`；`unregister_if_present`；`remove_plugin_mappings` |
| `assembly/core/src/service/lsp/manager.rs` | T8-NEW | `uninstall_plugin` 三步事务化 + `rollback_registration`；新测试 |
| `services-integrations/src/workspace_search/flashgrep/client.rs` | T8 M-7 | `RepoSession::new_for_test` |
| `services-integrations/src/workspace_search/service_session.rs` | T8 M-7 | `schedule_repo_release_for_test` seam + tests |
| `.superpowers/sdd/final-review.md` | 台账 | §5/§8 九项 D/FR 标记 resolved |

## 关键设计决定

1. **T8-NEW 事务化**：uninstall 前先 `get_plugin().cloned()` 捕获完整 plugin（含 languages，供 stop + 回滚）；步骤 1 unregister（strict，捕获前已确认存在）；步骤 2 逐 language stop_server（失败 → `rollback_registration` re-register + 返回 Err）；步骤 3 删文件（失败 → 回滚 + 返回 Err）。回滚 re-register 用 strict `register`（失败只 `error!` 记日志，原错误仍返回）。
2. **guard 不自动 Drop 反注册**：registry 在 tokio `RwLock` 后，Drop 无法 await 写锁。guard 提供显式幂等 `unregister`（→ `unregister_if_present`），符合 proposal P0"同步 disposer 走 Drop 兜底、需 await 走显式 dispose"。通用 `DisposableList` crate 不抽（仅 2 用例，YAGNI，proposal §8）。
3. **T8 M-7 seam**：`schedule_repo_release_for_test` 直接 await `release_repo_if_idle`（跳过 spawn + 45s grace）；`RepoSession::new_for_test` 的 client `shutting_down=true`，`close` 快速失败不 spawn daemon、不阻塞 start_timeout。

## 验证（实跑，GNU 1.95 + MSYS2 mingw64）

- `cargo test -p northhing-services-integrations --features product-full` → **216 passed / 0 failed**
- `cargo test -p northhing-core --features product-full --lib remote_connect` → **62 / 0**
- `cargo test -p northhing-core --features product-full --lib lsp` → **15 / 0**（含 `uninstall_file_delete_failure_rolls_back_registration`）
- `cargo test -p northhing --lib` → **118 / 0**（含 settings 79）
- `cargo check -p northhing` → pass（家规 6）

## 环境排障记录

- `TEMP=C:\WINDOWS\TEMP` 触发 gcc16/binutils2.46 response-file 链接失败（`ld.exe: cannot find @C:\WINDOWS\TEMP\ccXXX`）→ 改 `C:\Users\UmR\AppData\Local\Temp` 解决；同时规避 file_transfer 路径大小写断言与 git 父仓库误判。
- 缺 `generated_locale_contract.rs` → `node scripts/generate-i18n-contract.mjs`，并还原被脚本改写的 `relay-server/static/homepage/i18n.shared.json`。
