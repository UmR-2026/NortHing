# Task G1-T1 Brief — 成长核心 crate 骨架 + 层表登记

> 需求唯一来源。本文件之外的任何信息（包括你的记忆和常识）不得作为需求依据。
> 类型：机械转录 + 登记。**零行为变更**：不改任何现有 Rust 逻辑。

## 0. 工作位置

- 工作目录（唯一）：`E:\agent-project\northing\.worktrees\growth-core-0804`
- 分支：`feat/growth-core-0804`，基线 commit `f2a16c7`
- 只在此 worktree 内改动与提交。**不要**碰 `E:\agent-project\northing`（main 工作区）或其它 worktree。
- 报告写到：`E:\agent-project\northing\.superpowers\sdd\task-g1-t1-report.md`（在 worktree 之外，不进 commit）

## 1. 目标

新建成长核心 crate `northhing-agentic-growth`（物理路径 `src/agentic`），**把全部模块文件以空壳预先声明好**（后续 5 个并行任务各填一个文件，靠预声明避免互相冲突），并在 6 处完成登记，最后给边界检查脚本补一条断言。

验收后此 crate 应：能编译、有 1 个通过的单元测试、通过边界检查脚本、且不依赖 `northhing-core`。

## 2. 新建文件（内容照抄，不要发挥）

### 2.1 `src/agentic/Cargo.toml`

```toml
[package]
name = "northhing-agentic-growth"
version.workspace = true
authors.workspace = true
edition.workspace = true
description = "Agent growth core: memory orchestration decisions, semantic weighting, and skill promotion proposals for northhing"

[lib]
name = "northhing_agentic_growth"
crate-type = ["rlib"]

[dependencies]
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tokio = { workspace = true }

# Prohibited dependencies (enforced by scripts/core-boundaries):
# - northhing-core (any feature)
# - rusqlite (storage stays in the host adapter; this crate is pure decision logic)
# - reqwest / git2 / rmcp / axum / tauri
```

### 2.2 `src/agentic/src/lib.rs`

```rust
//! Growth core: agent growth decisions for northhing.
//!
//! This crate owns the decision side of agent growth: when to distill memory,
//! how to weight topics, which memories to merge or suppress, which experiences
//! become skill candidates, and when the agent updates its own self-cognition.
//!
//! It performs no IO. Storage, LLM access, judge execution, and episode-log
//! access are injected as ports (see `ports`). Host adapters live in
//! `northhing-core::agentic::growth_adapter`.

pub mod distill;
pub mod error;
pub mod executor;
pub mod garden;
pub mod negation;
pub mod ports;
pub mod promote;
pub mod review;
pub mod scheduler;
pub mod selfcog;
pub mod state;
pub mod topics;

pub use error::{GrowthError, GrowthResult};
```

### 2.3 `src/agentic/src/error.rs`

```rust
//! Error type for growth-core decisions.
//!
//! Growth is always warn-only at the host boundary: the host logs these errors
//! and never propagates them into the dialog turn path.

use thiserror::Error;

/// Errors produced by growth-core decision logic and port calls.
#[derive(Debug, Error)]
pub enum GrowthError {
    /// A port implementation failed.
    #[error("growth port failure: {0}")]
    Port(String),
    /// Model output could not be parsed into a trusted shape.
    #[error("growth parse failure: {0}")]
    Parse(String),
    /// Persisted state was unusable and was replaced by defaults.
    #[error("growth state failure: {0}")]
    State(String),
}

/// Convenience result alias for growth-core APIs.
pub type GrowthResult<T> = Result<T, GrowthError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_includes_context() {
        let err = GrowthError::Parse("bad json".to_string());
        assert_eq!(err.to_string(), "growth parse failure: bad json");
    }
}
```

### 2.4 模块空壳（每个文件只放模块文档注释，**不要放任何 struct/fn**）

按下表创建。"注释内容"照抄进文件首行 `//!` 注释（可写成 2-3 行，但必须包含该文字）。

| 文件 | 注释内容（英文） |
|---|---|
| `src/agentic/src/ports.rs` | `Ports injected by the host: memory stores, topic store, LLM, judge, episode log, clock. Filled by task G1-T2.` |
| `src/agentic/src/state.rs` | `Persisted growth state and legacy key migration. Filled by task G1-T2.` |
| `src/agentic/src/scheduler.rs` | `Pure scheduling decisions: when to distill, review, or run the garden pass. Filled by task G1-T4.` |
| `src/agentic/src/executor.rs` | `Executes growth actions through ports. The only IO touch point of this crate. Filled by task G1-T4.` |
| `src/agentic/src/negation.rs` | `Explicit user negation detection. The only path allowed to hard-retire a memory. Filled by task G2-T11.` |
| `src/agentic/src/promote.rs` | `Episode aggregation into skill candidates. Filled by task G3-T15.` |
| `src/agentic/src/selfcog.rs` | `Self-cognition store writes. Agent-exclusive; judge-mom has no access. Filled by task G3-T17.` |
| `src/agentic/src/distill/mod.rs` | `Memory distillation prompt building and strict output parsing. Filled by task G1-T5.` + 模块声明（见下） |
| `src/agentic/src/distill/prompt.rs` | `Distillation prompt construction with user-content isolation. Filled by task G1-T5.` |
| `src/agentic/src/distill/parse.rs` | `Strict JSON parsing of distillation output. Filled by task G1-T5.` |
| `src/agentic/src/topics/mod.rs` | `Topic layer: extraction, two-level scoring, competition groups. Filled by tasks G1-T6 and G2-T8.` + 模块声明 |
| `src/agentic/src/topics/extract.rs` | `Topic extraction from memory text. Filled by task G1-T6.` |
| `src/agentic/src/topics/score.rs` | `Two-level retrieval scoring: topic weight dominant, entry score as fine tuning. Filled by task G1-T6.` |
| `src/agentic/src/topics/competition.rs` | `Competition groups: in-group normalization, suppression, revival. Filled by task G2-T8.` |
| `src/agentic/src/review/mod.rs` | `Judge-mom review: merge-with-boost, routing, verdict parsing. Filled by tasks G2-T9 and G2-T10.` + 模块声明 |
| `src/agentic/src/review/merge.rs` | `Near-duplicate merge that boosts weight instead of adding entries. Filled by task G2-T10.` |
| `src/agentic/src/review/route.rs` | `Routing of review verdicts into weight and relation actions. Filled by task G2-T9.` |
| `src/agentic/src/review/verdict.rs` | `Strict parsing of judge-mom verdict output. Filled by task G2-T9.` |
| `src/agentic/src/garden/mod.rs` | `Garden pass: orphan topic cleanup, synonym merge, cold storage, weight health checks. Filled by task G2-T12.` + 模块声明 |
| `src/agentic/src/garden/cleanup.rs` | `Orphan topic cleanup and synonym merge decisions. Filled by task G2-T12.` |
| `src/agentic/src/garden/health.rs` | `Weight health checks: normalization drift and out-of-range detection. Filled by task G2-T12.` |

四个 `mod.rs` 的模块声明（在注释之后）：

- `distill/mod.rs`：`pub mod parse;` `pub mod prompt;`
- `topics/mod.rs`：`pub mod competition;` `pub mod extract;` `pub mod score;`
- `review/mod.rs`：`pub mod merge;` `pub mod route;` `pub mod verdict;`
- `garden/mod.rs`：`pub mod cleanup;` `pub mod health;`

### 2.5 `src/agentic/AGENTS.md`

新建，英文或中英混排均可，必须包含以下 5 节（内容按本 brief 事实写，不要编造）：

1. **Layer position**：本 crate 是层表里的成长核心层，物理路径 `src/agentic`（**故意不在 `src/crates` 下**）。只允许依赖 contracts 层；禁止依赖 assembly / services / adapters / interfaces，禁止 `rusqlite`。宿主适配器在 `northhing-core::agentic::growth_adapter`。
2. **Permission matrix**：照抄下表
   | Subject | External memory (user profile / project / reference) | Self-cognition | Episode log |
   |---|---|---|---|
   | Human user | read only | no access | read only |
   | Main agent | read | read + write (exclusive) | write only |
   | judge-mom | read + write | **no access** | read + append-only annotations |
3. **Retirement authority**：judge-mom 只能改权重与关系，**无作废权**。唯一硬作废入口是 `negation.rs`（仅用户显式否定）。`garden` 与 `review` 路径出现 supersede 语义即违规。
4. **Parameters**：表格占位，写明"参数由后续任务填入，禁止散落魔法数；所有阈值集中在各模块常量并在此登记"。
5. **Verification**：`cargo test -p northhing-agentic-growth`；`node scripts/check-core-boundaries.mjs`。

## 3. 登记编辑（6 处，逐条精确执行）

### 3.1 根 `Cargo.toml`

在 `members = [` 之后紧接一行插入：

```
    "src/agentic",
```

不要动其它成员，不要重排。

### 3.2 `scripts/core-boundaries/rules/crate-layout.mjs`

(a) `crateLayoutRules` 数组内（在 `export const crateLayoutRules = [` 之后紧接）插入：

```js
  // Growth core lives outside src/crates by design (see AGENTS.md layer table).
  { crateName: 'agentic-growth', layer: 'growth', path: 'src/agentic' },

```

(b) `crateLayoutLayerNames` 数组内，在 `'support',` 之后插入 `'growth',`。

### 3.3 `scripts/core-boundaries/rules/crate-rules.mjs`

`noCoreDependencyCrates` 数组内，在 `export const noCoreDependencyCrates = [` 之后紧接插入：

```js
  'agentic-growth',
```

### 3.4 `AGENTS.md`（英文，Layered Module Index）

当前表有 6 行（`| 1 |` .. `| 6 |`）。成长核心**依赖 contracts、被 assembly 依赖**，所以它必须排在 execution（现 5）与 contracts（现 6）之间：

1. 新插入一行成为第 **6** 行：
   ```
   | 6 | Growth core | `src/agentic` | Agent growth decisions: memory distillation scheduling, judge-mom semantic weighting, competition groups, skill-promotion proposals, self-cognition writes | `agentic-growth` | [AGENTS.md](src/agentic/AGENTS.md) |
   ```
2. 原第 6 行（contracts）编号改为 **7**。其它行编号不动。
3. 在 `Boundary rules:` 列表中，紧跟 "Execution crates are portable runtime building blocks..." 那条之后，插入一条：
   ```
   - Growth core owns agent growth decisions (what to remember, how to weight it, what to promote, when self-cognition changes); it depends on contracts only and receives storage, LLM, judge, and episode-log capabilities through injected ports. It must not depend on assembly, services, adapters, or interfaces, and must not own storage implementations.
   ```

### 3.5 `AGENTS-CN.md`（中文，分层模块索引）

同 3.4 的三处等价改动，中文表述：新第 6 行「成长核心 | `src/agentic` | agent 成长决策：记忆蒸馏调度、judge-mom 语义加权、竞争组、技能固化提案、自我认知写入 | `agentic-growth` | [AGENTS.md](src/agentic/AGENTS.md)」；原 6 → 7；边界规则加一条等价中文条目。

### 3.6 `docs/status/surfaces.md`

在 "Active Capability Crates (Agent Toolbox)" 表末追加一行：

```
| `agentic-growth` | `src/agentic` | Agent growth core: memory orchestration decisions, semantic weighting, skill promotion (pure logic, no IO) |
```

## 4. 边界脚本补断言（必做）

`scripts/core-boundaries/checker.mjs` 的 `checkCrateLayoutRules()` 里，现有这段循环只覆盖 `src/crates/` 成员：

```js
  for (const member of workspaceMembers) {
    if (!member.startsWith('src/crates/')) {
      continue;
    }
    if (!expectedWorkspaceCratePaths.has(member)) {
      ...
    }
  }
```

在其之后追加一个新循环，使 `src/crates/` 之外的 crate（应用与工具除外）也必须登记在 `crateLayoutRules` 内：

```js
  for (const member of workspaceMembers) {
    if (
      member.startsWith('src/crates/') ||
      member.startsWith('src/apps/') ||
      member.startsWith('tools/') ||
      member.startsWith('northing-installer')
    ) {
      continue;
    }
    if (!expectedWorkspaceCratePaths.has(member)) {
      failures.push({
        path: manifestPath,
        line: 1,
        message: `workspace crate member outside src/crates must be registered in crate layout rules: ${member}`,
      });
    }
  }
```

排除前缀必须完整照抄（漏掉会让脚本对现有 `src/apps/*` 成员误报）。

## 5. 硬约束

- **零行为变更**：除本 brief 列出的文件外，不改任何 `.rs`。特别是不要动 `src/crates/**` 下的任何 Rust 文件。
- 空壳文件里**不要**写 `struct` / `fn` / `trait` / `TODO!()`，只放 `//!` 注释与（4 个 mod.rs 的）模块声明。这是为了让后续 5 个并行任务互不冲突。
- 日志与注释 **English-only、无 emoji**。中文只允许出现在 `AGENTS-CN.md` 的既有中文语境里。
- **不要跑 `cargo fmt`**（会污染无关文件）。手工对齐格式：4 空格缩进，与相邻文件风格一致。
- 不新增任何 workspace 依赖版本；只用 `workspace = true` 引用已有依赖。
- 只 commit 本 brief 范围内的文件。`.superpowers/sdd/**` 下的 report 在 worktree 之外，**不进 commit**。
- Rust 文件 < 800 行（本任务不会接近）。

## 6. 验证（必须实际执行并把命令与输出贴进报告）

PowerShell，每条命令前先设 PATH：

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
```

1. `cargo test -p northhing-agentic-growth`
   期望：编译通过，1 个测试通过（`error_display_includes_context`）。
2. `node scripts/check-core-boundaries.mjs`
   期望：通过（退出码 0）。若报错，必须修到通过——**这是本任务的核心验收点**。
3. `cargo check -p northhing-agentic-growth`
   期望：无 warning。若有 `unused` 类 warning，说明你在空壳里写了东西，删掉。

不要跑 `cargo check --workspace`（被上游 embed-resource 阻断，与本任务无关）。
不要跑其它 crate 的测试。

## 7. 交付

1. 在 worktree 内提交（一个 commit）：
   - message 用中文摘要 + 英文技术要点均可，格式参照仓库既有风格（如 `feat(growth): scaffold northhing-agentic-growth crate + layer registration`）
   - 提交前 `git status --short` 确认无越范围文件
2. 写报告到 `E:\agent-project\northing\.superpowers\sdd\task-g1-t1-report.md`，必须包含：
   - 状态：`DONE` / `DONE_WITH_CONCERNS` / `NEEDS_CONTEXT` / `BLOCKED`
   - 创建/修改文件清单（逐个列出）
   - §6 三条命令的**原始命令 + 输出**（不要只写"通过"）
   - `git log --oneline -1` 与 `git status --short` 输出
   - 偏离本 brief 的地方（若有）及原因
   - 遇到的意外（例如层表编号被其它文档引用）——只报告，不擅自扩大改动范围
