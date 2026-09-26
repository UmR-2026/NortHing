# W26-5 Brief — P1-8 CredentialStore port + set_remote_authorization 接线（单 judge）

## 1. 来源与验收标准（逐字）

Plan §1：
> "W26-5 P1-8 CredentialStore port + update_remote_authorization 接线 [单 judge]"

用户拍板（2026-09-09）："P1-8 修自己那一处（update_remote_authorization 走 CredentialStore port）"。**范围只到这一处**：MCP env 明文（stdio）与存量数据迁移不在本单（C2 论据的更大范围缓办在案）。

名称订正（侦察实证）：代码中无 `update_remote_authorization`，实际函数 = `set_remote_authorization` / `clear_remote_authorization`（services-integrations/mcp/config/service.rs:176/:204 + core 包装 :101/:112）。

### 验收标准（机械可核对）

- AC1: 新 port `McpCredentialStore` 落在 `src/crates/contracts/runtime-ports/src/credentials.rs`（新文件，lib.rs 增 mod 导出）：
  ```rust
  #[async_trait::async_trait]
  pub trait McpCredentialStore: Send + Sync {
      async fn store(&self, account: &str, secret: &str) -> PortResult<()>;
      async fn get(&self, account: &str) -> PortResult<Option<String>>;
      async fn delete(&self, account: &str) -> PortResult<()>;
  }
  ```
  **命名纪律（复审 I2）**：仓内 `rmcp::transport::auth::CredentialStore` 已被 services-integrations/mcp/auth.rs:15 与 core/service/mcp/auth.rs:7 导入使用，新 port 禁用裸名 `CredentialStore`（防 import 二义）。同文件提供 `NullMcpCredentialStore`（store/delete → Err 不可用语义；get → Ok(None)）与哨兵常量 `MCP_AUTH_SENTINEL: &str = "__kr_mcp_auth__"`（与 desktop 既有 `__kr__`/`__kr_env__` 哨兵命名同族）与账号键函数 `mcp_remote_authorization_account(server_id) -> String`（格式 `mcp.remote.{server_id}.authorization`，唯一定义点）。
- AC2: `services-integrations` 的 `MCPConfigService::new` 增第二参数 `credential_store: Arc<dyn McpCredentialStore>`；全部构造点适配（core 包装 + 12 处测试点，测试用 NullMcpCredentialStore 或就地内存 double）。**Cargo.toml 必须同步**：`src/crates/services/services-integrations/Cargo.toml` 的 `mcp` feature 列表加入 `"northhing-runtime-ports"`（该依赖已是 optional，:19；`deep-research` feature :56 已有同样加法先例；crate-rules.mjs:438 仅禁非可选依赖，feature 内启用合法）。缺此步默认/单 `mcp` feature 编译即 E0432/E0433。
- AC3: `set_remote_authorization` 改为：normalize → `credential_store.store(account, normalized)`（失败 → 直接 Err，磁盘不动）→ 清 headers/env 里的授权键 → `headers["Authorization"] = MCP_AUTH_SENTINEL` → save_server_config 落盘（磁盘只有哨兵）。save 失败时 best-effort 调 `credential_store.delete(account)` 回滚（不保证成功；失败仅 warn，不改错误返回）。`clear_remote_authorization`：先尝试 store.delete（失败仅 warn 并继续——磁盘明文残留比 keyring 孤儿更糟）→ 清键 → 落盘。
- AC4: 运行时解析（挂点 A）：`assembly/core` 的 `runtime_server_config`（mcp/server/manager/lifecycle.rs）读配置后，若 `headers["Authorization"] == MCP_AUTH_SENTINEL` → 经 `global_credential_store()`（调用时解析，见 S3/S4）取真值替换**内存副本**（Ok(Some)）；取不到（None/Err）→ fail-closed 返回 Err（消息指明 server_id 与凭据不可用语义，不打使命头真值）。下游 process/connection/transport 零签名改动。注意 `lifecycle.rs:533`（reauthenticate_remote_server）/`:549`（clear_remote_server_auth）两条生产 API 路径经 config_service 调 set/clear——本单语义变更同时覆盖该调用栈（二者自身全仓零 caller，链式调用为零）。
- AC5: 生产注册（**调用时解析 + 注册时序无关设计**，见 S3/S4）：core 侧 `infrastructure/credentials.rs` 提供 `set_global_credential_store(Arc<dyn McpCredentialStore>)` + `global_credential_store() -> Option<Arc<dyn McpCredentialStore>>` + 代理 `GlobalCredentialStore`（impl McpCredentialStore，每次调用委派当前全局，全局为 None 时按 Null 语义）。desktop 在 `ui_dioxus/entry.rs` 的 `launch()` 起始处注册 adapter（adapter impl 放 keyring.rs，委托既有 ProductionKeyring via PRODUCTION_KEYRING）；cli 在 main.rs:381 `push_keyring_keys_into_core()` 调用邻近处注册（adapter 放 keyring_keys.rs，复用其既有 keyring 函数）。注册失败/未注册时 Null 兜底（set 报错可用语义，resolve 遇哨兵报错）。**注册时序 race 对 v1 无害**（哨兵不可能存在于升级前数据 + set 生产零 caller），代理设计使未来 UI 接线单无需关心注册早于 init_core。
- AC6: 测试：services-integrations 既有断言更新（config_and_server_lifecycle.rs:43-47 现断言磁盘含明文 "Bearer plain-token"——改为断言磁盘为哨兵且 store 内为真值）+ 新增：store 失败 → set 整体 Err 且磁盘不变；clear 后磁盘与 store 均清；core 侧解析：哨兵→真值替换、缺凭据→Err。
- AC6b: services-integrations 测试**必须带 `--features mcp` 运行**（tests/config_and_server_lifecycle.rs:1 `#![cfg(feature = "mcp")]`，裸跑 0 tests——W15-1f 已实证同形态）。M2 回滚路径（store 成功 + save 失败 → best-effort delete）以 report 疑虑节声明覆盖，不强制新增用例。
- AC7: §6 验证全绿。**台账 P1-8 不动**（部分修复，编排者收口时加注记）。
- AC8: 存量迁移不做（已有明文 Authorization 维持原样、正常运行；下次 set 时自然转哨兵）——report 声明。

## 2. 编排者预检结论（逐项钉死，直接采信）

BASE = `831094c`。

| 事实 | 证据 |
|---|---|
| port 先例形态：runtime-ports/src/mcp.rs:75-81（McpCatalogReader）、session_workspace.rs:518-521（PermissionPort: RuntimeServicePort）；PortResult 来自 port_core | 侦察实证 |
| `MCPConfigService::new(config_store: Arc<dyn MCPConfigStore>)`（services-integrations service.rs:37）；生产唯一构造点 = core 包装 service/mcp/config/service.rs:79-87；12 处测试构造点 | 侦察实证 |
| core 包装 = 组合+纯委托；MCPService::new 构造它（service/mcp/mod.rs:51） | 侦察实证 |
| 解析挂点 A：lifecycle.rs 的 runtime_server_config 是 start_server 的统一漏斗（:152 定义 start_server，:156 调它、:247 proc.start_remote 前） | 侦察实证（行号经复审订正 :152→:156） |
| 全局单例先例：set_global_mcp_service / kernel_facade 全局；core 有 infrastructure/keyring.rs（KEYRING_SERVICE 常量） | 侦察实证 |
| desktop 哨兵先例：API_KEY_SENTINEL `__kr__` + sync.rs push_resolved_keys_to_core（fail-closed 跳过语义；注意该函数生产零 caller——仅 tests.rs:369，故注册点不能落 sync.rs，落 ui_dioxus/entry.rs launch() 起始处，复审 C3） | 侦察实证 |
| 落盘路径：cursor_format.rs:69-71 headers 明文；save_user_config :239-247 | 侦察实证 |
| `set_remote_authorization` 生产链式零调用方：:533 reauthenticate_remote_server / :549 clear_remote_server_auth 两条生产 API 路径暴露 set/clear，但二者自身全仓零 caller——注册时序 race 对 v1 无害（哨兵不可能存在于升级前数据） | 侦察实证（措辞经复审订正） |
| 非 meta-ratchet → 单 judge；但 runtime-ports 新增模块文件须过 core-boundaries 检查（若红，BLOCKED 上报，不得自扩范围改闸） | workflow-policy.json + 仓规 |
| 仓规：core 定向测试须 `--features product-full` | AGENTS.md |

## 3. 复用侦察（强制）

- port 形态复用 runtime-ports 先例（async_trait + PortResult）；哨兵模式复用 desktop `__kr__` 先例；全局注册复用 set_global_* 先例；win 侧 keyring 复用 ProductionKeyring（desktop）与 keyring_keys（cli）。
- report 必须有「复用侦察」节。无此节 = 未完成。

## 4. Spec（必须全部满足）

- S1: credentials.rs（AC1 形状）+ lib.rs 导出。
- S2: services-integrations service.rs 构造签名与 set/clear 语义（AC2/AC3）；测试适配与新增（AC6 前两条）。
- S3: core 全局：`src/crates/assembly/core/src/infrastructure/credentials.rs`（新）——`set_global_credential_store(Arc<dyn McpCredentialStore>)` + `global_credential_store() -> Option<Arc<dyn McpCredentialStore>>` + 代理结构 `GlobalCredentialStore`（`impl McpCredentialStore`，三个方法均调用时读取当前全局；全局 None → 按 NullMcpCredentialStore 语义：store/delete → Err 不可用、get → Ok(None)）。OnceLock/RwLock 模式照仓内既有全局先例。放 infrastructure 的依据：credential 跨 mcp/api 共享，keyring 服务常量已在 infra/keyring.rs（KEYRING_SERVICE）。infrastructure mod 注册。
- S4: core 接线：MCPConfigService 包装层 `new` 构造 inner 时传 `Arc::new(GlobalCredentialStore::default())` 作为第二参（**调用时解析**，注册时序无关）；manager lifecycle 的 runtime_server_config 加哨兵解析（AC4，同样调用时读 `global_credential_store()`）。
- S5: desktop/cli 注册与 adapter（AC5）。
- S6: 禁区：kernel-api、ci.yml、scripts/ 全部、workflow-policy.json、gate-registry.json；W26-1/2/3/4 领地（kernel_facade/events.rs、cli modes/chat/run.rs、cli modes/exec.rs、dialog_turn.rs、turn_lifecycle.rs、save.rs、turn_persist.rs、desktop main.rs、lsp/manager.rs）；cursor_format.rs 不动（哨兵透明穿透）；transport_remote.rs 不动。
- S7: 不做：env 哨兵化、存量迁移、UI/命令入口（set/clear 经 lifecycle.rs:533/:549 生产 API 路径暴露但链式零 caller；接线 UI 属后续单）。
- S8: unsafe 无；async trait 内同步 keyring 调用可接受（desktop 既有先例同步调用），注释标注。

## 5. Global Constraints（逐字遵守）

- 真值不落盘（磁盘只许哨兵）；真值不进日志。
- fail-closed：store 失败不盘、缺凭据不启动（报错语义清晰）。
- serde/磁盘格式兼容：旧数据（无明文=无授权/有明文=照跑）零迁移。
- 禁整树 git 操作；只点名 add/commit。
- 测试真实执行，输出原文进 report；路径卫生（D-2）；cargo 带 rustup 前缀。
- 日志英文无 emoji。

## 6. 验证（命令 + 输出原文进 report）

BASE 全 sha = `831094c`（下文 `<BASE>`）。

1. `rustup run stable-x86_64-pc-windows-msvc cargo check --workspace` → 0 error
2. `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-services-integrations --features mcp` → 绿（含更新后的断言与新用例；**必须带 `--features mcp`**，裸跑测试被 cfg-out 为 0 tests，W15-1f 实证）
3. `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full mcp` → 绿（core 解析路径用例）
4. `rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing` 与 `cargo check -p northhing-cli` → 0 error
5. `node scripts/check-core-boundaries.mjs` → exit 0（runtime-ports 新模块过闸证据；红则 BLOCKED）
6. `node scripts/check-repo-hygiene.mjs` → exit 0
7. 红态探针（commit 后执行）：临时把 disk 上的 headers 哨兵值改回明文跑一次 core 解析路径测试思路的负例已在 AC6 覆盖，此处不重复——本单免探针（测试即红态证据）。
8. `node scripts/verify-task-gate.mjs verify-attempt --base <BASE> --tip <本单最后 commit sha> --allowlist .superpowers/sdd/w26-5-allowlist.txt` → exit 0（自建 allowlist 含自身；**窗口内若出现其它 W26 单文件，停手报编排者复跑**）

## 7. 报告

路径 `.superpowers/sdd/w26-5-report.md`。章节：改动摘要 / 复用侦察 / 验证 / 疑虑 / 状态。

## 8. 派发元信息

- BASE: `831094c`
- 允许文件集：
  - `src/crates/contracts/runtime-ports/src/credentials.rs`（新建）+ `src/crates/contracts/runtime-ports/src/lib.rs`
  - `src/crates/services/services-integrations/Cargo.toml`（**仅限** `mcp` feature 列表加 `"northhing-runtime-ports"` 一行）
  - `src/crates/services/services-integrations/src/mcp/config/service.rs`
  - `src/crates/services/services-integrations/tests/` 下既有或新测试文件
  - `src/crates/assembly/core/src/infrastructure/credentials.rs`（新建）+ 其 mod 注册文件
  - `src/crates/assembly/core/src/service/mcp/config/service.rs`、`src/crates/assembly/core/src/service/mcp/mod.rs`、`src/crates/assembly/core/src/service/mcp/server/manager/lifecycle.rs`
  - `src/crates/assembly/core/src/service/mcp/server/manager/tests.rs`（core 侧解析用例可选落点：lifecycle.rs 内 `#[cfg(test)] mod tests` 或本文件，二选一）
  - `src/apps/desktop/src/app_state/settings/keyring.rs`、`src/apps/desktop/src/app_state/settings/sync.rs`
  - `src/apps/desktop/src/ui_dioxus/entry.rs`（**仅限** `launch()` 函数起始处加注册调用 + 注释；desktop main.rs 属 W26-3 领地，sync.rs 无生产 caller——侦察实证 push_resolved_keys_to_core 仅 tests.rs:369 调用）
  - `src/apps/cli/src/keyring_keys.rs`、`src/apps/cli/src/main.rs`（注册点 :381 push_keyring_keys_into_core 邻近）
  - `.superpowers/sdd/w26-5-brief.md` / `w26-5-report.md` / `w26-5-allowlist.txt`
- 禁区：§4-S6 + 其它未列出文件。
- commit 规则：点名 git add；前缀 `feat(mcp): W26-5`，body 注「用户 2026-09-09 拍板 P1-8 端口方案」；允许多 commit。
- 并行声明：W26-1/2/3/4 同波全并行，文件集不相交（desktop main.rs 是 W26-3 领地——本单 desktop 注册调用落 ui_dioxus/entry.rs 的 launch()，adapter 落 settings/keyring.rs，两者均非 W26-3 领地）。同工作树 cargo 锁等待可容忍。


## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。

