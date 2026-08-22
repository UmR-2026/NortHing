# Task A1 Brief — ports.rs + state.rs（端口定义 + 成长状态与旧键迁移）

> 需求唯一来源。本文件之外的信息不得作为需求依据。
> 工作目录（唯一）：`E:\agent-project\northing\.worktrees\growth-a1`，分支 `feat/growth-a1`，基线 `7e96126`
> 报告：`E:\agent-project\northing\.superpowers\sdd\task-a1-report.md`（在 worktree 之外，不进 commit）

## 0. 你只能改这两个文件

- `src/agentic/src/ports.rs`
- `src/agentic/src/state.rs`

**其它任何文件一行都不能改**（`lib.rs` 已预声明模块，不需要你动）。有 4 个并行任务在同一 crate 的其它文件上作业，越界就会撞车。

## 1. 背景（只读，不要照抄进代码）

本 crate 是 agent 成长核心，**纯决策逻辑，零 IO**。存储、LLM、judge 子代理、日志访问全部由宿主（`northhing-core`）以端口形式注入。本任务定义这些端口，以及成长状态的持久化与旧数据迁移。

权限矩阵（必须体现在端口的文档注释里）：

| 主体 | 外部记忆库（用户画像/项目/资源） | 自我认知库 | 日志 |
|---|---|---|---|
| 人类 | 只读 | 无权 | 只读 |
| 主 agent | 读 | 读 + 写（独占） | 只写 |
| judge-mom | 读 + 写 | **无权** | 读 + 只能追加标注 |

作废权：judge-mom 只能改权重与关系，**无作废权**。`supersede_fact` 只允许"用户显式否定"路径调用——必须在该方法的文档注释里写明这条约束。

## 2. `ports.rs` 规格

### 2.1 数据类型（全部 `#[derive(Debug, Clone)]`，需要序列化的加 `Serialize, Deserialize`）

```rust
pub enum FactType { UserPreference, UserFeedback, ProjectMotivation, Reference }
pub enum FactStatus { Active, Superseded }
pub enum Reviewer { Distiller, JudgeMom, Garden, UserNegation }

pub struct FactRef {
    pub id: String,
    pub text: String,
    pub fact_type: FactType,
    pub entry_score: f64,          // 条目分数，0.0..=1.0
    pub topics: Vec<String>,
    pub last_mentioned_at_ms: u64,
    pub status: FactStatus,
}

pub struct FactDraft {
    pub text: String,
    pub fact_type: FactType,
    pub entry_score: f64,
    pub topics: Vec<String>,
}

pub struct TopicWeight {
    pub topic: String,
    pub weight: f64,               // 0.0..=1.0
    pub mention_count: u64,
    pub last_boosted_at_ms: u64,
    pub group_id: Option<String>,  // 竞争组 id
}

pub struct ReviewRecord {
    pub fact_id: String,
    pub reviewer: Reviewer,
    pub action: String,
    pub reason: Option<String>,
    pub created_at_ms: u64,
}

pub struct SelfNote {
    pub text: String,
    pub created_at_ms: u64,
    pub trigger: String,           // 成长时刻来源描述
}

pub struct EpisodeSummary {
    pub turn_id: String,
    pub task_summary: String,
    pub tools_used: Vec<String>,
    pub failures: Vec<String>,
    pub outcome: String,
    pub ts_ms: u64,
}

pub struct SkillCandidateRef {
    pub title: String,
    pub trigger: String,
    pub steps: Vec<String>,
    pub evidence_turn_ids: Vec<String>,
}

pub enum JudgeVerdict { Approved { receipt_id: String }, Rejected { reason: String } }
```

`FactType` / `FactStatus` / `Reviewer` 需要 `as_str()` 与 `from_str_opt(&str) -> Option<Self>` 两个方法（宿主适配层要用字符串与 DB 互转）。`Reviewer::as_str()` 必须返回：`"distiller"` / `"judge-mom"` / `"garden"` / `"user-negation"`。

### 2.2 端口 trait

同步 trait（存储语义，宿主用 rusqlite 实现）：

```rust
pub trait ExternalMemoryStore {
    fn search_similar(&self, text: &str, limit: usize) -> GrowthResult<Vec<FactRef>>;
    fn insert_fact(&self, draft: &FactDraft) -> GrowthResult<String>;
    fn touch_fact(&self, fact_id: &str, at_ms: u64) -> GrowthResult<()>;
    fn record_review(&self, record: &ReviewRecord) -> GrowthResult<()>;
    fn stale_facts(&self, last_mentioned_before_ms: u64, limit: usize) -> GrowthResult<Vec<FactRef>>;
    fn facts_for_topic(&self, topic: &str, limit: usize) -> GrowthResult<Vec<FactRef>>;
    /// Hard retirement. ONLY the explicit-user-negation path may call this.
    /// judge-mom and the garden pass have no retirement authority.
    fn supersede_fact(&self, fact_id: &str, superseded_by: Option<&str>, at_ms: u64) -> GrowthResult<()>;
}

pub trait TopicStore {
    fn get_topics(&self, names: &[String]) -> GrowthResult<Vec<TopicWeight>>;
    fn upsert_topic(&self, topic: &TopicWeight) -> GrowthResult<()>;
    fn boost_topic(&self, topic: &str, delta: f64, at_ms: u64) -> GrowthResult<()>;
    fn decay_all(&self, factor: f64, floor: f64) -> GrowthResult<usize>;
    fn group_members(&self, group_id: &str) -> GrowthResult<Vec<TopicWeight>>;
    fn set_group(&self, group_id: &str, topics: &[String]) -> GrowthResult<()>;
}

pub trait GrowthStateStore {
    fn get_blob(&self, key: &str) -> GrowthResult<Option<String>>;
    fn set_blob(&self, key: &str, value: &str, at_ms: u64) -> GrowthResult<()>;
    /// Reads a pre-migration flat key. Used once by state migration; never written.
    fn get_legacy_kv(&self, key: &str) -> GrowthResult<Option<String>>;
}

/// Self-cognition store. Agent-exclusive: judge-mom, the garden pass, and the
/// review path must never receive this port.
pub trait SelfCognitionStore {
    fn load(&self) -> GrowthResult<Vec<SelfNote>>;
    fn append(&self, note: &SelfNote) -> GrowthResult<()>;
}

/// Episode log. The main agent writes only; judge-mom reads and may append
/// annotations; existing lines are never rewritten.
pub trait EpisodeLog {
    fn recent(&self, workspace_slug: &str, limit: usize) -> GrowthResult<Vec<EpisodeSummary>>;
    fn append_annotation(&self, turn_id: &str, note: &str) -> GrowthResult<()>;
}

pub trait Clock {
    fn now_ms(&self) -> u64;
}
```

异步 trait（用 `#[async_trait::async_trait]`）：

```rust
/// Cheap in-process single-shot LLM call (distillation, judge-mom routine review).
pub trait LlmPort {
    async fn complete(&self, system_prompt: &str, user_content: &str) -> GrowthResult<String>;
}

/// Expensive subagent-backed judge. Used only for skill promotion.
pub trait JudgePort {
    async fn judge_skill_candidate(&self, candidate: &SkillCandidateRef) -> GrowthResult<JudgeVerdict>;
}
```

`GrowthResult` / `GrowthError` 从 `crate::error` 引入（已存在，不要改）。

### 2.3 ports.rs 测试

- `Reviewer` / `FactType` / `FactStatus` 的 `as_str` ↔ `from_str_opt` 往返；未知字符串 → `None`
- 一个最小 fake 实现（例如 `Clock`）证明 trait 可被实现（object-safe：`ExternalMemoryStore` / `TopicStore` / `GrowthStateStore` 必须能作为 `&dyn` 使用，写一个 `&dyn` 断言测试）

## 3. `state.rs` 规格

### 3.1 常量与类型

```rust
pub const GROWTH_STATE_KEY: &str = "growth_state_v1";
pub const GROWTH_STATE_SCHEMA_VERSION: u32 = 1;

pub const LEGACY_KEY_DISTILL_TURNS: &str = "distill_turns";
pub const LEGACY_KEY_DISTILL_HIT_TURNS: &str = "distill_hit_turns";
pub const LEGACY_KEY_DISTILLER_PAUSED: &str = "distiller_paused";
pub const LEGACY_KEY_DREAM_LAST_SWEEP: &str = "dream_last_sweep_at";

pub struct DistillStats { pub turns: u64, pub hit_turns: u64, pub paused: bool }
pub struct GardenCursor { pub last_sweep_at_ms: u64 }
pub struct JudgeStats { pub reviewed: u64, pub boosted: u64, pub merged: u64, pub competitions_proposed: u64, pub competitions_confirmed: u64 }
pub struct TimingPrefs { pub background_every_n_turns: u32, pub cold_start_turns_left: u32 }
pub struct GrowthState { pub schema_version: u32, pub distill: DistillStats, pub garden: GardenCursor, pub judge: JudgeStats, pub prefs: TimingPrefs }
```

全部 `Serialize, Deserialize, Debug, Clone` + `#[serde(default)]` 保证前向兼容（新增字段不炸旧 blob）。

`Default`：`schema_version = GROWTH_STATE_SCHEMA_VERSION`，计数全 0，`paused = false`，`background_every_n_turns = 1`，`cold_start_turns_left = 10`。

### 3.2 加载与保存

```rust
pub fn load_state(store: &dyn GrowthStateStore) -> GrowthState;
pub fn save_state(store: &dyn GrowthStateStore, state: &GrowthState, at_ms: u64) -> GrowthResult<()>;
```

`load_state` 判定顺序（**逐条实现，测试逐条覆盖**）：

1. 读 `GROWTH_STATE_KEY` blob：
   - 解析成功且 `schema_version == 1` → 返回它
   - 解析成功但 `schema_version` 未知（≠1）→ 返回 `Default`，并 `tracing::warn!` 记录（不 panic、不报错）
   - JSON 解析失败 → 返回 `Default` 并 warn
2. blob 不存在 → 走**旧键迁移**：读 4 个 legacy key，能解析的填入（`distill_turns`/`distill_hit_turns` 解析成 u64，失败当 0；`distiller_paused` 仅字符串 `"true"` 视为 true；`dream_last_sweep_at` 解析成 u64，失败当 0），其余字段取默认值。
   - **旧键只读不删**（审计轨迹只增）。
3. 读取端口报错 → 返回 `Default` 并 warn（成长路径 warn-only，绝不向上传播）。

`save_state` 序列化为紧凑 JSON 写入 `GROWTH_STATE_KEY`；写失败原样返回 `Err`（由调用方 warn，不在此处吞掉）。

### 3.3 state.rs 测试（每条都要有）

1. blob 存在且合法 → 原样返回（字段逐个断言）
2. blob 不存在 + 4 个旧键齐全 → 迁移正确（含 `paused = true` 的情况）
3. blob 不存在 + 旧键为脏值（`"abc"` / `"TRUE"` / 空串）→ 不 panic，脏值当默认（注意：`"TRUE"` 不等于 `"true"`，应为 false）
4. blob 不存在 + 无旧键 → `Default`
5. 未知 `schema_version`（如 99）→ `Default`
6. 坏 JSON → `Default`
7. **迁移幂等**：`load_state` → `save_state` → 再 `load_state`，两次结果相等（用同一个 fake store，第二次会走 blob 分支）
8. 端口报错 → `Default`，不 panic
9. `save_state` 写失败 → 返回 `Err`

fake store 写在 `#[cfg(test)] mod tests` 内（用 `std::cell::RefCell<HashMap<String,String>>` 即可，注意 trait 方法是 `&self`）。

## 4. 硬约束

- 只改第 0 节那两个文件。
- 无 IO：不得出现 `std::fs`、`tokio::fs`、网络、数据库、进程调用。不得新增任何依赖（`Cargo.toml` 不许改）。
- 注释与日志 **English-only、无 emoji**。
- **禁止运行 `cargo fmt`**（本仓两次污染前科）。手工对齐：4 空格缩进，与 `error.rs` 风格一致。
- 每个文件 < 800 行。`ports.rs` 预计 250-350 行，`state.rs` 预计 250-400 行（含测试）。若超 800 行说明你写多了，回头砍。
- 不要实现别的模块的逻辑（topics / review / negation / scheduler 归其它任务）。
- 不要 `#[allow(dead_code)]` 整文件；未被使用的公开 API 属正常（后续任务会用）。

## 5. 验证（必须实际执行并把命令与原始输出贴进报告）

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-agentic-growth
cargo check -p northhing-agentic-growth
```

期望：全部测试通过；`cargo check` 无 warning（`unused` warning 若来自你新增的公开 API 通常不会触发，因为是 `pub`；若出现请说明来源）。

不要跑 `cargo check --workspace`（被上游 embed-resource 阻断，与本任务无关）。

## 6. 交付

1. 在本 worktree 内提交一个 commit：`feat(growth): define growth ports and persisted state with legacy key migration`
   提交前 `git status --short` 确认只有那两个文件。
2. 报告写到 `E:\agent-project\northing\.superpowers\sdd\task-a1-report.md`，包含：
   - 状态：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
   - 两个文件的最终行数
   - §5 命令的原始输出（含测试条目名与通过数）
   - `git log --oneline -1`、`git status --short`
   - 与本 brief 的任何偏离及原因
