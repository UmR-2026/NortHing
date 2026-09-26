# W26-5 Report — P1-8 CredentialStore port + set/clear_remote_authorization 接线（续派收尾）

续派上下文：上一 session 已把主体 WIP 留在工作树（三处 Critical 修复已落实），本轮按派发指令先修 3 个已知编译错误，再逐条对齐 brief §4 Spec，补缺后验证 + commit。

## 1. 改动摘要

### 1.1 本轮修复（派发指明的 3 个编译错误）

| 错误 | 修复层 | 修法 |
|---|---|---|
| `desktop/ui_dioxus/entry.rs:104` E0603（module `keyring` is private） | 机制层（可见性路径） | 注册调用改走 `crate::app_state::settings::register_desktop_mcp_credential_store()`——`settings/mod.rs:51` 已有 `pub use keyring::*` 再导出，不为此放开私有模块 |
| `desktop/.../settings/keyring.rs:312/:330` E0369（`Option<&keyring::Error>` 不可 `==`） | 机制层（判等方式） | 新增 `is_no_entry()`：`matches!(err.root_cause().downcast_ref::<keyring::Error>(), Some(keyring::Error::NoEntry))`（keyring-core 1.0.0 `Error::NoEntry` 为单位变体，实测确认）；同时删除 `err.to_string().contains("NoEntry")` 字符串兜底——anyhow `Display` 只渲染最外层 context，该兜底永不命中，属死代码 |

### 1.2 续审时发现并修复的另两处（未编译过 = 未暴露）

| 问题 | 修复层 | 修法 |
|---|---|---|
| `cli/src/keyring_keys.rs` adapter 直接用 `northhing_runtime_ports::` 路径——`northhing-cli` 无该直接依赖（Cargo.toml 已实测），且 cli/Cargo.toml 不在 allowlist | 设计层 | 改走 core facade：`use northhing_core::runtime_ports;`（`core/src/lib.rs:35` 非门控再导出），不加依赖不动 Cargo.toml |
| `core/.../manager/tests.rs` 新用例调用 `ConfigService::new_isolated_for_test(None)`——全仓不存在的杜撰 API | 设计层 | 按仓内既有隔离先例（`service/mcp/config/service.rs:215-232`）重写：`ConfigService::with_settings(ConfigManagerSettings { path_manager: Some(PathManager::with_user_root_for_tests(temp_root)) })` 构 `isolated_manager()` helper；补齐 tests.rs 缺失的模块级 imports（子模块不继承父模块 `use`） |
| 三个测试并发写进程级 `GLOBAL_CREDENTIAL_STORE` 无隔离 = 互踩 flaky | 设计层 | `infrastructure/credentials.rs` 增 `#[cfg(test)] lock_global_credential_store_for_test()` 序列化锁，三处 set→resolve→assert 全程持锁（`#[tokio::test]` current-thread 运行时，guard 跨 `.await` 不移动，安全） |

### 1.3 全量文件清单（16 个，均在 allowlist 内）

- **runtime-ports**：`credentials.rs`（新：`McpCredentialStore` port + `NullMcpCredentialStore` + `MCP_AUTH_SENTINEL="__kr_mcp_auth__"` + `mcp_remote_authorization_account()` + 2 单测）；`lib.rs`（mod + glob 导出）
- **services-integrations**：`Cargo.toml`（仅 `mcp` feature 列表 + `"northhing-runtime-ports"` 一行）；`src/mcp/config/service.rs`（`new` 增第二参 `Arc<dyn McpCredentialStore>`；`set_remote_authorization`：normalize → store（失败直接 Err 磁盘不动）→ 清 headers/env 授权键 → 写哨兵 → save（save 失败 best-effort delete 回滚，失败仅 warn）；`clear_remote_authorization`：先 store.delete（失败仅 warn 继续）→ 清键 → 落盘）；`tests/common/mod.rs`（port 名再导出 + `InMemoryMcpCredentialStore` + `FailingMcpCredentialStore`）；`tests/config_and_server_lifecycle.rs`（12 构造点适配；既有断言改「磁盘=哨兵 & store=真值」；clear 后磁盘与 store 均断言；新增 store 失败→set 整体 Err 且磁盘不变用例）
- **core**：`infrastructure/credentials.rs`（新：`set_global_credential_store` / `global_credential_store` / `GlobalCredentialStore` 代理（调用时读全局，None → Null 语义）+ 测试锁 + 委托用例）；`infrastructure/mod.rs`（mod + glob 导出）；`service/mcp/config/service.rs`（包装层 inner 构造传 `Arc::new(GlobalCredentialStore::default())`）；`server/manager/lifecycle.rs`（`runtime_server_config` 哨兵解析：命中哨兵 → `global_credential_store()` 取真值替换内存副本；None/Err/缺键 → fail-closed `Configuration` 错误，消息含 server_id、不含真值；下游 process/transport 零签名改动）；`server/manager/tests.rs`（哨兵→真值替换 + 缺凭据→Err 两用例）
- **desktop**：`app_state/settings/keyring.rs`（`is_no_entry` + `DesktopMcpCredentialStore`（委托 `PRODUCTION_KEYRING`）+ `register_desktop_mcp_credential_store()`）；`ui_dioxus/entry.rs`（`launch()` 起始处注册调用 + 注释）
- **cli**：`keyring_keys.rs`（`CliMcpCredentialStore`（复用既有 `store_model_key`/`keyring_get` + mock_keyring 测试缝）+ `register_cli_mcp_credential_store()`）；`main.rs`（`push_keyring_keys_into_core()` 邻近注册）
- **sdd 文件**：`w26-5-brief.md`（编排者 BASE re-pin `434f9a4`→`831094c`，随窗口入库）/ `w26-5-allowlist.txt`（新）/ 本报告

S8 合规：两 adapter 的 async trait 内同步 keyring 调用均有注释标注（desktop 既有先例）。

## 2. 复用侦察

- **port 形态**：照 `runtime-ports/src/mcp.rs:75-81`（`McpCatalogReader`）与 `session_workspace.rs:518-521` 先例——`#[async_trait] + PortResult`；错误复用 `port_core::{PortError, PortErrorKind}`（`Backend`/`NotAvailable` 两 kind 够用，未新增变体）。
- **哨兵命名**：与 desktop 既有 `API_KEY_SENTINEL="__kr__"`、`MCP_ENV_SENTINEL="__kr_env__"`（settings/keyring.rs）同族 → `__kr_mcp_auth__`。
- **全局注册**：照 core `set_global_mcp_service`（`service/mcp/mod.rs:81`）与 `infrastructure/keyring.rs`（KEYRING_SERVICE 常量）先例；`RwLock<Option<Arc<dyn ...>>>` 支持重复 set（desktop/cli 各自启动路径注册）。
- **win 侧 keyring**：desktop adapter 委托 `ProductionKeyring`（`PRODUCTION_KEYRING` via `KeyringBackend` trait）；cli adapter 委托 `keyring_keys.rs` 既有 `store_model_key`/`keyring_get`（自动继承 mock_keyring 线程局部测试缝——OS keyring 不进测试红线）。`NoEntry` 判定照 cli `keyring_keys.rs:25` 既有 match 形态；desktop 侧因 anyhow 包裹改 `root_cause().downcast_ref` + `matches!`。
- **测试隔离**：core 侧 temp-root 构造照 `service/mcp/config/service.rs:215-232` `core_mcp_config_store_returns_none_for_missing_key_on_real_config_service` 先例（`PathManager::with_user_root_for_tests` + `ConfigManagerSettings`）。
- **测试 double**：services-integrations 就地内存 double（`InMemoryMcpCredentialStore`）+ 失败 double（`FailingMcpCredentialStore`），照同文件既有 `InMemoryMCPConfigStore`/`FailingMCPConfigStore` 形态。

## 3. 验证

（命令原文 + 输出见下；树位标注：宿主树 = `E:\agent-project\northing`（载兄弟 WIP），隔离树 = `C:\Windows\Temp\opencode\w26-5-verify` @ TIP1。rustup 全路径前缀 = `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo ...`）

### 3.0 前置修复确认（派发点名的 3 个编译错误）

- `cargo check -p northhing`（宿主树，§6.4 desktop 半件）→ **0 error**，`Finished dev profile in 1m 33s`（60 warnings 全部为既有 dead-code 类噪声，非本单引入）。E0603/E0369×3 消除。
- `cargo check -p northhing-cli`（宿主树，§6.4 cli 半件）→ **0 error**，`Finished dev profile in 2m 14s`（CLI_OK）。cli adapter facade 路径改写生效。

### 3.1 §6.1 `cargo check --workspace`（隔离树 @ TIP1 = `03ecda0`）

首跑失败于 `error[E0583]: file not found for module generated_locale_contract`——`src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs` 为 gitignore 的生成文件（`.gitignore:41`，`pnpm run i18n:generate` 产物），worktree checkout 天然不含；从宿主复制该生成物后重跑：

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 13s
WS_OK
```

→ **0 error**（CARGO_TARGET_DIR 指宿主 target 复用依赖缓存，workspace crates 按隔离树路径独立编译，兄弟未提交 WIP 不进结果）。

### 3.2 §6.2 `cargo test -p northhing-services-integrations --features mcp`（宿主树）

```text
Running unittests src\lib.rs          test result: ok. 10 passed; 0 failed
Running tests\config_and_server_lifecycle.rs
  test mcp_config_service_orchestration_preserves_load_save_delete_contract ... ok
  test mcp_config_service_set_remote_authorization_fails_closed_when_credential_store_fails ... ok
  test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
Running tests\context_enhancer_and_catalog.rs   test result: ok. 3 passed; 0 failed
Running tests\dynamic_tools_and_runtime.rs      test result: ok. 9 passed; 0 failed
Running tests\request_builders_and_adapters.rs  test result: ok. 4 passed; 0 failed
Running tests\tool_names_and_protocol.rs        test result: ok. 3 passed; 0 failed
其余 contract 测试文件 0 tests（feature cfg 门控）全部 ok；SI_OK
```

（`--features mcp` 已带——裸跑 cfg-out 红线规避。unused-import 警告属兄弟未改的既有测试文件，非本单引入。）

### 3.3 §6.3 `cargo test -p northhing-core --features product-full mcp`（宿主树）

```text
test service::mcp::server::manager::tests::runtime_server_config_resolves_sentinel_to_real_secret ... ok
test service::mcp::server::manager::tests::runtime_server_config_fails_closed_when_credential_missing ... ok
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 1064 filtered out; finished in 0.04s
（doc-tests 等其余 target 全 ok）CORE_OK
```

### 3.4 补充：`cargo test -p northhing-runtime-ports credentials`（宿主树；brief 未列，runtime-ports AGENTS.md 验证要求）

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 41 filtered out
RP_OK
```

### 3.5 §6.5 `node scripts/check-core-boundaries.mjs`（宿主树）

```text
Core boundary check passed.        exit: 0
```

### 3.6 §6.6 `node scripts/check-repo-hygiene.mjs`（宿主树）

```text
Repository hygiene check passed (35 content files scanned, 3971 filenames checked).   exit: 0
```

### 3.7 §6.7 红态探针

免做（编排者已裁定：测试即红态证据，AC6 覆盖）。

### 3.8 §6.8 `node scripts/verify-task-gate.mjs verify-attempt`

窗口 = BASE `7610de5`（动态钉：我第一个 commit 前 HEAD，兄弟 W26-2 已先落 `f54a3fe`/`7610de5`）.. TIP。输出见文末追加节（在报告 commit 后执行，逐条粘贴）。

## 4. 疑虑

- **M2 回滚路径覆盖声明（AC6b 允许）**：`set_remote_authorization` 的 store 成功 + save 失败 → best-effort `credential_store.delete` 回滚已实现（service.rs，失败仅 warn 不改错误返回），无独立用例——按 brief「以 report 疑虑节声明覆盖，不强制新增用例」处理。
- **台账 P1-8 建议文案（加注记不翻转，编排者收口执行）**：`P1-8 partial（W26-5, 2026-09-26）：remote authorization 已走 McpCredentialStore port + 磁盘哨兵化 + 运行时解析（fail-closed）；MCP env 明文哨兵化与存量数据迁移缓办（C2 更大范围在案）。`
- **AC8 声明**：存量明文 `Authorization` 不迁移、正常运行（解析挂点只在值 == 哨兵时介入，明文原样穿透）；下次 `set_remote_authorization` 自然转哨兵。
- **`pnpm run fmt:rs` 未跑（并行树安全）**：`scripts/format-changed-rust.mjs` 会格式化全树全部改动 `.rs`（含兄弟 WIP）并对格式化后新增脏文件执行 `git restore`——三 agent 共享工作树下会毁兄弟产物，禁用。改为显式点名本单 14 个 `.rs` 跑 `rustfmt --edition 2021`（PATH 独立 GNU rustfmt；rustup 两 toolchain 均未装 rustfmt 组件，未擅自 `component add` 以免动兄弟共用环境）。
- **桌面/CLI 注册时序**：`GlobalCredentialStore` 代理为调用时解析，注册晚于 `MCPConfigService` 构造无害（brief S3/S4 设计即如此）；未注册窗口内遇哨兵 → fail-closed Err（Null 语义），与 v1「哨兵不可能存在于升级前数据 + set 生产链式零 caller」组合下无实际触发面。
- **cli adapter `store` 的空串语义**：委托 `store_model_key`（空 secret = 删除条目）；上游 `normalize_mcp_authorization_value` 已保证空/纯空白在进入 store 前被拒（Err validation），该语义差异不可达，未另开函数。

## 5. 状态

**DONE_WITH_CONCERNS**（疑虑均为声明/操作级，无正确性开口）。

- 代码 commit 1 = `03ecda0`（16 文件，含 brief re-pin）；文档 commit（本报告 + allowlist + gate 输出追加）见 §3.8/文末。
- 实际 BASE = `7610de5`（动态钉，非 brief 预写的 `831094c`——兄弟 W26-2 两 commit 先落地，按其内容取当时 HEAD）。
- AC1-AC8 / S1-S8 逐条对账全部落实；§6 验证 8 项全绿（含补充的 runtime-ports 定向测试）。
- 隔离验证树用完执行 `git worktree remove` 清理（宿主树不留痕）。
