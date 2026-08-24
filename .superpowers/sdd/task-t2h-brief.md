# Task T2H Brief — 宿主侧成长状态适配层（core → growth 端口接线，零行为变更）

> 需求唯一来源。本文件之外的信息不得作为需求依据。
> 工作目录（唯一）：`E:\agent-project\northing\.worktrees\growth-core-0804`，分支 `feat/growth-core-0804`，基线 HEAD `1488a0d`
> 报告：`E:\agent-project\northing\.superpowers\sdd\task-t2h-report.md`（在 worktree 之外，不进 commit）

## 1. 这个任务在做什么（背景，只读）

上一批任务已完成纯逻辑 crate `northhing-agentic-growth`（`src/agentic`，第 6 层 Growth core，**零 IO**）。它靠端口 trait 由宿主注入能力。

本任务只做一件事：**在宿主 `northhing-core` 侧实现其中两个最简单的端口（`GrowthStateStore` + `Clock`），把成长状态接到真实数据库上，并把 4 个历史裸键迁移进新状态 blob。**

**零行为变更**：本任务结束后，除测试外**没有任何生产代码路径会调用**这些新函数。接线到对话回合（`turn_persist.rs` 的 4 处 hook 收敛）是后续任务，本任务**严禁**触碰。

## 2. 现状事实（已核实，直接用，不要再自行探索）

存储侧（`src/crates/assembly/core/src/service/agent_memory/`，下称 `AM/`）：

- `AM/memory_db.rs:8-10`：`pub(crate) struct MemoryDb { conn: Mutex<Connection> }`；构造 `pub(crate) fn open(db_path: &Path) -> NortHingResult<Self>`（`:30`）；无连接池、无 Arc。
- 通用 KV 表就是 **`judge_mom`**（`AM/memory_db.rs:98-102`）：列 `key TEXT PRIMARY KEY` / `value TEXT NOT NULL` / `updated_at INTEGER NOT NULL`。
- 读写 API（薄封装，已由 `AM/mod.rs:16` 导出）：
  - `AM/judge_memory.rs:4`：`pub(crate) fn get_judge_state(db: &MemoryDb, key: &str) -> NortHingResult<Option<String>>`
  - `AM/judge_memory.rs:8`：`pub(crate) fn set_judge_state(db: &MemoryDb, key: &str, value: &str, at_ms: u64) -> NortHingResult<()>`
- `MemoryDb` / `default_memory_db_path` 由 `AM/mod.rs:17` 以 `pub(crate) use` 暴露，**crate 内任何模块可用**。
- 测试用路径 seam（仅 `#[cfg(test)]`，`AM/mod.rs:19-20` 导出）：`unique_test_memory_db_path()`、`with_test_memory_db_path`、`MemoryDbPathGuard`。
- 4 个历史裸键都存在 `judge_mom` 表里，无常量定义：`distill_turns`、`distill_hit_turns`、`distiller_paused`（写侧 `AM/../turn_persist.rs:506,507,511`，值为 `to_string()` 与字符串 `"true"`）、`dream_last_sweep_at`（写侧 `AM/dream.rs:79` 等）。

crate 侧（`src/agentic/src/`，签名已固定，逐字对齐）：

- `ports.rs:182`：
  ```rust
  pub trait GrowthStateStore {
      fn get_blob(&self, key: &str) -> GrowthResult<Option<String>>;
      fn set_blob(&self, key: &str, value: &str, at_ms: u64) -> GrowthResult<()>;
      fn get_legacy_kv(&self, key: &str) -> GrowthResult<Option<String>>;
  }
  ```
- `ports.rs:203`：`pub trait Clock { fn now_ms(&self) -> u64; }`
- `state.rs:78`：`pub fn load_state(store: &dyn GrowthStateStore) -> GrowthState`（warn-only，任何异常返回 `Default`）
- `state.rs:142`：`pub fn save_state(store: &dyn GrowthStateStore, state: &GrowthState, at_ms: u64) -> GrowthResult<()>`
- 常量：`GROWTH_STATE_KEY = "growth_state_v1"`、`LEGACY_KEY_DISTILL_TURNS`/`LEGACY_KEY_DISTILL_HIT_TURNS`/`LEGACY_KEY_DISTILLER_PAUSED`/`LEGACY_KEY_DREAM_LAST_SWEEP`（`state.rs:8-14`）
- 错误类型：`GrowthError::{Port(String), Parse(String), State(String)}`（`error.rs:10-20`）
- `lib.rs` 文档已写明宿主适配层位置是 `northhing-core::agentic::growth_adapter` —— 本任务必须落在这个路径。

## 3. 交付物（只允许改/加这 3 个文件）

1. **改** `src/crates/assembly/core/Cargo.toml`：加内部依赖
2. **改** `src/crates/assembly/core/src/agentic/mod.rs`：加一行模块声明
3. **新增** `src/crates/assembly/core/src/agentic/growth_adapter.rs`

除此之外**任何文件都不许动**。特别点名禁改：`turn_persist.rs`、`dream.rs`、`distiller.rs`、`memory_db.rs`、`facts.rs`、`judge_memory.rs`、`AM/mod.rs`、`src/agentic/**`（crate 侧本任务不改一行）。

### 3.1 Cargo.toml 依赖

仓库惯例：内部 crate 用相对 `path`（**不是** `workspace = true`），每条依赖上方一行 `#` 注释。参照 `Cargo.toml:150-152` 的契约层依赖块，在其邻近位置追加：

```toml
# Growth core: agent growth decision logic (pure logic, no IO; ports injected by this crate).
northhing-agentic-growth = { path = "../../../agentic" }
```

路径校验：core 的 manifest 在 `src/crates/assembly/core/`，`../../../agentic` 解析为 `src/agentic`。必需依赖，**不要**加 `optional`、**不要**放进任何 feature。

### 3.2 模块声明

`src/crates/assembly/core/src/agentic/mod.rs`：现有 `pub mod identity;` 在 `:49`。按同样形态加：

```rust
pub mod growth_adapter;
```

**必须是 `pub mod`（不是 `pub(crate) mod`）**，理由写进注释：本模块目前无生产调用方（接线在后续任务），`pub` 可避免 `dead_code` 警告，与同样零调用方的 `identity.rs`（`:49`）保持一致。放置位置按字母序（`growth_adapter` 在 `identity` 之前）。

### 3.3 `growth_adapter.rs` 内容规格

模块文档注释（`//!`）必须说明：这是 growth 端口的宿主适配层；成长路径 warn-only；本轮只实现 `GrowthStateStore` 与 `Clock`，其余端口（记忆库/话题库/自我认知库/日志/LLM/judge）由后续任务补齐。

```rust
/// Wall-clock implementation of the growth Clock port.
pub struct SystemClock;

impl Clock for SystemClock { fn now_ms(&self) -> u64; }
```
实现：`SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)` —— 与 `turn_persist.rs:479-482` 现有写法**一致**（禁止 `unwrap()`/`expect()`）。

```rust
/// GrowthStateStore backed by the existing `judge_mom` key-value table.
pub struct JudgeMomStateStore<'a> { db: &'a MemoryDb }

impl<'a> JudgeMomStateStore<'a> { pub fn new(db: &'a MemoryDb) -> Self }
impl GrowthStateStore for JudgeMomStateStore<'_> { ... }
```
实现要点：
- `get_blob` / `get_legacy_kv` 都转发 `get_judge_state(self.db, key)`；两者读同一张表，**必须在文档注释里写明**：新状态 blob 与历史裸键共用 `judge_mom` 表，键名不冲突（blob 键是 `growth_state_v1`）。
- `set_blob` 转发 `set_judge_state(self.db, key, value, at_ms)`。
- 错误映射：`NortHingError` → `GrowthError::Port(format!("judge_mom {}: {}", 操作描述, err))`。**不得**把错误吞成 `Ok(None)`（吞错会让迁移静默丢数据；crate 侧 `load_state` 已负责 warn-only 回落）。

两个便利函数（宿主侧唯一对外入口，warn-only 语义在此落地）：

```rust
/// Loads growth state, migrating the four legacy judge_mom keys on first read.
/// Never fails: any error path yields defaults (growth is warn-only).
pub fn load_growth_state(db: &MemoryDb) -> GrowthState;

/// Persists growth state. Logs a warning on failure and returns (); growth
/// failures must never propagate into the dialog turn path.
pub fn save_growth_state(db: &MemoryDb, state: &GrowthState, at_ms: u64);
```
`save_growth_state` 内部调 `state::save_state`，`Err` 分支 `tracing::warn!` 后返回 `()`。

**不要**加 `#[allow(dead_code)]`；**不要**新增数据库表或列（`judge_mom` 已够用）；**不要**改任何 SQL。

## 4. 测试（inline `#[cfg(test)] mod tests`，仓库主流惯例）

用 `unique_test_memory_db_path()` 取路径后 `MemoryDb::open(&path)` 直接建库（`create_tables` 在 `open` 内自动执行）。

必须覆盖：
1. 全新库 → `load_growth_state` 返回默认值（逐字段断言：`schema_version == 1`、计数全 0、`paused == false`、`background_every_n_turns == 1`、`cold_start_turns_left == 10`）
2. 只有 4 个历史裸键（用 `set_judge_state` 写入 `distill_turns="7"`、`distill_hit_turns="3"`、`distiller_paused="true"`、`dream_last_sweep_at="1700000000000"`）→ 迁移后字段逐个正确
3. **旧键保留不删**：迁移并 `save_growth_state` 之后，`get_judge_state` 读 4 个旧键仍返回原值
4. **迁移幂等**：`load → save → load`，两次 `GrowthState` 相等（第二次走 blob 分支）
5. **blob 优先**：同时存在 blob 与旧键（旧键值与 blob 不同）→ 取 blob 的值
6. 脏旧键（`distill_turns="abc"`、`distiller_paused="TRUE"`）→ 不 panic；`"abc"` 当 0，`"TRUE"` 当 false（大小写敏感）
7. 修改后的 state 存取往返（改若干字段 → save → load → 相等）
8. `SystemClock::now_ms()` 返回值 > `1_700_000_000_000`（合理性）

`GrowthState` 需要 `PartialEq` 才能整体断言相等——若 crate 侧未派生 `PartialEq`，**不要去改 crate**，改为逐字段断言。

## 5. 硬约束

- 只改/加第 3 节列出的 3 个文件；`git status --short` 不得出现第 4 个文件。
- 零行为变更：不得在任何既有生产路径插入调用；不得改任何 SQL、表结构、既有函数签名。
- 成长路径 warn-only：`load_growth_state` 永不返回 `Err`；`save_growth_state` 失败只 warn。
- 日志与注释 **English-only、无 emoji**；日志用 `tracing`。
- 非测试代码禁止 `unwrap()` / `expect()` / `panic!`。
- **禁止运行 `cargo fmt`**（本仓两次污染前科）；手工 4 空格对齐，与 `identity.rs` 风格一致。
- 新文件 < 800 行（预计 250-400 行含测试）。
- 不得引入新第三方依赖。

## 6. 验证（全部实际执行，把命令与**原始输出**贴进报告）

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo check -p northhing-core --features product-full
cargo test -p northhing-core --features product-full growth_adapter
cargo test -p northhing-agentic-growth
node scripts/check-core-boundaries.mjs
```

要求：
- `cargo check` **零 warning**（若出现 `dead_code`，说明 §3.2 的 `pub mod` 没做对，修掉而不是加 `allow`）
- 边界脚本必须 `passed`（本任务新增了 assembly → growth 的依赖方向，脚本必须认可；若脚本报错，**先在报告里贴出报错原文**，不要擅自改脚本规则，停下并标 `BLOCKED`）
- 不要跑 `cargo check --workspace`（被上游 embed-resource 阻断，与本任务无关）
- 不要跑 core 全量测试（耗时且与本任务无关），只跑 `growth_adapter` 过滤

## 7. 交付

1. 在 `feat/growth-core-0804` 上提交一个 commit：`feat(growth): wire host adapter for growth state over judge_mom kv`
2. 报告写到 `E:\agent-project\northing\.superpowers\sdd\task-t2h-report.md`，包含：
   - 状态：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
   - 3 个文件的改动摘要与新文件行数
   - §6 四条命令的原始输出（`cargo check` 的 warning 计数要能看出是 0）
   - 你对「blob 与旧键共用 `judge_mom` 表」这一设计的确认说明（键名冲突分析）
   - `git log --oneline -1`、`git status --short`
   - 与本 brief 的任何偏离及原因
