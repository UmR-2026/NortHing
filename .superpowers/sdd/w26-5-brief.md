# W26-5 Brief — P1-8 CredentialStore port + set_remote_authorization 接线（单 judge）

## 1. 来源与验收标准（逐字）

Plan §1：
> "W26-5 P1-8 CredentialStore port + update_remote_authorization 接线 [单 judge]"

用户拍板（2026-09-09）："P1-8 修自己那一处（update_remote_authorization 走 CredentialStore port）"。**范围只到这一处**：MCP env 明文（stdio）与存量数据迁移不在本单（C2 论据的更大范围缓办在案）。

名称订正（侦察实证）：代码中无 `update_remote_authorization`，实际函数 = `set_remote_authorization` / `clear_remote_authorization`（services-integrations/mcp/config/service.rs:176/:204 + core 包装 :101/:112）。

### 验收标准（机械可核对）

- AC1: 新 port `CredentialStore` 落在 `src/crates/contracts/runtime-ports/src/credentials.rs`（新文件，lib.rs 增 mod 导出）：
  ```rust
  #[async_trait::async_trait]
  pub trait CredentialStore: Send + Sync {
      async fn store(&self, account: &str, secret: &str) -> PortResult<()>;
      async fn get(&self, account: &str) -> PortResult<Option<String>>;
      async fn delete(&self, account: &str) -> PortResult<()>;
  }
  ```
  同文件提供 `NullCredentialStore`（store/delete → Err 不可用语义；get → Ok(None)）与哨兵常量 `MCP_AUTH_SENTINEL: &str = "__kr_mcp_auth__"`（与 desktop 既有 `__kr__`/`__kr_env__` 哨兵命名同族）与账号键函数 `mcp_remote_authorization_account(server_id) -> String`（格式 `mcp.remote.{server_id}.authorization`，唯一定义点）。
- AC2: `services-integrations` 的 `MCPConfigService::new` 增第二参数 `credential_store: Arc<dyn CredentialStore>`；全部构造点适配（core 包装 + 12 处测试点，测试用 NullCredentialStore 或就地内存 double）。
- AC3: `set_remote_authorization` 改为：normalize → `credential_store.store(account, normalized)`（失败 → 直接 Err，磁盘不动）→ 清 headers/env 里的授权键 → `headers["Authorization"] = MCP_AUTH_SENTINEL` → save_server_config 落盘（磁盘只有哨兵）。`clear_remote_authorization`：先尝试 store.delete（失败仅 warn 并继续——磁盘明文残留比 keyring 孤儿更糟）→ 清键 → 落盘。
- AC4: 运行时解析（挂点 A）：`assembly/core` 的 `runtime_server_config`（mcp/server/manager/lifecycle.rs:4 区）读配置后，若 `headers["Authorization"] == MCP_AUTH_SENTINEL` → 经凭据存储取真值替换**内存副本**（Ok(Some)）；取不到（None/Err）→ fail-closed 返回 Err（消息指明 server_id 与凭据不可用语义，不打使命头真值）。下游 process/connection/transport 零签名改动。
- AC5: 生产注册：desktop 在启动 keyring 同步路径（app_state/settings/sync.rs）注册 adapter（adapter impl 放 keyring.rs，委托既有 ProductionKeyring）；cli 在 main.rs 的 `push_keyring_keys_into_core` 邻近处注册（adapter 放 keyring_keys.rs）。注册失败/未注册时 NullCredentialStore 兜底（set 报错可用语义，resolve 遇哨兵报错）。
- AC6: 测试：services-integrations 既有断言更新（config_and_server_lifecycle.rs:43-47 现断言磁盘含明文 "Bearer plain-token"——改为断言磁盘为哨兵且 store 内为真值）+ 新增：store 失败 → set 整体 Err 且磁盘不变；clear 后磁盘与 store 均清；core 侧解析：哨兵→真值替换、缺凭据→Err。
- AC7: §6 验证全绿。**台账 P1-8 不动**（部分修复，编排者收口时加注记）。
- AC8: 存量迁移不做（已有明文 Authorization 维持原样、正常运行；下次 set 时自然转哨兵）——report 声明。

## 2. 编排者预检结论（逐项钉死，直接采信）

BASE = `9222bbc42c2b0a13961dd8e63686c3c804abe698`。

| 事实 | 证据 |
|---|---|
| port 先例形态：runtime-ports/src/mcp.rs:75-81（McpCatalogReader）、session_workspace.rs:518-521（PermissionPort: RuntimeServicePort）；PortResult 来自 port_core | 侦察实证 |
| `MCPConfigService::new(config_store: Arc<dyn MCPConfigStore>)`（services-integrations service.rs:37）；生产唯一构造点 = core 包装 service/mcp/config/service.rs:79-87；12 处测试构造点 | 侦察实证 |
| core 包装 = 组合+纯委托；MCPService::new 构造它（service/mcp/mod.rs:51） | 侦察实证 |
| 解析挂点 A：lifecycle.rs 的 runtime_server_config 是 start_server 的统一漏斗（:152 调它、:247 proc.start_remote 前） | 侦察实证 |
| 全局单例先例：set_global_mcp_service / kernel_facade 全局；core 有 infrastructure/keyring.rs（KEYRING_SERVICE 常量） | 侦察实证 |
| desktop 哨兵先例：API_KEY_SENTINEL `__kr__` + sync.rs push_resolved_keys_to_core（fail-closed 跳过语义） | 侦察实证 |
| 落盘路径：cursor_format.rs:69-71 headers 明文；save_user_config :239-247 | 侦察实证 |
| `set_remote_authorization` 生产零调用方（仅测试）——注册时序 race 对 v1 无害（哨兵不可能存在于升级前数据） | 侦察实证 |
| 非 meta-ratchet → 单 judge；但 runtime-ports 新增模块文件须过 core-boundaries 检查（若红，BLOCKED 上报，不得自扩范围改闸） | workflow-policy.json + 仓规 |
| 仓规：core 定向测试须 `--features product-full` | AGENTS.md |

## 3. 复用侦察（强制）

- port 形态复用 runtime-ports 先例（async_trait + PortResult）；哨兵模式复用 desktop `__kr__` 先例；全局注册复用 set_global_* 先例；win 侧 keyring 复用 ProductionKeyring（desktop）与 keyring_keys（cli）。
- report 必须有「复用侦察」节。无此节 = 未完成。

## 4. Spec（必须全部满足）

- S1: credentials.rs（AC1 形状）+ lib.rs 导出。
- S2: services-integrations service.rs 构造签名与 set/clear 语义（AC2/AC3）；测试适配与新增（AC6 前两条）。
- S3: core 全局：`src/crates/assembly/core/src/infrastructure/credentials.rs`（新）——`set_global_credential_store(Arc<dyn CredentialStore>)` + `global_credential_store() -> Option<...>`（OnceLock/RwLock 模式照仓内既有全局先例）；infrastructure mod 注册。
- S4: core 接线：MCPConfigService 包装层 `new` 把 `global_credential_store().unwrap_or(Null)` 传入 inner；manager lifecycle 的 runtime_server_config 加哨兵解析（AC4）。
- S5: desktop/cli 注册与 adapter（AC5）。
- S6: 禁区：kernel-api、ci.yml、scripts/ 全部、workflow-policy.json、gate-registry.json；W26-1/2/3/4 领地（kernel_facade/events.rs、cli modes/chat/run.rs、cli modes/exec.rs、dialog_turn.rs、turn_lifecycle.rs、save.rs、turn_persist.rs、desktop main.rs、lsp/manager.rs）；cursor_format.rs 不动（哨兵透明穿透）；transport_remote.rs 不动。
- S7: 不做：env 哨兵化、存量迁移、UI/命令入口（set_remote_authorization 的生产调用方本单不接——它至今零生产调用方，接线 UI 属后续单）。
- S8: unsafe 无；async trait 内同步 keyring 调用可接受（desktop 既有先例同步调用），注释标注。

## 5. Global Constraints（逐字遵守）

- 真值不落盘（磁盘只许哨兵）；真值不进日志。
- fail-closed：store 失败不盘、缺凭据不启动（报错语义清晰）。
- serde/磁盘格式兼容：旧数据（无明文=无授权/有明文=照跑）零迁移。
- 禁整树 git 操作；只点名 add/commit。
- 测试真实执行，输出原文进 report；路径卫生（D-2）；cargo 带 rustup 前缀。
- 日志英文无 emoji。

## 6. 验证（命令 + 输出原文进 report）

BASE 全 sha = `9222bbc42c2b0a13961dd8e63686c3c804abe698`（下文 `<BASE>`）。

1. `rustup run stable-x86_64-pc-windows-msvc cargo check --workspace` → 0 error
2. `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-services-integrations` → 绿（含更新后的断言与新用例）
3. `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full mcp` → 绿（core 解析路径用例）
4. `rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing` 与 `cargo check -p northhing-cli` → 0 error
5. `node scripts/check-core-boundaries.mjs` → exit 0（runtime-ports 新模块过闸证据；红则 BLOCKED）
6. `node scripts/check-repo-hygiene.mjs` → exit 0
7. 红态探针（commit 后执行）：临时把 disk 上的 headers 哨兵值改回明文跑一次 core 解析路径测试思路的负例已在 AC6 覆盖，此处不重复——本单免探针（测试即红态证据）。
8. `node scripts/verify-task-gate.mjs verify-attempt --base <BASE> --tip <本单最后 commit sha> --allowlist .superpowers/sdd/w26-5-allowlist.txt` → exit 0（自建 allowlist 含自身；**窗口内若出现其它 W26 单文件，停手报编排者复跑**）

## 7. 报告

路径 `.superpowers/sdd/w26-5-report.md`。章节：改动摘要 / 复用侦察 / 验证 / 疑虑 / 状态。

## 8. 派发元信息

- BASE: `9222bbc42c2b0a13961dd8e63686c3c804abe698`
- 允许文件集：
  - `src/crates/contracts/runtime-ports/src/credentials.rs`（新建）+ `src/crates/contracts/runtime-ports/src/lib.rs`
  - `src/crates/services/services-integrations/src/mcp/config/service.rs`
  - `src/crates/services/services-integrations/tests/` 下既有或新测试文件
  - `src/crates/assembly/core/src/infrastructure/credentials.rs`（新建）+ 其 mod 注册文件
  - `src/crates/assembly/core/src/service/mcp/config/service.rs`、`src/crates/assembly/core/src/service/mcp/mod.rs`、`src/crates/assembly/core/src/service/mcp/server/manager/lifecycle.rs`
  - `src/apps/desktop/src/app_state/settings/keyring.rs`、`src/apps/desktop/src/app_state/settings/sync.rs`
  - `src/apps/cli/src/keyring_keys.rs`、`src/apps/cli/src/main.rs`
  - `.superpowers/sdd/w26-5-brief.md` / `w26-5-report.md` / `w26-5-allowlist.txt`
- 禁区：§4-S6 + 其它未列出文件。
- commit 规则：点名 git add；前缀 `feat(mcp): W26-5`，body 注「用户 2026-09-09 拍板 P1-8 端口方案」；允许多 commit。
- 并行声明：W26-1/2/3/4 同波全并行，文件集不相交（desktop main.rs 是 W26-3 领地，本单 desktop 注册走 sync.rs 避开）。同工作树 cargo 锁等待可容忍。

