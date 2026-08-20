# Handoff — Wave 2 Session B（B6 services → B7 desktop/lsp）完成（2026-08-14）

接手者：从 `E:\agent-project\northing` 启动的编排者会话（或合并 Session A 的会话）。
上游交接：`2026-08-14-wave2-ready.md`（双 session 分工）。本文只记 Session B 增量。
事实源：计划 `.superpowers/sdd/plan-2026-08-04-backend-followups.md` §3；插件化 `docs/architecture/plugin-system-proposal.md` §8。

## 1. 一句话结论

Session B 的 B6（services/assembly 批）+ B7（desktop/lsp 批）**已由编排者本体直接完成**（用户 2026-08-14 指令"你自己做就行，不需要分批给其它 subagent"），单 commit `5c69651`，验证全绿，台账九项已翻转。**未派任何实现子代理。**

## 2. 基线状态

- northing 分支 `fix/wave2-services`（worktree `.worktrees/wave2-services`），基线 main `0f4ddb4`。
- 提交：`5c69651`（11 文件 +250/-50）。
- 未 push；未合并 main（等待 Session A 的 B5 完成后串行合，**先 A 后 B**）。

## 3. 完成内容（commit 5c69651）

### B6 services/assembly 批

| 项 | 内容 |
|---|---|
| T4 M-2 | SSH + MCP OAuth vault 的 `set_permissions(0o600)` 失败从静默吞掉改 `tracing::warn!`（4 处：`password_vault.rs` / `auth.rs` 各 key+file） |
| T4 M-4 | 4 个 vault 测试名补 `vault` 前缀（`vault_store_is_atomic_*` / `vault_clear_*` / `vault_remove_*`），`cargo test ... vault` filter 全捕获 |
| T5 M-1 | bot persistence `PERSISTENCE_WRITE_LOCK` poison 恢复加 `warn!("Bot persistence write lock poisoned, recovering")` |
| FR-2 | `storage_app_io.rs` esm_deps.json 从 `.exists()` 预检改 `ErrorKind::NotFound` match，与 `read_optional_source_file` 统一（消 TOCTOU 语义） |
| FR-1 | bot persistence 原子写补显式 flush（`File::create`+`write_all`+`flush`+drop-before-rename，对齐 settings 模式） |

### B7 desktop/lsp 批

| 项 | 内容 |
|---|---|
| T7 M-2 | `upsert_provider` 未知类型分支恢复具体文案 `不支持的服务类型: {ptype}`，走 `validation_error` 通道 |
| T8 M-1 | Windows symlink 测试静默 `return` 改 `eprintln!` 报告 skip |
| T8 M-5 | invalid plugin id 日志只输出校验错误，不再 `{:?}` 暴露原始目录名 |
| T8 M-7 | services-integrations 加 `#[cfg(test)] schedule_repo_release_for_test` seam + `RepoSession::new_for_test`，测试观察 idle session 实际释放 |
| **T8-NEW** | LSP `uninstall_plugin` 三步事务化（clone plugin → unregister → 逐 language stop_server → 删文件），步骤 2/3 失败逆序回滚 re-register，根治 FU-2 同类半卸载态；`PluginRegistry.register` 返回 `PluginRegistrationGuard`（幂等 undo） |

- **T8 M-4 范围裁定（退回 Wave 3）**：plugin_dir 并发安装 TOCTOU 涉安装语义，不在 B7 任务清单，退回计划 §4 决策清单，不随 B7。
- 新增测试 2 个：`uninstall_file_delete_failure_rolls_back_registration`、`schedule_repo_release_for_test_releases_idle_session`。
- 台账翻转：`final-review.md` §5/§8 的 T4 M-2/M-4、T5 M-1、T7 M-2、T8 M-1/M-5/M-7、FR-1、FR-2 共 9 项标记 resolved（同 commit）。

## 4. 验证（编排者实跑，GNU toolchain 1.95 + MSYS2）

| 命令 | 结果 |
|---|---|
| `cargo test -p northhing-services-integrations --features product-full` | **216/216 pass** |
| `cargo test -p northhing-core --features product-full --lib remote_connect` | **62/62 pass** |
| `cargo test -p northhing-core --features product-full --lib lsp` | **15/15 pass** |
| `cargo test -p northhing --lib` | **118/118 pass** |
| `cargo check -p northhing`（家规 6） | **pass** |

## 5. 环境陷阱（本机，下一 session 必读）

1. **gcc 16.1.0 + binutils 2.46.1 的 response-file bug**：`TEMP=C:\WINDOWS\TEMP` 时任何 build script 链接报 `ld.exe: cannot find @C:\WINDOWS\TEMP\ccXXX: Invalid argument`（aws-lc-sys、northhing-core 均中招）。**改 `TEMP=C:\Users\UmR\AppData\Local\Temp` 即愈**——该目录可写可删、不在 git 仓库内、大小写与 `canonicalize` 一致（同时避免 file_transfer 的路径大小写断言与 git `branch --show-current` 误判父仓库）。验证命令统一：`$env:TEMP="C:\Users\UmR\AppData\Local\Temp"; $env:PATH="C:\msys64\mingw64\bin;"+$env:PATH; cargo ...`（只 prepend mingw64，**不要** usr/bin）。
2. **新 worktree 缺 `generated_locale_contract.rs`**（`core/src/service/i18n/`，gitignore 的生成文件）→ `node scripts/generate-i18n-contract.mjs` 生成。⚠️ 该脚本会顺带改写 `relay-server/static/homepage/i18n.shared.json`（**Session A 禁区**），跑完必须 `git checkout -- src/apps/relay-server/static/homepage/i18n.shared.json` 还原。
3. 测试临时目录必须落在 git 仓库外，否则 `git branch --show-current` 会返回父仓库分支名，打破"非 git 目录"断言。

## 6. 待办 / 交接

- **串行合 main（先 A 后 B）**：Session A（B5 relay）先合，Session B 再合。两分支文件集不相交（B5=relay，B6/B7=services+core+desktop），但 `final-review.md` §5 是共享文件——A 翻转 T1/T2/T3 行、B 翻转 T4/T5/T7/T8 行，不同行可自动合并；冲突时按行解。
- **Wave 2 收口（合并后）**：回归扫对齐各自基线 → 终审（独立视角）→ 更新 handoff。计划 §6 完成定义：final-review §5 的 18 D 项清零至 ≤ Wave 3 决策项。
- **Wave 3 决策项（未动）**：T2 capability-token、T3 SPA-fallback、T3 axum-dep、T8 M-4、跨模块竞态（§6 Gap 4）。
- 本 session 未更新 `tech-debt-ledger.md`（B6/B7 的 D 项 ledger 在 `final-review.md` §5，已翻转；`P2-18 uninstall_plugin 无生产调用方` 事实未变）。
