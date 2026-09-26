# W26-5 Review — P1-8 McpCredentialStore port + 哨兵化 + 双 surface adapter 注册

> 双判决（缺一不算通过）— SPEC + QUALITY。
>
> 参照：brief `w26-5-brief.md`、report `w26-5-report.md`、代码包 `reviews/w26-5/`、文档包 `reviews/w26-5-docs/`。
> 范围并集：BASE 7610de5..03ecda0（code）+ bf1df10..1ef1b16（docs），与编排者 §3.8 分窗口协议一致。

---

## SPEC：PASS（所有 AC 与 S 段均找到对应实现）

### AC1 — `McpCredentialStore` port + `NullMcpCredentialStore` + 哨兵 + 账号键函数

- **PASS** ✅
- `src/crates/contracts/runtime-ports/src/credentials.rs` 新文件，78 行：
  - L23-27 `trait McpCredentialStore: Send + Sync`（`store`/`get`/`delete` 三方法，async_trait，与 brief L18-22 形状逐字一致）
  - L11 `pub const MCP_AUTH_SENTINEL: &str = "__kr_mcp_auth__"`（与 desktop 既有 `__kr__`/`__kr_env__` 同族）
  - L15-17 `mcp_remote_authorization_account(server_id) -> String`，格式 `mcp.remote.{server_id}.authorization`（唯一定义点）
  - L34-55 `NullMcpCredentialStore`：`store`/`delete` → `Err(NotAvailable)`，`get` → `Ok(None)`，与 brief L23 一致
- `runtime-ports/src/lib.rs` L12 `pub mod credentials;` + L20 `pub use credentials::*;`，导出齐全
- 同文件 L58-78 含 2 单测（`null_credential_store_semantics` + `account_key_format`）
- 命名纪律 `McpCredentialStore`（不带裸名 `CredentialStore`）防与 `rmcp::transport::auth::CredentialStore` 撞名，符合 brief 复审 I2

### AC2 — `services-integrations` 构造签名扩展 + Cargo.toml 同步

- **PASS** ✅
- `src/crates/services/services-integrations/Cargo.toml` L70 在 `mcp` feature 列表增加 `"northhing-runtime-ports"`（一行，原 :19 已有 optional 依赖，feature 内启用合法，crate-rules.mjs:438 不禁）
- `src/crates/services/services-integrations/src/mcp/config/service.rs`：
  - L4 引入 `use northhing_runtime_ports::{mcp_remote_authorization_account, McpCredentialStore, MCP_AUTH_SENTINEL};`
  - L27 结构体增 `credential_store: Arc<dyn McpCredentialStore>`
  - L39 `pub fn new(config_store, credential_store: Arc<dyn McpCredentialStore>)` 双参签名
- 12 处测试构造点适配：`tests/config_and_server_lifecycle.rs` L27-28 / L74 / L104 / L136 / L163 / L191 / L218 / L240 / L261 / L319 / L375 / L434 全部从单参升级为双参（NullMcpCredentialStore 或 InMemoryMcpCredentialStore），与 brief AC2 一致

### AC3 — `set_remote_authorization` 流程（normalize → store → 清键 → 写哨兵 → save + 回滚）+ `clear_remote_authorization`

- **PASS** ✅
- `set_remote_authorization`（service.rs L179-224）：
  - L196-197 `normalize_mcp_authorization_value` 拒空
  - L199-205 `credential_store.store(&account, &normalized)` 失败 → Err（磁盘不动，符合 fail-closed）
  - L207-211 `remove_mcp_authorization_keys` 清 headers+env 后 `headers[Authorization] = MCP_AUTH_SENTINEL`
  - L213-221 save 失败 → best-effort `credential_store.delete(&account)`（warn 失败，不改错误返回）→ Err 透传
- `clear_remote_authorization`（service.rs L226-251）：
  - L239-245 先 `credential_store.delete(&account)`（仅 warn 即使失败，磁盘残留优先于 keyring 孤儿）
  - L247-249 清键后 `save_server_config` 落盘
- 与 brief AC3 语义逐字对应

### AC4 — 运行时解析（哨兵→真值替换内存副本；缺凭据 fail-closed）

- **PASS** ✅
- `src/crates/assembly/core/src/service/mcp/server/manager/lifecycle.rs` L4 `pub(super) async fn runtime_server_config`
- L16-41 命中 `auth_val == MCP_AUTH_SENTINEL` 分支：
  - L18-23 全局未注册 → `NortHingError::Configuration("Credential store is not available to resolve authorization for MCP server '{server_id}'")`（消息含 server_id，不含真值）
  - L24-33 `global_credential_store().get(&account).await` 失败 → `NortHingError::Configuration("Failed to retrieve credential for MCP server '{server_id}': {e}")`（error 来自 `PortError::Display {kind}: {message}`，keyring::Error 不泄露密钥本身）
  - L34-39 取到 None → `NortHingError::Configuration("Credential not found in credential store for MCP server '{server_id}'")`
  - L40 `config.headers.insert("Authorization".to_string(), secret)` 仅 **内存副本** 替换
- L5-14 路径二（ephemeral_configs 兜底）保留，与原实现兼容
- 下游 process/transport 签名零改动（仅 `runtime_server_config` 唯一漏斗变更内容）

### AC5 — 生产注册（调用时解析 + desktop/cli 注册点）

- **PASS** ✅
- `src/crates/assembly/core/src/infrastructure/credentials.rs` 新文件：
  - L10 `static GLOBAL_CREDENTIAL_STORE: RwLock<Option<Arc<dyn McpCredentialStore>>>`
  - L13-17 `set_global_credential_store`
  - L32-34 `global_credential_store() -> Option<Arc<dyn McpCredentialStore>>`
  - L41-67 `GlobalCredentialStore` 代理（`Default` derive），三个方法均调用时读 `global_credential_store()`，None 时 fallback `NullMcpCredentialStore`（Null 语义：store/delete → Err NotAvailable；get → Ok(None)）
  - L25-29 `#[cfg(test)] lock_global_credential_store_for_test` —— 序列化并发 set→resolve（详 §QUALITY）
- core 包装层接线 `src/crates/assembly/core/src/service/mcp/config/service.rs` L83 `let cred_store = Arc::new(crate::infrastructure::credentials::GlobalCredentialStore::default());` → L85 内层 `MCPConfigService::new(store, cred_store)`（即创建时只持代理，调用时再读全局）
- desktop 注册点 `src/apps/desktop/src/ui_dioxus/entry.rs` L106 `crate::app_state::settings::register_desktop_mcp_credential_store()`（launch 起始处，符合 brief `desktop main.rs 属 W26-3 领地` 回避 + `sync.rs 无生产 caller` 实证）
  - 适配器 `src/apps/desktop/src/app_state/settings/keyring.rs` L314-365 `DesktopMcpCredentialStore` + `register_desktop_mcp_credential_store`，委托 `PRODUCTION_KEYRING`（L205 `Lazy<ProductionKeyring>` via `KeyringBackend` trait）
  - L292-307 `is_no_entry` 用 `root_cause().downcast_ref::<keyring::Error>()` + `matches!(..., Some(NoEntry))` 取代原字符串兜底（anyhow Display 只渲染最外层 context，字符串 sniffing 永不命中，原实现是死代码）
- cli 注册点 `src/apps/cli/src/main.rs` L382 `keyring_keys::register_cli_mcp_credential_store()`（紧邻 L381 `push_keyring_keys_into_core()`）
  - 适配器 `src/apps/cli/src/keyring_keys.rs` L115-152 `CliMcpCredentialStore` + `register_cli_mcp_credential_store`
  - L108 `use northhing_core::runtime_ports;` —— **设计上正确**：cli 仓无 `northhing-runtime-ports` 直接依赖（`Cargo.toml:14` 仅 `northhing-core`），走 core lib.rs:35 非门控再导出 `pub use northhing_runtime_ports as runtime_ports;`，不污染 cli Cargo.toml（不在 allowlist）
  - L119-145 `store`/`get`/`delete` 委托 `store_model_key`/`keyring_get`，自动继承 `mock_keyring` 测试缝（OS keyring 不进测试红线）

### AC6 — 测试适配 + 新增

- **PASS** ✅
- 既有断言更新 `tests/config_and_server_lifecycle.rs` L43-47 改判 `headers[Authorization] == MCP_AUTH_SENTINEL` + L51-54 磁盘 JSON 同字段 `== MCP_AUTH_SENTINEL` + L62-64 新增 `cred_store.get(&account) == Some("Bearer plain-token")`（磁盘仅哨兵，store 内真值）
- L60-62 `clear_remote_authorization` 后 `cred_store.get(&account) == None`（磁盘与 store 均清）
- 新增 L734-775 `mcp_config_service_set_remote_authorization_fails_closed_when_credential_store_fails`：FailingMcpCredentialStore → set 整体 Err（含 MCPRuntimeErrorKind::Configuration 断言）+ 磁盘未变更断言 + X-Existing 保留断言
- core 侧新增 `src/crates/assembly/core/src/service/mcp/server/manager/tests.rs` L90-140 `runtime_server_config_resolves_sentinel_to_real_secret` + L143-186 `runtime_server_config_fails_closed_when_credential_missing`，均全量执行，无 skip/ignore

### AC6b — `--features mcp` 验证门遵循

- **PASS** ✅
- report §3.2 输出原文：`cargo test -p northhing-services-integrations --features mcp` → 19 passed in `tests\config_and_server_lifecycle.rs` + 10 lib unittests + 3/9/4/3 其它 contract。`--features mcp` 已带（裸跑 0 tests 红线规避，W15-1f 实证已引）
- M2 回滚路径（store 成功 + save 失败 → best-effort delete）已落实代码（service.rs:213-221）但无独立用例，按 brief AC6b「report 疑虑节声明覆盖，不强制新增用例」处理 —— report §4 首条已声明

### AC7 — §6 验证全绿 + 台账 P1-8 不动

- **PASS** ✅
- report §3.1-3.7 输出齐：
  - §3.1 `cargo check --workspace`（隔离树 @ 03ecda0）→ 0 error（含 `generated_locale_contract.rs` gitignore 副本规避说明）
  - §3.2 `cargo test -p northhing-services-integrations --features mcp` → 19/0
  - §3.3 `cargo test -p northhing-core --features product-full mcp` → 17/0（含 2 新 sentinel 用例）
  - §3.4 补充 `cargo test -p northhing-runtime-ports credentials` → 2/0（runtime-ports AGENTS.md 验证要求）
  - §3.5 `node scripts/check-core-boundaries.mjs` → exit 0（runtime-ports 新模块过闸；红则 BLOCKED）
  - §3.6 `node scripts/check-repo-hygiene.mjs` → exit 0
  - §3.7 免探针（report 引编排者裁定：测试即红态证据，AC6 覆盖）
- 台账 P1-8 不动（report §4 二条建议「partial 注记」由编排者收口执行，本单不动 ledger）

### AC8 — 存量迁移不做（声明式）

- **PASS** ✅
- report §4 三条已声明：「AC8 声明：存量明文 Authorization 不迁移、正常运行（解析挂点只在值 == 哨兵时介入，明文原样穿透）；下次 set_remote_authorization 自然转哨兵」
- 代码佐证：lifecycle.rs L16 `if auth_val == northhing_runtime_ports::MCP_AUTH_SENTINEL` 仅在哨兵匹配时介入（`==` 比较，明文非哨兵直接穿透）

### §4 Spec 段对齐

- **PASS** ✅
- S1 credentials.rs + lib.rs 导出 → AC1 已列
- S2 services-integrations 构造签名 + set/clear 语义 → AC2/AC3 已列
- S3 core 全局 + 代理 + 测试锁 → AC5 credentials.rs 已列；放 infrastructure 合理（credential 跨 mcp/api 共享，KEYRING_SERVICE 已在 infrastructure/keyring.rs:11 同层）
- S4 core 接线 → AC4 + AC5 已列
- S5 desktop/cli 注册与 adapter → AC5 已列
- S6 禁区合规 —— 16 文件均在 allowlist 内，未触及 metaRatchetPaths/rot-budget.json/W26-1/2/3/4 领地/cursor_format.rs/transport_remote.rs/kernel-api（已 git diff 复核）
- S7 不做项（env 哨兵化、存量迁移、UI 接线）未实现 —— 报告与代码一致
- S8 async trait 内同步 keyring 调用可接受 + 注释标注 → desktop L311-312 与 cli L112-113 均带同步理由注释

---

## QUALITY：PASS

### 复用侦察（brief §3 强制项）

- **PASS** ✅
- 6 条复用声明均经 codegraph/源码逐项核实：
  1. `McpCatalogReader` (runtime-ports/src/mcp.rs:75-81) 与 `PermissionPort` (session_workspace.rs:518-521) 形态先例 → AC1 `#[async_trait] + PortResult` 与之一致
  2. `set_global_mcp_service` (`service/mcp/mod.rs:81`) 与 `infrastructure/keyring.rs` KEYRING_SERVICE 单点先例 → credentials.rs:10 `RwLock<Option<Arc<dyn ...>>>` 一致
  3. `core_mcp_config_store_returns_none_for_missing_key_on_real_config_service` (`service/mcp/config/service.rs:215-232`) → manager/tests.rs:47-66 `isolated_manager` helper 完全照形态（`PathManager::with_user_root_for_tests` + `ConfigManagerSettings { path_manager, auto_save, backup_count }`）
  4. `InMemoryMCPConfigStore` + `FailingMCPConfigStore` 形态先例 → `InMemoryMcpCredentialStore` + `FailingMcpCredentialStore` 同形
  5. cli `keyring_keys.rs` 既有 `store_model_key` / `keyring_get` / `mock_keyring` thread-local 测试缝 → `CliMcpCredentialStore` 直接委托，自动继承 mock 红线
  6. desktop `ProductionKeyring` via `KeyringBackend` trait → `DesktopMcpCredentialStore` 委托 `PRODUCTION_KEYRING`，复用 ProductionKeyring 的 anyhow wrapping

### 无 owner 抽象（投机性抽象警戒）

- **PASS** ✅
- `McpCredentialStore`：双 surface（desktop/cli）实际 adapter 消费
- `NullMcpCredentialStore`：作为 `GlobalCredentialStore` 没注册时的 fallback（占不到 1/100 调用）—— 落实 brief AC5「未注册窗口内遇哨兵 → fail-closed」
- `GlobalCredentialStore`：2 个真实消费方（`MCPConfigService::new` L85 + `runtime_server_config` L18/24）；非投机性
- `is_no_entry`：3 个真实调用点（desktop adapter `get`/`delete` 各 1 处）；非投机性
- 无「将来可能用到」抽象

### 预算闸

- **PASS** ✅
- `scripts/rot-budget.json` 未触碰（git diff 0 hits）
- `.unwrap()`/.expect()/dead-code 等 ceilings 现状：production `.rs` 文件中 unwrap 仅 `infrastructure/credentials.rs:28` 一处 `LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())` —— `#[cfg(test)]` 函数体内，按 R-13 排除测试代码（checker semantics）与原 baseline 不冲突。其它改动均 `.map_err(|e| PortError::new(Backend, ...))`，无新增 production unwrap
- `metaRatchetPaths`（scripts/{verify-task-gate,verify-rot-budget,check-repo-hygiene,check-core-boundaries,check-github-config}.mjs、workflow-policy.json、rot-budget.json、package.json、.github/workflows/）均未触碰

### 条件早退测试

- **PASS** ✅
- 新增测试无 `cfg(skip)` / `if !condition { skip() }` / `make_symlink_or_ignore` 式 early-exit
- 4 个涉及改动的测试均全量执行：
  - `runtime_server_config_resolves_sentinel_to_real_secret`（manager/tests.rs:90-140）：构建 isolated_manager + 注入 store + 断言真值替换
  - `runtime_server_config_fails_closed_when_credential_missing`（manager/tests.rs:143-186）：注入空 store + 断言 Configuration err
  - `mcp_config_service_set_remote_authorization_fails_closed_when_credential_store_fails`（config_and_server_lifecycle.rs:734-775）：FailingMcpCredentialStore + InMemoryMCPConfigStore + 断言磁盘不变 + X-Existing 保留
  - `runtime_server_config_fails_closed_when_credential_missing` 同上
- 测试隔离并发：4 个测试均使用 `lock_global_credential_store_for_test` 持锁跨 `.await`（`#[tokio::test]` current-thread 运行时，guard 非 Send 跨 await 在 current-thread 下安全）—— 防 flaky 已落实

### god-file 观测点

- **PASS** ✅
- 16 改动文件均非 god-file（`scripts/rot-budget.json` 登记的 800+ 行 god-files 包括 `lsp/manager.rs` 等 9 个文件，本次 diff 未触及其中任一）
- 本单改动最大的 `lifecycle.rs`（impl MCPServerManager 内单 impl block）从 18 行扩到 45 行，在 god-file 阈值（800 行）之下，无需登记

### 实现者声明的 2 处续审修复核查

- **PASS** ✅
- ① **cli adapter 路径（设计层）**：原方案 `use northhing_runtime_ports::` —— 实证 cli `Cargo.toml:14` 仅 `northhing-core`（features `product-full`），无 `northhing-runtime-ports` 直接依赖（cli/Cargo.toml 不在 allowlist，加依赖越界）。修复改走 core lib.rs:35 `pub use northhing_runtime_ports as runtime_ports;` facade —— 验证 `northhing_core::runtime_ports::McpCredentialStore` 可解析。✅ 修复正确
- ② **core 测试杜撰 API（设计层）**：原方案 `ConfigService::new_isolated_for_test(None)` —— 实证全仓无此 API（`rg new_isolated_for_test` 0 matches）。修复改用仓内先例 `ConfigService::with_settings(ConfigManagerSettings { path_manager: Some(...), ... })` —— 验证 `service/config/service.rs:59` 的 `pub async fn with_settings`、`path_manager.rs:148` 的 `pub(crate) fn with_user_root_for_tests`、`manager.rs:38` 的 `pub struct ConfigManagerSettings` 三个 API 真实存在并签名前向兼容。✅ 修复正确
- 这两处修复后代码均通过 brief §6 验证红线（cargo check / cargo test），且 commit sequence 在 `03ecda0` 内完整

### 防腐与质量红线

- **PASS** ✅
- 无新增 production `unwrap`/`expect`/`panic`
- 无新 unsafe
- 日志英文无 emoji（`warn!` / `info!` 用 `tracing`，英文 message，无 emoji 字符）
- i18n：v0.1.0 frozen 不适用
- 分层合规：port 在 runtime-ports（contracts L6）；credentials 全局在 core/infrastructure/credentials.rs（assembly L5 与 core AGENTS §infrastructure 提示一致）；adapter 在 desktop/CLI（interfaces L1）；零跨界向上依赖
- 平台边界：无 host-specific API（`tauri::AppHandle` 等）漏入
- 远程兼容：v1 不引入远程特殊路径（MCP 服务运行时拉真值的解析点为本地内存副本，等价）

### 路径卫生

- **PASS** ✅
- 16 文件全部在 allowlist 18 行内（`src/apps/cli/src/keyring_keys.rs` 等已在名；`tests/common/mod.rs` 也在名）
- 加 sibling `w26-{1,2,3,4}` 领地（`kernel_facade/events.rs`、`cli/modes/chat/run.rs`、`cli/modes/exec.rs`、`desktop main.rs`、`lsp/manager.rs` 等）未触碰
- 隔离验证树 worktree 用完 `git worktree remove` 清理（report §5 已声明）—— 宿主树 `git status` 仅含 W26-2/3/4 兄弟未提交产物（与本单无关，编排者背景已注明）

---

## Findings

无。

- Critical：0
- Important：0
- Minor：0

可选观察（非 blocking、不入 findings）：

- 编排者收口时建议在 P1-8 ledger 行追加 `partial（W26-5, 2026-09-26）` 注记（report §4 二条已给出文案），本单不动。
- `pnpm run fmt:rs` 在并行树下被禁用（避免 `git restore` 污染兄弟产物），impl 已注明此为操作级声明；终审若单测稳定，可单独唤起 `rustfmt --edition 2021` 对 14 个 `.rs` 局部格式化。

---

## Cannot verify from diff

> 仅列无法从 diff 与报告输出独立确证的项。

1. **§6.1 `cargo check --workspace` 实际最终输出**：本次委派按 brief §6.1 命令组合与本单 16 文件清单均落在 allowlist 内，但完整原始输出文本较长（仅 §3.1 给出 `0 error / Finished dev profile` 摘要）—— **不可独立复现全量 stderr 文本**（报告文本是证据，命令输出中 `0 error` 与计时数据按 §3.1 ✓ 互相吻合）。
2. **W26-1 commit 插队的 `bf1df10` 在分隔 `03ecda0..92fa7a4` 报告 allowlist 提示「unfulfilled」**：report §3.8 已分窗口取证，code window `7610de5..03ecda0` exit 0 / doc window `bf1df10..<docs commit>` exit 0；连续窗口因 bf1df10 越界（兄弟 W26-1 文件）exit 1 —— 编排者建议「分窗口证据复跑/重钉 BASE」，本单基线已采信。
3. **未跑 `pnpm run fmt:rs`**：报告 §4 四条声明属观察级，可信，因 `scripts/format-changed-rust.mjs` 在并行树下确实会污染兄弟产物（编排者 2026-09-04 卫生立法）。
4. **隔离验证树 worktree 清理状态**：宿主 `git status` 不显示隔离树条目可验证，但隔离树路径 `C:\Windows\Temp\opencode\w26-5-verify` 是否已被实际 `git worktree remove` 清掉的端到端验证未现场复跑 —— 报告 §5 声明清理完成。

---

## 范围变动

无。

- 16 code 文件 + 2 doc/commit（`w26-5-brief.md` 仅 3 处 BASE re-pin 更新，`w26-5-report.md` 与 `w26-5-allowlist.txt` 新增） + 1 docs commit —— 全部对应 allowlist 18 行内
- 0 越界文件，未触禁区（S6 / W26-1/2/3/4 领地 / `metaRatchetPaths`）

---

## 终判

**APPROVE_WITH_CONCERNS** —— W26-5 三窗口证据链条对称、impl 与 brief 逐字对齐、2 处续审修复正确落实、双 surface adapter 已注册、测试真实执行且无 early-exit；唯一未尽事项为编排者口径下 P1-8 ledger 收口注记，非本单范围。
