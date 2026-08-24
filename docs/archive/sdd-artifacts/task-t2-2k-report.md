# Task T2-2k Report — MiniApp 整删 M1：入口摘除

## 状态
**DONE**

## 改动清单（文件:行 + 前后摘要）

### A. InitMiniApp 工具摘除
1. `src/crates/assembly/core/src/agentic/tools/implementations/miniapp_init_tool.rs`
   - **改动**：整删（221 行，InitMiniAppTool 工具本体与单元测试）。
2. `src/crates/assembly/core/src/agentic/tools/implementations/mod.rs`
   - **前**：:51 `pub mod miniapp_init_tool;`、:93 `pub use miniapp_init_tool::InitMiniAppTool;`
   - **后**：两行均已删除。
3. `src/crates/assembly/core/src/agentic/tools/product_runtime/materialization.rs`
   - **前**：:61 `"InitMiniApp",`（collapsed 列表项）、:111 `"InitMiniApp" => Some(Arc::new(InitMiniAppTool::new())),`
   - **后**：两处均已删除。
4. `src/crates/assembly/core/src/agentic/agents/mod.rs`
   - **前**：:96 `"InitMiniApp".to_string(),`
   - **后**：该行已删除。
5. `src/crates/assembly/core/src/agentic/tools/agent-tool-exposure.md`
   - **前**：:44 `| InitMiniApp | Collapsed | None | - |`
   - **后**：该行已删除。
6. `src/crates/assembly/core/src/agentic/tools/registry/tests.rs`
   - **前**：:209 `"InitMiniApp",`、:349 `assert!(!registry.is_tool_collapsed("InitMiniApp"));`
   - **后**：两处均已删除。

### B. headless 假限制通路摘除
7. `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs`
   - **前**：:27-29 import `is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions`；:156-161 条件分支判断 `if is_miniapp_headless_agent_run(...) { miniapp_headless_agent_tool_restrictions() } else { ToolRuntimeRestrictions::default() }`
   - **后**：import 改为 `use crate::agentic::tools::ToolRuntimeRestrictions;`；赋值简化为 `let runtime_tool_restrictions = ToolRuntimeRestrictions::default();`。
8. `src/crates/assembly/core/src/agentic/tools/restrictions.rs`
   - **前**：:5 `use std::collections::{BTreeMap, BTreeSet};`、:8-90 `is_miniapp_headless_agent_run` 与 `miniapp_headless_agent_tool_restrictions`、:149-167 两个 miniapp headless 单元测试。
   - **后**：删除上述内容；保留 `ToolPathPolicy`、`ToolPathOperation`、`ToolRuntimeRestrictions`、`is_local_path_within_root` 及存活测试。
9. `src/crates/assembly/core/src/agentic/tools/mod.rs`
   - **前**：:39-42 `pub use restrictions::{is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolPathOperation, ToolPathPolicy, ToolRuntimeRestrictions};`
   - **后**：改为 `pub use restrictions::{ToolPathOperation, ToolPathPolicy, ToolRuntimeRestrictions};`。
10. **7 个死 import 行清理**：
    - `src/crates/assembly/core/src/agentic/coordination/coordinator.rs:35-37`
    - `src/crates/assembly/core/src/agentic/coordination/dialog_turn/compaction.rs:40-42`
    - `src/crates/assembly/core/src/agentic/coordination/dialog_turn/session.rs:40-42`
    - `src/crates/assembly/core/src/agentic/coordination/dialog_turn/thread_goal.rs:40-42`
    - `src/crates/assembly/core/src/agentic/coordination/dialog_turn/workspace.rs:40-42`
    - `src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_dispatch.rs:16-18`
    - `src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_types.rs:7-9`
    - **前**：均 `use crate::agentic::tools::{is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions};`
    - **后**：均改为 `use crate::agentic::tools::ToolRuntimeRestrictions;`。

### C. product-capabilities 的 MiniApp capability 摘除
11. `src/crates/assembly/product-capabilities/src/lib.rs`
    - **前**：:17 `MiniApp,` 变体、:26 `Self::MiniApp => "miniapp",` 映射臂、:366-371 `MINIAPP_SERVICES` 常量、:386-389 在 `DEFAULT_PRODUCT_CAPABILITY_PACKS` 中的注册块。
    - **后**：四处均已删除。
12. `src/crates/assembly/product-capabilities/tests/product_capabilities.rs`
    - **前**：:19、:31、:86 断言中 `vec!["code-agent", "deep-review", "deep-research", "miniapp"]`
    - **后**：均更新为 `vec!["code-agent", "deep-review", "deep-research"]`。

### D. 用户面入口
13. 删 3 个 announcement tips 文件：
    - `src/crates/assembly/core/src/service/announcement/content/tips/en-US/013_miniapp.md`（整删）
    - `src/crates/assembly/core/src/service/announcement/content/tips/zh-CN/013_miniapp.md`（整删）
    - `src/crates/assembly/core/src/service/announcement/content/tips/zh-TW/013_miniapp.md`（整删）
14. e2e 死选择器清理：
    - `tests/e2e/specs/l0-navigation.spec.ts:14`：删除 `'.northhing-nav-panel__miniapp-entry',`。
    - `tests/e2e/specs/l1-navigation.spec.ts:18,173,194,232`：删除 `NAV_ENTRY_SELECTORS` 中的 `'.northhing-nav-panel__miniapp-entry',`，并将 3 处 `activeItems` / `initialActive` / `afterActive` 的选择器字符串中的 `, .northhing-nav-panel__miniapp-entry.is-active` 摘除。

### E. boundary 规则同步
15. 经侦察与检索，本批删除项（`InitMiniApp`、`miniapp_init_tool.rs`、`miniapp_headless_*`、`ProductCapabilityId::MiniApp`）在 `scripts/core-boundaries/` 中未设独立规则锚点；M2-M5 层的 miniapp 目录与 feature 锚点全部完整保留（规则行数保持 474 行不变）。`node scripts/check-core-boundaries.mjs` 绿灯通过。

---

## 验证证据（命令 + 原始输出）

### 1. cargo check --workspace
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
输出：
```text
   Compiling northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing-product-capabilities v0.2.10 (E:\agent-project\northing\src\crates\assembly\product-capabilities)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 26s
```

### 2. cargo check -p northhing (desktop 门禁)
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
输出：
```text
   Compiling northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing-product-capabilities v0.2.10 (E:\agent-project\northing\src\crates\assembly\product-capabilities)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 27s
```

### 3. check-core-boundaries.mjs
```powershell
node scripts/check-core-boundaries.mjs
```
输出：
```text
Core boundary check passed.
```

### 4. cargo test -p northhing-core --lib --features product-full tools::registry & restrictions
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --lib --features product-full tools::registry
```
输出：
```text
running 22 tests
test agentic::tools::registry::tests::product_capability_provider_plan_keeps_owner_group_order ... ok
test agentic::tools::registry::tests::capability_summary_expanded_plus_collapsed_equals_total ... ok
test agentic::tools::registry::tests::registry_exposes_controlhub_and_computer_use ... ok
test agentic::tools::registry::tests::registry_preserves_collapsed_tool_manifest_for_owner_migration ... ok
test agentic::tools::registry::tests::registry_includes_webfetch_tool ... ok
test agentic::tools::registry::tests::registry_marks_collapsed_tools_for_get_tool_spec ... ok
test agentic::tools::registry::tests::registry_preserves_builtin_tool_manifest_for_owner_migration ... ok
test agentic::tools::registry::tests::registry_includes_cron_tool ... ok
test agentic::tools::registry::tests::product_capability_provider_plan_covers_registry_manifest_in_order ... ok
test agentic::tools::registry::tests::registry_wraps_file_modification_tools_for_snapshot_tracking ... ok
test agentic::tools::registry::tests::registering_static_tool_clears_stale_dynamic_metadata_for_same_name ... ok
test agentic::tools::registry::tests::dynamic_tool_provider_prefers_mcp_registry_metadata ... ok
test agentic::tools::registry::tests::dynamic_tool_provider_uses_explicit_provider_metadata ... ok
test agentic::tools::registry::tests::registry_preserves_readonly_tool_manifest_for_owner_migration ... ok
test agentic::tools::registry::tests::capability_summary_no_overlap_between_expanded_and_collapsed ... ok
test agentic::tools::registry::tests::capability_summary_preserves_known_collapsed_tools ... ok
test agentic::tools::registry::tests::capability_summary_readonly_is_subset_of_expanded ... ok
test agentic::tools::registry::tests::dynamic_tool_provider_preserves_descriptor_shape_and_order ... ok
test agentic::tools::registry::tests::product_tool_runtime_keeps_custom_decorator_provider_contract ... ok
test agentic::tools::registry::tests::product_tool_runtime_owner_preserves_registry_contract ... ok
test agentic::tools::registry::tests::product_tool_runtime_preserves_core_owned_registry_contract ... ok
test agentic::tools::registry::tests::capability_summary_total_matches_registered_count ... ok

test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 1015 filtered out; finished in 0.01s
```

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --lib --features product-full tools::restrictions
```
输出：
```text
running 6 tests
test agentic::tools::restrictions::tests::runtime_restrictions_allow_all_when_empty ... ok
test agentic::tools::restrictions::tests::denied_tool_names_override_allow_list ... ok
test agentic::tools::restrictions::tests::tool_restriction_errors_map_to_tool_errors ... ok
test agentic::tools::restrictions::tests::remote_posix_roots_require_true_containment ... ok
test agentic::tools::restrictions::tests::custom_deny_message_overrides_generic_runtime_error ... ok
test agentic::tools::restrictions::tests::local_path_containment_handles_missing_children ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1031 filtered out; finished in 0.00s
```

### 5. cargo test -p northhing-product-capabilities
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-product-capabilities
```
输出：
```text
running 5 tests
test capability_assembly_reports_missing_services_without_concrete_runtime_dependency ... ok
test default_capability_assembly_keeps_service_facts_together ... ok
test capability_packs_describe_service_requirements ... ok
test product_assembly_plan_reports_service_availability_by_capability ... ok
test product_assembly_plan_makes_delivery_profile_explicit_without_reducing_capabilities ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 6. 收束自检
```powershell
rg -n "InitMiniApp|miniapp_init_tool|is_miniapp_headless|miniapp_headless" src/ tests/ scripts/
```
输出：
```text
src/crates\execution\agent-stream\src\tool_call_accumulator.rs:150:            ("InitMiniApp", "Markdown Viewer"),
```
（注：`tool_call_accumulator.rs:150` 属于 M5 红线保护范围，本批保留不动，符合 brief 要求；其余全部零命中。）

`scripts/core-boundaries` 规则行数检查：
- 改前：474 行
- 改后：474 行（M1 项无独立规则锚点，M2-M5 规则锚点全部保留）

### 7. git status --short
```powershell
git status --short
```
输出：
```text
 M .opencode/model-capability-notes.md
 M memory/northhing.md
 M src/crates/assembly/core/src/agentic/agents/mod.rs
 M src/crates/assembly/core/src/agentic/coordination/coordinator.rs
 M src/crates/assembly/core/src/agentic/coordination/dialog_turn/compaction.rs
 M src/crates/assembly/core/src/agentic/coordination/dialog_turn/session.rs
 M src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs
 M src/crates/assembly/core/src/agentic/coordination/dialog_turn/thread_goal.rs
 M src/crates/assembly/core/src/agentic/coordination/dialog_turn/workspace.rs
 M src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_dispatch.rs
 M src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_types.rs
 M src/crates/assembly/core/src/agentic/tools/agent-tool-exposure.md
 D src/crates/assembly/core/src/agentic/tools/implementations/miniapp_init_tool.rs
 M src/crates/assembly/core/src/agentic/tools/implementations/mod.rs
 M src/crates/assembly/core/src/agentic/tools/mod.rs
 M src/crates/assembly/core/src/agentic/tools/product_runtime/materialization.rs
 M src/crates/assembly/core/src/agentic/tools/registry/tests.rs
 M src/crates/assembly/core/src/agentic/tools/restrictions.rs
 D src/crates/assembly/core/src/service/announcement/content/tips/en-US/013_miniapp.md
 D src/crates/assembly/core/src/service/announcement/content/tips/zh-CN/013_miniapp.md
 D src/crates/assembly/core/src/service/announcement/content/tips/zh-TW/013_miniapp.md
 M src/crates/assembly/product-capabilities/src/lib.rs
 M src/crates/assembly/product-capabilities/tests/product_capabilities.rs
 M tests/e2e/specs/l0-navigation.spec.ts
 M tests/e2e/specs/l1-navigation.spec.ts
?? .handoffs/handoff-g2-t9-2026-08-07.md
?? .superpowers/sdd/task-t2-2-miniapp-recon.md
?? .superpowers/sdd/task-t2-2k-brief.md
?? .superpowers/sdd/task-t2-2k-report.md
```

---

## 编译错误层级说明
- 无编译错误（机制层/设计层一次性通过）。

## 偏离说明
- 零偏离。所有改动与删除均严格遵守 brief 清单 A-E 与红线要求。
