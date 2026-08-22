# Task T2-2k Brief — MiniApp 整删 M1：入口摘除（agent 面 + 用户面，先灭拉起路径）

> 批次划分权威文档：`.superpowers/sdd/task-t2-2-miniapp-recon.md`（Q3 活消费图谱 / Q5 测试面 / Q8 批次划分 M1）。
> 决策基线：P-14 MiniApp 整删已生效（decision-register.md:40）；roadmap:96 要求"删除前先摘除所有启动入口"——本批即该前置。
> 本批之后：agent 不再可见 InitMiniApp 工具、capability 选择无 miniapp、announcement 无安利卡、headless 假通路灭。子系统本体（core/services/product-domains 的 miniapp 目录）**本批不删**，后续 M2-M4 处理。

## 工作目录
`E:\agent-project\northing`（git main，HEAD=3702baf）

## 环境硬事实（必读）
- cargo 一律 MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`（仓库 override 是 GNU，`cargo +toolchain` 不可用）。
- 中文文件编辑一律用 edit 工具（PowerShell 写非 ASCII 会 GBK 双重编码）。
- 工作区有并行 session 未提交改动（`memory/`、`.opencode/model-capability-notes.md`、`.handoffs/`），勿碰勿提交。
- 行号来自 recon（基线 3702baf），若漂移以符号搜索为准；每处改动前先确认目标内容与 recon 描述一致，不一致则 STOP 报告。

## 变更清单（全部显式授权；除此之外不得动任何文件）

### A. InitMiniApp 工具摘除（agent 可见面）
1. **删文件** `src/crates/assembly/core/src/agentic/tools/implementations/miniapp_init_tool.rs`（整删，约 221 行）。
2. `src/crates/assembly/core/src/agentic/tools/implementations/mod.rs:51,93`：删 `mod miniapp_init_tool;` 声明与 `pub use` re-export 行。
3. `src/crates/assembly/core/src/product_runtime/materialization.rs`：:61 collapsed 列表摘 `"InitMiniApp"`、:111 工厂臂 `"InitMiniApp" => ...` 整臂删除（:4 的 `implementations::*` 通配 import 不动）。
4. `src/crates/assembly/core/src/agentic/agents/mod.rs:96`：从 agent 工具清单删 `"InitMiniApp".to_string()` 一行。
5. `src/crates/assembly/core/src/agentic/tools/agent-tool-exposure.md:44`：删 `| InitMiniApp | Collapsed | None | - |` 整行。
6. `src/crates/assembly/core/src/agentic/tools/registry/tests.rs`：:209 工具列表里的 InitMiniApp 项、:349 `assert!(!registry.is_tool_collapsed("InitMiniApp"))` 断言行——两处删除，其余测试不动。

### B. headless 假限制通路摘除（半死代码；标记无生产者，检测恒 false——recon Q3 实测）
7. `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs:157-158`：删 `is_miniapp_headless_agent_run` 调用分支（死分支），并清理该文件顶部对应 import（若因此变为未使用）。
8. `src/crates/assembly/core/src/agentic/tools/restrictions.rs`：删 :8-88 的 `is_miniapp_headless_agent_run` + `miniapp_headless_agent_tool_restrictions` 两个函数、:149-167 的 2 个 miniapp_headless_* 测试。**文件不可整删**——ToolPathPolicy / ToolRuntimeRestrictions / `is_local_path_within_root` 等（:2-4 re-export 与 :95 起）全部存活保留。
9. `src/crates/assembly/core/src/agentic/tools/mod.rs:39-42`：`pub use restrictions::{...}` 组内摘掉刚删的 2 个名字（保留其余）。
10. **7 个死 import 行清理**（这些文件 import 了 miniapp headless 符号但从未调用）：`agentic/coordination/coordinator.rs:36`、`dialog_turn/compaction.rs:41`、`session.rs:41`、`thread_goal.rs:41`、`workspace.rs:41`、`subagent_orchestrator/so_dispatch.rs:17`、`subagent_orchestrator/so_types.rs:8`。每个文件只删 import 组内的 miniapp 符号名；若整行 import 因此为空则删整行。逐文件 cargo check 佐证无未使用 import 警告新增。

### C. product-capabilities 的 MiniApp capability 摘除
11. `src/crates/assembly/product-capabilities/src/lib.rs`：:17 `MiniApp` 变体、:26 `=> "miniapp"` 映射臂、:366-371 `MINIAPP_SERVICES` 常量、:386-389 在 `DEFAULT_PRODUCT_CAPABILITY_PACKS` 的注册块——四处全删。
12. `src/crates/assembly/product-capabilities/tests/product_capabilities.rs`：:19,:31,:86 三处含 `"miniapp"` 的断言/列表项同步摘除（保持其余断言与语义不变）。

### D. 用户面入口
13. 删 3 个 announcement tips：`src/crates/assembly/core/src/service/announcement/content/tips/en-US/013_miniapp.md`、`zh-CN/013_miniapp.md`、`zh-TW/013_miniapp.md`。（build.rs:303-306 是目录扫描，删文件即生效，无需改代码。）
14. e2e 死选择器清理（src/ 内零对应物，recon Q4 实测）：`tests/e2e/specs/l0-navigation.spec.ts:14`、`tests/e2e/specs/l1-navigation.spec.ts:18,173,194,232` 中引用 `.northhing-nav-panel__miniapp-entry` 的测试步骤/用例——删到该 spec 不再引用此选择器为止，同文件其它用例不动。**授权说明**：e2e 面属冻结敏感面，本项授权仅限"删死选择器引用"，不做任何其它 e2e 改动。

### E. boundary 规则同步（house rule 2，同 commit）
15. `rg -i miniapp scripts/core-boundaries/`：只摘**本批删除项**的规则锚点（例如 InitMiniApp 工具名、miniapp_init_tool.rs 文件、miniapp headless 函数、product-capabilities MiniApp 变体 若被锚定）。**严禁**提前摘除 M2-M5 层的锚点（core/services-integrations/product-domains miniapp 目录与 feature 的规则必须保留——它们强制存在性，那些层本批还活着）。改完必须 `node scripts/check-core-boundaries.mjs` PASS（含 self-test）。

## 不做（红线）
- 不删 core/services-integrations/product-domains 的任何 miniapp 目录/文件（M2-M4 范围）。
- 不动 core Cargo.toml :197-203 feature 块、lib.rs:17-18（M2 范围）。
- 不动 core-types/services-core 的 serde 变体（M5 决策点，未授权）。
- 不动 `tool_call_accumulator.rs:150`（M5 范围）。
- 不动 i18n-audit.mjs / locales.json（M4 范围）。
- 不碰 parallel session 文件；不做清单外"顺手"改动；不做无关格式化（前批教训：越界格式化 hunk 会导致 review FAIL）。

## 验证（每条都跑，report 贴命令+输出原文）
1. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace` → PASS
2. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing` → PASS（desktop 门禁，P2-15 教训）
3. `node scripts/check-core-boundaries.mjs` → PASS
4. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --lib --features product-full tools::registry` 与 `... restrictions` → PASS（focused）
5. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-product-capabilities` → PASS
6. 收束自检：`rg -n "InitMiniApp|miniapp_init_tool|is_miniapp_headless|miniapp_headless" src/ tests/ scripts/` → 零命中（历史 docs 除外）；`rg -i miniapp scripts/core-boundaries | wc -l` 计数应小于改前且 >0（M2-M5 锚点仍在），report 贴前后两个数。
7. `git status --short` → 仅本批清单文件 + 并行 session 预存改动。

## Report
写到 `.superpowers/sdd/task-t2-2k-report.md`：每项变更文件:行+前后摘要、验证命令+输出原文、遇到的每个编译错误修在哪一层（机制层/设计层，一行一个）、任何偏离。假汇报=停用，编排者 diff 逐条核对。不要自己 commit。最终状态 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
