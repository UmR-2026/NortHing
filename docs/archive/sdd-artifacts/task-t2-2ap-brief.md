# Task T2-2a' Brief: 拆接线 + 删 tool-provider-groups & harness（E-09 裁决执行）

## Source
- 决策：E-09（2026-08-18 用户拍板，登记册 §3；handoff `docs/handoffs/2026-08-18-t2-1-t2-2a-done-e09.md`）：full-review R-18/SW2-2 的"零调用/直接删"定性被侦察推翻（两 crate 实为活接线），用户裁决**仍删，以计划为准**——执行方式 = 先拆装配面再删 crate。
- 前置侦察：`.superpowers/sdd/task-t2-2a-recon.md` §2（tool-packs）/§8（harness）。
- 本 brief 的行号以当前 main（HEAD `1fdb819`）实测为准；执行前对每项重跑 grep 复核，漂移以实测为准，发现新增消费方 → 对应项跳过并在报告标注。

## 运行时语义（编排者 2026-08-18 已核实，brief 之外不必重查）

1. **tool-packs 是活接线**：全局工具注册表唯一构建路径 =
   `global_tool_registry()` → `ToolRegistry::new()`（`registry_global.rs:20`）→ `ProductToolRuntime::default().create_registry()`（`product_runtime.rs:71-77`）→ `create_product_tool_registry_from_plan(assembly_plan.capability_assembly().tool_provider_group_plan(), decorator)`（`materialization.rs:76-89`）。
   plan 的唯一运行时贡献 = 4 个 provider id + 40 个工具名的**分组与顺序**；具体工具实例由 `ProductConcreteToolFactory::materialize_tool` 的 match（materialization.rs:17-60）按名创建。feature_groups 仅被 product-capabilities 测试断言，无运行时消费。
2. **harness 是纯死脚手架**：core 侧仅 `agentic/harness.rs` facade（re-export + #[cfg(test)] 测试）；product-capabilities 仅装配描述 + 测试构建 registry；provider `execute()` 按设计报错；`agents/definitions/modes/` 无任何 harness 引用（已核，此前 codegraph 报的 `plan` 调用方是 PlanMode 同名误报）。无运行时消费方。
3. provider id 可观测面：`AgentToolRegistry` 保留 provider 分组（tool-contracts `framework/registry.rs:104-111`），core 测试 `registry/tests.rs:260` 断言四个 id。**生产代码无 `.providers()` 消费方**（已核），但为保行为恒等，拆接线必须保留四个组 id 与组内顺序不变。

## 改造设计 C1（core 拆接线——本任务唯一设计决策，照做勿自由发挥）

把 plan 数据源从 tool-packs crate 换成 core 内联常量，**四个 provider id、40 个工具名、分组划分、顺序逐字保持**（逐字复制自 `tool-provider-groups/src/lib.rs:102-162`）：

- `core/src/agentic/tools/product_runtime/materialization.rs`：
  - 删 `use northhing_tool_packs::ToolProviderGroupPlan;`（:9）与 `ProductToolProviderPlanAdapter(ToolProviderGroupPlan)`（:63-74）。
  - 新增 core 内联数据：`pub(in crate::agentic::tools) const PRODUCT_TOOL_GROUPS: &[(&str, &[&str])]`，内容 =
    - `"core.basic"` → `["LS","Read","Glob","Grep","Write","Edit","Delete","ExecCommand","WriteStdin","ExecControl","GetTime"]`
    - `"core.agent"` → `["Task","Skill","AskUserQuestion","TodoWrite","get_goal","create_goal","update_goal","CreatePlan","submit_code_review","GetToolSpec","GetFileDiff","Log"]`
    - `"core.session"` → `["SessionControl","SessionMessage","SessionHistory","Cron"]`
    - `"core.integration"` → `["WebSearch","WebFetch","ListMCPResources","ReadMCPResource","ListMCPPrompts","GetMCPPrompt","GenerativeUI","Git","ReviewPlatform","InitMiniApp","ControlHub","ComputerUse","Playbook"]`
  - 用一个小的 `StaticToolProviderPlan` 实现（持有 `&'static str` id + `&'static [&'static str]` names）替代原 adapter；`create_product_tool_registry_from_plan` 改为**无 plan 参数**（内部遍历 PRODUCT_TOOL_GROUPS），保留 `ToolRuntimeAssembly::with_tool_decorator(...).create_registry_from_static_provider_plans(...)` 调用与原 expect 语义。
- `core/src/agentic/tools/product_runtime.rs`：
  - `ProductToolRuntime` 删 `assembly_plan` 字段、`for_profile`、`with_tool_decorator_and_assembly_plan`；保留 `new()` / `with_tool_decorator()`；`create_registry()` 改调新签名。
  - 删 `product_assembly_plan_for_profile, DeliveryProfile, ProductAssemblyPlan` import（:17，确认无他用后）。
  - 测试调整：`product_tool_runtime_can_consume_explicit_product_assembly_plan` 删除（前提消失）；`product_tool_runtime_registry_preserves_provider_plan_order` 改为断言 core 内联常量的扁平顺序 == registry.tool_names()；`product_tool_runtime_owner_preserves_registry_contract` 保留。
- `core/src/agentic/tools/registry/tests.rs:234-253`：凡以 `default_product_capability_assembly().tool_provider_group_plan()` 为数据源的断言，数据源换成 core 内联 `PRODUCT_TOOL_GROUPS`；**测试意图（注册表内容/顺序/分组断言）必须保留**，断言字面值（含 :260 的四个 provider id）不变。

## 删除清单

### D1. tool-provider-groups crate（package `northhing-tool-packs`，402 rs 行）
- 根 `Cargo.toml` 删 member 行 `"src/crates/execution/tool-provider-groups",`
- 删目录 `src/crates/execution/tool-provider-groups/`（含其 AGENTS.md）
- `core/Cargo.toml`：删 :100 `northhing-tool-packs = { ... optional = true }` 行；:203 `product-full` feature 列表里的 `"tool-packs",`；:232 `tool-packs = ["dep:northhing-tool-packs", "northhing-tool-packs/product-full"]` 整行
- `product-capabilities/Cargo.toml:15` 删 dep 行
- `product-capabilities/src/lib.rs`：
  - :15 `pub use northhing_tool_packs::ToolProviderGroupPlanSelectionError as ProductCapabilityBuildError;` 删；:16 import 删
  - pack struct 的 `tool_provider_group_ids` 字段 + const 构造参数 + accessor（:47,:55,:61,:74-75）删；`CODE_AGENT_TOOL_GROUPS`/`INTEGRATION_TOOL_GROUPS` 常量（:462-463）与各 pack 常量对应实参删
  - `tool_provider_group_ids()`(:369-377 段)、`try_tool_provider_group_plan`(:382-385)、`tool_provider_group_plan`(:387-390) 删
  - `ProductCapabilityAssembly` 的 `tool_provider_group_plan` 字段 + `new` 对应参数 + accessor :312-314 删
  - `try_build_assembly`(:409-)：`self.try_tool_provider_group_plan()?` 是其唯一 fallible 源——拆除后若无其余 fallible 源，`try_build_assembly` 整体塌缩并入 `build_assembly`（infallible），调用方同步
  - `product-capabilities/tests/product_capabilities.rs`：tool-packs 相关断言（:12-21,:104-121,:159-194,:237-253 段等）删/改，保留 capability id / service requirement 覆盖
- 删后归零复核：`rg -n "tool_provider_group|ToolProviderGroup|northhing.tool.packs|northhing_tool_packs" src --glob "*.rs" --glob "Cargo.toml"` → 0

### D2. harness crate（package `northhing-harness`，571 rs 行）
- 根 `Cargo.toml` 删 member 行 `"src/crates/execution/harness",`
- 删目录 `src/crates/execution/harness/`（含 tests/registry.rs、AGENTS.md）
- `core/Cargo.toml:91` 删 dep 行（非 optional）
- 删 `core/src/agentic/harness.rs` 整文件；`core/src/agentic/mod.rs:26` 删 `pub mod harness;`
- `core/src/product_assembly.rs:9` 删 `default_product_harness_registry,`（该行其余 re-export 保留）
- `product-capabilities/Cargo.toml:13` 删 dep 行
- `product-capabilities/src/lib.rs`：
  - :10-12 `use northhing_harness::{...}` 删
  - pack struct 的 `harness_provider_descriptors` 字段 + const 构造参数 + accessor（:48,:56,:62,:78-79）删；各 pack 常量对应实参删
  - `ProductCapabilityAssembly` 同名字段 + `new` 参数 + accessor(:316-318) + `build_harness_registry`(:320-322) 删；`ProductAssemblyPlan::build_harness_registry`(:261-263) 删
  - builder 的 `harness_provider_descriptors()`(:392-403) 与 `build_harness_registry`(:405-407) 删；`try_build_assembly` 里 `self.harness_provider_descriptors()` 实参删
  - provider 常量段（:465-552：`DEEP_REVIEW_HARNESS_PROVIDER`/`DEEP_RESEARCH_HARNESS_PROVIDER`/`MINIAPP_HARNESS_PROVIDER`/`NO_HARNESS_PROVIDERS`/`DEEP_REVIEW_HARNESS_PROVIDERS` 等 + `CORE_DEEP_REVIEW_HARNESS_PROVIDER_ID`/`CORE_DEEP_RESEARCH_HARNESS_PROVIDER_ID`/`CORE_MINIAPP_HARNESS_PROVIDER_ID` 常量定义）整段删
  - `product_harness_registry_for_profile`(:545-549)、`default_product_harness_registry`(:551-553) 删
  - tests：harness 相关断言（:1,:4,:27,:64,:198,:248 段等）删
- 删后归零复核：`rg -n "northhing.harness|northhing_harness|HarnessProvider|HarnessRegistry|HarnessWorkflow|HarnessCapability" src --glob "*.rs" --glob "Cargo.toml"` → 0（注意 product-domains / miniapp 资产里的自然语言 "harness" 命中需人工判别，非代码引用可留）

### D3. boundary 检查器同步（`scripts/core-boundaries/`）
- 先跑 `rg -n -i "tool-packs|tool_packs|tool-provider-groups|northhing-harness|northhing_harness" scripts/core-boundaries/` 取全量命中逐条处理（规则与断言成对删）。已知命中面：
  - `rules/crate-layout.mjs`：tool-packs 条目(:16)、harness 条目(:14)
  - `rules/crate-rules.mjs`：:16 及全部 `'northhing-tool-packs'` forbiddenDeps 字符串（:35,:62,:90,:116,:142,:194,:224,:266,:295,:324,:351 附近）；harness 相关 :9 附近
  - `rules/feature-rules.mjs`：:15,:138,:145-150
  - `rules/source/required-rules.mjs`：:2452,:2499,:2503,:5492
  - `rules/source/forbidden-rules.mjs`：:560,:565（harness）、:570,:575、:2364（tool-packs）
  - `self-test.mjs`：:133,:161,:713-733,:1972-1976,:2313-2326（其中 :2313-2326 断言 core Cargo.toml 含 tool-packs manifest 原文，必删）
- ⚠️ 已知 pre-existing 出界项：`check-core-boundaries.test.mjs` 有 1 条 tool-contracts anchor rule 失败（T2-2a M5，已登记台账）——**不在本任务修**；门禁以 `node scripts/check-core-boundaries.mjs` 绿为准

### D4. 文档同步（同一工作区改动集，家规 2）
- `docs/status/surfaces.md`：删 tool-packs 行(:36)、harness 行(:25)（其 "SDLC Harness" 标签系误标；`docs/sdlc-harness/` 目录是另一物，**勿动**）
- `src/crates/execution/AGENTS.md`：删 harness 行(:18)、tool-provider-groups 行(:20)；`AGENTS-CN.md` 对应行同步删
- 根 `AGENTS.md` L5 行（:27）改写：Owns 列去掉 "harness, " 与 "tool-group, "，Modules 列去掉 `` `harness`, `` 与 `` `tool-provider-groups`, ``；根 `AGENTS-CN.md` 镜像行同步
- `src/crates/assembly/core/AGENTS.md`：删 owner references 里 harness(:78)、tool-provider-groups(:83) 两行；`AGENTS-CN.md` 镜像同步
- `src/crates/execution/tool-execution/AGENTS.md:22`：提及 `northhing-tool-packs` 的句子改写去引用（有 CN 镜像则同步）
- `docs/architecture/core-decomposition.md:66`：mermaid 图 ToolPacks 节点删除（同文件若另有 harness/tool-packs 节点一并处理，`rg -n -i "tool-packs|tool-provider-groups|harness" docs/architecture/core-decomposition.md` 实测）
- `docs/architecture/agent-runtime-services-design.md`：tool-provider-groups 行(:50,:438 附近) 最小同步（删除或改写为不再存在的描述）；⚠️ 该文件有 pre-existing mojibake，**只用 edit 工具做最小改动，不要重写文件**
- 历史文档（CHANGELOG、docs/plans、docs/archive、handoffs）不动

## Constraints
- 不 commit、不 push（编排者统一收口）；改动留在工作区
- 文档同步与代码删除必须在**同一工作区改动集**（家规 2）
- **运行时行为不变量**：全局工具注册表的工具集合（40 个）、顺序、四个 provider id（core.basic/core.agent/core.session/core.integration）、collapsed/expanded 暴露、GetToolSpec 行为全部不变；catalog.rs / snapshot.rs / unlock_state.rs 不在本任务触碰
- 不多拆：product-capabilities 其余 API（capability ids / service requirements / DeliveryProfile / ProductAssemblyPlan 余下方法 / ProductCapabilitySet）保持不动；core 其余装配面不动
- cargo 一律 `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`（默认 GNU 工具链缺 gcc 必失败）；全量 check timeout 给足（冷编译可能 15-30 分钟）
- 勿碰并行 session 资产：`memory/`、`.graph/`、`.opencode/model-capability-notes.md`、`.superpowers/sdd/` 里其它 task-* 文件、前端 session 相关文件、growth 线 worktree
- 排除项勿碰：`judge_gate`、`remote_connect`、`miniapp`、`relay-*`、`tests/e2e/`、`mobile-web`、`docs/sdlc-harness/`
- 若复核发现某删除项出现新增消费方：跳过该项，报告标注，不强行删

## Verification（报告贴原始输出）
1. `cargo check --workspace`（MSVC wrapper）pass
2. `cargo check -p northhing`（MSVC wrapper）pass（家规 6 desktop 门禁）
3. `node scripts/check-core-boundaries.mjs` pass
4. `cargo test -p northhing-product-capabilities`（MSVC）pass
5. `cargo test -p northhing-core --lib` 中 tools 相关 focused 测试 pass（至少覆盖 `product_tool_runtime` 与 `tools::registry` 前缀；报告列出实际命令与结果）
6. `cargo metadata --no-deps --format-version 1` 无解析错误（> nul 即可，报错才会输出）
7. D1/D2 删后归零 grep 输出（命令 + 命中数）
8. `git diff --stat` 摘要；行数对账预期 ≈1.4k+ rs 行（tool-packs 402 + harness 571 + product-capabilities 拆除段 + core 拆除段 - 内联常量约 50 行）

## Report
写 `.superpowers/sdd/task-t2-2ap-report.md`，首行 `DONE` / `DONE_WITH_CONCERNS` / `NEEDS_CONTEXT` / `BLOCKED`。含：逐项执行状态（删了/跳过+原因）、拆接线落地说明、验证原始输出、行数对账、遗留疑虑。报告之外只回状态 + 一行测试摘要 + concerns。
