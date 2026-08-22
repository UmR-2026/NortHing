# Task Report T2-2f — remote 栈子批 C4：contracts 修剪（remote_connect-era 契约删除）

## Status
DONE

## 逐文件操作清单

1. **整删文件** `src/crates/contracts/runtime-ports/src/remote.rs`（143 行，共 16 个符号全部清理）：
   - DTO / Enum：`RemoteWorkspaceKind`, `RemoteWorkspaceFacts`, `RemoteRecentWorkspaceFacts`, `RemoteAssistantWorkspaceFacts`, `RemoteWorkspaceUpdate`, `RemoteSessionMetadata`, `RemoteWorkspaceFileContent`, `RemoteWorkspaceFileChunk`, `RemoteWorkspaceFileInfo`, `RemoteFileChunkRange`
   - Host Traits & Port Traits：`RemoteWorkspaceRuntimeHost`, `RemoteWorkspacePort`, `RemoteInitialSyncRuntimeHost`, `RemoteWorkspaceFileRuntimeHost`, `RemoteProjectionPort`, `RemoteCapabilityPort`

2. **模块注册与 re-export 清理** `src/crates/contracts/runtime-ports/src/lib.rs`：
   - 移除 `pub mod remote;`
   - 移除 `pub use remote::*;`
   - 更新顶部 doc 注释，将 4 sibling sub-domain 改为 3 sibling sub-domain（移除 remote）

3. **Port Trait 清理** `src/crates/contracts/runtime-ports/src/session_workspace.rs`：
   - 移除 `pub trait RemoteConnectionPort: RuntimeServicePort {}` 及其 doc 注释块

4. **Capability 枚举清理** `src/crates/contracts/runtime-ports/src/port_core.rs`：
   - 从 `RuntimeServiceCapability` 中移除 4 个变体：`RemoteConnection`, `RemoteWorkspace`, `RemoteProjection`, `RemoteCapabilities`
   - 从 `as_str(&self)` 中移除对应的 4 个 match 臂（`"remote_connection"`, `"remote_workspace"`, `"remote_projection"`, `"remote_capabilities"`）

5. **Facade 契约测试清理** `src/crates/contracts/runtime-ports/src/runtime_facade_tests.rs`：
   - 移除 2 个测试用例：`remote_workspace_contracts_preserve_workspace_and_session_facts` 与 `remote_projection_contract_preserves_file_chunk_identity`
   - 保留其余测试（`remote_control_state_snapshot_serializes_active_turn_contract`, `session_transcript_request_serializes_turn_id_contract`, `workspace_services_contract_is_runtime_port_owned`, `tool_runtime_handles_keep_workspace_services_and_cancellation_contracts`）

6. **Core Types 契约清理** `src/crates/contracts/core-types/src/surface.rs`：
   - 从 `SurfaceKind` 中移除变体 `Remote`
   - 从 `ThreadEnvironmentKind` 中移除变体 `RemoteConnect`
   - **严格保留** `RemoteSsh` 变体与 `ThreadEnvironment.remote_connection_id` 字段（SSH 语义）

7. **Core Types 测试同步** `src/crates/contracts/core-types/tests/surface_contracts.rs`：
   - `permission_and_capability_contracts_keep_source_identity`：将 `SurfaceKind::Remote` 替换为 `SurfaceKind::Server`（断言值对齐为 `"server"`）
   - `thread_environment_contract_does_not_require_surface_specific_fields`：将 `ThreadEnvironmentKind::RemoteConnect` 替换为 `ThreadEnvironmentKind::RemoteSsh`（断言值对齐为 `"remote_ssh"`）

8. **Runtime Services Registry & Builder 清理** `src/crates/execution/runtime-services/src/lib.rs`：
   - 移除 4 个 remote port import（`RemoteCapabilityPort`, `RemoteConnectionPort`, `RemoteProjectionPort`, `RemoteWorkspacePort`）
   - 从 `RuntimeServices` 结构体移除 4 个字段（`remote_connection`, `remote_workspace`, `remote_projection`, `remote_capabilities`）
   - 从 `Debug` 实现和 `has_capability()` 中移除对应 4 个变体处理
   - 从 `RuntimeServicesBuilder` 结构体移除 4 个字段与 4 个 `with_optional_remote_*` builder 方法
   - 从 `build()` 中移除对应 4 个服务的构建逻辑

9. **Test Support 清理** `src/crates/execution/runtime-services/src/test_support.rs`：
   - 移除 unused remote imports
   - 移除 `FakeRuntimePort` 对 `RemoteConnectionPort`, `RemoteCapabilityPort`, `RemoteWorkspaceRuntimeHost`, `RemoteWorkspaceFileRuntimeHost` 的 trait 实现
   - 移除 `FakeRuntimeServicesProvider` 中的 `include_remote` 字段及 `with_all_remote()` 方法，简化 `register()`

10. **Runtime Services 契约测试清理** `src/crates/execution/runtime-services/tests/runtime_services_contracts.rs`：
    - 移除 unused `RemoteWorkspaceKind` import
    - 将 `fake_provider_registers_required_and_remote_services_through_registry` 重命名为 `fake_provider_registers_required_services_through_registry` 并移除 remote capability 断言与 `with_all_remote()` 调用
    - `missing_optional_capability_returns_typed_unsupported_error` 与 `capability_availability_reports_optional_service_status_without_side_effects` 中将 `RemoteConnection` / `RemoteWorkspace` 改为 `Terminal`
    - 整删 `registered_remote_ports_expose_owner_contract_methods` 测试

11. **Assembly 测试同步** `src/crates/assembly/core/tests/product_assembly.rs`：
    - 移除 `core_runtime_services_provider_registers_existing_adapters_and_capability_markers` 中对 `RemoteWorkspace` 和 `RemoteProjection` 的两条断言

12. **Boundary 检查规则与 Self-Test 同步** `scripts/core-boundaries/`：
    - `rules/source/required-rules.mjs`：移除 `registered_remote_ports_expose_owner_contract_methods` 规则；更新 `fake_provider_registers_required_services_through_registry` 规则；移除 `runtime-ports/src/remote.rs` 规则块；更新 `runtime_facade_tests.rs` 规则
    - `self-test.mjs`：移除 `runtime-ports/src/remote.rs` 期望条目；更新 `runtime_facade_tests.rs` 与 `runtime_services_contracts.rs` 期望列表

---

## 授权删除的 Wire 词汇删除确认

本批次授权删除的 6 个序列化 wire 词汇已逐一确认删除，且无任何清单外词汇被修改：

| Wire 词汇 | 原位置 | 序列化字符串 | 处置确认 |
|---|---|---|---|
| `SurfaceKind::Remote` | `core-types/src/surface.rs:16` | `"remote"` | **已删除**（`core-types/src/surface.rs` 中已无此变体） |
| `ThreadEnvironmentKind::RemoteConnect` | `core-types/src/surface.rs:27` | `"remote_connect"` | **已删除**（`core-types/src/surface.rs` 中已无此变体） |
| `RuntimeServiceCapability::RemoteConnection` | `runtime-ports/src/port_core.rs:58` | `"remote_connection"` | **已删除**（`runtime-ports/src/port_core.rs` 中已无此变体与 as_str 匹配臂） |
| `RuntimeServiceCapability::RemoteWorkspace` | `runtime-ports/src/port_core.rs:59` | `"remote_workspace"` | **已删除**（`runtime-ports/src/port_core.rs` 中已无此变体与 as_str 匹配臂） |
| `RuntimeServiceCapability::RemoteProjection` | `runtime-ports/src/port_core.rs:60` | `"remote_projection"` | **已删除**（`runtime-ports/src/port_core.rs` 中已无此变体与 as_str 匹配臂） |
| `RuntimeServiceCapability::RemoteCapabilities` | `runtime-ports/src/port_core.rs:61` | `"remote_capabilities"` | **已删除**（`runtime-ports/src/port_core.rs` 中已无此变体与 as_str 匹配臂） |

**清单外 Contracts / 符号保留确认**：
- `DialogTriggerSource::{RemoteRelay, Bot}`：100% 保留
- `ThreadEnvironmentKind::RemoteSsh`：100% 保留
- `ThreadEnvironment.remote_connection_id`：100% 保留
- `session_workspace.rs` 其余全部 ports / DTOs：100% 保留
- SSH 模块相关符号（`remote_ssh` 模块、`lookup_remote_connection*`、`RemoteWorkspaceEntry` 等）：100% 保留，零改动

---

## 验证原始输出

### 1. Workspace 编译检查

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
**输出**：
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.38s
```

### 2. Desktop 编译检查

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
**输出**：
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.87s
```

### 3. Boundary 规则与 Self-Test 检查

```powershell
node scripts/check-core-boundaries.mjs
node scripts/core-boundaries/self-test.mjs
```
**输出**：
```text
Core boundary check passed.
```

### 4. 契约与服务单元测试

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-runtime-ports
```
**输出**：
```text
     Running unittests src\lib.rs (target\debug\deps\northhing_runtime_ports-a700b1321141da1b.exe)

running 41 tests
test agent_facade_tests::agent_session_reply_decisions_preserve_cancel_suppression_boundary ... ok
test agent_facade_tests::agent_submission_request_serializes_with_stable_camel_case ... ok
test agent_facade_tests::agent_submission_request_serializes_source_without_changing_field_case ... ok
test agent_facade_tests::agent_session_management_contracts_serialize_stable_shape ... ok
test agent_facade_tests::agent_submission_request_serializes_explicit_turn_id_contract ... ok
test agent_facade_tests::agent_background_result_request_serializes_lifecycle_contract ... ok
test agent_facade_tests::dialog_submission_policy_preserves_current_surface_queue_defaults ... ok
test agent_facade_tests::agent_dialog_turn_request_serializes_lifecycle_contract ... ok
test agent_facade_tests::dialog_trigger_source_reuses_agent_submission_source_contract ... ok
test agent_facade_tests::delegation_policy_child_blocks_recursive_spawn_without_losing_depth ... ok
test agent_facade_tests::agent_turn_cancellation_request_serializes_current_contract ... ok
test agent_facade_tests::dialog_steer_outcome_preserves_buffered_fields ... ok
test agent_facade_tests::agent_session_reply_route_keeps_requester_fields ... ok
test agent_facade_tests::dialog_submit_outcome_preserves_started_and_queued_fields ... ok
test agent_facade_tests::dialog_submit_queue_action_preserves_current_scheduler_routing_policy ... ok
test agent_facade_tests::thread_goal_active_status_includes_budget_limited ... ok
test agent_facade_tests::thread_goal_tool_response_serializes_optional_fields ... ok
test lightweight_task::tests::output_tag_is_stable ... ok
test agent_facade_tests::dynamic_tool_descriptor_omits_missing_provider_id ... ok
test mcp::tests::all_connected_renders_count ... ok
test agent_facade_tests::dynamic_tool_descriptor_serializes_current_wire_shape ... ok
test mcp::tests::dto_round_trips_through_camel_case_json ... ok
test agent_facade_tests::round_injection_contract_keeps_kind_and_target_identity ... ok
test agent_facade_tests::round_injection_source_contract_drains_portable_injections ... ok
test agent_facade_tests::runtime_event_envelope_serializes_observational_surface_facts ... ok
test agent_facade_tests::subagent_context_mode_preserves_fork_wire_value ... ok
test agent_facade_tests::agent_thread_goal_delivery_request_serializes_lifecycle_contract ... ok
test lightweight_task::tests::request_round_trips_with_stable_camel_case ... ok
test agent_facade_tests::remote_image_attachment_serializes_portable_metadata_contract ... ok
test mcp::tests::empty_catalog_renders_not_configured ... ok
test mcp::tests::error_path_renders_message ... ok
test mcp::tests::mixed_with_disabled_renders_both ... ok
test port_facade_tests::compression_contract_renders_model_visible_fields ... ok
test port_facade_tests::port_error_display_keeps_kind_and_message ... ok
test lightweight_task::tests::port_trait_is_implementable ... ok
test mcp::tests::port_trait_is_implementable ... ok
test port_facade_tests::related_path_serializes_as_request_context_fact ... ok
test runtime_facade_tests::remote_control_state_snapshot_serializes_active_turn_contract ... ok
test runtime_facade_tests::session_transcript_request_serializes_turn_id_contract ... ok
test runtime_facade_tests::tool_runtime_handles_keep_workspace_services_and_cancellation_contracts ... ok
test runtime_facade_tests::workspace_services_contract_is_runtime_port_owned ... ok

test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\session_store_contracts.rs (target\debug\deps\session_store_contracts-9e5c9340c700223e.exe)

running 3 tests
test session_storage_path_resolution_carries_local_and_remote_facts ... ok
test session_restore_timing_serializes_camel_case_fields ... ok
test session_store_port_exposes_typed_storage_path_resolution ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core-types
```
**输出**：
```text
     Running unittests src\lib.rs (target\debug\deps\northhing_core_types-a85201bfafce6914.exe)

running 2 tests
test errors::tests::classifies_quota_and_provider_unavailable_errors ... ok
test errors::tests::builds_ai_error_detail_from_provider_metadata ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\session_contracts.rs (target\debug\deps\session_contracts-3a6743eeba7311a8.exe)

running 2 tests
test session_kind_preserves_legacy_snake_case_deserialization ... ok
test session_kind_preserves_default_and_serialized_shape ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\surface_contracts.rs (target\debug\deps\surface_contracts-1058f053783747c4.exe)

running 3 tests
test permission_and_capability_contracts_keep_source_identity ... ok
test surface_contract_serializes_observational_runtime_facts ... ok
test thread_environment_contract_does_not_require_surface_specific_fields ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-runtime-services
```
**输出**：
```text
     Running tests\runtime_services_contracts.rs (target\debug\deps\runtime_services_contracts-e7627936e4f6051a.exe)

running 6 tests
test builder_rejects_port_registered_under_the_wrong_capability ... ok
test capability_availability_reports_optional_service_status_without_side_effects ... ok
test fake_provider_registers_required_services_through_registry ... ok
test builder_requires_mandatory_runtime_services ... ok
test missing_optional_capability_returns_typed_unsupported_error ... ok
test registered_session_store_port_exposes_storage_path_resolution ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --test product_assembly
```
**输出**：
```text
     Running tests\product_assembly.rs (target\debug\deps\product_assembly-b0b19872ae550959.exe)

running 3 tests
test product_assembly_facade_preserves_legacy_provider_import_path ... ok
test core_runtime_services_provider_registers_existing_adapters_and_capability_markers ... ok
test core_provider_closes_current_product_full_service_capability_requirements ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 5. 归零检查

```powershell
rg -n "RemoteConnectionPort|RemoteWorkspacePort|RemoteProjectionPort|RemoteCapabilityPort|RemoteWorkspaceKind|RemoteInitialSyncRuntimeHost|RemoteWorkspaceFileRuntimeHost|RemoteWorkspaceRuntimeHost" src --glob "*.rs"
```
**输出**：
```text
(0 命中)
```

```powershell
rg -n "SurfaceKind::Remote\b|ThreadEnvironmentKind::RemoteConnect|RuntimeServiceCapability::Remote" src --glob "*.rs"
```
**输出**：
```text
(0 命中)
```

---

## 遗留疑虑与说明

1. **SSH 语义与变体保留**：`ThreadEnvironmentKind::RemoteSsh` 与 `ThreadEnvironment.remote_connection_id` 保留，用于远程 SSH 环境识别；`RemoteRelay` 与 `Bot` 保留，用于 dialog trigger source 契约。
2. **工作区隔离**：未触碰 `memory/`、`.opencode/`、`.superpowers/sdd/` 下其它 task-* 文件，未做 git commit / push。
