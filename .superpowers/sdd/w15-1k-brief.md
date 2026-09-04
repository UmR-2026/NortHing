# W15-1k Brief — rot 闸红修复：两处 god-file 纯位移瘦身（app.rs / memory_db.rs）

## 1. 来源与验收标准（逐字）

来源 = CI run 33872662968 rot budget check 红 + 编排者本地复现（BASE 上 `pnpm run check:rot` 输出原文）：

```
god_file:src/apps/desktop/src/ui_dioxus/app.rs: current 847 exceeds ceiling 800
god_file:src/crates/assembly/core/src/service/agent_memory/memory_db.rs: current 920 exceeds ceiling 894
```

成因：W15-1i（F1 挪窝）把 app.rs 推过 800；W15-1h 续单（WAL busy 重试）把 memory_db.rs 推过其登记 ceiling 894。家规⑦：ceiling 只降不升、登记新条目需用户拍板——本单走**纯位移瘦身**（不碰语义），不抬线。

验收标准（逐条可机械核对）：
1. `app.rs` ≤ 800 行（以 `pnpm run check:rot` 判定为准）。
2. `memory_db.rs` ≤ 其 manifest ceiling（当前 894）行。
3. `pnpm run check:rot` 全绿（exit 0）。
4. `cargo check --workspace` 绿；`cargo test -p northhing-core --features product-full memory_db` 全绿（缝迁移后 facts/auto_memory/continuity_selfcheck 的测试依赖不能断）。
5. 零行为变化：纯代码搬移 + import/mod 声明调整 + ceiling 下调；不改任何函数体语义。

## 2. 编排者预检结论（直接采信，勿重复侦察）

### Target A — app.rs（847 行，需减 ≥47）

- 搬运对象：`spawn_module_window`（`app.rs:654-663`）+ `spawn_module_window_with_theme_rx`（`app.rs:665-776`），共 ~123 行（含 I2/T7 证据注释，**注释随代码走，一字不丢**）。
- 调用方：`app.rs` 内部 5 处（:419/:431/:443/:575/:590 区域 + :628/:643），无外部调用（codegraph 已核）。
- 落点（复用优先）：`window_ops.rs`（现 91 行，名字即"窗口操作"，主题契合则落这里）或 `windows/` 目录模块的新子文件（`windows/mod.rs` 已存在，含 facility/self_app/work）。**先读 window_ops.rs 判断主题契合度**，二选一，report 写理由。
- 依赖符号（新模块需要 use）：`ShellWindowManager`、`GeometryRxArc`、`GlobalTheme`、`shared_webview_data_directory_for_inner`、`startup_scale_factor`、`DockSide`、`DOCK_GAP_PX`、`ModuleAppProps`、tao `WindowBuilder`/`LogicalSize`/`LogicalPosition`、dioxus `Config`/`VirtualDom`/`window()` 等——按原 import 逐一核实来源模块。

### Target B — memory_db.rs（920 行，ceiling 894，需减 ≥26）

- 搬运对象：「Test-only isolation seam」整段（`// ── Test-only isolation seam ──` 注释块 + `thread_local! TEST_MEMORY_DB_PATH` + `test_memory_db_path_override` + `MemoryDbPathGuard` + `with_test_memory_db_path` + `unique_test_memory_db_path` + `impl Drop`），约 55-60 行，全部 `#[cfg(test)]`。
- 落点：同目录新文件 `src/crates/assembly/core/src/service/agent_memory/test_seam.rs`，`#[cfg(test)] pub(crate) mod test_seam;` 挂进 `agent_memory/mod.rs`。
- 依赖改造点：
  - `memory_db.rs` 的 `default_memory_db_path` 内 `#[cfg(test)]` 分支调用 `test_memory_db_path_override()`——改路径引用（如 `super::test_seam::test_memory_db_path_override()`）。
  - `agent_memory/mod.rs:23` 的 re-export `pub(crate) use memory_db::{unique_test_memory_db_path, with_test_memory_db_path, MemoryDbPathGuard};` → 改从 `test_seam` re-export（消费方 facts.rs / auto_memory.rs / continuity_selfcheck.rs 的 import 路径不变 = 零改动）。
- 注意：`test_seam.rs` 整个文件 `#[cfg(test)]`，新文件行数远低于 800，无需登记 manifest。

### Ceiling 棘轮（家规⑦授权范围）

- `scripts/rot-budget.json` 里 `memory_db.rs` 的 ceiling：**只许下调**。搬迁完成后把 ceiling 调成「新实际行数 + 10」（如实际 860 → 870）。禁止上调任何条目、禁止新增 >800 行登记条目（那需要用户拍板，不在本单授权内）。

## 3. 复用侦察（强制）

- 落点选择必须先读 `window_ops.rs` / `windows/mod.rs` 现状再决定（复用既有模块 > 新建文件）。
- report 必须有「复用侦察」节：查了哪些文件、为什么选该落点。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. Target A 搬迁完成，app.rs ≤800，spawner 全部调用点改新路径，行为不变。
2. Target B 搬迁完成，memory_db.rs ≤894，缝的三处消费方（facts.rs:664/727、auto_memory.rs 多处、continuity_selfcheck.rs:98）零改动继续工作。
3. rot-budget.json 的 memory_db.rs ceiling 下调至「新实际+10」；其它条目不动。
4. 验证四条全绿（§1 验收 3-4）。
5. 每个被移代码块的注释（含 I2/T7 证据注释、缝的设计说明注释）随块完整搬迁。

**明确界外（不要碰，越界即 judge Critical）**：
- 除上述文件外的一切；尤其不改任何函数体语义、不动测试断言、不动 ci.yml、不动其它 rot-budget 条目。
- 不顺手清理无关代码（哪怕看到明显的）。

## 5. Global Constraints（逐字遵守）

- 禁整树 git 操作：禁止 `git restore .` / `git checkout .` / `git stash` / `git add -A`，只许点名文件 add/commit。
- 测试必须真实执行：report 贴验证命令真实输出原文。
- 本任务不碰真实用户配置/数据。

## 6. 验证（命令 + 输出原文都要进 report）

仓库根 `E:\agent-project\NortHing`：

```
pnpm run check:rot
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo check --workspace
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full memory_db
```

（编排者已在 BASE `4f2a564` 预跑：check:rot 红（两条违规原文在 §1）、cargo check 绿、memory_db 测试绿——基线已立。）

## 7. 报告

写到 `E:\agent-project\NortHing\.superpowers\sdd\reports\w15-1k-report.md`。含：改动摘要（含搬迁前后行数实测）、Spec 逐条自核、复用侦察节、落点选择理由、每个编译错误修在哪一层（机制层/设计层）、验证命令+输出原文、god-file 健康度观察一句（memory_db.rs 是登记观测点：本次更纠结/持平/更清晰+依据）。结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## 8. 派发元信息

- BASE commit：`4f2a564`（main 本地 HEAD，未推送——推送由编排者收口时执行）。
- **允许文件集**（diff 越出 = judge Critical）：
  - `src/crates/assembly/core/src/service/agent_memory/memory_db.rs`
  - `src/crates/assembly/core/src/service/agent_memory/mod.rs`
  - `src/crates/assembly/core/src/service/agent_memory/test_seam.rs`（新建）
  - `src/apps/desktop/src/ui_dioxus/app.rs`
  - `src/apps/desktop/src/ui_dioxus/window_ops.rs` 或 `src/apps/desktop/src/ui_dioxus/windows/` 下新子文件（二选一，含 `windows/mod.rs` 声明改动）
  - `src/apps/desktop/src/ui_dioxus/mod.rs`（仅当新增模块声明需要）
  - `scripts/rot-budget.json`（仅 memory_db.rs ceiling 下调）
- 禁区：其它一切文件。
- commit 规则：点名 `git add`；message：`refactor: ... (W15-1k)`（可拆两个 commit：core 缝迁移 + desktop spawner 迁移）。
- 长命令纪律：cargo/pnpm 一律 PTY 或重定向。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源，优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill，trace 到设计层原因再改——禁止无脑 .clone() / .unwrap() 糊住编译器。
3. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。

## Skill 前置阅读（约束输入，不是需求输入）

- `E:\agent-project\.opencode\skills\long-running-shell\SKILL.md`（Windows 下 cargo/pnpm 长命令纪律）

遵循其中与本任务相关的约定，不因此扩展任务范围。
