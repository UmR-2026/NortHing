# Task T2-2l Brief — MiniApp 整删 M2：assembly/core miniapp 整删

> 批次权威文档：`.superpowers/sdd/task-t2-2-miniapp-recon.md`（Q1-A core 清单 / Q3 feature 链 / Q8-M2）。
> 前置 M1（T2-2k）已完成：agent 面入口已灭，miniapp_init_tool.rs（path_manager miniapp 方法的消费方之一）已删。
> 本批删除 core 层 miniapp 子系统本体（2,349 行）+ feature 链 core 段 + path_manager miniapp 方法 + 对应 boundary 锚点。

## 工作目录
`E:\agent-project\northing`（git main，HEAD=dd2edd5）

## 环境硬事实（必读）
- cargo 一律 MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`（仓库 override 是 GNU，`cargo +toolchain` 不可用）。
- 中文文件编辑一律用 edit 工具（PowerShell 写非 ASCII 会 GBK 双重编码）。
- 工作区有并行 session 未提交改动（`memory/`、`.opencode/model-capability-notes.md`、`.handoffs/`），勿碰勿提交。
- 行号来自 recon（基线 3702baf），M1 未动本批文件，但行号漂移时以符号搜索为准；目标内容与 recon 描述不一致则 STOP 报告。

## 变更清单（全部显式授权）

### A. core miniapp 目录整删
1. 整删目录 `src/crates/assembly/core/src/miniapp/`（14 文件 2,349 行：mod.rs / host_dispatch.rs / js_worker.rs / js_worker_pool.rs / runtime_detect.rs / compiler.rs / exporter.rs / storage.rs / builtin/mod.rs / manager/mod.rs / manager/mgr_types.rs / manager/mgr_registry.rs / manager/mgr_runtime.rs / manager/mgr_lifecycle.rs）。
2. `src/crates/assembly/core/src/lib.rs:17-18`：删 `#[cfg(feature = "product-domains")] pub mod miniapp;` 两行。**严禁动 :14-15 的 function_agents 门控**（存活功能）。

### B. core Cargo.toml feature 链（只抽两行，别动整个 feature）
3. `src/crates/assembly/core/Cargo.toml` product-domains feature 块（recon 基线 :197-203）：
   - 删整行 `"northhing-services-integrations/miniapp-runtime"`（recon :201）
   - 把 `"northhing-product-domains/product-full"` 改为 `"northhing-product-domains/function-agents"`（recon :202）
   - 块内其余行一律不动。

### C. core 内残余耦合点
4. `src/crates/assembly/core/src/product_domain_runtime.rs`：删 :14 use 行的 miniapp 导入 + :25-27 `miniapp_runtime_facade` 方法（零调用方，recon 实测）。**文件其余方法（function_agents 三方法）存活保留**。
5. path_manager miniapp 方法（M1 后已零消费方）：
   - `src/crates/assembly/core/src/infrastructure/app_paths/path_manager/user_paths.rs:99-106`：删 `miniapps_dir()` / `miniapp_dir(app_id)` 两方法
   - `src/crates/assembly/core/src/infrastructure/app_paths/path_manager/init.rs:35`：删每次启动创建 `data/miniapps/` 目录的行（启动副作用摘除）
   - `src/crates/assembly/core/src/infrastructure/app_paths/path_manager.rs:9`（或附近）：文档注释中 miniapp 提及同步清理

### D. boundary 规则同步（同 commit，house rule 2）
6. `rg -i miniapp scripts/core-boundaries/`：摘除**本批删除项**（core miniapp 目录文件/符号、core lib.rs `pub mod miniapp`、core Cargo.toml miniapp-runtime 行）对应的规则锚点——已知含 required-rules.mjs :2447（强制 core Cargo.toml 含 miniapp-runtime）、:2495（强制 lib.rs pub mod miniapp）、:5370-6816 大段中属 core 层的部分、forbidden-rules.mjs :480-510（core miniapp facade 禁令）、self-test.mjs 中 core 层锚（含 :613-616 Command::new 例外、:2120/:2134 相关项）。
7. **保留**：services-integrations 层与 product-domains 层的 miniapp 锚点（那两层 M3/M4 才删，本批还活着）。改完 `node scripts/check-core-boundaries.mjs` PASS（含 self-test）。

## 不做（红线）
- 不碰 services-integrations 与 product-domains 的任何 miniapp 内容（M3/M4）。
- 不删 function_agents 任何东西；不删 core `product-domains` feature 本体。
- 不动 product-domains Cargo.toml / services-integrations Cargo.toml。
- 不碰 serde 变体、tool_call_accumulator.rs、i18n 脚本、并行 session 文件。
- 不做清单外改动与无关格式化。

## 验证（每条都跑，report 贴命令+输出原文）
1. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace` → PASS
2. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing` → PASS（desktop 门禁）
3. `node scripts/check-core-boundaries.mjs` → PASS
4. 功能等价抽验：core 仍能编出 function_agents 路径——`cargo test -p northhing-core --lib --features product-full function_agents` → PASS（若有该测试模块；无则用 `cargo check -p northhing-core --features product-full` 佐证并说明）
5. 收束自检：`rg -l -i miniapp src/crates/assembly/core/` → 零命中或仅剩注释残留（report 逐条列出并说明）；`rg -i miniapp scripts/core-boundaries | wc -l` 贴改前（474）/改后计数，改后应显著下降且 >0。
6. `git status --short` → 仅本批清单文件 + 并行 session 预存改动。

## Report
写 `.superpowers/sdd/task-t2-2l-report.md`：每项文件:行+前后摘要、验证命令+输出原文、编译错误修在哪一层（机制层/设计层，一行一个）、偏离说明。假汇报=停用。不要自己 commit。最终状态 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
