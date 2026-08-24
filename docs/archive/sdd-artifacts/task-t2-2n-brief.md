# Task T2-2n Brief — MiniApp 整删 M4：product-domains miniapp 整删（含内置 6 件套资产 + 专测）

> 批次权威文档：`.superpowers/sdd/task-t2-2-miniapp-recon.md`（Q1-A product-domains 清单 / Q2 内置 6 件套 / Q6 i18n 挂点 / Q8-M4）。
> 前置 M1-M3 已并 main（a930c93 / 980c879 / 111938e）：core 与 services-integrations 层 miniapp 已删光，对 product-domains 的 miniapp feature 引用已全部断开。本批删最后一层本体 + 全部专测 + 55.9k 内置资产 + i18n-audit 扫描挂点 + 剩余 boundary 锚点。
> 本批后 `rg -i miniapp scripts/core-boundaries` 应归零（或仅剩可枚举说明的残留）。

## 工作目录
`E:\agent-project\northing`（git main，HEAD 为 M3 artifacts commit 之后；开工前 `git log --oneline -1` 实测并在 report 记录）

## 环境硬事实（必读）
- cargo 一律 MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`（仓库 override 是 GNU，`cargo +toolchain` 不可用）。
- 中文文件编辑一律用 edit 工具（PowerShell 写非 ASCII 会 GBK 双重编码）。
- **`scripts/i18n-audit.mjs` 有 pre-existing mojibake 语法级损伤**（约 :481 截断字符串，文件本就无法 parse，多批双向实证）。不许修、不许扩大。判据：改动前后各跑 `node --check scripts/i18n-audit.mjs`，必须报**同一个** SyntaxError（行号前移正常）。
- 工作区有并行 session 未提交改动（`memory/`、`.opencode/model-capability-notes.md`、`.handoffs/`），勿碰勿提交。
- 行号来自 recon（基线 3702baf），漂移时以符号搜索为准；目标与 recon 描述不一致则 STOP 报告。

## 变更清单（全部显式授权）

### A. miniapp 目录与专测整删
1. 整删目录 `src/crates/contracts/product-domains/src/miniapp/`（16 个 .rs 共 3,885 行 + `builtin/assets/` 6 应用资产共 55,889 行——含 ppt-live 的 27,805 行 vendored bundle；资产经 include_str! 嵌入，随目录一并消失）。
2. 整删 `src/crates/contracts/product-domains/tests/` 下 6 个 miniapp 专测文件（2,011 行，全部 `#![cfg(feature = "miniapp")]`：`builtin_and_ports.rs`、`compiler_export_storage_and_runtime.rs`、`host_routing_and_lifecycle_helpers.rs`、`permissions_and_bridge.rs`、`runtime_facade_and_customization.rs`、`common/mod.rs`）。若 tests/ 目录因此为空，删空目录。
3. `src/crates/contracts/product-domains/src/lib.rs:7-8`：删 `#[cfg(feature = "miniapp")] pub mod miniapp;` 两行。

### B. Cargo.toml（product-domains）
4. 删 `miniapp` feature 行（recon :22 `miniapp = ["dirs", "sha2", "which"]`）。
5. `product-full` feature（recon :24）改为 `["function-agents"]`（摘掉 `"miniapp"`）。
6. 删 miniapp 独占 optional dep 行：`dirs` / `sha2` / `which`（recon :15,17,18；feature-rules.mjs :86-88 锚定三者独占 miniapp）。**动手前必须 rg 复核**这三者在 product-domains 内确实仅被 miniapp 使用（`rg -n "dirs::|sha2::|which::" src/ tests/` 排除 miniapp 后应零命中）——若有其它使用，STOP 报告，不许删。
7. `Cargo.lock` 同步：若删除 dep 后 lock 出现对应孤儿包，用 `cargo check -p northhing-product-domains` 让 cargo 自动收敛 lock（把 lock 改动一并纳入）；不手工编辑 lock。

### C. i18n-audit 扫描挂点（frozen 面最小授权）
8. `scripts/i18n-audit.mjs`：删 `core-miniapp` locale-format 扫描 spec（recon :1823-1827，5 行对象——root 指向即将消失的 builtin/assets 路径）。**授权仅限此 5 行**；逐字遵守 mojibake 红线（见环境硬事实），除这 5 行外文件中所有字节 byte-preserved。

### D. boundary 规则同步（同 commit，house rule 2）
9. `rg -i miniapp scripts/core-boundaries/` 逐条摘 product-domains 层锚点：`required-rules.mjs` :5370-6816 大段剩余部分、`forbidden-rules.mjs` 剩余 miniapp 段、`self-test.mjs` 剩余 miniapp 锚、`feature-rules.mjs` :86-88（dirs/sha2/which 独占表）与 :151（product-domains requiredProductFullFeatures 的 `'miniapp'`）。
10. 目标：本批后 `rg -i miniapp scripts/core-boundaries` 归零。若有无法归属本批的残留命中，report 逐条列出并说明（M5 终扫兜底）。

### E. 就近文档（house rule 2 同 commit）
11. `src/crates/contracts/product-domains/AGENTS.md:24,29` + `AGENTS-CN.md:17,21`：miniapp 模块/职责描述摘除，中英文同步，其余行不动。

## 不做（红线）
- 不碰 product-domains 的 function-agents 模块与 feature（存活）。
- 不碰 core-types/services-core serde 变体、tool_call_accumulator.rs、根 AGENTS/docs（M5）。
- 不动 i18n-audit.mjs 除 C 项 5 行外的任何字节；不修 mojibake。
- 不碰并行 session 文件；不做清单外改动与无关格式化。

## 验证（每条都跑，report 贴命令+输出原文）
1. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace` → PASS
2. `... cargo test -p northhing-product-domains --no-default-features` → PASS
3. `... cargo test -p northhing-product-domains --features function-agents` → PASS
4. `node scripts/check-core-boundaries.mjs` → PASS（含 self-test）
5. `node --check scripts/i18n-audit.mjs`（改前改后各一次）→ 同一 SyntaxError（行号前移正常）
6. 收束自检：`rg -l -i miniapp src/crates/contracts/` → 零命中；`rg -i miniapp scripts/core-boundaries | wc -l` 贴改前（222）/改后计数（目标 0，非零则逐条归属说明）。
7. `git status --short` → 仅本批清单文件 + Cargo.lock + 并行 session 预存改动。

## Report
写 `.superpowers/sdd/task-t2-2n-report.md`：每项文件:行+前后摘要、验证命令+输出原文、B6 的 rg 复核输出、编译错误修在哪一层（机制层/设计层，一行一个）、偏离说明。假汇报=停用。不要自己 commit。最终状态 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
