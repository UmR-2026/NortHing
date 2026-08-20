# Task T3-1a Brief — kernel_facade `list_tools` 接线

> 需求唯一来源。roadmap:212 T3-1 行的第一刀（编排者已把 T3-1 拆分：usage 两项卡 K4b PersistenceManager 折叠、
> load_project_skills 卡契约层 API 形状、list_artifacts/onboarding 缺数据源——全部不在本任务）。

## 背景

`KernelFacade`（`src/crates/assembly/core/src/kernel_facade/`）是 kernel-api trait 的纯透传实现，
当前 12 处 `not yet wired` 桩。本任务只接 `list_tools`（T3-1 优先级清单首位）。

## 侦察事实（编排者亲验，可直接信任）

- 桩：`kernel_facade/tools.rs:9-12`，`Err(KernelError::Internal("not yet wired: list_tools"))`。
- 数据通路：`KernelFacade.coordinator()`（mod.rs:52-58，已初始化则 Ok）→
  `coordinator.tool_pipeline`（**pub 字段**，coordinator.rs:509）→
  `tool_pipeline.tool_registry`（**pub(crate)**，pipeline_types.rs:21；kernel_facade 同 crate，可直接访问）→
  `registry.read().await.all_tools()`（registry_lookup.rs 有 `pub fn all_tools(&self) -> Vec<Arc<dyn Tool>>`）。
- Tool trait（agentic/tools/framework.rs）：`fn name(&self) -> &str`（:18）、
  `async fn description(&self) -> NortHingResult<String>`（:21，**可失败**）、`fn input_schema(&self) -> Value`（:40）。
- DTO：`northhing_kernel_api::tools::ToolInfoDto { id: String, name: String, description: String, input_schema: Option<serde_json::Value> }`
  （contracts/kernel-api/src/tools.rs:10-16）。
- 现有测试基建：`kernel_facade/tests.rs`（601 行），先看其中是否已有带 coordinator 的 facade 测试夹具，复用之。

## 要求的实现

`list_tools`：
1. `let coordinator = self.coordinator()?;`（未初始化错误保持现状语义）
2. 读 registry（`tokio::sync::RwLock` read），`all_tools()`，逐个映射：
   - `id` = `name`（内置工具 name 即唯一标识；仓内无其它 id 概念，勿发明）
   - `name` = `tool.name().to_string()`
   - `description` = `tool.description().await.unwrap_or_default()`——失败降级为空串而非整体 Err
     （目录列举是只读探测，单个工具描述失败不该炸掉整个列表；在代码注释里写明这个取舍，English）
   - `input_schema` = `Some(tool.input_schema())`
3. 输出按 name 排序（`sort_by`），保证确定性。
4. 语义：列**全部已注册**工具（含 collapsed）。collapsed/expanded 是 prompt 暴露面策略，不是目录语义；
   在函数 doc comment 写明（English）。

约束：不得动 kernel-api 契约、不得动 registry/pipeline 的任何签名、不得给 facade 加新字段。
如需 `use` 追加仅限 tools.rs 本文件。

## 测试（kernel_facade/tests.rs 内追加）

- 用现有夹具（若没有带 coordinator 的夹具，参考
  `agentic/coordination/tests/subagent_ports/mod.rs` 的 `build_test_coordinator_with_mock_tool` 构造路径，
  在 kernel_facade/tests.rs 内建最小等价物——注意 facade 是全局 OnceLock 单例，
  `set_coordinator` 幂等（mod.rs:48-50 忽略重复 set），测试间共享无碍）。
- 断言：注册一个 mock tool 后 `list_tools` 返回包含它，字段逐一匹配；多工具时输出按 name 有序。
- 已有测试保持绿。

## 文档同步

roadmap:212 T3-1 行**不动**（只接了 1/5 优先级项，行保持 active；在行尾括号备注追加
`（2026-08-20 list_tools 已接）`——找到该行 `| T3-1 | kernel_facade 10+ \`not yet wired\` 接线（...） | F 交汇+review | M |`，
把内容列末尾的括号内追加该注记，不改状态）。

## 验证（MSVC wrapper）

`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. `cargo check --workspace`
2. `cargo test -p northhing-core --features product-full --lib kernel_facade`
3. `git diff --check`

## 纪律

- 预计改动：kernel_facade/tools.rs + kernel_facade/tests.rs + roadmap.md，共 3 文件；要动第 4 个 → STOP, NEEDS_CONTEXT。
- 日志/注释 English-only。
- git status 里 `.opencode/model-capability-notes.md`、`memory/northhing.md`、`.handoffs/` 是并行 session 产物，勿碰勿提交。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
