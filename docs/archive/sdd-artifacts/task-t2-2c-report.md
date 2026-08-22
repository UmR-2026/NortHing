# Task T2-2c Report: remote 栈子批 C1——core 侧摘除

DONE

## 1. 逐项执行状态

| 项 | 描述 | 状态 | 详细说明 |
|---|---|---|---|
| **S1** | 删 core `remote_connect` 模块 | COMPLETED | 删除 `src/crates/assembly/core/src/service/remote_connect/` 目录下全部 48 个文件（10,443 行）；删除 `src/crates/assembly/core/src/service/mod.rs` 中 `pub mod remote_connect;` 及其 cfg 门控行。 |
| **S2** | SAR remote 适配器摘除 | COMPLETED | 保留 `CoreServiceAgentRuntime` 本体（agent_runtime* 与 runtime_error_message 方法）；删除 `sar_handler.rs`、`sar_lifecycle.rs`、`sar_state.rs`、`sar_types.rs`；从 `sar_dispatch.rs` 移除全部 remote_* 方法；从 `mod.rs` 移除 `CoreRemote*Host` 重新导出及 remote 专测。 |
| **S3** | `product_runtime` 注册摘除 | COMPLETED | 从 `src/crates/assembly/core/src/product_runtime/runtime_services.rs` 移除 `CoreRemoteWorkspaceRuntimeHost` 与 `CoreRemoteWorkspaceFileRuntimeHost` 的注册逻辑及未使用的 trait import。 |
| **S4** | core `Cargo.toml` relay 依赖摘除 | COMPLETED | 删除 `northhing-relay-core` optional 依赖声明与 `service-integrations` feature 中的 `"dep:northhing-relay-core"`。 |
| **S5** | 顺手清失效测试注释 | COMPLETED | 修改 `src/apps/desktop/src/app_state/settings/io/io_tests.rs:4`，移除对已删除的 `remote_connect/bot/persistence_tests.rs` 引用。 |
| **S6** | boundary 规则同步 | COMPLETED | 同步修改 `scripts/core-boundaries/rules/source/required-rules.mjs`、`forbidden-rules.mjs` 及 `self-test.mjs`，清理已删除文件与 remote 适配器规则并保持存活规则。 |
| **S7** | 文档同步 | COMPLETED | 更新 `src/crates/assembly/core/AGENTS.md:20` 与 `AGENTS-CN.md:16` 中的 `src/service/` 枚举，移除 `remote connect`。 |

---

## 2. 验证原始输出

### 1. `cargo check --workspace` (MSVC)
```
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.19s
```

### 2. `cargo check -p northhing` (MSVC)
```
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.96s
```

### 3. `node scripts/check-core-boundaries.mjs`
```
node scripts/check-core-boundaries.mjs
Core boundary check passed.
```

### 4. Focused 测试

#### (a) `cargo test -p northhing-core --features product-full --lib service_agent_runtime`
```
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib service_agent_runtime
     Running unittests src\lib.rs (target\debug\deps\northhing_core-b99735cdae331ad8.exe)

running 3 tests
test service_agent_runtime::tests::core_service_agent_runtime_owner_exposes_agent_runtime_and_remote_control_port ... ok
test service_agent_runtime::tests::core_service_agent_runtime_owner_keeps_scheduler_lifecycle_port_contracts ... ok
test service_agent_runtime::tests::core_service_agent_runtime_owner_keeps_coordinator_port_contracts ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1040 filtered out; finished in 0.00s
```

#### (b) `cargo test -p northhing-core --features product-full --lib product_runtime`
```
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib product_runtime
     Running unittests src\lib.rs (target\debug\deps\northhing_core-b99735cdae331ad8.exe)

running 22 tests
test agentic::tools::product_runtime::get_tool_spec_tool::tests::collapsed_tools_context_lists_names_without_short_descriptions ... ok
test agentic::tools::product_runtime::unlock_state::tests::product_unlock_state_collects_visible_get_tool_spec_results ... ok
test agentic::tools::product_runtime::unlock_state::tests::product_collapsed_unlock_state_preserves_message_derived_lifecycle ... ok
test agentic::tools::product_runtime::unlock_state::tests::product_unlock_state_dedupes_and_filters_runtime_unlocks ... ok
test agentic::tools::product_runtime::get_tool_spec_tool::tests::reloading_already_unlocked_tool_returns_assistant_hint ... ok
test agentic::tools::product_runtime::catalog::tests::product_catalog_provider_context_requires_agent_type ... ok
test agentic::tools::product_runtime::tests::product_tool_runtime_registry_preserves_provider_plan_order ... ok
test agentic::tools::product_runtime::catalog::tests::product_resolved_visible_tools_owner_matches_registry_visibility ... ok
test agentic::tools::product_runtime::catalog::tests::product_get_tool_spec_rejects_expanded_webfetch_in_agentic_mode ... ok
test agentic::tools::product_runtime::catalog::tests::product_manifest_snapshot_preserves_collapsed_tool_discovery_contract ... ok
test agentic::tools::product_runtime::catalog::tests::product_catalog_facade_resolves_manifest_from_same_provider_owner ... ok
test agentic::tools::product_runtime::catalog::tests::product_manifest_expands_tool_when_agent_override_requests_it ... ok
test agentic::tools::product_runtime::catalog::tests::product_resolved_manifest_owner_matches_legacy_shape ... ok
test agentic::tools::product_runtime::catalog::tests::product_manifest_preserves_explicit_get_tool_spec_runtime_contract ... ok
test agentic::tools::product_runtime::catalog::tests::product_manifest_write_schema_requires_content ... ok
test agentic::tools::product_runtime::catalog::tests::product_manifest_guard_preserves_get_tool_spec_unlock_surface ... ok
test agentic::tools::product_runtime::catalog::tests::product_manifest_omits_get_tool_spec_without_collapsed_tools ... ok
test agentic::tools::product_runtime::catalog::tests::product_catalog_facade_resolves_get_tool_spec_results_from_same_provider_owner ... ok
test agentic::tools::product_runtime::tests::product_tool_runtime_owner_preserves_registry_contract ... ok
test agentic::tools::product_runtime::catalog::tests::product_catalog_provider_reads_global_registry_snapshot ... ok
test agentic::tools::product_runtime::catalog::tests::product_catalog_provider_default_get_tool_spec_catalog_matches_registry ... ok
test agentic::tools::product_runtime::catalog::tests::product_catalog_facade_resolves_readonly_enabled_tools_from_same_provider_owner ... ok

test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 1021 filtered out; finished in 0.01s
```

### 5. S1/S2 删后归零复核

#### (a) `rg -n "remote_connect\b|RemoteConnect\b" src/crates/assembly/core --glob "*.rs"`
```
rg -n "remote_connect\b|RemoteConnect\b" src/crates/assembly/core --glob "*.rs"
(0 matches)
```

#### (b) `rg -n -i "remote_connect|RemoteConnect" src/crates/assembly/core --glob "*.rs"`
全部命中均为 SSH 远程工作区语义（`remote_connection_id` 字段及 `lookup_remote_connection` 相关的 SSH 逻辑，属独立子系统保持未动）：
- `src/crates/assembly/core/src/agentic/core/session.rs:209` (`pub remote_connection_id: Option<String>`)
- `src/crates/assembly/core/src/agentic/coordination/dialog_turn/workspace.rs`
- `src/crates/assembly/core/src/agentic/fork_agent/mod.rs`
- `src/crates/assembly/core/src/agentic/session/session_store_port.rs`
- `src/crates/assembly/core/src/service/workspace/types.rs`
- `src/crates/assembly/core/src/service/filesystem/service.rs`
- `src/crates/assembly/core/src/service/remote_ssh/workspace_state.rs`
- `src/crates/assembly/core/src/service/cron/`

#### (c) SAR 目录 `rg -n -i "remote" src/crates/assembly/core/src/service_agent_runtime`
```
rg -n -i "remote" src/crates/assembly/core/src/service_agent_runtime
src/crates/assembly/core/src/service_agent_runtime\mod.rs:24:                + northhing_runtime_ports::RemoteControlStatePort
src/crates/assembly/core/src/service_agent_runtime\mod.rs:46:    fn core_service_agent_runtime_owner_exposes_agent_runtime_and_remote_control_port() {
```
判别：
1. `mod.rs:24`: `northhing_runtime_ports::RemoteControlStatePort` 为 contracts 契约层 trait（在 coordinator port contracts 静态断言中），非 remote 适配器实现。
2. `mod.rs:46`: 测试函数名称，测试 `CoreServiceAgentRuntime` 接口。
SAR 目录下 0 处 remote_connect 适配器残留。

### 6. `cargo metadata --no-deps --format-version 1`
```
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo metadata --no-deps --format-version 1 > $null
(exit code 0, no output)
```

---

## 3. 行数与 Diffstat 对账

```
git diff --stat HEAD -- src/crates/assembly/core scripts/core-boundaries src/apps/desktop/src/app_state/settings/io/io_tests.rs Cargo.lock

 Cargo.lock                                         |   1 -
 .../rules/source/forbidden-rules.mjs               | 359 ----------
 .../rules/source/required-rules.mjs                | 366 +---------
 scripts/core-boundaries/self-test.mjs              | 149 ----
 .../desktop/src/app_state/settings/io/io_tests.rs  |   3 +-
 src/crates/assembly/core/AGENTS-CN.md              |   2 +-
 src/crates/assembly/core/AGENTS.md                 |   2 +-
 src/crates/assembly/core/Cargo.toml                |   4 -
 .../core/src/product_runtime/runtime_services.rs   |  25 +-
 src/crates/assembly/core/src/service/mod.rs        |   2 -
 .../service/remote_connect/bot/command_router.rs   | 300 --------
 .../remote_connect/bot/command_router_dispatch.rs  | 668 -----------------
 .../bot/command_router_forwarded_turn.rs           | 191 -----
 .../remote_connect/bot/command_router_questions.rs | 167 -----
 .../remote_connect/bot/command_router_resume.rs    | 139 ----
 .../remote_connect/bot/command_router_session.rs   | 292 --------
 .../remote_connect/bot/command_router_state.rs     | 151 ----
 .../remote_connect/bot/command_router_tests.rs     | 314 --------
 .../remote_connect/bot/command_router_util.rs      | 105 ---
 .../remote_connect/bot/command_router_view.rs      | 282 --------
 .../remote_connect/bot/feishu/feishu_actions.rs    | 265 -------
 .../remote_connect/bot/feishu/feishu_commands.rs   | 306 --------
 .../remote_connect/bot/feishu/feishu_messages.rs   | 321 ---------
 .../remote_connect/bot/feishu/feishu_types.rs      | 257 -------
 .../remote_connect/bot/feishu/feishu_webhook.rs    | 409 -----------
 .../src/service/remote_connect/bot/feishu/mod.rs   |  96 ---
 .../core/src/service/remote_connect/bot/locale.rs  | 600 ----------------
 .../service/remote_connect/bot/media_download.rs   | 136 ----
 .../service/remote_connect/bot/media_send_text.rs  | 114 ---
 .../src/service/remote_connect/bot/media_types.rs  |  33 -
 .../src/service/remote_connect/bot/media_typing.rs | 148 ----
 .../src/service/remote_connect/bot/media_upload.rs | 250 -------
 .../service/remote_connect/bot/media_validate.rs   | 146 ----
 .../core/src/service/remote_connect/bot/menu.rs    | 208 ------
 .../core/src/service/remote_connect/bot/mod.rs     | 794 ---------------------
 .../remote_connect/bot/persistence_tests.rs        | 191 -----
 .../src/service/remote_connect/bot/telegram.rs     | 653 -----------------
 .../core/src/service/remote_connect/bot/weixin.rs  |  46 --
 .../src/service/remote_connect/bot/weixin_bot.rs   | 248 -------
 .../remote_connect/bot/weixin_bot_inbound.rs       | 567 ---------------
 .../service/remote_connect/bot/weixin_bot_media.rs |  31 -
 .../remote_connect/bot/weixin_crypto/helpers.rs    |  83 ---
 .../remote_connect/bot/weixin_crypto/init.rs       | 106 ---
 .../remote_connect/bot/weixin_crypto/mod.rs        |  18 -
 .../remote_connect/bot/weixin_crypto/types.rs      |  27 -
 .../service/remote_connect/bot/weixin_qr_login.rs  | 448 ------------
 .../core/src/service/remote_connect/command.rs     | 130 ----
 .../core/src/service/remote_connect/connect.rs     | 155 ----
 .../remote_connect/connect/bot_connection.rs       | 332 ---------
 .../remote_connect/connect/mobile_identity.rs      |  61 --
 .../remote_connect/connect/relay_connection.rs     | 262 -------
 .../src/service/remote_connect/embedded_relay.rs   | 131 ----
 .../core/src/service/remote_connect/lan.rs         |  34 -
 .../core/src/service/remote_connect/mod.rs         | 108 ---
 .../core/src/service/remote_connect/ngrok.rs       | 275 -------
 .../src/service/remote_connect/remote_server.rs    | 561 ---------------
 .../core/src/service/remote_connect/session.rs     |  45 --
 .../core/src/service/remote_connect/sync.rs        | 315 --------
 .../assembly/core/src/service_agent_runtime/mod.rs | 288 +-------
 .../core/src/service_agent_runtime/sar_dispatch.rs | 192 +----
 .../core/src/service_agent_runtime/sar_handler.rs  | 323 ---------
 .../src/service_agent_runtime/sar_lifecycle.rs     | 121 ----
 .../core/src/service_agent_runtime/sar_state.rs    | 119 ---
 .../core/src/service_agent_runtime/sar_types.rs    | 396 ----------
 64 files changed, 14 insertions(+), 13857 deletions(-)
```

对账分析：
- 删除文件总计：52 个文件（48 个 `service/remote_connect/` + 4 个 SAR 模块文件）。
- `service/remote_connect/`: 10,443 行完全移除。
- `service_agent_runtime/`: 1,760 行（4 文件 959 行 + sar_dispatch / mod.rs 瘦身 801 行）完全移除。
- `scripts/core-boundaries/`: 874 行规则同步移除。
- 其余（runtime_services.rs、Cargo.toml、mod.rs、AGENTS 等）：约 40 行改动。
- 净删除行数：13,843 行。

---

## 4. 遗留疑虑 (Concerns)

1. **contracts 层 `RuntimeServiceCapability::RemoteConnection` / `RemoteWorkspacePort` / `RemoteProjectionPort`**：按任务书规定留在 C4 子批删除，本批仅移除了 core assembly 层的注册。
2. **`agentic/remote_file_delivery.rs` 与 `computer://` 提示词通路**：按任务书规定留在 C2 子批处理。
3. **`northhing-services-integrations` 的 remote_connect 模块**：按任务书规定留在 C3 子批处理。
4. 无阻塞性疑虑，所有门禁检查与单元测试全部绿线通过。
