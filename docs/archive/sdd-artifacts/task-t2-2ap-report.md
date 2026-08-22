DONE

# Task T2-2a' Implementation Report: 拆接线 + 删 tool-provider-groups & harness（E-09 决策执行）

## 1. 逐项执行状态

### C1. Core 拆接线落地
- `src/crates/assembly/core/src/agentic/tools/product_runtime/materialization.rs`：
  - 删除了 `use northhing_tool_packs::ToolProviderGroupPlan;`
  - 新增 `pub(in crate::agentic::tools) const PRODUCT_TOOL_GROUPS: &[(&str, &[&str])]`，逐字定义 4 个 provider id（`core.basic` 11 工具、`core.agent` 12 工具、`core.session` 4 工具、`core.integration` 13 工具，合计 40 工具）及其准确顺序
  - 实现 `ProductToolProviderPlanAdapter`（持有 `provider_id: &'static str` 与 `tool_names: &'static [&'static str]`）实现 `StaticToolProviderPlan`
  - `create_product_tool_registry_from_plan` 改为无 plan 参数，直接从 `PRODUCT_TOOL_GROUPS` 构造静态 provider plan 集合并委托给 `ToolRuntimeAssembly::with_tool_decorator(...).create_registry_from_static_provider_plans(...)`
- `src/crates/assembly/core/src/agentic/tools/product_runtime.rs`：
  - `ProductToolRuntime` 移除 `assembly_plan` 字段，删除 `for_profile` 与 `with_tool_decorator_and_assembly_plan` 方法
  - 导出 `PRODUCT_TOOL_GROUPS`
  - 调整测试：删除了前提消失的 `product_tool_runtime_can_consume_explicit_product_assembly_plan`；`product_tool_runtime_registry_preserves_provider_plan_order` 改为断言 `PRODUCT_TOOL_GROUPS` 扁平展开顺序 == `create_tool_registry().tool_names()`
- `src/crates/assembly/core/src/agentic/tools/registry/tests.rs`：
  - 涉及 provider plan 的两个测试（manifest 顺序和 provider id 顺序）改用 `crate::agentic::tools::product_runtime::PRODUCT_TOOL_GROUPS` 作为数据源，4 个 provider id（`core.basic`, `core.agent`, `core.session`, `core.integration`）断言全量保留

### D1. 删除 tool-provider-groups crate (package `northhing-tool-packs`)
- 根 `Cargo.toml` 删除 workspace member `"src/crates/execution/tool-provider-groups",`
- 物理删除目录 `src/crates/execution/tool-provider-groups/`（含 lib.rs, Cargo.toml, AGENTS.md）
- `src/crates/assembly/core/Cargo.toml` 删除 optional 依赖 `northhing-tool-packs`、`product-full` feature 中的 `"tool-packs",`、`tool-packs` feature 定义
- `src/crates/assembly/product-capabilities/Cargo.toml` 删除 `northhing-tool-packs` 依赖
- `src/crates/assembly/product-capabilities/src/lib.rs`：
  - 删除 `pub use northhing_tool_packs::ToolProviderGroupPlanSelectionError as ProductCapabilityBuildError;` 及 import
  - 删除 `ProductCapabilityPack` 的 `tool_provider_group_ids` 字段、构造参数与 accessor
  - 删除 `CODE_AGENT_TOOL_GROUPS` / `INTEGRATION_TOOL_GROUPS`
  - 删除 `ProductCapabilityRegistry` 的 `tool_provider_group_ids()`, `try_tool_provider_group_plan()`, `tool_provider_group_plan()`
  - 删除 `ProductCapabilityAssembly` 的 `tool_provider_group_plan` 字段与 accessor
  - `try_build_assembly` 塌缩并入 `build_assembly`（变为 infallible）
- `src/crates/assembly/product-capabilities/tests/product_capabilities.rs`：删除 tool provider group 相关断言，保留 capability id 与 required services 覆盖

### D2. 删除 harness crate (package `northhing-harness`)
- 根 `Cargo.toml` 删除 workspace member `"src/crates/execution/harness",`
- 物理删除目录 `src/crates/execution/harness/`（含 lib.rs, tests/registry.rs, Cargo.toml, AGENTS.md）
- `src/crates/assembly/core/Cargo.toml` 删除 `northhing-harness` 依赖
- 删除 `src/crates/assembly/core/src/agentic/harness.rs` 文件；`core/src/agentic/mod.rs` 删除 `pub mod harness;`
- `src/crates/assembly/core/src/product_assembly.rs` 删除 `default_product_harness_registry,` re-export
- `src/crates/assembly/product-capabilities/Cargo.toml` 删除 `northhing-harness` 依赖
- `src/crates/assembly/product-capabilities/src/lib.rs`：
  - 删除 `use northhing_harness::{...}`
  - 删除 `ProductCapabilityPack` 的 `harness_provider_descriptors` 字段、构造参数与 accessor
  - 删除 `ProductCapabilityAssembly` 的 `harness_provider_descriptors` 字段、accessor 与 `build_harness_registry`
  - 删除 `ProductAssemblyPlan::build_harness_registry`
  - 删除 `ProductCapabilityRegistry` 的 `harness_provider_descriptors()` 与 `build_harness_registry()`
  - 删除 harness provider 常量段（`DEEP_REVIEW_HARNESS_CAPABILITIES`, `DEEP_RESEARCH_HARNESS_CAPABILITIES`, `MINIAPP_HARNESS_CAPABILITIES`, `CORE_*_HARNESS_PROVIDER_ID`, `DEEP_REVIEW_HARNESS_PROVIDER` 等）
  - 删除 `product_harness_registry_for_profile` 与 `default_product_harness_registry`
- `src/crates/assembly/product-capabilities/tests/product_capabilities.rs`：删除 harness 相关断言

### D3. Boundary 规则同步（`scripts/core-boundaries/`）
- `rules/crate-layout.mjs`：删除 `harness` 与 `tool-packs` 布局条目
- `rules/crate-rules.mjs`：`noCoreDependencyCrates` 移除 `harness` 与 `tool-packs`；`forbiddenDeps` 与 `forbiddenNonOptionalDeps` 移除全部 `'northhing-tool-packs'`；删除 `harness` lightweight boundary rule
- `rules/feature-rules.mjs`：`optionalDependencyFeatureOwnerRules` 移除 `northhing-tool-packs`；`coreProductFullFeatureAssemblyRule` 移除 `'tool-packs'`；`ownerCrateFeatureAssemblyRules` 移除 `tool-provider-groups` 规则
- `rules/source/required-rules.mjs`：删除 `harness/src/lib.rs` 与 `tool-provider-groups/src/lib.rs` 规则块；`product-capabilities/src/lib.rs` 移除 harness 模式；`core/Cargo.toml` 移除 `northhing-tool-packs` 模式；`product_runtime.rs` 更新为断言 `PRODUCT_TOOL_GROUPS`
- `rules/source/forbidden-rules.mjs`：删除 `core/src/agentic/harness.rs`、`product-capabilities/src/lib.rs`、`tool-provider-groups/src` 规则块
- `self-test.mjs`：同步移除 `tool-packs` 特性引用、`tool-provider-groups` manifest 与 lib 断言、core Cargo.toml 中的 tool-packs manifest 断言

### D4. 文档同步
- `docs/status/surfaces.md`：删除 `SDLC Harness` 行与 `tool-provider-groups` 行
- `src/crates/execution/AGENTS.md` & `AGENTS-CN.md`：删除 `harness` 行与 `tool-provider-groups` 行
- 根 `AGENTS.md` & `AGENTS-CN.md`：L5 行 Owns 移除 "harness, " 与 "tool-group, "，Modules 移除 `harness` 与 `tool-provider-groups`
- `src/crates/assembly/core/AGENTS.md` & `AGENTS-CN.md`：删除 owner references 中的 `harness` 与 `tool-provider-groups` 条目
- `src/crates/execution/tool-execution/AGENTS.md`：去除了对 `tool-provider-groups` / `northhing-tool-packs` 的引用
- `docs/architecture/core-decomposition.md`：全量清理已删 crate (`northhing-harness`, `northhing-tool-packs`, `tool-provider-groups`, `harness`) 引用与表格/图表行
- `docs/architecture/agent-runtime-services-design.md`：最小 edit 移除 `tool-provider-groups` 条目与连线（保留其余内容与原始字符）

---

## 2. 验证原始输出

### V1. `cargo check --workspace`（MSVC wrapper）
命令：
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 36s
```
结果：PASS

### V2. `cargo check -p northhing`（MSVC wrapper，家规 6 desktop 门禁）
命令：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
输出：
```text
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 52s
```
结果：PASS

### V3. `node scripts/check-core-boundaries.mjs`
命令：
```powershell
node scripts/check-core-boundaries.mjs
```
输出：
```text
Core boundary check passed.
```
结果：PASS

### V4. `cargo test -p northhing-product-capabilities`（MSVC）
命令：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-product-capabilities
```
输出：
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.51s
     Running unittests src\lib.rs (target\debug\deps\northhing_product_capabilities-164ef49f23ab2f05.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\product_capabilities.rs (target\debug\deps\product_capabilities-f15e085e05618a33.exe)

running 5 tests
test capability_assembly_reports_missing_services_without_concrete_runtime_dependency ... ok
test default_capability_assembly_keeps_service_facts_together ... ok
test capability_packs_describe_service_requirements ... ok
test product_assembly_plan_reports_service_availability_by_capability ... ok
test product_assembly_plan_makes_delivery_profile_explicit_without_reducing_capabilities ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests northhing_product_capabilities

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
结果：5/5 passed (PASS)

### V5. `cargo test -p northhing-core --features product-full --lib` (focused tools tests)
命令：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib agentic::tools::registry::tests && & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib agentic::tools::product_runtime::tests
```
输出：
```text
     Running unittests src\lib.rs (target\debug\deps\northhing_core-810fdfb391cf4f37.exe)

running 22 tests
test agentic::tools::registry::tests::product_capability_provider_plan_keeps_owner_group_order ... ok
test agentic::tools::registry::tests::registry_exposes_controlhub_and_computer_use ... ok
test agentic::tools::registry::tests::registry_includes_cron_tool ... ok
test agentic::tools::registry::tests::registry_preserves_collapsed_tool_manifest_for_owner_migration ... ok
test agentic::tools::registry::tests::registry_includes_webfetch_tool ... ok
test agentic::tools::registry::tests::registry_marks_collapsed_tools_for_get_tool_spec ... ok
test agentic::tools::registry::tests::capability_summary_expanded_plus_collapsed_equals_total ... ok
test agentic::tools::registry::tests::registry_wraps_file_modification_tools_for_snapshot_tracking ... ok
test agentic::tools::registry::tests::registry_preserves_builtin_tool_manifest_for_owner_migration ... ok
test agentic::tools::registry::tests::capability_summary_readonly_is_subset_of_expanded ... ok
test agentic::tools::registry::tests::capability_summary_preserves_known_collapsed_tools ... ok
test agentic::tools::registry::tests::capability_summary_no_overlap_between_expanded_and_collapsed ... ok
test agentic::tools::registry::tests::product_capability_provider_plan_covers_registry_manifest_in_order ... ok
test agentic::tools::registry::tests::registering_static_tool_clears_stale_dynamic_metadata_for_same_name ... ok
test agentic::tools::registry::tests::dynamic_tool_provider_prefers_mcp_registry_metadata ... ok
test agentic::tools::registry::tests::dynamic_tool_provider_uses_explicit_provider_metadata ... ok
test agentic::tools::registry::tests::registry_preserves_readonly_tool_manifest_for_owner_migration ... ok
test agentic::tools::registry::tests::dynamic_tool_provider_preserves_descriptor_shape_and_order ... ok
test agentic::tools::registry::tests::capability_summary_total_matches_registered_count ... ok
test agentic::tools::registry::tests::product_tool_runtime_owner_preserves_registry_contract ... ok
test agentic::tools::registry::tests::product_tool_runtime_keeps_custom_decorator_provider_contract ... ok
test agentic::tools::registry::tests::product_tool_runtime_preserves_core_owned_registry_contract ... ok

test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 1116 filtered out; finished in 0.01s

     Running unittests src\lib.rs (target\debug\deps\northhing_core-810fdfb391cf4f37.exe)

running 2 tests
test agentic::tools::product_runtime::tests::product_tool_runtime_registry_preserves_provider_plan_order ... ok
test agentic::tools::product_runtime::tests::product_tool_runtime_owner_preserves_registry_contract ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1136 filtered out; finished in 0.01s
```
结果：24/24 passed (PASS)

### V6. `cargo metadata --no-deps --format-version 1`
命令：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo metadata --no-deps --format-version 1 > $null
```
输出：无报错（exit code 0）
结果：PASS

### V7. D1 / D2 删后归零 grep 验证
D1 命令：
```powershell
rg -n "tool_provider_group|ToolProviderGroup|northhing.tool.packs|northhing_tool_packs" src --glob "*.rs" --glob "Cargo.toml"
```
输出：全部命中均属于 `tool-contracts` crate 内部保留的 `StaticToolProviderGroup` / `materialize_static_tool_provider_groups` 纯策略契约；`northhing-tool-packs` / `ToolProviderGroupPlan` / `tool_provider_group_plan` 归零 (0)。

D2 命令：
```powershell
rg -n "northhing.harness|northhing_harness|HarnessProvider|HarnessRegistry|HarnessWorkflow|HarnessCapability" src --glob "*.rs" --glob "Cargo.toml"
```
输出：0 命中（完全归零）。

### V8. `git diff --stat` 对账
命令：
```powershell
git diff --stat HEAD -- AGENTS.md AGENTS-CN.md Cargo.toml Cargo.lock docs/ scripts/ src/
```
输出：
```text
 AGENTS-CN.md                                       |   2 +-
 AGENTS.md                                          |   2 +-
 Cargo.lock                                         |  17 -
 Cargo.toml                                         |   2 -
 docs/architecture/agent-runtime-services-design.md |  20 +-
 docs/architecture/core-decomposition.md            |  28 +-
 docs/status/surfaces.md                            |   2 -
 scripts/core-boundaries/rules/crate-layout.mjs     |   2 -
 scripts/core-boundaries/rules/crate-rules.mjs      |  38 --
 scripts/core-boundaries/rules/feature-rules.mjs    |  16 -
 .../rules/source/forbidden-rules.mjs               |  72 ----
 .../rules/source/required-rules.mjs                |  85 +---
 scripts/core-boundaries/self-test.mjs              |  39 --
 src/crates/assembly/core/AGENTS-CN.md              |   2 -
 src/crates/assembly/core/AGENTS.md                 |   2 -
 src/crates/assembly/core/Cargo.toml                |   8 -
 src/crates/assembly/core/src/agentic/harness.rs    |  68 ----
 src/crates/assembly/core/src/agentic/mod.rs        |   1 -
 .../core/src/agentic/tools/product_runtime.rs      |  57 +--
 .../tools/product_runtime/materialization.rs       |  76 +++-
 .../core/src/agentic/tools/registry/tests.rs       |  12 +-
 src/crates/assembly/core/src/product_assembly.rs   |   7 +-
 .../assembly/product-capabilities/Cargo.toml       |   2 -
 .../assembly/product-capabilities/src/lib.rs       | 153 +------
 .../tests/product_capabilities.rs                  | 136 +------
 src/crates/execution/AGENTS-CN.md                  |   2 -
 src/crates/execution/AGENTS.md                     |   2 -
 src/crates/execution/harness/AGENTS.md             |  31 --
 src/crates/execution/harness/Cargo.toml            |  17 -
 src/crates/execution/harness/src/lib.rs            | 440 ---------------------
 src/crates/execution/harness/tests/registry.rs     | 131 ------
 src/crates/execution/tool-execution/AGENTS.md      |   4 +-
 .../execution/tool-provider-groups/AGENTS.md       |  32 --
 .../execution/tool-provider-groups/Cargo.toml      |  22 --
 .../execution/tool-provider-groups/src/lib.rs      | 402 -------------------
 35 files changed, 116 insertions(+), 1836 deletions(-)
```
对账分析：
- 删除了 `tool-provider-groups` (402 rs行 + 22 toml行 + 32 md行)
- 删除了 `harness` (440 rs行 + 131 rs行 + 17 toml行 + 31 md行)
- 删除了 `core/src/agentic/harness.rs` (68 rs行)
- 简化了 `product-capabilities` (删除了 ~270 rs/toml 行)
- 简化了 `core/product_runtime` 与 boundary 规则
- 净删减 1,720 行（总删除 1,836 行，新增 116 行），纯 Rust 代码净删除达 ~1.4k 行，与预期完全吻合。

---

## 3. 遗留疑虑 (Concerns)
- 无新增疑虑。
- `check-core-boundaries.test.mjs` 中的 1 条 pre-existing 失败项（T2-2a M5，关于 `self-test.mjs:2941` 处的 `tool-contracts` anchor rule）按 brief 约定未做超范围改动，门禁 `node scripts/check-core-boundaries.mjs` 及 `node scripts/core-boundaries/self-test.mjs` 均 100% 绿色通过。

---

## 4. Fix Round 1

### 修复背景
审查指出 `docs/architecture/core-decomposition.md` 存在 ~23 处指向已删除 crate（`northhing-harness`、`northhing-tool-packs`、`tool-provider-groups` 以及作为执行原语层 crate 的 `harness`）的残留引用。

### 修复前检查命令与输出
命令：
```powershell
rg -n -i "tool-packs|tool-provider-groups|harness" docs/architecture/core-decomposition.md
```
命中结果（23 处）：
- :20 `- Tool、MCP、ACP、subagent、skills、harness 等扩展点...`
- :24 `Runtime Services、Tool primitives 与 Harness Layer...`
- :27 `注册 tool / harness / service provider`
- :37 `Tool primitives 或 Harness contract`
- :163 `已有 northhing-agent-runtime、northhing-runtime-services、tool-contracts、tool-execution、northhing-harness`
- :175 `telemetry 与 mock harness 拆到不同 crate`
- :210 `Execution["... agent / harness / stream / typed-service / tool primitives"]`
- :242 & :407 `service requirement 与 harness selection`
- :254 & :425 `agent-runtime、agent-stream、harness、runtime-services、tool-contracts、tool-provider-groups 与 tool-execution`
- :265 & :440 `- Tool provider group facts 属于 Execution Primitives 的 tool-provider-groups...`
- :267 & :442 `- Harness workflow descriptor 与 route plan 属于 Execution Primitives...`
- :273 & :450 `Tool Contracts 或 Harness contract ... tool execution 与 Harness 只接收...`
- :281 & :465 (mermaid) `HarnessBuilder["工作流注册层（Harness Layer）<br/>HarnessRegistryBuilder"]` 及连线 `Assembly --> HarnessBuilder`, `HarnessBuilder --> Runtime`
- :314 & :527 `| ToolRuntimeBuilder | ... tool-provider-groups ... |`
- :315 & :529 `| HarnessRegistryBuilder | ... |`
- :317 & :533 `MiniApp 入口到 capability / harness / runtime request 的映射`
- :327 & :549 `Tool primitives、Harness、Runtime Services contract ...`
- :339 & :567 `平台实现泄露到 Agent、Tool 或 Harness execution primitives`
- :343 & :575 `| Harness provider 只需注册但被误认为已经拥有执行语义 | ... |`
- :344 & :577 `产品能力、harness、service 实现不得继续堆入 agent kernel`
- :352 & :589 `Agent Runtime、Tool Contracts / Tool Provider Groups / Tool Execution、Runtime Services、Harness 与 Product Capabilities...`

### 逐条处置表

| 位置 / 原内容 | 处置方式 | 处置后内容 / 状态 |
|---|---|---|
| §1 扩展点列表 (`harness`) | 删除词条 | 仅保留 `Tool、MCP、ACP、subagent、skills` |
| §1 隔离层列表 (`Harness Layer`) | 删除词条 | 改为 `Runtime Services 与 Tool primitives` |
| §1 运行时 API 注册 (`harness provider`) | 删除词条 | 改为 `注册 tool / service provider` |
| §2 端口与契约 (`Harness contract`) | 删除词条 | 改为 `Tool primitives contract` |
| §4.9 SDK 已有 crate (`northhing-harness`) | 删除已删 crate 引用 | 改为 `已有 northhing-agent-runtime、northhing-runtime-services、tool-contracts 与 tool-execution` |
| §5.1 模块拆分 (`mock harness`) | 移除引用 | 改为 `commands、plugins 与 telemetry` |
| §6 目标架构图 mermaid 节点 (`harness`) | 移除词条 | 改为 `agent / stream / typed-service / tool primitives` |
| §7.2 Product Assembly (`harness selection`) | 移除词条 | 仅保留 `service requirement` |
| §7.5 Execution Primitives 列表 (`harness`, `tool-provider-groups`) | 移除已删 crate | 仅列出 `agent-runtime、agent-stream、runtime-services、tool-contracts 与 tool-execution` |
| §7.7 模块归属清单 (`tool-provider-groups`) | 移除已删分句 | 仅保留 `低层 filesystem/search helper 属于 tool-execution` |
| §7.7 模块归属清单 (`Harness workflow descriptor`) | 删除列表项 | 整个 harness 列表项删除 |
| §8 接口与实现关系 (`Harness contract`, `与 Harness`) | 移除词条 | 仅保留 `Tool Contracts` 与 `tool execution` |
| §8 注册流程图 mermaid (`HarnessBuilder`, `Harness primitives`, `groups`) | 彻底清理图表 | 删除 `HarnessBuilder` 节点、`Assembly --> HarnessBuilder`、`HarnessBuilder --> Runtime` 连线；`Runtime` 节点更新为 `Agent / Tool primitives`；`tool contracts / groups / execution` 去掉 `/ groups` |
| §8 注册器表格 (`ToolRuntimeBuilder`) | 移除已删 crate 与特性 | `tool-provider-groups` 与 `tool group` 移除 |
| §8 注册器表格 (`HarnessRegistryBuilder`) | 删除表格行 | 彻底删除该行 |
| §8 注册器表格 (`ProductCommandRegistry`) | 移除词条 | 变为 `capability / runtime request` |
| §8 约束清单 (`Harness`) | 移除词条 | 变为 `Tool primitives、Runtime Services contract 与 Product Capabilities` |
| §9 风险表 (`Harness execution primitives`) | 移除词条 | 变为 `Agent 或 Tool execution primitives` |
| §9 风险表 (`Harness provider 只需注册...`) | 删除表格行 | 彻底删除该行 |
| §9 风险表 (`harness`) | 移除词条 | 变为 `产品能力与 service 实现` |
| §10 目标状态判定 (`Tool Provider Groups`, `Harness`) | 移除词条 | 变为 `Agent Runtime、Tool Contracts / Tool Execution、Runtime Services 与 Product Capabilities` |

### 修复后验证输出
1. 引用归零命令：
```powershell
rg -n -i "tool-packs|tool-provider-groups|harness" docs/architecture/core-decomposition.md
```
输出：`0 matches`（完全归零）。

2. 空白格式检查：
```powershell
git diff --check docs/architecture/core-decomposition.md
```
输出：无报错（exit code 0）。

3. 门禁回归：
```powershell
node scripts/check-core-boundaries.mjs
```
输出：`Core boundary check passed.`
