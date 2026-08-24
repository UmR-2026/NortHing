# Task A1 - ports.rs + state.rs (port definitions + growth state with legacy key migration)

> Needs source: `E:\agent-project\northing\.superpowers\sdd\task-a1-brief.md` (read fully first).
> Working dir (only place to change + commit): `E:\agent-project\northing\.worktrees\growth-a1`, branch `feat/growth-a1`, baseline `7e96126`.
> Report (outside worktree, do not commit): `E:\agent-project\northing\.superpowers\sdd\task-a1-report.md` - must include all of brief §6, especially raw output of the two validation commands.

## 0. You may ONLY change these two files

- `src/agentic/src/ports.rs` (currently a 1-line shell: `//! Ports injected by the host...`)
- `src/agentic/src/state.rs` (currently a 1-line shell: `//! Persisted growth state...`)

Touching any third file (incl. `lib.rs`, `Cargo.toml`, other module shells) WILL collide - 4 parallel tasks are editing this same crate. `lib.rs` already declares `pub mod ports; pub mod state;` and `pub use error::{GrowthError, GrowthResult};`. `Cargo.toml` already has the deps you need (async-trait, serde, serde_json, thiserror, tracing; dev-dep tokio). Do NOT touch them.

## 1. Resolved ambiguities (do NOT re-decide)

- `GrowthResult` / `GrowthError` already exist in `src/agentic/src/error.rs` with variants `Port(String)`, `Parse(String)`, `State(String)`. Import from `crate::error`. Do NOT modify that file, do NOT redefine.
- Existing module files being empty shells is INTENTIONAL (only `//!` comments), not an omission - do NOT fill other modules.
- Growth path is warn-only: any exception in `load_state` returns `Default` + `tracing::warn!`, NEVER propagates errors upward.
- Hand-align to 4-space indent matching `error.rs` style. DO NOT run `cargo fmt` (repo has two prior contamination incidents).

## 2. ports.rs spec (brief §2)

### 2.1 Data types (all `#[derive(Debug, Clone)]`, add `Serialize, Deserialize` where they need to cross the DB boundary)

```rust
pub enum FactType { UserPreference, UserFeedback, ProjectMotivation, Reference }
pub enum FactStatus { Active, Superseded }
pub enum Reviewer { Distiller, JudgeMom, Garden, UserNegation }

pub struct FactRef {
    pub id: String,
    pub text: String,
    pub fact_type: FactType,
    pub entry_score: f64,          // 0.0..=1.0
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
    pub group_id: Option<String>,
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
    pub trigger: String,
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

- `FactType` / `FactStatus` / `Reviewer` need `as_str()` and `from_str_opt(&str) -> Option<Self>`.
- `Reviewer::as_str()` MUST return exactly: `"distiller"` / `"judge-mom"` / `"garden"` / `"user-negation"`.
- Document the permission matrix (brief §1 table) in the module/struct doc comments:
  | subject | external memory (user profile/project/resource) | self-cognition | log |
  | human | read-only | no access | read-only |
  | main agent | read | read + write (exclusive) | write-only |
  | judge-mom | read + write | NO access | read + append-only annotations |
- `supersede_fact` doc MUST state: judge-mom and the garden pass have NO retirement authority; ONLY the explicit-user-negation path may call it.

### 2.2 Port traits

Synchronous traits (storage semantics, host implements with rusqlite):

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

Async traits (use `#[async_trait::async_trait]`):

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

### 2.3 ports.rs tests

- `Reviewer` / `FactType` / `FactStatus` round-trip `as_str` <-> `from_str_opt`; unknown string -> `None`.
- A minimal fake impl (e.g. `Clock`) proving the trait is implementable.
- Object-safety: `ExternalMemoryStore` / `TopicStore` / `GrowthStateStore` MUST be usable as `&dyn` - write a `&dyn` assertion test.

## 3. state.rs spec (brief §3)

### 3.1 Constants & types

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

All structs derive `Serialize, Deserialize, Debug, Clone` + `#[serde(default)]` for forward-compat (new fields won't break old blobs).

`Default` for `GrowthState`: `schema_version = GROWTH_STATE_SCHEMA_VERSION`, counts all 0, `paused = false`, `background_every_n_turns = 1`, `cold_start_turns_left = 10`. (Implement `Default` manually or derive with custom defaults - your choice, but the values must match exactly.)

### 3.2 Load & save

```rust
pub fn load_state(store: &dyn GrowthStateStore) -> GrowthState;
pub fn save_state(store: &dyn GrowthStateStore, state: &GrowthState, at_ms: u64) -> GrowthResult<()>;
```

`load_state` decision order (implement each branch, test each):
1. Read `GROWTH_STATE_KEY` blob:
   - parse OK and `schema_version == 1` -> return it
   - parse OK but `schema_version` unknown (!=1) -> return `Default` + `tracing::warn!` (no panic, no Err)
   - JSON parse fails -> return `Default` + warn
2. blob missing -> run **legacy key migration**: read 4 legacy keys; parseable ones fill in (`distill_turns`/`distill_hit_turns` parse to u64, failure -> 0; `distiller_paused` only string `"true"` counts as true; `dream_last_sweep_at` parse to u64, failure -> 0); other fields default.
   - Legacy keys are read-only, never deleted (audit trail is append-only).
3. port read error -> return `Default` + warn (growth path warn-only, never propagate).

`save_state` serializes to compact JSON and writes to `GROWTH_STATE_KEY`; write failure returns `Err` as-is (caller warns, don't swallow here).

### 3.3 state.rs tests (every one required)

1. blob exists and valid -> returned as-is (assert fields individually)
2. blob missing + all 4 legacy keys present -> migration correct (including `paused = true` case)
3. blob missing + legacy keys are dirty (`"abc"` / `"TRUE"` / empty string) -> no panic, dirty values become defaults (note: `"TRUE"` != `"true"`, should be false)
4. blob missing + no legacy keys -> `Default`
5. unknown `schema_version` (e.g. 99) -> `Default`
6. bad JSON -> `Default`
7. **migration idempotent**: `load_state` -> `save_state` -> `load_state` again, two results equal (same fake store, second call hits blob branch)
8. port error -> `Default`, no panic
9. `save_state` write failure -> returns `Err`

Fake store inside `#[cfg(test)] mod tests` (use `std::cell::RefCell<HashMap<String,String>>`; note trait methods are `&self`).

## 4. Hard constraints

- Only the two files in §0.
- No IO: no `std::fs`, `tokio::fs`, network, db, process spawn. No new deps (don't touch `Cargo.toml`).
- Comments & logs English-only, no emoji.
- DO NOT run `cargo fmt`. Hand-align to 4-space indent, match `error.rs` style.
- Each file < 800 lines. `ports.rs` ~250-350 lines, `state.rs` ~250-400 lines (with tests). Over 800 = you wrote too much, cut back.
- Don't implement other modules' logic (topics / review / negation / scheduler belong to other tasks).
- No file-wide `#[allow(dead_code)]`; unused pub API is normal (later tasks use it).

## 5. Validation (must actually run and paste raw command+output into report)

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-agentic-growth
cargo check -p northhing-agentic-growth
```

Expected: all tests pass; `cargo check` no warning (`unused` warnings from new pub API usually don't fire since they're `pub`; if any appear, explain the source).

Do NOT run `cargo check --workspace` (blocked by upstream embed-resource, unrelated to this task).

## 6. Delivery

1. Commit in this worktree: `feat(growth): define growth ports and persisted state with legacy key migration`
   Before committing, `git status --short` to confirm only those two files.
2. Report at `E:\agent-project\northing\.superpowers\sdd\task-a1-report.md`, including:
   - Status: DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
   - Final line count of both files
   - Raw output of §5 commands (incl. test item names and pass counts)
   - `git log --oneline -1`, `git status --short`
   - Any deviation from this brief and the reason
