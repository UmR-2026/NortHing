# Task Review T2-2e — services-integrations remote_connect 整删

## Verdict: **PASS**

## 总结

实施者严格按 brief 执行 services-integrations `remote_connect` 模块整删（14 源文件 + 7 纯 remote-connect 测试文件 + 10 orphan deps + 全部 boundary 规则同步），SSH / contracts / 共享 deps / 边界非本任务区域全部零损伤。验证门槛 5/5 全绿，归零检查通过。

---

## 双判决一：Spec 合规

### Constraint 验证

| Constraint | 结果 | 证据 |
|---|---|---|
| 1. SSH 零改动 | ✅ | `git diff -- 'src/.../remote_ssh/*' 'src/.../runtime-ports/src/remote.rs'` = 空；`remote-ssh`/`remote-ssh-concrete` feature 在 Cargo.toml 保留；`RemoteWorkspaceEntry`/`lookup_remote_connection*` 锚点在 self-test.mjs:2088 + required-rules.mjs:2035,5335 完整保留 |
| 2. contracts 层零改动 | ✅ | `git diff -- 'src/.../contracts/core-types/src/surface.rs' 'src/.../runtime-ports/src/remote.rs'` = 空；`ThreadEnvironmentKind::RemoteConnect` 在 `surface.rs:27` 与 `surface_contracts.rs:64,73` 三处故意保留（C4 范围） |
| 3. 共享 dep 保留 | ✅ | Cargo.toml L20-37 含 aes-gcm/anyhow/base64/chrono/futures/rand/sha2/tokio-util/uuid 全部 optional 保留；feature-rules.mjs 仅从 ownerFeatures 移除 `'remote-connect'`，dep 条目与其它 owner 完整保留 |
| 4. 只动任务书清单内文件 | ✅ | `git status` 显示 32 变更 + 2 untracked worktree 目录；`memory/` 与 `.opencode/` 改动时间戳 2026-08-18（早于本次 2026-08-19），属并行 session 残留非本任务改动（实施者报告 §3 已声明） |
| 5. 验证门槛 MSVC 全绿 | ✅ | 见下方"原始验证" |

### 审查要点逐条验证

#### (a) Cargo.toml：10 个 orphan dep 删除干净
- ✅ 10 个全部移除：`hostname` `image` `mac_address` `qrcode` `rustls` `rustls-native-certs` `schannel` `tokio-tungstenite` `urlencoding` `x25519-dalek`
- ✅ `[target.'cfg(windows)'.dependencies]` schannel 块整段删除（diff -U10 显示）
- ✅ `remote-connect = [...]` feature 块（22 行）删除
- ✅ `product-full` 列表移除 `"remote-connect"`，保留 `remote-ssh` / `remote-ssh-concrete`
- ✅ `northhing-runtime-ports` 保留为 optional，**依据属实**：`rg -n northhing_runtime_ports src/.../services-integrations/src` 仅命中 `deep_research.rs:6`，使用 `northhing_runtime_ports::deep_research::{renumber_research_report, ResearchCitationDisplayMapEntry}`；feature-rules.mjs owner 已对齐为 `['deep-research']`，与 Cargo.toml L57 `deep-research = ["northhing-runtime-ports"]` 一致
- ✅ 残留引用归零：`rg "use hostname|use image|use mac_address|use qrcode|use rustls|use rustls_native_certs|use schannel|use tokio_tungstenite|use urlencoding|use x25519_dalek"` 在 services-integrations 全部 0 命中
- ✅ Cargo.lock 同步：`hostname`/`qrcode`/`x25519-dalek`/`zeroize_derive` 包定义彻底消失；`mac_address`/`image`/`rustls`/`urlencoding`/`tokio-tungstenite` 仍存于 lock——经查是其它 crate（如 `wezterm-blob-leases` 依赖 `mac_address`）的传递依赖，非本任务范围，Cargo 行为正确

#### (b) feature-rules.mjs / crate-rules.mjs / required-rules.mjs / self-test.mjs 编辑精度

| 文件 | 关键操作 | 结果 |
|---|---|---|
| feature-rules.mjs | services-integrations 块内 8 个共享 dep 移除 `'remote-connect'` from ownerFeatures；10 orphan dep 整条目删除；`northhing-runtime-ports` owner 改为 `['deep-research']`；`ownerCrateFeatureAssemblyRules` 移除 `'remote-connect'` | ✅ 精确，29 行改动吻合 |
| crate-rules.mjs | services-integrations `forbiddenNonOptionalDeps` 移除 `'tokio-tungstenite'`（仅此 1 行；该列表是产品级 non-optional 禁用，orphan deps 中仅它被列于此） | ✅ 1 行删除吻合 |
| required-rules.mjs | 删除 9 个 src 块（mod.rs / remote_session_state / remote_request_builders / remote_workspace_resolver / remote_cancel_handlers / remote_dialog_handlers / remote_file_io / remote_session_handlers / remote_session_response_builders）+ 7 个 tests 块（command_runtime / dialog_cancel_contracts / file_transfer / model_catalog_tracker_poll / pairing_qr_relay / session_wire_and_responses / submission_images）= 16 块共 647 行 | ✅ 647 行删除吻合 |
| self-test.mjs | (i) `servicesOptionalOwnerRule` 校验列表移除 5 个 orphan（hostname/mac_address/qrcode/tokio-tungstenite/x25519-dalek）；(ii) 1691-1775 范围 remote_connect fixture 块（85 行）整段删除 | ✅ 91 行删除吻合 |

#### (c) self-test.mjs 保留项反向验证

| 保留目标 | 当前位置 | 状态 |
|---|---|---|
| `:546` `coreFullyMigratedDeps = new Set(['hostname', 'mac_address', 'qrcode', 'x25519-dalek'])` | self-test.mjs:546 | ✅ 完整保留（反向断言：core 不依赖它们，方向相反依然有效） |
| `:575` services-integrations 校验循环 | self-test.mjs:575 | ✅ 完整保留 |
| `:2179` SSH 锚点（删除 91 行后偏移至 `:2088`）| self-test.mjs:2088 | ✅ `lookup_remote_connection_with_hint` 保留 |
| crate-rules.mjs `:208,216` 的 qrcode/x25519-dalek 列表 | crate-rules.mjs:208,216 | ✅ 保留 |

#### (d) tests/common/mod.rs 手术

- ✅ `pub use northhing_services_integrations::remote_connect::{...}` 整块删除（约 32 行 re-export）
- ✅ 文件内所有 remote 专属 helper 删除：`TestImageContext`、`RecordingDialogHost`、`RecordingCancelHost`、`RecordingCommandHost`、`RecordingFileHost`、`RecordingTrackerHost`、`remote_history_contract_turn`、`remote_state`、`make_temp_remote_workspace`、`sample_remote_model_catalog`
- ✅ 保留 helper 全为 MCP 相关：`InMemoryMCPConfigStore` / `FailingMCPConfigStore` / `FakeMCPToolCatalogClient` / `make_mcp_config` / `make_resource` + MCP re-export
- ✅ **"为了能编译而删掉非 remote helper"风险排除**：编译绿 + 6 个 MCP 测试全跑过即证明 helper 完整覆盖需求；测试文件名（`config_and_server_lifecycle` / `context_enhancer_and_catalog` / `dynamic_tools_and_runtime` / `request_builders_and_adapters` / `tool_names_and_protocol` / `announcement_contracts`）全部为 MCP 相关，依赖类型与保留 helper 完全对应
- ✅ 文件总长 577 → 119（-458），修改幅度与 diff 包一致

#### (e) 7 个整删测试文件 cfg 头部验证

| 文件 | cfg 头 | 来源 |
|---|---|---|
| pairing_qr_relay.rs | `#![cfg(feature = "remote-connect")]` | diff 包 §"Deleted files" 头部 |
| command_runtime.rs | `#![cfg(feature = "remote-connect")]` | 同上 |
| dialog_cancel_contracts.rs | `#![cfg(feature = "remote-connect")]` | 同上 |
| file_transfer.rs | `#![cfg(feature = "remote-connect")]` | 同上 |
| model_catalog_tracker_poll.rs | `#![cfg(feature = "remote-connect")]` | 同上 |
| session_wire_and_responses.rs | `#![cfg(feature = "remote-connect")]` | 同上 |
| submission_images.rs | `#![cfg(feature = "remote-connect")]` | 同上 |

✅ 7/7 头部 cfg 门控一致，确认文件删除后不会导致 default 编译失败。

#### (f) remote_ssh_contracts.rs 100% 保留

- ✅ `git diff tests/remote_ssh_contracts.rs` = 空
- ✅ 文件含 7 个 `#[test]`（含 5 个同步 + 2 个 async），全部 `cargo test` 通过
- ✅ 测试名与报告完全一致：`remote_ssh_legacy_agent_auth_maps_to_default_private_key` / `remote_workspace_defaults_keep_older_files_loadable` / `remote_workspace_path_helpers_preserve_current_identity_contract` / `remote_workspace_session_paths_use_supplied_mirror_root` / `local_workspace_identity_helpers_preserve_canonical_root_contract` / `remote_workspace_registry_preserves_ambiguous_root_resolution_contract` / `remote_workspace_registry_preserves_legacy_state_and_clear_contract`

#### (g) 归零检查

| 命令 | 期望 | 实测 |
|---|---|---|
| `rg "remote_connect\b\|RemoteConnect\b" src --glob *.rs` | 仅 contracts 残留 | ✅ 仅 3 命中：`surface.rs:27` + `surface_contracts.rs:64,73`（C4 范围） |
| `rg "remote-connect" src scripts --glob *.toml --glob *.mjs` | 0 命中 | ✅ 0 命中 |
| `rg "remote_connect\b\|RemoteConnect\b" tests src/.../tests` | 0 命中 | ✅ 0 命中 |

---

## 双判决二：代码质量

### 准确性
- 所有删除操作零残留（rg 验证）
- 所有保留操作零遗漏（diff 验证 + 编译绿 + 测试绿三重证据）
- Boundary 规则编辑与 Cargo.toml/Cargo.lock 状态自洽

### 最小改动原则
- Cargo.toml：仅移除目标 dep 与 feature 行，未触碰其它
- lib.rs：仅移除 3 行 `pub mod remote_connect;`，保留所有其它 module 声明
- AGENTS.md 更新：精确删除 1 行（services 层）+ 4 行（services-integrations 层），无"顺手清配额"
- 未引入任何新文件、新依赖、新 feature

### 一致性
- test 函数删/留判定表（报告 §测试函数删/留判定表）与实际删除 7 个整文件 + 1 个 common/mod.rs 完全一致
- orphan dep owner 处置表（报告 §每个 orphan dep 的处置与依据）与 Cargo.toml/feature-rules.mjs/Cargo.lock 三方状态完全一致

### 报告诚实度
- 实施者报告所有声明可由 diff 包 + git status + 实测命令重现
- 未发现假数据或截断输出
- 残留解释（§残留解释与遗留疑虑）准确——`ThreadEnvironmentKind::RemoteConnect` 在 contracts 范围依 brief Constraint #2 故意保留

---

## 原始验证（重新实测）

| 命令 | 结果 |
|---|---|
| `cargo check --workspace` (stable-msvc) | ✅ Finished in 2.68s |
| `cargo check -p northhing` | ✅ Finished in 3.54s |
| `cargo check -p northhing-services-integrations --features remote-ssh,remote-ssh-concrete` | ✅ Finished in 2.28s |
| `cargo check -p northhing-services-integrations` (default) | ✅ Finished in 1.94s |
| `node scripts/check-core-boundaries.mjs` | ✅ "Core boundary check passed." |
| `node scripts/core-boundaries/self-test.mjs` | ✅ exit code 0 |
| `cargo test -p northhing-services-integrations --features product-full` | ✅ 142 tests passed (76 lib + 4 announcement + 18 config + 3 context + 9 dynamic + 2 file_watch + 3 function_agent + 10 git + 7 remote_ssh + 4 request_builders + 3 tool_names + 3 workspace_search) |

---

## Findings

### Critical
无。

### Important
无。

### Minor

1. **AGENTS.md 中"Remote workspace facts"那行语义模糊化（services-integrations/AGENTS.md L21-22）**：删除了"Remote-connect platform-neutral primitives"整段后，紧接着"Remote workspace facts, session metadata, file projection DTOs, and workspace/projection host traits belong in `northhing-runtime-ports`"这条规则仍指向已删除功能的事实类 DTO（属于 C4 范围未删）。当前文本不构成误导（C4 阶段会同步），但审查者提请关注：**这条规则的指向主体（`northhing-runtime-ports` 中的 remote 事实）也将被 C4 处理**，届时需同样精修。**不属本批工作**——本批严格遵守 contracts 零改动约束，仅记录供 C4 编排者参考。

2. **self-test.mjs `:2179` SSH 锚点的行号漂移**：删除 91 行后原 `:2179` 实际下移到 `:2088`。报告与 diff 包对此未显式说明（仅声明"preserve"），实施者在 C4 批次或后续如果继续编辑 self-test.mjs，需要按"删除量调整偏移"原则重新核对锚点行号。**不属本批问题**——本批所有保留锚点均正确保留，只是行号偏移属正常编辑副作用。

3. **Cargo.lock 仍含 `mac_address`/`image`/`rustls`/`urlencoding`/`tokio-tungstenite` 等同名依赖**：这些是其它 crate（如 `wezterm-blob-leases`、`notify-rust` 等）的传递依赖，**非本任务范围**——Cargo 自动锁文件保留它们是正确的。报告 §"Cargo.lock 自动同步"中"hostname, qrcode, x25519-dalek, zeroize_derive 从 lock 文件中彻底移除"准确描述了本任务范围内的 lock 状态变化。**报告建议**：未来类似批次可在"自动同步"行显式说明"其余同名包保留系其它 crate 传递依赖，非本任务残留"以避免读者混淆。**不属本批问题**。

---

## Cannot verify from diff 项（编排者复核建议）

| 项 | 状态 | 备注 |
|---|---|---|
| `services-integrations` default features 行为（删 `remote-connect` 后是否影响其它 product-full 子集组合） | ⚠️ 仅 cargo check default 通过，未穷举 feature 笛卡尔积 | 建议 CI 后续补齐；但本批 brief 仅要求 check + test 全绿，已达成 |
| `northhing-runtime-ports` 路径在 `deep_research` 与 `remote_connect` 之外的潜在交叉引用 | ✅ 范围内 grep 0 命中 | 无遗漏风险 |
| `tests/common/mod.rs` 保留的 MCP helper 是否覆盖未来新增 MCP tests | ⚠️ 仅覆盖现存 6 个 MCP test 文件 | 本任务范围，仅保证存量测试可跑；后续新增 MCP test 应自行按需扩展 |
| C4 阶段删除 `ThreadEnvironmentKind::RemoteConnect` 时的影响面 | ⚠️ 未涉及 | C4 范围 |

---

## 结论

**PASS** — 实施者准确执行 brief，未越界、未漏项、未顺手重构。SSH / contracts / 共享依赖全部零损伤。验证门槛 5/5 全绿，归零检查精确。建议编排者直接 ledger 追加并推进至 C4 或下一批次。