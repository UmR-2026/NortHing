# Task G1-T1 Review — 成长核心 crate 骨架 + 层表登记

> 审查者：judge。基准：`f2a16c7` → 头：`7e96126`。
> 需求唯一来源：`E:\agent-project\northing\.superpowers\sdd\task-g1-t1-brief.md`。
> 报告路径：`E:\agent-project\northing\.superpowers\sdd\task-g1-t1-report.md`。

---

## 1. 判决摘要

- **SPEC**: **PASS** — 所有 brief 要求的文件、6 处登记、脚本补断言均严格按 spec 完成。
- **QUALITY**: **PASS** — 代码风格干净（4 空格缩进，与邻近 crate 一致），命名精确，空壳契约自洽，报告完备可追溯。
- **总体**: **APPROVED**

---

## 2. Findings

### Critical

无。

### Important

无。

### Minor

- **`scripts/core-boundaries/checker.mjs:403`** — 新循环对 `northing-installer` 使用 `startsWith('northing-installer')`，**无尾斜杠**。
  这意味着未来若新增一个以 `northing-installer` 为前缀的 workspace 成员（例如 `northing-installer-tools`）会被静默排除。
  本任务按照 brief 精确照抄字符串，故这属于设计决策，不是 implementer 的问题。
  **缓解建议（仅登记，不需要修复）**：未来若计划在该前缀下加成员，将 `northing-installer` 改为 `northing-installer/` 或补更精确的边界判断。文件：`scripts/core-boundaries/checker.mjs:398-414`。

---

## 3. Constraints 逐条核对表

| # | Constraint | 判决 | 依据 |
|---|------------|------|------|
| 1 | **零行为变更**：除 brief 列出的文件外，不得改动任何 `.rs`；特别是 `src/crates/**` 下任何 Rust 文件一行都不该动。 | **PASS** | `git diff f2a16c7 7e96126 --name-only -- '*.rs'` 仅返回 `src/agentic/src/**` 25 个文件；`src/crates/**` 0 个文件改动。 |
| 2 | **空壳契约**：`src/agentic/src/` 下除 `lib.rs` 与 `error.rs` 外，所有文件只允许 `//!` 文档注释以及 4 个 `mod.rs` 中的模块声明。 | **PASS** | 仅 `error.rs` 含 `fn error_display_includes_context`（brief §2.3 显式允许的测试）和 `enum GrowthError`（brief §2.3 显式允许）。其余 23 个 `.rs` 全部只含 `//!` 单行注释；4 个 `mod.rs` 包含 `pub mod xxx;` 模块声明。`\bstruct\b\|\bfn\b\|\btrait\b\|todo!\(\|unimplemented!` 严格匹配仅命中 `error.rs:30`。 |
| 3 | **crate 依赖白名单**：只允许 `async-trait` / `serde` / `serde_json` / `thiserror` / `tracing`（+ dev-dep `tokio`），均 `workspace = true`。 | **PASS** | `src/agentic/Cargo.toml` 依赖段逐字匹配 brief §2.1，含 5 个依赖 + 1 个 dev-dep。无 `northhing-core`、`rusqlite`、`reqwest`、`git2`、`rmcp`、`axum`、`tauri`。`Cargo.lock` 新增的 `northhing-agentic-growth` 包仅依赖 `async-trait / serde / serde_json / thiserror 2.0.18 / tokio / tracing`，与白名单一致。 |
| 4 | **层表插入位置**：新 6 行 Growth core，原 contracts → 7；其它行不动；AGENTS.md 与 AGENTS-CN.md 一致。 | **PASS** | `AGENTS.md:28` 行 6 Growth core；`:29` 行 7 Stable contracts。`AGENTS-CN.md:27` 行 6 成长核心；`:28` 行 7 稳定契约与产品域。git diff 显示原 1-5 行编号与内容均未变化。Boundary rules 中两条文件均在 execution 之后、contracts 之前新增 Growth core 条目（AGENTS.md:38 / AGENTS-CN.md:37）。 |
| 5 | **边界脚本**：新循环排除前缀必须完整包含 `src/crates/`、`src/apps/`、`tools/`、`northing-installer`。 | **PASS** | `checker.mjs:398-414` 的新循环逐字包含 4 个前缀（见上方 f2-Min 指出的尾斜杠缺失问题，不影响本任务）。新循环与既有 `src/crates/` 循环（385-396）职责互补、不重叠、不绕过 — 第一个循环跳过非 `src/crates/`，新循环跳过四个前缀；两个循环共用 `expectedWorkspaceCratePaths` 同一集合。 |
| 6 | **doc sync**：crate 结构变动必须同 commit 更新 `docs/status/surfaces.md`。 | **PASS** | `docs/status/surfaces.md:59` 已在 Active Capability Crates 表末追加 `\| agentic-growth \| src/agentic \| Agent growth core: memory orchestration decisions, semantic weighting, skill promotion (pure logic, no IO) \|`，包含同 commit。 |
| 7 | **日志与注释 English-only、无 emoji**（中文仅限 `AGENTS-CN.md` 既有中文语境）。 | **PASS** | 所有新建 `*.rs`（25 个）全 ASCII。`AGENTS-CN.md` 中文语境保留。`Cargo.toml`、`crate-layout.mjs`/`crate-rules.mjs`/`checker.mjs` 注释全英文。Hex dump 抽查 `distill/prompt.rs` 全 ASCII，无 emoji。 |
| 8 | **未跑 `cargo fmt`**：diff 中若出现与本任务无关文件的纯格式改动即违规。 | **PASS** | 改动文件清单除 Cargo.lock（仓库新增成员 cargo 自动产物）和新增的 `src/agentic/**`/`AGENTS*`/`Cargo.toml`/`surfaces.md`/3 个 .mjs 之外，无其它文件改动。`scripts/core-boundaries/rules/crate-rules.mjs` 仅追加 1 行（12 行 diff 全为新增行，无重排）；`crate-layout.mjs` 仅在 2 处插入（6 + 3 行新增）；`checker.mjs` 仅插入 18 行。三个 mjs 改动均紧贴 brief 指定位置且无格式副作用。 |
| 9 | **只 commit 范围内文件；`.superpowers/sdd/**` 不应进 commit**。 | **PASS** | `git diff f2a16c7 7e96126 --name-only` 列出 33 个文件，全在 brief §3 与 §4 范围内。`Select-String ".superpowers"` 无任何命中。 |
| 10 | **生产 `.rs` < 800 行**。 | **PASS** | 最大文件 `src/agentic/src/error.rs` 34 行；`src/agentic/src/lib.rs` 24 行；其余均为 1-5 行。 |

---

## 4. 无法从 diff 判定的项

报告命令均按 brief §6 执行，本审查者按"报告即证据"原则未重跑：

- `cargo test -p northhing-agentic-growth`（报告输出 1 test passed, 0 failed）
- `node scripts/check-core-boundaries.mjs`（报告输出 "Core boundary check passed."）
- `cargo check -p northhing-agentic-growth`（报告输出无 warning）

**可疑依据**：上述输出无法被本次审查独立验证，但内容与 brief §1 期望一致、报告格式规范、命令路径正确，且与 diff 中实际新建的文件结构自洽（如 `error.rs:30` 的 `#[test] fn error_display_includes_context` 一定会通过其内置断言）。建议 CI 复跑这三项。

**约束判定**：

- **Constraint 1** 严格依赖 `git diff` 列出的文件，PASS 不依赖验证命令。
- **Constraint 5** 中"通过 = 新循环不误报现有成员"这一面，依赖 `node check-core-boundaries.mjs` 的实际通过（报告证据）。
- **其余约束**全部由文件/diff 静态证据覆盖。

---

## 5. 对下一步（5 个并行任务填空壳）的风险提示

1. **依赖坐标变动**：后续任务填入 `ports.rs` / `state.rs` 等时，按 brief §2.1 列出的 5 个 crate-level 依赖（`async-trait`、`serde`、`serde_json`、`thiserror`、`tracing`）是允许的全部运行时依赖；如果某个空模块需要额外依赖，必须先回到本任务扩 `Cargo.toml`（或者使用现有五个之一），否则会破坏约束 3。

2. **AGENTS.md 边界规则已固化**：AGENTS.md:38 与 AGENTS-CN.md:37 已经写入 "Growth core owns agent growth decisions... must not depend on assembly, services, adapters, or interfaces, and must not own storage implementations." 后续任务填实现时若发现与这些文字冲突，必须先回到本任务升级 AGENTS（即"骨干不变量"变更路径），不能在没有任何文本说明的情况下越权扩展边界。

3. **checker.mjs 排除前缀含 `src/apps/`**：后续若新增 `src/apps/*` 子目录作为 workspace 成员，会被自动跳过登记检查；这是有意为之，避免重复检查。

4. **`northing-installer` 前缀无尾斜杠**：见上方 Minor 段，未来若有同名变体需要小心。

5. **空壳文件只能 `//!` 注释或 `pub mod`**：5 个并行任务填模块时，应当只保留 `lib.rs` 中的 `pub mod xxx;` 与 4 个 `mod.rs` 中的 `pub mod subxxx;`，不要在新填充的文件顶部添加除 `//!` 之外的注释块（保持空壳契约直到下一个工作流任务显式允许）。
