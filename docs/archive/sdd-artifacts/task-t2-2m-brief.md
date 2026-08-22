# Task T2-2m Brief — MiniApp 整删 M3：services-integrations miniapp 整删

> 批次权威文档：`.superpowers/sdd/task-t2-2-miniapp-recon.md`（Q1-A services 清单 / Q4 feature-rules 细节 / Q8-M3）。
> 前置 M1（a930c93）+ M2（980c879）已并 main：core 层 miniapp 已删光，core 对 services-integrations 的 miniapp-runtime feature 引用已断（M2 已改 function-agents）。本批删 services-integrations 层 miniapp 本体（2,989 行）+ miniapp-runtime feature 块 + 对应 boundary 锚点 + 就近 AGENTS 文档措辞。

## 工作目录
`E:\agent-project\northing`（git main，HEAD=6d6b86c）

## 环境硬事实（必读）
- cargo 一律 MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`（仓库 override 是 GNU，`cargo +toolchain` 不可用）。
- 中文文件编辑一律用 edit 工具（PowerShell 写非 ASCII 会 GBK 双重编码）。
- 工作区有并行 session 未提交改动（`memory/`、`.opencode/model-capability-notes.md`、`.handoffs/`），勿碰勿提交。
- 行号来自 recon（基线 3702baf）；M1/M2 未动本批文件，行号漂移时以符号搜索为准；目标内容与 recon 描述不一致则 STOP 报告。

## 变更清单（全部显式授权）

### A. miniapp 目录整删
1. 整删目录 `src/crates/services/services-integrations/src/miniapp/`（11 文件 2,989 行：mod.rs(11) / host_dispatch.rs(691) / storage.rs(165) / storage_port.rs(124) / storage_app_io.rs(313) / storage_drafts.rs(237) / storage_imports_io.rs(122) / storage_tests.rs(544) / builtin_io.rs(164) / worker.rs(177) / worker_pool.rs(441)）。
2. `src/crates/services/services-integrations/src/lib.rs:27-28`：删 `#[cfg(feature = "miniapp-runtime")] pub mod miniapp;` 两行。

### B. Cargo.toml feature 摘除（⚠️ 不动 [dependencies]）
3. `src/crates/services/services-integrations/Cargo.toml`：
   - 删 `miniapp-runtime` feature 块（recon :78-87，含 :80 `northhing-product-domains/miniapp` 引用行）
   - `product-full` feature（recon :121）的列表中摘除 `"miniapp-runtime"` 名字（feature 本体保留）
   - **[dependencies] 一律不动**——recon Q4 实测 miniapp 关联 optional dep 全部为共享 owner（base64/reqwest→mcp、dirs/uuid→remote-ssh-concrete、which→workspace-search），无 orphan。

### C. boundary 规则同步（同 commit，house rule 2）
4. `scripts/core-boundaries/rules/feature-rules.mjs`：
   - :50,52,56,59,65,77,78 dep→ownerFeatures 表中含 `miniapp-runtime` 的条目——把 `miniapp-runtime` 从各 ownerFeatures 数组摘除（dep 行保留，其它 owner feature 保留）
   - :141 services-integrations requiredProductFullFeatures 数组摘 `'miniapp-runtime'`
5. `scripts/core-boundaries/rules/source/required-rules.mjs` / `forbidden-rules.mjs` / `scripts/core-boundaries/self-test.mjs`：摘 services-integrations 层 miniapp 锚点（`rg -i miniapp scripts/core-boundaries` 逐条过，本批只动归属 services 层的）。
6. **保留**：product-domains 层 miniapp 锚点（M4 才删）；feature-rules.mjs :86-88（product-domains dirs/sha2/which 独占 miniapp）与 :151 保留到 M4。

### D. 就近文档措辞（house rule 2 同 commit）
7. `src/crates/services/services-integrations/AGENTS.md:34-37`：miniapp 相关段落摘除/改写为不含 miniapp 的描述。
8. `src/crates/services/AGENTS.md:7,15,22` + `src/crates/services/AGENTS-CN.md:5,12,17`：miniapp/MiniApp runtime IO 措辞同步清理（保持其余行不动；中英文两版同步）。

## 不做（红线）
- 不碰 product-domains 任何内容（M4）；不碰 core（M2 已清）；不动 [dependencies]。
- 不动 feature-rules.mjs :86-88 与 :151（M4 范围）。
- 不碰 serde 变体、tool_call_accumulator.rs、i18n 脚本、根 AGENTS.md/CN（M5）、并行 session 文件。
- 不做清单外改动与无关格式化。

## 验证（每条都跑，report 贴命令+输出原文）
1. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace` → PASS
2. feature 组合抽验（共享 dep 不受损）：
   - `... cargo check -p northhing-services-integrations`（默认）→ PASS
   - `... cargo check -p northhing-services-integrations --features remote-ssh,remote-ssh-concrete` → PASS
   - `... cargo check -p northhing-services-integrations --no-default-features` → PASS（若该 crate 有 default feature；无则说明）
3. `node scripts/check-core-boundaries.mjs` → PASS（含 self-test）
4. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-services-integrations --lib` → PASS（存活测试回归）
5. 收束自检：`rg -l -i miniapp src/crates/services/` → 零命中（含 AGENTS.md/CN）；`rg -i miniapp scripts/core-boundaries | wc -l` 贴改前（293）/改后计数，改后应仅剩 product-domains 层锚点。
6. `git status --short` → 仅本批清单文件 + 并行 session 预存改动。

## Report
写 `.superpowers/sdd/task-t2-2m-report.md`：每项文件:行+前后摘要、验证命令+输出原文、编译错误修在哪一层（机制层/设计层，一行一个）、偏离说明。假汇报=停用。不要自己 commit。最终状态 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
