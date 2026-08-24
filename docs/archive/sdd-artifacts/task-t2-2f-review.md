# Task Review T2-2f — contracts 修剪（remote_connect-era 契约删除）

## Verdict: **PASS**

## 总结

实施者严格按 brief 11 项清单执行 contracts 修剪，授权 6 项 wire 词汇完整删除、4 项 capability 变体归零、boundary 规则同步、`surface_contracts.rs`/`runtime_services_contracts.rs` 测试用剩余等价变体替代（语义等价）。SSH 语义、DialogTriggerSource::RemoteRelay/Bot、`session_workspace.rs` 其余内容、其它 contracts 全部零损伤。验证门槛 5/5 全绿，归零检查精确，无新增编译器警告。

---

## 双判决一：Spec 合规

### Constraint 验证

| Constraint | 结果 | 证据 |
|---|---|---|
| 1. 清单外 contracts 零改动 | ✅ | `DialogTriggerSource::{RemoteRelay,Bot}` 保留（`agent_facade_tests.rs:56,434`、`agent/agent_dialog.rs:62`）；`ThreadEnvironmentKind::RemoteSsh` 保留（`core-types/src/surface.rs:25`）；`ThreadEnvironment.remote_connection_id` 字段保留（`surface.rs:37`）；`session_workspace.rs:1-538` 仅 :544 trait + 其 4 行 doc 块删除，远程 SSH 模块 + `SessionStoragePathResolution::remote()` 等全部保留（`session_workspace.rs:18,38,44,68,74,...`） |
| 2. 授权 wire 删除仅 6 项 | ✅ | `rg "SurfaceKind::Remote\b|ThreadEnvironmentKind::RemoteConnect|RuntimeServiceCapability::Remote"` 全 0 命中；`rg "RemoteConnectionPort\|RemoteWorkspacePort\|RemoteProjectionPort\|RemoteCapabilityPort\|RemoteWorkspaceKind\|RemoteInitialSyncRuntimeHost\|RemoteWorkspaceFileRuntimeHost\|RemoteWorkspaceRuntimeHost"` 全 0 命中 |
| 3. SSH 语义零改动 | ✅ | `remote_ssh` 模块、`RemoteWorkspaceEntry`（C5-C8 范围外）、`remote_connection_id`、`remote_ssh_host`、`SessionStorageKind::Remote`、`SessionStoragePathResolution::remote()` 等 SSH/事实型字段 100% 保留；`tests/runtime_services_contracts.rs:107-108` SSH 字段（`remote_connection_id: None, remote_ssh_host: None`）原样保留 |
| 4. 只动任务书 11 项清单 + boundary 脚本 | ✅ | `git diff --name-only 65145cf -- src scripts` = 13 文件 = 11 source 文件（runtime-ports 4 / core-types 2 / runtime-services 3 / assembly 1 + remote.rs 删除）+ 2 boundary 脚本（brief §3 第 4 条授权）。`memory/`、`.opencode/`、`docs/` 改动属工作树残留（早于本次时间戳），非本任务改动 |
| 5. 验证门槛 MSVC 全绿 | ✅ | 见下方"原始验证" |

### 审查要点逐条验证

#### (a) 6 项 wire 词汇删除确认

| Wire 词汇 | 原位置 | 序列化字符串 | 删除证据 |
|---|---|---|---|
| `SurfaceKind::Remote` | `core-types/src/surface.rs:16` | `"remote"` | ✅ diff -U10 显示 `Remote` 整行删除；当前文件仅剩 `Desktop,Cli,Acp,Server`；`surface_contracts.rs` 测试断言替换为 `Server`/`"server"` |
| `ThreadEnvironmentKind::RemoteConnect` | `core-types/src/surface.rs:27` | `"remote_connect"` | ✅ diff -U10 显示 `RemoteConnect` 整行删除；当前文件仅剩 `Local,Worktree,RemoteSsh,CloudLike,Acp`；`surface_contracts.rs:64,73` 测试断言替换为 `RemoteSsh`/`"remote_ssh"` |
| `RuntimeServiceCapability::RemoteConnection` | `runtime-ports/src/port_core.rs:58` | `"remote_connection"` | ✅ 变体 + 对应 as_str match 臂 `"remote_connection"` 同时删除；`has_capability` 与 builder/build() 同步清理 |
| `RuntimeServiceCapability::RemoteWorkspace` | `runtime-ports/src/port_core.rs:59` | `"remote_workspace"` | ✅ 同上模式 |
| `RuntimeServiceCapability::RemoteProjection` | `runtime-ports/src/port_core.rs:60` | `"remote_projection"` | ✅ 同上模式 |
| `RuntimeServiceCapability::RemoteCapabilities` | `runtime-ports/src/port_core.rs:61` | `"remote_capabilities"` | ✅ 同上模式 |

#### (b) 11 项 file 清单逐项核对

| # | 文件 | 期望 | 实际 | 结果 |
|---|---|---|---|---|
| 1 | `runtime-ports/src/remote.rs` | 整删（143 行） | `git diff` 显示 `D` 状态；`Test-Path` 返回 False；`git ls-files --stage` 仍记录 hash（索引未暂存删除） | ✅ |
| 2 | `runtime-ports/src/lib.rs` | 删 `pub mod remote;` + `pub use remote::*;` + 顶部 doc "remote" 提及 | 当前文件 `:14` (mod) 与 `:28` (use) 已无 remote；顶部 doc :7-9 由 4 sibling 改为 3 sibling | ✅ |
| 3 | `runtime-ports/src/session_workspace.rs:544` | 删 `RemoteConnectionPort` + 其 doc 注释块 | 当前文件 :527-538 仅含 `TerminalPort`/`NetworkPort`/`GitPort`/`McpCatalogPort`（含其 doc）；末尾无 RemoteConnectionPort | ✅ |
| 4 | `runtime-ports/src/port_core.rs` | 删 4 变体 + 4 match 臂 | 当前文件 `:47-58` 枚举仅 10 变体；`:60-75` as_str 仅 10 match 臂 | ✅ |
| 5 | `runtime-ports/src/runtime_facade_tests.rs` | 删 remote 引用测试，保留其余 | 当前文件 :7-23,25-37,100-114,115-142 4 个测试；2 个 remote 测试整段删除；顶部 doc 由 "(remote + workspace)" 改为 "(workspace)" | ✅ |
| 6 | `core-types/src/surface.rs` | 删 `SurfaceKind::Remote` + `ThreadEnvironmentKind::RemoteConnect`，保留 `RemoteSsh` + `remote_connection_id` | 当前 :13-18 (SurfaceKind 4 变体)、:22-28 (ThreadEnvironmentKind 5 变体含 RemoteSsh)、:37 (`remote_connection_id`) 全部就位 | ✅ |
| 7 | `core-types/tests/surface_contracts.rs` | 删 RemoteConnect/Remote 断言，保留其余 | 3 测试均通过；`permission_and_capability_contracts_keep_source_identity` 用 `Server` 替代；`thread_environment_contract_does_not_require_surface_specific_fields` 用 `RemoteSsh` 替代 | ✅ |
| 8 | `runtime-services/src/lib.rs` | 删 4 registry 字段 + 4 match 臂 + 4 builder 字段 + 4 `with_optional_remote_*` + 4 build() 段 + 4 import | 当前文件 :6-9 import 仅 11 项无 remote；:33-44 结构体 10 字段；:46-61 Debug 10 .field()；:65-77 has_capability 10 match 臂；:96-107 Builder 10 字段；:109-162 含 4 个 with_optional_*（无 remote）；:164-177 build() 10 行 | ✅ |
| 9 | `runtime-services/src/test_support.rs` | 删 remote port impl + include_remote/with_all_remote | 当前文件 :3-8 import 无 remote 符号；:29-43 含 FileSystem/Workspace/SessionStore/Terminal/Network/Git/McpCatalog 7 个 trait impl；:68-99 `FakeRuntimeServicesProvider` 单元结构体（含 `Default` 派生），`with_all_required` 直接返回 `Self`，`register` 不需 `mut self` | ✅ |
| 10 | `runtime-services/tests/runtime_services_contracts.rs` | 删 remote 相关断言 | 当前 6 测试均通过；`fake_provider_registers_required_services_through_registry` 重命名 + 去除 4 remote capability 断言；`missing_optional_capability_*` 改用 Terminal；`capability_availability_*` 改用 Terminal；`registered_remote_ports_expose_owner_contract_methods` 整删 | ✅ |
| 11 | `assembly/core/tests/product_assembly.rs` | 删 2 条 remote capability 断言 | 当前 :26-29 仅含 4 个断言（Terminal/Network/Git/McpCatalog），2 条 RemoteWorkspace/RemoteProjection 已删除 | ✅ |

#### (c) port_core.rs / surface.rs 序列化属性在剩余变体上未被扰动

| 文件 | serde rename | 剩余变体 | 验证 |
|---|---|---|---|
| `port_core.rs:46-58` | `#[serde(rename_all = "snake_case")]` | `FileSystem, Workspace, SessionStore, Permission, Events, Clock, Terminal, Network, Git, McpCatalog` | ✅ 序列化字符串 `filesystem/workspace/session_store/permission/events/clock/terminal/network/git/mcp_catalog` 与 as_str 完全对应 |
| `surface.rs:11-18` | `#[serde(rename_all = "snake_case")]` | `Desktop, Cli, Acp, Server` | ✅ 替换断言 `json["source"]["surface"] == "server"` 通过 = 序列化行为未受扰动 |
| `surface.rs:21-28` | `#[serde(rename_all = "snake_case")]` | `Local, Worktree, RemoteSsh, CloudLike, Acp` | ✅ 替换断言 `json["kind"] == "remote_ssh"` 通过 |

#### (d) runtime-services lib.rs 三段删除后语义一致

| 段 | 删除前 | 删除后 | 验证 |
|---|---|---|---|
| Registry 字段（`RuntimeServices`） | 14 字段（含 4 remote optional） | 10 字段（4 required + 6 optional） | ✅ |
| Builder 字段（`RuntimeServicesBuilder`） | 14 字段 + 4 `with_optional_remote_*` | 10 字段 + `with_optional_{terminal,network,git,mcp_catalog}` | ✅ |
| `build()` 装配 | 14 行（含 4 remote optional） | 10 行 | ✅ |

`Debug` impl 与 `has_capability` 同步骤除，10 字段全部对应：`FileSystem/Workspace/SessionStore/Permission/Events/Clock` 必填，`Terminal/Network/Git/McpCatalog` optional。`validate_capability`/`required`/`required_service`/`optional_service` 三个内部 helper 100% 保留。

#### (e) test_support FakeRuntimeServicesProvider 移除后其它 fake 能力完整

- `include_remote: bool` 字段删除，`FakeRuntimeServicesProvider` 由 `pub struct FakeRuntimeServicesProvider { include_remote: bool }` 简化为 `pub struct FakeRuntimeServicesProvider;` 单元结构体。
- `with_all_remote()` 方法删除，`with_all_required()` 直接返回 `Self`（无 `mut self`）。
- `register()` 不再需 `mut self`，由 `let builder = builder.with_*()` 改为 `builder.with_*(...)` 流式链式调用（diff 显示末尾删除原 `let builder = builder.with_clock(clock); ... if !self.include_remote ...` 块并新增 `.with_clock(clock)` 在链末）。
- 其余 fake 能力完整：`FileSystem/Workspace/SessionStore/Permission/Events/Clock/Terminal/Network/Git/McpCatalog` 10 个 trait impl 全部保留（`test_support.rs:29-43`）。

#### (f) required-rules.mjs / self-test.mjs 编辑精度

| 文件 | 关键删除 | 当前文件 | 结果 |
|---|---|---|---|
| `required-rules.mjs` | 删除 4 个规则块 + 1 个测试名 rename | `rg "registered_remote_ports_expose_owner_contract_methods\|runtime-ports/src/remote\.rs\|RemoteWorkspaceFacts\|RemoteWorkspaceRuntimeHost\|RemoteWorkspacePort\|RemoteWorkspaceFileRuntimeHost\|RemoteProjectionPort\|RemoteInitialSyncRuntimeHost"` 在该文件 0 命中 | ✅ |
| `required-rules.mjs:50` | `fake_provider_registers_required_and_remote_services_through_registry` → `fake_provider_registers_required_services_through_registry` | diff -U10 显示精确 rename | ✅ |
| `self-test.mjs` | 删除 runtime-ports/src/remote.rs contracts 块（6 项）+ runtime_facade_tests.rs 2 项 + runtime_services_contracts.rs 1 项 + rename 1 项 | `rg` 上述锚点在 self-test.mjs 0 命中 | ✅ |

**SSH 锚点与未触碰模块的"remote"提及（属允许保留）**：
- `required-rules.mjs:5279` 提及 `src/crates/assembly/core/src/service/search/remote.rs`（搜索功能，**未在 diff 中**，属 C5-C8 范围）
- `forbidden-rules.mjs:626,1632`、`facade-rules.mjs:80`、`self-test.mjs:2073,2959,3051` 同样指向 `search/remote.rs` 或 `mcp/protocol/transport_remote.rs`（**全部未在 diff 中**，均非本任务范围）

#### (g) product_assembly.rs 2 条断言删除精度

- 原 `:30-31` `assert!(services.has_capability(RuntimeServiceCapability::RemoteWorkspace));` 与 `assert!(services.has_capability(RuntimeServiceCapability::RemoteProjection));` 精确删除。
- 第 3 测试 `core_provider_closes_current_product_full_service_capability_requirements` 通过 = product-full plan 自身不再要求 Remote* 能力（计划层 C1-C3 已同步）。

#### (h) 归零检查

| rg 命令 | 结果 |
|---|---|
| `rg -n "RemoteConnectionPort\|RemoteWorkspacePort\|RemoteProjectionPort\|RemoteCapabilityPort\|RemoteWorkspaceKind\|RemoteInitialSyncRuntimeHost\|RemoteWorkspaceFileRuntimeHost\|RemoteWorkspaceRuntimeHost" src --glob "*.rs"` | 0 命中 |
| `rg -n "SurfaceKind::Remote\b\|ThreadEnvironmentKind::RemoteConnect\|RuntimeServiceCapability::Remote" src --glob "*.rs"` | 0 命中 |

剩余 "remote" 词命中（已核实全部为允许保留）：
- SSH 语义：`RemoteSsh`、`remote_connection_id`、`remote_ssh_host`、`remote_image_attachment_serializes_portable_metadata_contract`（agent attachment，非 port）
- Dialog trigger source：`DialogTriggerSource::RemoteRelay`、`AgentSubmissionSource::RemoteRelay`（约束 #1 强制保留）
- SessionStore 事实型：`SessionStoragePathResolution::remote()`、`SessionStorageKind::Remote`、`is_remote_storage()`（属 `session_store.rs`，未在本任务范围）
- AI 配置型：`RemoteModelInfo`（`core-types/src/ai.rs:202`，未在本任务范围）
- 文档注释：`surface.rs:4` "not encode CLI, desktop, remote, ACP, or server presentation behavior"（描述性，未指 wire 词汇）

---

## 双判决二：代码质量

### 删除手术清洁度

| 检查项 | 结果 |
|---|---|
| 未引入新的 `cargo` 警告（对比 base 65145cf） | ✅ stash 测试：base = 19+5+1 warnings；after t2-2f = 同 19+5+1；零增量 |
| 无残留未使用 import | ✅ `test_support.rs:3-8` import 与 impl 一一对应；`runtime_services_contracts.rs:1-9` 仅引入仍在使用的符号 |
| 无 `mut self` 残留 | ✅ `FakeRuntimeServicesProvider::register` 不再需要 `mut self`（已无 `include_remote` 字段需修改） |
| 单元结构体 `Default` 派生 | ✅ `pub struct FakeRuntimeServicesProvider;` 配 `#[derive(Debug, Clone, Default)]` 正确（unit struct + Default 返回 `()`） |
| `#[serde(rename_all)]` 在剩余变体上未扰动 | ✅ 见 §(c) |

### 测试断言替换合理性（不属"顺手重构"）

| 原断言 | 替换为 | 合理性 |
|---|---|---|
| `SurfaceKind::Remote` → `"remote"` | `SurfaceKind::Server` → `"server"` | ✅ 仅变体名 + 序列化值对齐；测试目的（验证 wire shape + source identity）保留 |
| `ThreadEnvironmentKind::RemoteConnect` → `"remote_connect"` | `ThreadEnvironmentKind::RemoteSsh` → `"remote_ssh"` | ✅ 同上；SSH 字段 `remote_connection_id: Some("paired-phone")` 原样保留，与 SSH 语义锚点一致 |
| `RuntimeServiceCapability::RemoteConnection` (unsupported 测试) | `RuntimeServiceCapability::Terminal` | ✅ Terminal 也是 optional capability，fake provider 不注册，unsupported 路径语义等价 |
| `RuntimeServiceCapability::RemoteWorkspace` (availability 测试) | `RuntimeServiceCapability::Terminal` | ✅ 同上 |

替换而非删除的选择与 brief §(1)-(7) 第 7 项"删 RemoteConnect/Remote 相关断言段（:64,73 附近），保留其余"语义一致——保留测试"骨架 + 用剩余等价变体重填数据。

### 风格一致性

- `runtime-services/src/lib.rs` import 列表 `ClockPort, FileSystemPort, GitPort, McpCatalogPort, NetworkPort, PermissionPort, RuntimeEventSink, RuntimeServiceCapability, RuntimeServicePort, SessionStorePort, TerminalPort, WorkspacePort` 按字母序排列，与修改前一致 ✓
- `test_support.rs` import 同理 ✓
- `runtime_facade_tests.rs` 模块 doc 注释由 "remote + workspace" 简化为 "workspace"，与 `:144-147` 删除 `runtime-ports/src/remote.rs` 规则块的语义对齐 ✓

---

## 原始验证（独立重跑）

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
输出：`Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.15s`（与报告 2.38s 一致，仅 warnings 噪声，0 错误）

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
输出：`Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.34s`（与报告 1.87s 量级一致；5 warnings 均为 pre-existing）

```powershell
node scripts/check-core-boundaries.mjs
```
输出：`Core boundary check passed.`

```powershell
node scripts/core-boundaries/self-test.mjs
```
输出：exit=0（无 stdout/stderr）

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-runtime-ports
```
输出：`test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured`（lib） + `3 passed`（session_store_contracts）

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core-types
```
输出：`test result: ok. 2 passed`（lib） + `2 passed`（session_contracts） + `3 passed`（surface_contracts）

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-runtime-services
```
输出：`test result: ok. 6 passed`（runtime_services_contracts）

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --test product_assembly
```
输出：`test result: ok. 3 passed`

所有 5 个验证门槛与报告一致；归零检查两 rg 命令均 0 命中。

---

## Findings

### Critical
无。

### Important
无。

### Minor

1. **`session_workspace.rs:1` 模块 doc 注释残留 "remote-connection port traits"**：
   - 当前文件 :1 顶部 doc：`//! R26 sibling 2/4: session_workspace — session storage + workspace filesystem/shell + permission + clock + terminal + network + git + mcp + remote-connection port traits.`
   - 实施者严格按 brief §(3) "删 RemoteConnectionPort 及其 doc 注释块"执行，trait 删除 + 其紧邻的 4 行 doc 注释块正确删除；但模块级 doc 注释不在 brief 范围。
   - 经查 `git show 65145cf:...session_workspace.rs` 此行在 base 已存在同样文本，因此属 pre-existing，非本任务引入。**不属本批问题**，仅记录供后续 batch（如归一化模块 doc）参考。

---

## Cannot verify from diff 项（编排者复核建议）

| 项 | 状态 | 备注 |
|---|---|---|
| `cargo check --workspace` 全 features 笛卡尔积 | ⚠️ 本次仅 default features | brief 仅要求 check + test 全绿，已达成；CI 覆盖 full features |
| `SessionStorageKind::Remote` 在 product-full 范围外是否被其它 remote 模块消费 | ⚠️ 已 rg 确认存在定义与消费（在 `session_store.rs` 等未触碰文件），但未遍历全产品使用面 | 属 `services-integrations` 的事实层，独立范围 |
| 工作树残留 `.opencode/model-capability-notes.md` / `memory/northhing.md` / `.handoffs/handoff-g2-t9-2026-08-07.md` / `docs/design/2026-08-05-memory-architecture-research/` | ⚠️ 不在 diff 内 | 时间戳早于本次任务，属并行 session/之前任务残留，与本任务无关 |
| `required-rules.mjs` 中 `search/remote.rs` / `mcp/protocol/transport_remote.rs` 提及 | ✅ 不在本任务范围（路径不同） | 已被 `forbidden-rules.mjs` 与 `facade-rules.mjs` 引用；属不同模块的 boundary 规则 |
| 终审时 `SessionStoragePathResolution::remote()`（`session_workspace.rs`）的 wire shape 是否需同步删除 | ✅ 不在本任务范围 | brief 仅授权 6 项 + SSH 语义保留；SSH 沿用 remote_connection_id 而非 SessionStorageKind |

---

## 结论

**PASS** — 实施者准确执行 brief §Files 11 项清单，6 项授权 wire 词汇完整归零、4 项 capability 变体 + match 臂同步删除、boundary 规则同步、`surface_contracts.rs` 与 `runtime_services_contracts.rs` 测试用剩余等价变体（Server/RemoteSsh/Terminal）替换、SSH 语义保留。约束 1-5 全部满足，验证门槛 5/5 全绿，无新增编译器警告。建议编排者直接 ledger 追加并推进至下一批次。