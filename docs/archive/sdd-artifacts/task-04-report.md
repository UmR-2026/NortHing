# Task 4 Report: SSH/MCP OAuth vault fail-closed + 原子写

状态：DONE（实现 + 测试 + 边界检查全部通过；未 commit，未触碰 worktree 既有改动）

## 改动清单（file:line）

| 文件 | 位置 | 改动 |
|---|---|---|
| `src/crates/services/services-integrations/src/remote_ssh/password_vault.rs` | `read_vault_file` L97-107、`backup_vault` L109-119、`write_vault` L121-133 | 新增私有 helper |
| 同上 | `store` L135-142、`load` L144-171、`remove` L173-186、`migrate_entry` L188-202 | 全部写路径改为 fail-closed 读 + 原子写；load 损坏返回 Err |
| `src/crates/services/services-integrations/src/mcp/auth.rs` | `read_vault_file` L158-168、`backup_vault` L170-180、`write_vault` L182-194 | 新增私有 helper |
| 同上 | `load` L196-229、`store` L231-242、`clear` L244-258 | 同上 |
| 两文件 tests 模块 | password_vault L206-349（10 测试）、auth.rs L415-519（6 测试） | 见测试清单 |
| `src/crates/services/services-integrations/Cargo.toml` | `remote-ssh-concrete` feature L125 | 增加 `northhing-services-core`（复用 `JsonFileStore` 所需） |
| `scripts/core-boundaries/rules/feature-rules.mjs` | `northhing-services-core` ownerFeatures L57 | 增加 `'remote-ssh-concrete'`（否则边界检查报 owner 覆盖缺失） |

## 复用 json_store 的方式选择及理由

**真复用 `JsonFileStore::write_atomic`**（services-core/src/json_store.rs:136-200），未复刻 tmp+nonce+rename 模式。理由：

- `write_atomic` 输出 `serde_json::to_string_pretty`，与两 vault 现有格式一致（无 pretty/后缀不匹配问题）；
- 自带 tmp（pid+nonce+attempt 命名）+ rename + Windows share-handle 重试 + PermissionDenied 兜底；
- services-integrations 本就分层依赖 services-core（git/mcp/miniapp/workspace-search 已启用）；`remote-ssh-concrete` 此前未启用该依赖，已在 Cargo.toml feature 中补上，并同步 feature-rules.mjs 的 owner 覆盖规则（两处同 commit 一致改，`check-core-boundaries.mjs` 通过）。
- 唯一取舍：`write_atomic` 在 PermissionDenied 时会降级为直接覆写（非原子）——但 `.bak` 备份在调用前已生成，且失败场景下数据不丢；此行为在 json_store 文档注释中已有说明，未改。

备份与权限策略（写入 `write_vault` 前做）：
1. `backup_vault()`：目标存在时先 `tokio::fs::copy(target, target.with_extension("bak"))`，失败仅 `tracing::warn` 不阻塞写（brief 要求）；
2. `JsonFileStore.write_atomic(...)` 原子替换（内含 `create_dir_all`，替代原手工建目录）；
3. rename 后 `#[cfg(unix)]` 补设 0o600（保持原行为）。

## 需求 1 实现细节（fail-closed）

`read_vault_file`（两 vault 同构）：
- 文件不存在（NotFound）→ `Ok(VaultFile::default())`（合法初始态）；
- `read_to_string` 其他失败 → `Err` + context `"failed to read vault: {path}"`；
- JSON 解析失败 → `Err` + context `"vault corrupted: {path}"`；
- 写路径（store/remove/migrate/clear）额外 `.context("refusing to overwrite vault")`（满足 "vault corrupted, refusing to overwrite" 类字面要求）。

load 路径：解析失败返回 `Err`（原 `unwrap_or_default` 已移除）；单 entry 解密失败保持 `warn + Ok(None)` 现状未改；文件缺失仍 `Ok(None)`。remove/clear 到空 entries 的 `remove_file` 语义保持（前提是解析成功）。

## load Err 传播的调用点影响评估

**SSHPasswordVault::load**（5 处，全部已处理 Result，无致命弹窗路径）：
- `manager_saved_connections.rs:145`（prune_saved_connections_without_credentials）：Err → warn + 按"不可用"剔除 profile；损坏时不再触发覆盖写，符合预期。
- `manager_saved_connections.rs:198`（save_connection）：`?` 传播给调用方（anyhow Err）；经 `assembly/core/src/service/remote_ssh` 再导出；src/apps 下无直接调用点，无弹窗语义。
- `manager_saved_connections.rs:244`/`has_stored_password` L248-256：Err → warn + false，非致命。
- `manager_saved_connections.rs:309`（load_connection_config_from_saved）：`?` 传播；`mgr_lifecycle_persist.rs:179-191` 对 Err 分支显式返回 "Failed to load stored SSH password"，行为明确。
- `mgr_lifecycle_persist.rs:179`：显式 match，Err 分支处理完备。

**MCP vault load**：`MCPRemoteOAuthCredentialStore::load` 将 anyhow Err 映射为 `AuthError::InternalError`；上游 `build_authorization_manager` / `has_stored_oauth_credentials` 均 `?` 传播；assembly 层 `map_auth_error` → `NortHingError::MCPError`。桌面无直接调用点（src/apps 无引用）。损坏 vault 现在表现为显式错误（触发重新授权流程而非静默按空处理），未发现将 Err 当致命弹窗的调用点。

## 测试清单与输出

新增 16 个测试（password_vault 10、mcp::auth 6），覆盖 brief 全部要求：损坏 JSON×store/remove/migrate/clear、截断×store/remove/migrate/clear（原文件字节不变断言）、load 损坏返回 Err、正常 store 可读回 + 第二次写后 `.bak` 存在且为上一版内容、清空后 remove_file 语义。

实际命令与输出（本机 PATH 下 `cargo` 解析到 GNU standalone，且 MSYS2 gcc 需 `C:\msys64\mingw64\bin` 在 PATH 首位才能找到 DLL——环境问题非本次改动引入，见"环境备注"）：

```
cargo check -p northhing-services-integrations                                    → exit 0
cargo check -p northhing-services-integrations --features product-full            → exit 0
cargo test  -p northhing-services-integrations --features product-full vault      → 16 passed; 0 failed
cargo test  -p northhing-services-integrations --features product-full password_vault → 10 passed; 0 failed
cargo test  -p northhing-services-integrations --features product-full mcp::auth   → 6 passed; 0 failed
cargo test  -p northhing-services-integrations --features product-full            → 全部 test binary ok，0 failed（lib 82 passed 等）
node scripts/check-core-boundaries.mjs                                            → Core boundary check passed
cargo fmt -p northhing-services-integrations -- --check                           → 干净（仅本任务两文件被格式化，无其他文件受影响）
```

## 顺手清配额

无（未发现范围内可顺手修正的同类问题）。

## 环境备注（非本次改动引入）

1. `cargo`/`rustc` 在 PATH 中解析到 `C:\Program Files\Rust stable GNU 1.95`（standalone），rustup shim 被覆盖；按 AGENTS.md 的 GNU 覆盖约定使用。用 `rustup run stable-msvc cargo` 会因 host 混合导致 `-Csplit-debuginfo=packed` 报错，故验证走 GNU 工具链。
2. MSYS2 gcc（C:\msys64\mingw64\bin）直接从 PowerShell 调用时静默失败（exit 1，无输出，DLL 解析问题）；将 `C:\msys64\mingw64\bin;C:\msys64\usr\bin` 置于 PATH 首位后可正常构建 aws-lc-sys。
3. `git status` 显示全仓 ~786 个 "M" 为 CRLF/LF 归一化伪影（`git diff` 为空），为 worktree 既有状态，未触碰。
4. `Cargo.lock` 有 1 处既有改动（northhing-relay-core 增加 base64/serde_json，前序 relay 任务产生），未触碰。

## 明确不做（已遵守）

- 未改 VaultFile schema / 加密结构 / key 管理；
- 未动 desktop 调用点 UI 流程；
- 未触碰 H-6/H-7/H-8 范围；
- 未 git commit。
